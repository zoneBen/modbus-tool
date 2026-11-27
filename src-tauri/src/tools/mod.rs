mod arp;
pub mod commands;
mod discovery;
mod dns;
pub mod event;
mod fileserver;
mod fs_rename;
mod hash;
mod http_client;
mod icmp;
mod latency;
pub mod model;
mod mqtt;
mod ping;
mod ports;
mod public_ip;
mod search;
mod snmp;
mod speedtest;
mod ssh;
mod ssl_cert;
mod tasks;
mod telnet;
mod text_codec;
mod traceroute;

use uuid::Uuid;

use crate::event::EventBus;

pub use self::fileserver::FileServerManager;
pub use self::mqtt::MqttManager;

use self::model::{
    ArpCapabilitiesDto, ArpScanInput, ArpScanResultDto, ContinuousPingInput, HashInput, PingInput,
    SearchInput, ToolResult, TracerouteInput,
};
use self::tasks::ToolTasks;

pub struct ToolsManager {
    bus: EventBus,
    tasks: ToolTasks,
}

impl ToolsManager {
    pub fn new(bus: EventBus) -> Self {
        Self {
            bus,
            tasks: ToolTasks::default(),
        }
    }

    pub fn start_ping(&self, input: PingInput) -> ToolResult<String> {
        ping::expand_targets(&input.targets)?;
        let task_id = Uuid::new_v4().to_string();
        let cancel = self.tasks.register(&task_id);
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = task_id.clone();
        tauri::async_runtime::spawn(async move {
            let run_id_for_task = run_id.clone();
            ping::run(run_id, input, cancel, tasks.clone(), bus).await;
            tasks.finish(&run_id_for_task);
        });
        Ok(task_id)
    }

    pub fn arp_capabilities(&self) -> ArpCapabilitiesDto {
        arp::capabilities()
    }

    pub async fn arp_scan(&self, input: ArpScanInput) -> ToolResult<ArpScanResultDto> {
        arp::scan(input).await
    }

    pub fn start_latency(&self, input: ContinuousPingInput) -> ToolResult<String> {
        if input.host.trim().is_empty() {
            return Err(model::ToolErrorDto::new(
                "invalidHost",
                "Target host is required",
            ));
        }
        let task_id = Uuid::new_v4().to_string();
        let cancel = self.tasks.register(&task_id);
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = task_id.clone();
        tauri::async_runtime::spawn(async move {
            let run_id_for_task = run_id.clone();
            latency::run(run_id, input, cancel, tasks.clone(), bus).await;
            tasks.finish(&run_id_for_task);
        });
        Ok(task_id)
    }

    pub fn start_hash(&self, input: HashInput) -> ToolResult<String> {
        if input.path.trim().is_empty() {
            return Err(model::ToolErrorDto::new(
                "invalidPath",
                "Select a file to hash",
            ));
        }
        let task_id = Uuid::new_v4().to_string();
        let cancel = self.tasks.register(&task_id);
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = task_id.clone();
        tauri::async_runtime::spawn(async move {
            let run_id_for_task = run_id.clone();
            hash::run(run_id, input, cancel, tasks.clone(), bus).await;
            tasks.finish(&run_id_for_task);
        });
        Ok(task_id)
    }

    pub fn start_search(&self, input: SearchInput) -> ToolResult<String> {
        if input.folder.trim().is_empty() {
            return Err(model::ToolErrorDto::new(
                "invalidFolder",
                "Select an existing folder",
            ));
        }
        let task_id = Uuid::new_v4().to_string();
        let cancel = self.tasks.register(&task_id);
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = task_id.clone();
        tauri::async_runtime::spawn(async move {
            let run_id_for_task = run_id.clone();
            search::run(run_id, input, cancel, tasks.clone(), bus).await;
            tasks.finish(&run_id_for_task);
        });
        Ok(task_id)
    }

    pub fn start_traceroute(&self, input: TracerouteInput) -> ToolResult<String> {
        if input.target.trim().is_empty() {
            return Err(model::ToolErrorDto::new(
                "invalidHost",
                "Target is required",
            ));
        }
        let task_id = Uuid::new_v4().to_string();
        let cancel = self.tasks.register(&task_id);
        let bus = self.bus.clone();
        let tasks = self.tasks.clone();
        let run_id = task_id.clone();
        tauri::async_runtime::spawn(async move {
            let run_id_for_task = run_id.clone();
            traceroute::run(run_id, input, cancel, tasks.clone(), bus).await;
            tasks.finish(&run_id_for_task);
        });
        Ok(task_id)
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        self.tasks.cancel(task_id)
    }

    pub fn shutdown(&self) {
        self.tasks.cancel_all();
    }
}
