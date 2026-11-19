//! Serializable view models and internal session configuration.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use modbus_core::{BytePos, DataType, Framing, ModbusError, OrderPreset, PduRequest, ValueRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionRole {
    Master,
    Slave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportKind {
    Serial,
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionAccent {
    Signal,
    Sim,
    Rx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SerialParity {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkConfig {
    Serial(SerialLinkConfig),
    Tcp(NetworkLinkConfig),
    Udp(NetworkLinkConfig),
}

impl LinkConfig {
    pub fn transport_kind(&self) -> TransportKind {
        match self {
            Self::Serial(_) => TransportKind::Serial,
            Self::Tcp(_) => TransportKind::Tcp,
            Self::Udp(_) => TransportKind::Udp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialLinkConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: SerialParity,
    pub stop_bits: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkLinkConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertOperator {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Ne,
}

impl AlertOperator {
    pub fn compare(self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Gt => value > threshold,
            Self::Lt => value < threshold,
            Self::Gte => value >= threshold,
            Self::Lte => value <= threshold,
            Self::Eq => (value - threshold).abs() < f64::EPSILON,
            Self::Ne => (value - threshold).abs() >= f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

fn default_alert_severity() -> AlertSeverity {
    AlertSeverity::Warning
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRecoveryDto {
    pub operator: AlertOperator,
    pub value: f64,
    #[serde(default)]
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleDto {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_alert_severity")]
    pub severity: AlertSeverity,
    pub operator: AlertOperator,
    pub value: f64,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<AlertRecoveryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertEventDto {
    pub rule_id: String,
    pub session_id: String,
    pub definition_id: String,
    pub definition_name: String,
    pub address: u16,
    pub meaning: String,
    pub unit: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub operator: AlertOperator,
    pub threshold: f64,
    pub value: f64,
    pub delay_ms: u64,
    pub triggered: bool,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_operator: Option<AlertOperator>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_delay_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SimulatorMode {
    Increment,
    Decrement,
    Random,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulatorPointDto {
    pub mode: SimulatorMode,
    pub refresh_ms: u64,
    #[serde(default)]
    pub min: f64,
    #[serde(default)]
    pub max: f64,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPointDto {
    pub address: u16,
    pub data_type: DataType,
    pub byte_order: OrderPreset,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_pos: Option<BytePos>,
    #[serde(default)]
    pub meaning: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default = "default_scale")]
    pub scale: f64,
    #[serde(default)]
    pub offset: f64,
    #[serde(default)]
    pub alerts: Vec<AlertRuleDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulator: Option<SimulatorPointDto>,
}

impl RegisterPointDto {
    pub fn rule(&self) -> ValueRule {
        let (byte_order, word_order) = self.byte_order.parts();
        ValueRule {
            data_type: self.data_type,
            byte_pos: Some(self.byte_pos.unwrap_or(BytePos::Low)),
            byte_order,
            word_order,
        }
    }

    pub fn span(&self) -> usize {
        self.rule().word_span()
    }
}

pub fn scale_decoded(
    point: &RegisterPointDto,
    mut formatted: String,
    mut numeric: Option<f64>,
) -> (String, Option<f64>) {
    if let Some(raw) = numeric {
        let scaled = raw * point.scale + point.offset;
        if !matches!(
            point.data_type,
            DataType::Bool | DataType::Hex | DataType::Bin | DataType::Ascii
        ) {
            formatted = format_scaled(scaled);
        }
        numeric = Some(scaled);
    }
    (formatted, numeric)
}

fn format_scaled(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    if value.fract().abs() < 1e-9 && value.abs() < 9e15 {
        return format!("{value:.0}");
    }
    let mut text = format!("{value:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn default_scale() -> f64 {
    1.0
}

/// One read definition (row group) belonging to a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionDto {
    pub id: String,
    pub name: String,
    pub function_code: u8,
    pub slave_id: u8,
    pub start_address: u16,
    pub quantity: u16,
    pub data_type: DataType,
    pub byte_order: OrderPreset,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_pos: Option<BytePos>,
    pub interval_ms: u64,
    pub enabled: bool,
    #[serde(default)]
    pub aliases: BTreeMap<u16, String>,
    #[serde(default)]
    pub points: Vec<RegisterPointDto>,
}

impl DefinitionDto {
    pub fn rule(&self) -> ValueRule {
        let (byte_order, word_order) = self.byte_order.parts();
        ValueRule {
            data_type: self.data_type,
            byte_pos: Some(self.byte_pos.unwrap_or(BytePos::Low)),
            byte_order,
            word_order,
        }
    }

    pub fn effective_points(&self) -> Vec<RegisterPointDto> {
        if self.points.is_empty() {
            default_points(
                self.start_address,
                self.quantity,
                self.function_code,
                self.data_type,
                self.byte_order,
                self.byte_pos,
                &self.aliases,
            )
        } else {
            self.points.clone()
        }
    }

    pub fn is_read(&self) -> bool {
        (1..=4).contains(&self.function_code)
    }

    pub fn read_request(&self) -> Result<PduRequest, ModbusError> {
        let address = self.start_address;
        let quantity = self.quantity;
        match self.function_code {
            0x01 => Ok(PduRequest::ReadCoils { address, quantity }),
            0x02 => Ok(PduRequest::ReadDiscreteInputs { address, quantity }),
            0x03 => Ok(PduRequest::ReadHoldingRegisters { address, quantity }),
            0x04 => Ok(PduRequest::ReadInputRegisters { address, quantity }),
            other => Err(ModbusError::UnsupportedFunction(other)),
        }
    }
}

/// Editable definition fields (the id is generated by the backend).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionInput {
    pub name: String,
    pub function_code: u8,
    pub slave_id: u8,
    pub start_address: u16,
    pub quantity: u16,
    pub data_type: DataType,
    pub byte_order: OrderPreset,
    #[serde(default)]
    pub byte_pos: Option<BytePos>,
    pub interval_ms: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub aliases: BTreeMap<u16, String>,
    #[serde(default)]
    pub points: Vec<RegisterPointDto>,
}

impl DefinitionInput {
    pub fn into_dto(self, id: String) -> Result<DefinitionDto, String> {
        self.into_dto_validated(id, Self::validate)
    }

    fn into_dto_validated(
        self,
        id: String,
        validate: fn(&Self) -> Result<(), String>,
    ) -> Result<DefinitionDto, String> {
        validate(&self)?;
        let (points, aliases) = self.normalized_points()?;
        Ok(DefinitionDto {
            id,
            name: self.name.trim().to_owned(),
            function_code: self.function_code,
            slave_id: self.slave_id,
            start_address: self.start_address,
            quantity: self.quantity,
            data_type: self.effective_data_type(),
            byte_order: self.byte_order,
            byte_pos: self.byte_pos,
            interval_ms: self.interval_ms,
            enabled: self.enabled,
            aliases,
            points,
        })
    }

    pub fn apply_to(&self, definition: &mut DefinitionDto) -> Result<(), String> {
        self.apply_to_validated(definition, Self::validate)
    }

    /// Simulator mappings accept sparse point addresses and an empty mapping.
    pub fn apply_mapping_to(&self, definition: &mut DefinitionDto) -> Result<(), String> {
        self.apply_to_validated(definition, Self::validate_mapping)
    }

    fn apply_to_validated(
        &self,
        definition: &mut DefinitionDto,
        validate: fn(&Self) -> Result<(), String>,
    ) -> Result<(), String> {
        validate(self)?;
        let (points, aliases) = self.normalized_points()?;
        definition.name = self.name.trim().to_owned();
        definition.function_code = self.function_code;
        definition.slave_id = self.slave_id;
        definition.start_address = self.start_address;
        definition.quantity = self.quantity;
        definition.data_type = self.effective_data_type();
        definition.byte_order = self.byte_order;
        definition.byte_pos = self.byte_pos;
        definition.interval_ms = self.interval_ms;
        definition.enabled = self.enabled;
        definition.aliases = aliases;
        definition.points = points;
        Ok(())
    }

    fn normalized_points(&self) -> Result<(Vec<RegisterPointDto>, BTreeMap<u16, String>), String> {
        let is_bit = (1..=2).contains(&self.function_code);
        let mut points = if self.points.is_empty() {
            default_points(
                self.start_address,
                self.quantity,
                self.function_code,
                self.effective_data_type(),
                self.byte_order,
                self.byte_pos,
                &self.aliases,
            )
        } else {
            self.points.clone()
        };

        let end = u32::from(self.start_address) + u32::from(self.quantity);
        let mut seen = std::collections::BTreeSet::new();
        for point in &mut points {
            if !seen.insert(point.address) {
                return Err(format!("点位地址 {} 重复", point.address));
            }
            if point.address < self.start_address || u32::from(point.address) >= end {
                return Err(format!(
                    "点位地址 {} 超出读取范围 {}–{}",
                    point.address,
                    self.start_address,
                    end.saturating_sub(1)
                ));
            }
            if is_bit {
                point.data_type = DataType::Bool;
            }
            if u32::from(point.address) + point.span() as u32 > end {
                return Err(format!(
                    "点位 {} 占用 {} 个寄存器，超出读取范围",
                    point.address,
                    point.span()
                ));
            }
            if !point.scale.is_finite() || !point.offset.is_finite() {
                return Err(format!("点位 {} 的缩放/偏置必须为有限数值", point.address));
            }
            for rule in &mut point.alerts {
                if rule.id.trim().is_empty() {
                    rule.id = uuid::Uuid::new_v4().to_string();
                }
                if !rule.value.is_finite() {
                    return Err(format!("点位 {} 的告警阈值必须为有限数值", point.address));
                }
                if let Some(recovery) = &mut rule.recovery {
                    if !recovery.value.is_finite() {
                        return Err(format!(
                            "点位 {} 的告警恢复阈值必须为有限数值",
                            point.address
                        ));
                    }
                    recovery.delay_ms = recovery.delay_ms.min(3_600_000);
                }
                rule.delay_ms = rule.delay_ms.min(3_600_000);
            }
            if let Some(alias) = self.aliases.get(&point.address)
                && point.meaning.trim().is_empty()
            {
                point.meaning = alias.clone();
            }
            if let Some(simulator) = &mut point.simulator {
                if !simulator.min.is_finite() || !simulator.max.is_finite() {
                    return Err(format!("点位 {} 的模拟范围必须为有限数值", point.address));
                }
                if simulator.min > simulator.max {
                    return Err(format!("点位 {} 的模拟最小值不能大于最大值", point.address));
                }
                simulator.refresh_ms = simulator.refresh_ms.clamp(20, 3_600_000);
            }
        }

        let block_span = if is_bit {
            1
        } else {
            ValueRule::with_preset(self.effective_data_type(), self.byte_order).word_span()
        };
        for (address, alias) in &self.aliases {
            if points.iter().any(|point| point.address == *address) {
                continue;
            }
            if *address < self.start_address
                || u32::from(*address) >= end
                || u32::from(*address) + block_span as u32 > end
            {
                continue;
            }
            points.push(RegisterPointDto {
                address: *address,
                data_type: if is_bit {
                    DataType::Bool
                } else {
                    self.effective_data_type()
                },
                byte_order: self.byte_order,
                byte_pos: self.byte_pos,
                meaning: alias.clone(),
                unit: String::new(),
                scale: 1.0,
                offset: 0.0,
                alerts: Vec::new(),
                simulator: None,
            });
        }

        points.sort_by_key(|point| point.address);
        let aliases = points
            .iter()
            .filter(|point| !point.meaning.trim().is_empty())
            .map(|point| (point.address, point.meaning.trim().to_owned()))
            .collect();
        Ok((points, aliases))
    }

    fn effective_data_type(&self) -> DataType {
        if (1..=2).contains(&self.function_code) {
            DataType::Bool
        } else {
            self.data_type
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("定义名称不能为空".to_owned());
        }
        if !(1..=4).contains(&self.function_code) {
            return Err("读取定义仅支持 FC01–FC04".to_owned());
        }
        if !(1..=247).contains(&self.slave_id) {
            return Err("从站 ID 必须在 1–247 之间".to_owned());
        }
        if self.quantity == 0 {
            return Err("寄存器/位数量必须大于 0".to_owned());
        }
        let limit = if (1..=2).contains(&self.function_code) {
            2000
        } else {
            125
        };
        if self.quantity > limit {
            return Err(format!("数量超出限制，最多 {limit}"));
        }
        if u32::from(self.start_address) + u32::from(self.quantity) > 65_536 {
            return Err("地址范围超出 16 位地址空间".to_owned());
        }
        if !(10..=600_000).contains(&self.interval_ms) {
            return Err("轮询周期必须在 10 ms – 600 s 之间".to_owned());
        }
        Ok(())
    }

    fn validate_mapping(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("模拟映射名称不能为空".to_owned());
        }
        if !(1..=4).contains(&self.function_code) {
            return Err("模拟映射功能码仅支持 FC01–FC04".to_owned());
        }
        if !(1..=247).contains(&self.slave_id) {
            return Err("从站 ID 必须在 1–247 之间".to_owned());
        }
        if u32::from(self.start_address) + u32::from(self.quantity) > 65_536 {
            return Err("地址范围超出 16 位地址空间".to_owned());
        }
        if !(10..=600_000).contains(&self.interval_ms) {
            return Err("轮询周期必须在 10 ms – 600 s 之间".to_owned());
        }
        Ok(())
    }
}

fn default_points(
    start_address: u16,
    quantity: u16,
    function_code: u8,
    data_type: DataType,
    byte_order: OrderPreset,
    byte_pos: Option<BytePos>,
    aliases: &BTreeMap<u16, String>,
) -> Vec<RegisterPointDto> {
    let is_bit = (1..=2).contains(&function_code);
    let mut points = Vec::new();
    if is_bit {
        for offset in 0..quantity {
            let address = start_address.wrapping_add(offset);
            points.push(RegisterPointDto {
                address,
                data_type: DataType::Bool,
                byte_order: OrderPreset::Abcd,
                byte_pos: None,
                meaning: aliases.get(&address).cloned().unwrap_or_default(),
                unit: String::new(),
                scale: 1.0,
                offset: 0.0,
                alerts: Vec::new(),
                simulator: None,
            });
        }
        return points;
    }

    let (order, word_order) = byte_order.parts();
    let span = ValueRule {
        data_type,
        byte_pos: Some(byte_pos.unwrap_or(BytePos::Low)),
        byte_order: order,
        word_order,
    }
    .word_span()
    .max(1);
    let mut offset = 0u32;
    while offset + span as u32 <= u32::from(quantity) {
        let address = start_address.wrapping_add(offset as u16);
        points.push(RegisterPointDto {
            address,
            data_type,
            byte_order,
            byte_pos,
            meaning: aliases.get(&address).cloned().unwrap_or_default(),
            unit: String::new(),
            scale: 1.0,
            offset: 0.0,
            alerts: Vec::new(),
            simulator: None,
        });
        offset += span as u32;
    }
    points
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatsDto {
    pub tx: u64,
    pub rx: u64,
    pub errors: u64,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: String,
    pub name: String,
    pub role: SessionRole,
    pub transport: TransportKind,
    pub framing: Framing,
    pub endpoint: String,
    pub status: SessionStatus,
    pub polling: bool,
    pub accent: SessionAccent,
    pub definitions: Vec<DefinitionDto>,
    pub stats: SessionStatsDto,
    pub started_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baud_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_bits: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_bits: Option<u8>,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default)]
    pub retries: u32,
    #[serde(default)]
    pub inter_frame_delay_ms: u64,
    #[serde(default)]
    pub auto_reconnect: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterValueDto {
    pub address: u16,
    pub words: Vec<u16>,
    pub formatted: String,
    pub numeric: Option<f64>,
    #[serde(default)]
    pub stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionSnapshotDto {
    pub session_id: String,
    pub definition_id: String,
    pub timestamp: u64,
    pub values: Vec<RegisterValueDto>,
    pub latency_ms: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameFieldDto {
    pub label: String,
    pub value: String,
    pub offset: usize,
    pub length: usize,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedFrameDto {
    pub slave_id: u8,
    pub function_code: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ascii: Option<String>,
    pub fields: Vec<FrameFieldDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDto {
    pub seq: u64,
    pub session_id: String,
    pub timestamp: u64,
    pub direction: String,
    pub raw: Vec<u8>,
    pub decoded: DecodedFrameDto,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusEventDto {
    pub session_id: String,
    pub status: SessionStatus,
    pub polling: bool,
    pub stats: SessionStatsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshotDto {
    pub sessions: Vec<SessionDto>,
    pub version: u32,
    pub demo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFramesEventDto {
    pub session_id: String,
    pub frames: Vec<FrameDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortInfoDto {
    pub path: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
}

/// Runtime configuration owned by one session actor.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub id: String,
    pub name: String,
    pub role: SessionRole,
    pub link: LinkConfig,
    pub framing: Framing,
    pub endpoint: String,
    pub timeout_ms: u64,
    pub retries: u32,
    pub inter_frame_delay_ms: u64,
    pub auto_reconnect: bool,
    pub definitions: Vec<DefinitionDto>,
    pub polling: bool,
    pub accent: SessionAccent,
    pub started_at: u64,
}

/// Mutable runtime state shared between the manager and the actor.
#[derive(Debug)]
pub struct SessionState {
    pub config: SessionConfig,
    pub status: SessionStatus,
    pub polling: bool,
    pub stats: SessionStatsDto,
}

impl SessionState {
    pub fn new(config: SessionConfig) -> Self {
        let polling = config.polling;
        Self {
            config,
            status: SessionStatus::Disconnected,
            polling,
            stats: SessionStatsDto::default(),
        }
    }

    pub fn to_dto(&self) -> SessionDto {
        let (host, port, serial_port, baud_rate, data_bits, parity, stop_bits) =
            match &self.config.link {
                LinkConfig::Serial(serial) => (
                    None,
                    None,
                    Some(serial.port.clone()),
                    Some(serial.baud_rate),
                    Some(serial.data_bits),
                    Some(parity_name(serial.parity).to_owned()),
                    Some(serial.stop_bits),
                ),
                LinkConfig::Tcp(network) | LinkConfig::Udp(network) => (
                    Some(network.host.clone()),
                    Some(network.port),
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            };
        SessionDto {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            role: self.config.role,
            transport: self.config.link.transport_kind(),
            framing: self.config.framing,
            endpoint: self.config.endpoint.clone(),
            status: self.status,
            polling: self.polling,
            accent: self.config.accent,
            definitions: self.config.definitions.clone(),
            stats: self.stats.clone(),
            started_at: self.config.started_at,
            host,
            port,
            serial_port,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            timeout_ms: self.config.timeout_ms,
            retries: self.config.retries,
            inter_frame_delay_ms: self.config.inter_frame_delay_ms,
            auto_reconnect: self.config.auto_reconnect,
        }
    }

    pub fn status_event(&self, error: Option<String>) -> SessionStatusEventDto {
        SessionStatusEventDto {
            session_id: self.config.id.clone(),
            status: self.status,
            polling: self.polling,
            stats: self.stats.clone(),
            error,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionInput {
    pub name: String,
    pub role: SessionRole,
    pub transport: TransportKind,
    pub framing: Framing,
    pub endpoint: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub serial_port: Option<String>,
    #[serde(default)]
    pub baud_rate: Option<u32>,
    #[serde(default)]
    pub data_bits: Option<u8>,
    #[serde(default)]
    pub parity: Option<String>,
    #[serde(default)]
    pub stop_bits: Option<u8>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retries: Option<u32>,
    #[serde(default)]
    pub inter_frame_delay_ms: Option<u64>,
    #[serde(default)]
    pub auto_reconnect: Option<bool>,
    #[serde(default)]
    pub slave_id: Option<u8>,
    #[serde(default = "default_true")]
    pub auto_connect: bool,
}

impl NewSessionInput {
    pub fn into_config(self, id: String) -> Result<SessionConfig, String> {
        if self.role == SessionRole::Slave && self.transport == TransportKind::Udp {
            return Err("从站模拟暂不支持 UDP，请使用 TCP 或串口".to_owned());
        }

        let link = build_link(
            self.transport,
            self.framing,
            self.host,
            self.port,
            self.serial_port,
            self.baud_rate,
            self.data_bits,
            self.parity.as_deref(),
            self.stop_bits,
            &self.endpoint,
        )?;

        let endpoint = if self.endpoint.trim().is_empty() {
            endpoint_from_link(&link)
        } else {
            self.endpoint.clone()
        };

        let accent = match self.transport {
            TransportKind::Serial => SessionAccent::Rx,
            TransportKind::Tcp | TransportKind::Udp => SessionAccent::Signal,
        };

        Ok(SessionConfig {
            id,
            name: if self.name.trim().is_empty() {
                "新会话".to_owned()
            } else {
                self.name.trim().to_owned()
            },
            role: self.role,
            link,
            framing: self.framing,
            endpoint,
            timeout_ms: self.timeout_ms.unwrap_or(1000).clamp(50, 60_000),
            retries: self.retries.unwrap_or(1).min(10),
            inter_frame_delay_ms: self.inter_frame_delay_ms.unwrap_or(0).min(60_000),
            auto_reconnect: self.auto_reconnect.unwrap_or(true),
            definitions: if self.role == SessionRole::Slave {
                vec![simulator_definition(self.slave_id.unwrap_or(1))]
            } else {
                Vec::new()
            },
            polling: self.role != SessionRole::Slave,
            accent,
            started_at: now_ms(),
        })
    }
}

fn endpoint_from_link(link: &LinkConfig) -> String {
    match link {
        LinkConfig::Serial(serial) => format!(
            "{} · {} {}{}{}",
            serial.port,
            serial.baud_rate,
            serial.data_bits,
            parity_letter(serial.parity),
            serial.stop_bits
        ),
        LinkConfig::Tcp(network) | LinkConfig::Udp(network) => {
            format!("{}:{}", network.host, network.port)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionInput {
    pub name: String,
    pub transport: TransportKind,
    pub framing: Framing,
    pub endpoint: String,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub serial_port: Option<String>,
    #[serde(default)]
    pub baud_rate: Option<u32>,
    #[serde(default)]
    pub data_bits: Option<u8>,
    #[serde(default)]
    pub parity: Option<String>,
    #[serde(default)]
    pub stop_bits: Option<u8>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub retries: Option<u32>,
    #[serde(default)]
    pub inter_frame_delay_ms: Option<u64>,
    #[serde(default)]
    pub auto_reconnect: Option<bool>,
}

impl UpdateSessionInput {
    pub fn apply_to(self, config: &mut SessionConfig) -> Result<(), String> {
        if config.role == SessionRole::Slave && self.transport == TransportKind::Udp {
            return Err("从站模拟暂不支持 UDP，请使用 TCP 或串口".to_owned());
        }
        let link = build_link(
            self.transport,
            self.framing,
            self.host,
            self.port,
            self.serial_port,
            self.baud_rate,
            self.data_bits,
            self.parity.as_deref(),
            self.stop_bits,
            &self.endpoint,
        )?;
        let endpoint = if self.endpoint.trim().is_empty() {
            endpoint_from_link(&link)
        } else {
            self.endpoint.clone()
        };
        let name = self.name.trim();
        if !name.is_empty() {
            config.name = name.to_owned();
        }
        config.link = link;
        config.framing = self.framing;
        config.endpoint = endpoint;
        config.timeout_ms = self
            .timeout_ms
            .unwrap_or(config.timeout_ms)
            .clamp(50, 60_000);
        config.retries = self.retries.unwrap_or(config.retries).min(10);
        config.inter_frame_delay_ms = self
            .inter_frame_delay_ms
            .unwrap_or(config.inter_frame_delay_ms)
            .min(60_000);
        config.auto_reconnect = self.auto_reconnect.unwrap_or(config.auto_reconnect);
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaveValueInput {
    pub address: u16,
    pub data_type: DataType,
    pub byte_order: OrderPreset,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomWriteInput {
    pub slave_id: u8,
    pub function_code: u8,
    pub address: u16,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RawSendMode {
    Ascii,
    Hex,
}

const MAX_RAW_SEND_BYTES: usize = 4096;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSendInput {
    pub data: String,
    pub mode: RawSendMode,
    #[serde(default)]
    pub append_cr: bool,
    #[serde(default)]
    pub append_lf: bool,
}

impl RawSendInput {
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let mut bytes = match self.mode {
            RawSendMode::Ascii => self.data.as_bytes().to_vec(),
            RawSendMode::Hex => {
                let cleaned = self
                    .data
                    .replace("0x", "")
                    .replace("0X", "")
                    .chars()
                    .filter(|character| {
                        !character.is_whitespace() && !matches!(character, ',' | '-' | ':' | '_')
                    })
                    .collect::<String>();
                if cleaned.is_empty() {
                    Vec::new()
                } else if !cleaned
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
                {
                    return Err("HEX 包含非法字符，仅支持 0–9 / A–F".to_owned());
                } else if !cleaned.len().is_multiple_of(2) {
                    return Err("HEX 内容需为偶数个十六进制字符".to_owned());
                } else {
                    cleaned
                        .as_bytes()
                        .chunks(2)
                        .map(|pair| {
                            let text =
                                std::str::from_utf8(pair).map_err(|_| "HEX 编码失败".to_owned())?;
                            u8::from_str_radix(text, 16).map_err(|_| "HEX 解析失败".to_owned())
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
            }
        };
        if self.append_cr {
            bytes.push(0x0d);
        }
        if self.append_lf {
            bytes.push(0x0a);
        }
        if bytes.is_empty() {
            return Err("发送内容为空".to_owned());
        }
        if bytes.len() > MAX_RAW_SEND_BYTES {
            return Err(format!(
                "单次发送不能超过 {MAX_RAW_SEND_BYTES} 字节，当前 {} 字节",
                bytes.len()
            ));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugDataDto {
    pub session_id: String,
    pub direction: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStateDto {
    pub session_id: String,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkMode {
    TcpServer,
    TcpClient,
    UdpServer,
    UdpClient,
    TcpRelay,
}

impl NetworkMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::TcpServer => "tcp-server",
            Self::TcpClient => "tcp-client",
            Self::UdpServer => "udp-server",
            Self::UdpClient => "udp-client",
            Self::TcpRelay => "tcp-relay",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStartInput {
    pub mode: NetworkMode,
    #[serde(default)]
    pub local_host: String,
    #[serde(default)]
    pub local_port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStateDto {
    pub session_id: String,
    pub active: bool,
    pub mode: Option<String>,
    pub endpoint: Option<String>,
    pub clients: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDataDto {
    pub session_id: String,
    pub direction: String,
    pub data: Vec<u8>,
    pub connection: String,
    pub timestamp: u64,
}

#[allow(clippy::too_many_arguments)]
fn build_link(
    transport: TransportKind,
    framing: Framing,
    host: Option<String>,
    port: Option<u16>,
    serial_port: Option<String>,
    baud_rate: Option<u32>,
    data_bits: Option<u8>,
    parity: Option<&str>,
    stop_bits: Option<u8>,
    endpoint: &str,
) -> Result<LinkConfig, String> {
    if framing == Framing::Mbap && transport == TransportKind::Serial {
        return Err("串口链路不支持 MBAP 帧格式".to_owned());
    }
    match transport {
        TransportKind::Serial => {
            let port = serial_port
                .or_else(|| endpoint.split('·').next().map(str::trim).map(str::to_owned))
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "缺少串口端口".to_owned())?;
            let parity = match parity.unwrap_or("none") {
                "even" => SerialParity::Even,
                "odd" => SerialParity::Odd,
                "none" | "" => SerialParity::None,
                other => return Err(format!("未知校验方式: {other}")),
            };
            Ok(LinkConfig::Serial(SerialLinkConfig {
                port,
                baud_rate: baud_rate.unwrap_or(9600).max(1),
                data_bits: data_bits.unwrap_or(8).clamp(5, 8),
                parity,
                stop_bits: stop_bits.unwrap_or(1).clamp(1, 2),
            }))
        }
        TransportKind::Tcp | TransportKind::Udp => {
            let (endpoint_host, endpoint_port) = split_endpoint(endpoint);
            let host = host
                .or(endpoint_host)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "缺少主机地址".to_owned())?;
            let port = port.or(endpoint_port).unwrap_or(502);
            let network = NetworkLinkConfig { host, port };
            Ok(match transport {
                TransportKind::Tcp => LinkConfig::Tcp(network),
                TransportKind::Udp => LinkConfig::Udp(network),
                TransportKind::Serial => unreachable!("serial handled above"),
            })
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanLinkInput {
    pub transport: TransportKind,
    pub framing: Framing,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub serial_port: Option<String>,
    #[serde(default)]
    pub baud_rate: Option<u32>,
    #[serde(default)]
    pub data_bits: Option<u8>,
    #[serde(default)]
    pub parity: Option<String>,
    #[serde(default)]
    pub stop_bits: Option<u8>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl ScanLinkInput {
    pub fn link(&self) -> Result<LinkConfig, String> {
        build_link(
            self.transport,
            self.framing,
            self.host.clone(),
            self.port,
            self.serial_port.clone(),
            self.baud_rate,
            self.data_bits,
            self.parity.as_deref(),
            self.stop_bits,
            "",
        )
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms.unwrap_or(200).clamp(50, 5000))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanKind {
    SlaveId,
    Address,
    Baud,
    Network,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaveIdScanInput {
    pub link: ScanLinkInput,
    #[serde(default = "default_read_function")]
    pub function_code: u8,
    #[serde(default)]
    pub address: u16,
    #[serde(default = "default_one")]
    pub quantity: u16,
    pub id_start: u8,
    pub id_end: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressScanInput {
    pub link: ScanLinkInput,
    #[serde(default = "default_read_function")]
    pub function_code: u8,
    #[serde(default = "default_one_u8")]
    pub slave_id: u8,
    pub start_address: u16,
    pub end_address: u16,
    #[serde(default = "default_block_size")]
    pub block_size: u16,
    #[serde(default)]
    pub non_zero_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaudScanInput {
    pub serial_port: String,
    pub framing: Framing,
    #[serde(default)]
    pub data_bits: Option<u8>,
    #[serde(default)]
    pub parity: Option<String>,
    #[serde(default)]
    pub stop_bits: Option<u8>,
    pub baud_rates: Vec<u32>,
    #[serde(default = "default_one_u8")]
    pub slave_id: u8,
    #[serde(default = "default_read_function")]
    pub function_code: u8,
    #[serde(default)]
    pub address: u16,
    #[serde(default = "default_one")]
    pub quantity: u16,
    #[serde(default = "default_scan_timeout")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkScanInput {
    pub cidr: String,
    /// Legacy single-port field; still accepted for old clients and configs.
    #[serde(default = "default_modbus_port")]
    pub port: u16,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default = "default_network_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_true")]
    pub probe_modbus: bool,
    #[serde(default)]
    pub grab_banner: bool,
    #[serde(default = "default_one_u8")]
    pub slave_id: u8,
}

impl NetworkScanInput {
    pub fn effective_ports(&self) -> Result<Vec<u16>, String> {
        let mut ports = if self.ports.is_empty() {
            vec![self.port]
        } else {
            self.ports.clone()
        };
        ports.retain(|port| *port > 0);
        ports.sort_unstable();
        ports.dedup();
        if ports.is_empty() {
            return Err("至少选择一个端口".to_owned());
        }
        if ports.len() > 256 {
            return Err("单次扫描最多 256 个端口".to_owned());
        }
        Ok(ports)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressDto {
    pub scan_id: String,
    pub kind: ScanKind,
    pub done: u64,
    pub total: u64,
    pub found: u64,
    pub phase: String,
    pub finished: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultDto {
    pub scan_id: String,
    pub kind: ScanKind,
    pub label: String,
    pub status: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slave_id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baud_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanDoneDto {
    pub scan_id: String,
    pub kind: ScanKind,
    pub found: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn default_read_function() -> u8 {
    0x03
}

fn default_one() -> u16 {
    1
}

fn default_one_u8() -> u8 {
    1
}

fn default_block_size() -> u16 {
    10
}

fn default_scan_timeout() -> u64 {
    200
}

fn default_modbus_port() -> u16 {
    502
}

fn default_network_timeout() -> u64 {
    400
}

fn default_concurrency() -> usize {
    64
}

fn simulator_definition(unit_id: u8) -> DefinitionDto {
    DefinitionDto {
        id: uuid::Uuid::new_v4().to_string(),
        name: "从站模拟映射".to_owned(),
        function_code: 0x03,
        slave_id: unit_id,
        start_address: 0,
        quantity: 0,
        data_type: DataType::U16,
        byte_order: OrderPreset::Abcd,
        byte_pos: None,
        interval_ms: 500,
        enabled: true,
        aliases: BTreeMap::new(),
        points: Vec::new(),
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn split_endpoint(endpoint: &str) -> (Option<String>, Option<u16>) {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    match trimmed.rsplit_once(':') {
        Some((host, port)) => (
            (!host.is_empty()).then(|| host.to_owned()),
            port.parse::<u16>().ok(),
        ),
        None => (Some(trimmed.to_owned()), None),
    }
}

fn parity_name(parity: SerialParity) -> &'static str {
    match parity {
        SerialParity::None => "none",
        SerialParity::Even => "even",
        SerialParity::Odd => "odd",
    }
}

fn parity_letter(parity: SerialParity) -> &'static str {
    match parity {
        SerialParity::None => "N",
        SerialParity::Even => "E",
        SerialParity::Odd => "O",
    }
}
