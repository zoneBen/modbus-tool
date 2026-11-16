//! Modbus Protocol Data Unit (PDU) encoding and decoding.
//!
//! A PDU is the function code followed by its data. The unit id and framing
//! checksum belong to the transport layer and are handled by [`crate::frame`].

use crate::error::ModbusError;
use crate::exception::ExceptionCode;

/// Maximum PDU length defined by the Modbus specification.
pub const MAX_PDU_LEN: usize = 253;

pub const MAX_READ_BITS: u16 = 2000;
pub const MAX_READ_REGISTERS: u16 = 125;
pub const MAX_WRITE_BITS: u16 = 1968;
pub const MAX_WRITE_REGISTERS: u16 = 123;

/// A request sent from a master to a slave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PduRequest {
    ReadCoils { address: u16, quantity: u16 },
    ReadDiscreteInputs { address: u16, quantity: u16 },
    ReadHoldingRegisters { address: u16, quantity: u16 },
    ReadInputRegisters { address: u16, quantity: u16 },
    WriteSingleCoil { address: u16, value: bool },
    WriteSingleRegister { address: u16, value: u16 },
    WriteMultipleCoils { address: u16, values: Vec<bool> },
    WriteMultipleRegisters { address: u16, values: Vec<u16> },
}

/// A response sent from a slave to a master.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PduResponse {
    ReadBits {
        function_code: u8,
        values: Vec<bool>,
    },
    ReadRegisters {
        function_code: u8,
        values: Vec<u16>,
    },
    WriteSingleAck {
        function_code: u8,
        address: u16,
        value: u16,
    },
    WriteMultipleAck {
        function_code: u8,
        address: u16,
        quantity: u16,
    },
    Exception {
        function_code: u8,
        code: ExceptionCode,
    },
}

impl PduRequest {
    /// Function code of this request.
    pub const fn function_code(&self) -> u8 {
        match self {
            Self::ReadCoils { .. } => 0x01,
            Self::ReadDiscreteInputs { .. } => 0x02,
            Self::ReadHoldingRegisters { .. } => 0x03,
            Self::ReadInputRegisters { .. } => 0x04,
            Self::WriteSingleCoil { .. } => 0x05,
            Self::WriteSingleRegister { .. } => 0x06,
            Self::WriteMultipleCoils { .. } => 0x0F,
            Self::WriteMultipleRegisters { .. } => 0x10,
        }
    }

    /// Starting address of the request.
    pub const fn address(&self) -> u16 {
        match self {
            Self::ReadCoils { address, .. }
            | Self::ReadDiscreteInputs { address, .. }
            | Self::ReadHoldingRegisters { address, .. }
            | Self::ReadInputRegisters { address, .. }
            | Self::WriteSingleCoil { address, .. }
            | Self::WriteSingleRegister { address, .. }
            | Self::WriteMultipleCoils { address, .. }
            | Self::WriteMultipleRegisters { address, .. } => *address,
        }
    }

    /// Number of coils, discrete inputs or registers addressed.
    pub fn quantity(&self) -> u16 {
        match self {
            Self::ReadCoils { quantity, .. }
            | Self::ReadDiscreteInputs { quantity, .. }
            | Self::ReadHoldingRegisters { quantity, .. }
            | Self::ReadInputRegisters { quantity, .. } => *quantity,
            Self::WriteSingleCoil { .. } | Self::WriteSingleRegister { .. } => 1,
            Self::WriteMultipleCoils { values, .. } => values.len() as u16,
            Self::WriteMultipleRegisters { values, .. } => values.len() as u16,
        }
    }

    /// Whether this request modifies slave state.
    pub const fn is_write(&self) -> bool {
        matches!(
            self,
            Self::WriteSingleCoil { .. }
                | Self::WriteSingleRegister { .. }
                | Self::WriteMultipleCoils { .. }
                | Self::WriteMultipleRegisters { .. }
        )
    }

