//! In-process Modbus slave simulator.
//!
//! The engine is transport agnostic: feed it a request PDU and a unit id, and
//! it returns the response PDU (or `None` when the frame must be ignored).
//! Missing register/coil addresses read as zero/false, which keeps the
//! simulator usable without pre-populating the entire 16-bit address space.

use std::collections::BTreeMap;

use crate::error::ModbusError;
use crate::exception::ExceptionCode;
use crate::pdu::{PduRequest, PduResponse};

/// State of one simulated slave unit.
#[derive(Debug, Clone)]
pub struct SlaveUnit {
    pub unit_id: u8,
    pub coils: BTreeMap<u16, bool>,
    pub discrete_inputs: BTreeMap<u16, bool>,
    pub input_registers: BTreeMap<u16, u16>,
    pub holding_registers: BTreeMap<u16, u16>,
    pub write_enabled: bool,
}

impl SlaveUnit {
    /// Create a writable unit with empty maps.
    pub fn new(unit_id: u8) -> Self {
        Self {
            unit_id,
            coils: BTreeMap::new(),
            discrete_inputs: BTreeMap::new(),
            input_registers: BTreeMap::new(),
            holding_registers: BTreeMap::new(),
            write_enabled: true,
        }
    }

    /// Preload holding registers.
    pub fn with_holding_registers(mut self, values: impl IntoIterator<Item = (u16, u16)>) -> Self {
        self.holding_registers.extend(values);
        self
    }

    /// Preload coils.
    pub fn with_coils(mut self, values: impl IntoIterator<Item = (u16, bool)>) -> Self {
        self.coils.extend(values);
        self
    }

    /// Read a holding register, defaulting to zero.
    pub fn holding(&self, address: u16) -> u16 {
        self.holding_registers.get(&address).copied().unwrap_or(0)
    }

    /// Read an input register, defaulting to zero.
    pub fn input(&self, address: u16) -> u16 {
        self.input_registers.get(&address).copied().unwrap_or(0)
    }

    /// Read a coil, defaulting to false.
    pub fn coil(&self, address: u16) -> bool {
        self.coils.get(&address).copied().unwrap_or(false)
    }

    /// Set a holding register.
    pub fn set_holding(&mut self, address: u16, value: u16) {
        self.holding_registers.insert(address, value);
    }

    /// Set an input register.
    pub fn set_input(&mut self, address: u16, value: u16) {
        self.input_registers.insert(address, value);
    }

    /// Set a coil.
    pub fn set_coil(&mut self, address: u16, value: bool) {
        self.coils.insert(address, value);
    }
}

/// A collection of simulated slave units addressed by unit id.
#[derive(Debug, Clone, Default)]
pub struct Simulator {
    units: BTreeMap<u8, SlaveUnit>,
}

impl Simulator {
    /// Build a simulator from slave units.
    pub fn new(units: impl IntoIterator<Item = SlaveUnit>) -> Self {
        let mut simulator = Self::default();
        for unit in units {
            simulator.units.insert(unit.unit_id, unit);
        }
        simulator
    }

    /// Iterate over configured units.
    pub fn units(&self) -> impl Iterator<Item = &SlaveUnit> {
        self.units.values()
    }

    /// Look up a unit.
    pub fn unit(&self, unit_id: u8) -> Option<&SlaveUnit> {
        self.units.get(&unit_id)
    }

    /// Mutably look up a unit.
    pub fn unit_mut(&mut self, unit_id: u8) -> Option<&mut SlaveUnit> {
        self.units.get_mut(&unit_id)
    }

    /// Insert or replace a unit.
    pub fn insert_unit(&mut self, unit: SlaveUnit) -> Option<SlaveUnit> {
        self.units.insert(unit.unit_id, unit)
    }

    /// Handle a request PDU for a unit.
    ///
    /// Returns the response PDU, or `None` for unknown units, broadcast writes
    /// that must not be answered, and malformed frames that cannot be routed.
    pub fn handle_request(&mut self, unit_id: u8, pdu: &[u8]) -> Option<Vec<u8>> {
        if pdu.is_empty() {
            return None;
        }

        let request = match PduRequest::decode(pdu) {
            Ok(request) => request,
            Err(error) => {
                if unit_id == 0 || !self.units.contains_key(&unit_id) {
                    return None;
                }
                let code = match error {
                    ModbusError::UnsupportedFunction(_) => ExceptionCode::IllegalFunction,
                    ModbusError::AddressRange => ExceptionCode::IllegalDataAddress,
                    _ => ExceptionCode::IllegalDataValue,
                };
                let response = PduResponse::Exception {
                    function_code: pdu[0] & 0x7F,
                    code,
                };
                return response.encode().ok();
            }
        };

        // Unit id 0 is a broadcast: apply writes to every unit, never answer.
        if unit_id == 0 {
            if request.is_write() {
                for unit in self.units.values_mut() {
                    let _ = apply_write(unit, &request);
                }
            }
            return None;
        }

        let unit = self.units.get_mut(&unit_id)?;
        match dispatch(unit, &request) {
            Ok(response) => response.encode().ok(),
            Err(code) => PduResponse::Exception {
                function_code: request.function_code(),
                code,
            }
            .encode()
            .ok(),
        }
    }
}

