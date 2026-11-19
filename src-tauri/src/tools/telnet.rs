use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, lookup_host};
use tokio::time::timeout;

use crate::model::RawSendInput;

use super::model::{TelnetResultDto, TelnetTestInput, ToolErrorDto, ToolResult};

const MAX_READ_BYTES: usize = 64 * 1024;

pub async fn test(input: TelnetTestInput) -> ToolResult<TelnetResultDto> {
    let host = input.host.trim().to_owned();
    if host.is_empty() {
        return Err(ToolErrorDto::new("invalidHost", "Host is required"));
    }
    if input.port == 0 {
        return Err(ToolErrorDto::new(
            "invalidPort",
            "Port must be between 1 and 65535",
        ));
    }
    let timeout_duration = Duration::from_millis(input.timeout_ms.clamp(100, 30_000));
    let read_timeout = Duration::from_millis(input.read_timeout_ms.clamp(50, 10_000));

    let address = timeout(timeout_duration, lookup_host((host.as_str(), input.port)))
        .await
        .map_err(|_| ToolErrorDto::new("resolveTimeout", "Host resolution timed out"))?
        .map_err(|error| ToolErrorDto::new("resolveFailed", error.to_string()))?
        .next()
        .ok_or_else(|| ToolErrorDto::new("resolveFailed", "No address found for host"))?;

    let started = Instant::now();
    let mut stream = timeout(timeout_duration, TcpStream::connect(address))
        .await
        .map_err(|_| ToolErrorDto::new("timeout", "TCP connection timed out"))?
        .map_err(|error| ToolErrorDto::new("connectFailed", error.to_string()))?;
    let latency_ms = round1(started.elapsed().as_secs_f64() * 1000.0);

    let payload = match input.payload.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => {
            let raw = RawSendInput {
                data: value.to_owned(),
                mode: input.payload_mode,
                append_cr: input.append_cr,
                append_lf: input.append_lf,
            };
            Some(
                raw.encode()
                    .map_err(|error| ToolErrorDto::new("invalidPayload", error))?,
            )
        }
        _ => None,
    };
    if let Some(bytes) = &payload {
        timeout(timeout_duration, stream.write_all(bytes))
            .await
            .map_err(|_| ToolErrorDto::new("timeout", "Sending test payload timed out"))?
            .map_err(|error| ToolErrorDto::new("sendFailed", error.to_string()))?;
    }

    let data = read_available(&mut stream, read_timeout).await;
    Ok(TelnetResultDto {
        host,
        port: input.port,
        connected: true,
        latency_ms,
        bytes_read: data.len(),
        text: display_text(&data),
        hex: display_hex(&data),
    })
}

async fn read_available(stream: &mut TcpStream, timeout_duration: Duration) -> Vec<u8> {
    let mut data = Vec::new();
    let mut idle_deadline = Instant::now() + timeout_duration;
    let mut buffer = [0u8; 4096];
    loop {
        let now = Instant::now();
        if now >= idle_deadline || data.len() >= MAX_READ_BYTES {
            break;
        }
        let remaining = idle_deadline.saturating_duration_since(now);
        match timeout(remaining, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(read)) => {
                data.extend_from_slice(&buffer[..read]);
                idle_deadline = Instant::now() + Duration::from_millis(120);
            }
            _ => break,
        }
    }
    data.truncate(MAX_READ_BYTES);
    data
}

fn display_text(data: &[u8]) -> String {
    let text = String::from_utf8_lossy(data);
    text.chars()
        .map(|character| {
            if character == '\r'
                || character == '\n'
                || character == '\t'
                || !character.is_control()
            {
                character
            } else {
                '·'
            }
        })
        .collect()
}

fn display_hex(data: &[u8]) -> String {
    data.iter()
        .take(1024)
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
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn reads_banner_from_loopback() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            socket.write_all(b"HELLO\r\n").await.expect("write");
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        let result = test(TelnetTestInput {
            host: "127.0.0.1".to_owned(),
            port,
            timeout_ms: 1000,
            read_timeout_ms: 300,
            payload: None,
            payload_mode: crate::model::RawSendMode::Ascii,
            append_cr: false,
            append_lf: false,
        })
        .await
        .expect("telnet result");
        assert!(result.connected);
        assert_eq!(result.text, "HELLO\r\n");
        assert_eq!(result.bytes_read, 7);
        server.await.expect("server");
    }
}
