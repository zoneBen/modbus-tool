use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use regex::Regex;
use russh::client::{self, Handle, Msg};
use russh::keys::{PrivateKeyWithHashAlg, PublicKey, load_secret_key};
use russh::{ChannelMsg, ChannelWriteHalf, Disconnect};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

use crate::event::{AppEvent, EventBus, emit};

use super::model::{
    SshConnectInput, SshCwdDto, SshDataDto, SshDownloadInput, SshFileEntryDto, SshListResultDto,
    SshReadResultDto, SshSessionStateDto, SshTransferDto, SshTransferInput, ToolErrorDto,
    ToolResult,
};

const OUTPUT_LIMIT: usize = 512 * 1024;
const OUTPUT_READ_LIMIT: usize = 128 * 1024;
const TRANSFER_CHUNK: usize = 64 * 1024;
const CWD_SCAN_LIMIT: usize = 16 * 1024;

const CWD_TRACKING_SNIPPET: &str = r#"if [ -n "${BASH_VERSION:-}" ]; then case "$PROMPT_COMMAND" in *__mt_osc7*) ;; *) PROMPT_COMMAND='printf "\033]7;file://%s%s\033\\" "${HOSTNAME:-localhost}" "$PWD";'"$PROMPT_COMMAND";; esac; elif [ -n "${ZSH_VERSION:-}" ]; then _mt_osc7() { printf "\033]7;file://%s%s\033\\" "${HOSTNAME:-localhost}" "$PWD"; }; (( ${precmd_functions[(I)_mt_osc7]} )) || precmd_functions+=(_mt_osc7); fi; echo"#;

struct AcceptAnyHostKey;

impl client::Handler for AcceptAnyHostKey {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // Interactive console: the user explicitly targets the host. Surface
        // this limitation in the UI instead of silently pinning keys.
        Ok(true)
    }
}

#[derive(Default)]
struct OutputBuffer {
    data: VecDeque<u8>,
    base: u64,
    end: u64,
}

impl OutputBuffer {
    fn push(&mut self, bytes: &[u8]) {
        self.data.extend(bytes.iter().copied());
        self.end += bytes.len() as u64;
        while self.data.len() > OUTPUT_LIMIT {
            self.data.pop_front();
            self.base += 1;
        }
    }

    fn read(&self, since: Option<u64>) -> (String, u64, bool) {
        let start = since.unwrap_or(self.base);
        let truncated = start < self.base;
        let start = start.max(self.base).min(self.end);
        let offset = (start - self.base) as usize;
        let available = self.data.len().saturating_sub(offset);
        let take = available.min(OUTPUT_READ_LIMIT);
        let bytes: Vec<u8> = self.data.iter().skip(offset).take(take).copied().collect();
        let cursor = start + take as u64;
        (
            String::from_utf8_lossy(&bytes).into_owned(),
            cursor,
            truncated,
        )
    }
}

struct SessionStatus {
    status: String,
    detail: Option<String>,
}

struct SshSession {
    id: String,
    name: Option<String>,
    host: String,
    port: u16,
    username: String,
    status: Mutex<SessionStatus>,
    cols: AtomicU32,
    rows: AtomicU32,
    connected_at_ms: u64,
    handle: tokio::sync::Mutex<Handle<AcceptAnyHostKey>>,
    writer: tokio::sync::Mutex<ChannelWriteHalf<Msg>>,
    output: Mutex<OutputBuffer>,
    cwd: Mutex<Option<String>>,
}

impl SshSession {
    fn state(&self) -> SshSessionStateDto {
        let status = self
            .status
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        SshSessionStateDto {
            session_id: self.id.clone(),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            status: status.status.clone(),
            detail: status.detail.clone(),
            cols: self.cols.load(Ordering::Relaxed),
            rows: self.rows.load(Ordering::Relaxed),
            connected_at_ms: self.connected_at_ms,
            cwd: self.cwd(),
        }
    }

