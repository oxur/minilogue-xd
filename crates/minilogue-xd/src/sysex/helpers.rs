//! Blob parsing helpers for reading and writing multi-byte fields.
//!
//! These functions operate on raw byte slices at given offsets, providing
//! convenient access to 10-bit, big-endian u16, and little-endian u16 values
//! found in program and global data blobs.

/// Read a 10-bit value from a big-endian byte pair at `offset`.
///
/// The high byte is at `bytes[offset]`, the low byte at `bytes[offset + 1]`.
/// Only the lower 10 bits of the combined value are returned.
///
/// # Panics
///
/// Panics if `offset + 1 >= bytes.len()`.
pub fn read_10bit(bytes: &[u8], offset: usize) -> u16 {
    let hi = u16::from(bytes[offset]);
    let lo = u16::from(bytes[offset + 1]);
    ((hi << 8) | lo) & 0x03FF
}

/// Write a 10-bit value as a big-endian byte pair at `offset`.
///
/// Only the lower 10 bits of `value` are written. The upper 6 bits of the
/// high byte are cleared.
///
/// # Panics
///
/// Panics if `offset + 1 >= bytes.len()`.
pub fn write_10bit(bytes: &mut [u8], offset: usize, value: u16) {
    let v = value & 0x03FF;
    bytes[offset] = (v >> 8) as u8;
    bytes[offset + 1] = (v & 0xFF) as u8;
}

/// Read a big-endian `u16` at `offset`.
///
/// # Panics
///
/// Panics if `offset + 1 >= bytes.len()`.
pub fn read_u16_be(bytes: &[u8], offset: usize) -> u16 {
    u16::from(bytes[offset]) << 8 | u16::from(bytes[offset + 1])
}

/// Write a big-endian `u16` at `offset`.
///
/// # Panics
///
/// Panics if `offset + 1 >= bytes.len()`.
pub fn write_u16_be(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset] = (value >> 8) as u8;
    bytes[offset + 1] = (value & 0xFF) as u8;
}

/// Read a little-endian `u16` at `offset`.
///
/// Used for BPM, favorite entries, and other LE fields in the blobs.
///
/// # Panics
///
/// Panics if `offset + 1 >= bytes.len()`.
pub fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from(bytes[offset]) | (u16::from(bytes[offset + 1]) << 8)
}

