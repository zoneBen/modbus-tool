//! Per-session actor: owns one link and serializes all transactions.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{Instant, sleep, sleep_until};

use modbus_core::frame;
use modbus_core::value::{decode_words, encode_value};
use modbus_core::{DataType, Framing, PduRequest, PduResponse};

use crate::event::{AppEvent, EventBus, emit};
use crate::frames::{FrameInfo, build_frame};
use crate::model::{
    DefinitionDto, DefinitionInput, DefinitionSnapshotDto, FrameDto, LinkConfig, RegisterValueDto,
    SessionState, SessionStatus, now_ms,
};
use crate::transport::{Transport, TransportError};

pub enum SessionCommand {
    Connect {
        reply: oneshot::Sender<Result<(), String>>,
    },
    Disconnect {
        reply: oneshot::Sender<Result<(), String>>,
    },
    SetPolling {
        enabled: bool,
        reply: oneshot::Sender<Result<(), String>>,
    },
    ReadOnce {
        definition_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SetDefinitionEnabled {
        definition_id: String,
        enabled: bool,
        reply: oneshot::Sender<Result<(), String>>,
    },
    CreateDefinition {
        input: DefinitionInput,
        reply: oneshot::Sender<Result<DefinitionDto, String>>,
    },
    UpdateDefinition {
        definition_id: String,
        input: DefinitionInput,
        reply: oneshot::Sender<Result<DefinitionDto, String>>,
    },
    DeleteDefinition {
        definition_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    WriteRegister {
        definition_id: String,
        address: u16,
        value: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
enum ReadFailure {
    Protocol(String),
    Transport(String),
}

impl ReadFailure {
    fn message(&self) -> String {
        match self {
            Self::Protocol(message) | Self::Transport(message) => message.clone(),
        }
    }
}

#[derive(Debug, Clone)]
enum TransactionError {
    Protocol(String),
    Transport(String),
}

impl TransactionError {
    fn message(self) -> String {
        match self {
            Self::Protocol(message) | Self::Transport(message) => message,
        }
    }

    fn into_read_failure(self) -> ReadFailure {
        match self {
            Self::Protocol(message) => ReadFailure::Protocol(message),
            Self::Transport(message) => ReadFailure::Transport(message),
        }
    }
}

pub struct Actor {
    state: Arc<RwLock<SessionState>>,
    bus: EventBus,
    commands: Option<mpsc::UnboundedReceiver<SessionCommand>>,
    seq: Arc<AtomicU64>,
    transport: Option<Transport>,
    next_tid: u16,
    last_run: HashMap<String, Instant>,
}

impl Actor {
    pub fn new(
        state: Arc<RwLock<SessionState>>,
        bus: EventBus,
        commands: mpsc::UnboundedReceiver<SessionCommand>,
        seq: Arc<AtomicU64>,
    ) -> Self {
        Self {
            state,
            bus,
            commands: Some(commands),
            seq,
            transport: None,
            next_tid: 0,
            last_run: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        let Some(mut commands) = self.commands.take() else {
            return;
        };
        loop {
            let next_due = self.next_due().await;
            let command = tokio::select! {
                command = commands.recv() => command,
                _ = wait_until(next_due) => {
                    self.poll_due().await;
                    continue;
                }
            };
            match command {
                Some(command) => {
                    if !self.handle_command(command).await {
                        break;
                    }
                }
                None => break,
            }
        }
        self.transport = None;
    }

    async fn handle_command(&mut self, command: SessionCommand) -> bool {
        match command {
            SessionCommand::Connect { reply } => {
                let result = self.connect().await;
                let _ = reply.send(result);
                true
            }
            SessionCommand::Disconnect { reply } => {
                self.disconnect().await;
                let _ = reply.send(Ok(()));
                true
            }
            SessionCommand::SetPolling { enabled, reply } => {
                {
                    let mut state = self.state.write().await;
                    state.polling = enabled;
                }
                self.last_run.clear();
                self.emit_status(None).await;
                let _ = reply.send(Ok(()));
                true
            }
            SessionCommand::ReadOnce {
                definition_id,
                reply,
            } => {
                let result = self.read_once(&definition_id).await;
                if matches!(&result, Err(ReadFailure::Transport(_))) {
                    self.reconnect().await;
                }
                let _ = reply.send(result.map_err(|failure| failure.message()));
                true
            }
            SessionCommand::SetDefinitionEnabled {
                definition_id,
                enabled,
                reply,
            } => {
                let result = self.set_definition_enabled(&definition_id, enabled).await;
                let _ = reply.send(result);
                true
            }
            SessionCommand::CreateDefinition { input, reply } => {
                let result = self.create_definition(input).await;
                let _ = reply.send(result);
                true
            }
            SessionCommand::UpdateDefinition {
                definition_id,
                input,
                reply,
            } => {
                let result = self.update_definition(&definition_id, input).await;
                let _ = reply.send(result);
                true
            }
            SessionCommand::DeleteDefinition {
                definition_id,
                reply,
            } => {
                let result = self.delete_definition(&definition_id).await;
                let _ = reply.send(result);
                true
            }
            SessionCommand::WriteRegister {
                definition_id,
                address,
                value,
                reply,
            } => {
                let result = self.write_by_id(&definition_id, address, &value).await;
                if matches!(&result, Err(TransactionError::Transport(_))) {
                    self.reconnect().await;
                }
                let _ = reply.send(result.map_err(TransactionError::message));
                true
            }
            SessionCommand::Shutdown => false,
        }
    }

    async fn connect(&mut self) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            state.status = SessionStatus::Connecting;
        }
        self.emit_status(None).await;

        let (link, timeout_ms) = {
            let state = self.state.read().await;
            (state.config.link.clone(), state.config.timeout_ms.max(3000))
        };
        match Transport::connect(&link, Duration::from_millis(timeout_ms)).await {
            Ok(transport) => {
                self.transport = Some(transport);
                {
                    let mut state = self.state.write().await;
                    state.status = SessionStatus::Connected;
                }
                self.last_run.clear();
                self.emit_status(None).await;
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                {
                    let mut state = self.state.write().await;
                    state.status = SessionStatus::Error;
                }
                self.emit_status(Some(message.clone())).await;
                Err(message)
            }
        }
    }

    async fn disconnect(&mut self) {
        self.transport = None;
        self.last_run.clear();
        {
            let mut state = self.state.write().await;
            state.status = SessionStatus::Disconnected;
        }
        self.emit_status(None).await;
    }

    async fn reconnect(&mut self) {
        let auto_reconnect = { self.state.read().await.config.auto_reconnect };
        if !auto_reconnect {
            return;
        }
        self.transport = None;
        let mut last_error = "连接已断开".to_owned();
        for attempt in 1..=3u32 {
            {
                let mut state = self.state.write().await;
                state.status = SessionStatus::Connecting;
            }
            self.emit_status(Some(format!("重连中 {attempt}/3"))).await;
            let (link, timeout_ms) = {
                let state = self.state.read().await;
                (state.config.link.clone(), state.config.timeout_ms.max(3000))
            };
            match Transport::connect(&link, Duration::from_millis(timeout_ms)).await {
                Ok(transport) => {
                    self.transport = Some(transport);
                    {
                        let mut state = self.state.write().await;
                        state.status = SessionStatus::Connected;
                    }
                    self.last_run.clear();
                    self.emit_status(None).await;
                    return;
                }
                Err(error) => {
                    last_error = error.to_string();
                    if attempt < 3 {
                        sleep(Duration::from_secs(u64::from(attempt))).await;
                    }
                }
            }
        }
        {
            let mut state = self.state.write().await;
            state.status = SessionStatus::Error;
        }
        self.emit_status(Some(last_error)).await;
    }

    async fn next_due(&self) -> Option<Instant> {
        let state = self.state.read().await;
        if state.status != SessionStatus::Connected || !state.polling {
            return None;
        }
        let now = Instant::now();
        state
            .config
            .definitions
            .iter()
            .filter(|definition| definition.enabled && definition.is_read())
            .map(|definition| {
                let interval = Duration::from_millis(definition.interval_ms.max(10));
                match self.last_run.get(&definition.id) {
                    Some(last) => {
                        let due = *last + interval;
                        if due < now { now } else { due }
                    }
                    None => now,
                }
            })
            .min()
    }

    async fn poll_due(&mut self) {
        let due: Vec<DefinitionDto> = {
            let state = self.state.read().await;
            if state.status != SessionStatus::Connected || !state.polling {
                return;
            }
            let now = Instant::now();
            state
                .config
                .definitions
                .iter()
                .filter(|definition| definition.enabled && definition.is_read())
                .filter(|definition| {
                    let interval = Duration::from_millis(definition.interval_ms.max(10));
                    self.last_run
                        .get(&definition.id)
                        .map(|last| *last + interval <= now)
                        .unwrap_or(true)
                })
                .cloned()
                .collect()
        };

        for definition in due {
            let outcome = self.read_definition(&definition).await;
            self.last_run.insert(definition.id.clone(), Instant::now());
            if let Err(ReadFailure::Transport(_)) = outcome {
                self.reconnect().await;
                break;
            }
        }
    }

    async fn read_once(&mut self, definition_id: &str) -> Result<(), ReadFailure> {
        let definition = self
            .definition(definition_id)
            .await
            .ok_or_else(|| ReadFailure::Protocol("定义不存在".to_owned()))?;
        self.read_definition(&definition).await
    }

    async fn read_definition(&mut self, definition: &DefinitionDto) -> Result<(), ReadFailure> {
        let request = definition
            .read_request()
            .map_err(|error| ReadFailure::Protocol(error.to_string()))?;

        let (response, latency_ms) = match self.transact(&request, definition.slave_id, true).await
        {
            Ok(outcome) => outcome,
            Err(error) => {
                let failure = error.into_read_failure();
                let message = failure.message();
                self.emit_data(definition, Vec::new(), 0.0, Some(message))
                    .await;
                self.wait_inter_frame().await;
                return Err(failure);
            }
        };

        if let PduResponse::Exception { code, .. } = &response {
            let message = format!(
                "Modbus 异常 0x{:02X} ({})",
                code.as_u8(),
                code.description()
            );
            self.emit_data(definition, Vec::new(), latency_ms, Some(message.clone()))
                .await;
            self.wait_inter_frame().await;
            return Err(ReadFailure::Protocol(message));
        }

        match build_values(definition, &response) {
            Ok(values) => {
                self.emit_data(definition, values, latency_ms, None).await;
                self.wait_inter_frame().await;
                Ok(())
            }
            Err(message) => {
                self.emit_data(definition, Vec::new(), latency_ms, Some(message.clone()))
                    .await;
                self.wait_inter_frame().await;
                Err(ReadFailure::Protocol(message))
            }
        }
    }

    async fn write_by_id(
        &mut self,
        definition_id: &str,
        address: u16,
        value: &str,
    ) -> Result<(), TransactionError> {
        let definition = self
            .definition(definition_id)
            .await
            .ok_or_else(|| TransactionError::Protocol("定义不存在".to_owned()))?;
        self.write_value(&definition, address, value).await
    }

    async fn write_value(
        &mut self,
        definition: &DefinitionDto,
        address: u16,
        value: &str,
    ) -> Result<(), TransactionError> {
        let rule = definition.rule();
        let words = encode_value(&rule, value)
            .map_err(|error| TransactionError::Protocol(error.to_string()))?;
        if words.is_empty() {
            return Err(TransactionError::Protocol("写入值为空".to_owned()));
        }

        let request = match rule.data_type {
            DataType::Bool => PduRequest::WriteSingleCoil {
                address,
                value: words[0] != 0,
            },
            _ if words.len() == 1 => PduRequest::WriteSingleRegister {
                address,
                value: words[0],
            },
            _ => PduRequest::WriteMultipleRegisters {
                address,
                values: words,
            },
        };

        let (response, _) = self.transact(&request, definition.slave_id, false).await?;
        self.wait_inter_frame().await;

        match response {
            PduResponse::Exception { code, .. } => {
                Err(TransactionError::Protocol(format!("Modbus 异常: {code}")))
            }
            PduResponse::WriteSingleAck { .. } | PduResponse::WriteMultipleAck { .. } => {
                if definition.is_read() {
                    let _ = self.read_definition(definition).await;
                }
                Ok(())
            }
            _ => Err(TransactionError::Protocol("写入响应类型不匹配".to_owned())),
        }
    }

    async fn create_definition(&mut self, input: DefinitionInput) -> Result<DefinitionDto, String> {
        let definition = input.into_dto(uuid::Uuid::new_v4().to_string())?;
        {
            let mut state = self.state.write().await;
            state.config.definitions.push(definition.clone());
        }
        self.last_run.remove(&definition.id);
        self.emit_status(None).await;
        Ok(definition)
    }

    async fn update_definition(
        &mut self,
        definition_id: &str,
        input: DefinitionInput,
    ) -> Result<DefinitionDto, String> {
        let definition = {
            let mut state = self.state.write().await;
            let definition = state
                .config
                .definitions
                .iter_mut()
                .find(|definition| definition.id == definition_id)
                .ok_or_else(|| "定义不存在".to_owned())?;
            input.apply_to(definition)?;
            definition.clone()
        };
        self.last_run.remove(definition_id);
        self.emit_status(None).await;
        Ok(definition)
    }

    async fn delete_definition(&mut self, definition_id: &str) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            let before = state.config.definitions.len();
            state
                .config
                .definitions
                .retain(|definition| definition.id != definition_id);
            if state.config.definitions.len() == before {
                return Err("定义不存在".to_owned());
            }
        }
        self.last_run.remove(definition_id);
        self.emit_status(None).await;
        Ok(())
    }

    async fn set_definition_enabled(
        &mut self,
        definition_id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        {
            let mut state = self.state.write().await;
            let definition = state
                .config
                .definitions
                .iter_mut()
                .find(|definition| definition.id == definition_id)
                .ok_or_else(|| "定义不存在".to_owned())?;
            definition.enabled = enabled;
        }
        if enabled {
            self.last_run.remove(definition_id);
        }
        self.emit_status(None).await;
        Ok(())
    }

    async fn definition(&self, definition_id: &str) -> Option<DefinitionDto> {
        let state = self.state.read().await;
        state
            .config
            .definitions
            .iter()
            .find(|definition| definition.id == definition_id)
            .cloned()
    }

    async fn transact(
        &mut self,
        request: &PduRequest,
        unit_id: u8,
        allow_retry: bool,
    ) -> Result<(PduResponse, f64), TransactionError> {
        if self.transport.is_none() {
            return Err(TransactionError::Transport("会话未连接".to_owned()));
        }

        let (framing, timeout_ms, retries, session_id) = {
            let state = self.state.read().await;
            (
                state.config.framing,
                state.config.timeout_ms,
                state.config.retries,
                state.config.id.clone(),
            )
        };
        let pdu = request
            .encode()
            .map_err(|error| TransactionError::Protocol(error.to_string()))?;
        let attempts = if allow_retry {
            1 + u64::from(retries)
        } else {
            1
        };
        let mut frames: Vec<FrameDto> = Vec::new();
        let mut tx = 0u64;
        let mut rx = 0u64;
        let mut errors = 0u64;
        let mut last_latency_ms = 0.0;
        let mut last_error = String::new();
        let mut last_was_transport = false;
        let request_summary = summarize_request(request, unit_id);

        for _ in 0..attempts {
            let transaction_id = self.next_transaction_id();
            let request_frame = frame::encode(framing, unit_id, Some(transaction_id), &pdu)
                .map_err(|error| TransactionError::Protocol(error.to_string()))?;
            frames.push(build_frame(
                self.next_seq(),
                &session_id,
                &request_frame,
                framing,
                FrameInfo {
                    direction: "tx",
                    summary: request_summary.clone(),
                    latency_ms: None,
                    status: "ok",
                },
            ));
            tx += 1;

            let started = Instant::now();
            let result = match self.transport.as_mut() {
                Some(transport) => {
                    transport
                        .transaction(&request_frame, framing, Duration::from_millis(timeout_ms))
                        .await
                }
                None => Err(TransportError::Closed),
            };
            last_latency_ms = started.elapsed().as_secs_f64() * 1000.0;

            match result {
                Ok(response_frame) => {
                    rx += 1;
                    let decoded = frame::decode(framing, &response_frame);
                    let pdu_response = decoded
                        .as_ref()
                        .ok()
                        .and_then(|adu| PduResponse::decode_for_request(request, &adu.pdu).ok());
                    let status = match &pdu_response {
                        Some(PduResponse::Exception { .. }) => "exception",
                        Some(_) => "ok",
                        None => "error",
                    };
                    frames.push(build_frame(
                        self.next_seq(),
                        &session_id,
                        &response_frame,
                        framing,
                        FrameInfo {
                            direction: "rx",
                            summary: summarize_response(request, pdu_response.as_ref()),
                            latency_ms: Some(round1(last_latency_ms)),
                            status,
                        },
                    ));

                    match decoded {
                        Err(error) => {
                            errors += 1;
                            last_error = error.to_string();
                            last_was_transport = false;
                        }
                        Ok(adu) => {
                            let tid_ok = framing != Framing::Mbap
                                || adu.transaction_id == Some(transaction_id);
                            if !tid_ok {
                                errors += 1;
                                last_error = "MBAP 事务 ID 不匹配".to_owned();
                                last_was_transport = false;
                            } else if adu.unit_id != unit_id {
                                errors += 1;
                                last_error = format!("从站 ID 不匹配: {}", adu.unit_id);
                                last_was_transport = false;
                            } else if let Some(response) = pdu_response {
                                self.finish_transaction(
                                    frames,
                                    tx,
                                    rx,
                                    errors,
                                    last_latency_ms,
                                    None,
                                )
                                .await;
                                return Ok((response, last_latency_ms));
                            } else {
                                errors += 1;
                                last_error = PduResponse::decode_for_request(request, &adu.pdu)
                                    .err()
                                    .map(|error| error.to_string())
                                    .unwrap_or_else(|| "响应解析失败".to_owned());
                                last_was_transport = false;
                            }
                        }
                    }
                }
                Err(error) => {
                    errors += 1;
                    last_was_transport = !matches!(error, TransportError::InvalidFrame(_));
                    let partial = error.partial();
                    let status = match &error {
                        TransportError::Timeout { .. } => "timeout",
                        _ => "error",
                    };
                    if !partial.is_empty() || status == "error" {
                        frames.push(build_frame(
                            self.next_seq(),
                            &session_id,
                            &partial,
                            framing,
                            FrameInfo {
                                direction: "rx",
                                summary: error.to_string(),
                                latency_ms: Some(round1(last_latency_ms)),
                                status,
                            },
                        ));
                    }
                    last_error = error.to_string();
                }
            }
        }

        let message = if last_error.is_empty() {
            "请求失败".to_owned()
        } else {
            last_error
        };
        self.finish_transaction(
            frames,
            tx,
            rx,
            errors,
            last_latency_ms,
            Some(message.clone()),
        )
        .await;
        if last_was_transport {
            Err(TransactionError::Transport(message))
        } else {
            Err(TransactionError::Protocol(message))
        }
    }

    async fn finish_transaction(
        &self,
        frames: Vec<FrameDto>,
        tx: u64,
        rx: u64,
        errors: u64,
        latency_ms: f64,
        error: Option<String>,
    ) {
        {
            let mut state = self.state.write().await;
            state.stats.tx += tx;
            state.stats.rx += rx;
            state.stats.errors += errors;
            if rx > 0 {
                state.stats.latency_ms = round1(latency_ms);
            }
        }
        let event = {
            let state = self.state.read().await;
            state.status_event(error)
        };
        let session_id = event.session_id.clone();
        if !frames.is_empty() {
            emit(&self.bus, AppEvent::SessionFrames { session_id, frames });
        }
        emit(&self.bus, AppEvent::SessionStatus(event));
    }

    async fn emit_status(&self, error: Option<String>) {
        let event = {
            let state = self.state.read().await;
            state.status_event(error)
        };
        emit(&self.bus, AppEvent::SessionStatus(event));
    }

    async fn emit_data(
        &self,
        definition: &DefinitionDto,
        values: Vec<RegisterValueDto>,
        latency_ms: f64,
        error: Option<String>,
    ) {
        let session_id = self.state.read().await.config.id.clone();
        emit(
            &self.bus,
            AppEvent::SessionData(DefinitionSnapshotDto {
                session_id,
                definition_id: definition.id.clone(),
                timestamp: now_ms(),
                values,
                latency_ms: round1(latency_ms),
                error,
            }),
        );
    }

    async fn wait_inter_frame(&self) {
        let delay = {
            let state = self.state.read().await;
            if state.config.inter_frame_delay_ms > 0 {
                Duration::from_millis(state.config.inter_frame_delay_ms)
            } else if state.config.framing == Framing::Rtu {
                match &state.config.link {
                    LinkConfig::Serial(serial) => {
                        Duration::from_micros(modbus_core::frame::t35_micros(serial.baud_rate))
                    }
                    _ => Duration::ZERO,
                }
            } else {
                Duration::ZERO
            }
        };
        if !delay.is_zero() {
            sleep(delay).await;
        }
    }

    fn next_transaction_id(&mut self) -> u16 {
        self.next_tid = self.next_tid.wrapping_add(1);
        if self.next_tid == 0 {
            self.next_tid = 1;
        }
        self.next_tid
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed)
    }
}