    fn set_status(&self, status: &str, detail: Option<String>) {
        *self
            .status
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = SessionStatus {
            status: status.to_owned(),
            detail,
        };
    }

    fn append_output(&self, bytes: &[u8]) {
        self.output
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(bytes);
    }

    fn set_cwd(&self, path: String) {
        *self.cwd.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(path);
    }

    fn cwd(&self) -> Option<String> {
        self.cwd
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

struct SshRuntime {
    session: Arc<SshSession>,
    reader: tokio::task::JoinHandle<()>,
}

pub struct SshSessionManager {
    bus: EventBus,
    sessions: Mutex<HashMap<String, SshRuntime>>,
}

impl SshSessionManager {
    pub fn new(bus: EventBus) -> Self {
        Self {
            bus,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn connect(&self, input: SshConnectInput) -> ToolResult<SshSessionStateDto> {
        let host = input.host.trim().to_owned();
        if host.is_empty() {
            return Err(ToolErrorDto::new("invalidHost", "Host is required"));
        }
        if input.username.trim().is_empty() {
            return Err(ToolErrorDto::new("invalidUser", "Username is required"));
        }
        let session_id = Uuid::new_v4().to_string();
        let cols = input.cols.clamp(20, 500);
        let rows = input.rows.clamp(4, 300);
        emit_state(
            &self.bus,
            SshSessionStateDto {
                session_id: session_id.clone(),
                name: input.name.clone(),
                host: host.clone(),
                port: input.port,
                username: input.username.trim().to_owned(),
                status: "connecting".to_owned(),
                detail: None,
                cols,
                rows,
                connected_at_ms: now_ms(),
                cwd: None,
            },
        );

        let timeout = Duration::from_millis(input.timeout_ms.clamp(2_000, 60_000));
        let follow_cwd = input.follow_cwd;
        let connected =
            tokio::time::timeout(timeout, self.open(input, session_id.clone(), cols, rows)).await;
        let (session, read_half) = match connected {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                emit_error_state(&self.bus, &session_id, &error);
                return Err(error);
            }
            Err(_) => {
                let error = ToolErrorDto::new("sshTimeout", "SSH connection timed out");
                emit_error_state(&self.bus, &session_id, &error);
                return Err(error);
            }
        };

        let session = Arc::new(session);
        let reader_session = session.clone();
        let bus = self.bus.clone();
        let reader_id = session_id.clone();
        let reader = tokio::spawn(async move {
            let mut read_half = read_half;
            let mut cwd_scan = Vec::new();
            while let Some(message) = read_half.wait().await {
                match message {
                    ChannelMsg::Data { data } => {
                        let bytes = data.as_ref();
                        reader_session.append_output(bytes);
                        emit_data(&bus, &reader_id, bytes);
                        if let Some(path) = extract_osc7(&mut cwd_scan, bytes) {
                            reader_session.set_cwd(path.clone());
                            emit_cwd(&bus, &reader_id, path);
                        }
                    }
                    ChannelMsg::ExtendedData { data, .. } => {
                        let bytes = data.as_ref();
                        reader_session.append_output(bytes);
                        emit_data(&bus, &reader_id, bytes);
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        reader_session.set_status(
                            "disconnected",
                            Some(format!("remote exit status {exit_status}")),
                        );
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => break,
                    _ => {}
                }
            }
            reader_session.set_status("disconnected", Some("channel closed".to_owned()));
            emit_state(&bus, reader_session.state());
        });

        let state = session.state();
        if follow_cwd {
            let _ = send_cwd_tracking(&session).await;
        }
        {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            sessions.insert(session_id, SshRuntime { session, reader });
        }
        emit_state(&self.bus, state.clone());
        Ok(state)
    }

    async fn open(
        &self,
        input: SshConnectInput,
        session_id: String,
        cols: u32,
        rows: u32,
    ) -> ToolResult<(SshSession, russh::ChannelReadHalf)> {
        let host = input.host.trim().to_owned();
        let config = Arc::new(client::Config {
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(30)),
            keepalive_max: 3,
            nodelay: true,
            ..client::Config::default()
        });
        let mut handle = client::connect(config, (host.as_str(), input.port), AcceptAnyHostKey)
            .await
            .map_err(|error| ToolErrorDto::new("sshConnect", error.to_string()))?;

        let authenticated = authenticate(&mut handle, &input).await?;
        if !authenticated {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "auth failed", "en")
                .await;
            return Err(ToolErrorDto::new(
                "sshAuthFailed",
                "The server rejected the credentials",
            ));
        }

        let channel = handle
            .channel_open_session()
            .await
            .map_err(|error| ToolErrorDto::new("sshChannel", error.to_string()))?;
        channel
            .request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .map_err(|error| ToolErrorDto::new("sshPty", error.to_string()))?;
        channel
            .request_shell(true)
            .await
            .map_err(|error| ToolErrorDto::new("sshShell", error.to_string()))?;
        let (read_half, write_half) = channel.split();

        Ok((
            SshSession {
                id: session_id,
                name: input.name,
                host,
                port: input.port,
                username: input.username.trim().to_owned(),
                status: Mutex::new(SessionStatus {
                    status: "connected".to_owned(),
                    detail: None,
                }),
                cols: AtomicU32::new(cols),
                rows: AtomicU32::new(rows),
                connected_at_ms: now_ms(),
                handle: tokio::sync::Mutex::new(handle),
                writer: tokio::sync::Mutex::new(write_half),
                output: Mutex::new(OutputBuffer::default()),
                cwd: Mutex::new(None),
            },
            read_half,
        ))
    }

    pub fn list(&self) -> Vec<SshSessionStateDto> {
        let mut states: Vec<SshSessionStateDto> = self
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .values()
            .map(|runtime| runtime.session.state())
            .collect();
        states.sort_by_key(|left| left.connected_at_ms);
        states
    }

    pub async fn write_input(&self, session_id: &str, data: &str) -> ToolResult<()> {
        let session = self.session(session_id)?;
        let writer = session.writer.lock().await;
        let mut stream = writer.make_writer();
        stream
            .write_all(data.as_bytes())
            .await
            .map_err(|error| ToolErrorDto::new("sshWrite", error.to_string()))?;
        stream
            .flush()
            .await
            .map_err(|error| ToolErrorDto::new("sshWrite", error.to_string()))
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> ToolResult<()> {
        let session = self.session(session_id)?;
        let cols = cols.clamp(20, 500);
        let rows = rows.clamp(4, 300);
        session.cols.store(cols, Ordering::Relaxed);
        session.rows.store(rows, Ordering::Relaxed);
        let writer = session.writer.lock().await;
        writer
            .window_change(cols, rows, 0, 0)
            .await
            .map_err(|error| ToolErrorDto::new("sshResize", error.to_string()))
    }

    pub async fn disconnect(&self, session_id: &str) -> bool {
        let runtime = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            sessions.remove(session_id)
        };
        let Some(runtime) = runtime else {
            return false;
        };
        runtime
            .session
            .set_status("disconnected", Some("disconnected by user".to_owned()));
        {
            let writer = runtime.session.writer.lock().await;
            let _ = writer.close().await;
        }
        {
            let handle = runtime.session.handle.lock().await;
            let _ = handle.disconnect(Disconnect::ByApplication, "", "en").await;
        }
        runtime.reader.abort();
        emit_state(&self.bus, runtime.session.state());
        true
    }

    pub async fn stop_all(&self) {
        let ids: Vec<String> = self
            .sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .keys()
            .cloned()
            .collect();
        for id in ids {
            self.disconnect(&id).await;
        }
    }

    pub async fn set_cwd_tracking(&self, session_id: &str, enabled: bool) -> ToolResult<()> {
        if !enabled {
            return Ok(());
        }
        let session = self.session(session_id)?;
        send_cwd_tracking(&session).await
    }

    pub fn read_output(
        &self,
        session_id: &str,
        cursor: Option<u64>,
    ) -> ToolResult<SshReadResultDto> {
        let session = self.session(session_id)?;
        let (data, cursor, truncated) = session
            .output
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .read(cursor);
        Ok(SshReadResultDto {
            session_id: session_id.to_owned(),
            data,
            cursor,
            truncated,
        })
    }

    pub async fn sftp_list(
        &self,
        session_id: &str,
        path: Option<String>,
    ) -> ToolResult<SshListResultDto> {
        let sftp = self.sftp(session_id).await?;
        let requested = path.unwrap_or_default();
        let requested = if requested.trim().is_empty() {
            ".".to_owned()
        } else {
            requested.trim().to_owned()
        };
        let absolute = match sftp.canonicalize(&requested).await {
            Ok(value) => value.to_string(),
            Err(_) => requested,
        };
        let mut entries = Vec::new();
        let read_dir = sftp
            .read_dir(&absolute)
            .await
            .map_err(|error| ToolErrorDto::new("sftpList", error.to_string()))?;
        for entry in read_dir {
            let name = entry.file_name().to_string();
            if name == "." || name == ".." {
                continue;
            }
            let metadata = entry.metadata();
            let is_dir = metadata.file_type().is_dir();
            let is_symlink = metadata.file_type().is_symlink();
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0);
            entries.push(SshFileEntryDto {
                name,
                path: entry.path().to_string(),
                is_dir,
                size: metadata.len(),
                modified_ms,
                permissions: format!(
                    "{}{}",
                    if is_dir {
                        "d"
                    } else if is_symlink {
                        "l"
                    } else {
                        "-"
                    },
                    metadata.permissions()
                ),
                is_symlink,
            });
        }
        entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        Ok(SshListResultDto {
            path: absolute,
            entries,
        })
    }

    pub async fn sftp_mkdir(&self, session_id: &str, path: &str) -> ToolResult<()> {
        let sftp = self.sftp(session_id).await?;
        sftp.create_dir(path)
            .await
            .map_err(|error| ToolErrorDto::new("sftpMkdir", error.to_string()))
    }

    pub async fn sftp_remove(&self, session_id: &str, path: &str, is_dir: bool) -> ToolResult<()> {
        let sftp = self.sftp(session_id).await?;
        let result = if is_dir {
            sftp.remove_dir(path).await
        } else {
            sftp.remove_file(path).await
        };
        result.map_err(|error| ToolErrorDto::new("sftpRemove", error.to_string()))
    }

    pub async fn sftp_rename(&self, session_id: &str, from: &str, to: &str) -> ToolResult<()> {
        let sftp = self.sftp(session_id).await?;
        sftp.rename(from, to)
            .await
            .map_err(|error| ToolErrorDto::new("sftpRename", error.to_string()))
    }

    pub async fn upload(&self, input: SshTransferInput) -> ToolResult<()> {
        let session_id = input.session_id.clone();
        let result = self.upload_inner(&input).await;
        if let Err(error) = &result {
            emit_transfer(
                &self.bus,
                &session_id,
                "upload",
                &input.local_path,
                0,
                0,
                "error",
                Some(error.detail.clone()),
            );
        }
        result
    }

    async fn upload_inner(&self, input: &SshTransferInput) -> ToolResult<()> {
        let local = Path::new(input.local_path.trim());
        if !local.is_file() {
            return Err(ToolErrorDto::new(
                "notAFile",
                "Selected upload path is not a file",
            ));
        }
        let filename = local
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ToolErrorDto::new("invalidPath", "Invalid local file name"))?;
        let total = tokio::fs::metadata(local)
            .await
            .map_err(|error| ToolErrorDto::new("fsReadFailed", error.to_string()))?
            .len();
        let remote_path = remote_join(&input.remote_dir, filename);
        let sftp = self.sftp(&input.session_id).await?;
        let mut source = tokio::fs::File::open(local)
            .await
            .map_err(|error| ToolErrorDto::new("fsReadFailed", error.to_string()))?;
        let mut target = sftp
            .create(&remote_path)
            .await
            .map_err(|error| ToolErrorDto::new("sftpUpload", error.to_string()))?;
        emit_transfer(
            &self.bus,
            &input.session_id,
            "upload",
            &remote_path,
            0,
            total,
            "start",
            None,
        );
        copy_with_progress(
            &self.bus,
            &input.session_id,
            "upload",
            &remote_path,
            &mut source,
            &mut target,
            total,
        )
        .await?;
        target
            .close()
            .await
            .map_err(|error| ToolErrorDto::new("sftpUpload", error.to_string()))?;
        emit_transfer(
            &self.bus,
            &input.session_id,
            "upload",
            &remote_path,
            total,
            total,
            "done",
            None,
        );
        Ok(())
    }

