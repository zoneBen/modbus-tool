use std::time::{Duration, Instant};

use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use sha2::Sha256;
use tokio::net::UdpSocket;

use super::model::{SnmpInput, SnmpResultDto, SnmpV3Input, SnmpVarDto, ToolErrorDto, ToolResult};

const MAX_WALK: usize = 2000;
const SNMP_V3: i64 = 3;
const TAG_INTEGER: u8 = 0x02;
const TAG_OCTET_STRING: u8 = 0x04;
const TAG_NULL: u8 = 0x05;
const TAG_OID: u8 = 0x06;
const TAG_SEQUENCE: u8 = 0x30;
const TAG_IP_ADDRESS: u8 = 0x40;
const TAG_COUNTER32: u8 = 0x41;
const TAG_GAUGE32: u8 = 0x42;
const TAG_TIME_TICKS: u8 = 0x43;
const TAG_COUNTER64: u8 = 0x46;
const TAG_NO_SUCH_OBJECT: u8 = 0x80;
const TAG_NO_SUCH_INSTANCE: u8 = 0x81;
const TAG_END_OF_MIB: u8 = 0x82;
const TAG_GET_REQUEST: u8 = 0xa0;
const TAG_GET_NEXT: u8 = 0xa1;
const TAG_GET_RESPONSE: u8 = 0xa2;
const TAG_SET_REQUEST: u8 = 0xa3;
const TAG_REPORT: u8 = 0xa8;

pub async fn query(input: SnmpInput) -> ToolResult<SnmpResultDto> {
    let host = input.host.trim().to_owned();
    if host.is_empty() {
        return Err(ToolErrorDto::new("invalidHost", "Host is required"));
    }
    if input.oids.is_empty() {
        return Err(ToolErrorDto::new(
            "invalidOid",
            "At least one OID is required",
        ));
    }
    let started = Instant::now();
    let operation = input.operation.to_ascii_lowercase();
    let version = input.version.trim().to_ascii_lowercase();
    let variables = match version.as_str() {
        "3" => {
            let v3 = input
                .v3
                .as_ref()
                .ok_or_else(|| ToolErrorDto::new("invalidV3", "SNMPv3 settings are required"))?;
            query_v3(&input, &host, &operation, v3).await?
        }
        _ => query_v1_v2(&input, &host, &operation, &version).await?,
    };
    Ok(SnmpResultDto {
        operation,
        version,
        elapsed_ms: round_ms(started.elapsed().as_secs_f64() * 1000.0),
        variables,
    })
}

async fn query_v1_v2(
    input: &SnmpInput,
    host: &str,
    operation: &str,
    version: &str,
) -> ToolResult<Vec<SnmpVarDto>> {
    let version_number = if version == "1" { 0 } else { 1 };
    let community = input.community.as_bytes();
    let mut socket = open_socket(host, input.port).await?;

    if operation == "walk" {
        let base = input.oids.first().cloned().unwrap_or_default();
        let mut results = Vec::new();
        let mut current = base.clone();
        let mut request_id = 1_000i64;
        loop {
            let pdu = build_request_pdu(request_id, TAG_GET_NEXT, &[current.clone()], input);
            let message = build_v1_message(version_number, community, &pdu);
            let response = exchange(&mut socket, &message, input.timeout_ms, input.retries).await?;
            let parsed = parse_v1_message(&response)?;
            if parsed.error_status != 0 {
                // SNMPv1 agents signal the end of a walk with noSuchName.
                if parsed.error_status == 2 {
                    break;
                }
                return Err(status_error(parsed.error_status));
            }
            let Some(first) = parsed.variables.first() else {
                break;
            };
            if first.value_type == "endOfMibView"
                || first.value_type == "noSuchObject"
                || first.value_type == "noSuchInstance"
            {
                break;
            }
            if !first.oid.starts_with(&format!("{base}.")) {
                break;
            }
            results.push(first.clone());
            if results.len() >= MAX_WALK {
                break;
            }
            if first.oid == current {
                break;
            }
            current = first.oid.clone();
            request_id += 1;
        }
        return Ok(results);
    }

    let pdu_tag = if operation == "set" {
        TAG_SET_REQUEST
    } else {
        TAG_GET_REQUEST
    };
    let pdu = build_request_pdu(1_001, pdu_tag, &input.oids, input);
    let message = build_v1_message(version_number, community, &pdu);
    let response = exchange(&mut socket, &message, input.timeout_ms, input.retries).await?;
    let parsed = parse_v1_message(&response)?;
    if parsed.error_status != 0 {
        return Err(status_error(parsed.error_status));
    }
    Ok(parsed.variables)
}

