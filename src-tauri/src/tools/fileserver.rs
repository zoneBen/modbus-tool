use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, DefaultBodyLimit, Multipart, Path as AxumPath, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;

use super::model::{
    FileServerEntryDto, FileServerRequestDto, FileServerStartInput, FileServerStatusDto,
    ToolErrorDto, ToolResult,
};

const MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LISTED_ENTRIES: usize = 500;
const MAX_REQUESTS: usize = 200;

#[derive(Debug, Default, Deserialize)]
struct BrowseQuery {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MkdirForm {
    name: String,
}

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
    base_url: String,
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
        let urls = build_urls(&host, port);
        let base_url = urls
            .iter()
            .find(|url| !url.contains("127.0.0.1") && !url.contains("localhost"))
            .cloned()
            .or_else(|| urls.first().cloned())
            .unwrap_or_else(|| format!("http://127.0.0.1:{port}"));
        let context = Arc::new(ServerContext {
            root: root.clone(),
            logs: logs.clone(),
            base_url,
        });
        let app = Router::new()
            .route("/", get(index))
            .route("/api/files", get(api_files))
            .route("/download/{*path}", get(download))
            .route("/upload", post(upload))
            .route("/mkdir", post(mkdir))
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
            urls,
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
        self.snapshot("").await
    }

    pub async fn stop(&self) -> ToolResult<FileServerStatusDto> {
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
        self.snapshot("").await
    }

    pub async fn snapshot(&self, path: &str) -> ToolResult<FileServerStatusDto> {
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
            return Ok(FileServerStatusDto {
                running: false,
                root: None,
                host: None,
                port: None,
                urls: Vec::new(),
                started_at: None,
                path: String::new(),
                parent: None,
                uploads: 0,
                downloads: 0,
                entries: Vec::new(),
                requests: Vec::new(),
            });
        };
        let normalized = normalize_relative(path)
            .ok_or_else(|| ToolErrorDto::new("invalidPath", "Invalid folder path"))?;
        let directory = resolve_directory(&root, &normalized).await?;
        let entries = list_entries(&directory, &normalized).await;
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
        Ok(FileServerStatusDto {
            running: true,
            root: Some(root.to_string_lossy().into_owned()),
            host: Some(host),
            port: Some(port),
            urls,
            started_at: Some(started_at),
            path: normalized.clone(),
            parent: parent_path(&normalized),
            uploads,
            downloads,
            entries,
            requests,
        })
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

async fn list_entries(directory: &Path, relative: &str) -> Vec<FileServerEntryDto> {
    let mut entries = Vec::new();
    let Ok(mut read_dir) = tokio::fs::read_dir(directory).await else {
        return entries;
    };
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        if entries.len() >= MAX_LISTED_ENTRIES {
            break;
        }
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        // Skip symlinks so a link cannot escape the shared root.
        if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
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
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push(FileServerEntryDto {
            path: join_relative(relative, &name),
            name,
            is_dir: file_type.is_dir(),
            size: if file_type.is_dir() {
                0
            } else {
                metadata.len()
            },
            modified_ms,
        });
    }
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    entries
}

fn normalize_relative(value: &str) -> Option<String> {
    let value = value.replace('\\', "/");
    let mut segments = Vec::new();
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." || segment.contains('\0') || segment.contains(':') {
            return None;
        }
        segments.push(segment);
    }
    Some(segments.join("/"))
}

fn join_relative(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_owned()
    } else {
        format!("{base}/{name}")
    }
}

fn parent_path(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let mut segments: Vec<&str> = value.split('/').collect();
    segments.pop();
    Some(segments.join("/"))
}