    pub async fn download(&self, input: SshDownloadInput) -> ToolResult<()> {
        let result = self.download_inner(&input).await;
        if let Err(error) = &result {
            emit_transfer(
                &self.bus,
                &input.session_id,
                "download",
                &input.remote_path,
                0,
                0,
                "error",
                Some(error.detail.clone()),
            );
        }
        result
    }

    async fn download_inner(&self, input: &SshDownloadInput) -> ToolResult<()> {
        let sftp = self.sftp(&input.session_id).await?;
        let metadata = sftp
            .metadata(&input.remote_path)
            .await
            .map_err(|error| ToolErrorDto::new("sftpDownload", error.to_string()))?;
        if metadata.file_type().is_dir() {
            return Err(ToolErrorDto::new(
                "sftpDownload",
                "Cannot download a directory",
            ));
        }
        let total = metadata.len();
        let mut source = sftp
            .open(&input.remote_path)
            .await
            .map_err(|error| ToolErrorDto::new("sftpDownload", error.to_string()))?;
        let mut target = tokio::fs::File::create(&input.local_path)
            .await
            .map_err(|error| ToolErrorDto::new("fsWriteFailed", error.to_string()))?;
        emit_transfer(
            &self.bus,
            &input.session_id,
            "download",
            &input.remote_path,
            0,
            total,
            "start",
            None,
        );
        copy_with_progress(
            &self.bus,
            &input.session_id,
            "download",
            &input.remote_path,
            &mut source,
            &mut target,
            total,
        )
        .await?;
        target
            .flush()
            .await
            .map_err(|error| ToolErrorDto::new("fsWriteFailed", error.to_string()))?;
        emit_transfer(
            &self.bus,
            &input.session_id,
            "download",
            &input.remote_path,
            total,
            total,
            "done",
            None,
        );
        Ok(())
    }