async fn open_socket(host: &str, port: u16) -> ToolResult<UdpSocket> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|error| ToolErrorDto::new("snmpBind", error.to_string()))?;
    socket
        .connect((host, port))
        .await
        .map_err(|error| ToolErrorDto::new("snmpConnect", error.to_string()))?;
    Ok(socket)
}

async fn exchange(
    socket: &mut UdpSocket,
    message: &[u8],
    timeout_ms: u64,
    retries: u8,
) -> ToolResult<Vec<u8>> {
    let timeout = Duration::from_millis(timeout_ms.clamp(500, 20_000));
    let mut buffer = vec![0u8; 65_535];
    let attempts = retries.saturating_add(1);
    for _ in 0..attempts {
        socket
            .send(message)
            .await
            .map_err(|error| ToolErrorDto::new("snmpSend", error.to_string()))?;
        match tokio::time::timeout(timeout, socket.recv(&mut buffer)).await {
            Ok(Ok(length)) => return Ok(buffer[..length].to_vec()),
            Ok(Err(error)) => {
                return Err(ToolErrorDto::new("snmpReceive", error.to_string()));
            }
            Err(_) => continue,
        }
    }
    Err(ToolErrorDto::new(
        "snmpTimeout",
        "No SNMP response received",
    ))
}

fn build_v1_message(version: i64, community: &[u8], pdu: &[u8]) -> Vec<u8> {
    let mut content = Vec::new();
    content.extend(encode_integer(version));
    content.extend(encode_tlv(TAG_OCTET_STRING, community));
    content.extend_from_slice(pdu);
    encode_tlv(TAG_SEQUENCE, &content)
}

fn build_request_pdu(request_id: i64, tag: u8, oids: &[String], input: &SnmpInput) -> Vec<u8> {
    let mut varbinds = Vec::new();
    for oid in oids {
        let mut varbind = Vec::new();
        match encode_oid(oid) {
            Some(encoded) => varbind.extend(encoded),
            None => varbind.extend(encode_tlv(TAG_OID, &[])),
        }
        if tag == TAG_SET_REQUEST {
            varbind.extend(encode_set_value(input));
        } else {
            varbind.extend(encode_tlv(TAG_NULL, &[]));
        }
        varbinds.extend(encode_tlv(TAG_SEQUENCE, &varbind));
    }
    let mut content = Vec::new();
    content.extend(encode_integer(request_id));
    content.extend(encode_integer(0));
    content.extend(encode_integer(0));
    content.extend(encode_tlv(TAG_SEQUENCE, &varbinds));
    encode_tlv(tag, &content)
}

fn encode_set_value(input: &SnmpInput) -> Vec<u8> {
    let raw = input.value.clone().unwrap_or_default();
    match input.value_type.to_ascii_lowercase().as_str() {
        "integer" => encode_integer(raw.trim().parse::<i64>().unwrap_or(0)),
        "oid" => encode_oid(raw.trim()).unwrap_or_else(|| encode_tlv(TAG_OID, &[])),
        "ipaddress" => {
            let octets: Vec<u8> = raw
                .trim()
                .split('.')
                .filter_map(|part| part.parse::<u8>().ok())
                .collect();
            encode_tlv(TAG_IP_ADDRESS, &octets[..octets.len().min(4)])
        }
        "counter32" | "gauge32" | "timeticks" => {
            let value = raw.trim().parse::<u64>().unwrap_or(0);
            let tag = match input.value_type.to_ascii_lowercase().as_str() {
                "counter32" => TAG_COUNTER32,
                "gauge32" => TAG_GAUGE32,
                _ => TAG_TIME_TICKS,
            };
            encode_tlv(tag, &encode_unsigned_content(value))
        }
        "counter64" => {
            let value = raw.trim().parse::<u64>().unwrap_or(0);
            encode_tlv(TAG_COUNTER64, &encode_unsigned_content(value))
        }
        "hex" => {
            let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = hex::decode(cleaned).unwrap_or_default();
            encode_tlv(TAG_OCTET_STRING, &bytes)
        }
        _ => encode_tlv(TAG_OCTET_STRING, raw.as_bytes()),
    }
}

