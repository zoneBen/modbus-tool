use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Multipart, Path as AxumPath, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Redirect, Response};
use axum::routing::{get, post};
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;

use super::model::{
    FileServerFileDto, FileServerRequestDto, FileServerStartInput, FileServerStatusDto,
    ToolErrorDto, ToolResult,
};

const MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LISTED_FILES: usize = 500;
const MAX_REQUESTS: usize = 200;

struct RequestLog {
    timestamp_ms: u64,
    remote: String,
    method: String,
    path: String,
    status: u16,
    bytes: u64,
}

#[derive(Default)]
struct LogState {
    entries: VecDeque<RequestLog>,
    uploads: u64,
    downloads: u64,
}

struct ServerContext {
    root: PathBuf,
    logs: Arc<Mutex<LogState>>,
}

struct RunningServer {
    root: PathBuf,
    host: String,
    port: u16,
    urls: Vec<String>,
    started_at: u64,
    logs: Arc<Mutex<LogState>>,
    shutdown: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

#[derive(Default)]
pub struct FileServerManager {
    inner: Mutex<Option<RunningServer>>,
}

impl FileServerManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, input: FileServerStartInput) -> ToolResult<FileServerStatusDto> {
        let root = PathBuf::from(input.root.trim());
        if input.root.trim().is_empty() {
            return Err(ToolErrorDto::new("invalidRoot", "Select a folder to share"));
        }
        let root = tokio::fs::canonicalize(&root)
            .await
            .map_err(|error| ToolErrorDto::new("invalidRoot", error.to_string()))?;
        let metadata = tokio::fs::metadata(&root)
            .await
            .map_err(|error| ToolErrorDto::new("invalidRoot", error.to_string()))?;
        if !metadata.is_dir() {
            return Err(ToolErrorDto::new(
                "notADirectory",
                "Selected path is not a folder",
            ));
        }
        if input.port == 0 {
            return Err(ToolErrorDto::new(
                "invalidPort",
                "Port must be between 1 and 65535",
            ));
        }
        let (host, bind_ip) = normalize_host(&input.host)?;
        if self.is_running() {
            return Err(ToolErrorDto::new(
                "serverAlreadyRunning",
                "The file server is already running",
            ));
        }

