//! CRC16 (RTU) and LRC (ASCII) checksums.

/// Compute the Modbus RTU CRC-16 (polynomial 0xA001, initial value 0xFFFF).
pub fn crc16_modbus(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in bytes {
        crc ^= u16::from(*byte);
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/// CRC bytes as transmitted on the wire: low byte first, then high byte.
pub fn crc16_bytes(bytes: &[u8]) -> [u8; 2] {
    crc16_modbus(bytes).to_le_bytes()
}

/// Compute the Modbus ASCII LRC: two's complement of the 8-bit sum.
pub fn lrc(bytes: &[u8]) -> u8 {
    let sum = bytes
        .iter()
        .fold(0u8, |accumulator, byte| accumulator.wrapping_add(*byte));
    (!sum).wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_standard_vector() {
        let request = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        assert_eq!(crc16_modbus(&request), 0xCDC5);
        assert_eq!(crc16_bytes(&request), [0xC5, 0xCD]);
    }

    #[test]
    fn crc_of_empty_input_is_initial_value() {
        assert_eq!(crc16_modbus(&[]), 0xFFFF);
        assert_eq!(crc16_bytes(&[]), [0xFF, 0xFF]);
    }

    #[test]
    fn lrc_matches_standard_vector() {
        let request = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        assert_eq!(lrc(&request), 0xF2);
    }

    #[test]
    fn lrc_of_zero_sum_is_zero() {
        assert_eq!(lrc(&[]), 0x00);
        assert_eq!(lrc(&[0xFF, 0x01]), 0x00);
    }

    #[test]
    fn broadcast_detection() {
        assert!(crate::is_broadcast(0));
        assert!(!crate::is_broadcast(1));
        assert!(!crate::is_broadcast(247));
    }
}
