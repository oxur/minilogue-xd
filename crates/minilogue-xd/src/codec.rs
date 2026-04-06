//! Korg 7-bit SysEx codec (NOTE 1 of the MIDI Implementation spec).
//!
//! The Minilogue XD (and other Korg instruments) transfer 8-bit data inside
//! SysEx messages by packing every group of 7 data bytes into 8 wire bytes:
//!
//! - **Wire byte 0** carries the MSBs: bit *i* holds bit 7 of data byte *i*
//!   (for *i* in 0..group_len).
//! - **Wire bytes 1..=group_len** are the data bytes with bit 7 cleared.
//!
//! A final partial group (fewer than 7 data bytes) follows the same rule,
//! emitting `group_len + 1` wire bytes.

use crate::error::{Error, Result};

/// Encode 8-bit data to the Korg 7-bit SysEx wire format (NOTE 1).
///
/// Every group of 7 input bytes becomes 8 wire bytes. A final partial group
/// of *n* bytes (where *n* < 7) becomes *n* + 1 wire bytes. The encoding is
/// infallible because any byte sequence can be represented.
///
/// # Examples
///
/// ```
/// use minilogue_xd::codec::encode_7bit;
///
/// let wire = encode_7bit(&[0xFF, 0x80, 0x00]);
/// assert_eq!(wire, vec![0b0000_0011, 0x7F, 0x00, 0x00]);
/// ```
pub fn encode_7bit(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let full_groups = n / 7;
    let remainder = n % 7;
    let wire_len = full_groups * 8 + if remainder > 0 { remainder + 1 } else { 0 };
    let mut out = Vec::with_capacity(wire_len);

    for chunk in data.chunks(7) {
        let mut msb_byte: u8 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b & 0x80 != 0 {
                msb_byte |= 1 << i;
            }
        }
        out.push(msb_byte);
        for &b in chunk {
            out.push(b & 0x7F);
        }
    }

    out
}

