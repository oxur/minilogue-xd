//! Tuning data for user scales, user octaves, and MIDI Tuning Standard (MTS).
//!
//! The Minilogue XD supports two tuning tables:
//!
//! - **User Scale** (TABLE 3): 128 notes, each with a semitone number and a
//!   14-bit fractional cent offset. Total blob size: 384 bytes.
//! - **User Octave** (TABLE 4): 12 notes with the same 3-byte format, for a
//!   total of 36 bytes. The pattern repeats across all octaves.
//!
//! Each note is encoded as 3 bytes:
//! - Byte 0: semitone number (u8)
//! - Byte 1: fraction high (bits 13..7 of a 14-bit value, stored as u7)
//! - Byte 2: fraction low (bits 6..0 of a 14-bit value, stored as u7)
//!
//! The 14-bit fraction maps linearly: 0..16383 = 0..100 cents.
//!
//! This module also provides builders and parsers for the MIDI Tuning Standard
//! (MTS) Bulk Tuning Dump and Single Note Tuning Change messages.

use crate::error::{Result, SysexError};
use crate::message::types::U4;
use crate::sysex::frame;

// ---------------------------------------------------------------------------
// CentOffset
// ---------------------------------------------------------------------------

/// A pitch offset encoded as a semitone number plus a 14-bit fractional cent.
///
/// The 14-bit fraction represents 0..16383 mapping to 0..100 cents, giving
/// a resolution of approximately 0.0061 cents per step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CentOffset {
    /// Semitone number (0--127 for scale data, or a wider range for octave).
    pub semitone: u8,
    /// Fractional cent offset (0--16383, where 16384 would equal 100 cents).
    pub fraction: u16,
}

impl CentOffset {
    /// Parse a `CentOffset` from a 3-byte array.
    ///
    /// - `bytes[0]`: semitone
    /// - `bytes[1]`: fraction high 7 bits (bits 13..7)
    /// - `bytes[2]`: fraction low 7 bits (bits 6..0)
    pub fn from_bytes(bytes: &[u8; 3]) -> Self {
        let semitone = bytes[0];
        let fraction = (u16::from(bytes[1] & 0x7F) << 7) | u16::from(bytes[2] & 0x7F);
        Self { semitone, fraction }
    }

    /// Serialize to a 3-byte array.
    pub fn to_bytes(&self) -> [u8; 3] {
        let frac = self.fraction & 0x3FFF;
        [self.semitone, (frac >> 7) as u8, (frac & 0x7F) as u8]
    }

    /// Convert to a floating-point cent value.
    ///
    /// The result is `semitone * 100.0 + fraction * (100.0 / 16384.0)`.
    pub fn to_cents(&self) -> f32 {
        f32::from(self.semitone) * 100.0 + f32::from(self.fraction) * (100.0 / 16384.0)
    }

    /// Create a `CentOffset` from a semitone number and a fractional cent value.
    ///
    /// The `cents` parameter is the fractional part within the semitone
    /// (0.0..100.0). Values outside this range are clamped.
    pub fn from_cents(semitone: u8, cents: f32) -> Self {
        let clamped = cents.clamp(0.0, 100.0 - (100.0 / 16384.0));
        let fraction = (clamped * 16384.0 / 100.0).round() as u16;
        let fraction = fraction.min(16383);
        Self { semitone, fraction }
    }
}

// ---------------------------------------------------------------------------
// UserScale (TABLE 3)
// ---------------------------------------------------------------------------

/// A 128-note user scale tuning table (TABLE 3).
///
/// Each note has an independent [`CentOffset`] defining its absolute pitch.
/// The raw blob is 384 bytes (128 notes x 3 bytes each).
#[derive(Debug, Clone, PartialEq)]
pub struct UserScale(pub [CentOffset; 128]);

impl UserScale {
    /// Size of the raw 8-bit blob in bytes.
    pub const BLOB_SIZE: usize = 384;

