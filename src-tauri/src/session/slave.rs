//! Slave simulator servers for TCP and serial links.

use std::io::{ErrorKind, Read};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use modbus_core::frame::{self, MAX_ASCII_FRAME_LEN, MAX_RTU_ADU_LEN};
use modbus_core::simulator::{Simulator, SlaveUnit};
use modbus_core::value::{decode_words, encode_value};
use modbus_core::{DataType, Framing, PduRequest};
use serialport::{DataBits, Parity, SerialPort, StopBits};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, watch};

use crate::event::{AppEvent, EventBus, emit};
use crate::frames::{FrameInfo, build_frame};
use crate::model::{
    DefinitionDto, DefinitionSnapshotDto, FrameDto, LinkConfig, RegisterPointDto, RegisterValueDto,
    SerialLinkConfig, SerialParity, SessionState, SessionStatus, SimulatorMode, SimulatorPointDto,
    now_ms, scale_decoded,
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
    simulator: Arc<Mutex<Simulator>>,
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
    let generator_states = build_generator_states(&definition);
    seed_generator_states(&simulator, definition.slave_id, &generator_states);
    let generator_definition = definition.clone();
    let generator_session = session_id.clone();
    let generator_bus = bus.clone();
    let generator_simulator = simulator.clone();
    let generator_active = generator_states
        .iter()
        .any(|state| state.config.enabled && state.config.refresh_ms > 0);

    let stop_flag = Arc::new(AtomicBool::new(false));
    let (stop_tx, stop_rx) = watch::channel(false);
    let generator_stop = stop_rx.clone();

    let server = match link {
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
    }?;

    if generator_active {
        tokio::spawn(run_simulator_generator(
            generator_states,
            generator_definition,
            generator_simulator,
            generator_session,
            generator_bus,
            generator_stop,
        ));
    }

    Ok(server)
}

pub fn build_simulator(unit_id: u8) -> Simulator {
    let mut unit = SlaveUnit::new(unit_id);
    for address in 0..MAP_QUANTITY {
        unit.set_holding(address, 1000 + address);
        unit.set_input(address, 2000 + address);
    }
    for address in 0..8u16 {
        unit.set_coil(address, address.is_multiple_of(2));
        unit.discrete_inputs
            .insert(address, !address.is_multiple_of(2));
    }
    Simulator::new([unit])
}

struct GeneratorState {
    point: RegisterPointDto,
    config: SimulatorPointDto,
    value: f64,
    last: Instant,
    seed: u64,
}

