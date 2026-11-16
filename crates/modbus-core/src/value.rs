//! Register value interpretation and writing.
//!
//! Wire words are converted to a canonical big-endian byte stream using the
//! configured byte order (within each 16-bit word) and word order (between
//! words). The four familiar presets map directly onto these two dimensions:
//!
//! | Preset | Byte order | Word order |
//! |--------|------------|------------|
//! | ABCD   | AB         | high first |
//! | BADC   | BA         | high first |
//! | CDAB   | AB         | low first  |
//! | DCBA   | BA         | low first  |

use serde::{Deserialize, Serialize};

use crate::error::ModbusError;

/// Logical data type displayed for a register or field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Hex,
    Bin,
    Ascii,
}

impl DataType {
    /// Number of 16-bit registers occupied by one value.
    pub const fn word_span(self) -> usize {
        match self {
            Self::I32 | Self::U32 | Self::F32 => 2,
            Self::I64 | Self::U64 | Self::F64 => 4,
            Self::Bool
            | Self::I8
            | Self::U8
            | Self::I16
            | Self::U16
            | Self::Hex
            | Self::Bin
            | Self::Ascii => 1,
        }
    }
}

/// Byte selection for 8-bit values inside a 16-bit register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BytePos {
    Low,
    High,
}

/// Byte order inside each 16-bit word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ByteOrder {
    Ab,
    Ba,
}

/// Word order between registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordOrder {
    HighFirst,
    LowFirst,
}

/// UI preset for byte and word order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderPreset {
    Abcd,
    Dcba,
    Badc,
    Cdab,
}

impl OrderPreset {
    /// Decompose the preset into byte order and word order.
    pub const fn parts(self) -> (ByteOrder, WordOrder) {
        match self {
            Self::Abcd => (ByteOrder::Ab, WordOrder::HighFirst),
            Self::Badc => (ByteOrder::Ba, WordOrder::HighFirst),
            Self::Cdab => (ByteOrder::Ab, WordOrder::LowFirst),
            Self::Dcba => (ByteOrder::Ba, WordOrder::LowFirst),
        }
    }

    /// Compose a preset from byte and word order.
    pub const fn from_parts(byte_order: ByteOrder, word_order: WordOrder) -> Self {
        match (byte_order, word_order) {
            (ByteOrder::Ab, WordOrder::HighFirst) => Self::Abcd,
            (ByteOrder::Ba, WordOrder::HighFirst) => Self::Badc,
            (ByteOrder::Ab, WordOrder::LowFirst) => Self::Cdab,
            (ByteOrder::Ba, WordOrder::LowFirst) => Self::Dcba,
        }
    }
}

/// Complete parsing rule for a register group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueRule {
    pub data_type: DataType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_pos: Option<BytePos>,
    pub byte_order: ByteOrder,
    pub word_order: WordOrder,
}

impl ValueRule {
    /// Rule for a type with the ABCD preset and low-byte position.
    pub const fn new(data_type: DataType) -> Self {
        Self {
            data_type,
            byte_pos: Some(BytePos::Low),
            byte_order: ByteOrder::Ab,
            word_order: WordOrder::HighFirst,
        }
    }

    /// Rule using one of the four UI presets.
    pub const fn with_preset(data_type: DataType, preset: OrderPreset) -> Self {
        let (byte_order, word_order) = preset.parts();
        Self {
            data_type,
            byte_pos: Some(BytePos::Low),
            byte_order,
            word_order,
        }
    }

    /// Set the 8-bit byte position.
    pub const fn with_byte_pos(mut self, byte_pos: BytePos) -> Self {
        self.byte_pos = Some(byte_pos);
        self
    }

    /// Number of registers consumed by one value.
    pub const fn word_span(&self) -> usize {
        self.data_type.word_span()
    }
}

impl Default for ValueRule {
    fn default() -> Self {
        Self::new(DataType::U16)
    }
}

/// Result of decoding one value from wire words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedValue {
    pub words: Vec<u16>,
    pub formatted: String,
    pub numeric: Option<f64>,
}