    pub async fn call_mcp(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> ToolResult<serde_json::Value> {
        super::ssh_mcp::call(self, name, arguments).await
    }

    fn session(&self, session_id: &str) -> ToolResult<Arc<SshSession>> {
        self.sessions
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session_id)
            .map(|runtime| runtime.session.clone())
            .ok_or_else(|| ToolErrorDto::new("sshSessionMissing", "SSH session is not connected"))
    }

    async fn sftp(&self, session_id: &str) -> ToolResult<SftpSession> {
        let session = self.session(session_id)?;
        let channel = {
            let handle = session.handle.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|error| ToolErrorDto::new("sshChannel", error.to_string()))?
        };
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| ToolErrorDto::new("sftpSubsystem", error.to_string()))?;
        SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| ToolErrorDto::new("sftpInit", error.to_string()))
    }
}

async fn authenticate(
    handle: &mut Handle<AcceptAnyHostKey>,
    input: &SshConnectInput,
) -> ToolResult<bool> {
    match input.auth.as_str() {
        "key" => {
            let path = input
                .key_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ToolErrorDto::new("invalidKey", "Private key path is required"))?;
            let key = load_secret_key(path, input.key_passphrase.as_deref())
                .map_err(|error| ToolErrorDto::new("sshKey", error.to_string()))?;
            let hash = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|error| ToolErrorDto::new("sshAuth", error.to_string()))?
                .flatten();
            Ok(handle
                .authenticate_publickey(
                    input.username.trim(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                )
                .await
                .map_err(|error| ToolErrorDto::new("sshAuth", error.to_string()))?
                .success())
        }
        _ => Ok(handle
            .authenticate_password(
                input.username.trim(),
                input.password.as_deref().unwrap_or_default(),
            )
            .await
            .map_err(|error| ToolErrorDto::new("sshAuth", error.to_string()))?
            .success()),
    }
}

