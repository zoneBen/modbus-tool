//! RTU, ASCII and MBAP framing.

use serde::{Deserialize, Serialize};

use crate::checksum::{crc16_bytes, lrc};
use crate::error::ModbusError;
use crate::pdu::MAX_PDU_LEN;

pub const MAX_RTU_ADU_LEN: usize = 256;
pub const MAX_ASCII_FRAME_LEN: usize = 513;
pub const MBAP_HEADER_LEN: usize = 7;

/// Framing used on top of the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Framing {
    Rtu,
    Ascii,
    Mbap,
}

/// A decoded application data unit (unit id + PDU).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adu {
    pub transaction_id: Option<u16>,
    pub unit_id: u8,
    pub pdu: Vec<u8>,
}

/// Encode a PDU into a complete frame.
///
/// The transaction id is only used by MBAP framing and defaults to zero for
/// RTU/ASCII.
pub fn encode(
    framing: Framing,
    unit_id: u8,
    transaction_id: Option<u16>,
    pdu: &[u8],
) -> Result<Vec<u8>, ModbusError> {
    if pdu.is_empty() || pdu.len() > MAX_PDU_LEN {
        return Err(ModbusError::InvalidByteCount(pdu.len()));
    }
    match framing {
        Framing::Rtu => Ok(encode_rtu(unit_id, pdu)),
        Framing::Ascii => encode_ascii(unit_id, pdu),
        Framing::Mbap => encode_mbap(transaction_id.unwrap_or(0), unit_id, pdu),
    }
}

/// Decode a complete frame into an ADU.
pub fn decode(framing: Framing, bytes: &[u8]) -> Result<Adu, ModbusError> {
    match framing {
        Framing::Rtu => decode_rtu(bytes),
        Framing::Ascii => decode_ascii(bytes),
        Framing::Mbap => decode_mbap(bytes),
    }
}

/// Compute the total RTU ADU length.
pub fn encode_rtu(unit_id: u8, pdu: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(pdu.len() + 3);
    body.push(unit_id);
    body.extend_from_slice(pdu);
    body.extend_from_slice(&crc16_bytes(&body));
    body
}

