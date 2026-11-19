use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::event::{AppEvent, EventBus, emit};
use crate::tools::event::ToolEventDto;
use crate::tools::icmp::{self, IcmpStatus};
use crate::tools::model::{ContinuousPingInput, LatencyDoneDto, LatencySampleDto, ToolErrorDto};
use crate::tools::tasks::ToolTasks;

pub async fn run(
    task_id: String,
    input: ContinuousPingInput,
    cancel: Arc<AtomicBool>,
    tasks: ToolTasks,
    bus: EventBus,
) {
    let address = match super::ping::resolve_host(&input.host, 5000).await {
        Ok(address) => address,
        Err(error) => {
            finish(&task_id, &tasks, &bus, 0, 0, Vec::new(), Some(error));
            return;
        }
    };
    let interval = Duration::from_millis(input.interval_ms.clamp(200, 60_000));
    let timeout_ms = input.timeout_ms.clamp(100, 10_000) as u32;
    let payload_size = input.payload_size.clamp(8, 1400);
    let mut sent = 0u32;
    let mut received = 0u32;
    let mut rtts: Vec<f64> = Vec::new();

    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        sent += 1;
        let reply = tokio::task::spawn_blocking(move || {
            icmp::ping_once(address, 64, timeout_ms, payload_size)
        })
        .await;
        let (status, rtt_ms) = match reply {
            Ok(Ok(reply)) => {
                if reply.status == IcmpStatus::Success {
                    received += 1;
                    if let Some(rtt) = reply.rtt_ms {
                        rtts.push(rtt);
                    }
                }
                (reply.status.as_str().to_owned(), reply.rtt_ms)
            }
            Ok(Err(error)) => (error.code, None),
            Err(_error) => ("error".to_owned(), None),
        };
        emit(
            &bus,
            AppEvent::Tool(ToolEventDto::LatencySample {
                task_id: task_id.clone(),
                sample: LatencySampleDto {
                    task_id: task_id.clone(),
                    timestamp_ms: now_ms(),
                    address: address.to_string(),
                    status,
                    rtt_ms: rtt_ms.map(round1),
                    seq: sent,
                },
            }),
        );
        tokio::time::sleep(interval).await;
    }

    let error = cancel
        .load(Ordering::Relaxed)
        .then(|| ToolErrorDto::new("cancelled", "Continuous ping stopped"));
    finish(&task_id, &tasks, &bus, sent, received, rtts, error);
}

fn finish(
    task_id: &str,
    tasks: &ToolTasks,
    bus: &EventBus,
    sent: u32,
    received: u32,
    rtts: Vec<f64>,
    error: Option<ToolErrorDto>,
) {
    tasks.finish(task_id);
    let min = rtts.iter().copied().reduce(f64::min).map(round1);
    let max = rtts.iter().copied().reduce(f64::max).map(round1);
    let avg = (!rtts.is_empty()).then(|| round1(rtts.iter().sum::<f64>() / rtts.len() as f64));
    emit(
        bus,
        AppEvent::Tool(ToolEventDto::LatencyDone(LatencyDoneDto {
            task_id: task_id.to_owned(),
            sent,
            received,
            lost: sent.saturating_sub(received),
            min_ms: min,
            avg_ms: avg,
            max_ms: max,
            error,
        })),
    );
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