    /// Validate ranges and encode the PDU (without unit id or checksum).
    pub fn encode(&self) -> Result<Vec<u8>, ModbusError> {
        self.validate()?;
        let function = self.function_code();
        let mut output = Vec::with_capacity(MAX_PDU_LEN.min(8 + self.quantity() as usize * 2));
        output.push(function);

        match self {
            Self::ReadCoils { address, quantity }
            | Self::ReadDiscreteInputs { address, quantity }
            | Self::ReadHoldingRegisters { address, quantity }
            | Self::ReadInputRegisters { address, quantity } => {
                push_u16(&mut output, *address);
                push_u16(&mut output, *quantity);
            }
            Self::WriteSingleCoil { address, value } => {
                push_u16(&mut output, *address);
                push_u16(&mut output, if *value { 0xFF00 } else { 0x0000 });
            }
            Self::WriteSingleRegister { address, value } => {
                push_u16(&mut output, *address);
                push_u16(&mut output, *value);
            }
            Self::WriteMultipleCoils { address, values } => {
                let packed = pack_bits(values);
                push_u16(&mut output, *address);
                push_u16(&mut output, values.len() as u16);
                output.push(packed.len() as u8);
                output.extend_from_slice(&packed);
            }
            Self::WriteMultipleRegisters { address, values } => {
                push_u16(&mut output, *address);
                push_u16(&mut output, values.len() as u16);
                output.push((values.len() * 2) as u8);
                for value in values {
                    push_u16(&mut output, *value);
                }
            }
        }

        if output.len() > MAX_PDU_LEN {
            return Err(ModbusError::FrameTooLong(output.len()));
        }
        Ok(output)
    }

    /// Decode a request PDU.
    pub fn decode(bytes: &[u8]) -> Result<Self, ModbusError> {
        if bytes.is_empty() {
            return Err(ModbusError::FrameTooShort {
                expected: 1,
                actual: 0,
            });
        }
        if bytes.len() > MAX_PDU_LEN {
            return Err(ModbusError::FrameTooLong(bytes.len()));
        }

        let function = bytes[0];
        let request = match function {
            0x01..=0x04 => {
                require_len(bytes, 5)?;
                let address = read_u16(bytes, 1);
                let quantity = read_u16(bytes, 3);
                match function {
                    0x01 => Self::ReadCoils { address, quantity },
                    0x02 => Self::ReadDiscreteInputs { address, quantity },
                    0x03 => Self::ReadHoldingRegisters { address, quantity },
                    _ => Self::ReadInputRegisters { address, quantity },
                }
            }
            0x05 => {
                require_len(bytes, 5)?;
                let address = read_u16(bytes, 1);
                let raw = read_u16(bytes, 3);
                let value = match raw {
                    0xFF00 => true,
                    0x0000 => false,
                    _ => return Err(ModbusError::InvalidValue),
                };
                Self::WriteSingleCoil { address, value }
            }
            0x06 => {
                require_len(bytes, 5)?;
                Self::WriteSingleRegister {
                    address: read_u16(bytes, 1),
                    value: read_u16(bytes, 3),
                }
            }
            0x0F => {
                require_len_at_least(bytes, 6)?;
                let address = read_u16(bytes, 1);
                let quantity = read_u16(bytes, 3);
                if quantity == 0 || quantity > MAX_WRITE_BITS {
                    return Err(ModbusError::InvalidQuantity(quantity));
                }
                let byte_count = bytes[5] as usize;
                let expected = quantity.div_ceil(8) as usize;
                if byte_count != expected {
                    return Err(ModbusError::InvalidByteCount(byte_count));
                }
                require_len(bytes, 6 + byte_count)?;
                Self::WriteMultipleCoils {
                    address,
                    values: unpack_bits(&bytes[6..], quantity as usize),
                }
            }
            0x10 => {
                require_len_at_least(bytes, 6)?;
                let address = read_u16(bytes, 1);
                let quantity = read_u16(bytes, 3);
                if quantity == 0 || quantity > MAX_WRITE_REGISTERS {
                    return Err(ModbusError::InvalidQuantity(quantity));
                }
                let byte_count = bytes[5] as usize;
                if byte_count != quantity as usize * 2 {
                    return Err(ModbusError::InvalidByteCount(byte_count));
                }
                require_len(bytes, 6 + byte_count)?;
                let values = bytes[6..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u16::from_be_bytes(*pair))
                    .collect();
                Self::WriteMultipleRegisters { address, values }
            }
            other => return Err(ModbusError::UnsupportedFunction(other)),
        };

        request.validate()?;
        Ok(request)
    }