    /// Parse a user scale from raw 8-bit data (384 bytes).
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::PayloadTooShort`] if `data` is shorter than 384 bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::BLOB_SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::BLOB_SIZE,
                actual: data.len(),
            }
            .into());
        }
        let mut notes = [CentOffset {
            semitone: 0,
            fraction: 0,
        }; 128];
        for (i, note) in notes.iter_mut().enumerate() {
            let offset = i * 3;
            let bytes: &[u8; 3] = data[offset..offset + 3]
                .try_into()
                .expect("slice is exactly 3 bytes");
            *note = CentOffset::from_bytes(bytes);
        }
        Ok(Self(notes))
    }

    /// Serialize to a 384-byte blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::BLOB_SIZE);
        for note in &self.0 {
            out.extend_from_slice(&note.to_bytes());
        }
        out
    }

    /// Create an equal-temperament scale where each note maps to its own
    /// semitone number with zero fractional offset.
    pub fn equal_temperament() -> Self {
        let mut notes = [CentOffset {
            semitone: 0,
            fraction: 0,
        }; 128];
        for (i, note) in notes.iter_mut().enumerate() {
            note.semitone = i as u8;
        }
        Self(notes)
    }
}

// ---------------------------------------------------------------------------
// UserOctave (TABLE 4)
// ---------------------------------------------------------------------------

/// A 12-note user octave tuning table (TABLE 4).
///
/// Defines pitch offsets for one octave that repeats across the keyboard.
/// The raw blob is 36 bytes (12 notes x 3 bytes each).
#[derive(Debug, Clone, PartialEq)]
pub struct UserOctave(pub [CentOffset; 12]);

impl UserOctave {
    /// Size of the raw 8-bit blob in bytes.
    pub const BLOB_SIZE: usize = 36;

    /// Parse a user octave from raw 8-bit data (36 bytes).
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::PayloadTooShort`] if `data` is shorter than 36 bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::BLOB_SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::BLOB_SIZE,
                actual: data.len(),
            }
            .into());
        }
        let mut notes = [CentOffset {
            semitone: 0,
            fraction: 0,
        }; 12];
        for (i, note) in notes.iter_mut().enumerate() {
            let offset = i * 3;
            let bytes: &[u8; 3] = data[offset..offset + 3]
                .try_into()
                .expect("slice is exactly 3 bytes");
            *note = CentOffset::from_bytes(bytes);
        }
        Ok(Self(notes))
    }

    /// Serialize to a 36-byte blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::BLOB_SIZE);
        for note in &self.0 {
            out.extend_from_slice(&note.to_bytes());
        }
        out
    }

    /// Create an equal-temperament octave where each note maps to its own
    /// semitone number (0--11) with zero fractional offset.
    pub fn equal_temperament() -> Self {
        let mut notes = [CentOffset {
            semitone: 0,
            fraction: 0,
        }; 12];
        for (i, note) in notes.iter_mut().enumerate() {
            note.semitone = i as u8;
        }
        Self(notes)
    }
}

// ---------------------------------------------------------------------------
// Korg SysEx wrappers
// ---------------------------------------------------------------------------

/// Build a SysEx user scale dump request (function 0x14).
pub fn build_user_scale_request(channel: U4) -> Vec<u8> {
    frame::build_sysex_request(channel, frame::USER_SCALE_REQUEST)
}

/// Build a SysEx user scale data dump (function 0x44).
///
/// The 384-byte scale blob is 7-bit encoded inside the Korg SysEx frame.
pub fn build_user_scale_dump(channel: U4, scale: &UserScale) -> Vec<u8> {
    let blob = scale.to_bytes();
    frame::build_sysex(channel, frame::USER_SCALE_DUMP, &blob)
}