#[derive(Debug)]
struct ParsedMessage {
    error_status: i64,
    variables: Vec<SnmpVarDto>,
}

fn parse_v1_message(data: &[u8]) -> ToolResult<ParsedMessage> {
    let mut reader = Ber::new(data);
    let (tag, content) = reader.read_tlv()?;
    if tag != TAG_SEQUENCE {
        return Err(ToolErrorDto::new("snmpParse", "Expected SNMP message"));
    }
    let mut message = Ber::new(content);
    let _version = message.read_integer()?;
    let _community = message.read_octets()?;
    let (pdu_tag, pdu_content) = message.read_tlv()?;
    if pdu_tag == TAG_REPORT {
        return Err(ToolErrorDto::new(
            "snmpReport",
            "The agent returned a REPORT PDU",
        ));
    }
    if pdu_tag != TAG_GET_RESPONSE {
        return Err(ToolErrorDto::new(
            "snmpParse",
            format!("Unexpected PDU tag 0x{pdu_tag:02x}"),
        ));
    }
    parse_pdu(pdu_content)
}

fn parse_pdu(content: &[u8]) -> ToolResult<ParsedMessage> {
    let mut pdu = Ber::new(content);
    let _request_id = pdu.read_integer()?;
    let error_status = pdu.read_integer()?;
    let _error_index = pdu.read_integer()?;
    let (list_tag, list) = pdu.read_tlv()?;
    if list_tag != TAG_SEQUENCE {
        return Err(ToolErrorDto::new("snmpParse", "Expected varbind list"));
    }
    let mut variables = Vec::new();
    let mut list_reader = Ber::new(list);
    while !list_reader.is_empty() {
        let (varbind_tag, varbind) = list_reader.read_tlv()?;
        if varbind_tag != TAG_SEQUENCE {
            continue;
        }
        let mut var_reader = Ber::new(varbind);
        let oid = var_reader.read_oid()?;
        let (value_tag, value_content) = var_reader.read_tlv()?;
        let (value_type, value) = decode_value(value_tag, value_content);
        variables.push(SnmpVarDto {
            oid,
            value_type,
            value,
        });
    }
    Ok(ParsedMessage {
        error_status,
        variables,
    })
}

fn decode_value(tag: u8, content: &[u8]) -> (String, String) {
    match tag {
        TAG_INTEGER => ("Integer".to_owned(), read_signed(content).to_string()),
        TAG_OCTET_STRING => {
            if content.iter().all(|byte| {
                *byte == 0x09 || *byte == 0x0a || *byte == 0x0d || (0x20..0x7f).contains(byte)
            }) {
                (
                    "OctetString".to_owned(),
                    String::from_utf8_lossy(content).into_owned(),
                )
            } else {
                ("OctetString".to_owned(), hex::encode_upper(content))
            }
        }
        TAG_NULL => ("Null".to_owned(), "NULL".to_owned()),
        TAG_OID => (
            "OID".to_owned(),
            decode_oid(content).unwrap_or_else(|| "—".to_owned()),
        ),
        TAG_IP_ADDRESS => (
            "IpAddress".to_owned(),
            content
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join("."),
        ),
        TAG_COUNTER32 => ("Counter32".to_owned(), read_unsigned(content).to_string()),
        TAG_GAUGE32 => ("Gauge32".to_owned(), read_unsigned(content).to_string()),
        TAG_TIME_TICKS => {
            let ticks = read_unsigned(content);
            (
                "TimeTicks".to_owned(),
                format!("{ticks} ({})", format_ticks(ticks)),
            )
        }
        TAG_COUNTER64 => ("Counter64".to_owned(), read_unsigned(content).to_string()),
        TAG_NO_SUCH_OBJECT => ("noSuchObject".to_owned(), "noSuchObject".to_owned()),
        TAG_NO_SUCH_INSTANCE => ("noSuchInstance".to_owned(), "noSuchInstance".to_owned()),
        TAG_END_OF_MIB => ("endOfMibView".to_owned(), "endOfMibView".to_owned()),
        other => (format!("0x{other:02x}"), hex::encode_upper(content)),
    }
}