        let listener = tokio::net::TcpListener::bind(SocketAddr::new(bind_ip, input.port))
            .await
            .map_err(|error| ToolErrorDto::new("bindFailed", error.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|error| ToolErrorDto::new("bindFailed", error.to_string()))?
            .port();
        let logs = Arc::new(Mutex::new(LogState::default()));
        let context = Arc::new(ServerContext {
            root: root.clone(),
            logs: logs.clone(),
        });
        let app = Router::new()
            .route("/", get(index))
            .route("/api/files", get(api_files))
            .route("/download/{name}", get(download))
            .route("/upload", post(upload))
            .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES as usize))
            .with_state(context);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            let server = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            if let Err(error) = server.await {
                tracing::warn!(%error, "file server stopped with an error");
            }
        });
        let running = RunningServer {
            root: root.clone(),
            host: host.clone(),
            port,
            urls: build_urls(&host, port),
            started_at: now_ms(),
            logs,
            shutdown: Some(shutdown_tx),
            handle: Some(handle),
        };
        {
            let mut guard = self.lock();
            if guard.is_some() {
                let mut running = running;
                if let Some(sender) = running.shutdown.take() {
                    let _ = sender.send(());
                }
                if let Some(handle) = running.handle.take() {
                    handle.abort();
                }
                return Err(ToolErrorDto::new(
                    "serverAlreadyRunning",
                    "The file server is already running",
                ));
            }
            *guard = Some(running);
        }
        Ok(self.snapshot().await)
    }

    pub async fn stop(&self) -> FileServerStatusDto {
        let running = { self.lock().take() };
        if let Some(mut running) = running {
            if let Some(sender) = running.shutdown.take() {
                let _ = sender.send(());
            }
            if let Some(handle) = running.handle.take() {
                let abort = handle.abort_handle();
                if tokio::time::timeout(std::time::Duration::from_secs(2), handle)
                    .await
                    .is_err()
                {
                    abort.abort();
                }
            }
        }
        self.snapshot().await
    }

    pub async fn snapshot(&self) -> FileServerStatusDto {
        let state = {
            let guard = self.lock();
            guard.as_ref().map(|running| {
                (
                    running.root.clone(),
                    running.host.clone(),
                    running.port,
                    running.urls.clone(),
                    running.started_at,
                    running.logs.clone(),
                )
            })
        };
        let Some((root, host, port, urls, started_at, logs)) = state else {
            return FileServerStatusDto {
                running: false,
                root: None,
                host: None,
                port: None,
                urls: Vec::new(),
                started_at: None,
                uploads: 0,
                downloads: 0,
                files: Vec::new(),
                requests: Vec::new(),
            };
        };
        let files = list_files(&root).await;
        let (uploads, downloads, requests) = {
            let guard = logs.lock().unwrap_or_else(|poison| poison.into_inner());
            (
                guard.uploads,
                guard.downloads,
                guard
                    .entries
                    .iter()
                    .map(|entry| FileServerRequestDto {
                        timestamp_ms: entry.timestamp_ms,
                        remote: entry.remote.clone(),
                        method: entry.method.clone(),
                        path: entry.path.clone(),
                        status: entry.status,
                        bytes: entry.bytes,
                    })
                    .collect(),
            )
        };
        FileServerStatusDto {
            running: true,
            root: Some(root.to_string_lossy().into_owned()),
            host: Some(host),
            port: Some(port),
            urls,
            started_at: Some(started_at),
            uploads,
            downloads,
            files,
            requests,
        }
    }

    pub async fn shutdown(&self) {
        let _ = self.stop().await;
    }

    fn is_running(&self) -> bool {
        self.lock().is_some()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<RunningServer>> {
        self.inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

fn normalize_host(value: &str) -> ToolResult<(String, IpAddr)> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("localhost") {
        return Ok(("127.0.0.1".to_owned(), IpAddr::V4(Ipv4Addr::LOCALHOST)));
    }
    let address: IpAddr = value
        .parse()
        .map_err(|_| ToolErrorDto::new("invalidHost", "Enter a valid listening address"))?;
    Ok((address.to_string(), address))
}

fn build_urls(host: &str, port: u16) -> Vec<String> {
    let mut urls = vec![format!("http://127.0.0.1:{port}")];
    if host == "0.0.0.0" {
        if let Some(address) = detected_ipv4() {
            let url = format!("http://{address}:{port}");
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    } else if host != "127.0.0.1" && host != "::1" {
        urls.clear();
        urls.push(format!("http://{host}:{port}"));
    }
    urls
}

fn detected_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(address) => Some(address),
        IpAddr::V6(_) => None,
    }
}

async fn list_files(root: &Path) -> Vec<FileServerFileDto> {
    let mut files = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(root).await else {
        return files;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if files.len() >= MAX_LISTED_FILES {
            break;
        }
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let Ok(metadata) = entry.metadata().await else {
            continue;
        };
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
        files.push(FileServerFileDto {
            name: entry.file_name().to_string_lossy().into_owned(),
            size: metadata.len(),
            modified_ms,
        });
    }
    files.sort_by_key(|file| file.name.to_lowercase());
    files
}

