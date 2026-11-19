use std::net::Ipv4Addr;

use crate::tools::model::ArpAdapterDto;

#[cfg(windows)]
pub fn adapters() -> Vec<ArpAdapterDto> {
    use ipconfig::{IfType, OperStatus};

    ipconfig::get_adapters()
        .unwrap_or_default()
        .into_iter()
        .filter(|adapter| {
            adapter.oper_status() == OperStatus::IfOperStatusUp
                && adapter.if_type() != IfType::SoftwareLoopback
        })
        .flat_map(|adapter| {
            let mac = adapter
                .physical_address()
                .filter(|mac| mac.len() >= 6)
                .map(format_mac)
                .unwrap_or_else(|| "—".to_owned());
            let name = adapter.adapter_name().to_owned();
            let description = adapter.friendly_name().to_owned();
            adapter
                .prefixes()
                .iter()
                .filter_map(|(address, prefix)| match address {
                    std::net::IpAddr::V4(address) if *prefix > 0 && *prefix <= 32 => {
                        let network = ipnet::Ipv4Net::new(*address, *prefix as u8).ok()?;
                        Some(ArpAdapterDto {
                            name: name.clone(),
                            description: description.clone(),
                            ip: address.to_string(),
                            cidr: network.trunc().to_string(),
                            mac: mac.clone(),
                            npcap_name: None,
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(not(windows))]
pub fn adapters() -> Vec<ArpAdapterDto> {
    Vec::new()
}

#[cfg(windows)]
pub async fn scan_native(hosts: &[Ipv4Addr], concurrency: usize) -> Vec<(Ipv4Addr, String)> {
    use futures::StreamExt;

    futures::stream::iter(hosts.iter().copied().map(|address| async move {
        let mac = tokio::task::spawn_blocking(move || send_arp(address)).await;
        match mac {
            Ok(Some(mac)) => Some((address, mac)),
            _ => None,
        }
    }))
    .buffer_unordered(concurrency.clamp(1, 64))
    .filter_map(|entry| async move { entry })
    .collect()
    .await
}

#[cfg(windows)]
fn send_arp(address: Ipv4Addr) -> Option<String> {
    use std::ffi::c_void;

    use windows_sys::Win32::NetworkManagement::IpHelper::SendARP;

    let mut mac = [0u8; 6];
    let mut length = mac.len() as u32;
    // SAFETY: `mac` points to a six-byte buffer and `length` describes its capacity. SendARP
    // is synchronous and does not retain either pointer.
    let result = unsafe {
        SendARP(
            u32::from_ne_bytes(address.octets()),
            0,
            mac.as_mut_ptr() as *mut c_void,
            &mut length,
        )
    };
    if result != 0 || length < 6 {
        return None;
    }
    Some(format_mac(&mac))
}

pub fn format_mac(mac: &[u8]) -> String {
    mac.iter()
        .take(6)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}
