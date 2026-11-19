use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::StreamExt;
use ipnet::IpNet;
use tokio::net::lookup_host;
use tokio::time::{sleep, timeout};

use crate::event::EventBus;
use crate::tools::event::ToolEventDto;
use crate::tools::icmp::{self, IcmpStatus};
use crate::tools::model::{PingDoneDto, PingHostResultDto, PingInput, ToolErrorDto, ToolResult};
use crate::tools::tasks::ToolTasks;

const MAX_HOSTS: usize = 256;

pub fn expand_targets(input: &str) -> ToolResult<Vec<String>> {
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for token in input
        .split(|character: char| character.is_whitespace() || character == ',' || character == ';')
    {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if token.contains('/') {
            let network: IpNet = token
                .parse()
                .map_err(|_| ToolErrorDto::new("invalidCidr", token))?;
            let IpNet::V4(network) = network else {
                return Err(ToolErrorDto::new(
                    "ipv6Unsupported",
                    "Only IPv4 is supported",
                ));
            };
            for host in network.hosts() {
                let host = host.to_string();
                if seen.insert(host.clone()) {
                    targets.push(host);
                }
                if targets.len() > MAX_HOSTS {
                    return Err(ToolErrorDto::new(
                        "tooManyHosts",
                        format!("At most {MAX_HOSTS} hosts are allowed"),
                    ));
                }
            }
        } else if seen.insert(token.to_owned()) {
            targets.push(token.to_owned());
        }
        if targets.len() > MAX_HOSTS {
            return Err(ToolErrorDto::new(
                "tooManyHosts",
                format!("At most {MAX_HOSTS} hosts are allowed"),
            ));
        }
    }
    if targets.is_empty() {
        return Err(ToolErrorDto::new(
            "noTargets",
            "Enter at least one host or CIDR",
        ));
    }
    Ok(targets)
}

pub async fn run(
    task_id: String,
    input: PingInput,
    cancel: Arc<AtomicBool>,
    tasks: ToolTasks,
    bus: EventBus,
) {
    let targets = match expand_targets(&input.targets) {
        Ok(targets) => targets,
        Err(error) => {
            finish_error(&task_id, &tasks, &bus, error);
            return;
        }
    };
    let total = targets.len() as u32;
    let concurrency = input.concurrency.clamp(1, 32);
    let count = input.count.clamp(1, 10);
    let interval = Duration::from_millis(input.interval_ms.clamp(100, 5000));
    let timeout_ms = input.timeout_ms.clamp(100, 5000) as u32;
    let payload_size = input.payload_size.clamp(8, 1400);

    let mut stream = futures::stream::iter(targets.into_iter().map(|host| {
        let cancel = cancel.clone();
        async move { ping_host(host, count, interval, timeout_ms, payload_size, cancel).await }
    }))
    .buffer_unordered(concurrency);

    let mut completed = 0u32;
    let mut failed = 0u32;
    let mut cancelled = false;
    while let Some(result) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        completed += 1;
        if result.status != "ok" {
            failed += 1;
        }
        crate::event::emit(
            &bus,
            crate::event::AppEvent::Tool(ToolEventDto::PingResult {
                task_id: task_id.clone(),
                result,
            }),
        );
        crate::event::emit(
            &bus,
            crate::event::AppEvent::Tool(ToolEventDto::PingProgress {
                task_id: task_id.clone(),
                done: completed,
                total,
            }),
        );
    }
    drop(stream);

    let error = cancelled.then(|| ToolErrorDto::new("cancelled", "Ping cancelled"));
    tasks.finish(&task_id);
    crate::event::emit(
        &bus,
        crate::event::AppEvent::Tool(ToolEventDto::PingDone(PingDoneDto {
            task_id,
            completed,
            failed,
            error,
        })),
    );
}