async fn resolve_directory(root: &Path, relative: &str) -> ToolResult<PathBuf> {
    if relative.is_empty() {
        return Ok(root.to_path_buf());
    }
    let candidate = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    let resolved = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|_| ToolErrorDto::new("invalidPath", "Folder does not exist"))?;
    if !resolved.starts_with(root) {
        return Err(ToolErrorDto::new(
            "invalidPath",
            "Folder is outside the shared root",
        ));
    }
    let metadata = tokio::fs::metadata(&resolved)
        .await
        .map_err(|_| ToolErrorDto::new("invalidPath", "Folder does not exist"))?;
    if !metadata.is_dir() {
        return Err(ToolErrorDto::new(
            "notADirectory",
            "The selected path is not a folder",
        ));
    }
    Ok(resolved)
}

async fn resolve_file(root: &Path, relative: &str) -> ToolResult<PathBuf> {
    if relative.is_empty() {
        return Err(ToolErrorDto::new("invalidPath", "Invalid file path"));
    }
    let candidate = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    let resolved = tokio::fs::canonicalize(&candidate)
        .await
        .map_err(|_| ToolErrorDto::new("notFound", "File does not exist"))?;
    if !resolved.starts_with(root) {
        return Err(ToolErrorDto::new(
            "invalidPath",
            "File is outside the shared root",
        ));
    }
    let metadata = tokio::fs::metadata(&resolved)
        .await
        .map_err(|_| ToolErrorDto::new("notFound", "File does not exist"))?;
    if !metadata.is_file() {
        return Err(ToolErrorDto::new(
            "notAFile",
            "The selected path is not a file",
        ));
    }
    Ok(resolved)
}