async fn send_cwd_tracking(session: &SshSession) -> ToolResult<()> {
    let writer = session.writer.lock().await;
    let mut stream = writer.make_writer();
    stream
        .write_all(CWD_TRACKING_SNIPPET.as_bytes())
        .await
        .map_err(|error| ToolErrorDto::new("sshWrite", error.to_string()))?;
    stream
        .write_all(b"\r")
        .await
        .map_err(|error| ToolErrorDto::new("sshWrite", error.to_string()))?;
    stream
        .flush()
        .await
        .map_err(|error| ToolErrorDto::new("sshWrite", error.to_string()))
}

async fn copy_with_progress<R, W>(
    bus: &EventBus,
    session_id: &str,
    direction: &str,
    path: &str,
    source: &mut R,
    target: &mut W,
    total: u64,
) -> ToolResult<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = vec![0u8; TRANSFER_CHUNK];
    let mut transferred = 0u64;
    let mut last_emit = Instant::now();
    loop {
        let read = source
            .read(&mut buffer)
            .await
            .map_err(|error| ToolErrorDto::new("transferRead", error.to_string()))?;
        if read == 0 {
            break;
        }
        target
            .write_all(&buffer[..read])
            .await
            .map_err(|error| ToolErrorDto::new("transferWrite", error.to_string()))?;
        transferred += read as u64;
        if last_emit.elapsed() >= Duration::from_millis(180) {
            emit_transfer(
                bus,
                session_id,
                direction,
                path,
                transferred,
                total,
                "progress",
                None,
            );
            last_emit = Instant::now();
        }
    }
    target
        .flush()
        .await
        .map_err(|error| ToolErrorDto::new("transferWrite", error.to_string()))?;
    emit_transfer(
        bus,
        session_id,
        direction,
        path,
        transferred,
        total,
        "progress",
        None,
    );
    Ok(())
}

