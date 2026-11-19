//! Application events emitted by session actors and forwarded to the webview.

use tokio::sync::broadcast;

use crate::model::{
    AlertEventDto, DebugDataDto, DebugStateDto, DefinitionSnapshotDto, FrameDto, NetworkDataDto,
    NetworkStateDto, ScanDoneDto, ScanProgressDto, ScanResultDto, SessionStatusEventDto,
    WorkspaceSnapshotDto,
};
use crate::tools::event::ToolEventDto;
use crate::tools::model::{MqttMessageDto, MqttStateDto};

#[derive(Debug, Clone)]
pub enum AppEvent {
    SessionStatus(SessionStatusEventDto),
    SessionData(DefinitionSnapshotDto),
    SessionFrames {
        session_id: String,
        frames: Vec<FrameDto>,
    },
    DebugData(DebugDataDto),
    DebugState(DebugStateDto),
    Alert(AlertEventDto),
    NetworkState(NetworkStateDto),
    NetworkData(NetworkDataDto),
    WorkspaceUpdated(WorkspaceSnapshotDto),
    ScanProgress(ScanProgressDto),
    ScanResult(ScanResultDto),
    ScanDone(ScanDoneDto),
    Tool(ToolEventDto),
    MqttState(MqttStateDto),
    MqttMessage(MqttMessageDto),
}

pub type EventBus = broadcast::Sender<AppEvent>;

pub fn event_bus() -> EventBus {
    let (sender, _) = broadcast::channel(2048);
    sender
}

pub fn emit(bus: &EventBus, event: AppEvent) {
    // No receivers simply means the UI is not listening; that is fine.
    let _ = bus.send(event);
}
