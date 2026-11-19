use std::time::{Duration, Instant};

use reqwest::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use reqwest::{Certificate, Method, Url, redirect};

use super::model::{HttpHeaderDto, HttpRequestInput, HttpResponseDto, ToolErrorDto, ToolResult};

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_CA_BYTES: u64 = 1024 * 1024;

pub async fn request(input: HttpRequestInput) -> ToolResult<HttpResponseDto> {
    let url = Url::parse(input.url.trim())
        .map_err(|error| ToolErrorDto::new("invalidUrl", error.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ToolErrorDto::new(
            "invalidUrl",
            "Only http and https URLs are supported",
        ));
    }
    let method = Method::from_bytes(input.method.trim().to_ascii_uppercase().as_bytes())
        .map_err(|error| ToolErrorDto::new("invalidMethod", error.to_string()))?;
    if input.headers.len() > 100 {
        return Err(ToolErrorDto::new(
            "tooManyHeaders",
            "At most 100 headers are allowed",
        ));
    }

    let timeout_duration = Duration::from_millis(input.timeout_ms.clamp(100, 120_000));
    let mut builder = reqwest::Client::builder()
        .timeout(timeout_duration)
        .redirect(if input.follow_redirects {
            redirect::Policy::limited(10)
        } else {
            redirect::Policy::none()
        });

    if let Some(path) = input
        .ca_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|error| ToolErrorDto::new("caReadFailed", error.to_string()))?;
        if metadata.len() > MAX_CA_BYTES {
            return Err(ToolErrorDto::new("caReadFailed", "CA file is too large"));
        }
        let pem = tokio::fs::read(path)
            .await
            .map_err(|error| ToolErrorDto::new("caReadFailed", error.to_string()))?;
        let certificate = Certificate::from_pem(&pem)
            .map_err(|error| ToolErrorDto::new("invalidCa", error.to_string()))?;
        builder = builder.add_root_certificate(certificate);
    }

    let client = builder
        .build()
        .map_err(|error| ToolErrorDto::new("clientBuildFailed", error.to_string()))?;
    let mut request = client.request(method.clone(), url);
    for header in &input.headers {
        let name = HeaderName::from_bytes(header.name.trim().as_bytes())
            .map_err(|error| ToolErrorDto::new("invalidHeader", error.to_string()))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|error| ToolErrorDto::new("invalidHeader", error.to_string()))?;
        request = request.header(name, value);
    }
    if let Some(body) = input.body.as_deref()
        && !matches!(method, Method::GET | Method::HEAD)
    {
        request = request.body(body.to_owned());
    }

    let started = Instant::now();
    let response = request
        .send()
        .await
        .map_err(|error| map_request_error(error, timeout_duration))?;
    let elapsed_ms = round1(started.elapsed().as_secs_f64() * 1000.0);
    let status = response.status();
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let headers: Vec<HttpHeaderDto> = response
        .headers()
        .iter()
        .map(|(name, value)| HttpHeaderDto {
            name: name.to_string(),
            value: value
                .to_str()
                .map(str::to_owned)
                .unwrap_or_else(|_| format!("0x{}", hex_bytes(value.as_bytes()))),
        })
        .collect();

    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| map_request_error(error, timeout_duration))?
    {
        if bytes.len() + chunk.len() > MAX_BODY_BYTES {
            let remaining = MAX_BODY_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    let textual = content_type
        .as_deref()
        .map(is_textual_content_type)
        .unwrap_or_else(|| std::str::from_utf8(&bytes).is_ok());
    let body = if textual {
        String::from_utf8_lossy(&bytes).into_owned()
    } else {
        hex_bytes(&bytes)
    };

    Ok(HttpResponseDto {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or_default().to_owned(),
        final_url,
        elapsed_ms,
        headers,
        body,
        body_bytes: bytes.len(),
        truncated,
        content_type,
        binary: !textual,
    })
}

fn is_textual_content_type(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("text/")
        || value.contains("json")
        || value.contains("xml")
        || value.contains("javascript")
        || value.contains("x-www-form-urlencoded")
}

fn map_request_error(error: reqwest::Error, timeout_duration: Duration) -> ToolErrorDto {
    if error.is_timeout() {
        ToolErrorDto::new(
            "timeout",
            format!(
                "Request timed out after {} ms",
                timeout_duration.as_millis()
            ),
        )
    } else if error.is_connect() {
        let detail = error.to_string();
        if detail.to_ascii_lowercase().contains("certificate") {
            ToolErrorDto::new("tlsError", detail)
        } else {
            ToolErrorDto::new("connectFailed", detail)
        }
    } else {
        ToolErrorDto::new("httpError", error.to_string())
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(4096)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn posts_body_and_reads_response_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0u8; 2048];
            let read = socket.read(&mut buffer).await.expect("read request");
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            assert!(request.starts_with("POST /echo HTTP/1.1"));
            assert!(request.contains("hello"));
            socket
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nX-Test: yes\r\nContent-Length: 11\r\n\r\n{\"ok\":true}",
                )
                .await
                .expect("write response");
        });

        let result = request(HttpRequestInput {
            method: "POST".to_owned(),
            url: format!("http://127.0.0.1:{port}/echo"),
            headers: vec![super::super::model::HttpHeaderInput {
                name: "X-Request".to_owned(),
                value: "demo".to_owned(),
            }],
            body: Some("hello".to_owned()),
            timeout_ms: 2000,
            follow_redirects: false,
            ca_path: None,
        })
        .await
        .expect("response");
        assert_eq!(result.status, 201);
        assert_eq!(result.status_text, "Created");
        assert_eq!(result.body, "{\"ok\":true}");
        assert!(result.headers.iter().any(|header| header.name == "x-test"));
        server.await.expect("server");
    }

    #[test]
    fn detects_textual_content_types() {
        assert!(is_textual_content_type("application/json; charset=utf-8"));
        assert!(is_textual_content_type("text/plain"));
        assert!(!is_textual_content_type("image/png"));
    }
}