/// Decode a value from wire-order register words.
pub fn decode_words(rule: &ValueRule, words: &[u16]) -> Result<DecodedValue, ModbusError> {
    let span = rule.word_span();
    if words.len() < span {
        return Err(ModbusError::NotEnoughWords {
            needed: span,
            actual: words.len(),
        });
    }
    let used = &words[..span];
    let bytes = logical_bytes(used, rule.byte_order, rule.word_order);

    let (formatted, numeric) = match rule.data_type {
        DataType::Bool => {
            let value = used[0] & 1 == 1;
            (
                (if value { "ON" } else { "OFF" }).to_owned(),
                Some(if value { 1.0 } else { 0.0 }),
            )
        }
        DataType::I8 => {
            let byte = select_byte(&bytes, rule.byte_pos);
            let value = i8::from_be_bytes([byte]);
            (value.to_string(), Some(f64::from(value)))
        }
        DataType::U8 => {
            let byte = select_byte(&bytes, rule.byte_pos);
            (byte.to_string(), Some(f64::from(byte)))
        }
        DataType::I16 => {
            let value = i16::from_be_bytes([bytes[0], bytes[1]]);
            (value.to_string(), Some(f64::from(value)))
        }
        DataType::U16 => {
            let value = u16::from_be_bytes([bytes[0], bytes[1]]);
            (value.to_string(), Some(f64::from(value)))
        }
        DataType::I32 => {
            let value = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            (value.to_string(), Some(f64::from(value)))
        }
        DataType::U32 => {
            let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            (value.to_string(), Some(f64::from(value)))
        }
        DataType::I64 => {
            let value = i64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            (value.to_string(), Some(value as f64))
        }
        DataType::U64 => {
            let value = u64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            (value.to_string(), Some(value as f64))
        }
        DataType::F32 => {
            let value = f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            (format_float(f64::from(value)), Some(f64::from(value)))
        }
        DataType::F64 => {
            let value = f64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]);
            (format_float(value), Some(value))
        }
        DataType::Hex => (format!("0x{:04X}", used[0]), None),
        DataType::Bin => (format!("{:016b}", used[0]), None),
        DataType::Ascii => {
            let text = used[0]
                .to_be_bytes()
                .iter()
                .map(|byte| ascii_char(*byte))
                .collect::<String>();
            (text, None)
        }
    };

    Ok(DecodedValue {
        words: used.to_vec(),
        formatted,
        numeric,
    })
}