    /// Validate quantity limits and address overflow.
    pub fn validate(&self) -> Result<(), ModbusError> {
        let address = u32::from(self.address());
        let quantity = u32::from(self.quantity());
        if quantity == 0 || address + quantity > 65_536 {
            return Err(ModbusError::AddressRange);
        }
        match self {
            Self::ReadCoils { quantity, .. } | Self::ReadDiscreteInputs { quantity, .. } => {
                if *quantity > MAX_READ_BITS {
                    return Err(ModbusError::InvalidQuantity(*quantity));
                }
            }
            Self::ReadHoldingRegisters { quantity, .. }
            | Self::ReadInputRegisters { quantity, .. } => {
                if *quantity > MAX_READ_REGISTERS {
                    return Err(ModbusError::InvalidQuantity(*quantity));
                }
            }
            Self::WriteMultipleCoils { values, .. } => {
                if values.is_empty() || values.len() > MAX_WRITE_BITS as usize {
                    return Err(ModbusError::InvalidQuantity(values.len() as u16));
                }
            }
            Self::WriteMultipleRegisters { values, .. } => {
                if values.is_empty() || values.len() > MAX_WRITE_REGISTERS as usize {
                    return Err(ModbusError::InvalidQuantity(values.len() as u16));
                }
            }
            Self::WriteSingleCoil { .. } | Self::WriteSingleRegister { .. } => {}
        }
        Ok(())
    }
}

impl PduResponse {
    /// Function code carried by this response.
    pub const fn function_code(&self) -> u8 {
        match self {
            Self::ReadBits { function_code, .. }
            | Self::ReadRegisters { function_code, .. }
            | Self::WriteSingleAck { function_code, .. }
            | Self::WriteMultipleAck { function_code, .. }
            | Self::Exception { function_code, .. } => *function_code,
        }
    }

    /// Encode the response PDU.
    pub fn encode(&self) -> Result<Vec<u8>, ModbusError> {
        let mut output = Vec::new();
        match self {
            Self::ReadBits {
                function_code,
                values,
            } => {
                let packed = pack_bits(values);
                output.push(*function_code);
                output.push(packed.len() as u8);
                output.extend_from_slice(&packed);
            }
            Self::ReadRegisters {
                function_code,
                values,
            } => {
                output.push(*function_code);
                output.push((values.len() * 2) as u8);
                for value in values {
                    push_u16(&mut output, *value);
                }
            }
            Self::WriteSingleAck {
                function_code,
                address,
                value,
            } => {
                output.push(*function_code);
                push_u16(&mut output, *address);
                push_u16(&mut output, *value);
            }
            Self::WriteMultipleAck {
                function_code,
                address,
                quantity,
            } => {
                output.push(*function_code);
                push_u16(&mut output, *address);
                push_u16(&mut output, *quantity);
            }
            Self::Exception {
                function_code,
                code,
            } => {
                output.push(function_code | 0x80);
                output.push(code.as_u8());
            }
        }
        if output.len() > MAX_PDU_LEN {
            return Err(ModbusError::FrameTooLong(output.len()));
        }
        Ok(output)
    }

