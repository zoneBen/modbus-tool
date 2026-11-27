use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use futures::future::join_all;
use tokio::net::UdpSocket;

use super::model::{
    DiscoveredDeviceDto, DiscoveryInput, DiscoveryResultDto, ToolErrorDto, ToolResult,
};

const SSDP_ADDR: &str = "239.255.255.250:1900";
const MDNS_ADDR: &str = "224.0.0.251:5353";
const MAX_DEVICES: usize = 300;
const MDNS_SERVICES: &[&str] = &[
    "_services._dns-sd._udp.local",
    "_http._tcp.local",
    "_https._tcp.local",
    "_ipp._tcp.local",
    "_printer._tcp.local",
    "_airplay._tcp.local",
    "_googlecast._tcp.local",
    "_spotify-connect._tcp.local",
    "_hap._tcp.local",
    "_smb._tcp.local",
    "_ssh._tcp.local",
    "_workstation._tcp.local",
];

pub async fn scan(input: DiscoveryInput) -> ToolResult<DiscoveryResultDto> {
    let started = Instant::now();
    let timeout = Duration::from_millis(input.timeout_ms.clamp(1_000, 15_000));

    let (ssdp, mdns) = tokio::join!(
        async {
            if input.ssdp {
                discover_ssdp(timeout).await.ok()
            } else {
                None
            }
        },
        async {
            if input.mdns {
                discover_mdns(timeout).await.ok()
            } else {
                None
            }
        }
    );

    let mut devices = Vec::new();
    if let Some(mut list) = ssdp {
        devices.append(&mut list);
    }
    if let Some(mut list) = mdns {
        devices.append(&mut list);
    }

    dedupe(&mut devices);
    devices.truncate(MAX_DEVICES);
    if devices.is_empty() {
        // Discovery often depends on the network allowing multicast; an empty
        // result is not an error, the caller decides how to present it.
        tracing::debug!("no discovery responses received");
    }

    Ok(DiscoveryResultDto {
        elapsed_ms: round_ms(started.elapsed().as_secs_f64() * 1000.0),
        devices,
    })
}

fn dedupe(devices: &mut Vec<DiscoveredDeviceDto>) {
    let mut seen = std::collections::HashSet::new();
    devices.retain(|device| {
        let key = format!(
            "{}|{}|{}|{}",
            device.source,
            device.name,
            device.address.as_deref().unwrap_or(""),
            device.port.unwrap_or(0)
        );
        seen.insert(key)
    });
}

async fn discover_ssdp(timeout: Duration) -> Result<Vec<DiscoveredDeviceDto>, ToolErrorDto> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|error| ToolErrorDto::new("ssdpBind", error.to_string()))?;
    let request = format!(
        "M-SEARCH * HTTP/1.1\r\nHOST: {SSDP_ADDR}\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: ssdp:all\r\n\r\n"
    );
    for _ in 0..2 {
        let _ = socket.send_to(request.as_bytes(), SSDP_ADDR).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    let deadline = Instant::now() + timeout;
    let mut responses: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut buffer = vec![0u8; 8192];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let read = tokio::time::timeout(remaining, socket.recv_from(&mut buffer)).await;
        let Ok(Ok((length, _peer))) = read else {
            break;
        };
        let text = String::from_utf8_lossy(&buffer[..length]);
        let headers = parse_ssdp_headers(&text);
        let key = headers
            .get("usn")
            .or_else(|| headers.get("location"))
            .or_else(|| headers.get("st"))
            .cloned()
            .unwrap_or_else(|| format!("{length}-{}", responses.len()));
        responses.insert(key, headers);
    }

    let fetches = responses.into_values().map(|headers| async move {
        let location = headers.get("location").cloned();
        let listing = match location.as_deref() {
            Some(url) => fetch_upnp_description(url).await,
            None => None,
        };
        let host = location
            .as_deref()
            .and_then(|url| reqwest::Url::parse(url).ok())
            .and_then(|url| url.host_str().map(str::to_owned));
        DiscoveredDeviceDto {
            name: listing
                .as_ref()
                .and_then(|item| item.friendly_name.clone())
                .or_else(|| headers.get("server").cloned())
                .unwrap_or_else(|| "UPnP device".to_owned()),
            host: host.clone(),
            address: host,
            port: None,
            service: headers
                .get("st")
                .or_else(|| headers.get("nt"))
                .cloned()
                .unwrap_or_else(|| "ssdp".to_owned()),
            source: if listing.is_some() { "upnp" } else { "ssdp" }.to_owned(),
            location,
            server: headers.get("server").cloned(),
            usn: headers.get("usn").cloned(),
            manufacturer: listing.as_ref().and_then(|item| item.manufacturer.clone()),
            model: listing.as_ref().and_then(|item| item.model.clone()),
            detail: headers.get("st").cloned(),
        }
    });
    Ok(join_all(fetches).await)
}