fn format_ticks(ticks: u64) -> String {
    let centiseconds = ticks % 100;
    let total_seconds = ticks / 100;
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 3600) % 24;
    let days = total_seconds / 86_400;
    format!("{days}d {hours:02}:{minutes:02}:{seconds:02}.{centiseconds:02}")
}

fn read_signed(content: &[u8]) -> i64 {
    if content.is_empty() {
        return 0;
    }
    let mut value: i64 = if content[0] & 0x80 != 0 { -1 } else { 0 };
    for byte in content {
        value = (value << 8) | i64::from(*byte);
    }
    value
}

fn read_unsigned(content: &[u8]) -> u64 {
    let mut value = 0u64;
    for byte in content {
        value = (value << 8) | u64::from(*byte);
    }
    value
}

fn status_error(status: i64) -> ToolErrorDto {
    let text = match status {
        1 => "tooBig",
        2 => "noSuchName",
        3 => "badValue",
        4 => "readOnly",
        5 => "genErr",
        _ => "snmpError",
    };
    ToolErrorDto::new(text, format!("SNMP error-status {status}"))
}

fn round_ms(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

// --- BER helpers ---------------------------------------------------------

struct Ber<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Ber<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn read_tlv(&mut self) -> ToolResult<(u8, &'a [u8])> {
        if self.pos >= self.data.len() {
            return Err(ToolErrorDto::new("snmpParse", "Unexpected end of BER data"));
        }
        let tag = self.data[self.pos];
        self.pos += 1;
        let length = self.read_length()?;
        if self.pos + length > self.data.len() {
            return Err(ToolErrorDto::new("snmpParse", "BER length out of range"));
        }
        let content = &self.data[self.pos..self.pos + length];
        self.pos += length;
        Ok((tag, content))
    }

    fn read_length(&mut self) -> ToolResult<usize> {
        if self.pos >= self.data.len() {
            return Err(ToolErrorDto::new("snmpParse", "Missing BER length"));
        }
        let first = self.data[self.pos];
        self.pos += 1;
        if first & 0x80 == 0 {
            return Ok(first as usize);
        }
        let count = (first & 0x7f) as usize;
        if count == 0 || count > 4 || self.pos + count > self.data.len() {
            return Err(ToolErrorDto::new("snmpParse", "Invalid BER length"));
        }
        let mut length = 0usize;
        for _ in 0..count {
            length = (length << 8) | self.data[self.pos] as usize;
            self.pos += 1;
        }
        Ok(length)
    }

    fn read_integer(&mut self) -> ToolResult<i64> {
        let (tag, content) = self.read_tlv()?;
        if tag != TAG_INTEGER {
            return Err(ToolErrorDto::new("snmpParse", "Expected INTEGER"));
        }
        Ok(read_signed(content))
    }

    fn read_octets(&mut self) -> ToolResult<Vec<u8>> {
        let (tag, content) = self.read_tlv()?;
        if tag != TAG_OCTET_STRING {
            return Err(ToolErrorDto::new("snmpParse", "Expected OCTET STRING"));
        }
        Ok(content.to_vec())
    }

    fn read_oid(&mut self) -> ToolResult<String> {
        let (tag, content) = self.read_tlv()?;
        if tag != TAG_OID {
            return Err(ToolErrorDto::new("snmpParse", "Expected OID"));
        }
        decode_oid(content).ok_or_else(|| ToolErrorDto::new("snmpParse", "Invalid OID"))
    }
}

fn encode_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(content.len() + 6);
    output.push(tag);
    encode_length(content.len(), &mut output);
    output.extend_from_slice(content);
    output
}