    /// Decode a response PDU using the request for expected shape validation.
    pub fn decode_for_request(request: &PduRequest, bytes: &[u8]) -> Result<Self, ModbusError> {
        if bytes.len() < 2 {
            return Err(ModbusError::FrameTooShort {
                expected: 2,
                actual: bytes.len(),
            });
        }
        if bytes.len() > MAX_PDU_LEN {
            return Err(ModbusError::FrameTooLong(bytes.len()));
        }

        let request_function = request.function_code();
        let function = bytes[0];
        if function == request_function | 0x80 {
            if bytes.len() != 2 {
                return Err(ModbusError::InvalidByteCount(bytes.len()));
            }
            let code = ExceptionCode::from_u8(bytes[1]).ok_or(ModbusError::InvalidValue)?;
            return Ok(Self::Exception {
                function_code: request_function,
                code,
            });
        }
        if function != request_function {
            return Err(ModbusError::FunctionMismatch {
                expected: request_function,
                actual: function,
            });
        }

        match request {
            PduRequest::ReadCoils { quantity, .. }
            | PduRequest::ReadDiscreteInputs { quantity, .. } => {
                require_len_at_least(bytes, 3)?;
                let byte_count = bytes[1] as usize;
                let expected = (*quantity).div_ceil(8) as usize;
                if byte_count != expected {
                    return Err(ModbusError::InvalidByteCount(byte_count));
                }
                require_len(bytes, 2 + byte_count)?;
                Ok(Self::ReadBits {
                    function_code: request_function,
                    values: unpack_bits(&bytes[2..], *quantity as usize),
                })
            }
            PduRequest::ReadHoldingRegisters { quantity, .. }
            | PduRequest::ReadInputRegisters { quantity, .. } => {
                require_len_at_least(bytes, 3)?;
                let byte_count = bytes[1] as usize;
                if byte_count != *quantity as usize * 2 {
                    return Err(ModbusError::InvalidByteCount(byte_count));
                }
                require_len(bytes, 2 + byte_count)?;
                let values = bytes[2..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u16::from_be_bytes(*pair))
                    .collect();
                Ok(Self::ReadRegisters {
                    function_code: request_function,
                    values,
                })
            }
            PduRequest::WriteSingleCoil { address, value } => {
                require_len(bytes, 5)?;
                let echoed_address = read_u16(bytes, 1);
                let echoed_value = read_u16(bytes, 3);
                let expected_value = if *value { 0xFF00 } else { 0x0000 };
                if echoed_address != *address || echoed_value != expected_value {
                    return Err(ModbusError::InvalidValue);
                }
                Ok(Self::WriteSingleAck {
                    function_code: request_function,
                    address: echoed_address,
                    value: echoed_value,
                })
            }
            PduRequest::WriteSingleRegister { address, value } => {
                require_len(bytes, 5)?;
                let echoed_address = read_u16(bytes, 1);
                let echoed_value = read_u16(bytes, 3);
                if echoed_address != *address || echoed_value != *value {
                    return Err(ModbusError::InvalidValue);
                }
                Ok(Self::WriteSingleAck {
                    function_code: request_function,
                    address: echoed_address,
                    value: echoed_value,
                })
            }
            PduRequest::WriteMultipleCoils { address, values } => {
                require_len(bytes, 5)?;
                let echoed_address = read_u16(bytes, 1);
                let echoed_quantity = read_u16(bytes, 3);
                if echoed_address != *address || echoed_quantity != values.len() as u16 {
                    return Err(ModbusError::InvalidValue);
                }
                Ok(Self::WriteMultipleAck {
                    function_code: request_function,
                    address: echoed_address,
                    quantity: echoed_quantity,
                })
            }
            PduRequest::WriteMultipleRegisters { address, values } => {
                require_len(bytes, 5)?;
                let echoed_address = read_u16(bytes, 1);
                let echoed_quantity = read_u16(bytes, 3);
                if echoed_address != *address || echoed_quantity != values.len() as u16 {
                    return Err(ModbusError::InvalidValue);
                }
                Ok(Self::WriteMultipleAck {
                    function_code: request_function,
                    address: echoed_address,
                    quantity: echoed_quantity,
                })
            }
        }
    }

    /// Whether this response reports a Modbus exception.
    pub const fn is_exception(&self) -> bool {
        matches!(self, Self::Exception { .. })
    }
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
}

fn require_len(bytes: &[u8], expected: usize) -> Result<(), ModbusError> {
    if bytes.len() < expected {
        return Err(ModbusError::FrameTooShort {
            expected,
            actual: bytes.len(),
        });
    }
    if bytes.len() > expected {
        return Err(ModbusError::FrameTooLong(bytes.len()));
    }
    Ok(())
}

fn require_len_at_least(bytes: &[u8], expected: usize) -> Result<(), ModbusError> {
    if bytes.len() < expected {
        return Err(ModbusError::FrameTooShort {
            expected,
            actual: bytes.len(),
        });
    }
    Ok(())
}

fn pack_bits(values: &[bool]) -> Vec<u8> {
    let mut packed = vec![0u8; values.len().div_ceil(8)];
    for (index, value) in values.iter().enumerate() {
        if *value {
            packed[index / 8] |= 1 << (index % 8);
        }
    }
    packed
}

