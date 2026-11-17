//! Serial transport backed by one dedicated blocking IO thread per port.

use std::time::{Duration, Instant};

use serialport::{ClearBuffer, DataBits, Parity, SerialPort, StopBits};
use tokio::sync::{mpsc, oneshot};

use modbus_core::Framing;
use modbus_core::frame::{self, MAX_ASCII_FRAME_LEN, MAX_RTU_ADU_LEN};

use crate::model::{SerialLinkConfig, SerialParity};

use super::{RawReply, TransportError};

const RAW_IDLE: Duration = Duration::from_millis(30);
const RAW_REPLY_LIMIT: usize = 8192;

enum SerialCommand {
    Transaction {
        request: Vec<u8>,
        framing: Framing,
        timeout: Duration,
        reply: oneshot::Sender<Result<Vec<u8>, TransportError>>,
    },
    SendRaw {
        data: Vec<u8>,
        timeout: Duration,
        reply: oneshot::Sender<Result<RawReply, TransportError>>,
    },
    Shutdown,
}

pub struct SerialTransport {
    sender: mpsc::UnboundedSender<SerialCommand>,
}

impl SerialTransport {
    pub fn open(config: &SerialLinkConfig) -> Result<Self, TransportError> {
        let port = serialport::new(&config.port, config.baud_rate)
            .data_bits(map_data_bits(config.data_bits))
            .parity(map_parity(config.parity))
            .stop_bits(map_stop_bits(config.stop_bits))
            .timeout(Duration::from_millis(15))
            .open()
            .map_err(|error| TransportError::Serial(error.to_string()))?;

        let (sender, receiver) = mpsc::unbounded_channel();
        std::thread::Builder::new()
            .name(format!("modbus-serial-{}", config.port))
            .spawn(move || serial_loop(port, receiver))
            .map_err(|error| TransportError::Serial(error.to_string()))?;
        Ok(Self { sender })
    }

    pub async fn transaction(
        &mut self,
        request: &[u8],
        framing: Framing,
        timeout_duration: Duration,
    ) -> Result<Vec<u8>, TransportError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(SerialCommand::Transaction {
                request: request.to_vec(),
                framing,
                timeout: timeout_duration,
                reply,
            })
            .map_err(|_| TransportError::Serial("串口线程已退出".to_owned()))?;

        match tokio::time::timeout(timeout_duration + Duration::from_millis(250), response).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TransportError::Serial("串口线程未返回结果".to_owned())),
            Err(_) => Err(TransportError::Timeout {
                partial: Vec::new(),
            }),
        }
    }

    pub async fn send_raw(
        &mut self,
        data: &[u8],
        timeout_duration: Duration,
    ) -> Result<RawReply, TransportError> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(SerialCommand::SendRaw {
                data: data.to_vec(),
                timeout: timeout_duration,
                reply,
            })
            .map_err(|_| TransportError::Serial("串口线程已退出".to_owned()))?;

        match tokio::time::timeout(timeout_duration + Duration::from_millis(250), response).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(TransportError::Serial("串口线程未返回结果".to_owned())),
            Err(_) => Err(TransportError::Timeout {
                partial: Vec::new(),
            }),
        }
    }
}

impl Drop for SerialTransport {
    fn drop(&mut self) {
        let _ = self.sender.send(SerialCommand::Shutdown);
    }
}

fn serial_loop(
    mut port: Box<dyn SerialPort>,
    mut receiver: mpsc::UnboundedReceiver<SerialCommand>,
) {
    while let Some(command) = receiver.blocking_recv() {
        match command {
            SerialCommand::Transaction {
                request,
                framing,
                timeout,
                reply,
            } => {
                let result = serial_transaction(&mut *port, &request, framing, timeout);
                let _ = reply.send(result);
            }
            SerialCommand::SendRaw {
                data,
                timeout,
                reply,
            } => {
                let result = serial_send_raw(&mut *port, &data, timeout);
                let _ = reply.send(result);
            }
            SerialCommand::Shutdown => break,
        }
    }
}

fn serial_transaction(
    port: &mut dyn SerialPort,
    request: &[u8],
    framing: Framing,
    timeout: Duration,
) -> Result<Vec<u8>, TransportError> {
    let deadline = Instant::now() + timeout;
    let _ = port.clear(ClearBuffer::Input);
    port.write_all(request)
        .map_err(|error| TransportError::Serial(error.to_string()))?;
    port.flush()
        .map_err(|error| TransportError::Serial(error.to_string()))?;

    let result = read_frame_blocking(port, framing, deadline);
    if result.is_err() {
        let _ = port.clear(ClearBuffer::Input);
    }
    result
}