fn remote_join(dir: &str, name: &str) -> String {
    let dir = dir.trim();
    if dir.is_empty() || dir == "." {
        name.to_owned()
    } else if dir == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", dir.trim_end_matches('/'), name)
    }
}

fn extract_osc7(tail: &mut Vec<u8>, chunk: &[u8]) -> Option<String> {
    tail.extend_from_slice(chunk);
    if tail.len() > CWD_SCAN_LIMIT {
        let keep = CWD_SCAN_LIMIT / 2;
        let drain = tail.len() - keep;
        tail.drain(..drain);
    }
    loop {
        let start = find_bytes(tail, b"\x1b]7;")?;
        let rest = &tail[start + 4..];
        let bell = rest.iter().position(|byte| *byte == 0x07);
        let st = find_bytes(rest, b"\x1b\\");
        let (end, terminator) = match (bell, st) {
            (Some(bell), Some(st)) => {
                if bell < st {
                    (bell, 1)
                } else {
                    (st, 2)
                }
            }
            (Some(bell), None) => (bell, 1),
            (None, Some(st)) => (st, 2),
            (None, None) => return None,
        };
        let payload = String::from_utf8_lossy(&rest[..end]).into_owned();
        tail.drain(..start + 4 + end + terminator);
        if let Some(path) = parse_file_url(&payload) {
            return Some(path);
        }
    }
}