async fn index(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Query(query): Query<BrowseQuery>,
) -> Response {
    let Some(relative) = normalize_relative(query.path.as_deref().unwrap_or("")) else {
        record(&context, &remote, "GET", "/", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "目录路径无效");
    };
    let directory = match resolve_directory(&context.root, &relative).await {
        Ok(directory) => directory,
        Err(error) => {
            record(&context, &remote, "GET", "/", 404, 0);
            return error_response(StatusCode::NOT_FOUND, &error.detail);
        }
    };
    let entries = list_entries(&directory, &relative).await;
    let file_count = entries.iter().filter(|entry| !entry.is_dir).count();
    let dir_count = entries.len() - file_count;
    let total_bytes: u64 = entries.iter().map(|entry| entry.size).sum();
    let query_path = urlencoding::encode(&relative);
    let upload_action = if relative.is_empty() {
        "/upload".to_owned()
    } else {
        format!("/upload?path={query_path}")
    };
    let mkdir_action = if relative.is_empty() {
        "/mkdir".to_owned()
    } else {
        format!("/mkdir?path={query_path}")
    };
    let mut rows = String::new();
    for entry in &entries {
        let encoded_path = encode_relative(&entry.path);
        if entry.is_dir {
            let browse = format!("/?path={}", urlencoding::encode(&entry.path));
            rows.push_str(&format!(
                r#"<tr><td class="name"><a class="folder" href="{}">{}</a></td><td class="meta">—</td><td class="meta"><span class="localtime" data-ts="{}">—</span></td><td class="actions"></td></tr>"#,
                escape_html(&browse),
                escape_html(&entry.name),
                entry.modified_ms
            ));
        } else {
            let download = format!("/download/{encoded_path}");
            rows.push_str(&format!(
                r#"<tr><td class="name"><a href="{}">{}</a></td><td class="meta">{}</td><td class="meta"><span class="localtime" data-ts="{}">—</span></td><td class="actions"><button type="button" class="copy" data-path="{}">复制链接</button></td></tr>"#,
                escape_html(&download),
                escape_html(&entry.name),
                format_size(entry.size),
                entry.modified_ms,
                escape_html(&encoded_path)
            ));
        }
    }
    if rows.is_empty() {
        rows.push_str(
            r#"<tr><td colspan="4" class="empty">当前文件夹为空，使用上方表单上传文件</td></tr>"#,
        );
    }
    let mut crumbs = r#"<a href="/">根目录</a>"#.to_owned();
    let mut cumulative = String::new();
    for segment in relative.split('/').filter(|segment| !segment.is_empty()) {
        cumulative = join_relative(&cumulative, segment);
        crumbs.push_str(&format!(
            r#"<span class="sep">/</span><a href="/?path={}">{}</a>"#,
            urlencoding::encode(&cumulative),
            escape_html(segment)
        ));
    }
    let parent_link = parent_path(&relative)
        .map(|parent| {
            let href = if parent.is_empty() {
                "/".to_owned()
            } else {
                format!("/?path={}", urlencoding::encode(&parent))
            };
            format!(
                r#"<a class="up" href="{}">↑ 上一级</a>"#,
                escape_html(&href)
            )
        })
        .unwrap_or_default();
    let html = format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Modbus Tool · 文件交换</title>
<style>{style}</style>
</head>
<body data-base="{base}">
<main class="shell">
  <header class="head">
    <div>
      <p class="eyebrow">MODBUS TOOL · LAN EXCHANGE</p>
      <h1>局域网文件交换</h1>
      <p class="sub">浏览共享文件夹，上传或下载文件，并把文件链接发送给同一网络中的其他设备。</p>
    </div>
    <div class="badge">HTTP · 无访问密码</div>
  </header>
  <section class="warn">注意：本服务未设置访问密码，同一局域网内任何能访问此地址的人都可以浏览、上传或下载文件。</section>
  <nav class="crumbs">{crumbs}</nav>
  <section class="card">
    <div class="card-head">
      <h2>上传到当前文件夹</h2>
      <span>{dir_count} 个文件夹 · {file_count} 个文件 · {total}</span>
    </div>
    <form action="{upload_action}" method="post" enctype="multipart/form-data">
      <label class="drop">
        <input type="file" name="file" multiple required>
        <span>选择文件后点击上传</span>
      </label>
      <button type="submit">上传</button>
    </form>
    <form class="mkdir" action="{mkdir_action}" method="post">
      <input type="text" name="name" maxlength="180" placeholder="新建文件夹名称" required>
      <button type="submit">新建文件夹</button>
    </form>
  </section>
  <section class="card">
    <div class="card-head">
      <h2>当前文件夹</h2>
      <span>{parent_link}</span>
    </div>
    <div class="table-wrap">
      <table>
        <thead><tr><th>名称</th><th>大小</th><th>修改时间</th><th>操作</th></tr></thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  </section>
  <footer>Modbus Tool · 局域网文件交换</footer>
</main>
<div id="toast" class="toast" role="status" aria-live="polite"></div>
<script>{script}</script>
</body>
</html>"#,
        style = PAGE_STYLE,
        script = PAGE_SCRIPT,
        base = escape_html(&context.base_url),
        crumbs = crumbs,
        dir_count = dir_count,
        file_count = file_count,
        total = format_size(total_bytes),
        upload_action = escape_html(&upload_action),
        mkdir_action = escape_html(&mkdir_action),
        parent_link = parent_link,
        rows = rows,
    );
    let log_path = if relative.is_empty() {
        "/".to_owned()
    } else {
        format!("/?path={query_path}")
    };
    record(&context, &remote, "GET", &log_path, 200, html.len() as u64);
    Html(html).into_response()
}

async fn api_files(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Query(query): Query<BrowseQuery>,
) -> Response {
    let Some(relative) = normalize_relative(query.path.as_deref().unwrap_or("")) else {
        record(&context, &remote, "GET", "/api/files", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "目录路径无效");
    };
    let directory = match resolve_directory(&context.root, &relative).await {
        Ok(directory) => directory,
        Err(error) => {
            record(&context, &remote, "GET", "/api/files", 404, 0);
            return error_response(StatusCode::NOT_FOUND, &error.detail);
        }
    };
    let entries = list_entries(&directory, &relative).await;
    record(&context, &remote, "GET", "/api/files", 200, 0);
    Json(entries).into_response()
}

