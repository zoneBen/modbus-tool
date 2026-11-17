mod commands;
mod event;
mod frames;
mod model;
mod network;
mod scan;
mod session;
mod state;
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
    let setup_bus = bus.clone();
    let state_manager = manager.clone();
    let state_scans = scans.clone();
    let state_network = network.clone();
    let shutdown_manager = manager.clone();
    let shutdown_network = network.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            manager: state_manager,
            scans: state_scans,
            network: state_network,
        })
        .on_window_event(move |_window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let manager = shutdown_manager.clone();
                let network = shutdown_network.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = network.stop().await;
                    manager.shutdown_all().await;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
