use std::collections::HashMap;

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info};
use sysinfo::System;

use super::model::{PortEntryDto, PortListInput, PortListResultDto, ToolErrorDto, ToolResult};

const MAX_ENTRIES: usize = 5000;

pub fn list(input: PortListInput) -> ToolResult<PortListResultDto> {
    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets = get_sockets_info(af_flags, proto_flags)
        .map_err(|error| ToolErrorDto::new("portEnumeration", error.to_string()))?;

    let names: HashMap<u32, String> = System::new_all()
        .map(|system| {
            system
                .processes()
                .iter()
                .map(|(pid, process)| (pid.as_u32(), process.name().to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();

    let filter = input
        .filter
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);

    let mut entries = Vec::new();
    let mut truncated = false;
    for socket in sockets {
        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
        let (protocol, local_addr, local_port, remote_addr, remote_port, state) =
            match socket.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => (
                    "TCP",
                    tcp.local_addr.to_string(),
                    tcp.local_port,
                    Some(tcp.remote_addr.to_string()),
                    Some(tcp.remote_port),
                    tcp_state(&tcp.state),
                ),
                ProtocolSocketInfo::Udp(udp) => (
                    "UDP",
                    udp.local_addr.to_string(),
                    udp.local_port,
                    None,
                    None,
                    "bound",
                ),
            };

        if input.listening_only {
            let listens = match protocol {
                "TCP" => state == "listen",
                _ => local_port != 0,
            };
            if !listens {
                continue;
            }
        }

        let pid = socket.associated_pids.first().copied().unwrap_or(0);
        let process = names.get(&pid).cloned();
        if let Some(filter) = &filter {
            let haystack = format!(
                "{} {} {} {} {}",
                protocol,
                local_addr,
                local_port,
                pid,
                process.as_deref().unwrap_or("")
            )
            .to_ascii_lowercase();
            if !haystack.contains(filter) {
                continue;
            }
        }

        entries.push(PortEntryDto {
            protocol: protocol.to_owned(),
            local_addr,
            local_port,
            remote_addr: remote_addr.filter(|value| value != "0.0.0.0" && value != "::"),
            remote_port: remote_port.filter(|value| *value != 0),
            state: state.to_owned(),
            pid,
            process,
        });
    }

    entries.sort_by(|left, right| {
        left.local_port
            .cmp(&right.local_port)
            .then_with(|| left.protocol.cmp(&right.protocol))
            .then_with(|| left.local_addr.cmp(&right.local_addr))
    });

    Ok(PortListResultDto { entries, truncated })
}

fn tcp_state(state: &TcpState) -> &'static str {
    match state {
        TcpState::Closed => "closed",
        TcpState::Listen => "listen",
        TcpState::SynSent => "synSent",
        TcpState::SynReceived => "synReceived",
        TcpState::Established => "established",
        TcpState::FinWait1 => "finWait1",
        TcpState::FinWait2 => "finWait2",
        TcpState::CloseWait => "closeWait",
        TcpState::Closing => "closing",
        TcpState::LastAck => "lastAck",
        TcpState::TimeWait => "timeWait",
        TcpState::DeleteTcb => "deleteTcb",
        _ => "unknown",
    }
}