fn encode_length(length: usize, output: &mut Vec<u8>) {
    if length < 0x80 {
        output.push(length as u8);
        return;
    }
    let bytes = length.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    let count = bytes.len() - first;
    output.push(0x80 | count as u8);
    output.extend_from_slice(&bytes[first..]);
}

fn encode_integer(value: i64) -> Vec<u8> {
    encode_tlv(TAG_INTEGER, &encode_signed_content(value))
}

fn encode_signed_content(value: i64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let mut start = 0;
    while start < bytes.len() - 1 {
        let byte = bytes[start];
        let next = bytes[start + 1];
        if (byte == 0x00 && next & 0x80 == 0) || (byte == 0xff && next & 0x80 != 0) {
            start += 1;
        } else {
            break;
        }
    }
    bytes[start..].to_vec()
}

fn encode_unsigned_content(value: u64) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len() - 1);
    let mut content = bytes[first..].to_vec();
    if content[0] & 0x80 != 0 {
        content.insert(0, 0);
    }
    content
}

fn encode_oid(oid: &str) -> Option<Vec<u8>> {
    let parts: Vec<u64> = oid
        .trim()
        .trim_start_matches('.')
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    if parts.len() < 2 {
        return None;
    }
    let mut content = Vec::new();
    encode_base128(parts[0] * 40 + parts[1], &mut content);
    for part in &parts[2..] {
        encode_base128(*part, &mut content);
    }
    Some(encode_tlv(TAG_OID, &content))
}

fn encode_base128(mut value: u64, output: &mut Vec<u8>) {
    let mut stack = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        stack.push(((value & 0x7f) as u8) | 0x80);
        value >>= 7;
    }
    for byte in stack.iter().rev() {
        output.push(*byte);
    }
}

fn decode_oid(content: &[u8]) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    let mut values = Vec::new();
    let mut current = 0u64;
    for byte in content {
        current = (current << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            values.push(current);
            current = 0;
        }
    }
    if values.is_empty() {
        return None;
    }
    let first = values[0];
    let (root, second) = if first < 40 {
        (0, first)
    } else if first < 80 {
        (1, first - 40)
    } else {
        (2, first - 80)
    };
    let mut parts = vec![root, second];
    parts.extend_from_slice(&values[1..]);
    Some(
        parts
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("."),
    )
}

// --- SNMPv3 --------------------------------------------------------------

