//! Pure Modbus protocol primitives shared by the Tauri application and tests.
//!
//! This crate deliberately has no tokio, serialport, Tauri or filesystem
//! dependencies. Transports and session orchestration live in `src-tauri`.

pub mod checksum;
pub mod error;
pub mod exception;
pub mod frame;
pub mod pdu;
pub mod simulator;
pub mod value;

pub use error::ModbusError;
pub use exception::ExceptionCode;
pub use frame::{Adu, Framing};
pub use pdu::{PduRequest, PduResponse};
pub use value::{ByteOrder, BytePos, DataType, DecodedValue, OrderPreset, ValueRule, WordOrder};

/// Human readable summary of the protocol variants supported by the tool.
pub const PROTOCOL_SUMMARY: &str = "Modbus RTU / ASCII / MBAP";

/// Function codes planned for the first release.
pub const SUPPORTED_FUNCTION_CODES: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x0F, 0x10];

/// Broadcast unit id (writes only, no response expected).
pub fn is_broadcast(unit_id: u8) -> bool {
    unit_id == 0
}
