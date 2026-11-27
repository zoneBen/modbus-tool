mod commands;
mod event;
mod frames;
mod model;
mod network;
mod scan;
mod session;
mod state;
mod tools;
mod transport;

use std::sync::Arc;

use tauri::Emitter;

use crate::event::{AppEvent, event_bus};
use crate::model::SessionFramesEventDto;
use crate::network::NetworkManager;
use crate::scan::ScanManager;
use crate::session::SessionManager;
use crate::state::AppState;

fn forward(app: &tauri::AppHandle, event: AppEvent) {
    match event {
        AppEvent::SessionStatus(payload) => {
            let _ = app.emit("session:status", payload);
        }
        AppEvent::SessionData(payload) => {
            let _ = app.emit("session:data", payload);
        }
        AppEvent::SessionFrames { session_id, frames } => {
            let _ = app.emit(
                "session:frames",
                SessionFramesEventDto { session_id, frames },
            );
        }
        AppEvent::DebugData(payload) => {
            let _ = app.emit("debug:data", payload);
        }
        AppEvent::DebugState(payload) => {
            let _ = app.emit("debug:state", payload);
        }
        AppEvent::Alert(payload) => {
            let _ = app.emit("alert:event", payload);
        }
        AppEvent::NetworkState(payload) => {
            let _ = app.emit("network:state", payload);
        }
        AppEvent::NetworkData(payload) => {
            let _ = app.emit("network:data", payload);
        }
        AppEvent::WorkspaceUpdated(payload) => {
            let _ = app.emit("workspace:updated", payload);
        }
        AppEvent::ScanProgress(payload) => {
            let _ = app.emit("scan:progress", payload);
        }
        AppEvent::ScanResult(payload) => {
            let _ = app.emit("scan:result", payload);
        }
        AppEvent::ScanDone(payload) => {
            let _ = app.emit("scan:done", payload);
        }
        AppEvent::Tool(payload) => {
            let _ = app.emit("tool:event", payload);
        }
        AppEvent::MqttState(payload) => {
            let _ = app.emit("mqtt:state", payload);
        }
        AppEvent::MqttMessage(payload) => {
            let _ = app.emit("mqtt:message", payload);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let bus = event_bus();
    let manager = Arc::new(SessionManager::new(bus.clone()));
    let scans = Arc::new(ScanManager::new(bus.clone()));
    let network = Arc::new(NetworkManager::new(bus.clone()));
    let tools = Arc::new(tools::ToolsManager::new(bus.clone()));
    let fileserver = Arc::new(tools::FileServerManager::new());
    let mqtt = Arc::new(tools::MqttManager::new(bus.clone()));
    let setup_bus = bus.clone();
    let state_manager = manager.clone();
    let state_scans = scans.clone();
    let state_network = network.clone();
    let state_tools = tools.clone();
    let state_fileserver = fileserver.clone();
    let state_mqtt = mqtt.clone();
    let shutdown_manager = manager.clone();
    let shutdown_network = network.clone();
    let shutdown_scans = scans.clone();
    let shutdown_tools = tools.clone();
    let shutdown_fileserver = fileserver.clone();
    let shutdown_mqtt = mqtt.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            manager: state_manager,
            scans: state_scans,
            network: state_network,
            tools: state_tools,
            fileserver: state_fileserver,
            mqtt: state_mqtt,
        })
        .on_window_event(move |_window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let manager = shutdown_manager.clone();
                let network = shutdown_network.clone();
                let scans = shutdown_scans.clone();
                let tools = shutdown_tools.clone();
                let fileserver = shutdown_fileserver.clone();
                let mqtt = shutdown_mqtt.clone();
                tauri::async_runtime::spawn(async move {
                    network.stop_all().await;
                    manager.shutdown_all().await;
                    scans.shutdown_all();
                    tools.shutdown();
                    mqtt.stop_all().await;
                    fileserver.shutdown().await;
                });
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let mut receiver = setup_bus.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(event) => forward(&handle, event),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!(skipped, "event bus lagged");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::save_text_file,
            commands::read_text_file,
            commands::workspace_get,
            commands::list_serial_ports,
            commands::session_create,
            commands::session_set_polling,
            commands::definition_read_once,
            commands::definition_set_enabled,
            commands::definition_create,
            commands::definition_update,
            commands::definition_delete,
            commands::write_register,
            commands::write_custom,
            commands::debug_start,
            commands::debug_stop,
            commands::debug_send,
            commands::session_connect,
            commands::session_disconnect,
            commands::session_update,
            commands::session_delete,
            commands::slave_write_value,
            commands::network_start,
            commands::network_stop,
            commands::network_send,
            commands::scan_slave_ids,
            commands::scan_addresses,
            commands::scan_baud_rates,
            commands::scan_network,
            commands::scan_cancel,
            tools::commands::tool_dns_query,
            tools::commands::tool_telnet_test,
            tools::commands::tool_http_request,
            tools::commands::tool_ping_start,
            tools::commands::tool_latency_start,
            tools::commands::tool_traceroute_start,
            tools::commands::tool_task_cancel,
            tools::commands::tool_arp_capabilities,
            tools::commands::tool_arp_scan,
            tools::commands::tool_hash_start,
            tools::commands::tool_search_start,
            tools::commands::tool_fileserver_start,
            tools::commands::tool_fileserver_stop,
            tools::commands::tool_fileserver_status,
            tools::commands::tool_mqtt_start,
            tools::commands::tool_mqtt_stop,
            tools::commands::tool_mqtt_publish,
            tools::commands::tool_mqtt_subscribe,
            tools::commands::tool_mqtt_unsubscribe,
            tools::commands::tool_public_ip,
            tools::commands::tool_ssh_execute,
            tools::commands::tool_lan_discovery,
            tools::commands::tool_speedtest_run,
            tools::commands::tool_ssl_cert,
            tools::commands::tool_snmp_query,
            tools::commands::tool_text_convert,
            tools::commands::tool_fs_list,
            tools::commands::tool_fs_rename,
            tools::commands::tool_port_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
