//! Error type shared by the PDU, framing and value layers.

use crate::exception::ExceptionCode;
use thiserror::Error;

/// Errors produced while building, parsing or interpreting Modbus data.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModbusError {
    #[error("frame too short: expected at least {expected} bytes, got {actual}")]
    FrameTooShort { expected: usize, actual: usize },

    #[error("frame too long: {0} bytes")]
    FrameTooLong(usize),

    #[error("unsupported function code 0x{0:02X}")]
    UnsupportedFunction(u8),

    #[error("function code mismatch: expected 0x{expected:02X}, got 0x{actual:02X}")]
    FunctionMismatch { expected: u8, actual: u8 },

    #[error("invalid quantity {0}")]
    InvalidQuantity(u16),

    #[error("invalid byte count {0}")]
    InvalidByteCount(usize),

    #[error("invalid value for this operation")]
    InvalidValue,

    #[error("address range overflows the 16-bit address space")]
    AddressRange,

    #[error("CRC mismatch")]
    CrcMismatch,

    #[error("LRC mismatch")]
    LrcMismatch,

    #[error("invalid ASCII frame")]
    InvalidAsciiFrame,

    #[error("MBAP protocol id must be 0, got {0}")]
    InvalidProtocolId(u16),

    #[error("invalid MBAP length {0}")]
    InvalidMbapLength(u16),

    #[error("not enough words: need {needed}, got {actual}")]
    NotEnoughWords { needed: usize, actual: usize },

    #[error("value out of range")]
    ValueError,

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("exception response: {0}")]
    Exception(ExceptionCode),
}
