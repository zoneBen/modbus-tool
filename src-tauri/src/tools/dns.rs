use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use tokio::time::timeout;

use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::error::ResolveErrorKind;
use hickory_resolver::proto::op::ResponseCode;
use hickory_resolver::proto::rr::RecordType;

use super::model::{DnsAnswerDto, DnsQueryInput, DnsResultDto, ToolErrorDto, ToolResult};

const DEFAULT_DNS_PORT: u16 = 53;
const MAX_ANSWERS: usize = 200;

pub async fn query(input: DnsQueryInput) -> ToolResult<DnsResultDto> {
    let name = input.name.trim().to_owned();
    if name.is_empty() {
        return Err(ToolErrorDto::new("invalidName", "DNS name is required"));
    }
    let record_type = parse_record_type(&input.record_type)
        .ok_or_else(|| ToolErrorDto::new("invalidRecordType", "Unsupported DNS record type"))?;
    let timeout_ms = input.timeout_ms.clamp(500, 30_000);
    let mut options = ResolverOpts::default();
    options.timeout = Duration::from_millis(timeout_ms);
    options.attempts = 2;
    options.cache_size = 0;

    let (resolver, server_label) = match input.server.as_deref().map(str::trim) {
        Some(server) if !server.is_empty() => {
            let (ip, port) = parse_server(server)?;
            let group = NameServerConfigGroup::from_ips_clear(&[ip], port, true);
            let config = ResolverConfig::from_parts(None, Vec::new(), group);
            (
                TokioAsyncResolver::tokio(config, options),
                server.to_owned(),
            )
        }
        _ => {
            let (config, _) = hickory_resolver::system_conf::read_system_conf()
                .map_err(|error| ToolErrorDto::new("systemResolver", error.to_string()))?;
            (
                TokioAsyncResolver::tokio(config, options),
                "system".to_owned(),
            )
        }
    };

    let started = Instant::now();
    let query_name = if record_type == RecordType::PTR
        && let Ok(ip) = name.parse::<IpAddr>()
    {
        reverse_name(ip)
    } else {
        name.clone()
    };
    let lookup = resolver.lookup(query_name, record_type).await;

    match lookup {
        Ok(lookup) => {
            let answers: Vec<DnsAnswerDto> = lookup
                .records()
                .iter()
                .take(MAX_ANSWERS)
                .map(|record| DnsAnswerDto {
                    name: record.name().to_string(),
                    record_type: record.record_type().to_string(),
                    ttl: record.ttl(),
                    value: record
                        .data()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "—".to_owned()),
                })
                .collect();
            let status = if answers.is_empty() {
                "noRecords"
            } else {
                "ok"
            };
            Ok(DnsResultDto {
                question: name,
                record_type: record_type.to_string(),
                server: server_label,
                status: status.to_owned(),
                elapsed_ms: elapsed(started),
                answers,
            })
        }
        Err(error) => {
            if let ResolveErrorKind::NoRecordsFound { response_code, .. } = error.kind() {
                let status = match response_code {
                    ResponseCode::NXDomain => "nxdomain",
                    _ => "noRecords",
                };
                return Ok(DnsResultDto {
                    question: name,
                    record_type: record_type.to_string(),
                    server: server_label,
                    status: status.to_owned(),
                    elapsed_ms: elapsed(started),
                    answers: Vec::new(),
                });
            }
            Err(ToolErrorDto::new("dnsError", error.to_string()))
        }
    }
}

fn parse_server(value: &str) -> ToolResult<(IpAddr, u16)> {
    let candidate = if value.starts_with('[') {
        value.to_owned()
    } else if value.parse::<IpAddr>().is_ok() {
        format!("{value}:{DEFAULT_DNS_PORT}")
    } else {
        value.to_owned()
    };
    let address: SocketAddr = candidate
        .parse()
        .map_err(|_| ToolErrorDto::new("invalidServer", "DNS server must be an IP or IP:port"))?;
    Ok((address.ip(), address.port()))
}

pub async fn reverse_lookup_name(ip: IpAddr) -> Option<String> {
    let (config, _) = hickory_resolver::system_conf::read_system_conf().ok()?;
    let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());
    let lookup = timeout(
        Duration::from_millis(800),
        resolver.lookup(reverse_name(ip), RecordType::PTR),
    )
    .await
    .ok()?
    .ok()?;
    lookup
        .records()
        .first()
        .and_then(|record| record.data())
        .map(ToString::to_string)
        .map(|name| name.trim_end_matches('.').to_owned())
}

fn reverse_name(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(address) => {
            let octets = address.octets();
            format!(
                "{}.{}.{}.{}.in-addr.arpa.",
                octets[3], octets[2], octets[1], octets[0]
            )
        }
        IpAddr::V6(address) => {
            let mut nibbles = String::new();
            for byte in address.octets().iter().rev() {
                nibbles.push_str(&format!("{:x}.{:x}.", byte & 0x0f, byte >> 4));
            }
            format!("{nibbles}ip6.arpa.")
        }
    }
}

fn parse_record_type(value: &str) -> Option<RecordType> {
    match value.trim().to_ascii_uppercase().as_str() {
        "A" => Some(RecordType::A),
        "AAAA" => Some(RecordType::AAAA),
        "CNAME" => Some(RecordType::CNAME),
        "MX" => Some(RecordType::MX),
        "TXT" => Some(RecordType::TXT),
        "NS" => Some(RecordType::NS),
        "SOA" => Some(RecordType::SOA),
        "SRV" => Some(RecordType::SRV),
        "PTR" => Some(RecordType::PTR),
        "CAA" => Some(RecordType::CAA),
        _ => None,
    }
}

fn elapsed(started: Instant) -> f64 {
    (started.elapsed().as_secs_f64() * 1000.0 * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dns_servers_with_and_without_ports() {
        assert_eq!(
            parse_server("8.8.8.8").expect("server"),
            (IpAddr::from([8, 8, 8, 8]), 53)
        );
        assert_eq!(
            parse_server("1.1.1.1:5353").expect("server"),
            (IpAddr::from([1, 1, 1, 1]), 5353)
        );
        assert!(parse_server("[2001:4860:4860::8888]:53").is_ok());
        assert!(parse_server("dns.example.com").is_err());
    }

    #[test]
    fn rejects_unknown_record_types() {
        assert!(parse_record_type("A").is_some());
        assert!(parse_record_type("ptr").is_some());
        assert!(parse_record_type("NOPE").is_none());
    }

    #[test]
    fn builds_reverse_lookup_names() {
        assert_eq!(
            reverse_name("192.168.1.10".parse().expect("ipv4")),
            "10.1.168.192.in-addr.arpa."
        );
        assert!(reverse_name("2001:db8::1".parse().expect("ipv6")).ends_with(".ip6.arpa."));
    }
}
