//! Modbus over TCP, including RTU/ASCII over TCP.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout, timeout_at};

use modbus_core::Framing;

use super::stream::read_frame;
use super::{RawReply, TransportError};

const RAW_IDLE: Duration = Duration::from_millis(60);
const RAW_REPLY_LIMIT: usize = 8192;

pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub async fn connect(
        host: &str,
        port: u16,
        timeout_duration: Duration,
    ) -> Result<Self, TransportError> {
        let stream = timeout(timeout_duration, TcpStream::connect((host, port)))
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;
        let _ = stream.set_nodelay(true);
        Ok(Self { stream })
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        let _ = stream.set_nodelay(true);
        Self { stream }
    }

    pub async fn transaction(
        &mut self,
        request: &[u8],
        framing: Framing,
        timeout_duration: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        let deadline = Instant::now() + timeout_duration;
        timeout_at(deadline, self.stream.write_all(request))
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;
        timeout_at(deadline, self.stream.flush())
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;

        loop {
            let response = read_frame(&mut self.stream, framing, deadline).await?;
            // MBAP responses may be stale; ignore transaction ids that do not
            // match the request and keep reading until the deadline.
            if framing == Framing::Mbap
                && response.len() >= 2
                && request.len() >= 2
                && response[..2] != request[..2]
            {
                continue;
            }
            return Ok(response);
        }
    }

    pub async fn send_raw(
        &mut self,
        data: &[u8],
        timeout_duration: Duration,
    ) -> Result<RawReply, TransportError> {
        let deadline = Instant::now() + timeout_duration;
        timeout_at(deadline, self.stream.write_all(data))
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;
        timeout_at(deadline, self.stream.flush())
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;

        let started = Instant::now();
        let mut buffer = [0u8; 1024];
        let mut response = Vec::new();
        match timeout_at(deadline, self.stream.read(&mut buffer)).await {
            Err(_) => {
                return Ok(RawReply {
                    data: response,
                    latency_ms: None,
                });
            }
            Ok(Ok(0)) => return Err(TransportError::Closed),
            Ok(Ok(read)) => {
                response.extend_from_slice(&buffer[..read]);
            }
            Ok(Err(error)) => return Err(TransportError::Io(error.to_string())),
        }
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;

        while response.len() < RAW_REPLY_LIMIT {
            let idle_deadline = (Instant::now() + RAW_IDLE).min(deadline);
            match timeout_at(idle_deadline, self.stream.read(&mut buffer)).await {
                Err(_) | Ok(Ok(0)) => break,
                Ok(Ok(read)) => response.extend_from_slice(&buffer[..read]),
                Ok(Err(error)) => return Err(TransportError::Io(error.to_string())),
            }
        }
        response.truncate(RAW_REPLY_LIMIT);
        Ok(RawReply {
            data: response,
            latency_ms: Some(latency_ms),
        })
    }
}