/// Decode Korg 7-bit SysEx wire format back to 8-bit data.
///
/// Returns an error if any wire byte has bit 7 set, since all valid wire
/// bytes must be in the 0x00..=0x7F range required by MIDI SysEx.
///
/// # Errors
///
/// Returns [`Error::Codec`] when a wire byte has the high bit set.
///
/// # Examples
///
/// ```
/// use minilogue_xd::codec::decode_7bit;
///
/// let data = decode_7bit(&[0b0000_0011, 0x7F, 0x00, 0x00]).unwrap();
/// assert_eq!(data, vec![0xFF, 0x80, 0x00]);
/// ```
pub fn decode_7bit(data: &[u8]) -> Result<Vec<u8>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    // Validate: every wire byte must have bit 7 clear.
    for (pos, &b) in data.iter().enumerate() {
        if b & 0x80 != 0 {
            return Err(Error::Codec(format!(
                "wire byte at offset {pos} has high bit set (0x{b:02X})"
            )));
        }
    }

    // Each wire group is 1 MSB byte + up to 7 data bytes = up to 8 wire bytes.
    // Upper bound on output size: same as input length.
    let mut out = Vec::with_capacity(data.len());
    let mut pos = 0;

    while pos < data.len() {
        let msb_byte = data[pos];
        pos += 1;

        // Remaining wire bytes in this group (at most 7).
        let group_len = (data.len() - pos).min(7);
        for i in 0..group_len {
            let lo = data[pos + i];
            let hi = if msb_byte & (1 << i) != 0 { 0x80 } else { 0x00 };
            out.push(lo | hi);
        }
        pos += group_len;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ---------------------------------------------------------------
    // 1. Round-trip identity for lengths 0 through 21
    // ---------------------------------------------------------------
    #[test]
    fn round_trip_identity_0_through_21() {
        for len in 0..=21 {
            let data: Vec<u8> = (0..len).map(|i| (i * 37) as u8).collect();
            let encoded = encode_7bit(&data);
            let decoded = decode_7bit(&encoded).unwrap();
            assert_eq!(decoded, data, "round-trip failed for length {len}");
        }
    }

    // ---------------------------------------------------------------
    // 2. Known size vectors
    // ---------------------------------------------------------------
    #[test]
    fn wire_length_336_bytes() {
        let data = vec![0xAB; 336];
        let wire = encode_7bit(&data);
        assert_eq!(wire.len(), 384);
    }

    #[test]
    fn wire_length_32_bytes() {
        let data = vec![0x00; 32];
        let wire = encode_7bit(&data);
        // 32 / 7 = 4 full groups (4*8=32), remainder 4 -> 4+1=5, total 37
        assert_eq!(wire.len(), 37);
    }

    #[test]
    fn wire_length_9_bytes() {
        let data = vec![0x00; 9];
        let wire = encode_7bit(&data);
        // 9 / 7 = 1 full group (8), remainder 2 -> 2+1=3, total 11
        assert_eq!(wire.len(), 11);
    }

    // ---------------------------------------------------------------
    // 3. Error on high bit set
    // ---------------------------------------------------------------
    #[test]
    fn decode_error_on_high_bit() {
        let wire = vec![0x00, 0x80];
        let result = decode_7bit(&wire);
        assert!(result.is_err());
        match result {
            Err(Error::Codec(msg)) => {
                assert!(msg.contains("high bit set"), "unexpected message: {msg}");
            }
            other => panic!("expected Error::Codec, got {other:?}"),
        }
    }

    #[test]
    fn decode_error_various_positions() {
        for pos in 0..8 {
            let mut wire = vec![0x00; 8];
            wire[pos] = 0xFF;
            assert!(
                decode_7bit(&wire).is_err(),
                "should error with high bit at position {pos}"
            );
        }
    }

    // ---------------------------------------------------------------
    // 4. Empty input
    // ---------------------------------------------------------------
    #[test]
    fn encode_empty() {
        assert!(encode_7bit(&[]).is_empty());
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode_7bit(&[]).unwrap(), Vec::<u8>::new());
    }

    // ---------------------------------------------------------------
    // 5. Wire length formula verification
    // ---------------------------------------------------------------
    #[test]
    fn wire_length_formula() {
        for n in 0..=300 {
            let data = vec![0u8; n];
            let wire = encode_7bit(&data);
            let expected = (n / 7) * 8 + if n % 7 > 0 { n % 7 + 1 } else { 0 };
            assert_eq!(
                wire.len(),
                expected,
                "wire length mismatch for {n} data bytes"
            );
        }
    }

    // ---------------------------------------------------------------
    // 6. Specific known values
    // ---------------------------------------------------------------
    #[test]
    fn encode_known_values_ff_80_00() {
        let data = [0xFF, 0x80, 0x00];
        let wire = encode_7bit(&data);
        // MSB byte: bit 0 set (0xFF has high bit), bit 1 set (0x80 has high bit),
        //           bit 2 clear (0x00) -> 0b0000_0011 = 0x03
        // Data bytes with bit 7 cleared: 0x7F, 0x00, 0x00
        assert_eq!(wire, vec![0x03, 0x7F, 0x00, 0x00]);

        let decoded = decode_7bit(&wire).unwrap();
        assert_eq!(decoded, data.to_vec());
    }

    #[test]
    fn encode_known_values_all_high() {
        let data = [0xFF; 7];
        let wire = encode_7bit(&data);
        // MSB byte: all 7 bits set -> 0b0111_1111 = 0x7F
        // Data bytes: all 0x7F
        assert_eq!(wire, vec![0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F]);

        let decoded = decode_7bit(&wire).unwrap();
        assert_eq!(decoded, data.to_vec());
    }

    #[test]
    fn encode_known_values_all_zero() {
        let data = [0x00; 7];
        let wire = encode_7bit(&data);
        assert_eq!(wire, vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let decoded = decode_7bit(&wire).unwrap();
        assert_eq!(decoded, data.to_vec());
    }

    // ---------------------------------------------------------------
    // Proptest: round-trip for arbitrary data
    // ---------------------------------------------------------------
    proptest! {
        #[test]
        fn round_trip_arbitrary(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            let encoded = encode_7bit(&data);
            let decoded = decode_7bit(&encoded).unwrap();
            prop_assert_eq!(decoded, data);
        }
    }
}