/// Decode and verify an RTU ADU.
pub fn decode_rtu(bytes: &[u8]) -> Result<Adu, ModbusError> {
    if bytes.len() < 4 {
        return Err(ModbusError::FrameTooShort {
            expected: 4,
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_RTU_ADU_LEN {
        return Err(ModbusError::FrameTooLong(bytes.len()));
    }
    let body = &bytes[..bytes.len() - 2];
    let expected = crc16_bytes(body);
    if bytes[bytes.len() - 2..] != expected[..] {
        return Err(ModbusError::CrcMismatch);
    }
    Ok(Adu {
        transaction_id: None,
        unit_id: body[0],
        pdu: body[1..].to_vec(),
    })
}

/// Encode an ASCII frame (`:HHHH...\r\n`).
pub fn encode_ascii(unit_id: u8, pdu: &[u8]) -> Result<Vec<u8>, ModbusError> {
    let mut body = Vec::with_capacity(pdu.len() + 1);
    body.push(unit_id);
    body.extend_from_slice(pdu);
    body.push(lrc(&body));

    let mut frame = String::with_capacity(1 + body.len() * 2 + 2);
    frame.push(':');
    for byte in &body {
        frame.push_str(&format!("{byte:02X}"));
    }
    frame.push_str("\r\n");
    if frame.len() > MAX_ASCII_FRAME_LEN {
        return Err(ModbusError::FrameTooLong(frame.len()));
    }
    Ok(frame.into_bytes())
}

/// Decode an ASCII frame and verify its LRC.
pub fn decode_ascii(bytes: &[u8]) -> Result<Adu, ModbusError> {
    if bytes.len() > MAX_ASCII_FRAME_LEN {
        return Err(ModbusError::FrameTooLong(bytes.len()));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ModbusError::InvalidAsciiFrame)?;
    let text = text.trim_end_matches(['\r', '\n']);
    let payload = text
        .strip_prefix(':')
        .ok_or(ModbusError::InvalidAsciiFrame)?;
    if payload.is_empty() || payload.len() % 2 != 0 {
        return Err(ModbusError::InvalidAsciiFrame);
    }
    let mut data = Vec::with_capacity(payload.len() / 2);
    for chunk in payload.as_bytes().as_chunks::<2>().0 {
        let pair = std::str::from_utf8(chunk).map_err(|_| ModbusError::InvalidAsciiFrame)?;
        let byte = u8::from_str_radix(pair, 16).map_err(|_| ModbusError::InvalidAsciiFrame)?;
        data.push(byte);
    }
    if data.len() < 3 {
        return Err(ModbusError::InvalidAsciiFrame);
    }
    let expected_lrc = lrc(&data[..data.len() - 1]);
    if data[data.len() - 1] != expected_lrc {
        return Err(ModbusError::LrcMismatch);
    }
    Ok(Adu {
        transaction_id: None,
        unit_id: data[0],
        pdu: data[1..data.len() - 1].to_vec(),
    })
}

/// Encode an MBAP frame.
pub fn encode_mbap(transaction_id: u16, unit_id: u8, pdu: &[u8]) -> Result<Vec<u8>, ModbusError> {
    let length = pdu.len() + 1;
    if !(2..=254).contains(&length) {
        return Err(ModbusError::InvalidMbapLength(length as u16));
    }
    let mut frame = Vec::with_capacity(MBAP_HEADER_LEN + pdu.len());
    frame.extend_from_slice(&transaction_id.to_be_bytes());
    frame.extend_from_slice(&[0, 0]);
    frame.extend_from_slice(&(length as u16).to_be_bytes());
    frame.push(unit_id);
    frame.extend_from_slice(pdu);
    Ok(frame)
}

/// Decode and validate an MBAP frame.
pub fn decode_mbap(bytes: &[u8]) -> Result<Adu, ModbusError> {
    if bytes.len() < MBAP_HEADER_LEN + 1 {
        return Err(ModbusError::FrameTooShort {
            expected: MBAP_HEADER_LEN + 1,
            actual: bytes.len(),
        });
    }
    let transaction_id = u16::from_be_bytes([bytes[0], bytes[1]]);
    let protocol_id = u16::from_be_bytes([bytes[2], bytes[3]]);
    if protocol_id != 0 {
        return Err(ModbusError::InvalidProtocolId(protocol_id));
    }
    let length = u16::from_be_bytes([bytes[4], bytes[5]]);
    if !(2..=254).contains(&length) {
        return Err(ModbusError::InvalidMbapLength(length));
    }
    let total = 6 + length as usize;
    if bytes.len() != total {
        return Err(ModbusError::InvalidMbapLength(length));
    }
    let pdu = bytes[MBAP_HEADER_LEN..].to_vec();
    if pdu.is_empty() {
        return Err(ModbusError::FrameTooShort {
            expected: MBAP_HEADER_LEN + 1,
            actual: bytes.len(),
        });
    }
    Ok(Adu {
        transaction_id: Some(transaction_id),
        unit_id: bytes[6],
        pdu,
    })
}

/// Expected total frame length for stream transports, given a prefix that has
/// already been read. Returns `None` when more bytes are needed or when the
/// frame cannot be framed from the prefix (unknown function code).
pub fn expected_frame_len(framing: Framing, prefix: &[u8]) -> Option<usize> {
    match framing {
        Framing::Rtu => expected_rtu_len(prefix),
        Framing::Ascii => None,
        Framing::Mbap => expected_mbap_len(prefix),
    }
}

fn expected_rtu_len(prefix: &[u8]) -> Option<usize> {
    if prefix.len() < 2 {
        return None;
    }
    let function = prefix[1];
    if function & 0x80 != 0 {
        return Some(5);
    }
    match function {
        0x01..=0x04 => {
            if prefix.len() < 3 {
                return None;
            }
            Some(5 + prefix[2] as usize)
        }
        0x05 | 0x06 | 0x0F | 0x10 => Some(8),
        _ => None,
    }
}

fn expected_mbap_len(prefix: &[u8]) -> Option<usize> {
    if prefix.len() < 6 {
        return None;
    }
    let length = u16::from_be_bytes([prefix[4], prefix[5]]);
    if !(2..=254).contains(&length) {
        return None;
    }
    Some(6 + length as usize)
}

/// Minimal 3.5 character silent interval in microseconds.
pub fn t35_micros(baud: u32) -> u64 {
    if baud == 0 || baud > 19_200 {
        return 1_750;
    }
    let micros = (3.5 * 11.0 / f64::from(baud) * 1_000_000.0).round() as u64;
    micros.max(1_750)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtu_round_trip_and_crc() {
        let pdu = [0x03, 0x00, 0x00, 0x00, 0x0A];
        let frame = encode_rtu(0x01, &pdu);
        assert_eq!(frame, vec![0x01, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xC5, 0xCD]);
        let adu = decode_rtu(&frame).expect("decode");
        assert_eq!(adu.unit_id, 1);
        assert_eq!(adu.pdu, pdu);

        let mut broken = frame;
        broken[6] ^= 0xFF;
        assert_eq!(decode_rtu(&broken), Err(ModbusError::CrcMismatch));
    }

    #[test]
    fn ascii_round_trip_and_lrc() {
        let pdu = [0x03, 0x00, 0x00, 0x00, 0x0A];
        let frame = encode_ascii(0x01, &pdu).expect("encode");
        assert_eq!(frame, b":01030000000AF2\r\n".to_vec());
        let adu = decode_ascii(&frame).expect("decode");
        assert_eq!(adu.unit_id, 1);
        assert_eq!(adu.pdu, pdu);

        let mut broken = frame;
        broken[2] = b'9';
        assert_eq!(decode_ascii(&broken), Err(ModbusError::LrcMismatch));
        assert_eq!(
            decode_ascii(b"not-a-frame"),
            Err(ModbusError::InvalidAsciiFrame)
        );
    }

    #[test]
    fn mbap_round_trip() {
        let pdu = [0x03, 0x00, 0x00, 0x00, 0x0A];
        let frame = encode_mbap(0x1234, 0x01, &pdu).expect("encode");
        assert_eq!(
            frame,
            vec![
                0x12, 0x34, 0x00, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00, 0x00, 0x00, 0x0A
            ]
        );
        let adu = decode_mbap(&frame).expect("decode");
        assert_eq!(adu.transaction_id, Some(0x1234));
        assert_eq!(adu.unit_id, 1);
        assert_eq!(adu.pdu, pdu);

        let mut bad_protocol = frame.clone();
        bad_protocol[3] = 1;
        assert_eq!(
            decode_mbap(&bad_protocol),
            Err(ModbusError::InvalidProtocolId(1))
        );

        let mut bad_length = frame;
        bad_length[5] = 9;
        assert_eq!(
            decode_mbap(&bad_length),
            Err(ModbusError::InvalidMbapLength(9))
        );
    }

    #[test]
    fn expected_lengths_cover_common_responses() {
        assert_eq!(
            expected_frame_len(Framing::Rtu, &[0x01, 0x03, 0x08]),
            Some(13)
        );
        assert_eq!(expected_frame_len(Framing::Rtu, &[0x01, 0x83]), Some(5));
        assert_eq!(expected_frame_len(Framing::Rtu, &[0x01, 0x06]), Some(8));
        assert_eq!(expected_frame_len(Framing::Rtu, &[0x01]), None);
        assert_eq!(expected_frame_len(Framing::Rtu, &[0x01, 0x2B]), None);
        assert_eq!(
            expected_frame_len(Framing::Mbap, &[0, 1, 0, 0, 0, 0x0E]),
            Some(20)
        );
        assert_eq!(expected_frame_len(Framing::Mbap, &[0, 1]), None);
        assert_eq!(expected_frame_len(Framing::Ascii, &[]), None);
    }

    #[test]
    fn t35_follows_baud_rate_rules() {
        assert_eq!(t35_micros(9_600), 4_010);
        assert_eq!(t35_micros(19_200), 2_005);
        assert_eq!(t35_micros(115_200), 1_750);
        assert_eq!(t35_micros(0), 1_750);
    }

    #[test]
    fn unified_encode_decode() {
        let pdu = [0x03, 0x00, 0x00, 0x00, 0x0A];
        for framing in [Framing::Rtu, Framing::Ascii, Framing::Mbap] {
            let frame = encode(framing, 1, Some(7), &pdu).expect("encode");
            let adu = decode(framing, &frame).expect("decode");
            assert_eq!(adu.unit_id, 1);
            assert_eq!(adu.pdu, pdu);
        }
    }
}