async fn index(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
) -> Response {
    let files = list_files(&context.root).await;
    let total_bytes: u64 = files.iter().map(|file| file.size).sum();
    let mut rows = String::new();
    for file in &files {
        let encoded = urlencoding::encode(&file.name);
        rows.push_str(&format!(
            r#"<tr><td class="name"><a href="/download/{encoded}">{}</a></td><td class="size">{}</td></tr>"#,
            escape_html(&file.name),
            format_size(file.size)
        ));
    }
    if rows.is_empty() {
        rows.push_str(r#"<tr><td colspan="2" class="empty">暂无文件，使用上方表单上传</td></tr>"#);
    }
    let html = format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Modbus Tool · 文件交换</title>
<style>{style}</style>
</head>
<body>
<main class="shell">
  <header class="head">
    <div>
      <p class="eyebrow">MODBUS TOOL · LAN EXCHANGE</p>
      <h1>局域网文件交换</h1>
      <p class="sub">同一网络内的设备可通过本页上传与下载共享文件夹中的文件。</p>
    </div>
    <div class="badge">HTTP · 无访问密码</div>
  </header>
  <section class="warn">注意：本服务未设置访问密码，同一局域网内任何能访问此地址的人都可以上传或下载文件。</section>
  <section class="card">
    <div class="card-head">
      <h2>上传文件</h2>
      <span>{count} 个文件 · {total}</span>
    </div>
    <form action="/upload" method="post" enctype="multipart/form-data">
      <label class="drop">
        <input type="file" name="file" multiple required>
        <span>选择文件后点击上传</span>
      </label>
      <button type="submit">上传</button>
    </form>
  </section>
  <section class="card">
    <div class="card-head">
      <h2>共享文件</h2>
      <span>{count} 个文件</span>
    </div>
    <table>
      <thead><tr><th>文件名</th><th>大小</th></tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </section>
  <footer>Modbus Tool · 局域网文件交换</footer>
</main>
</body>
</html>"#,
        style = PAGE_STYLE,
        count = files.len(),
        total = format_size(total_bytes),
        rows = rows,
    );
    record(&context, &remote, "GET", "/", 200, html.len() as u64);
    Html(html).into_response()
}

async fn api_files(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
) -> Response {
    let files = list_files(&context.root).await;
    record(&context, &remote, "GET", "/api/files", 200, 0);
    Json(files).into_response()
}

async fn download(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    AxumPath(name): AxumPath<String>,
) -> Response {
    let Some(name) = sanitize_request_name(&name) else {
        record(&context, &remote, "GET", "/download", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "无效的文件名");
    };
    let path = context.root.join(&name);
    let path = match tokio::fs::canonicalize(&path).await {
        Ok(path) if path.starts_with(&context.root) => path,
        _ => {
            record(
                &context,
                &remote,
                "GET",
                &format!("/download/{name}"),
                404,
                0,
            );
            return error_response(StatusCode::NOT_FOUND, "文件不存在");
        }
    };
    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => {
            record(
                &context,
                &remote,
                "GET",
                &format!("/download/{name}"),
                404,
                0,
            );
            return error_response(StatusCode::NOT_FOUND, "文件不存在");
        }
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(file) => file,
        Err(_) => {
            record(
                &context,
                &remote,
                "GET",
                &format!("/download/{name}"),
                500,
                0,
            );
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "读取文件失败");
        }
    };
    let size = metadata.len();
    let encoded = urlencoding::encode(&name);
    let disposition = format!("attachment; filename*=UTF-8''{encoded}");
    let body = Body::from_stream(ReaderStream::new(file));
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, size)
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition)
                .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
        )
        .body(body);
    record(
        &context,
        &remote,
        "GET",
        &format!("/download/{name}"),
        200,
        size,
    );
    match response {
        Ok(response) => response,
        Err(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "构建下载响应失败"),
    }
}

async fn upload(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> Response {
    let mut uploaded = 0u32;
    let mut total_bytes = 0u64;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(%error, "failed to read multipart upload");
                record(&context, &remote, "POST", "/upload", 400, total_bytes);
                return error_response(StatusCode::BAD_REQUEST, "上传数据无效");
            }
        };
        let Some(file_name) = field.file_name().map(str::to_owned) else {
            continue;
        };
        let Some(name) = sanitize_upload_name(&file_name) else {
            continue;
        };
        let (path, mut file) = match create_unique(&context.root, &name).await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%error, "failed to create uploaded file");
                record(&context, &remote, "POST", "/upload", 500, total_bytes);
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, "创建文件失败");
            }
        };
        let mut field = field;
        let mut write_failed = false;
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    total_bytes += chunk.len() as u64;
                    if total_bytes > MAX_UPLOAD_BYTES {
                        write_failed = true;
                        break;
                    }
                    if file.write_all(&chunk).await.is_err() {
                        write_failed = true;
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    write_failed = true;
                    break;
                }
            }
        }
        let _ = file.flush().await;
        if write_failed {
            drop(file);
            let _ = tokio::fs::remove_file(&path).await;
            record(&context, &remote, "POST", "/upload", 400, total_bytes);
            return error_response(StatusCode::BAD_REQUEST, "写入上传文件失败或文件过大");
        }
        uploaded += 1;
    }
    if uploaded == 0 {
        record(&context, &remote, "POST", "/upload", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "没有选择文件");
    }
    record(&context, &remote, "POST", "/upload", 303, total_bytes);
    Redirect::to("/").into_response()
}