async fn download(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    AxumPath(raw_path): AxumPath<String>,
) -> Response {
    let Some(relative) = normalize_relative(&raw_path) else {
        record(&context, &remote, "GET", "/download", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "无效的文件路径");
    };
    if relative.is_empty() {
        record(&context, &remote, "GET", "/download", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "无效的文件路径");
    }
    let path = match resolve_file(&context.root, &relative).await {
        Ok(path) => path,
        Err(_) => {
            record(
                &context,
                &remote,
                "GET",
                &format!("/download/{relative}"),
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
                &format!("/download/{relative}"),
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
                &format!("/download/{relative}"),
                500,
                0,
            );
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "读取文件失败");
        }
    };
    let size = metadata.len();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        urlencoding::encode(&file_name)
    );
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
        &format!("/download/{relative}"),
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
    Query(query): Query<BrowseQuery>,
    mut multipart: Multipart,
) -> Response {
    let Some(relative) = normalize_relative(query.path.as_deref().unwrap_or("")) else {
        record(&context, &remote, "POST", "/upload", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "目标文件夹无效");
    };
    let target = match resolve_directory(&context.root, &relative).await {
        Ok(target) => target,
        Err(error) => {
            record(&context, &remote, "POST", "/upload", 400, 0);
            return error_response(StatusCode::BAD_REQUEST, &error.detail);
        }
    };
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
        let (path, mut file) = match create_unique(&target, &name).await {
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
    Redirect::to(&browse_url(&relative)).into_response()
}

async fn mkdir(
    State(context): State<Arc<ServerContext>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Query(query): Query<BrowseQuery>,
    Form(form): Form<MkdirForm>,
) -> Response {
    let Some(relative) = normalize_relative(query.path.as_deref().unwrap_or("")) else {
        record(&context, &remote, "POST", "/mkdir", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "目标文件夹无效");
    };
    let target = match resolve_directory(&context.root, &relative).await {
        Ok(target) => target,
        Err(error) => {
            record(&context, &remote, "POST", "/mkdir", 400, 0);
            return error_response(StatusCode::BAD_REQUEST, &error.detail);
        }
    };
    let Some(name) = sanitize_upload_name(&form.name) else {
        record(&context, &remote, "POST", "/mkdir", 400, 0);
        return error_response(StatusCode::BAD_REQUEST, "文件夹名称无效");
    };
    if let Err(error) = tokio::fs::create_dir(target.join(&name)).await
        && error.kind() != std::io::ErrorKind::AlreadyExists
    {
        tracing::warn!(%error, "failed to create folder");
        record(&context, &remote, "POST", "/mkdir", 500, 0);
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "创建文件夹失败");
    }
    record(&context, &remote, "POST", "/mkdir", 303, 0);
    Redirect::to(&browse_url(&relative)).into_response()
}

fn browse_url(relative: &str) -> String {
    if relative.is_empty() {
        "/".to_owned()
    } else {
        format!("/?path={}", urlencoding::encode(relative))
    }
}