/// Write a little-endian `u16` at `offset`.
///
/// # Panics
///
/// Panics if `offset + 1 >= bytes.len()`.
pub fn write_u16_le(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset] = (value & 0xFF) as u8;
    bytes[offset + 1] = (value >> 8) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // read_10bit / write_10bit
    // ---------------------------------------------------------------

    #[test]
    fn read_10bit_zero() {
        let data = [0x00, 0x00];
        assert_eq!(read_10bit(&data, 0), 0);
    }

    #[test]
    fn read_10bit_max() {
        // 0x03FF = 1023
        let data = [0x03, 0xFF];
        assert_eq!(read_10bit(&data, 0), 1023);
    }

    #[test]
    fn read_10bit_masks_upper_bits() {
        // Full u16 = 0xFFFF, but only lower 10 bits = 0x03FF = 1023
        let data = [0xFF, 0xFF];
        assert_eq!(read_10bit(&data, 0), 1023);
    }

    #[test]
    fn read_10bit_at_offset() {
        let data = [0xAA, 0x01, 0x00];
        assert_eq!(read_10bit(&data, 1), 256);
    }

    #[test]
    fn write_10bit_zero() {
        let mut data = [0xFF, 0xFF];
        write_10bit(&mut data, 0, 0);
        assert_eq!(data, [0x00, 0x00]);
    }

    #[test]
    fn write_10bit_max() {
        let mut data = [0x00, 0x00];
        write_10bit(&mut data, 0, 1023);
        assert_eq!(data, [0x03, 0xFF]);
    }

    #[test]
    fn write_10bit_masks_upper_bits() {
        let mut data = [0x00, 0x00];
        write_10bit(&mut data, 0, 0xFFFF);
        // Only lower 10 bits written.
        assert_eq!(data, [0x03, 0xFF]);
    }

    #[test]
    fn round_trip_10bit() {
        for v in 0..=1023u16 {
            let mut buf = [0u8; 2];
            write_10bit(&mut buf, 0, v);
            assert_eq!(read_10bit(&buf, 0), v, "round-trip failed for {v}");
        }
    }

    // ---------------------------------------------------------------
    // read_u16_be / write_u16_be
    // ---------------------------------------------------------------

    #[test]
    fn read_u16_be_zero() {
        assert_eq!(read_u16_be(&[0x00, 0x00], 0), 0);
    }

    #[test]
    fn read_u16_be_max() {
        assert_eq!(read_u16_be(&[0xFF, 0xFF], 0), 0xFFFF);
    }

    #[test]
    fn read_u16_be_byte_order() {
        assert_eq!(read_u16_be(&[0x01, 0x00], 0), 256);
        assert_eq!(read_u16_be(&[0x00, 0x01], 0), 1);
    }

    #[test]
    fn write_u16_be_round_trip() {
        for v in [0u16, 1, 255, 256, 1023, 0x7FFF, 0xFFFF] {
            let mut buf = [0u8; 2];
            write_u16_be(&mut buf, 0, v);
            assert_eq!(read_u16_be(&buf, 0), v);
        }
    }

    // ---------------------------------------------------------------
    // read_u16_le / write_u16_le
    // ---------------------------------------------------------------

    #[test]
    fn read_u16_le_zero() {
        assert_eq!(read_u16_le(&[0x00, 0x00], 0), 0);
    }

    #[test]
    fn read_u16_le_max() {
        assert_eq!(read_u16_le(&[0xFF, 0xFF], 0), 0xFFFF);
    }

    #[test]
    fn read_u16_le_byte_order() {
        assert_eq!(read_u16_le(&[0x00, 0x01], 0), 256);
        assert_eq!(read_u16_le(&[0x01, 0x00], 0), 1);
    }

    #[test]
    fn write_u16_le_round_trip() {
        for v in [0u16, 1, 255, 256, 1023, 0x7FFF, 0xFFFF] {
            let mut buf = [0u8; 2];
            write_u16_le(&mut buf, 0, v);
            assert_eq!(read_u16_le(&buf, 0), v);
        }
    }

    // ---------------------------------------------------------------
    // Mixed offsets
    // ---------------------------------------------------------------

    #[test]
    fn read_at_nonzero_offset() {
        let data = [0x00, 0x12, 0x34, 0x56];
        assert_eq!(read_u16_be(&data, 1), 0x1234);
        assert_eq!(read_u16_le(&data, 1), 0x3412);
        assert_eq!(read_u16_be(&data, 2), 0x3456);
        assert_eq!(read_u16_le(&data, 2), 0x5634);
    }

    #[test]
    fn write_at_nonzero_offset() {
        let mut data = [0x00; 4];
        write_u16_be(&mut data, 1, 0xABCD);
        assert_eq!(data, [0x00, 0xAB, 0xCD, 0x00]);

        let mut data = [0x00; 4];
        write_u16_le(&mut data, 1, 0xABCD);
        assert_eq!(data, [0x00, 0xCD, 0xAB, 0x00]);
    }

    // ---------------------------------------------------------------
    // BE vs LE distinctness
    // ---------------------------------------------------------------

    #[test]
    fn be_and_le_differ_for_asymmetric_values() {
        let mut be_buf = [0u8; 2];
        let mut le_buf = [0u8; 2];
        write_u16_be(&mut be_buf, 0, 0x0102);
        write_u16_le(&mut le_buf, 0, 0x0102);
        assert_ne!(be_buf, le_buf);
        assert_eq!(be_buf, [0x01, 0x02]);
        assert_eq!(le_buf, [0x02, 0x01]);
    }
}