fn record(
    context: &ServerContext,
    remote: &SocketAddr,
    method: &str,
    path: &str,
    status: u16,
    bytes: u64,
) {
    let mut logs = context
        .logs
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if path == "/upload" && status < 400 {
        logs.uploads += 1;
    }
    if method == "GET" && path.starts_with("/download/") && status < 400 {
        logs.downloads += 1;
    }
    logs.entries.push_front(RequestLog {
        timestamp_ms: now_ms(),
        remote: remote.ip().to_string(),
        method: method.to_owned(),
        path: path.to_owned(),
        status,
        bytes,
    });
    while logs.entries.len() > MAX_REQUESTS {
        logs.entries.pop_back();
    }
}

fn sanitize_request_name(raw: &str) -> Option<String> {
    let base = raw.rsplit(['/', '\\']).next()?.trim();
    if base.is_empty() || base == "." || base == ".." || base.contains('\0') {
        return None;
    }
    Some(base.to_owned())
}

fn sanitize_upload_name(raw: &str) -> Option<String> {
    let base = raw.rsplit(['/', '\\']).next()?.trim();
    let base = base.trim_end_matches(['.', ' ']);
    if base.is_empty() || base == "." || base == ".." {
        return None;
    }
    let mut cleaned: String = base
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(character, '<' | '>' | ':' | '"' | '|' | '?' | '*')
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    if cleaned.chars().count() > 180 {
        cleaned = cleaned.chars().take(180).collect();
    }
    let upper = cleaned.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        cleaned.insert(0, '_');
    }
    Some(cleaned)
}

async fn create_unique(root: &Path, name: &str) -> std::io::Result<(PathBuf, tokio::fs::File)> {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_owned());
    let extension = path
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();
    for index in 0..1000 {
        let candidate = if index == 0 {
            root.join(name)
        } else {
            root.join(format!("{stem} ({index}){extension}"))
        };
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
            .await
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "too many duplicate file names",
    ))
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, message.to_owned()).into_response()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

