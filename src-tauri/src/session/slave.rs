//! Slave simulator servers for TCP and serial links.

use std::io::{ErrorKind, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use modbus_core::frame::{self, MAX_ASCII_FRAME_LEN, MAX_RTU_ADU_LEN};
use modbus_core::simulator::{Simulator, SlaveUnit};
use modbus_core::{Framing, PduRequest};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, watch};

use crate::event::{AppEvent, EventBus, emit};
use crate::frames::{FrameInfo, build_frame};
use crate::model::{
    DefinitionDto, DefinitionSnapshotDto, FrameDto, LinkConfig, RegisterValueDto, SerialLinkConfig,
    SerialParity, SessionState, SessionStatus, now_ms,
};

const MAP_QUANTITY: u16 = 16;

pub struct SlaveServer {
    stop_tx: Option<watch::Sender<bool>>,
    stop_flag: Arc<AtomicBool>,
    task: Option<tokio::task::JoinHandle<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SlaveServer {
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(sender) = self.stop_tx.take() {
            let _ = sender.send(true);
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
        if let Some(thread) = self.thread.take() {
            drop(thread);
        }
    }
}

impl Drop for SlaveServer {
    fn drop(&mut self) {
        self.stop();
    }
}

pub async fn start(
    state: Arc<RwLock<SessionState>>,
    bus: EventBus,
    seq: Arc<AtomicU64>,
) -> Result<SlaveServer, String> {
    let (definition, link, framing, session_id) = {
        let state = state.read().await;
        let definition = state
            .config
            .definitions
            .first()
            .cloned()
            .ok_or_else(|| "从站映射定义缺失".to_owned())?;
        (
            definition,
            state.config.link.clone(),
            state.config.framing,
            state.config.id.clone(),
        )
    };
    let simulator = Arc::new(Mutex::new(build_simulator(definition.slave_id)));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let (stop_tx, stop_rx) = watch::channel(false);

    match link {
        LinkConfig::Tcp(network) => {
            let listener = TcpListener::bind((network.host.as_str(), network.port))
                .await
                .map_err(|error| format!("无法监听 {}:{}：{error}", network.host, network.port))?;
            set_status(&state, &bus, SessionStatus::Connected, None).await;
            emit(
                &bus,
                AppEvent::SessionData(build_snapshot(&session_id, &definition, &simulator)),
            );
            let task = tokio::spawn(tcp_accept_loop(
                listener, framing, simulator, definition, session_id, state, bus, seq, stop_rx,
            ));
            Ok(SlaveServer {
                stop_tx: Some(stop_tx),
                stop_flag,
                task: Some(task),
                thread: None,
            })
        }
        LinkConfig::Serial(serial) => {
            let port = open_serial(&serial)?;
            set_status(&state, &bus, SessionStatus::Connected, None).await;
            emit(
                &bus,
                AppEvent::SessionData(build_snapshot(&session_id, &definition, &simulator)),
            );
            let thread_stop = stop_flag.clone();
            let thread = std::thread::Builder::new()
                .name(format!("modbus-slave-{}", serial.port))
                .spawn(move || {
                    serial_loop(
                        port,
                        framing,
                        simulator,
                        definition,
                        session_id,
                        state,
                        bus,
                        seq,
                        thread_stop,
                    );
                })
                .map_err(|error| format!("启动串口从站线程失败：{error}"))?;
            Ok(SlaveServer {
                stop_tx: Some(stop_tx),
                stop_flag,
                task: None,
                thread: Some(thread),
            })
        }
        LinkConfig::Udp(_) => Err("从站模拟暂不支持 UDP，请使用 TCP 或串口".to_owned()),
    }
}

fn build_simulator(unit_id: u8) -> Simulator {
    let mut unit = SlaveUnit::new(unit_id);
    for address in 0..MAP_QUANTITY {
        unit.set_holding(address, 1000 + address);
        unit.set_input(address, 2000 + address);
    }
    for address in 0..8u16 {
        unit.set_coil(address, address % 2 == 0);
        unit.discrete_inputs.insert(address, address % 2 == 1);
    }
    Simulator::new([unit])
}

async fn set_status(
    state: &Arc<RwLock<SessionState>>,
    bus: &EventBus,
    status: SessionStatus,
    error: Option<String>,
) {
    let event = {
        let mut state = state.write().await;
        state.status = status;
        state.status_event(error)
    };
    emit(bus, AppEvent::SessionStatus(event));
}

#[allow(clippy::too_many_arguments)]
async fn tcp_accept_loop(
    listener: TcpListener,
    framing: Framing,
    simulator: Arc<Mutex<Simulator>>,
    definition: DefinitionDto,
    session_id: String,
    state: Arc<RwLock<SessionState>>,
    bus: EventBus,
    seq: Arc<AtomicU64>,
    mut stop_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            _ = stop_rx.changed() => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((socket, _)) => {
                        tokio::spawn(tcp_connection(
                            socket,
                            framing,
                            simulator.clone(),
                            definition.clone(),
                            session_id.clone(),
                            state.clone(),
                            bus.clone(),
                            seq.clone(),
                            stop_rx.clone(),
                        ));
                    }
                    Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn tcp_connection(
    mut socket: TcpStream,
    framing: Framing,
    simulator: Arc<Mutex<Simulator>>,
    definition: DefinitionDto,
    session_id: String,
    state: Arc<RwLock<SessionState>>,
    bus: EventBus,
    seq: Arc<AtomicU64>,
    mut stop_rx: watch::Receiver<bool>,
) {
    loop {
        let request = tokio::select! {
            _ = stop_rx.changed() => break,
            request = read_tcp_request(&mut socket, framing) => {
                match request {
                    Ok(Some(request)) => request,
                    _ => break,
                }
            }
        };
        let Some(reply) = process_request(
            &request,
            framing,
            &simulator,
            &definition,
            &session_id,
            &seq,
        ) else {
            continue;
        };
        if let Some(response) = &reply.response {
            if socket.write_all(response).await.is_err() {
                break;
            }
        }
        publish_async(&state, &bus, reply, &simulator, &definition).await;
    }
}

async fn read_tcp_request(
    socket: &mut TcpStream,
    framing: Framing,
) -> std::io::Result<Option<Vec<u8>>> {
    match framing {
        Framing::Mbap => {
            let mut header = [0u8; 6];
            socket.read_exact(&mut header).await?;
            let length = u16::from_be_bytes([header[4], header[5]]) as usize;
            if !(2..=254).contains(&length) {
                return Ok(None);
            }
            let mut frame = header.to_vec();
            frame.resize(6 + length, 0);
            socket.read_exact(&mut frame[6..]).await?;
            Ok(Some(frame))
        }
        Framing::Ascii => {
            let mut frame = Vec::with_capacity(32);
            let mut byte = [0u8; 1];
            loop {
                socket.read_exact(&mut byte).await?;
                frame.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(Some(frame));
                }
                if frame.len() > MAX_ASCII_FRAME_LEN {
                    return Ok(None);
                }
            }
        }
        Framing::Rtu => {
            let mut frame = vec![0u8; 2];
            socket.read_exact(&mut frame).await?;
            let function = frame[1];
            if matches!(function, 0x0F | 0x10) {
                frame.resize(7, 0);
                socket.read_exact(&mut frame[2..]).await?;
                let byte_count = frame[6] as usize;
                frame.resize(9 + byte_count, 0);
                if frame.len() > MAX_RTU_ADU_LEN {
                    return Ok(None);
                }
                socket.read_exact(&mut frame[7..]).await?;
            } else {
                frame.resize(8, 0);
                socket.read_exact(&mut frame[2..]).await?;
            }
            Ok(Some(frame))
        }
    }
}

struct SlaveReply {
    response: Option<Vec<u8>>,
    frames: Vec<FrameDto>,
    snapshot: DefinitionSnapshotDto,
}

fn process_request(
    raw: &[u8],
    framing: Framing,
    simulator: &Arc<Mutex<Simulator>>,
    definition: &DefinitionDto,
    session_id: &str,
    seq: &Arc<AtomicU64>,
) -> Option<SlaveReply> {
    let adu = frame::decode(framing, raw).ok()?;
    let request = PduRequest::decode(&adu.pdu).ok();
    let summary = match &request {
        Some(request) => format!(
            "FC{:02} · 从站 {} · {} {} × {}",
            request.function_code(),
            adu.unit_id,
            if request.is_write() {
                "写入"
            } else {
                "读取"
            },
            request.address(),
            request.quantity()
        ),
        None => format!("从站 {} · 无法解析请求", adu.unit_id),
    };
    let response_pdu = {
        let mut simulator = simulator
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        simulator.handle_request(adu.unit_id, &adu.pdu)
    };
    let mut frames = vec![build_frame(
        next_seq(seq),
        session_id,
        raw,
        framing,
        FrameInfo {
            direction: "rx",
            summary,
            latency_ms: None,
            status: "ok",
        },
    )];
    let mut response = None;
    if let Some(pdu) = response_pdu {
        let encoded = frame::encode(framing, adu.unit_id, adu.transaction_id, &pdu).ok()?;
        frames.push(build_frame(
            next_seq(seq),
            session_id,
            &encoded,
            framing,
            FrameInfo {
                direction: "tx",
                summary: format!("从站 {} · 响应", adu.unit_id),
                latency_ms: Some(0.5),
                status: "ok",
            },
        ));
        response = Some(encoded);
    }
    let snapshot = build_snapshot(session_id, definition, simulator);
    Some(SlaveReply {
        response,
        frames,
        snapshot,
    })
}

fn build_snapshot(
    session_id: &str,
    definition: &DefinitionDto,
    simulator: &Arc<Mutex<Simulator>>,
) -> DefinitionSnapshotDto {
    let simulator = simulator
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let unit = simulator.unit(definition.slave_id);
    let values = (0..definition.quantity.min(MAP_QUANTITY))
        .map(|offset| {
            let address = definition.start_address.wrapping_add(offset);
            let word = unit.map(|unit| unit.holding(address)).unwrap_or(0);
            RegisterValueDto {
                address,
                words: vec![word],
                formatted: word.to_string(),
                numeric: Some(f64::from(word)),
                stale: false,
                error: None,
            }
        })
        .collect();
    DefinitionSnapshotDto {
        session_id: session_id.to_owned(),
        definition_id: definition.id.clone(),
        timestamp: now_ms(),
        values,
        latency_ms: 0.5,
        error: None,
    }
}

async fn publish_async(
    state: &Arc<RwLock<SessionState>>,
    bus: &EventBus,
    reply: SlaveReply,
    simulator: &Arc<Mutex<Simulator>>,
    definition: &DefinitionDto,
) {
    let event = {
        let mut state = state.write().await;
        state.stats.rx += 1;
        state.stats.tx += u64::from(reply.response.is_some());
        state.status_event(None)
    };
    emit(
        bus,
        AppEvent::SessionFrames {
            session_id: event.session_id.clone(),
            frames: reply.frames,
        },
    );
    let snapshot = build_snapshot(&event.session_id, definition, simulator);
    emit(bus, AppEvent::SessionData(snapshot));
    emit(bus, AppEvent::SessionStatus(event));
}

fn publish_blocking(
    state: &Arc<RwLock<SessionState>>,
    bus: &EventBus,
    reply: SlaveReply,
    simulator: &Arc<Mutex<Simulator>>,
    definition: &DefinitionDto,
) {
    if reply.frames.is_empty() {
        return;
    }
    if let Ok(mut state) = state.try_write() {
        state.stats.rx += 1;
        state.stats.tx += u64::from(reply.response.is_some());
        let event = state.status_event(None);
        let session_id = event.session_id.clone();
        emit(
            bus,
            AppEvent::SessionFrames {
                session_id: session_id.clone(),
                frames: reply.frames,
            },
        );
        emit(
            bus,
            AppEvent::SessionData(build_snapshot(&session_id, definition, simulator)),
        );
        emit(bus, AppEvent::SessionStatus(event));
    }
}

fn next_seq(seq: &Arc<AtomicU64>) -> u64 {
    seq.fetch_add(1, Ordering::Relaxed)
}

fn open_serial(config: &SerialLinkConfig) -> Result<Box<dyn SerialPort>, String> {
    serialport::new(&config.port, config.baud_rate)
        .data_bits(match config.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            _ => DataBits::Eight,
        })
        .parity(match config.parity {
            SerialParity::Even => Parity::Even,
            SerialParity::Odd => Parity::Odd,
            SerialParity::None => Parity::None,
        })
        .stop_bits(match config.stop_bits {
            2 => StopBits::Two,
            _ => StopBits::One,
        })
        .timeout(Duration::from_millis(25))
        .open()
        .map_err(|error| format!("打开串口 {} 失败：{error}", config.port))
}