fn unpack_bits(bytes: &[u8], quantity: usize) -> Vec<bool> {
    (0..quantity)
        .map(|index| {
            bytes
                .get(index / 8)
                .is_some_and(|byte| byte & (1 << (index % 8)) != 0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_read_holding() {
        let request = PduRequest::ReadHoldingRegisters {
            address: 0x006B,
            quantity: 3,
        };
        let encoded = request.encode().expect("encode");
        assert_eq!(encoded, vec![0x03, 0x00, 0x6B, 0x00, 0x03]);
        assert_eq!(PduRequest::decode(&encoded).expect("decode"), request);
    }

    #[test]
    fn decodes_read_response() {
        let request = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: 2,
        };
        let response =
            PduResponse::decode_for_request(&request, &[0x03, 0x04, 0x12, 0x34, 0xAB, 0xCD])
                .expect("decode");
        assert_eq!(
            response,
            PduResponse::ReadRegisters {
                function_code: 3,
                values: vec![0x1234, 0xABCD],
            }
        );
    }

    #[test]
    fn packs_and_unpacks_coils() {
        let values = vec![true, false, true, true, false, false, false, true, true];
        let request = PduRequest::WriteMultipleCoils {
            address: 0x0013,
            values: values.clone(),
        };
        let encoded = request.encode().expect("encode");
        assert_eq!(
            encoded,
            vec![0x0F, 0x00, 0x13, 0x00, 0x09, 0x02, 0x8D, 0x01]
        );
        assert_eq!(PduRequest::decode(&encoded).expect("decode"), request);

        let read = PduRequest::ReadCoils {
            address: 0,
            quantity: 9,
        };
        let response = PduResponse::ReadBits {
            function_code: 1,
            values: values.clone(),
        };
        let bytes = response.encode().expect("encode");
        assert_eq!(
            PduResponse::decode_for_request(&read, &bytes).expect("decode"),
            response
        );
    }

    #[test]
    fn write_single_coil_uses_ff00() {
        let request = PduRequest::WriteSingleCoil {
            address: 0x00AC,
            value: true,
        };
        let bytes = request.encode().expect("encode");
        assert_eq!(bytes, vec![0x05, 0x00, 0xAC, 0xFF, 0x00]);

        let response = PduResponse::WriteSingleAck {
            function_code: 5,
            address: 0x00AC,
            value: 0xFF00,
        };
        assert_eq!(
            PduResponse::decode_for_request(&request, &response.encode().expect("encode"))
                .expect("decode"),
            response
        );
    }

    #[test]
    fn write_multiple_registers_round_trips() {
        let request = PduRequest::WriteMultipleRegisters {
            address: 0x0001,
            values: vec![0x000A, 0x0102],
        };
        let bytes = request.encode().expect("encode");
        assert_eq!(
            bytes,
            vec![0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02]
        );
        assert_eq!(PduRequest::decode(&bytes).expect("decode"), request);
    }

    #[test]
    fn rejects_invalid_quantities_and_ranges() {
        let too_many = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: MAX_READ_REGISTERS + 1,
        };
        assert!(matches!(
            too_many.encode(),
            Err(ModbusError::InvalidQuantity(_))
        ));

        let overflow = PduRequest::ReadCoils {
            address: 0xFFFF,
            quantity: 2,
        };
        assert_eq!(overflow.encode(), Err(ModbusError::AddressRange));

        let bad_coil_value = [0x05, 0x00, 0x01, 0x12, 0x34];
        assert_eq!(
            PduRequest::decode(&bad_coil_value),
            Err(ModbusError::InvalidValue)
        );
    }

    #[test]
    fn decodes_exception_response() {
        let request = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: 1,
        };
        let response = PduResponse::decode_for_request(&request, &[0x83, 0x02]).expect("decode");
        assert_eq!(
            response,
            PduResponse::Exception {
                function_code: 3,
                code: ExceptionCode::IllegalDataAddress,
            }
        );
        assert!(response.is_exception());
        assert_eq!(response.encode().expect("encode"), vec![0x83, 0x02]);
    }

    #[test]
    fn rejects_wrong_function_and_echo() {
        let request = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: 1,
        };
        assert!(matches!(
            PduResponse::decode_for_request(&request, &[0x04, 0x02, 0x00, 0x01]),
            Err(ModbusError::FunctionMismatch { .. })
        ));

        let write = PduRequest::WriteSingleRegister {
            address: 1,
            value: 2,
        };
        assert_eq!(
            PduResponse::decode_for_request(&write, &[0x06, 0x00, 0x01, 0x00, 0x03]),
            Err(ModbusError::InvalidValue)
        );
    }

    #[test]
    fn rejects_zero_and_oversized_requests() {
        let request = PduRequest::WriteMultipleCoils {
            address: 0,
            values: Vec::new(),
        };
        assert!(matches!(
            request.encode(),
            Err(ModbusError::AddressRange | ModbusError::InvalidQuantity(0))
        ));

        let oversized = PduRequest::WriteMultipleRegisters {
            address: 0,
            values: vec![0; MAX_WRITE_REGISTERS as usize + 1],
        };
        assert!(matches!(
            oversized.encode(),
            Err(ModbusError::InvalidQuantity(_))
        ));
    }
}