async fn query_v3(
    input: &SnmpInput,
    host: &str,
    operation: &str,
    v3: &SnmpV3Input,
) -> ToolResult<Vec<SnmpVarDto>> {
    if operation == "walk" {
        // v3 walk is implemented as repeated GETNEXT, mirroring v1/v2c.
    }
    let security_level = v3.security_level.to_ascii_lowercase();
    if security_level.contains("priv") {
        return Err(ToolErrorDto::new(
            "snmpV3PrivUnsupported",
            "SNMPv3 privacy (encryption) is not supported; use authNoPriv or noAuthNoPriv",
        ));
    }
    let authenticated = security_level == "authnopriv";
    if authenticated
        && v3
            .auth_password
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(ToolErrorDto::new(
            "invalidAuth",
            "SNMPv3 authentication password is required",
        ));
    }

    let mut socket = open_socket(host, input.port).await?;
    // Engine discovery: an empty engine ID request makes the agent reply with
    // a REPORT carrying its engine ID, boots and time.
    let discovery_v3 = SnmpV3Input {
        username: String::new(),
        security_level: "noAuthNoPriv".to_owned(),
        auth_protocol: v3.auth_protocol.clone(),
        auth_password: None,
        context_name: None,
    };
    let discovery = build_v3_message(
        &discovery_v3,
        &[],
        0,
        0,
        false,
        &build_request_pdu(1, TAG_GET_REQUEST, &["1.3.6.1.2.1.1.1.0".to_owned()], input),
        String::new(),
    )?;
    let response = exchange(&mut socket, &discovery, input.timeout_ms, input.retries).await?;
    let (engine_id, engine_boots, engine_time) = parse_v3_security(&response)?;

    if operation == "walk" {
        let base = input.oids.first().cloned().unwrap_or_default();
        let mut results = Vec::new();
        let mut current = base.clone();
        let mut request_id = 2_000;
        loop {
            let pdu = build_request_pdu(request_id, TAG_GET_NEXT, &[current.clone()], input);
            let message = build_v3_message(
                v3,
                &engine_id,
                engine_boots,
                engine_time,
                authenticated,
                &pdu,
                v3.context_name.clone().unwrap_or_default(),
            )?;
            let response = exchange(&mut socket, &message, input.timeout_ms, input.retries).await?;
            let parsed = parse_v3_response(&response)?;
            if parsed.error_status != 0 {
                return Err(status_error(parsed.error_status));
            }
            let Some(first) = parsed.variables.first() else {
                break;
            };
            if first.value_type == "endOfMibView" {
                break;
            }
            if !first.oid.starts_with(&format!("{base}.")) {
                break;
            }
            results.push(first.clone());
            if results.len() >= MAX_WALK || first.oid == current {
                break;
            }
            current = first.oid.clone();
            request_id += 1;
        }
        return Ok(results);
    }

    let pdu_tag = if operation == "set" {
        TAG_SET_REQUEST
    } else {
        TAG_GET_REQUEST
    };
    let pdu = build_request_pdu(2_001, pdu_tag, &input.oids, input);
    let message = build_v3_message(
        v3,
        &engine_id,
        engine_boots,
        engine_time,
        authenticated,
        &pdu,
        v3.context_name.clone().unwrap_or_default(),
    )?;
    let response = exchange(&mut socket, &message, input.timeout_ms, input.retries).await?;
    let parsed = parse_v3_response(&response)?;
    if parsed.error_status != 0 {
        return Err(status_error(parsed.error_status));
    }
    Ok(parsed.variables)
}

#[allow(clippy::too_many_arguments)]
fn build_v3_message(
    v3: &SnmpV3Input,
    engine_id: &[u8],
    engine_boots: i64,
    engine_time: i64,
    authenticated: bool,
    pdu: &[u8],
    context_name: String,
) -> ToolResult<Vec<u8>> {
    let auth_placeholder = if authenticated {
        vec![0u8; auth_length(&v3.auth_protocol)]
    } else {
        Vec::new()
    };
    let usm_content = {
        let mut content = Vec::new();
        content.extend(encode_tlv(TAG_OCTET_STRING, engine_id));
        content.extend(encode_integer(engine_boots));
        content.extend(encode_integer(engine_time));
        content.extend(encode_tlv(TAG_OCTET_STRING, v3.username.as_bytes()));
        content.extend(encode_tlv(TAG_OCTET_STRING, &auth_placeholder));
        content.extend(encode_tlv(TAG_OCTET_STRING, &[]));
        content
    };
    let auth_offset_in_usm = if authenticated {
        Some(
            1 + length_size(usm_content.len()) + find_auth_offset(&usm_content, &auth_placeholder)?,
        )
    } else {
        None
    };
    let usm = encode_tlv(TAG_SEQUENCE, &usm_content);

    let mut header_content = Vec::new();
    header_content.extend(encode_integer(0x7fff_0000 + 1));
    header_content.extend(encode_integer(65_507));
    let flags = 0x04 | if authenticated { 0x01 } else { 0 };
    header_content.extend(encode_tlv(TAG_OCTET_STRING, &[flags]));
    header_content.extend(encode_integer(SNMP_V3));
    let header = encode_tlv(TAG_SEQUENCE, &header_content);

    let mut scoped_content = Vec::new();
    scoped_content.extend(encode_tlv(TAG_OCTET_STRING, engine_id));
    scoped_content.extend(encode_tlv(TAG_OCTET_STRING, context_name.as_bytes()));
    scoped_content.extend_from_slice(pdu);
    let scoped = encode_tlv(TAG_SEQUENCE, &scoped_content);

    let mut body = Vec::new();
    body.extend(encode_integer(SNMP_V3));
    body.extend_from_slice(&header);
    let security_params_header_start = body.len();
    body.extend(encode_tlv(TAG_OCTET_STRING, &usm));
    body.extend_from_slice(&scoped);

    let mut message = encode_tlv(TAG_SEQUENCE, &body);
    if let Some(auth_offset_in_usm) = auth_offset_in_usm {
        let security_header_len = 1 + length_size(usm.len());
        let outer_header_len = 1 + length_size(body.len());
        let auth_offset = outer_header_len
            + security_params_header_start
            + security_header_len
            + auth_offset_in_usm;
        let auth_len = auth_length(&v3.auth_protocol);
        if auth_offset + auth_len > message.len() {
            return Err(ToolErrorDto::new("snmpV3", "Internal auth offset error"));
        }
        let digest = compute_auth(&message, v3, engine_id)?;
        message[auth_offset..auth_offset + auth_len].copy_from_slice(&digest);
    }
    Ok(message)
}

