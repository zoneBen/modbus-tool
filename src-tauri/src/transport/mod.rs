//! Link transports: TCP, UDP and a dedicated-thread serial port.

pub mod serial;
pub mod stream;
pub mod tcp;
pub mod udp;

use std::time::Duration;

use modbus_core::Framing;

use crate::model::LinkConfig;

#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    #[error("连接已关闭")]
    Closed,
    #[error("响应超时")]
    Timeout { partial: Vec<u8> },
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("帧错误: {0}")]
    InvalidFrame(String),
    #[error("串口错误: {0}")]
    Serial(String),
}

impl TransportError {
    pub fn partial(&self) -> Vec<u8> {
        match self {
            Self::Timeout { partial } => partial.clone(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawReply {
    pub data: Vec<u8>,
    pub latency_ms: Option<f64>,
}

pub enum Transport {
    Tcp(tcp::TcpTransport),
    Udp(udp::UdpTransport),
    Serial(serial::SerialTransport),
}

impl Transport {
    pub async fn connect(link: &LinkConfig, timeout: Duration) -> Result<Self, TransportError> {
        match link {
            LinkConfig::Tcp(network) => Ok(Self::Tcp(
                tcp::TcpTransport::connect(&network.host, network.port, timeout).await?,
            )),
            LinkConfig::Udp(network) => Ok(Self::Udp(
                udp::UdpTransport::connect(&network.host, network.port, timeout).await?,
            )),
            LinkConfig::Serial(serial) => Ok(Self::Serial(serial::SerialTransport::open(serial)?)),
        }
    }

    pub async fn transaction(
        &mut self,
        request: &[u8],
        framing: Framing,
        timeout: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        match self {
            Self::Tcp(transport) => transport.transaction(request, framing, timeout).await,
            Self::Udp(transport) => transport.transaction(request, framing, timeout).await,
            Self::Serial(transport) => transport.transaction(request, framing, timeout).await,
        }
    }

    pub async fn send_raw(
        &mut self,
        data: &[u8],
        timeout: Duration,
    ) -> Result<RawReply, TransportError> {
        match self {
            Self::Tcp(transport) => transport.send_raw(data, timeout).await,
            Self::Udp(transport) => transport.send_raw(data, timeout).await,
            Self::Serial(transport) => transport.send_raw(data, timeout).await,
        }
    }
}
