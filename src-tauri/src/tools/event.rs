use serde::Serialize;

use super::model::{
    HashDoneDto, PingDoneDto, PingHostResultDto, SearchDoneDto, SearchHitDto, TracerouteDoneDto,
    TracerouteHopDto,
};

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
    HashProgress {
        task_id: String,
        done: u64,
        total: u64,
    },
    HashDone(HashDoneDto),
    SearchProgress {
        task_id: String,
        files: u64,
        matches: u64,
    },
    SearchHits {
        task_id: String,
        hits: Vec<SearchHitDto>,
    },
    SearchDone(SearchDoneDto),
}