fn parse_ssdp_headers(text: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in text.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    headers
}

#[derive(Default)]
struct UpnpListing {
    friendly_name: Option<String>,
    manufacturer: Option<String>,
    model: Option<String>,
}

async fn fetch_upnp_description(url: &str) -> Option<UpnpListing> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1_800))
        .build()
        .ok()?;
    let body = client.get(url).send().await.ok()?.text().await.ok()?;
    Some(UpnpListing {
        friendly_name: xml_tag(&body, "friendlyName"),
        manufacturer: xml_tag(&body, "manufacturer"),
        model: xml_tag(&body, "modelName").or_else(|| xml_tag(&body, "modelNumber")),
    })
}

fn xml_tag(body: &str, tag: &str) -> Option<String> {
    let pattern = regex::Regex::new(&format!(r"(?is)<{tag}[^>]*>(.*?)</{tag}>")).ok()?;
    let captured = pattern.captures(body)?.get(1)?.as_str().trim();
    if captured.is_empty() {
        return None;
    }
    Some(unescape_xml(captured))
}

fn unescape_xml(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[derive(Default, Clone)]
struct MdnsRecord {
    service: Option<String>,
    target: Option<String>,
    port: Option<u16>,
    addresses: Vec<String>,
    txt: HashMap<String, String>,
}

async fn discover_mdns(timeout: Duration) -> Result<Vec<DiscoveredDeviceDto>, ToolErrorDto> {
    let socket = bind_mdns_socket()?;

    let query = build_mdns_query(MDNS_SERVICES);
    let _ = socket.send_to(&query, MDNS_ADDR).await;

    let deadline = Instant::now() + timeout;
    let mut records: HashMap<String, MdnsRecord> = HashMap::new();
    let mut buffer = vec![0u8; 9000];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let read = tokio::time::timeout(remaining, socket.recv_from(&mut buffer)).await;
        let Ok(Ok((length, _peer))) = read else {
            break;
        };
        let _ = parse_mdns_message(&buffer[..length], &mut records);
    }

    let mut devices = Vec::new();
    for (instance, record) in &records {
        if record.target.is_none() && record.port.is_none() && record.txt.is_empty() {
            continue;
        }
        let name = record
            .txt
            .get("fn")
            .cloned()
            .or_else(|| record.txt.get("md").cloned())
            .unwrap_or_else(|| instance_label(instance));
        let address = record.addresses.first().cloned();
        devices.push(DiscoveredDeviceDto {
            name,
            host: record.target.clone(),
            address,
            port: record.port,
            service: record.service.clone().unwrap_or_default(),
            source: "mdns".to_owned(),
            location: None,
            server: None,
            usn: None,
            manufacturer: record.txt.get("mf").cloned(),
            model: record.txt.get("md").cloned(),
            detail: record
                .txt
                .get("ty")
                .cloned()
                .or_else(|| (!record.txt.is_empty()).then(|| format!("{} TXT", record.txt.len()))),
        });
    }
    Ok(devices)
}