fn find_auth_offset(usm_content: &[u8], placeholder: &[u8]) -> ToolResult<usize> {
    // The auth parameters are the fifth field in the USM sequence. Locate the
    // zero-filled placeholder inside the encoded USM content.
    let needle = encode_tlv(TAG_OCTET_STRING, placeholder);
    usm_content
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .map(|position| position + 1 + length_size(placeholder.len()))
        .ok_or_else(|| ToolErrorDto::new("snmpV3", "Unable to locate auth parameters"))
}

fn length_size(length: usize) -> usize {
    if length < 0x80 {
        1
    } else {
        let bytes = length.to_be_bytes();
        let first = bytes
            .iter()
            .position(|byte| *byte != 0)
            .unwrap_or(bytes.len() - 1);
        1 + (bytes.len() - first)
    }
}

fn auth_length(protocol: &str) -> usize {
    match protocol.to_ascii_lowercase().as_str() {
        "md5" | "sha1" => 12,
        "sha256" => 24,
        _ => 12,
    }
}

fn compute_auth(message: &[u8], v3: &SnmpV3Input, engine_id: &[u8]) -> ToolResult<Vec<u8>> {
    let password = v3.auth_password.as_deref().unwrap_or_default();
    let localized = localize_key(password.as_bytes(), engine_id, &v3.auth_protocol);
    let full = match v3.auth_protocol.to_ascii_lowercase().as_str() {
        "md5" => hmac_sha(&localized, message, HmacType::Md5),
        "sha256" => hmac_sha(&localized, message, HmacType::Sha256),
        _ => hmac_sha(&localized, message, HmacType::Sha1),
    };
    Ok(full[..auth_length(&v3.auth_protocol)].to_vec())
}

enum HmacType {
    Md5,
    Sha1,
    Sha256,
}