async fn wait_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending::<()>().await,
    }
}

fn build_values(
    definition: &DefinitionDto,
    response: &PduResponse,
) -> Result<Vec<RegisterValueDto>, String> {
    match response {
        PduResponse::ReadRegisters { values, .. } => {
            let rule = definition.rule();
            let span = rule.word_span();
            let mut result = Vec::with_capacity(values.len().div_ceil(span).max(1));
            for (index, chunk) in values.chunks(span).enumerate() {
                let address = definition.start_address.wrapping_add((index * span) as u16);
                match decode_words(&rule, chunk) {
                    Ok(value) => result.push(RegisterValueDto {
                        address,
                        words: value.words,
                        formatted: value.formatted,
                        numeric: value.numeric,
                        stale: false,
                        error: None,
                    }),
                    Err(error) => result.push(RegisterValueDto {
                        address,
                        words: chunk.to_vec(),
                        formatted: "—".to_owned(),
                        numeric: None,
                        stale: true,
                        error: Some(error.to_string()),
                    }),
                }
            }
            Ok(result)
        }
        PduResponse::ReadBits { values, .. } => Ok(values
            .iter()
            .enumerate()
            .map(|(index, value)| RegisterValueDto {
                address: definition.start_address.wrapping_add(index as u16),
                words: vec![if *value { 1 } else { 0 }],
                formatted: if *value { "ON" } else { "OFF" }.to_owned(),
                numeric: Some(if *value { 1.0 } else { 0.0 }),
                stale: false,
                error: None,
            })
            .collect()),
        PduResponse::Exception { .. } => Err("读取返回异常".to_owned()),
        _ => Err("响应类型与读取请求不匹配".to_owned()),
    }
}

fn summarize_request(request: &PduRequest, unit_id: u8) -> String {
    let function = request.function_code();
    if request.is_write() {
        format!(
            "FC{function:02} · 从站 {unit_id} · 写入 {} × {}",
            request.address(),
            request.quantity()
        )
    } else {
        format!(
            "FC{function:02} · 从站 {unit_id} · {} × {}",
            request.address(),
            request.quantity()
        )
    }
}

fn summarize_response(request: &PduRequest, response: Option<&PduResponse>) -> String {
    let function = request.function_code();
    match response {
        Some(PduResponse::Exception { code, .. }) => {
            format!("FC{function:02} · 异常 {code}")
        }
        Some(PduResponse::ReadRegisters { values, .. }) => {
            format!("FC{function:02} · {} 寄存器", values.len())
        }
        Some(PduResponse::ReadBits { values, .. }) => {
            format!("FC{function:02} · {} 位", values.len())
        }
        Some(PduResponse::WriteSingleAck { .. }) => {
            format!("FC{function:02} · 写入确认")
        }
        Some(PduResponse::WriteMultipleAck { quantity, .. }) => {
            format!("FC{function:02} · 写入确认 · {quantity} 寄存器")
        }
        None => format!("FC{function:02} · 解析失败"),
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
