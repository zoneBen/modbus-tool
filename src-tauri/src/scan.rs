//! Standalone scan engines for slave ids, addresses, baud rates and LAN hosts.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use ipnet::IpNet;
use modbus_core::{Framing, PduRequest, PduResponse, frame};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::{Instant, sleep, timeout};
use uuid::Uuid;

use crate::event::{AppEvent, EventBus, emit};
use crate::model::{
    AddressScanInput, BaudScanInput, LinkConfig, NetworkScanInput, ScanDoneDto, ScanKind,
    ScanProgressDto, ScanResultDto, SerialLinkConfig, SerialParity, SlaveIdScanInput,
};
use crate::transport::tcp::TcpTransport;
use crate::transport::{Transport, TransportError};

pub struct ScanManager {
    bus: EventBus,
    tasks: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl ScanManager {
    pub fn new(bus: EventBus) -> Self {
        Self {
            bus,
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_slave_scan(&self, input: SlaveIdScanInput) -> String {
        let id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        self.register(&id, cancel.clone());
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = id.clone();
        let remove_id = id.clone();
        tauri::async_runtime::spawn(async move {
            run_slave_scan(run_id, input, cancel, bus).await;
            remove_task(&tasks, &remove_id);
        });
        id
    }

    pub fn start_address_scan(&self, input: AddressScanInput) -> String {
        let id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        self.register(&id, cancel.clone());
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = id.clone();
        let remove_id = id.clone();
        tauri::async_runtime::spawn(async move {
            run_address_scan(run_id, input, cancel, bus).await;
            remove_task(&tasks, &remove_id);
        });
        id
    }

    pub fn start_baud_scan(&self, input: BaudScanInput) -> String {
        let id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        self.register(&id, cancel.clone());
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = id.clone();
        let remove_id = id.clone();
        tauri::async_runtime::spawn(async move {
            run_baud_scan(run_id, input, cancel, bus).await;
            remove_task(&tasks, &remove_id);
        });
        id
    }

    pub fn start_network_scan(&self, input: NetworkScanInput) -> String {
        let id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        self.register(&id, cancel.clone());
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = id.clone();
        let remove_id = id.clone();
        tauri::async_runtime::spawn(async move {
            run_network_scan(run_id, input, cancel, bus).await;
            remove_task(&tasks, &remove_id);
        });
        id
    }

    pub fn cancel(&self, scan_id: &str) -> bool {
        let tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        match tasks.get(scan_id) {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub fn shutdown_all(&self) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for flag in tasks.values() {
            flag.store(true, Ordering::Relaxed);
        }
        tasks.clear();
    }

    fn register(&self, id: &str, cancel: Arc<AtomicBool>) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        tasks.insert(id.to_owned(), cancel);
    }
}

fn remove_task(tasks: &Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>, id: &str) {
    let mut tasks = tasks.lock().unwrap_or_else(|poison| poison.into_inner());
    tasks.remove(id);
}

struct ProbeOutcome {
    status: &'static str,
    detail: String,
    latency_ms: Option<f64>,
    fatal: bool,
    response: Option<PduResponse>,
}

async fn probe_once(
    transport: &mut Transport,
    framing: Framing,
    unit_id: u8,
    request: &PduRequest,
    timeout_duration: Duration,
    transaction_id: u16,
) -> ProbeOutcome {
    let pdu = match request.encode() {
        Ok(pdu) => pdu,
        Err(error) => {
            return ProbeOutcome {
                status: "error",
                detail: error.to_string(),
                latency_ms: None,
                fatal: false,
                response: None,
            };
        }
    };
    let request_frame = match frame::encode(framing, unit_id, Some(transaction_id), &pdu) {
        Ok(frame) => frame,
        Err(error) => {
            return ProbeOutcome {
                status: "error",
                detail: error.to_string(),
                latency_ms: None,
                fatal: false,
                response: None,
            };
        }
    };

    let started = Instant::now();
    match transport
        .transaction(&request_frame, framing, timeout_duration)
        .await
    {
        Ok(response_frame) => {
            let latency_ms = round1(started.elapsed().as_secs_f64() * 1000.0);
            let adu = match frame::decode(framing, &response_frame) {
                Ok(adu) => adu,
                Err(error) => {
                    return ProbeOutcome {
                        status: "error",
                        detail: error.to_string(),
                        latency_ms: Some(latency_ms),
                        fatal: false,
                        response: None,
                    };
                }
            };
            if adu.unit_id != unit_id {
                return ProbeOutcome {
                    status: "error",
                    detail: format!("从站 ID 不匹配: {}", adu.unit_id),
                    latency_ms: Some(latency_ms),
                    fatal: false,
                    response: None,
                };
            }
            match PduResponse::decode_for_request(request, &adu.pdu) {
                Ok(response) => {
                    let (status, detail) = describe_response(&response);
                    ProbeOutcome {
                        status,
                        detail,
                        latency_ms: Some(latency_ms),
                        fatal: false,
                        response: Some(response),
                    }
                }
                Err(error) => ProbeOutcome {
                    status: "error",
                    detail: error.to_string(),
                    latency_ms: Some(latency_ms),
                    fatal: false,
                    response: None,
                },
            }
        }
        Err(error) => {
            let status = match error {
                TransportError::Timeout { .. } => "timeout",
                _ => "error",
            };
            let fatal = matches!(
                error,
                TransportError::Closed | TransportError::Io(_) | TransportError::Serial(_)
            );
            ProbeOutcome {
                status,
                detail: error.to_string(),
                latency_ms: None,
                fatal,
                response: None,
            }
        }
    }
}

fn describe_response(response: &PduResponse) -> (&'static str, String) {
    match response {
        PduResponse::Exception { code, .. } => (
            "exception",
            format!("异常 0x{:02X} ({})", code.as_u8(), code.description()),
        ),
        PduResponse::ReadRegisters { values, .. } => ("ok", format!("{} 寄存器", values.len())),
        PduResponse::ReadBits { values, .. } => ("ok", format!("{} 位", values.len())),
        PduResponse::WriteSingleAck { .. } | PduResponse::WriteMultipleAck { .. } => {
            ("ok", "写入确认".to_owned())
        }
    }
}

fn read_request(function_code: u8, address: u16, quantity: u16) -> Result<PduRequest, String> {
    match function_code {
        0x01 => Ok(PduRequest::ReadCoils { address, quantity }),
        0x02 => Ok(PduRequest::ReadDiscreteInputs { address, quantity }),
        0x03 => Ok(PduRequest::ReadHoldingRegisters { address, quantity }),
        0x04 => Ok(PduRequest::ReadInputRegisters { address, quantity }),
        other => Err(format!("扫描仅支持 FC01–FC04，收到 0x{other:02X}")),
    }
}

async fn probe_delay(link: &LinkConfig, framing: Framing) {
    if framing == Framing::Rtu
        && let LinkConfig::Serial(serial) = link
    {
        sleep(Duration::from_micros(frame::t35_micros(serial.baud_rate))).await;
    }
}

async fn run_slave_scan(
    scan_id: String,
    input: SlaveIdScanInput,
    cancel: Arc<AtomicBool>,
    bus: EventBus,
) {
    let kind = ScanKind::SlaveId;
    if input.id_start > input.id_end {
        finish_with_error(&bus, &scan_id, kind, "起始 ID 不能大于结束 ID");
        return;
    }
    let link = match input.link.link() {
        Ok(link) => link,
        Err(error) => {
            finish_with_error(&bus, &scan_id, kind, &error);
            return;
        }
    };
    let framing = input.link.framing;
    let timeout_duration = input.link.timeout();
    let request = match read_request(input.function_code, input.address, input.quantity) {
        Ok(request) => request,
        Err(error) => {
            finish_with_error(&bus, &scan_id, kind, &error);
            return;
        }
    };
    let mut transport =
        match Transport::connect(&link, timeout_duration.max(Duration::from_secs(2))).await {
            Ok(transport) => transport,
            Err(error) => {
                finish_with_error(&bus, &scan_id, kind, &error.to_string());
                return;
            }
        };

    let total = u64::from(input.id_end - input.id_start) + 1;
    let mut done = 0u64;
    let mut found = 0u64;
    let mut transaction_id = 0u16;
    let mut error: Option<String> = None;
    emit_progress(
        &bus,
        &scan_id,
        kind,
        0,
        total,
        0,
        "探测从站 ID",
        false,
        None,
    );

    for unit_id in input.id_start..=input.id_end {
        if cancel.load(Ordering::Relaxed) {
            error = Some("扫描已取消".to_owned());
            break;
        }
        if unit_id == 0 {
            done += 1;
            emit_progress(
                &bus,
                &scan_id,
                kind,
                done,
                total,
                found,
                "探测从站 ID",
                false,
                None,
            );
            continue;
        }

        transaction_id = transaction_id.wrapping_add(1);
        if transaction_id == 0 {
            transaction_id = 1;
        }
        let outcome = probe_once(
            &mut transport,
            framing,
            unit_id,
            &request,
            timeout_duration,
            transaction_id,
        )
        .await;
        if outcome.status == "ok" || outcome.status == "exception" {
            found += 1;
        }
        emit_result(
            &bus,
            ScanResultDto {
                scan_id: scan_id.clone(),
                kind,
                label: format!("ID {unit_id}"),
                status: outcome.status.to_owned(),
                detail: outcome.detail.clone(),
                latency_ms: outcome.latency_ms,
                slave_id: Some(unit_id),
                address: Some(input.address),
                baud_rate: None,
                endpoint: None,
                port: None,
                service: None,
                banner: None,
            },
        );
        done += 1;
        emit_progress(
            &bus,
            &scan_id,
            kind,
            done,
            total,
            found,
            "探测从站 ID",
            false,
            None,
        );
        if outcome.fatal {
            error = Some(outcome.detail);
            break;
        }
        probe_delay(&link, framing).await;
    }

    emit_progress(
        &bus,
        &scan_id,
        kind,
        done,
        total,
        found,
        "探测从站 ID",
        true,
        error.clone(),
    );
    emit(
        &bus,
        AppEvent::ScanDone(ScanDoneDto {
            scan_id,
            kind,
            found,
            error,
        }),
    );
}

async fn run_address_scan(
    scan_id: String,
    input: AddressScanInput,
    cancel: Arc<AtomicBool>,
    bus: EventBus,
) {
    let kind = ScanKind::Address;
    if input.start_address > input.end_address {
        finish_with_error(&bus, &scan_id, kind, "起始地址不能大于结束地址");
        return;
    }
    let link = match input.link.link() {
        Ok(link) => link,
        Err(error) => {
            finish_with_error(&bus, &scan_id, kind, &error);
            return;
        }
    };
    let framing = input.link.framing;
    let timeout_duration = input.link.timeout();
    let block_limit = if (1..=2).contains(&input.function_code) {
        2000
    } else {
        125
    };
    let block_size = input.block_size.clamp(1, block_limit);
    let total_span = u32::from(input.end_address) - u32::from(input.start_address) + 1;
    let total = u64::from(total_span.div_ceil(u32::from(block_size)));
    let mut transport =
        match Transport::connect(&link, timeout_duration.max(Duration::from_secs(2))).await {
            Ok(transport) => transport,
            Err(error) => {
                finish_with_error(&bus, &scan_id, kind, &error.to_string());
                return;
            }
        };

    let mut done = 0u64;
    let mut found = 0u64;
    let mut transaction_id = 0u16;
    let mut error: Option<String> = None;
    emit_progress(
        &bus,
        &scan_id,
        kind,
        0,
        total,
        0,
        "探测地址范围",
        false,
        None,
    );

    let mut block_start = input.start_address;
    while block_start <= input.end_address {
        if cancel.load(Ordering::Relaxed) {
            error = Some("扫描已取消".to_owned());
            break;
        }
        let remaining = u32::from(input.end_address) - u32::from(block_start) + 1;
        let quantity = u16::try_from(remaining.min(u32::from(block_size))).unwrap_or(block_size);
        let request = match read_request(input.function_code, block_start, quantity) {
            Ok(request) => request,
            Err(message) => {
                error = Some(message);
                break;
            }
        };
        transaction_id = transaction_id.wrapping_add(1);
        if transaction_id == 0 {
            transaction_id = 1;
        }
        let outcome = probe_once(
            &mut transport,
            framing,
            input.slave_id,
            &request,
            timeout_duration,
            transaction_id,
        )
        .await;
        if outcome.status == "ok"
            && let Some(response) = &outcome.response
        {
            let (readable, non_zero, first_non_zero) = response_stats(response, block_start);
            if !input.non_zero_only || non_zero > 0 {
                found += 1;
                let block_end = block_start.saturating_add(quantity - 1);
                let mut detail = format!("可读 {readable} · 非零 {non_zero}");
                if let Some(address) = first_non_zero {
                    detail.push_str(&format!(" · 首个非零 {address:05}"));
                }
                emit_result(
                    &bus,
                    ScanResultDto {
                        scan_id: scan_id.clone(),
                        kind,
                        label: format!("{block_start:05}–{block_end:05}"),
                        status: "ok".to_owned(),
                        detail,
                        latency_ms: outcome.latency_ms,
                        slave_id: None,
                        address: Some(block_start),
                        baud_rate: None,
                        endpoint: None,
                        port: None,
                        service: None,
                        banner: None,
                    },
                );
            }
        }
        done += 1;
        emit_progress(
            &bus,
            &scan_id,
            kind,
            done,
            total,
            found,
            "探测地址范围",
            false,
            None,
        );
        if outcome.fatal {
            error = Some(outcome.detail);
            break;
        }
        if u32::from(block_start) + u32::from(quantity) > u32::from(input.end_address) {
            break;
        }
        block_start = block_start.wrapping_add(quantity);
        probe_delay(&link, framing).await;
    }

    emit_progress(
        &bus,
        &scan_id,
        kind,
        done,
        total,
        found,
        "探测地址范围",
        true,
        error.clone(),
    );
    emit(
        &bus,
        AppEvent::ScanDone(ScanDoneDto {
            scan_id,
            kind,
            found,
            error,
        }),
    );
}

fn response_stats(response: &PduResponse, start_address: u16) -> (usize, usize, Option<u16>) {
    match response {
        PduResponse::ReadRegisters { values, .. } => {
            let non_zero = values.iter().filter(|value| **value != 0).count();
            let first = values
                .iter()
                .position(|value| *value != 0)
                .map(|index| start_address.wrapping_add(index as u16));
            (values.len(), non_zero, first)
        }
        PduResponse::ReadBits { values, .. } => {
            let non_zero = values.iter().filter(|value| **value).count();
            let first = values
                .iter()
                .position(|value| *value)
                .map(|index| start_address.wrapping_add(index as u16));
            (values.len(), non_zero, first)
        }
        _ => (0, 0, None),
    }
}

async fn run_baud_scan(
    scan_id: String,
    input: BaudScanInput,
    cancel: Arc<AtomicBool>,
    bus: EventBus,
) {
    let kind = ScanKind::Baud;
    if input.framing == Framing::Mbap {
        finish_with_error(&bus, &scan_id, kind, "波特率扫描仅支持 RTU / ASCII");
        return;
    }
    let mut baud_rates = input.baud_rates.clone();
    baud_rates.sort_unstable();
    baud_rates.dedup();
    baud_rates.retain(|baud| *baud > 0);
    baud_rates.truncate(20);
    if baud_rates.is_empty() {
        finish_with_error(&bus, &scan_id, kind, "请选择至少一个波特率");
        return;
    }
    let request = match read_request(input.function_code, input.address, input.quantity) {
        Ok(request) => request,
        Err(error) => {
            finish_with_error(&bus, &scan_id, kind, &error);
            return;
        }
    };
    let parity = match input.parity.as_deref().unwrap_or("none") {
        "even" => SerialParity::Even,
        "odd" => SerialParity::Odd,
        "none" | "" => SerialParity::None,
        other => {
            finish_with_error(&bus, &scan_id, kind, &format!("未知校验方式: {other}"));
            return;
        }
    };
    let timeout_duration = Duration::from_millis(input.timeout_ms.clamp(50, 5000));
    let total = baud_rates.len() as u64;
    let mut done = 0u64;
    let mut found = 0u64;
    let mut error: Option<String> = None;
    emit_progress(&bus, &scan_id, kind, 0, total, 0, "尝试波特率", false, None);

    for baud_rate in baud_rates {
        if cancel.load(Ordering::Relaxed) {
            error = Some("扫描已取消".to_owned());
            break;
        }
        let link = LinkConfig::Serial(SerialLinkConfig {
            port: input.serial_port.clone(),
            baud_rate,
            data_bits: input.data_bits.unwrap_or(8).clamp(5, 8),
            parity,
            stop_bits: input.stop_bits.unwrap_or(1).clamp(1, 2),
        });
        let outcome = match Transport::connect(&link, Duration::from_secs(2)).await {
            Ok(mut transport) => {
                probe_once(
                    &mut transport,
                    input.framing,
                    input.slave_id,
                    &request,
                    timeout_duration,
                    1,
                )
                .await
            }
            Err(error) => ProbeOutcome {
                status: "error",
                detail: error.to_string(),
                latency_ms: None,
                fatal: false,
                response: None,
            },
        };
        if outcome.status == "ok" || outcome.status == "exception" {
            found += 1;
        }
        emit_result(
            &bus,
            ScanResultDto {
                scan_id: scan_id.clone(),
                kind,
                label: format!("{baud_rate} bps"),
                status: outcome.status.to_owned(),
                detail: outcome.detail,
                latency_ms: outcome.latency_ms,
                slave_id: Some(input.slave_id),
                address: Some(input.address),
                baud_rate: Some(baud_rate),
                endpoint: Some(input.serial_port.clone()),
                port: None,
                service: None,
                banner: None,
            },
        );
        done += 1;
        emit_progress(
            &bus,
            &scan_id,
            kind,
            done,
            total,
            found,
            "尝试波特率",
            false,
            None,
        );
        sleep(Duration::from_millis(60)).await;
    }

    emit_progress(
        &bus,
        &scan_id,
        kind,
        done,
        total,
        found,
        "尝试波特率",
        true,
        error.clone(),
    );
    emit(
        &bus,
        AppEvent::ScanDone(ScanDoneDto {
            scan_id,
            kind,
            found,
            error,
        }),
    );
}

async fn run_network_scan(
    scan_id: String,
    input: NetworkScanInput,
    cancel: Arc<AtomicBool>,
    bus: EventBus,
) {
    let kind = ScanKind::Network;
    let network: IpNet = match input.cidr.trim().parse() {
        Ok(network) => network,
        Err(_) => {
            finish_with_error(&bus, &scan_id, kind, "CIDR 格式无效，例如 192.168.1.0/24");
            return;
        }
    };
    let IpNet::V4(network) = network else {
        finish_with_error(&bus, &scan_id, kind, "目前仅支持 IPv4 CIDR");
        return;
    };
    let hosts: Vec<Ipv4Addr> = network.hosts().collect();
    if hosts.is_empty() {
        finish_with_error(&bus, &scan_id, kind, "CIDR 内没有可扫描主机");
        return;
    }
    if hosts.len() > 65_536 {
        finish_with_error(
            &bus,
            &scan_id,
            kind,
            "扫描范围过大，请使用 /16 或更小的网段",
        );
        return;
    }
    let ports = match input.effective_ports() {
        Ok(ports) => ports,
        Err(message) => {
            finish_with_error(&bus, &scan_id, kind, &message);
            return;
        }
    };
    let total = (hosts.len() * ports.len()) as u64;
    if total > 65_536 {
        finish_with_error(
            &bus,
            &scan_id,
            kind,
            "扫描组合过大，请减少主机或端口数量（最多 65536 次探测）",
        );
        return;
    }
    let timeout_duration = Duration::from_millis(input.timeout_ms.clamp(100, 5000));
    let concurrency = input.concurrency.clamp(1, 128);
    let mut done = 0u64;
    let mut found = 0u64;
    let mut cancelled = false;
    emit_progress(&bus, &scan_id, kind, 0, total, 0, "扫描端口", false, None);

    let endpoints: Vec<(Ipv4Addr, u16)> = hosts
        .into_iter()
        .flat_map(|host| ports.iter().copied().map(move |port| (host, port)))
        .collect();
    let mut stream = futures::stream::iter(endpoints.into_iter().map(|(host, port)| {
        let scan_id = scan_id.clone();
        let cancel = cancel.clone();
        let probe_modbus = input.probe_modbus && port == 502;
        let grab_banner = input.grab_banner;
        let slave_id = input.slave_id;
        async move {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            probe_endpoint(
                scan_id,
                host,
                port,
                timeout_duration,
                probe_modbus,
                slave_id,
                grab_banner,
            )
            .await
        }
    }))
    .buffer_unordered(concurrency);

    while let Some(result) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        done += 1;
        if let Some(result) = result {
            found += 1;
            emit(&bus, AppEvent::ScanResult(result));
        }
        if done.is_multiple_of(32) || done == total {
            emit_progress(
                &bus,
                &scan_id,
                kind,
                done,
                total,
                found,
                "扫描端口",
                false,
                None,
            );
        }
    }

    drop(stream);
    let error = if cancelled {
        Some("扫描已取消".to_owned())
    } else {
        None
    };
    emit_progress(
        &bus,
        &scan_id,
        kind,
        done,
        total,
        found,
        "扫描端口",
        true,
        error.clone(),
    );
    emit(
        &bus,
        AppEvent::ScanDone(ScanDoneDto {
            scan_id,
            kind,
            found,
            error,
        }),
    );
}

#[allow(clippy::too_many_arguments)] // Probe keeps the scan context explicit.
async fn probe_endpoint(
    scan_id: String,
    host: Ipv4Addr,
    port: u16,
    timeout_duration: Duration,
    probe_modbus: bool,
    slave_id: u8,
    grab_banner: bool,
) -> Option<ScanResultDto> {
    let started = Instant::now();
    let mut stream = timeout(timeout_duration, TcpStream::connect((host, port)))
        .await
        .ok()?
        .ok()?;
    let latency_ms = round1(started.elapsed().as_secs_f64() * 1000.0);
    let endpoint = format!("{host}:{port}");
    let service = service_name(port).map(str::to_owned);

    if !probe_modbus {
        let banner = if grab_banner {
            read_banner(&mut stream).await
        } else {
            None
        };
        return Some(ScanResultDto {
            scan_id,
            kind: ScanKind::Network,
            label: endpoint.clone(),
            status: "open".to_owned(),
            detail: "TCP 端口开放".to_owned(),
            latency_ms: Some(latency_ms),
            slave_id: None,
            address: None,
            baud_rate: None,
            endpoint: Some(endpoint),
            port: Some(port),
            service,
            banner,
        });
    }

    let mut transport = TcpTransport::from_stream(stream);
    let request = PduRequest::ReadHoldingRegisters {
        address: 0,
        quantity: 1,
    };
    let pdu = request.encode().ok()?;
    let request_frame = frame::encode(Framing::Mbap, slave_id, Some(1), &pdu).ok()?;
    let (status, detail) = match transport
        .transaction(&request_frame, Framing::Mbap, timeout_duration)
        .await
    {
        Ok(response) => match frame::decode(Framing::Mbap, &response) {
            Ok(adu) => match PduResponse::decode_for_request(&request, &adu.pdu) {
                Ok(PduResponse::Exception { code, .. }) => (
                    "exception".to_owned(),
                    format!("Modbus 从站存在 · 异常 0x{:02X}", code.as_u8()),
                ),
                Ok(_) => ("ok".to_owned(), "Modbus 从站响应".to_owned()),
                Err(_) => ("open".to_owned(), "端口开放，响应非 Modbus".to_owned()),
            },
            Err(_) => ("open".to_owned(), "端口开放，响应非 Modbus".to_owned()),
        },
        Err(_) => ("open".to_owned(), "端口开放，Modbus 探测无响应".to_owned()),
    };
    Some(ScanResultDto {
        scan_id,
        kind: ScanKind::Network,
        label: endpoint.clone(),
        status,
        detail,
        latency_ms: Some(latency_ms),
        slave_id: Some(slave_id),
        address: None,
        baud_rate: None,
        endpoint: Some(endpoint),
        port: Some(port),
        service,
        banner: None,
    })
}

async fn read_banner(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = [0u8; 512];
    let read = timeout(Duration::from_millis(300), stream.read(&mut buffer))
        .await
        .ok()?
        .ok()?;
    if read == 0 {
        return None;
    }
    let text = String::from_utf8_lossy(&buffer[..read]);
    let cleaned: String = text
        .chars()
        .take(512)
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
        .collect();
    let trimmed = cleaned.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(200).collect())
}

fn service_name(port: u16) -> Option<&'static str> {
    match port {
        20 | 21 => Some("FTP"),
        22 => Some("SSH"),
        23 => Some("Telnet"),
        25 => Some("SMTP"),
        53 => Some("DNS"),
        80 | 8080 => Some("HTTP"),
        110 => Some("POP3"),
        143 => Some("IMAP"),
        443 | 8443 => Some("HTTPS"),
        445 => Some("SMB"),
        502 => Some("Modbus"),
        1433 => Some("SQL Server"),
        3306 => Some("MySQL"),
        3389 => Some("RDP"),
        5432 => Some("PostgreSQL"),
        6379 => Some("Redis"),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)] // Reporting helper with a fixed, explicit shape.
