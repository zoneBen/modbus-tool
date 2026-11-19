mod native;
mod npcap;

use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::Instant;

use ipnet::IpNet;

use super::model::{
    ArpCapabilitiesDto, ArpEntryDto, ArpScanInput, ArpScanResultDto, ToolErrorDto, ToolResult,
};

const MAX_ARP_HOSTS: usize = 1024;

pub fn capabilities() -> ArpCapabilitiesDto {
    let mut adapters = native::adapters();
    let (npcap_available, elevated, reason, reason_code, interfaces) = npcap::capabilities();
    for adapter in &mut adapters {
        let guid = adapter
            .name
            .trim_matches(|character| character == '{' || character == '}');
        adapter.npcap_name = interfaces
            .iter()
            .find(|interface| !guid.is_empty() && interface.name.contains(guid))
            .map(|interface| interface.name.clone());
    }
    ArpCapabilitiesDto {
        native: cfg!(windows),
        npcap_available,
        elevated,
        reason,
        reason_code,
        adapters,
        npcap_interfaces: interfaces,
    }
}

pub async fn scan(input: ArpScanInput) -> ToolResult<ArpScanResultDto> {
    let started = Instant::now();
    let network: IpNet = input
        .cidr
        .trim()
        .parse()
        .map_err(|_| ToolErrorDto::new("invalidCidr", "Invalid IPv4 CIDR"))?;
    let IpNet::V4(network) = network else {
        return Err(ToolErrorDto::new(
            "ipv6Unsupported",
            "Only IPv4 is supported",
        ));
    };
    let hosts: Vec<Ipv4Addr> = network.hosts().collect();
    if hosts.is_empty() {
        return Err(ToolErrorDto::new("noTargets", "CIDR contains no hosts"));
    }
    if hosts.len() > MAX_ARP_HOSTS {
        return Err(ToolErrorDto::new(
            "tooManyHosts",
            format!("At most {MAX_ARP_HOSTS} hosts are allowed for ARP scanning"),
        ));
    }
    let interface_label = input
        .adapter_name
        .as_deref()
        .and_then(|name| {
            native::adapters()
                .into_iter()
                .find(|adapter| adapter.name == name)
                .map(|adapter| adapter.description)
        })
        .unwrap_or_else(|| "local".to_owned());

    let mut entries: BTreeMap<Ipv4Addr, Vec<ArpEntryDto>> = BTreeMap::new();
    #[cfg(windows)]
    {
        let native = native::scan_native(&hosts, input.concurrency).await;
        for (ip, mac) in native {
            entries.entry(ip).or_default().push(ArpEntryDto {
                ip: ip.to_string(),
                mac,
                interface: interface_label.clone(),
                source: "native".to_owned(),
                replies: 1,
                conflict: false,
            });
        }
    }

    if input.enhanced {
        #[cfg(not(windows))]
        {
            return Err(ToolErrorDto::new(
                "unsupportedPlatform",
                "Enhanced ARP is only supported on Windows",
            ));
        }
        #[cfg(windows)]
        {
            let adapter = input
                .adapter_name
                .as_deref()
                .and_then(|name| {
                    capabilities()
                        .adapters
                        .into_iter()
                        .find(|adapter| adapter.name == name)
                })
                .ok_or_else(|| {
                    ToolErrorDto::new("invalidAdapter", "Select a local adapter first")
                })?;
            if adapter.mac == "—" {
                return Err(ToolErrorDto::new(
                    "invalidAdapter",
                    "The selected adapter has no MAC address",
                ));
            }
            let local_ip: Ipv4Addr = adapter
                .ip
                .parse()
                .map_err(|_| ToolErrorDto::new("invalidAdapter", "Adapter has no IPv4 address"))?;
            let local_mac = parse_mac(&adapter.mac)
                .ok_or_else(|| ToolErrorDto::new("invalidAdapter", "Invalid adapter MAC"))?;
            let interface_name = input
                .interface_name
                .clone()
                .or_else(|| adapter.npcap_name.clone())
                .ok_or_else(|| {
                    ToolErrorDto::new(
                        "npcapInterface",
                        "No matching Npcap interface; select one manually",
                    )
                })?;
            if !npcap::is_elevated() {
                return Err(ToolErrorDto::new(
                    "npcapPermission",
                    "Enhanced ARP requires administrator privileges",
                ));
            }
            let timeout_ms = input.timeout_ms.clamp(500, 10_000);
            let hosts_for_capture = hosts.clone();
            let replies = tokio::task::spawn_blocking(move || {
                let api = npcap::NpcapApi::load()?;
                api.scan(
                    &interface_name,
                    local_ip,
                    &local_mac,
                    &hosts_for_capture,
                    timeout_ms,
                )
            })
            .await
            .map_err(|error| ToolErrorDto::new("npcapError", error.to_string()))??;
            for (ip, macs) in replies {
                let conflict = macs.len() > 1;
                for (mac, count) in macs {
                    entries.entry(ip).or_default().push(ArpEntryDto {
                        ip: ip.to_string(),
                        mac,
                        interface: interface_label.clone(),
                        source: "npcap".to_owned(),
                        replies: count,
                        conflict,
                    });
                }
            }
        }
    }

    let mut entries: Vec<ArpEntryDto> = entries.into_values().flatten().collect();
    let conflict_ips = entries
        .iter()
        .filter(|entry| entry.conflict)
        .map(|entry| entry.ip.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .len() as u32;
    entries.sort_by(|left, right| {
        parse_ip(&left.ip)
            .cmp(&parse_ip(&right.ip))
            .then_with(|| left.mac.cmp(&right.mac))
    });
    Ok(ArpScanResultDto {
        cidr: network.to_string(),
        elapsed_ms: round1(started.elapsed().as_secs_f64() * 1000.0),
        enhanced: input.enhanced,
        entries,
        conflicts: conflict_ips,
    })
}

fn parse_mac(value: &str) -> Option<[u8; 6]> {
    let bytes: Vec<u8> = value
        .split([':', '-'])
        .map(|part| u8::from_str_radix(part, 16))
        .collect::<Result<_, _>>()
        .ok()?;
    if bytes.len() != 6 {
        return None;
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&bytes);
    Some(mac)
}

fn parse_ip(value: &str) -> Option<Ipv4Addr> {
    value.parse().ok()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