fn dispatch(unit: &mut SlaveUnit, request: &PduRequest) -> Result<PduResponse, ExceptionCode> {
    match request {
        PduRequest::ReadCoils { address, quantity } => {
            validate_range(*address, *quantity)?;
            let values = (0..*quantity)
                .map(|offset| unit.coil(address.wrapping_add(offset)))
                .collect();
            Ok(PduResponse::ReadBits {
                function_code: 0x01,
                values,
            })
        }
        PduRequest::ReadDiscreteInputs { address, quantity } => {
            validate_range(*address, *quantity)?;
            let values = (0..*quantity)
                .map(|offset| {
                    unit.discrete_inputs
                        .get(&address.wrapping_add(offset))
                        .copied()
                        .unwrap_or(false)
                })
                .collect();
            Ok(PduResponse::ReadBits {
                function_code: 0x02,
                values,
            })
        }
        PduRequest::ReadHoldingRegisters { address, quantity } => {
            validate_range(*address, *quantity)?;
            let values = (0..*quantity)
                .map(|offset| unit.holding(address.wrapping_add(offset)))
                .collect();
            Ok(PduResponse::ReadRegisters {
                function_code: 0x03,
                values,
            })
        }
        PduRequest::ReadInputRegisters { address, quantity } => {
            validate_range(*address, *quantity)?;
            let values = (0..*quantity)
                .map(|offset| unit.input(address.wrapping_add(offset)))
                .collect();
            Ok(PduResponse::ReadRegisters {
                function_code: 0x04,
                values,
            })
        }
        PduRequest::WriteSingleCoil { address, value } => {
            require_writable(unit)?;
            validate_range(*address, 1)?;
            unit.set_coil(*address, *value);
            Ok(PduResponse::WriteSingleAck {
                function_code: 0x05,
                address: *address,
                value: if *value { 0xFF00 } else { 0x0000 },
            })
        }
        PduRequest::WriteSingleRegister { address, value } => {
            require_writable(unit)?;
            validate_range(*address, 1)?;
            unit.set_holding(*address, *value);
            Ok(PduResponse::WriteSingleAck {
                function_code: 0x06,
                address: *address,
                value: *value,
            })
        }
        PduRequest::WriteMultipleCoils { address, values } => {
            require_writable(unit)?;
            validate_range(*address, values.len() as u16)?;
            for (offset, value) in values.iter().enumerate() {
                unit.set_coil(address.wrapping_add(offset as u16), *value);
            }
            Ok(PduResponse::WriteMultipleAck {
                function_code: 0x0F,
                address: *address,
                quantity: values.len() as u16,
            })
        }
        PduRequest::WriteMultipleRegisters { address, values } => {
            require_writable(unit)?;
            validate_range(*address, values.len() as u16)?;
            for (offset, value) in values.iter().enumerate() {
                unit.set_holding(address.wrapping_add(offset as u16), *value);
            }
            Ok(PduResponse::WriteMultipleAck {
                function_code: 0x10,
                address: *address,
                quantity: values.len() as u16,
            })
        }
    }
}

fn apply_write(unit: &mut SlaveUnit, request: &PduRequest) -> Result<(), ExceptionCode> {
    dispatch(unit, request).map(|_| ())
}

fn require_writable(unit: &SlaveUnit) -> Result<(), ExceptionCode> {
    if unit.write_enabled {
        Ok(())
    } else {
        Err(ExceptionCode::IllegalDataAddress)
    }
}

