//! Raw frame capture and best-effort decoding for the packet monitor.

use modbus_core::Framing;
use modbus_core::frame;

use crate::model::{DecodedFrameDto, FrameDto, FrameFieldDto, now_ms};

pub struct FrameInfo {
    pub direction: &'static str,
    pub summary: String,
    pub latency_ms: Option<f64>,
    pub status: &'static str,
}

pub fn build_frame(
    seq: u64,
    session_id: &str,
    raw: &[u8],
    framing: Framing,
    info: FrameInfo,
) -> FrameDto {
    FrameDto {
        seq,
        session_id: session_id.to_owned(),
        timestamp: now_ms(),
        direction: info.direction.to_owned(),
        raw: raw.to_vec(),
        decoded: decode_frame(framing, raw),
        summary: info.summary,
        latency_ms: info.latency_ms,
        status: info.status.to_owned(),
    }
}

fn decode_frame(framing: Framing, raw: &[u8]) -> DecodedFrameDto {
    match framing {
        Framing::Mbap => decode_mbap(raw),
        Framing::Rtu => decode_rtu(raw),
        Framing::Ascii => decode_ascii(raw),
    }
}

fn decode_mbap(raw: &[u8]) -> DecodedFrameDto {
    let unit_id = raw.get(6).copied().unwrap_or(0);
    let function_code = raw.get(7).copied().unwrap_or(0);
    let data = if raw.len() > 8 { &raw[8..] } else { &[][..] };
    let payload = payload_field(data);
    let fields = vec![
        FrameFieldDto {
            label: "事务 ID".to_owned(),
            value: format!(
                "0x{:02X}{:02X}",
                raw.first().copied().unwrap_or(0),
                raw.get(1).copied().unwrap_or(0)
            ),
            offset: 0,
            length: 2,
            tone: "header".to_owned(),
        },
        FrameFieldDto {
            label: "协议 ID".to_owned(),
            value: format!(
                "0x{:02X}{:02X}",
                raw.get(2).copied().unwrap_or(0),
                raw.get(3).copied().unwrap_or(0)
            ),
            offset: 2,
            length: 2,
            tone: "header".to_owned(),
        },
        FrameFieldDto {
            label: "长度".to_owned(),
            value: format!(
                "{} B",
                u16::from_be_bytes([
                    raw.get(4).copied().unwrap_or(0),
                    raw.get(5).copied().unwrap_or(0)
                ])
            ),
            offset: 4,
            length: 2,
            tone: "header".to_owned(),
        },
        FrameFieldDto {
            label: "从站 ID".to_owned(),
            value: unit_id.to_string(),
            offset: 6,
            length: 1,
            tone: "unit".to_owned(),
        },
        FrameFieldDto {
            label: "功能码".to_owned(),
            value: format!("0x{function_code:02X}"),
            offset: 7,
            length: 1,
            tone: "function".to_owned(),
        },
        FrameFieldDto {
            label: payload.0,
            value: payload.1,
            offset: 8,
            length: data.len(),
            tone: "data".to_owned(),
        },
    ];
    DecodedFrameDto {
        slave_id: unit_id,
        function_code,
        ascii: None,
        fields,
    }
}

fn decode_rtu(raw: &[u8]) -> DecodedFrameDto {
    let unit_id = raw.first().copied().unwrap_or(0);
    let function_code = raw.get(1).copied().unwrap_or(0);
    let data = if raw.len() > 4 {
        &raw[2..raw.len() - 2]
    } else {
        &[][..]
    };
    let payload = payload_field(data);
    let crc = if raw.len() >= 2 {
        format!("0x{:02X}{:02X}", raw[raw.len() - 2], raw[raw.len() - 1])
    } else {
        "—".to_owned()
    };
    let fields = vec![
        FrameFieldDto {
            label: "从站 ID".to_owned(),
            value: unit_id.to_string(),
            offset: 0,
            length: 1,
            tone: "unit".to_owned(),
        },
        FrameFieldDto {
            label: "功能码".to_owned(),
            value: format!("0x{function_code:02X}"),
            offset: 1,
            length: 1,
            tone: "function".to_owned(),
        },
        FrameFieldDto {
            label: payload.0,
            value: payload.1,
            offset: 2,
            length: data.len(),
            tone: "data".to_owned(),
        },
        FrameFieldDto {
            label: "CRC16".to_owned(),
            value: crc,
            offset: raw.len().saturating_sub(2),
            length: 2,
            tone: "checksum".to_owned(),
        },
    ];
    DecodedFrameDto {
        slave_id: unit_id,
        function_code,
        ascii: None,
        fields,
    }
}

fn decode_ascii(raw: &[u8]) -> DecodedFrameDto {
    let ascii = String::from_utf8_lossy(raw).into_owned();
    let mut unit_id = 0;
    let mut function_code = 0;
    let mut pdu_len = 0;
    if let Ok(adu) = frame::decode(Framing::Ascii, raw) {
        unit_id = adu.unit_id;
        function_code = adu.pdu.first().copied().unwrap_or(0);
        pdu_len = adu.pdu.len();
    }
    let fields = vec![
        FrameFieldDto {
            label: "ASCII 帧".to_owned(),
            value: ascii.trim().to_owned(),
            offset: 0,
            length: raw.len(),
            tone: "data".to_owned(),
        },
        FrameFieldDto {
            label: "从站 ID".to_owned(),
            value: unit_id.to_string(),
            offset: 0,
            length: 0,
            tone: "unit".to_owned(),
        },
        FrameFieldDto {
            label: "功能码".to_owned(),
            value: format!("0x{function_code:02X}"),
            offset: 0,
            length: 0,
            tone: "function".to_owned(),
        },
        FrameFieldDto {
            label: "PDU 长度".to_owned(),
            value: format!("{pdu_len} B"),
            offset: 0,
            length: 0,
            tone: "header".to_owned(),
        },
    ];
    DecodedFrameDto {
        slave_id: unit_id,
        function_code,
        ascii: Some(ascii),
        fields,
    }
}

fn payload_field(data: &[u8]) -> (String, String) {
    if data.is_empty() {
        return ("数据".to_owned(), "—".to_owned());
    }
    let byte_count = data[0] as usize;
    if byte_count + 1 == data.len() {
        ("字节数".to_owned(), format!("{byte_count} B"))
    } else {
        ("数据".to_owned(), format!("{} B", data.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_mbap_frame_fields() {
        let raw = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x01, 0x03, 0x00, 0x00, 0x00, 0x10,
        ];
        let decoded = decode_frame(Framing::Mbap, &raw);
        assert_eq!(decoded.slave_id, 1);
        assert_eq!(decoded.function_code, 3);
        assert_eq!(decoded.fields[0].label, "事务 ID");
        assert_eq!(decoded.fields[5].length, 4);
    }

    #[test]
    fn decodes_rtu_crc_field() {
        let raw = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xC5, 0xCD];
        let decoded = decode_frame(Framing::Rtu, &raw);
        assert_eq!(decoded.fields.last().expect("crc").value, "0xC5CD");
    }

    #[test]
    fn decodes_ascii_frames() {
        let raw = b":01030000000AF2\r\n";
        let decoded = decode_frame(Framing::Ascii, raw);
        assert_eq!(decoded.slave_id, 1);
        assert_eq!(decoded.function_code, 3);
        assert!(decoded.ascii.is_some());
    }

    #[test]
    fn tolerates_partial_frames() {
        let decoded = decode_frame(Framing::Rtu, &[0x01]);
        assert_eq!(decoded.slave_id, 1);
        assert_eq!(decoded.function_code, 0);
    }
}