fn hmac_sha(key: &[u8], message: &[u8], kind: HmacType) -> Vec<u8> {
    match kind {
        HmacType::Md5 => {
            let mut mac = Hmac::<Md5>::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        HmacType::Sha1 => {
            let mut mac = Hmac::<Sha1>::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        HmacType::Sha256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
    }
}

fn localize_key(password: &[u8], engine_id: &[u8], protocol: &str) -> Vec<u8> {
    let ku = hash_password(password, protocol);
    let mut material = Vec::new();
    material.extend_from_slice(&ku);
    material.extend_from_slice(engine_id);
    material.extend_from_slice(&ku);
    hash_bytes(&material, protocol)
}

fn hash_password(password: &[u8], protocol: &str) -> Vec<u8> {
    // RFC 3414 expands the password to 1 MB before hashing.
    const LENGTH: usize = 1_048_576;
    if password.is_empty() {
        return hash_bytes(&[], protocol);
    }
    let mut buffer = Vec::with_capacity(LENGTH);
    while buffer.len() < LENGTH {
        let remaining = LENGTH - buffer.len();
        buffer.extend_from_slice(&password[..password.len().min(remaining)]);
    }
    hash_bytes(&buffer, protocol)
}

fn hash_bytes(data: &[u8], protocol: &str) -> Vec<u8> {
    use sha1::Digest;
    match protocol.to_ascii_lowercase().as_str() {
        "md5" => Md5::digest(data).to_vec(),
        "sha256" => Sha256::digest(data).to_vec(),
        _ => Sha1::digest(data).to_vec(),
    }
}

fn parse_v3_security(data: &[u8]) -> ToolResult<(Vec<u8>, i64, i64)> {
    let mut reader = Ber::new(data);
    let (tag, content) = reader.read_tlv()?;
    if tag != TAG_SEQUENCE {
        return Err(ToolErrorDto::new("snmpParse", "Expected SNMPv3 message"));
    }
    let mut message = Ber::new(content);
    let _version = message.read_integer()?;
    let (_header_tag, _header) = message.read_tlv()?;
    let security = message.read_octets()?;
    let mut usm = Ber::new(&security);
    let (usm_tag, usm_content) = usm.read_tlv()?;
    if usm_tag != TAG_SEQUENCE {
        return Err(ToolErrorDto::new("snmpParse", "Expected USM parameters"));
    }
    let mut fields = Ber::new(usm_content);
    let engine_id = fields.read_octets()?;
    let boots = fields.read_integer()?;
    let time = fields.read_integer()?;
    Ok((engine_id, boots, time))
}

fn parse_v3_response(data: &[u8]) -> ToolResult<ParsedMessage> {
    let mut reader = Ber::new(data);
    let (tag, content) = reader.read_tlv()?;
    if tag != TAG_SEQUENCE {
        return Err(ToolErrorDto::new("snmpParse", "Expected SNMP message"));
    }
    let mut message = Ber::new(content);
    let _version = message.read_integer()?;
    let _header = message.read_tlv()?;
    let _security = message.read_octets()?;
    let (data_tag, data_content) = message.read_tlv()?;
    if data_tag == TAG_OCTET_STRING {
        return Err(ToolErrorDto::new(
            "snmpV3PrivUnsupported",
            "The agent replied with an encrypted scoped PDU",
        ));
    }
    if data_tag != TAG_SEQUENCE {
        return Err(ToolErrorDto::new("snmpParse", "Expected scoped PDU"));
    }
    let mut scoped = Ber::new(data_content);
    let _context_engine = scoped.read_octets()?;
    let _context_name = scoped.read_octets()?;
    let (pdu_tag, pdu_content) = scoped.read_tlv()?;
    if pdu_tag == TAG_REPORT {
        return Err(ToolErrorDto::new(
            "snmpReport",
            "The agent returned a REPORT PDU; check the SNMPv3 user and engine settings",
        ));
    }
    if pdu_tag != TAG_GET_RESPONSE {
        return Err(ToolErrorDto::new(
            "snmpParse",
            format!("Unexpected PDU tag 0x{pdu_tag:02x}"),
        ));
    }
    parse_pdu(pdu_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_oids() {
        let encoded = encode_oid("1.3.6.1.2.1.1.1.0").expect("oid");
        let (tag, content) = Ber::new(&encoded).read_tlv().expect("tlv");
        assert_eq!(tag, TAG_OID);
        assert_eq!(decode_oid(content).as_deref(), Some("1.3.6.1.2.1.1.1.0"));
    }

    #[test]
    fn encodes_short_and_long_lengths() {
        let short = encode_tlv(TAG_OCTET_STRING, &[1, 2, 3]);
        assert_eq!(short[1], 3);
        let long = encode_tlv(TAG_OCTET_STRING, &vec![0u8; 300]);
        assert_eq!(long[1], 0x82);
    }

    #[test]
    fn signs_with_auth_placeholder() {
        let placeholder = vec![0u8; 12];
        let content = {
            let mut value = Vec::new();
            value.extend(encode_tlv(TAG_OCTET_STRING, &[]));
            value.extend(encode_integer(0));
            value.extend(encode_integer(0));
            value.extend(encode_tlv(TAG_OCTET_STRING, b"user"));
            value.extend(encode_tlv(TAG_OCTET_STRING, &placeholder));
            value.extend(encode_tlv(TAG_OCTET_STRING, &[]));
            value
        };
        let offset = find_auth_offset(&content, &placeholder).expect("offset");
        assert!(offset > 0);
    }
}
