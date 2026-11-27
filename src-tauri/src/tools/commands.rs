use tauri::State;

use crate::state::AppState;

use super::dns;
use super::fs_rename;
use super::http_client;
use super::model::{
    ArpCapabilitiesDto, ArpScanInput, ArpScanResultDto, ContinuousPingInput, DiscoveryInput,
    DiscoveryResultDto, DnsQueryInput, DnsResultDto, FileServerStartInput, FileServerStatusDto,
    FsListInput, FsListResultDto, HashInput, HttpRequestInput, HttpResponseDto, McpCallInput,
    MqttBrokerInput, MqttPublishInput, MqttSubscribeInput, PingInput, PortListInput,
    PortListResultDto, PublicIpDto, RenameInput, RenameResultDto, SearchInput, SnmpInput,
    SnmpResultDto, SpeedTestInput, SpeedTestResultDto, SshConnectInput, SshDownloadInput, SshInput,
    SshListResultDto, SshReadResultDto, SshResultDto, SshSessionStateDto, SshTransferInput,
    SslCertInput, SslCertResultDto, TelnetResultDto, TelnetTestInput, TextConvertInput,
    TextConvertResultDto, ToolErrorDto, ToolResult, TracerouteInput,
};
use super::telnet;
use super::{discovery, ports, public_ip, snmp, speedtest, ssh, ssh_mcp, ssl_cert, text_codec};

#[tauri::command]
pub async fn tool_dns_query(input: DnsQueryInput) -> ToolResult<DnsResultDto> {
    dns::query(input).await
}

#[tauri::command]
pub async fn tool_telnet_test(input: TelnetTestInput) -> ToolResult<TelnetResultDto> {
    telnet::test(input).await
}

#[tauri::command]
pub async fn tool_http_request(input: HttpRequestInput) -> ToolResult<HttpResponseDto> {
    http_client::request(input).await
}

#[tauri::command]
pub fn tool_ping_start(state: State<'_, AppState>, input: PingInput) -> ToolResult<String> {
    state.tools.start_ping(input)
}

#[tauri::command]
pub fn tool_traceroute_start(
    state: State<'_, AppState>,
    input: TracerouteInput,
) -> ToolResult<String> {
    state.tools.start_traceroute(input)
}

#[tauri::command]
pub fn tool_latency_start(
    state: State<'_, AppState>,
    input: ContinuousPingInput,
) -> ToolResult<String> {
    state.tools.start_latency(input)
}

#[tauri::command]
pub fn tool_task_cancel(state: State<'_, AppState>, task_id: String) -> bool {
    state.tools.cancel(&task_id)
}

#[tauri::command]
pub fn tool_arp_capabilities(state: State<'_, AppState>) -> ArpCapabilitiesDto {
    state.tools.arp_capabilities()
}

#[tauri::command]
pub async fn tool_arp_scan(
    state: State<'_, AppState>,
    input: ArpScanInput,
) -> ToolResult<ArpScanResultDto> {
    state.tools.arp_scan(input).await
}

#[tauri::command]
pub fn tool_hash_start(state: State<'_, AppState>, input: HashInput) -> ToolResult<String> {
    state.tools.start_hash(input)
}

#[tauri::command]
pub fn tool_search_start(state: State<'_, AppState>, input: SearchInput) -> ToolResult<String> {
    state.tools.start_search(input)
}

#[tauri::command]
pub async fn tool_fileserver_start(
    state: State<'_, AppState>,
    input: FileServerStartInput,
) -> ToolResult<FileServerStatusDto> {
    state.fileserver.start(input).await
}

#[tauri::command]
pub async fn tool_fileserver_stop(state: State<'_, AppState>) -> ToolResult<FileServerStatusDto> {
    state.fileserver.stop().await
}

#[tauri::command]
pub async fn tool_fileserver_status(
    state: State<'_, AppState>,
    path: Option<String>,
) -> ToolResult<FileServerStatusDto> {
    state.fileserver.snapshot(&path.unwrap_or_default()).await
}

#[tauri::command]
pub async fn tool_mqtt_start(state: State<'_, AppState>, input: MqttBrokerInput) -> ToolResult<()> {
    state.mqtt.start(input).await
}

#[tauri::command]
pub async fn tool_mqtt_stop(state: State<'_, AppState>, broker_id: String) -> ToolResult<bool> {
    Ok(state.mqtt.stop(&broker_id).await)
}

#[tauri::command]
pub async fn tool_mqtt_publish(
    state: State<'_, AppState>,
    input: MqttPublishInput,
) -> ToolResult<()> {
    state.mqtt.publish(input).await
}

#[tauri::command]
pub async fn tool_mqtt_subscribe(
    state: State<'_, AppState>,
    input: MqttSubscribeInput,
) -> ToolResult<()> {
    state.mqtt.subscribe(input).await
}

#[tauri::command]
pub async fn tool_mqtt_unsubscribe(
    state: State<'_, AppState>,
    broker_id: String,
    topic: String,
) -> ToolResult<()> {
    state.mqtt.unsubscribe(&broker_id, &topic).await
}

