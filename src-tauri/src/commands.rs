//! Tauri command boundary.

use serde::Serialize;
use tauri::State;

use crate::model::{
    AddressScanInput, BaudScanInput, DefinitionDto, DefinitionInput, NetworkScanInput,
    NewSessionInput, SerialPortInfoDto, SessionDto, SlaveIdScanInput, WorkspaceSnapshotDto,
};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub protocol: String,
    pub function_codes: Vec<u8>,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Modbus Tool".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol: modbus_core::PROTOCOL_SUMMARY.to_owned(),
        function_codes: modbus_core::SUPPORTED_FUNCTION_CODES.to_vec(),
    }
}

#[tauri::command]
pub async fn workspace_get(state: State<'_, AppState>) -> Result<WorkspaceSnapshotDto, String> {
    Ok(state.manager.workspace().await)
}

#[tauri::command]
pub fn list_serial_ports() -> Vec<SerialPortInfoDto> {
    match serialport::available_ports() {
        Ok(ports) => ports
            .into_iter()
            .map(|port| {
                let manufacturer = match &port.port_type {
                    serialport::SerialPortType::UsbPort(usb) => usb.manufacturer.clone(),
                    _ => None,
                };
                SerialPortInfoDto {
                    label: label_for_port(&port),
                    path: port.port_name,
                    manufacturer,
                }
            })
            .collect(),
        Err(error) => {
            tracing::warn!(%error, "failed to enumerate serial ports");
            Vec::new()
        }
    }
}

#[tauri::command]
pub async fn session_create(
    input: NewSessionInput,
    state: State<'_, AppState>,
) -> Result<SessionDto, String> {
    state.manager.create_session(input).await
}

#[tauri::command]
pub async fn session_set_polling(
    session_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.manager.set_polling(&session_id, enabled).await
}

#[tauri::command]
pub async fn definition_read_once(
    session_id: String,
    definition_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.manager.read_once(&session_id, &definition_id).await
}

#[tauri::command]
pub async fn definition_set_enabled(
    session_id: String,
    definition_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .manager
        .set_definition_enabled(&session_id, &definition_id, enabled)
        .await
}

#[tauri::command]
pub async fn definition_create(
    session_id: String,
    input: DefinitionInput,
    state: State<'_, AppState>,
) -> Result<DefinitionDto, String> {
    state.manager.create_definition(&session_id, input).await
}

#[tauri::command]
pub async fn definition_update(
    session_id: String,
    definition_id: String,
    input: DefinitionInput,
    state: State<'_, AppState>,
) -> Result<DefinitionDto, String> {
    state
        .manager
        .update_definition(&session_id, &definition_id, input)
        .await
}

#[tauri::command]
pub async fn definition_delete(
    session_id: String,
    definition_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .manager
        .delete_definition(&session_id, &definition_id)
        .await
}

#[tauri::command]
pub async fn write_register(
    session_id: String,
    definition_id: String,
    address: u16,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .manager
        .write_register(&session_id, &definition_id, address, &value)
        .await
}

#[tauri::command]
pub async fn session_delete(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.manager.delete_session(&session_id).await
}

#[tauri::command]
pub async fn session_disconnect(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.manager.disconnect(&session_id).await
}

#[tauri::command]
pub fn scan_slave_ids(input: SlaveIdScanInput, state: State<'_, AppState>) -> String {
    state.scans.start_slave_scan(input)
}

#[tauri::command]
pub fn scan_addresses(input: AddressScanInput, state: State<'_, AppState>) -> String {
    state.scans.start_address_scan(input)
}

#[tauri::command]
pub fn scan_baud_rates(input: BaudScanInput, state: State<'_, AppState>) -> String {
    state.scans.start_baud_scan(input)
}

#[tauri::command]
pub fn scan_network(input: NetworkScanInput, state: State<'_, AppState>) -> String {
    state.scans.start_network_scan(input)
}

#[tauri::command]
pub fn scan_cancel(scan_id: String, state: State<'_, AppState>) -> bool {
    state.scans.cancel(&scan_id)
}

fn label_for_port(port: &serialport::SerialPortInfo) -> String {
    match &port.port_type {
        serialport::SerialPortType::UsbPort(usb) => {
            let product = usb
                .product
                .clone()
                .unwrap_or_else(|| "USB Serial".to_owned());
            format!("{product} ({})", port.port_name)
        }
        serialport::SerialPortType::PciPort => {
            format!("PCI Serial ({})", port.port_name)
        }
        serialport::SerialPortType::BluetoothPort => {
            format!("Bluetooth Serial ({})", port.port_name)
        }
        serialport::SerialPortType::Unknown => port.port_name.clone(),
    }
}