fn serial_send_raw(
    port: &mut dyn SerialPort,
    data: &[u8],
    timeout: Duration,
) -> Result<RawReply, TransportError> {
    let deadline = Instant::now() + timeout;
    let _ = port.clear(ClearBuffer::Input);
    port.write_all(data)
        .map_err(|error| TransportError::Serial(error.to_string()))?;
    port.flush()
        .map_err(|error| TransportError::Serial(error.to_string()))?;

    let started = Instant::now();
    let mut buffer = [0u8; 1024];
    let mut response = Vec::new();
    let mut first_byte_at = None;
    loop {
        if Instant::now() >= deadline {
            break;
        }
        match port.read(&mut buffer) {
            Ok(0) => std::thread::sleep(Duration::from_millis(1)),
            Ok(read) => {
                first_byte_at = Some(started.elapsed().as_secs_f64() * 1000.0);
                response.extend_from_slice(&buffer[..read]);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(error) => return Err(TransportError::Serial(error.to_string())),
        }
    }

    if first_byte_at.is_some() {
        loop {
            if response.len() >= RAW_REPLY_LIMIT {
                break;
            }
            let idle_deadline = (Instant::now() + RAW_IDLE).min(deadline);
            match port.read(&mut buffer) {
                Ok(0) => {
                    if Instant::now() >= idle_deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(read) => response.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                    if Instant::now() >= idle_deadline {
                        break;
                    }
                }
                Err(error) => return Err(TransportError::Serial(error.to_string())),
            }
        }
    }

    response.truncate(RAW_REPLY_LIMIT);
    Ok(RawReply {
        data: response,
        latency_ms: first_byte_at,
    })
}

fn read_frame_blocking(
    port: &mut dyn SerialPort,
    framing: Framing,
    deadline: Instant,
) -> Result<Vec<u8>, TransportError> {
    match framing {
        Framing::Mbap => {
            let mut frame = Vec::with_capacity(16);
            read_exact_blocking(port, &mut frame, 6, deadline)?;
            let length = u16::from_be_bytes([frame[4], frame[5]]);
            if !(2..=254).contains(&length) {
                return Err(TransportError::InvalidFrame(format!(
                    "非法 MBAP 长度 {length}"
                )));
            }
            let total = 6 + length as usize;
            read_exact_blocking(port, &mut frame, total, deadline)?;
            Ok(frame)
        }
        Framing::Rtu => {
            let mut frame = Vec::with_capacity(16);
            loop {
                if let Some(expected) = frame::expected_frame_len(Framing::Rtu, &frame) {
                    if frame.len() >= expected {
                        frame.truncate(expected);
                        return Ok(frame);
                    }
                } else if frame.len() >= 8 {
                    return Err(TransportError::InvalidFrame(
                        "无法从功能码推导 RTU 响应长度".to_owned(),
                    ));
                }
                let needed = frame.len() + 1;
                read_exact_blocking(port, &mut frame, needed, deadline)?;
                if frame.len() > MAX_RTU_ADU_LEN {
                    return Err(TransportError::InvalidFrame("RTU 帧超长".to_owned()));
                }
            }
        }
        Framing::Ascii => {
            let mut frame = Vec::with_capacity(16);
            loop {
                let needed = frame.len() + 1;
                read_exact_blocking(port, &mut frame, needed, deadline)?;
                if frame.last() == Some(&b'\n') {
                    return Ok(frame);
                }
                if frame.len() >= MAX_ASCII_FRAME_LEN {
                    return Err(TransportError::InvalidFrame("ASCII 帧超长".to_owned()));
                }
            }
        }
    }
}

fn read_exact_blocking(
    port: &mut dyn SerialPort,
    buffer: &mut Vec<u8>,
    needed: usize,
    deadline: Instant,
) -> Result<(), TransportError> {
    while buffer.len() < needed {
        if Instant::now() >= deadline {
            return Err(TransportError::Timeout {
                partial: buffer.clone(),
            });
        }
        let mut byte = [0u8; 1];
        match port.read(&mut byte) {
            Ok(1) => buffer.push(byte[0]),
            Ok(_) => std::thread::sleep(Duration::from_millis(1)),
            Err(error) => {
                if error.kind() != std::io::ErrorKind::TimedOut {
                    return Err(TransportError::Serial(error.to_string()));
                }
            }
        }
    }
    Ok(())
}

fn map_data_bits(value: u8) -> DataBits {
    match value {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        _ => DataBits::Eight,
    }
}

fn map_parity(parity: SerialParity) -> Parity {
    match parity {
        SerialParity::None => Parity::None,
        SerialParity::Even => Parity::Even,
        SerialParity::Odd => Parity::Odd,
    }
}

fn map_stop_bits(value: u8) -> StopBits {
    if value == 2 {
        StopBits::Two
    } else {
        StopBits::One
    }
}