async fn ping_host(
    host: String,
    count: u8,
    interval: Duration,
    timeout_ms: u32,
    payload_size: usize,
    cancel: Arc<AtomicBool>,
) -> PingHostResultDto {
    let address = match resolve_host(&host, timeout_ms).await {
        Ok(address) => address,
        Err(error) => {
            return PingHostResultDto {
                host,
                address: None,
                sent: 0,
                received: 0,
                loss_percent: 100.0,
                min_ms: None,
                avg_ms: None,
                max_ms: None,
                status: "error".to_owned(),
                error: Some(error),
            };
        }
    };

    let mut sent = 0u32;
    let mut rtts = Vec::new();
    let mut last_error = None;
    for index in 0..count {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        sent += 1;
        let reply = tokio::task::spawn_blocking(move || {
            icmp::ping_once(address, 64, timeout_ms, payload_size)
        })
        .await;
        match reply {
            Ok(Ok(reply)) if reply.status == IcmpStatus::Success => {
                if let Some(rtt) = reply.rtt_ms {
                    rtts.push(rtt);
                }
            }
            Ok(Ok(reply)) => {
                last_error = Some(ToolErrorDto::new(
                    if reply.status == IcmpStatus::Timeout {
                        "timeout"
                    } else {
                        "icmpError"
                    },
                    reply.status.as_str(),
                ));
            }
            Ok(Err(error)) => last_error = Some(error),
            Err(error) => {
                last_error = Some(ToolErrorDto::new("icmpError", error.to_string()));
            }
        }
        if index + 1 < count {
            sleep(interval).await;
        }
    }

    let received = rtts.len() as u32;
    let min = rtts.iter().copied().reduce(f64::min).map(round1);
    let max = rtts.iter().copied().reduce(f64::max).map(round1);
    let avg = (!rtts.is_empty()).then(|| round1(rtts.iter().sum::<f64>() / rtts.len() as f64));
    PingHostResultDto {
        host,
        address: Some(address.to_string()),
        sent,
        received,
        loss_percent: if sent == 0 {
            100.0
        } else {
            round1(((sent - received) as f64 / sent as f64) * 100.0)
        },
        min_ms: min,
        avg_ms: avg,
        max_ms: max,
        status: if received > 0 {
            "ok".to_owned()
        } else if last_error.is_some() {
            "timeout".to_owned()
        } else {
            "cancelled".to_owned()
        },
        error: if received > 0 { None } else { last_error },
    }
}

pub(crate) async fn resolve_host(host: &str, timeout_ms: u32) -> ToolResult<Ipv4Addr> {
    if let Ok(address) = host.parse::<Ipv4Addr>() {
        return Ok(address);
    }
    if host.parse::<IpAddr>().is_ok() {
        return Err(ToolErrorDto::new(
            "ipv6Unsupported",
            "Only IPv4 is supported",
        ));
    }
    let target = host.to_owned();
    let addresses = timeout(
        Duration::from_millis(u64::from(timeout_ms)),
        lookup_host((target.as_str(), 80)),
    )
    .await
    .map_err(|_| ToolErrorDto::new("resolveTimeout", host))?
    .map_err(|error| ToolErrorDto::new("resolveFailed", error.to_string()))?;
    addresses
        .filter_map(|address| match address.ip() {
            IpAddr::V4(address) => Some(address),
            IpAddr::V6(_) => None,
        })
        .next()
        .ok_or_else(|| ToolErrorDto::new("resolveFailed", "No IPv4 address found"))
}

fn finish_error(task_id: &str, tasks: &ToolTasks, bus: &EventBus, error: ToolErrorDto) {
    tasks.finish(task_id);
    crate::event::emit(
        bus,
        crate::event::AppEvent::Tool(ToolEventDto::PingDone(PingDoneDto {
            task_id: task_id.to_owned(),
            completed: 0,
            failed: 0,
            error: Some(error),
        })),
    );
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_hosts_and_cidrs() {
        assert_eq!(
            expand_targets("192.168.1.1, 192.168.1.2\n192.168.1.1").expect("targets"),
            vec!["192.168.1.1", "192.168.1.2"]
        );
        let targets = expand_targets("192.168.1.0/30").expect("cidr");
        assert_eq!(targets, vec!["192.168.1.1", "192.168.1.2"]);
    }

    #[test]
    fn rejects_empty_and_large_targets() {
        assert!(expand_targets(" ").is_err());
        assert!(expand_targets("10.0.0.0/16").is_err());
    }
}
