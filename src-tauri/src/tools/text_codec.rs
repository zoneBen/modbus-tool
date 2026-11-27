use encoding_rs::Encoding;

use super::model::{TextConvertInput, TextConvertResultDto, ToolErrorDto, ToolResult};

pub fn convert(input: TextConvertInput) -> ToolResult<TextConvertResultDto> {
    let from = normalize_label(&input.from_encoding);
    let to = normalize_label(&input.to_encoding);

    let bytes = if input.input_mode.eq_ignore_ascii_case("hex") {
        let cleaned: String = input
            .text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        hex::decode(&cleaned).map_err(|error| ToolErrorDto::new("invalidHex", error.to_string()))?
    } else {
        input.text.as_bytes().to_vec()
    };

    let (decoded, lossy) = decode(&bytes, &from)?;
    let encoded = encode(&decoded, &to)?;
    Ok(TextConvertResultDto {
        text: decoded,
        hex: hex::encode_upper(&encoded),
        bytes: encoded.len(),
        encoding: to,
        lossy,
    })
}

fn normalize_label(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn decode(bytes: &[u8], encoding: &str) -> ToolResult<(String, bool)> {
    match encoding {
        "utf-16" | "utf-16le" | "utf-16be" => {
            let (body, little_endian) = if bytes.starts_with(&[0xff, 0xfe]) {
                (&bytes[2..], true)
            } else if bytes.starts_with(&[0xfe, 0xff]) {
                (&bytes[2..], false)
            } else {
                (bytes, encoding != "utf-16be")
            };
            let units: Vec<u16> = (0..body.len() / 2)
                .map(|index| {
                    let pair = [body[index * 2], body[index * 2 + 1]];
                    if little_endian {
                        u16::from_le_bytes(pair)
                    } else {
                        u16::from_be_bytes(pair)
                    }
                })
                .collect();
            let text = String::from_utf16_lossy(&units);
            Ok((text, units.contains(&0xfffd)))
        }
        _ => {
            let codec = Encoding::for_label(encoding.as_bytes()).ok_or_else(|| {
                ToolErrorDto::new("unknownEncoding", format!("Unknown encoding: {encoding}"))
            })?;
            let (text, _, had_errors) = codec.decode(bytes);
            Ok((text.into_owned(), had_errors))
        }
    }
}

fn encode(text: &str, encoding: &str) -> ToolResult<Vec<u8>> {
    match encoding {
        "utf-16" | "utf-16le" => {
            let mut bytes = vec![0xff, 0xfe];
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            Ok(bytes)
        }
        "utf-16be" => {
            let mut bytes = vec![0xfe, 0xff];
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            Ok(bytes)
        }
        _ => {
            let codec = Encoding::for_label(encoding.as_bytes()).ok_or_else(|| {
                ToolErrorDto::new("unknownEncoding", format!("Unknown encoding: {encoding}"))
            })?;
            let (bytes, _, had_errors) = codec.encode(text);
            if had_errors {
                return Err(ToolErrorDto::new(
                    "encodingLossy",
                    format!("Some characters cannot be represented in {encoding}"),
                ));
            }
            Ok(bytes.into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str, from: &str, to: &str, mode: &str) -> TextConvertInput {
        TextConvertInput {
            text: text.to_owned(),
            input_mode: mode.to_owned(),
            from_encoding: from.to_owned(),
            to_encoding: to.to_owned(),
        }
    }

    #[test]
    fn converts_gbk_hex_to_utf8_text() {
        // "中文" in GBK
        let result = convert(input("D6 D0 CE C4", "gbk", "utf-8", "hex")).expect("convert");
        assert_eq!(result.text, "中文");
        assert_eq!(result.hex, "E4B8ADE69687");
    }

    #[test]
    fn encodes_utf8_to_utf16le_with_bom() {
        let result = convert(input("A", "utf-8", "utf-16le", "text")).expect("convert");
        assert_eq!(result.hex, "FFFE4100");
    }

    #[test]
    fn decodes_utf16be_hex() {
        let result = convert(input("FEFF0041", "utf-16be", "utf-8", "hex")).expect("convert");
        assert_eq!(result.text, "A");
        assert_eq!(result.hex, "41");
    }

    #[test]
    fn rejects_unknown_encoding() {
        assert!(convert(input("A", "utf-8", "klingon", "text")).is_err());
    }
}