const PAGE_STYLE: &str = r#"
:root { color-scheme: light; }
* { box-sizing: border-box; }
body { margin: 0; background: #f2f4f7; color: #1c2530; font-family: "Segoe UI", system-ui, sans-serif; }
.shell { max-width: 860px; margin: 0 auto; padding: 40px 20px 60px; }
.head { display: flex; gap: 20px; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; }
.eyebrow { margin: 0 0 8px; font: 11px/1.2 ui-monospace, monospace; letter-spacing: .14em; color: #6b7785; }
h1 { margin: 0; font-size: 30px; letter-spacing: -.02em; }
.sub { margin: 8px 0 0; color: #5b6674; font-size: 13px; }
.badge { padding: 6px 10px; border-radius: 999px; background: #e5edff; color: #2456c7; font-size: 11px; font-weight: 600; }
.warn { margin: 22px 0 16px; padding: 12px 14px; border: 1px solid #f1d3a6; background: #fff8ea; color: #8a5a12; border-radius: 12px; font-size: 12px; line-height: 1.6; }
.card { background: #fff; border: 1px solid #e1e6ed; border-radius: 16px; padding: 18px; margin-top: 16px; box-shadow: 0 10px 30px rgba(21, 37, 58, .05); }
.card-head { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; margin-bottom: 14px; }
h2 { margin: 0; font-size: 15px; }
.card-head span { color: #6b7785; font-size: 11px; font-family: ui-monospace, monospace; }
form { display: flex; gap: 10px; flex-wrap: wrap; align-items: center; }
.drop { flex: 1; min-width: 240px; display: flex; align-items: center; gap: 10px; padding: 14px; border: 1px dashed #c3ccd8; border-radius: 12px; background: #fafbfc; font-size: 12px; color: #5b6674; }
.drop input { max-width: 100%; }
button { border: 0; border-radius: 10px; padding: 11px 20px; background: #2456c7; color: #fff; font-size: 12px; font-weight: 600; cursor: pointer; }
button:hover { background: #1c45a4; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th { text-align: left; padding: 8px 10px; color: #6b7785; font-weight: 600; border-bottom: 1px solid #e1e6ed; }
td { padding: 10px; border-bottom: 1px solid #eef1f5; }
tr:last-child td { border-bottom: 0; }
.name a { color: #1c2530; text-decoration: none; word-break: break-all; }
.name a:hover { color: #2456c7; text-decoration: underline; }
.size { color: #6b7785; white-space: nowrap; font-family: ui-monospace, monospace; }
.empty { text-align: center; color: #8b96a4; padding: 26px 10px; }
footer { margin-top: 22px; color: #8b96a4; font-size: 10.5px; font-family: ui-monospace, monospace; word-break: break-all; }
@media (max-width: 540px) { .shell { padding: 24px 14px 40px; } h1 { font-size: 24px; } }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_upload_names() {
        assert_eq!(
            sanitize_upload_name("..\\evil.txt").as_deref(),
            Some("evil.txt")
        );
        assert_eq!(
            sanitize_upload_name("a<b>.txt").as_deref(),
            Some("a_b_.txt")
        );
        assert!(sanitize_upload_name("...").is_none());
        assert_eq!(sanitize_upload_name("CON.txt").as_deref(), Some("_CON.txt"));
    }

    #[test]
    fn strips_directories_from_download_names() {
        assert_eq!(
            sanitize_request_name("..\\secret").as_deref(),
            Some("secret")
        );
        assert_eq!(
            sanitize_request_name("../../etc/passwd").as_deref(),
            Some("passwd")
        );
        assert!(sanitize_request_name("..").is_none());
        assert_eq!(
            sanitize_request_name("file.txt").as_deref(),
            Some("file.txt")
        );
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2.0 KB");
    }

    #[test]
    fn escapes_html() {
        assert_eq!(escape_html("<a&b>"), "&lt;a&amp;b&gt;");
    }

    #[tokio::test]
    async fn serves_lists_and_accepts_uploads() {
        let directory = tempfile::tempdir().expect("temp dir");
        tokio::fs::write(directory.path().join("hello.txt"), b"hello")
            .await
            .expect("seed file");
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("free port");
            listener.local_addr().expect("local addr").port()
        };
        let manager = FileServerManager::new();
        let status = manager
            .start(FileServerStartInput {
                root: directory.path().to_string_lossy().into_owned(),
                host: "127.0.0.1".to_owned(),
                port,
            })
            .await
            .expect("start server");
        assert!(status.running);
        assert_eq!(status.files.len(), 1);

        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("http client");
        let base = format!("http://127.0.0.1:{port}");
        let download = client
            .get(format!("{base}/download/hello.txt"))
            .send()
            .await
            .expect("download");
        assert_eq!(download.status(), 200);
        assert_eq!(download.text().await.expect("body"), "hello");

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(b"world".to_vec()).file_name("upload.txt"),
        );
        let upload = client
            .post(format!("{base}/upload"))
            .multipart(form)
            .send()
            .await
            .expect("upload");
        assert!(upload.status().is_success());
        let files = client
            .get(format!("{base}/api/files"))
            .send()
            .await
            .expect("list")
            .json::<Vec<FileServerFileDto>>()
            .await
            .expect("json");
        assert!(files.iter().any(|file| file.name == "upload.txt"));
        assert!(
            tokio::fs::metadata(directory.path().join("upload.txt"))
                .await
                .expect("uploaded file")
                .is_file()
        );

        let stopped = manager.stop().await;
        assert!(!stopped.running);
    }
}