/// Parse a SysEx user scale data dump (function 0x44).
///
/// # Errors
///
/// Returns an error if the frame is malformed, the function ID is wrong,
/// or the blob data is invalid.
pub fn parse_user_scale_dump(bytes: &[u8]) -> Result<UserScale> {
    let parsed = frame::parse_sysex(bytes)?;
    if parsed.function_id != frame::USER_SCALE_DUMP {
        return Err(SysexError::WrongFunctionId {
            expected: frame::USER_SCALE_DUMP,
            found: parsed.function_id,
        }
        .into());
    }
    UserScale::from_bytes(&parsed.data)
}

/// Build a SysEx user octave dump request (function 0x15).
pub fn build_user_octave_request(channel: U4) -> Vec<u8> {
    frame::build_sysex_request(channel, frame::USER_OCTAVE_REQUEST)
}

/// Build a SysEx user octave data dump (function 0x45).
///
/// The 36-byte octave blob is 7-bit encoded inside the Korg SysEx frame.
pub fn build_user_octave_dump(channel: U4, octave: &UserOctave) -> Vec<u8> {
    let blob = octave.to_bytes();
    frame::build_sysex(channel, frame::USER_OCTAVE_DUMP, &blob)
}

/// Parse a SysEx user octave data dump (function 0x45).
///
/// # Errors
///
/// Returns an error if the frame is malformed, the function ID is wrong,
/// or the blob data is invalid.
pub fn parse_user_octave_dump(bytes: &[u8]) -> Result<UserOctave> {
    let parsed = frame::parse_sysex(bytes)?;
    if parsed.function_id != frame::USER_OCTAVE_DUMP {
        return Err(SysexError::WrongFunctionId {
            expected: frame::USER_OCTAVE_DUMP,
            found: parsed.function_id,
        }
        .into());
    }
    UserOctave::from_bytes(&parsed.data)
}

// ---------------------------------------------------------------------------
// MIDI Tuning Standard (MTS)
// ---------------------------------------------------------------------------

/// Build an MTS Bulk Tuning Dump message.
///
/// Format: `[F0, 7E, device_id, 08, 01, tt, name(16), data(384), checksum, F7]`
///
/// The checksum is the XOR of all bytes from `device_id` through the last
/// data byte (exclusive of F0 and F7).
///
/// The `name` is truncated or padded to exactly 16 bytes (ASCII, space-padded).
pub fn build_mts_bulk_dump(device_id: u8, program: u8, name: &str, scale: &UserScale) -> Vec<u8> {
    // Total: F0 + 7E + device_id + 08 + 01 + tt + 16 name + 384 data + checksum + F7
    // = 2 + 1 + 1 + 1 + 1 + 16 + 384 + 1 + 1 = 408
    let mut msg = Vec::with_capacity(408);
    msg.push(0xF0);
    msg.push(0x7E);
    msg.push(device_id & 0x7F);
    msg.push(0x08);
    msg.push(0x01);
    msg.push(program & 0x7F);

    // 16-byte name, space-padded.
    let name_bytes = name.as_bytes();
    for i in 0..16 {
        if i < name_bytes.len() {
            msg.push(name_bytes[i] & 0x7F);
        } else {
            msg.push(b' ');
        }
    }

    // 128 notes x 3 bytes = 384 bytes.
    for note in &scale.0 {
        let b = note.to_bytes();
        msg.push(b[0] & 0x7F);
        msg.push(b[1] & 0x7F);
        msg.push(b[2] & 0x7F);
    }

    // Checksum: XOR of all bytes from device_id to last data byte.
    let mut checksum: u8 = 0;
    for &b in &msg[2..] {
        checksum ^= b;
    }
    msg.push(checksum & 0x7F);
    msg.push(0xF7);
    msg
}