#[allow(clippy::too_many_arguments)]
fn serial_loop(
    mut port: Box<dyn SerialPort>,
    framing: Framing,
    simulator: Arc<Mutex<Simulator>>,
    definition: DefinitionDto,
    session_id: String,
    state: Arc<RwLock<SessionState>>,
    bus: EventBus,
    seq: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
) {
    let mut buffer = Vec::<u8>::new();
    while !stop.load(Ordering::Relaxed) {
        let mut chunk = [0u8; 256];
        match port.read(&mut chunk) {
            Ok(0) => continue,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(error) if error.kind() == ErrorKind::TimedOut => continue,
            Err(_) => break,
        }

        if framing == Framing::Ascii {
            while let Some(position) = buffer.iter().position(|byte| *byte == b'\n') {
                let request: Vec<u8> = buffer.drain(..=position).collect();
                handle_serial_request(
                    &request,
                    framing,
                    &mut *port,
                    &simulator,
                    &definition,
                    &session_id,
                    &bus,
                    &seq,
                    &state,
                );
            }
            if buffer.len() > MAX_ASCII_FRAME_LEN {
                buffer.clear();
            }
            continue;
        }

        loop {
            if buffer.len() < 2 {
                break;
            }
            let function = buffer[1];
            let total = match function {
                0x01..=0x06 => 8,
                0x0F | 0x10 => {
                    if buffer.len() < 7 {
                        break;
                    }
                    9 + buffer[6] as usize
                }
                _ => 8,
            };
            if total > MAX_RTU_ADU_LEN {
                buffer.clear();
                break;
            }
            if buffer.len() < total {
                break;
            }
            let request: Vec<u8> = buffer.drain(..total).collect();
            if frame::decode(Framing::Rtu, &request).is_err() {
                continue;
            }
            handle_serial_request(
                &request,
                Framing::Rtu,
                &mut *port,
                &simulator,
                &definition,
                &session_id,
                &bus,
                &seq,
                &state,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_serial_request(
    request: &[u8],
    framing: Framing,
    port: &mut dyn SerialPort,
    simulator: &Arc<Mutex<Simulator>>,
    definition: &DefinitionDto,
    session_id: &str,
    bus: &EventBus,
    seq: &Arc<AtomicU64>,
    state: &Arc<RwLock<SessionState>>,
) {
    let Some(reply) = process_request(request, framing, simulator, definition, session_id, seq)
    else {
        return;
    };
    if let Some(response) = &reply.response {
        if port.write_all(response).is_err() || port.flush().is_err() {
            return;
        }
    }
    publish_blocking(state, bus, reply, simulator, definition);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use modbus_core::{DataType, OrderPreset, PduResponse};

    use super::*;

    fn map_definition() -> DefinitionDto {
        DefinitionDto {
            id: "map".to_owned(),
            name: "map".to_owned(),
            function_code: 0x03,
            slave_id: 1,
            start_address: 0,
            quantity: MAP_QUANTITY,
            data_type: DataType::U16,
            byte_order: OrderPreset::Abcd,
            byte_pos: None,
            interval_ms: 500,
            enabled: true,
            aliases: BTreeMap::new(),
        }
    }

    #[test]
    fn mbap_read_and_write_update_the_map() {
        let simulator = Arc::new(Mutex::new(build_simulator(1)));
        let definition = map_definition();
        let seq = Arc::new(AtomicU64::new(1));

        let read = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: 2,
        };
        let read_frame =
            frame::encode(Framing::Mbap, 1, Some(1), &read.encode().expect("pdu")).expect("frame");
        let reply = process_request(
            &read_frame,
            Framing::Mbap,
            &simulator,
            &definition,
            "s1",
            &seq,
        )
        .expect("reply");
        let response = reply.response.expect("response");
        let adu = frame::decode(Framing::Mbap, &response).expect("decode");
        let decoded = PduResponse::decode_for_request(&read, &adu.pdu).expect("response pdu");
        assert_eq!(
            decoded,
            PduResponse::ReadRegisters {
                function_code: 3,
                values: vec![1000, 1001]
            }
        );
        assert_eq!(reply.snapshot.values[0].numeric, Some(1000.0));

        let write = PduRequest::WriteSingleRegister {
            address: 0,
            value: 0x1234,
        };
        let write_frame =
            frame::encode(Framing::Mbap, 1, Some(2), &write.encode().expect("pdu")).expect("frame");
        let reply = process_request(
            &write_frame,
            Framing::Mbap,
            &simulator,
            &definition,
            "s1",
            &seq,
        )
        .expect("reply");
        assert!(reply.response.is_some());
        assert_eq!(reply.snapshot.values[0].numeric, Some(4660.0));
    }
}
