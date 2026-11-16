//! Modbus exception codes returned by slave devices.

use std::fmt;

/// Exception code carried in an exception response (`function_code | 0x80`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionCode {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    ServerDeviceFailure = 0x04,
    Acknowledge = 0x05,
    ServerDeviceBusy = 0x06,
    MemoryParityError = 0x08,
    GatewayPathUnavailable = 0x0A,
    GatewayTargetFailed = 0x0B,
}

impl ExceptionCode {
    /// Parse an exception code, returning `None` for values outside the spec.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::IllegalFunction),
            0x02 => Some(Self::IllegalDataAddress),
            0x03 => Some(Self::IllegalDataValue),
            0x04 => Some(Self::ServerDeviceFailure),
            0x05 => Some(Self::Acknowledge),
            0x06 => Some(Self::ServerDeviceBusy),
            0x08 => Some(Self::MemoryParityError),
            0x0A => Some(Self::GatewayPathUnavailable),
            0x0B => Some(Self::GatewayTargetFailed),
            _ => None,
        }
    }

    /// Wire representation of the code.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Short human readable description.
    pub const fn description(self) -> &'static str {
        match self {
            Self::IllegalFunction => "illegal function",
            Self::IllegalDataAddress => "illegal data address",
            Self::IllegalDataValue => "illegal data value",
            Self::ServerDeviceFailure => "server device failure",
            Self::Acknowledge => "acknowledge",
            Self::ServerDeviceBusy => "server device busy",
            Self::MemoryParityError => "memory parity error",
            Self::GatewayPathUnavailable => "gateway path unavailable",
            Self::GatewayTargetFailed => "gateway target failed",
        }
    }
}

impl fmt::Display for ExceptionCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:02X} ({})", self.as_u8(), self.description())
    }
}

impl From<ExceptionCode> for u8 {
    fn from(code: ExceptionCode) -> Self {
        code.as_u8()
    }
}

impl TryFrom<u8> for ExceptionCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_u8(value).ok_or(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_known_codes() {
        for code in [
            ExceptionCode::IllegalFunction,
            ExceptionCode::IllegalDataAddress,
            ExceptionCode::IllegalDataValue,
            ExceptionCode::ServerDeviceFailure,
            ExceptionCode::Acknowledge,
            ExceptionCode::ServerDeviceBusy,
            ExceptionCode::MemoryParityError,
            ExceptionCode::GatewayPathUnavailable,
            ExceptionCode::GatewayTargetFailed,
        ] {
            assert_eq!(ExceptionCode::from_u8(code.as_u8()), Some(code));
            assert_eq!(ExceptionCode::try_from(code.as_u8()), Ok(code));
        }
    }

    #[test]
    fn rejects_unknown_codes() {
        assert_eq!(ExceptionCode::from_u8(0x00), None);
        assert_eq!(ExceptionCode::try_from(0x07), Err(0x07));
    }
}