/// Encode a textual value into wire-order register words.
pub fn encode_value(rule: &ValueRule, input: &str) -> Result<Vec<u16>, ModbusError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(ModbusError::InvalidInput("empty value".to_owned()));
    }

    match rule.data_type {
        DataType::Bool => {
            let value = parse_bool(text)?;
            Ok(vec![if value { 1 } else { 0 }])
        }
        DataType::I8 => {
            let value = parse_i64(text)?;
            let byte = i8::try_from(value).map_err(|_| ModbusError::ValueError)?;
            Ok(vec![pack_byte(byte as u8, rule)])
        }
        DataType::U8 => {
            let value = parse_u64(text)?;
            let byte = u8::try_from(value).map_err(|_| ModbusError::ValueError)?;
            Ok(vec![pack_byte(byte, rule)])
        }
        DataType::I16 => {
            let value = i16::try_from(parse_i64(text)?).map_err(|_| ModbusError::ValueError)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::U16 => {
            let value = u16::try_from(parse_u64(text)?).map_err(|_| ModbusError::ValueError)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::I32 => {
            let value = i32::try_from(parse_i64(text)?).map_err(|_| ModbusError::ValueError)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::U32 => {
            let value = u32::try_from(parse_u64(text)?).map_err(|_| ModbusError::ValueError)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::I64 => {
            let value = parse_i64(text)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::U64 => {
            let value = parse_u64(text)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::F32 => {
            let value = parse_f64(text)?;
            let narrowed = value as f32;
            if !narrowed.is_finite() {
                return Err(ModbusError::ValueError);
            }
            Ok(words_from_bytes(
                &narrowed.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::F64 => {
            let value = parse_f64(text)?;
            Ok(words_from_bytes(
                &value.to_be_bytes(),
                rule.byte_order,
                rule.word_order,
            ))
        }
        DataType::Hex => {
            let digits = text
                .strip_prefix("0x")
                .or_else(|| text.strip_prefix("0X"))
                .unwrap_or(text);
            if digits.is_empty() || digits.len() > 4 {
                return Err(ModbusError::InvalidInput(
                    "hex value must fit 16 bits".to_owned(),
                ));
            }
            let value = u16::from_str_radix(digits, 16)
                .map_err(|_| ModbusError::InvalidInput(text.to_owned()))?;
            Ok(vec![value])
        }
        DataType::Bin => {
            let digits = text.replace([' ', '_'], "");
            if digits.is_empty() || digits.len() > 16 {
                return Err(ModbusError::InvalidInput(
                    "binary value must fit 16 bits".to_owned(),
                ));
            }
            let value = u16::from_str_radix(&digits, 2)
                .map_err(|_| ModbusError::InvalidInput(text.to_owned()))?;
            Ok(vec![value])
        }
        DataType::Ascii => {
            let bytes = text.as_bytes();
            let mut words = Vec::with_capacity(bytes.len().div_ceil(2));
            for pair in bytes.chunks(2) {
                let high = pair[0];
                let low = pair.get(1).copied().unwrap_or(0);
                words.push(u16::from_be_bytes([high, low]));
            }
            Ok(words)
        }
    }
}

fn logical_bytes(words: &[u16], byte_order: ByteOrder, word_order: WordOrder) -> Vec<u8> {
    let mut ordered = words.to_vec();
    if word_order == WordOrder::LowFirst {
        ordered.reverse();
    }
    ordered
        .iter()
        .flat_map(|word| {
            let [high, low] = word.to_be_bytes();
            match byte_order {
                ByteOrder::Ab => [high, low],
                ByteOrder::Ba => [low, high],
            }
        })
        .collect()
}

fn words_from_bytes(bytes: &[u8], byte_order: ByteOrder, word_order: WordOrder) -> Vec<u16> {
    let mut words: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_be_bytes(*pair))
        .collect();
    if word_order == WordOrder::LowFirst {
        words.reverse();
    }
    if byte_order == ByteOrder::Ba {
        for word in &mut words {
            *word = word.swap_bytes();
        }
    }
    words
}

fn select_byte(bytes: &[u8], position: Option<BytePos>) -> u8 {
    match position.unwrap_or(BytePos::Low) {
        BytePos::High => bytes[0],
        BytePos::Low => bytes[1],
    }
}

fn pack_byte(byte: u8, rule: &ValueRule) -> u16 {
    let word = match rule.byte_pos.unwrap_or(BytePos::Low) {
        BytePos::High => u16::from(byte) << 8,
        BytePos::Low => u16::from(byte),
    };
    match rule.byte_order {
        ByteOrder::Ab => word,
        ByteOrder::Ba => word.swap_bytes(),
    }
}

fn parse_bool(input: &str) -> Result<bool, ModbusError> {
    match input.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        _ => Err(ModbusError::InvalidInput(input.to_owned())),
    }
}

fn cleaned_number(input: &str) -> String {
    input
        .chars()
        .filter(|character| *character != ',' && *character != '_' && !character.is_whitespace())
        .collect()
}

fn parse_i64(input: &str) -> Result<i64, ModbusError> {
    cleaned_number(input)
        .parse::<i64>()
        .map_err(|_| ModbusError::InvalidInput(input.to_owned()))
}

fn parse_u64(input: &str) -> Result<u64, ModbusError> {
    cleaned_number(input)
        .parse::<u64>()
        .map_err(|_| ModbusError::InvalidInput(input.to_owned()))
}

fn parse_f64(input: &str) -> Result<f64, ModbusError> {
    let value = cleaned_number(input)
        .parse::<f64>()
        .map_err(|_| ModbusError::InvalidInput(input.to_owned()))?;
    if value.is_nan() {
        return Err(ModbusError::ValueError);
    }
    Ok(value)
}

fn format_float(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "∞".to_owned()
        } else {
            "-∞".to_owned()
        };
    }
    let mut text = format!("{value:.3}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

fn ascii_char(byte: u8) -> char {
    if (0x20..=0x7E).contains(&byte) {
        char::from(byte)
    } else {
        '·'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn decodes_u16_and_i16() {
        let rule = ValueRule::new(DataType::U16);
        let value = decode_words(&rule, &[0x1234]).expect("decode");
        assert_eq!(value.formatted, "4660");
        assert_eq!(value.numeric, Some(4660.0));

        let signed = ValueRule::new(DataType::I16);
        let value = decode_words(&signed, &[0xFFFF]).expect("decode");
        assert_eq!(value.formatted, "-1");
    }

    #[test]
    fn handles_int8_byte_positions() {
        let high = ValueRule::new(DataType::U8).with_byte_pos(BytePos::High);
        assert_eq!(encode_value(&high, "171").expect("encode"), vec![0xAB00]);
        assert_eq!(
            decode_words(&high, &[0xAB00]).expect("decode").formatted,
            "171"
        );

        let low = ValueRule::new(DataType::U8).with_byte_pos(BytePos::Low);
        assert_eq!(encode_value(&low, "171").expect("encode"), vec![0x00AB]);
        assert_eq!(
            decode_words(&low, &[0x00AB]).expect("decode").formatted,
            "171"
        );
    }

    #[test]
    fn maps_all_four_orders_for_i32() {
        let value = 0x1234_5678_i32;
        let cases = [
            (OrderPreset::Abcd, vec![0x1234, 0x5678]),
            (OrderPreset::Cdab, vec![0x5678, 0x1234]),
            (OrderPreset::Badc, vec![0x3412, 0x7856]),
            (OrderPreset::Dcba, vec![0x7856, 0x3412]),
        ];
        for (preset, expected_words) in cases {
            let rule = ValueRule::with_preset(DataType::I32, preset);
            let words = encode_value(&rule, &value.to_string()).expect("encode");
            assert_eq!(words, expected_words, "preset {preset:?}");
            let decoded = decode_words(&rule, &words).expect("decode");
            assert_eq!(decoded.numeric, Some(f64::from(value)));
        }
    }

    #[test]
    fn decodes_floats_with_orders() {
        let value = 23.46_f32;
        for preset in [
            OrderPreset::Abcd,
            OrderPreset::Cdab,
            OrderPreset::Badc,
            OrderPreset::Dcba,
        ] {
            let rule = ValueRule::with_preset(DataType::F32, preset);
            let encoded = words_from_bytes(&value.to_be_bytes(), rule.byte_order, rule.word_order);
            let decoded = decode_words(&rule, &encoded).expect("decode");
            assert_eq!(decoded.words.len(), 2);
            assert!((decoded.numeric.unwrap_or_default() - f64::from(value)).abs() < 1e-4);
            assert!(!decoded.formatted.is_empty());
        }
    }

    #[test]
    fn encodes_and_decodes_hex_bin_ascii() {
        let hex = ValueRule::new(DataType::Hex);
        assert_eq!(encode_value(&hex, "0x1234").expect("encode"), vec![0x1234]);
        assert_eq!(
            decode_words(&hex, &[0x1234]).expect("decode").formatted,
            "0x1234"
        );

        let binary = ValueRule::new(DataType::Bin);
        assert_eq!(
            encode_value(&binary, "0001001000110100").expect("encode"),
            vec![0x1234]
        );
        assert_eq!(
            decode_words(&binary, &[0x1234]).expect("decode").formatted,
            "0001001000110100"
        );

        let ascii = ValueRule::new(DataType::Ascii);
        assert_eq!(encode_value(&ascii, "AB").expect("encode"), vec![0x4142]);
        assert_eq!(
            decode_words(&ascii, &[0x4142]).expect("decode").formatted,
            "AB"
        );
    }

    #[test]
    fn encodes_bool_aliases() {
        let rule = ValueRule::new(DataType::Bool);
        assert_eq!(encode_value(&rule, "on").expect("encode"), vec![1]);
        assert_eq!(encode_value(&rule, "off").expect("encode"), vec![0]);
        assert_eq!(decode_words(&rule, &[1]).expect("decode").formatted, "ON");
        assert!(encode_value(&rule, "maybe").is_err());
    }

    #[test]
    fn rejects_out_of_range_values() {
        assert!(encode_value(&ValueRule::new(DataType::U16), "65536").is_err());
        assert!(encode_value(&ValueRule::new(DataType::I16), "-32769").is_err());
        assert!(encode_value(&ValueRule::new(DataType::U8), "256").is_err());
        assert!(encode_value(&ValueRule::new(DataType::Hex), "0x1FFFF").is_err());
        assert!(encode_value(&ValueRule::new(DataType::Ascii), "").is_err());
    }

    #[test]
    fn rejects_short_word_slices() {
        assert_eq!(
            decode_words(&ValueRule::new(DataType::F32), &[0x1234]),
            Err(ModbusError::NotEnoughWords {
                needed: 2,
                actual: 1,
            })
        );
    }

    proptest! {
        #[test]
        fn u16_round_trip(value in any::<u16>()) {
            for preset in [OrderPreset::Abcd, OrderPreset::Cdab, OrderPreset::Badc, OrderPreset::Dcba] {
                let rule = ValueRule::with_preset(DataType::U16, preset);
                let words = encode_value(&rule, &value.to_string()).expect("encode");
                let decoded = decode_words(&rule, &words).expect("decode");
                prop_assert_eq!(decoded.numeric, Some(f64::from(value)));
            }
        }

        #[test]
        fn i32_round_trip(value in any::<i32>()) {
            for preset in [OrderPreset::Abcd, OrderPreset::Cdab, OrderPreset::Badc, OrderPreset::Dcba] {
                let rule = ValueRule::with_preset(DataType::I32, preset);
                let words = encode_value(&rule, &value.to_string()).expect("encode");
                let decoded = decode_words(&rule, &words).expect("decode");
                prop_assert_eq!(decoded.numeric, Some(f64::from(value)));
            }
        }
    }
}