/// Parse an MTS Bulk Tuning Dump message.
///
/// # Errors
///
/// Returns an error if the message is too short, has wrong sub-IDs,
/// or the checksum does not match.
pub fn parse_mts_bulk_dump(bytes: &[u8]) -> Result<UserScale> {
    // Minimum: F0 7E dev 08 01 tt name(16) data(384) checksum F7
    // = 1 + 1 + 1 + 1 + 1 + 1 + 16 + 384 + 1 + 1 = 408
    let expected_len = 408;
    if bytes.len() < expected_len {
        return Err(SysexError::PayloadTooShort {
            expected: expected_len,
            actual: bytes.len(),
        }
        .into());
    }

    if bytes[0] != 0xF0 || bytes[1] != 0x7E {
        return Err(SysexError::InvalidHeader("expected MTS header F0 7E".to_string()).into());
    }
    if bytes[3] != 0x08 || bytes[4] != 0x01 {
        return Err(SysexError::InvalidHeader(
            "expected MTS Bulk Tuning Dump sub-IDs 08 01".to_string(),
        )
        .into());
    }
    if bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader("expected F7 end byte".to_string()).into());
    }

    // Verify checksum: XOR of bytes[2..bytes.len()-2].
    let checksum_idx = bytes.len() - 2;
    let mut computed: u8 = 0;
    for &b in &bytes[2..checksum_idx] {
        computed ^= b;
    }
    computed &= 0x7F;
    let stored = bytes[checksum_idx];
    if computed != stored {
        return Err(SysexError::ChecksumMismatch {
            expected: computed,
            actual: stored,
        }
        .into());
    }

    // Data starts at byte 22 (after F0 7E dev 08 01 tt name(16)).
    let data_start = 22;
    let mut notes = [CentOffset {
        semitone: 0,
        fraction: 0,
    }; 128];
    for (i, note) in notes.iter_mut().enumerate() {
        let offset = data_start + i * 3;
        let b: [u8; 3] = [bytes[offset], bytes[offset + 1], bytes[offset + 2]];
        *note = CentOffset::from_bytes(&b);
    }

    Ok(UserScale(notes))
}

/// Build an MTS Single Note Tuning Change message.
///
/// Format: `[F0, 7E, device_id, 08, 02, tt, ll, (kk, xx, yy, zz)*ll, F7]`
///
/// Each change is 4 bytes: key number, semitone, fraction_hi, fraction_lo.
pub fn build_mts_single_note_change(
    device_id: u8,
    program: u8,
    changes: &[(u8, CentOffset)],
) -> Vec<u8> {
    let count = changes.len().min(127);
    let mut msg = Vec::with_capacity(8 + count * 4);
    msg.push(0xF0);
    msg.push(0x7E);
    msg.push(device_id & 0x7F);
    msg.push(0x08);
    msg.push(0x02);
    msg.push(program & 0x7F);
    msg.push(count as u8);

    for &(key, co) in changes.iter().take(count) {
        let b = co.to_bytes();
        msg.push(key & 0x7F);
        msg.push(b[0] & 0x7F);
        msg.push(b[1] & 0x7F);
        msg.push(b[2] & 0x7F);
    }

    msg.push(0xF7);
    msg
}

