use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::time::sleep;

use crate::event::{AppEvent, EventBus, emit};
use crate::tools::dns;
use crate::tools::event::ToolEventDto;
use crate::tools::icmp::{self, IcmpStatus};
use crate::tools::model::{ToolErrorDto, TracerouteDoneDto, TracerouteHopDto, TracerouteInput};
use crate::tools::ping;
use crate::tools::tasks::ToolTasks;

pub async fn run(
    task_id: String,
    input: TracerouteInput,
    cancel: Arc<AtomicBool>,
    tasks: ToolTasks,
    bus: EventBus,
) {
    let target = input.target.trim().to_owned();
    if target.is_empty() {
        finish_error(
            &task_id,
            &tasks,
            &bus,
            ToolErrorDto::new("invalidHost", "Target is required"),
        );
        return;
    }
    let timeout_ms = input.timeout_ms.clamp(100, 5000) as u32;
    let address = match ping::resolve_host(&target, timeout_ms).await {
        Ok(address) => address,
        Err(error) => {
            finish_error(&task_id, &tasks, &bus, error);
            return;
        }
    };
    let max_hops = input.max_hops.clamp(1, 64);
    let probes = input.probes.clamp(1, 3);
    let mut reached = false;
    let mut cancelled = false;

    for hop in 1..=max_hops {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let mut rtts = Vec::with_capacity(probes as usize);
        let mut hop_address = None;
        let mut any_reply = false;
        let mut last_status = IcmpStatus::Timeout;
        for _ in 0..probes {
            if cancel.load(Ordering::Relaxed) {
                cancelled = true;
                break;
            }
            let reply =
                tokio::task::spawn_blocking(move || icmp::ping_once(address, hop, timeout_ms, 32))
                    .await;
            match reply {
                Ok(Ok(reply)) => {
                    any_reply = true;
                    last_status = reply.status;
                    if hop_address.is_none() {
                        hop_address = reply.address;
                    }
                    if reply.status == IcmpStatus::Success && reply.address == Some(address) {
                        reached = true;
                    }
                    rtts.push(reply.rtt_ms.map(round1));
                }
                Ok(Err(_)) | Err(_) => rtts.push(None),
            }
            if reached || cancelled {
                break;
            }
        }
        if cancelled {
            break;
        }

        let hostname = if input.resolve_names {
            if let Some(address) = hop_address {
                dns::reverse_lookup_name(IpAddr::V4(address)).await
            } else {
                None
            }
        } else {
            None
        };
        let status = if reached {
            "reached"
        } else if any_reply {
            last_status.as_str()
        } else {
            "timeout"
        };
        emit(
            &bus,
            AppEvent::Tool(ToolEventDto::TracerouteHop {
                task_id: task_id.clone(),
                hop: TracerouteHopDto {
                    hop,
                    address: hop_address.map(|address| address.to_string()),
                    hostname,
                    rtts_ms: rtts,
                    status: status.to_owned(),
                    reached,
                },
            }),
        );
        if reached {
            break;
        }
        sleep(std::time::Duration::from_millis(40)).await;
    }

    tasks.finish(&task_id);
    let error = cancelled.then(|| ToolErrorDto::new("cancelled", "Traceroute cancelled"));
    emit(
        &bus,
        AppEvent::Tool(ToolEventDto::TracerouteDone(TracerouteDoneDto {
            task_id,
            reached,
            error,
        })),
    );
}

fn finish_error(task_id: &str, tasks: &ToolTasks, bus: &EventBus, error: ToolErrorDto) {
    tasks.finish(task_id);
    emit(
        bus,
        AppEvent::Tool(ToolEventDto::TracerouteDone(TracerouteDoneDto {
            task_id: task_id.to_owned(),
            reached: false,
            error: Some(error),
        })),
    );
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