fn validate_range(address: u16, quantity: u16) -> Result<(), ExceptionCode> {
    if quantity == 0 || u32::from(address) + u32::from(quantity) > 65_536 {
        Err(ExceptionCode::IllegalDataAddress)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulator() -> Simulator {
        let unit = SlaveUnit::new(1)
            .with_holding_registers([(0, 0x1234), (1, 0xABCD), (10, 42)])
            .with_coils([(0, true), (2, true), (7, true)]);
        Simulator::new([unit])
    }

    fn request_bytes(request: &PduRequest) -> Vec<u8> {
        request.encode().expect("encode request")
    }

    fn response_for(simulator: &mut Simulator, request: &PduRequest) -> PduResponse {
        let bytes = request_bytes(request);
        let response = simulator.handle_request(1, &bytes).expect("response");
        PduResponse::decode_for_request(request, &response).expect("decode response")
    }

    #[test]
    fn reads_holding_and_input_registers() {
        let mut simulator = simulator();
        let request = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: 3,
        };
        assert_eq!(
            response_for(&mut simulator, &request),
            PduResponse::ReadRegisters {
                function_code: 3,
                values: vec![0x1234, 0xABCD, 0],
            }
        );

        simulator.unit_mut(1).expect("unit").set_input(5, 77);
        let request = PduRequest::ReadInputRegisters {
            address: 5,
            quantity: 1,
        };
        assert_eq!(
            response_for(&mut simulator, &request),
            PduResponse::ReadRegisters {
                function_code: 4,
                values: vec![77],
            }
        );
    }

    #[test]
    fn reads_coils_and_discrete_inputs() {
        let mut simulator = simulator();
        let request = PduRequest::ReadCoils {
            address: 0,
            quantity: 8,
        };
        let response = response_for(&mut simulator, &request);
        assert_eq!(
            response,
            PduResponse::ReadBits {
                function_code: 1,
                values: vec![true, false, true, false, false, false, false, true],
            }
        );

        let request = PduRequest::ReadDiscreteInputs {
            address: 0,
            quantity: 2,
        };
        assert_eq!(
            response_for(&mut simulator, &request),
            PduResponse::ReadBits {
                function_code: 2,
                values: vec![false, false],
            }
        );
    }

    #[test]
    fn writes_single_and_multiple_values() {
        let mut simulator = simulator();
        let write = PduRequest::WriteSingleRegister {
            address: 1,
            value: 0x0001,
        };
        assert_eq!(
            response_for(&mut simulator, &write),
            PduResponse::WriteSingleAck {
                function_code: 6,
                address: 1,
                value: 1,
            }
        );
        assert_eq!(simulator.unit(1).expect("unit").holding(1), 1);

        let write = PduRequest::WriteMultipleRegisters {
            address: 20,
            values: vec![7, 8, 9],
        };
        response_for(&mut simulator, &write);
        let unit = simulator.unit(1).expect("unit");
        assert_eq!(unit.holding(20), 7);
        assert_eq!(unit.holding(21), 8);
        assert_eq!(unit.holding(22), 9);

        let write = PduRequest::WriteSingleCoil {
            address: 1,
            value: true,
        };
        response_for(&mut simulator, &write);
        assert!(simulator.unit(1).expect("unit").coil(1));
    }

    #[test]
    fn returns_exception_for_unknown_unit_and_bad_address() {
        let mut simulator = simulator();
        assert_eq!(
            simulator.handle_request(9, &[0x03, 0x00, 0x00, 0x00, 0x01]),
            None
        );

        let request = PduRequest::ReadHoldingRegisters {
            address: 0,
            quantity: 1,
        };
        let mut bad = request_bytes(&request);
        bad[1] = 0xFF;
        bad[2] = 0xFF;
        bad[3] = 0x00;
        bad[4] = 0x02;
        let response = simulator.handle_request(1, &bad).expect("exception");
        let decoded =
            PduResponse::decode_for_request(&request, &response).expect("decode exception");
        assert_eq!(
            decoded,
            PduResponse::Exception {
                function_code: 3,
                code: ExceptionCode::IllegalDataAddress,
            }
        );
    }

    #[test]
    fn rejects_writes_when_disabled() {
        let mut simulator = simulator();
        simulator.unit_mut(1).expect("unit").write_enabled = false;
        let request = PduRequest::WriteSingleRegister {
            address: 0,
            value: 1,
        };
        let response = response_for(&mut simulator, &request);
        assert_eq!(
            response,
            PduResponse::Exception {
                function_code: 6,
                code: ExceptionCode::IllegalDataAddress,
            }
        );
        assert_eq!(simulator.unit(1).expect("unit").holding(0), 0x1234);
    }

    #[test]
    fn broadcast_writes_apply_without_response() {
        let mut simulator = simulator();
        let request = PduRequest::WriteSingleRegister {
            address: 3,
            value: 99,
        };
        let bytes = request_bytes(&request);
        assert_eq!(simulator.handle_request(0, &bytes), None);
        assert_eq!(simulator.unit(1).expect("unit").holding(3), 99);
    }

    #[test]
    fn unknown_requests_get_illegal_function() {
        let mut simulator = simulator();
        let response = simulator
            .handle_request(1, &[0x2B, 0x0E, 0x01, 0x00])
            .expect("response");
        assert_eq!(response, vec![0xAB, 0x01]);
    }
}
