use tauri::State;

use crate::state::AppState;

use super::dns;
use super::http_client;
use super::model::{
    ArpCapabilitiesDto, ArpScanInput, ArpScanResultDto, DnsQueryInput, DnsResultDto, HashInput,
    HttpRequestInput, HttpResponseDto, PingInput, SearchInput, TelnetResultDto, TelnetTestInput,
    ToolResult, TracerouteInput,
};
use super::telnet;

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