fn bind_mdns_socket() -> Result<UdpSocket, ToolErrorDto> {
    use socket2::{Domain, Protocol, Socket, Type};

    if let Ok(socket) = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)) {
        let shared = socket.set_reuse_address(true).is_ok()
            && socket
                .bind(&SocketAddr::from(([0, 0, 0, 0], 5353)).into())
                .is_ok()
            && socket
                .join_multicast_v4(&Ipv4Addr::new(224, 0, 0, 251), &Ipv4Addr::UNSPECIFIED)
                .is_ok()
            && socket.set_nonblocking(true).is_ok();
        if shared {
            let std_socket: std::net::UdpSocket = socket.into();
            if let Ok(udp) = UdpSocket::from_std(std_socket) {
                return Ok(udp);
            }
        }
    }

    // Fall back to an ephemeral port; mDNS queries request unicast replies (QU),
    // so many responders still deliver their answers to this socket.
    let std_socket = std::net::UdpSocket::bind("0.0.0.0:0")
        .map_err(|error| ToolErrorDto::new("mdnsBind", error.to_string()))?;
    let _ = std_socket.join_multicast_v4(&Ipv4Addr::new(224, 0, 0, 251), &Ipv4Addr::UNSPECIFIED);
    std_socket
        .set_nonblocking(true)
        .map_err(|error| ToolErrorDto::new("mdnsBind", error.to_string()))?;
    UdpSocket::from_std(std_socket)
        .map_err(|error| ToolErrorDto::new("mdnsBind", error.to_string()))
}

fn instance_label(instance: &str) -> String {
    instance
        .split("._")
        .next()
        .unwrap_or(instance)
        .trim_end_matches('.')
        .to_owned()
}

fn build_mdns_query(services: &[&str]) -> Vec<u8> {
    let mut packet = Vec::new();
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&(services.len() as u16).to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    for service in services {
        encode_dns_name(service, &mut packet);
        packet.extend_from_slice(&12u16.to_be_bytes());
        // QU bit: ask responders to send a unicast reply to the querying port.
        packet.extend_from_slice(&0x8001u16.to_be_bytes());
    }
    packet
}

fn encode_dns_name(name: &str, packet: &mut Vec<u8>) {
    for label in name.trim_end_matches('.').split('.') {
        let bytes = label.as_bytes();
        let length = bytes.len().min(63);
        packet.push(length as u8);
        packet.extend_from_slice(&bytes[..length]);
    }
    packet.push(0);
}