fn encode_relative(value: &str) -> String {
    value
        .split('/')
        .map(urlencoding::encode)
        .collect::<Vec<_>>()
        .join("/")
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
.crumbs { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; margin: 18px 0 2px; color: #6b7785; font-size: 12px; }
.crumbs a { color: #2456c7; text-decoration: none; }
.crumbs a:hover { text-decoration: underline; }
.crumbs .sep { color: #a7b1bd; }
.card { background: #fff; border: 1px solid #e1e6ed; border-radius: 16px; padding: 18px; margin-top: 16px; box-shadow: 0 10px 30px rgba(21, 37, 58, .05); }
.card-head { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; margin-bottom: 14px; }
h2 { margin: 0; font-size: 15px; }
.card-head span { color: #6b7785; font-size: 11px; font-family: ui-monospace, monospace; }
.card-head .up { color: #2456c7; text-decoration: none; }
.card-head .up:hover { text-decoration: underline; }
form { display: flex; gap: 10px; flex-wrap: wrap; align-items: center; }
.mkdir { margin-top: 12px; padding-top: 12px; border-top: 1px dashed #e1e6ed; }
.mkdir input { flex: 1; min-width: 180px; padding: 9px 12px; border: 1px solid #c3ccd8; border-radius: 10px; background: #fff; font-size: 12px; color: #1c2530; }
.drop { flex: 1; min-width: 240px; display: flex; align-items: center; gap: 10px; padding: 14px; border: 1px dashed #c3ccd8; border-radius: 12px; background: #fafbfc; font-size: 12px; color: #5b6674; }
.drop input { max-width: 100%; }
button { border: 0; border-radius: 10px; padding: 11px 20px; background: #2456c7; color: #fff; font-size: 12px; font-weight: 600; cursor: pointer; }
button:hover { background: #1c45a4; }
.table-wrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th { text-align: left; padding: 8px 10px; color: #6b7785; font-weight: 600; border-bottom: 1px solid #e1e6ed; white-space: nowrap; }
td { padding: 10px; border-bottom: 1px solid #eef1f5; }
tr:last-child td { border-bottom: 0; }
.name a { color: #1c2530; text-decoration: none; word-break: break-all; }
.name a:hover { color: #2456c7; text-decoration: underline; }
.name a.folder { font-weight: 600; }
.name a.folder::before { content: "📁"; margin-right: 6px; }
.meta { color: #6b7785; white-space: nowrap; font-family: ui-monospace, monospace; }
.actions { text-align: right; white-space: nowrap; }
.copy { padding: 6px 10px; border: 1px solid #d6e2ff; border-radius: 8px; background: #eef3ff; color: #2456c7; font-size: 11px; }
.copy:hover { background: #dfeaff; }
.copy.copied { border-color: #bfe8d1; background: #e6f7ee; color: #1a7f4b; }
.empty { text-align: center; color: #8b96a4; padding: 26px 10px; }
footer { margin-top: 22px; color: #8b96a4; font-size: 10.5px; font-family: ui-monospace, monospace; word-break: break-all; }
.toast { position: fixed; left: 50%; bottom: 32px; z-index: 20; max-width: calc(100vw - 32px); padding: 12px 20px; border-radius: 999px; background: #1c2530; color: #fff; font-size: 13px; font-weight: 600; opacity: 0; pointer-events: none; transform: translateX(-50%) translateY(16px); transition: opacity .18s ease, transform .18s ease; box-shadow: 0 12px 30px rgba(0,0,0,.22); }
.toast.show { opacity: 1; transform: translateX(-50%) translateY(0); }
.toast.ok { background: #1a7f4b; }
.toast.err { background: #b42318; }
@media (max-width: 540px) { .shell { padding: 24px 14px 40px; } h1 { font-size: 24px; } td, th { padding: 8px 6px; } }
"#;

const PAGE_SCRIPT: &str = r#"
(function () {
  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(text).then(function () { return true; }).catch(function () { return fallbackCopy(text); });
    }
    return Promise.resolve(fallbackCopy(text));
  }
  function fallbackCopy(text) {
    var area = document.createElement('textarea');
    area.value = text;
    area.style.position = 'fixed';
    area.style.top = '-1000px';
    document.body.appendChild(area);
    area.focus();
    area.select();
    var ok = false;
    try { ok = document.execCommand('copy'); } catch (error) { ok = false; }
    area.remove();
    return ok;
  }
  function showToast(message, ok) {
    var toast = document.getElementById('toast');
    toast.textContent = message;
    toast.className = 'toast show ' + (ok ? 'ok' : 'err');
    clearTimeout(window.__modbusToolToast);
    window.__modbusToolToast = setTimeout(function () { toast.className = 'toast'; }, 2200);
  }
  document.querySelectorAll('.localtime').forEach(function (node) {
    var timestamp = Number(node.getAttribute('data-ts'));
    if (!timestamp) return;
    var date = new Date(timestamp);
    node.textContent = date.toLocaleString();
  });
  document.addEventListener('click', function (event) {
    var button = event.target.closest('.copy');
    if (!button) return;
    var path = button.getAttribute('data-path') || '';
    var base = document.body.getAttribute('data-base') || window.location.origin;
    var url = new URL('/download/' + path, base).href;
    copyText(url).then(function (ok) {
      if (ok) {
        button.classList.add('copied');
        button.textContent = '已复制';
        setTimeout(function () {
          button.classList.remove('copied');
          button.textContent = '复制链接';
        }, 1600);
        showToast('文件链接已复制', true);
      } else {
        showToast('复制失败，请长按文件链接手动复制', false);
      }
    });
  });
})();
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
    fn normalizes_relative_paths() {
        assert_eq!(normalize_relative("a\\b/./c").as_deref(), Some("a/b/c"));
        assert_eq!(normalize_relative("folder").as_deref(), Some("folder"));
        assert!(normalize_relative("../secret").is_none());
        assert!(normalize_relative("a/../../b").is_none());
        assert!(normalize_relative("a\0b").is_none());
        assert!(normalize_relative("C:/Windows").is_none());
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
    async fn serves_folders_lists_and_accepts_uploads() {
        let directory = tempfile::tempdir().expect("temp dir");
        tokio::fs::write(directory.path().join("hello.txt"), b"hello")
            .await
            .expect("seed file");
        tokio::fs::create_dir(directory.path().join("sub"))
            .await
            .expect("seed folder");
        tokio::fs::write(directory.path().join("sub").join("nested.txt"), b"nested")
            .await
            .expect("nested file");
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
        assert_eq!(status.entries.len(), 2);
        assert!(status.entries.iter().any(|entry| entry.is_dir));

        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("http client");
        let base = format!("http://127.0.0.1:{port}");
        let page = client
            .get(format!("{base}/"))
            .send()
            .await
            .expect("index")
            .text()
            .await
            .expect("html");
        assert!(page.contains("data-path=\"hello.txt\""));
        assert!(page.contains("复制链接"));

        let download = client
            .get(format!("{base}/download/hello.txt"))
            .send()
            .await
            .expect("download");
        assert_eq!(download.status(), 200);
        assert_eq!(download.text().await.expect("body"), "hello");

        let nested = client
            .get(format!("{base}/download/sub/nested.txt"))
            .send()
            .await
            .expect("nested download");
        assert_eq!(nested.status(), 200);
        assert_eq!(nested.text().await.expect("nested body"), "nested");

        let folder_entries = client
            .get(format!("{base}/api/files?path=sub"))
            .send()
            .await
            .expect("folder list")
            .json::<Vec<FileServerEntryDto>>()
            .await
            .expect("folder json");
        assert_eq!(folder_entries.len(), 1);
        assert_eq!(folder_entries[0].path, "sub/nested.txt");

        let form = reqwest::multipart::Form::new().part(
            "file",
            reqwest::multipart::Part::bytes(b"world".to_vec()).file_name("upload.txt"),
        );
        let upload = client
            .post(format!("{base}/upload?path=sub"))
            .multipart(form)
            .send()
            .await
            .expect("upload");
        assert!(upload.status().is_success());
        assert!(
            tokio::fs::metadata(directory.path().join("sub").join("upload.txt"))
                .await
                .expect("uploaded file")
                .is_file()
        );

        let created = client
            .post(format!("{base}/mkdir?path=sub"))
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("name=new-folder")
            .send()
            .await
            .expect("mkdir");
        assert!(created.status().is_success());
        assert!(
            tokio::fs::metadata(directory.path().join("sub").join("new-folder"))
                .await
                .expect("folder")
                .is_dir()
        );

        let traversal = client
            .get(format!("{base}/download/../package.json"))
            .send()
            .await
            .expect("traversal response");
        assert_ne!(traversal.status(), 200);

        let stopped = manager.stop().await.expect("stop");
        assert!(!stopped.running);
    }
}