fn random_seed(address: u16) -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(1)
        ^ (u64::from(address).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn random_between(min: f64, max: f64, seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let unit = ((*seed >> 11) as f64) / ((1u64 << 53) as f64);
    min + (max - min) * unit
}

fn initial_generator_value(config: &SimulatorPointDto, seed: &mut u64) -> f64 {
    let value = match config.mode {
        SimulatorMode::Increment | SimulatorMode::Fixed => config.min,
        SimulatorMode::Decrement => config.max,
        SimulatorMode::Random => random_between(config.min, config.max, seed),
    };
    value.clamp(config.min, config.max)
}

fn build_generator_states(definition: &DefinitionDto) -> Vec<GeneratorState> {
    definition
        .effective_points()
        .into_iter()
        .filter_map(|point| {
            let config = point.simulator.clone()?;
            let mut seed = random_seed(point.address);
            let value = initial_generator_value(&config, &mut seed);
            Some(GeneratorState {
                point,
                config,
                value,
                last: Instant::now(),
                seed,
            })
        })
        .collect()
}

fn write_point_value(unit: &mut SlaveUnit, point: &RegisterPointDto, value: f64) {
    if point.data_type == DataType::Bool {
        let on = value >= 0.5;
        unit.set_coil(point.address, on);
        unit.discrete_inputs.insert(point.address, on);
        return;
    }
    let text = match point.data_type {
        DataType::F32 | DataType::F64 => format!("{value}"),
        _ => format!("{}", value.round() as i64),
    };
    let Ok(words) = encode_value(&point.rule(), &text) else {
        return;
    };
    for (index, word) in words.into_iter().enumerate() {
        let address = point.address.wrapping_add(index as u16);
        unit.set_holding(address, word);
        unit.set_input(address, word);
    }
}

fn seed_generator_states(
    simulator: &Arc<Mutex<Simulator>>,
    unit_id: u8,
    states: &[GeneratorState],
) {
    if states.is_empty() {
        return;
    }
    let mut simulator = simulator
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let Some(unit) = simulator.unit_mut(unit_id) else {
        return;
    };
    for state in states {
        write_point_value(unit, &state.point, state.value);
    }
}

fn step_for(point: &RegisterPointDto, config: &SimulatorPointDto) -> f64 {
    let range = (config.max - config.min).abs();
    if matches!(point.data_type, DataType::F32 | DataType::F64) && range > 0.0 && range < 10.0 {
        range / 10.0
    } else {
        1.0
    }
}

fn advance_generator(state: &mut GeneratorState) -> f64 {
    let config = &state.config;
    let next = match config.mode {
        SimulatorMode::Increment => {
            let value = state.value + step_for(&state.point, config);
            if value > config.max {
                config.min
            } else {
                value
            }
        }
        SimulatorMode::Decrement => {
            let value = state.value - step_for(&state.point, config);
            if value < config.min {
                config.max
            } else {
                value
            }
        }
        SimulatorMode::Random => random_between(config.min, config.max, &mut state.seed),
        SimulatorMode::Fixed => config.min,
    };
    state.value = next.clamp(config.min, config.max);
    state.value
}

async fn run_simulator_generator(
    mut states: Vec<GeneratorState>,
    definition: DefinitionDto,
    simulator: Arc<Mutex<Simulator>>,
    session_id: String,
    bus: EventBus,
    mut stop_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_err() || *stop_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(25)) => {}
        }

        let now = Instant::now();
        let mut changed = false;
        {
            let mut simulator = simulator
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let Some(unit) = simulator.unit_mut(definition.slave_id) else {
                break;
            };
            for state in &mut states {
                if !state.config.enabled {
                    continue;
                }
                let interval = Duration::from_millis(state.config.refresh_ms.max(20));
                if now.duration_since(state.last) < interval {
                    continue;
                }
                state.last = now;
                let value = advance_generator(state);
                write_point_value(unit, &state.point, value);
                changed = true;
            }
        }
        if changed {
            emit(
                &bus,
                AppEvent::SessionData(build_snapshot(&session_id, &definition, &simulator)),
            );
        }
    }
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
        if let Some(response) = &reply.response
            && socket.write_all(response).await.is_err()
        {
            break;
        }
        publish_async(&state, &bus, reply).await;
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

