use std::sync::Arc;
use std::time::{Duration, Instant};

use russh::client;
use russh::keys::{PrivateKeyWithHashAlg, PublicKey, load_secret_key};
use russh::{ChannelMsg, Disconnect};

use super::model::{SshInput, SshResultDto, ToolErrorDto, ToolResult};

const MAX_OUTPUT: usize = 2 * 1024 * 1024;

struct AcceptAnyHostKey;

impl client::Handler for AcceptAnyHostKey {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // This is a diagnostic console: the user explicitly targets the host and
        // receives the captured output only. Host key pinning is left to the OS
        // SSH client for interactive sessions.
        Ok(true)
    }
}

pub async fn execute(input: SshInput) -> ToolResult<SshResultDto> {
    let host = input.host.trim().to_owned();
    if host.is_empty() {
        return Err(ToolErrorDto::new("invalidHost", "Host is required"));
    }
    if input.username.trim().is_empty() {
        return Err(ToolErrorDto::new("invalidUser", "Username is required"));
    }
    let timeout_ms = input.timeout_ms.clamp(2_000, 60_000);
    let started = Instant::now();
    let future = run(input, host, started);
    match tokio::time::timeout(Duration::from_millis(timeout_ms), future).await {
        Ok(result) => result,
        Err(_) => Err(ToolErrorDto::new(
            "sshTimeout",
            format!("SSH operation exceeded {timeout_ms} ms"),
        )),
    }
}

async fn run(input: SshInput, host: String, started: Instant) -> ToolResult<SshResultDto> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(20)),
        ..client::Config::default()
    });
    let mut session = client::connect(config, (host.as_str(), input.port), AcceptAnyHostKey)
        .await
        .map_err(|error| ToolErrorDto::new("sshConnect", error.to_string()))?;

    let authenticated = match input.auth.as_str() {
        "key" => {
            let path = input
                .key_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ToolErrorDto::new("invalidKey", "Private key path is required"))?;
            let key = load_secret_key(path, input.key_passphrase.as_deref())
                .map_err(|error| ToolErrorDto::new("sshKey", error.to_string()))?;
            let hash = session
                .best_supported_rsa_hash()
                .await
                .map_err(|error| ToolErrorDto::new("sshAuth", error.to_string()))?
                .flatten();
            session
                .authenticate_publickey(
                    input.username.trim(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                )
                .await
                .map_err(|error| ToolErrorDto::new("sshAuth", error.to_string()))?
                .success()
        }
        _ => {
            let password = input.password.as_deref().unwrap_or_default();
            session
                .authenticate_password(input.username.trim(), password)
                .await
                .map_err(|error| ToolErrorDto::new("sshAuth", error.to_string()))?
                .success()
        }
    };

    if !authenticated {
        let _ = session
            .disconnect(Disconnect::ByApplication, "auth failed", "en")
            .await;
        return Err(ToolErrorDto::new(
            "sshAuthFailed",
            "The server rejected the credentials",
        ));
    }

    let command = input.command.trim();
    let command = if command.is_empty() {
        "echo connected"
    } else {
        command
    };

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|error| ToolErrorDto::new("sshChannel", error.to_string()))?;
    channel
        .exec(true, command)
        .await
        .map_err(|error| ToolErrorDto::new("sshExec", error.to_string()))?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_status = None;
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } => {
                if stdout.len() < MAX_OUTPUT {
                    let remaining = MAX_OUTPUT - stdout.len();
                    stdout.extend_from_slice(&data[..data.len().min(remaining)]);
                }
            }
            ChannelMsg::ExtendedData { data, ext } => {
                if ext == 1 && stderr.len() < MAX_OUTPUT {
                    let remaining = MAX_OUTPUT - stderr.len();
                    stderr.extend_from_slice(&data[..data.len().min(remaining)]);
                }
            }
            ChannelMsg::ExitStatus { exit_status: code } => exit_status = Some(code),
            ChannelMsg::ExitSignal { .. } => {}
            ChannelMsg::Eof | ChannelMsg::Close => {}
            _ => {}
        }
    }

    let _ = session
        .disconnect(Disconnect::ByApplication, "", "en")
        .await;

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(SshResultDto {
        host,
        port: input.port,
        username: input.username.trim().to_owned(),
        authenticated,
        exit_status,
        truncated: stdout.len() >= MAX_OUTPUT || stderr.len() >= MAX_OUTPUT,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        elapsed_ms: (elapsed_ms * 10.0).round() / 10.0,
    })
}