/// Parse an MTS Single Note Tuning Change message.
///
/// Returns a vector of `(key_number, CentOffset)` tuples.
///
/// # Errors
///
/// Returns an error if the message is too short or has wrong sub-IDs.
pub fn parse_mts_single_note_change(bytes: &[u8]) -> Result<Vec<(u8, CentOffset)>> {
    // Minimum: F0 7E dev 08 02 tt ll F7 = 8
    if bytes.len() < 8 {
        return Err(SysexError::PayloadTooShort {
            expected: 8,
            actual: bytes.len(),
        }
        .into());
    }

    if bytes[0] != 0xF0 || bytes[1] != 0x7E {
        return Err(SysexError::InvalidHeader("expected MTS header F0 7E".to_string()).into());
    }
    if bytes[3] != 0x08 || bytes[4] != 0x02 {
        return Err(SysexError::InvalidHeader(
            "expected MTS Single Note Tuning Change sub-IDs 08 02".to_string(),
        )
        .into());
    }
    if bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader("expected F7 end byte".to_string()).into());
    }

    let count = bytes[6] as usize;
    let data_start = 7;
    let needed = data_start + count * 4 + 1; // +1 for F7
    if bytes.len() < needed {
        return Err(SysexError::PayloadTooShort {
            expected: needed,
            actual: bytes.len(),
        }
        .into());
    }

    let mut changes = Vec::with_capacity(count);
    for i in 0..count {
        let offset = data_start + i * 4;
        let key = bytes[offset];
        let b: [u8; 3] = [bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]];
        changes.push((key, CentOffset::from_bytes(&b)));
    }

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    // ---------------------------------------------------------------
    // CentOffset
    // ---------------------------------------------------------------

    #[test]
    fn cent_offset_from_bytes_zero() {
        let co = CentOffset::from_bytes(&[0, 0, 0]);
        assert_eq!(co.semitone, 0);
        assert_eq!(co.fraction, 0);
    }

    #[test]
    fn cent_offset_from_bytes_max_fraction() {
        // fraction = 0x7F << 7 | 0x7F = 16383
        let co = CentOffset::from_bytes(&[60, 0x7F, 0x7F]);
        assert_eq!(co.semitone, 60);
        assert_eq!(co.fraction, 16383);
    }

    #[test]
    fn cent_offset_to_bytes_roundtrip() {
        let co = CentOffset {
            semitone: 69,
            fraction: 8192,
        };
        let bytes = co.to_bytes();
        let co2 = CentOffset::from_bytes(&bytes);
        assert_eq!(co, co2);
    }

    #[test]
    fn cent_offset_to_bytes_roundtrip_all_fractions() {
        for frac in (0..=16383).step_by(127) {
            let co = CentOffset {
                semitone: 42,
                fraction: frac,
            };
            let bytes = co.to_bytes();
            let co2 = CentOffset::from_bytes(&bytes);
            assert_eq!(co, co2, "roundtrip failed for fraction {frac}");
        }
    }

    #[test]
    fn cent_offset_to_cents_zero() {
        let co = CentOffset {
            semitone: 0,
            fraction: 0,
        };
        assert!((co.to_cents() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cent_offset_to_cents_middle_c() {
        let co = CentOffset {
            semitone: 60,
            fraction: 0,
        };
        assert!((co.to_cents() - 6000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cent_offset_to_cents_with_fraction() {
        // fraction = 8192 => 50 cents
        let co = CentOffset {
            semitone: 60,
            fraction: 8192,
        };
        assert!((co.to_cents() - 6050.0).abs() < 0.01);
    }

    #[test]
    fn cent_offset_from_cents_zero() {
        let co = CentOffset::from_cents(60, 0.0);
        assert_eq!(co.semitone, 60);
        assert_eq!(co.fraction, 0);
    }

    #[test]
    fn cent_offset_from_cents_50() {
        let co = CentOffset::from_cents(60, 50.0);
        assert_eq!(co.semitone, 60);
        // 50.0 * 16384 / 100 = 8192
        assert_eq!(co.fraction, 8192);
    }

    #[test]
    fn cent_offset_from_cents_clamps_negative() {
        let co = CentOffset::from_cents(60, -10.0);
        assert_eq!(co.fraction, 0);
    }

    #[test]
    fn cent_offset_from_cents_clamps_high() {
        let co = CentOffset::from_cents(60, 200.0);
        // Should clamp to max fraction
        assert!(co.fraction <= 16383);
    }

    #[test]
    fn cent_offset_fraction_masks_to_14bit() {
        let co = CentOffset {
            semitone: 0,
            fraction: 0xFFFF,
        };
        let bytes = co.to_bytes();
        let co2 = CentOffset::from_bytes(&bytes);
        assert_eq!(co2.fraction, 16383);
    }

    // ---------------------------------------------------------------
    // UserScale
    // ---------------------------------------------------------------

    #[test]
    fn user_scale_blob_size() {
        assert_eq!(UserScale::BLOB_SIZE, 384);
    }

    #[test]
    fn user_scale_equal_temperament() {
        let scale = UserScale::equal_temperament();
        for (i, note) in scale.0.iter().enumerate() {
            assert_eq!(note.semitone, i as u8, "note {i} semitone");
            assert_eq!(note.fraction, 0, "note {i} fraction");
        }
    }

    #[test]
    fn user_scale_roundtrip() {
        let scale = UserScale::equal_temperament();
        let blob = scale.to_bytes();
        assert_eq!(blob.len(), UserScale::BLOB_SIZE);
        let scale2 = UserScale::from_bytes(&blob).unwrap();
        assert_eq!(scale, scale2);
    }

    #[test]
    fn user_scale_roundtrip_nonzero_fractions() {
        let mut scale = UserScale::equal_temperament();
        for (i, note) in scale.0.iter_mut().enumerate() {
            note.fraction = (i as u16 * 128) & 0x3FFF;
        }
        let blob = scale.to_bytes();
        let scale2 = UserScale::from_bytes(&blob).unwrap();
        assert_eq!(scale, scale2);
    }

    #[test]
    fn user_scale_from_bytes_too_short() {
        let data = vec![0u8; 383];
        assert!(UserScale::from_bytes(&data).is_err());
    }

    #[test]
    fn user_scale_from_bytes_extra_data_ok() {
        let data = vec![0u8; 400];
        assert!(UserScale::from_bytes(&data).is_ok());
    }

    // ---------------------------------------------------------------
    // UserOctave
    // ---------------------------------------------------------------

    #[test]
    fn user_octave_blob_size() {
        assert_eq!(UserOctave::BLOB_SIZE, 36);
    }

    #[test]
    fn user_octave_equal_temperament() {
        let octave = UserOctave::equal_temperament();
        for (i, note) in octave.0.iter().enumerate() {
            assert_eq!(note.semitone, i as u8, "note {i} semitone");
            assert_eq!(note.fraction, 0, "note {i} fraction");
        }
    }

    #[test]
    fn user_octave_roundtrip() {
        let octave = UserOctave::equal_temperament();
        let blob = octave.to_bytes();
        assert_eq!(blob.len(), UserOctave::BLOB_SIZE);
        let octave2 = UserOctave::from_bytes(&blob).unwrap();
        assert_eq!(octave, octave2);
    }

    #[test]
    fn user_octave_from_bytes_too_short() {
        let data = vec![0u8; 35];
        assert!(UserOctave::from_bytes(&data).is_err());
    }

    // ---------------------------------------------------------------
    // SysEx wrappers
    // ---------------------------------------------------------------

    #[test]
    fn build_user_scale_request_format() {
        let msg = build_user_scale_request(ch(0));
        assert_eq!(msg.len(), 8);
        assert_eq!(msg[6], frame::USER_SCALE_REQUEST);
    }

    #[test]
    fn user_scale_sysex_roundtrip() {
        let scale = UserScale::equal_temperament();
        let msg = build_user_scale_dump(ch(5), &scale);
        let scale2 = parse_user_scale_dump(&msg).unwrap();
        assert_eq!(scale, scale2);
    }

    #[test]
    fn parse_user_scale_dump_wrong_function_id() {
        let msg = frame::build_sysex(ch(0), frame::USER_OCTAVE_DUMP, &[0u8; 384]);
        assert!(parse_user_scale_dump(&msg).is_err());
    }

    #[test]
    fn build_user_octave_request_format() {
        let msg = build_user_octave_request(ch(3));
        assert_eq!(msg.len(), 8);
        assert_eq!(msg[6], frame::USER_OCTAVE_REQUEST);
    }

    #[test]
    fn user_octave_sysex_roundtrip() {
        let octave = UserOctave::equal_temperament();
        let msg = build_user_octave_dump(ch(7), &octave);
        let octave2 = parse_user_octave_dump(&msg).unwrap();
        assert_eq!(octave, octave2);
    }

    #[test]
    fn parse_user_octave_dump_wrong_function_id() {
        let msg = frame::build_sysex(ch(0), frame::USER_SCALE_DUMP, &[0u8; 36]);
        assert!(parse_user_octave_dump(&msg).is_err());
    }

    #[test]
    fn user_scale_sysex_roundtrip_with_data() {
        let mut scale = UserScale::equal_temperament();
        scale.0[69].fraction = 4096;
        scale.0[0].semitone = 12;
        let msg = build_user_scale_dump(ch(0), &scale);
        let scale2 = parse_user_scale_dump(&msg).unwrap();
        assert_eq!(scale, scale2);
    }

    // ---------------------------------------------------------------
    // MTS Bulk Tuning Dump
    // ---------------------------------------------------------------

    #[test]
    fn mts_bulk_dump_length() {
        let scale = UserScale::equal_temperament();
        let msg = build_mts_bulk_dump(0x7F, 0, "Test Scale", &scale);
        assert_eq!(msg.len(), 408);
    }

    #[test]
    fn mts_bulk_dump_header() {
        let scale = UserScale::equal_temperament();
        let msg = build_mts_bulk_dump(0x10, 5, "Hello", &scale);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x7E);
        assert_eq!(msg[2], 0x10);
        assert_eq!(msg[3], 0x08);
        assert_eq!(msg[4], 0x01);
        assert_eq!(msg[5], 5);
        assert_eq!(*msg.last().unwrap(), 0xF7);
    }

    #[test]
    fn mts_bulk_dump_name_padding() {
        let scale = UserScale::equal_temperament();
        let msg = build_mts_bulk_dump(0, 0, "AB", &scale);
        assert_eq!(msg[6], b'A');
        assert_eq!(msg[7], b'B');
        assert_eq!(msg[8], b' ');
        assert_eq!(msg[21], b' ');
    }

    #[test]
    fn mts_bulk_dump_roundtrip() {
        let mut scale = UserScale::equal_temperament();
        scale.0[60].fraction = 8192;
        let msg = build_mts_bulk_dump(0x7F, 0, "Round Trip", &scale);
        let scale2 = parse_mts_bulk_dump(&msg).unwrap();
        assert_eq!(scale, scale2);
    }

    #[test]
    fn mts_bulk_dump_checksum_verification() {
        let scale = UserScale::equal_temperament();
        let mut msg = build_mts_bulk_dump(0x7F, 0, "Test", &scale);
        // Corrupt one data byte.
        msg[30] ^= 0x01;
        assert!(parse_mts_bulk_dump(&msg).is_err());
    }

    #[test]
    fn mts_bulk_dump_too_short() {
        assert!(parse_mts_bulk_dump(&[0xF0, 0x7E]).is_err());
    }

    #[test]
    fn mts_bulk_dump_wrong_header() {
        let mut msg = vec![0u8; 408];
        msg[0] = 0xF0;
        msg[1] = 0x7D; // wrong
        msg[407] = 0xF7;
        assert!(parse_mts_bulk_dump(&msg).is_err());
    }

    #[test]
    fn mts_bulk_dump_wrong_sub_ids() {
        let mut msg = vec![0u8; 408];
        msg[0] = 0xF0;
        msg[1] = 0x7E;
        msg[3] = 0x09; // wrong
        msg[407] = 0xF7;
        assert!(parse_mts_bulk_dump(&msg).is_err());
    }

    #[test]
    fn mts_bulk_dump_missing_f7() {
        let scale = UserScale::equal_temperament();
        let mut msg = build_mts_bulk_dump(0x7F, 0, "Test", &scale);
        let last = msg.len() - 1;
        msg[last] = 0x00;
        assert!(parse_mts_bulk_dump(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // MTS Single Note Tuning Change
    // ---------------------------------------------------------------

    #[test]
    fn mts_single_note_change_empty() {
        let msg = build_mts_single_note_change(0x7F, 0, &[]);
        assert_eq!(msg.len(), 8);
        assert_eq!(msg[6], 0); // count = 0
    }

    #[test]
    fn mts_single_note_change_roundtrip_one() {
        let co = CentOffset {
            semitone: 69,
            fraction: 4096,
        };
        let changes = vec![(60u8, co)];
        let msg = build_mts_single_note_change(0x7F, 3, &changes);
        let parsed = parse_mts_single_note_change(&msg).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, 60);
        assert_eq!(parsed[0].1, co);
    }

    #[test]
    fn mts_single_note_change_roundtrip_multiple() {
        let changes: Vec<(u8, CentOffset)> = (0..10)
            .map(|i| {
                (
                    i * 12,
                    CentOffset {
                        semitone: i * 12,
                        fraction: u16::from(i) * 1000,
                    },
                )
            })
            .collect();
        let msg = build_mts_single_note_change(0x00, 0, &changes);
        let parsed = parse_mts_single_note_change(&msg).unwrap();
        assert_eq!(parsed.len(), changes.len());
        for (i, (key, co)) in parsed.iter().enumerate() {
            assert_eq!(*key, changes[i].0, "key {i}");
            assert_eq!(*co, changes[i].1, "offset {i}");
        }
    }

    #[test]
    fn mts_single_note_change_header() {
        let msg = build_mts_single_note_change(0x10, 7, &[]);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], 0x7E);
        assert_eq!(msg[2], 0x10);
        assert_eq!(msg[3], 0x08);
        assert_eq!(msg[4], 0x02);
        assert_eq!(msg[5], 7);
    }

    #[test]
    fn mts_single_note_change_too_short() {
        assert!(parse_mts_single_note_change(&[0xF0, 0x7E]).is_err());
    }

    #[test]
    fn mts_single_note_change_wrong_header() {
        let msg = vec![0xF0, 0x7D, 0x00, 0x08, 0x02, 0x00, 0x00, 0xF7];
        assert!(parse_mts_single_note_change(&msg).is_err());
    }

    #[test]
    fn mts_single_note_change_wrong_sub_ids() {
        let msg = vec![0xF0, 0x7E, 0x00, 0x08, 0x03, 0x00, 0x00, 0xF7];
        assert!(parse_mts_single_note_change(&msg).is_err());
    }

    #[test]
    fn mts_single_note_change_truncated_data() {
        // Claims 1 change but only has F7 after count.
        let msg = vec![0xF0, 0x7E, 0x00, 0x08, 0x02, 0x00, 0x01, 0xF7];
        assert!(parse_mts_single_note_change(&msg).is_err());
    }

    #[test]
    fn mts_single_note_change_missing_f7() {
        let msg = vec![0xF0, 0x7E, 0x00, 0x08, 0x02, 0x00, 0x00, 0x00];
        assert!(parse_mts_single_note_change(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // Trait derivations
    // ---------------------------------------------------------------

    #[test]
    fn cent_offset_debug_clone_copy() {
        let co = CentOffset {
            semitone: 60,
            fraction: 0,
        };
        let co2 = co; // Copy
        assert_eq!(co, co2);
        let _dbg = format!("{co:?}");
    }

    #[test]
    fn user_scale_debug_clone() {
        let scale = UserScale::equal_temperament();
        let scale2 = scale.clone();
        assert_eq!(scale, scale2);
        let _dbg = format!("{scale:?}");
    }

    #[test]
    fn user_octave_debug_clone() {
        let octave = UserOctave::equal_temperament();
        let octave2 = octave.clone();
        assert_eq!(octave, octave2);
        let _dbg = format!("{octave:?}");
    }
}
