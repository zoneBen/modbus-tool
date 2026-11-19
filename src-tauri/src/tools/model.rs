use serde::{Deserialize, Serialize};

use crate::model::RawSendMode;

pub type ToolResult<T> = Result<T, ToolErrorDto>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolErrorDto {
    pub code: String,
    pub detail: String,
}

impl ToolErrorDto {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsQueryInput {
    pub name: String,
    #[serde(default = "default_dns_type")]
    pub record_type: String,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_dns_type() -> String {
    "A".to_owned()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsAnswerDto {
    pub name: String,
    pub record_type: String,
    pub ttl: u32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsResultDto {
    pub question: String,
    pub record_type: String,
    pub server: String,
    pub status: String,
    pub elapsed_ms: f64,
    pub answers: Vec<DnsAnswerDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelnetTestInput {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_read_timeout")]
    pub read_timeout_ms: u64,
    #[serde(default)]
    pub payload: Option<String>,
    #[serde(default = "default_payload_mode")]
    pub payload_mode: RawSendMode,
    #[serde(default)]
    pub append_cr: bool,
    #[serde(default)]
    pub append_lf: bool,
}

fn default_read_timeout() -> u64 {
    800
}

fn default_payload_mode() -> RawSendMode {
    RawSendMode::Ascii
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelnetResultDto {
    pub host: String,
    pub port: u16,
    pub connected: bool,
    pub latency_ms: f64,
    pub bytes_read: usize,
    pub text: String,
    pub hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpHeaderInput {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpFormFieldInput {
    pub name: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HttpHeaderInput>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default = "default_body_mode")]
    pub body_mode: String,
    #[serde(default)]
    pub form_fields: Vec<HttpFormFieldInput>,
    #[serde(default)]
    pub upload_path: Option<String>,
    #[serde(default)]
    pub upload_field: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default)]
    pub ca_path: Option<String>,
}

fn default_body_mode() -> String {
    "raw".to_owned()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpHeaderDto {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseDto {
    pub status: u16,
    pub status_text: String,
    pub final_url: String,
    pub elapsed_ms: f64,
    pub headers: Vec<HttpHeaderDto>,
    pub body: String,
    pub body_bytes: usize,
    pub truncated: bool,
    pub content_type: Option<String>,
    pub binary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingInput {
    pub targets: String,
    #[serde(default = "default_ping_count")]
    pub count: u8,
    #[serde(default = "default_ping_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_ping_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_ping_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_ping_payload")]
    pub payload_size: usize,
}

fn default_ping_count() -> u8 {
    3
}

fn default_ping_interval() -> u64 {
    200
}

fn default_ping_timeout() -> u64 {
    1000
}

fn default_ping_concurrency() -> usize {
    16
}

fn default_ping_payload() -> usize {
    32
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingHostResultDto {
    pub host: String,
    pub address: Option<String>,
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f64,
    pub min_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub status: String,
    pub error: Option<ToolErrorDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingDoneDto {
    pub task_id: String,
    pub completed: u32,
    pub failed: u32,
    pub error: Option<ToolErrorDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracerouteInput {
    pub target: String,
    #[serde(default = "default_max_hops")]
    pub max_hops: u8,
    #[serde(default = "default_probes")]
    pub probes: u8,
    #[serde(default = "default_ping_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub resolve_names: bool,
}

fn default_max_hops() -> u8 {
    30
}

fn default_probes() -> u8 {
    3
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TracerouteHopDto {
    pub hop: u8,
    pub address: Option<String>,
    pub hostname: Option<String>,
    pub rtts_ms: Vec<Option<f64>>,
    pub status: String,
    pub reached: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TracerouteDoneDto {
    pub task_id: String,
    pub reached: bool,
    pub error: Option<ToolErrorDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpAdapterDto {
    pub name: String,
    pub description: String,
    pub ip: String,
    pub cidr: String,
    pub mac: String,
    pub npcap_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpInterfaceDto {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpCapabilitiesDto {
    pub native: bool,
    pub npcap_available: bool,
    pub elevated: bool,
    pub reason: Option<String>,
    pub reason_code: Option<String>,
    pub adapters: Vec<ArpAdapterDto>,
    pub npcap_interfaces: Vec<ArpInterfaceDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpScanInput {
    pub cidr: String,
    #[serde(default = "default_arp_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_arp_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub enhanced: bool,
    #[serde(default)]
    pub adapter_name: Option<String>,
    #[serde(default)]
    pub interface_name: Option<String>,
}

fn default_arp_timeout() -> u64 {
    2500
}

fn default_arp_concurrency() -> usize {
    32
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpEntryDto {
    pub ip: String,
    pub mac: String,
    pub interface: String,
    pub source: String,
    pub replies: u32,
    pub conflict: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpScanResultDto {
    pub cidr: String,
    pub elapsed_ms: f64,
    pub enhanced: bool,
    pub entries: Vec<ArpEntryDto>,
    pub conflicts: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashInput {
    pub path: String,
    #[serde(default = "default_hash_algorithms")]
    pub algorithms: Vec<String>,
}

fn default_hash_algorithms() -> Vec<String> {
    vec!["sha256".to_owned()]
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashValueDto {
    pub algorithm: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashResultDto {
    pub path: String,
    pub size: u64,
    pub elapsed_ms: f64,
    pub values: Vec<HashValueDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashDoneDto {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<HashResultDto>,
    pub error: Option<ToolErrorDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInput {
    pub folder: String,
    pub query: String,
    #[serde(default = "default_search_mode")]
    pub mode: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub include: String,
    #[serde(default)]
    pub exclude: String,
    #[serde(default = "default_search_encoding")]
    pub encoding: String,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_true")]
    pub skip_hidden: bool,
}

fn default_search_mode() -> String {
    "literal".to_owned()
}

fn default_search_encoding() -> String {
    "auto".to_owned()
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024
}

fn default_max_results() -> usize {
    1000
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitDto {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub preview: String,
    pub matched: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDoneDto {
    pub task_id: String,
    pub files: u64,
    pub matches: u64,
    pub skipped: u64,
    pub truncated: bool,
    pub error: Option<ToolErrorDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuousPingInput {
    pub host: String,
    #[serde(default = "default_latency_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_ping_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_ping_payload")]
    pub payload_size: usize,
}

fn default_latency_interval() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencySampleDto {
    pub task_id: String,
    pub timestamp_ms: u64,
    pub address: String,
    pub status: String,
    pub rtt_ms: Option<f64>,
    pub seq: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyDoneDto {
    pub task_id: String,
    pub sent: u32,
    pub received: u32,
    pub lost: u32,
    pub min_ms: Option<f64>,
    pub avg_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub error: Option<ToolErrorDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileServerStartInput {
    pub root: String,
    #[serde(default = "default_fileserver_host")]
    pub host: String,
    #[serde(default = "default_fileserver_port")]
    pub port: u16,
}

fn default_fileserver_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_fileserver_port() -> u16 {
    80
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileServerFileDto {
    pub name: String,
    pub size: u64,
    pub modified_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileServerRequestDto {
    pub timestamp_ms: u64,
    pub remote: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileServerStatusDto {
    pub running: bool,
    pub root: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub urls: Vec<String>,
    pub started_at: Option<u64>,
    pub uploads: u64,
    pub downloads: u64,
    pub files: Vec<FileServerFileDto>,
    pub requests: Vec<FileServerRequestDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttBrokerInput {
    pub broker_id: String,
    pub host: String,
    #[serde(default = "default_mqtt_port")]
    pub port: u16,
    #[serde(default = "default_mqtt_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub ca_path: Option<String>,
    #[serde(default = "default_mqtt_keep_alive")]
    pub keep_alive: u16,
    #[serde(default = "default_true")]
    pub clean_session: bool,
}

fn default_mqtt_port() -> u16 {
    1883
}

fn default_mqtt_protocol() -> String {
    "3.1.1".to_owned()
}

fn default_mqtt_keep_alive() -> u16 {
    60
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttStateDto {
    pub broker_id: String,
    pub status: String,
    pub detail: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttMessageDto {
    pub broker_id: String,
    pub topic: String,
    pub payload_text: String,
    pub payload_hex: String,
    pub payload_bytes: usize,
    pub payload_truncated: bool,
    pub qos: u8,
    pub retain: bool,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttPublishInput {
    pub broker_id: String,
    pub topic: String,
    pub payload: String,
    #[serde(default)]
    pub payload_mode: String,
    #[serde(default)]
    pub qos: u8,
    #[serde(default)]
    pub retain: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttSubscribeInput {
    pub broker_id: String,
    pub topic: String,
    #[serde(default)]
    pub qos: u8,
}
