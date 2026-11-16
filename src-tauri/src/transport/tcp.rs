//! Modbus over TCP, including RTU/ASCII over TCP.

use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout, timeout_at};

use modbus_core::Framing;

use super::TransportError;
use super::stream::read_frame;

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
}