pub fn build_snapshot(
    session_id: &str,
    definition: &DefinitionDto,
    simulator: &Arc<Mutex<Simulator>>,
) -> DefinitionSnapshotDto {
    let simulator = simulator
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let unit = simulator.unit(definition.slave_id);
    let raw_words: Vec<u16> = (0..definition.quantity.min(MAP_QUANTITY))
        .map(|offset| {
            let address = definition.start_address.wrapping_add(offset);
            unit.map_or(0, |unit| unit.holding(address))
        })
        .collect();
    let values = definition
        .effective_points()
        .into_iter()
        .map(|point| {
            if point.data_type == DataType::Bool {
                let value = unit.is_some_and(|unit| unit.coil(point.address));
                let (formatted, numeric) = scale_decoded(
                    &point,
                    if value { "ON" } else { "OFF" }.to_owned(),
                    Some(if value { 1.0 } else { 0.0 }),
                );
                return RegisterValueDto {
                    address: point.address,
                    words: vec![if value { 1 } else { 0 }],
                    formatted,
                    numeric,
                    stale: false,
                    error: None,
                };
            }
            let offset = usize::from(point.address.saturating_sub(definition.start_address));
            let span = point.span();
            if offset + span > raw_words.len() {
                return RegisterValueDto {
                    address: point.address,
                    words: Vec::new(),
                    formatted: "—".to_owned(),
                    numeric: None,
                    stale: true,
                    error: Some("模拟寄存器范围不足以解析该点位".to_owned()),
                };
            }
            let chunk = &raw_words[offset..offset + span];
            match decode_words(&point.rule(), chunk) {
                Ok(value) => {
                    let (formatted, numeric) =
                        scale_decoded(&point, value.formatted, value.numeric);
                    RegisterValueDto {
                        address: point.address,
                        words: value.words,
                        formatted,
                        numeric,
                        stale: false,
                        error: None,
                    }
                }
                Err(error) => RegisterValueDto {
                    address: point.address,
                    words: chunk.to_vec(),
                    formatted: "—".to_owned(),
                    numeric: None,
                    stale: true,
                    error: Some(error.to_string()),
                },
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

async fn publish_async(state: &Arc<RwLock<SessionState>>, bus: &EventBus, reply: SlaveReply) {
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
    emit(bus, AppEvent::SessionData(reply.snapshot));
    emit(bus, AppEvent::SessionStatus(event));
}

fn publish_blocking(state: &Arc<RwLock<SessionState>>, bus: &EventBus, reply: SlaveReply) {
    if reply.frames.is_empty() {
        return;
    }
    if let Ok(mut state) = state.try_write() {
        state.stats.rx += 1;
        state.stats.tx += u64::from(reply.response.is_some());
        let event = state.status_event(None);
        emit(
            bus,
            AppEvent::SessionFrames {
                session_id: event.session_id.clone(),
                frames: reply.frames,
            },
        );
        emit(bus, AppEvent::SessionData(reply.snapshot));
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
            let total = if matches!(function, 0x0F | 0x10) {
                if buffer.len() < 7 {
                    break;
                }
                9 + buffer[6] as usize
            } else {
                8
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
    if let Some(response) = &reply.response
        && (port.write_all(response).is_err() || port.flush().is_err())
    {
        return;
    }
    publish_blocking(state, bus, reply);
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
            points: Vec::new(),
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

    #[test]
    fn simulator_config_seeds_engineering_values_before_serving() {
        let mut definition = map_definition();
        definition.points = vec![RegisterPointDto {
            address: 0,
            data_type: DataType::F32,
            byte_order: OrderPreset::Abcd,
            byte_pos: None,
            meaning: "输入电压".to_owned(),
            unit: "V".to_owned(),
            scale: 1.0,
            offset: 0.0,
            alerts: Vec::new(),
            simulator: Some(SimulatorPointDto {
                mode: SimulatorMode::Fixed,
                refresh_ms: 1000,
                min: 12.5,
                max: 240.0,
                enabled: false,
            }),
        }];
        let simulator = Arc::new(Mutex::new(build_simulator(1)));
        let states = build_generator_states(&definition);
        assert_eq!(states.len(), 1);
        seed_generator_states(&simulator, 1, &states);

        let snapshot = build_snapshot("s1", &definition, &simulator);
        assert_eq!(snapshot.values.len(), 1);
        assert!((snapshot.values[0].numeric.unwrap_or_default() - 12.5).abs() < 1e-6);
        assert_eq!(snapshot.values[0].words.len(), 2);

        let simulator = simulator.lock().expect("simulator");
        let unit = simulator.unit(1).expect("unit");
        assert_eq!(unit.input(0), unit.holding(0));
        assert_eq!(unit.input(1), unit.holding(1));
    }

    #[test]
    fn increment_generator_wraps_at_the_upper_bound() {
        let definition = DefinitionDto {
            id: "gen".to_owned(),
            name: "gen".to_owned(),
            function_code: 0x03,
            slave_id: 1,
            start_address: 0,
            quantity: 1,
            data_type: DataType::U16,
            byte_order: OrderPreset::Abcd,
            byte_pos: None,
            interval_ms: 500,
            enabled: true,
            aliases: BTreeMap::new(),
            points: vec![RegisterPointDto {
                address: 0,
                data_type: DataType::U16,
                byte_order: OrderPreset::Abcd,
                byte_pos: None,
                meaning: String::new(),
                unit: String::new(),
                scale: 1.0,
                offset: 0.0,
                alerts: Vec::new(),
                simulator: Some(SimulatorPointDto {
                    mode: SimulatorMode::Increment,
                    refresh_ms: 100,
                    min: 0.0,
                    max: 2.0,
                    enabled: true,
                }),
            }],
        };
        let mut states = build_generator_states(&definition);
        let state = states.first_mut().expect("generator");
        assert_eq!(advance_generator(state), 1.0);
        assert_eq!(advance_generator(state), 2.0);
        assert_eq!(advance_generator(state), 0.0);
    }
}
