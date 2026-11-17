//! Application events emitted by session actors and forwarded to the webview.

use tokio::sync::broadcast;

use crate::model::{
    DebugDataDto, DebugStateDto, DefinitionSnapshotDto, FrameDto, ScanDoneDto, ScanProgressDto,
    ScanResultDto, SessionStatusEventDto, WorkspaceSnapshotDto,
};

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
    WorkspaceUpdated(WorkspaceSnapshotDto),
    ScanProgress(ScanProgressDto),
    ScanResult(ScanResultDto),
    ScanDone(ScanDoneDto),
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