#[tauri::command]
pub async fn tool_public_ip() -> ToolResult<PublicIpDto> {
    public_ip::lookup().await
}

#[tauri::command]
pub async fn tool_ssh_execute(input: SshInput) -> ToolResult<SshResultDto> {
    ssh::execute(input).await
}

#[tauri::command]
pub async fn tool_lan_discovery(input: DiscoveryInput) -> ToolResult<DiscoveryResultDto> {
    discovery::scan(input).await
}

#[tauri::command]
pub async fn tool_speedtest_run(input: SpeedTestInput) -> ToolResult<SpeedTestResultDto> {
    speedtest::run(input).await
}

#[tauri::command]
pub async fn tool_ssl_cert(input: SslCertInput) -> ToolResult<SslCertResultDto> {
    ssl_cert::inspect(input).await
}

#[tauri::command]
pub async fn tool_snmp_query(input: SnmpInput) -> ToolResult<SnmpResultDto> {
    snmp::query(input).await
}

#[tauri::command]
pub fn tool_text_convert(input: TextConvertInput) -> ToolResult<TextConvertResultDto> {
    text_codec::convert(input)
}

#[tauri::command]
pub fn tool_fs_list(input: FsListInput) -> ToolResult<FsListResultDto> {
    fs_rename::list(input)
}

#[tauri::command]
pub fn tool_fs_rename(input: RenameInput) -> ToolResult<RenameResultDto> {
    fs_rename::rename(input)
}

#[tauri::command]
pub fn tool_port_list(input: PortListInput) -> ToolResult<PortListResultDto> {
    ports::list(input)
}

#[tauri::command]
pub async fn tool_ssh_connect(
    state: State<'_, AppState>,
    input: SshConnectInput,
) -> ToolResult<SshSessionStateDto> {
    state.ssh.connect(input).await
}

#[tauri::command]
pub fn tool_ssh_list(state: State<'_, AppState>) -> Vec<SshSessionStateDto> {
    state.ssh.list()
}

#[tauri::command]
pub async fn tool_ssh_write(
    state: State<'_, AppState>,
    session_id: String,
    data: String,
) -> ToolResult<()> {
    state.ssh.write_input(&session_id, &data).await
}

#[tauri::command]
pub async fn tool_ssh_resize(
    state: State<'_, AppState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> ToolResult<()> {
    state.ssh.resize(&session_id, cols, rows).await
}

#[tauri::command]
pub async fn tool_ssh_disconnect(
    state: State<'_, AppState>,
    session_id: String,
) -> ToolResult<bool> {
    Ok(state.ssh.disconnect(&session_id).await)
}

#[tauri::command]
pub async fn tool_ssh_sftp_list(
    state: State<'_, AppState>,
    session_id: String,
    path: Option<String>,
) -> ToolResult<SshListResultDto> {
    state.ssh.sftp_list(&session_id, path).await
}

#[tauri::command]
pub async fn tool_ssh_sftp_mkdir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> ToolResult<()> {
    state.ssh.sftp_mkdir(&session_id, &path).await
}

#[tauri::command]
pub async fn tool_ssh_sftp_remove(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    is_dir: bool,
) -> ToolResult<()> {
    state.ssh.sftp_remove(&session_id, &path, is_dir).await
}

#[tauri::command]
pub async fn tool_ssh_sftp_rename(
    state: State<'_, AppState>,
    session_id: String,
    from: String,
    to: String,
) -> ToolResult<()> {
    state.ssh.sftp_rename(&session_id, &from, &to).await
}

#[tauri::command]
pub async fn tool_ssh_upload(
    state: State<'_, AppState>,
    input: SshTransferInput,
) -> ToolResult<()> {
    state.ssh.upload(input).await
}

#[tauri::command]
pub async fn tool_ssh_download(
    state: State<'_, AppState>,
    input: SshDownloadInput,
) -> ToolResult<()> {
    state.ssh.download(input).await
}

#[tauri::command]
pub async fn tool_ssh_set_cwd_tracking(
    state: State<'_, AppState>,
    session_id: String,
    enabled: bool,
) -> ToolResult<()> {
    state.ssh.set_cwd_tracking(&session_id, enabled).await
}

#[tauri::command]
pub fn tool_ssh_read_output(
    state: State<'_, AppState>,
    session_id: String,
    cursor: Option<u64>,
) -> ToolResult<SshReadResultDto> {
    state.ssh.read_output(&session_id, cursor)
}

#[tauri::command]
pub fn tool_ssh_mcp_tools() -> Vec<serde_json::Value> {
    ssh_mcp::tools()
}

#[tauri::command]
pub async fn tool_ssh_mcp_call(
    state: State<'_, AppState>,
    request: McpCallInput,
) -> ToolResult<serde_json::Value> {
    match state.ssh.call_mcp(&request.name, request.arguments).await {
        Ok(value) => Ok(value),
        Err(error) => Ok(mcp_error(error)),
    }
}

fn mcp_error(error: ToolErrorDto) -> serde_json::Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": format!("{}: {}", error.code, error.detail)
        }],
        "isError": true
    })
}