fn parse_file_url(payload: &str) -> Option<String> {
    let rest = payload.strip_prefix("file://")?;
    let slash = rest.find('/')?;
    let path = &rest[slash..];
    if path.is_empty() {
        return None;
    }
    urlencoding::decode(path)
        .ok()
        .map(|value| value.into_owned())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

pub(crate) fn strip_ansi(value: &str) -> String {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(\x07|\x1b\\)|\x1b[@-_]")
            .expect("valid ANSI regex")
    });
    pattern.replace_all(value, "").into_owned()
}

fn emit_state(bus: &EventBus, state: SshSessionStateDto) {
    emit(bus, AppEvent::SshState(state));
}

fn emit_error_state(bus: &EventBus, session_id: &str, error: &ToolErrorDto) {
    emit_state(
        bus,
        SshSessionStateDto {
            session_id: session_id.to_owned(),
            name: None,
            host: String::new(),
            port: 0,
            username: String::new(),
            status: "error".to_owned(),
            detail: Some(format!("{}: {}", error.code, error.detail)),
            cols: 0,
            rows: 0,
            connected_at_ms: now_ms(),
            cwd: None,
        },
    );
}

fn emit_data(bus: &EventBus, session_id: &str, bytes: &[u8]) {
    emit(
        bus,
        AppEvent::SshData(SshDataDto {
            session_id: session_id.to_owned(),
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
            timestamp_ms: now_ms(),
        }),
    );
}

fn emit_cwd(bus: &EventBus, session_id: &str, path: String) {
    emit(
        bus,
        AppEvent::SshCwd(SshCwdDto {
            session_id: session_id.to_owned(),
            path,
            timestamp_ms: now_ms(),
        }),
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_transfer(
    bus: &EventBus,
    session_id: &str,
    direction: &str,
    path: &str,
    transferred: u64,
    total: u64,
    status: &str,
    detail: Option<String>,
) {
    emit(
        bus,
        AppEvent::SshTransfer(SshTransferDto {
            session_id: session_id.to_owned(),
            direction: direction.to_owned(),
            path: path.to_owned(),
            transferred,
            total,
            status: status.to_owned(),
            detail,
            timestamp_ms: now_ms(),
        }),
    );
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_remote_paths() {
        assert_eq!(remote_join("/home/user", "a.txt"), "/home/user/a.txt");
        assert_eq!(remote_join("/", "a.txt"), "/a.txt");
        assert_eq!(remote_join(".", "a.txt"), "a.txt");
        assert_eq!(remote_join("/tmp/", "a.txt"), "/tmp/a.txt");
    }

    #[test]
    fn parses_osc7_urls() {
        assert_eq!(
            parse_file_url("file://host/home/user").as_deref(),
            Some("/home/user")
        );
        assert_eq!(
            parse_file_url("file:///tmp/a%20b").as_deref(),
            Some("/tmp/a b")
        );
        assert!(parse_file_url("http://host/path").is_none());
    }

    #[test]
    fn extracts_split_osc7_sequences() {
        let mut tail = Vec::new();
        assert!(extract_osc7(&mut tail, b"prompt\x1b]7;file://host/ho").is_none());
        let path = extract_osc7(&mut tail, b"me/user\x07$ ").expect("path");
        assert_eq!(path, "/home/user");
    }

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("a\x1b]0;title\x07b"), "ab");
    }

    #[test]
    fn output_buffer_tracks_cursor_and_truncation() {
        let mut buffer = OutputBuffer::default();
        buffer.push(b"hello");
        buffer.push(b" world");
        let (data, cursor, truncated) = buffer.read(None);
        assert_eq!(data, "hello world");
        assert_eq!(cursor, 11);
        assert!(!truncated);
        let (data, cursor, _) = buffer.read(Some(6));
        assert_eq!(data, "world");
        assert_eq!(cursor, 11);
    }
}