fn emit_progress(
    bus: &EventBus,
    scan_id: &str,
    kind: ScanKind,
    done: u64,
    total: u64,
    found: u64,
    phase: &str,
    finished: bool,
    error: Option<String>,
) {
    emit(
        bus,
        AppEvent::ScanProgress(ScanProgressDto {
            scan_id: scan_id.to_owned(),
            kind,
            done,
            total,
            found,
            phase: phase.to_owned(),
            finished,
            error,
        }),
    );
}

fn emit_result(bus: &EventBus, result: ScanResultDto) {
    emit(bus, AppEvent::ScanResult(result));
}

fn finish_with_error(bus: &EventBus, scan_id: &str, kind: ScanKind, message: &str) {
    emit_progress(
        bus,
        scan_id,
        kind,
        0,
        0,
        0,
        "扫描失败",
        true,
        Some(message.to_owned()),
    );
    emit(
        bus,
        AppEvent::ScanDone(ScanDoneDto {
            scan_id: scan_id.to_owned(),
            kind,
            found: 0,
            error: Some(message.to_owned()),
        }),
    );
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use modbus_core::simulator::{Simulator, SlaveUnit};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::{Mutex, broadcast};

    use crate::event::event_bus;
    use crate::model::{ScanLinkInput, TransportKind};

    async fn mock_tcp_slave(listener: TcpListener, simulator: Arc<Mutex<Simulator>>) {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let simulator = simulator.clone();
            tokio::spawn(async move {
                loop {
                    let mut header = [0u8; 6];
                    if socket.read_exact(&mut header).await.is_err() {
                        break;
                    }
                    let length = usize::from(u16::from_be_bytes([header[4], header[5]]));
                    let mut rest = vec![0u8; length];
                    if socket.read_exact(&mut rest).await.is_err() {
                        break;
                    }
                    let mut request = header.to_vec();
                    request.extend_from_slice(&rest);
                    let Ok(adu) = frame::decode(Framing::Mbap, &request) else {
                        break;
                    };
                    let response = {
                        let mut simulator = simulator.lock().await;
                        simulator.handle_request(adu.unit_id, &adu.pdu)
                    };
                    if let Some(pdu) = response {
                        let Ok(response) =
                            frame::encode(Framing::Mbap, adu.unit_id, adu.transaction_id, &pdu)
                        else {
                            break;
                        };
                        if socket.write_all(&response).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }
    }

    async fn wait_for_done(
        events: &mut broadcast::Receiver<AppEvent>,
        scan_id: &str,
        results: &mut Vec<ScanResultDto>,
    ) -> ScanDoneDto {
        loop {
            match timeout(Duration::from_secs(8), events.recv()).await {
                Ok(Ok(AppEvent::ScanResult(result))) if result.scan_id == scan_id => {
                    results.push(result);
                }
                Ok(Ok(AppEvent::ScanDone(done))) if done.scan_id == scan_id => {
                    return done;
                }
                Ok(Ok(_)) => {}
                Ok(Err(error)) => panic!("event bus error: {error}"),
                Err(_) => panic!("scan timed out"),
            }
        }
    }

    fn link(port: u16) -> ScanLinkInput {
        ScanLinkInput {
            transport: TransportKind::Tcp,
            framing: Framing::Mbap,
            host: Some("127.0.0.1".to_owned()),
            port: Some(port),
            serial_port: None,
            baud_rate: None,
            data_bits: None,
            parity: None,
            stop_bits: None,
            timeout_ms: Some(120),
        }
    }

    #[tokio::test]
    async fn scans_slave_ids_and_finds_responding_unit() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let simulator = Arc::new(Mutex::new(Simulator::new([
            SlaveUnit::new(1).with_holding_registers([(0, 42)])
        ])));
        let server = tokio::spawn(mock_tcp_slave(listener, simulator));

        let bus = event_bus();
        let mut events = bus.subscribe();
        let manager = ScanManager::new(bus);
        let scan_id = manager.start_slave_scan(SlaveIdScanInput {
            link: link(port),
            function_code: 3,
            address: 0,
            quantity: 1,
            id_start: 1,
            id_end: 3,
        });

        let mut results = Vec::new();
        let done = wait_for_done(&mut events, &scan_id, &mut results).await;
        assert_eq!(done.found, 1);
        assert_eq!(results.len(), 3);
        let found = results
            .iter()
            .find(|result| result.slave_id == Some(1))
            .expect("unit 1 result");
        assert_eq!(found.status, "ok");
        assert!(results.iter().any(|result| result.status == "timeout"));
        server.abort();
    }

    #[tokio::test]
    async fn scans_network_ports() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let bus = event_bus();
        let mut events = bus.subscribe();
        let manager = ScanManager::new(bus);
        let scan_id = manager.start_network_scan(NetworkScanInput {
            cidr: "127.0.0.0/30".to_owned(),
            port,
            ports: Vec::new(),
            timeout_ms: 400,
            concurrency: 4,
            probe_modbus: false,
            grab_banner: false,
            slave_id: 1,
        });

        let mut results = Vec::new();
        let done = wait_for_done(&mut events, &scan_id, &mut results).await;
        assert!(done.found >= 1, "loopback port should be detected");
        assert!(
            results
                .iter()
                .any(|result| matches!(result.status.as_str(), "open" | "ok" | "exception"))
        );
        drop(listener);
    }

    #[tokio::test]
    async fn scans_multiple_network_ports() {
        let first = TcpListener::bind("127.0.0.1:0").await.expect("bind first");
        let second = TcpListener::bind("127.0.0.1:0").await.expect("bind second");
        let first_port = first.local_addr().expect("first addr").port();
        let second_port = second.local_addr().expect("second addr").port();
        let bus = event_bus();
        let mut events = bus.subscribe();
        let manager = ScanManager::new(bus);
        let scan_id = manager.start_network_scan(NetworkScanInput {
            cidr: "127.0.0.0/30".to_owned(),
            port: 502,
            ports: vec![first_port, second_port, first_port],
            timeout_ms: 400,
            concurrency: 4,
            probe_modbus: false,
            grab_banner: false,
            slave_id: 1,
        });

        let mut results = Vec::new();
        let done = wait_for_done(&mut events, &scan_id, &mut results).await;
        assert_eq!(done.found, 2);
        assert!(results.iter().any(|result| result.port == Some(first_port)));
        assert!(
            results
                .iter()
                .any(|result| result.port == Some(second_port))
        );
        drop(first);
        drop(second);
    }

    #[test]
    fn network_scan_input_dedupes_ports_and_caps_them() {
        let mut input = NetworkScanInput {
            cidr: "127.0.0.1/32".to_owned(),
            port: 502,
            ports: vec![8080, 80, 8080, 0, 443],
            timeout_ms: 400,
            concurrency: 4,
            probe_modbus: false,
            grab_banner: false,
            slave_id: 1,
        };
        assert_eq!(input.effective_ports().expect("ports"), vec![80, 443, 8080]);
        input.ports = (1..=257).collect();
        assert!(input.effective_ports().is_err());
        input.ports.clear();
        assert_eq!(input.effective_ports().expect("legacy port"), vec![502]);
    }
}
