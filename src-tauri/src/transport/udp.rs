//! Modbus over UDP (MBAP or RTU/ASCII datagrams).

use std::io::ErrorKind;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::{Instant, timeout, timeout_at};

use modbus_core::Framing;

use super::{RawReply, TransportError};

const UDP_BUFFER: usize = 2048;

pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub async fn connect(
        host: &str,
        port: u16,
        timeout_duration: Duration,
    ) -> Result<Self, TransportError> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|error| TransportError::Io(error.to_string()))?;
        timeout(timeout_duration, socket.connect((host, port)))
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;
        Ok(Self { socket })
    }

    pub async fn transaction(
        &mut self,
        request: &[u8],
        framing: Framing,
        timeout_duration: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        let deadline = Instant::now() + timeout_duration;
        let mut buffer = [0u8; UDP_BUFFER];

        // Drain stale datagrams before sending the next request.
        loop {
            match self.socket.try_recv(&mut buffer) {
                Ok(_) => continue,
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        timeout_at(deadline, self.socket.send(request))
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;

        loop {
            let read = timeout_at(deadline, self.socket.recv(&mut buffer))
                .await
                .map_err(|_| TransportError::Timeout {
                    partial: Vec::new(),
                })?
                .map_err(|error| TransportError::Io(error.to_string()))?;
            let response = buffer[..read].to_vec();
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
        let mut buffer = [0u8; UDP_BUFFER];
        loop {
            match self.socket.try_recv(&mut buffer) {
                Ok(_) => continue,
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        timeout_at(deadline, self.socket.send(data))
            .await
            .map_err(|_| TransportError::Timeout {
                partial: Vec::new(),
            })?
            .map_err(|error| TransportError::Io(error.to_string()))?;

        let started = Instant::now();
        match timeout_at(deadline, self.socket.recv(&mut buffer)).await {
            Err(_) => Ok(RawReply {
                data: Vec::new(),
                latency_ms: None,
            }),
            Ok(Ok(read)) => Ok(RawReply {
                data: buffer[..read].to_vec(),
                latency_ms: Some(started.elapsed().as_secs_f64() * 1000.0),
            }),
            Ok(Err(error)) => Err(TransportError::Io(error.to_string())),
        }
    }
}