fn parse_mdns_message(data: &[u8], records: &mut HashMap<String, MdnsRecord>) -> Option<()> {
    if data.len() < 12 {
        return None;
    }
    let questions = u16::from_be_bytes([data[4], data[5]]) as usize;
    let answers = u16::from_be_bytes([data[6], data[7]]) as usize;
    let authority = u16::from_be_bytes([data[8], data[9]]) as usize;
    let additional = u16::from_be_bytes([data[10], data[11]]) as usize;

    let mut pos = 12;
    for _ in 0..questions {
        let (_, next) = read_dns_name(data, pos)?;
        pos = next + 4;
        if pos > data.len() {
            return None;
        }
    }

    for _ in 0..(answers + authority + additional) {
        let (owner, next) = read_dns_name(data, pos)?;
        pos = next;
        if pos + 10 > data.len() {
            return None;
        }
        let record_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let rd_length = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
        let rdata = pos + 10;
        let end = rdata.checked_add(rd_length)?;
        if end > data.len() {
            return None;
        }
        match record_type {
            12 => {
                let (instance, _) = read_dns_name(data, rdata)?;
                let service = owner.trim_end_matches('.').to_owned();
                let entry = records.entry(normalize(&instance)).or_default();
                if !service.starts_with("_services") {
                    entry.service.get_or_insert(service);
                }
            }
            16 => {
                let entry = records.entry(normalize(&owner)).or_default();
                let mut cursor = rdata;
                while cursor < end {
                    let length = data[cursor] as usize;
                    cursor += 1;
                    if cursor + length > end {
                        break;
                    }
                    let text = String::from_utf8_lossy(&data[cursor..cursor + length]);
                    cursor += length;
                    if let Some((key, value)) = text.split_once('=') {
                        entry.txt.insert(key.to_owned(), value.to_owned());
                    }
                }
            }
            33 => {
                if rdata + 6 <= end {
                    let port = u16::from_be_bytes([data[rdata + 4], data[rdata + 5]]);
                    let (target, _) = read_dns_name(data, rdata + 6)?;
                    let entry = records.entry(normalize(&owner)).or_default();
                    entry.port = Some(port);
                    entry.target = Some(target.trim_end_matches('.').to_owned());
                }
            }
            1 if rd_length == 4 => {
                let ip = Ipv4Addr::new(
                    data[rdata],
                    data[rdata + 1],
                    data[rdata + 2],
                    data[rdata + 3],
                );
                records
                    .entry(normalize(&owner))
                    .or_default()
                    .addresses
                    .push(ip.to_string());
            }
            28 if rd_length == 16 => {
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&data[rdata..rdata + 16]);
                records
                    .entry(normalize(&owner))
                    .or_default()
                    .addresses
                    .push(Ipv6Addr::from(octets).to_string());
            }
            _ => {}
        }
        pos = end;
    }
    Some(())
}

fn normalize(name: &str) -> String {
    name.trim_end_matches('.').to_owned()
}

fn read_dns_name(data: &[u8], start: usize) -> Option<(String, usize)> {
    let mut labels = Vec::new();
    let mut pos = start;
    let mut next = start;
    let mut jumped = false;
    let mut hops = 0;
    loop {
        if pos >= data.len() || hops > 32 {
            return None;
        }
        let length = data[pos];
        if length == 0 {
            if !jumped {
                next = pos + 1;
            }
            break;
        }
        if length & 0xc0 == 0xc0 {
            if pos + 1 >= data.len() {
                return None;
            }
            let pointer = (((length & 0x3f) as usize) << 8) | data[pos + 1] as usize;
            if !jumped {
                next = pos + 2;
            }
            jumped = true;
            pos = pointer;
            hops += 1;
            continue;
        }
        let length = length as usize;
        if pos + 1 + length > data.len() {
            return None;
        }
        labels.push(String::from_utf8_lossy(&data[pos + 1..pos + 1 + length]).into_owned());
        pos += 1 + length;
        if !jumped {
            next = pos;
        }
    }
    Some((labels.join("."), next))
}

fn round_ms(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compressed_dns_names() {
        // header + question is not needed for this helper
        let mut data = vec![0u8; 12];
        data.extend_from_slice(&[3, b'f', b'o', b'o', 3, b'b', b'a', b'r', 0]);
        let (name, next) = read_dns_name(&data, 12).expect("name");
        assert_eq!(name, "foo.bar");
        assert_eq!(next, data.len());

        let pointer_offset = data.len() as u16;
        data.extend_from_slice(&[0xc0, 12]);
        let (pointed, _) = read_dns_name(&data, pointer_offset as usize).expect("pointer");
        assert_eq!(pointed, "foo.bar");
    }

    #[test]
    fn builds_queries_for_each_service() {
        let query = build_mdns_query(&["_http._tcp.local", "_ipp._tcp.local"]);
        assert_eq!(u16::from_be_bytes([query[4], query[5]]), 2);
        assert!(query.ends_with(&[0, 12, 0x80, 0x01]));
    }

    #[test]
    fn extracts_xml_tags() {
        let body = "<root><friendlyName>Living Room &amp; TV</friendlyName></root>";
        assert_eq!(
            xml_tag(body, "friendlyName").as_deref(),
            Some("Living Room & TV")
        );
    }
}
