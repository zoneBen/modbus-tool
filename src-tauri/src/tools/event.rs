use serde::Serialize;

use super::model::{PingDoneDto, PingHostResultDto, TracerouteDoneDto, TracerouteHopDto};

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "event",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ToolEventDto {
    PingProgress {
        task_id: String,
        done: u32,
        total: u32,
    },
    PingResult {
        task_id: String,
        result: PingHostResultDto,
    },
    PingDone(PingDoneDto),
    TracerouteHop {
        task_id: String,
        hop: TracerouteHopDto,
    },
    TracerouteDone(TracerouteDoneDto),
}
