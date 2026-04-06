//! Poly Chain SysEx messages for the Korg Minilogue XD.
//!
//! When the Minilogue XD is operating in poly chain mode (linking multiple
//! units for extra polyphony), it exchanges note-on and note-off messages
//! via SysEx to coordinate voice assignment across units.
//!
//! ## Poly Chain Note On (function 0x60)
//!
//! ```text
//! [F0, 42, 3g, 00, 01, 51, 60, vv, kk, vv, hh, mm, ll, F7]
//! ```
//!
//! - `vv`: voice slot (2 bits, 0--3)
//! - `kk`: note number (0--127)
//! - `vv`: velocity (0--127)
//! - `hh`, `mm`, `ll`: 21-bit pitch split across 3 x 7-bit bytes
//!
//! ## Poly Chain Note Off (function 0x61)
//!
//! ```text
//! [F0, 42, 3g, 00, 01, 51, 61, vv, mm, F7]
//! ```
//!
//! - `vv`: voice slot (2 bits, 0--3)
//! - `mm`: mute flag (0 or 1)

use std::fmt;

use crate::error::{Error, Result, SysexError};
use crate::message::types::{U4, U7};
use crate::sysex::frame;
use crate::sysex::{DEVICE_ID, KORG_ID};

// ---------------------------------------------------------------------------
// U2 — 2-bit voice slot index
// ---------------------------------------------------------------------------

/// A 2-bit unsigned integer (0--3), used for poly chain voice slot indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct U2(u8);

impl U2 {
    /// The minimum value (0).
    pub const MIN: u8 = 0;
    /// The maximum value (3).
    pub const MAX: u8 = 3;

    /// Creates a new `U2` if `value` is in range 0..=3.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 3.
    pub fn new(value: u8) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "U2",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner `u8` value.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for U2 {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<U2> for u8 {
    fn from(val: U2) -> Self {
        val.0
    }
}

impl fmt::Display for U2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// PolyChainNoteOn
// ---------------------------------------------------------------------------

/// Maximum value for a 21-bit pitch field.
const PITCH_MAX: u32 = 0x1F_FFFF; // 2^21 - 1

/// A poly chain note-on message (function 0x60).
///
/// Contains the voice slot assignment, note number, velocity, and a 21-bit
/// pitch value that provides sub-semitone precision for the assigned voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyChainNoteOn {
    /// Voice slot index (0--3).
    pub voice_slot: U2,
    /// MIDI note number (0--127).
    pub note: U7,
    /// Note velocity (0--127).
    pub velocity: U7,
    /// 21-bit pitch value (0..=2_097_151).
    pub pitch: u32,
}

// ---------------------------------------------------------------------------
// PolyChainNoteOff
// ---------------------------------------------------------------------------

/// A poly chain note-off message (function 0x61).
///
/// Tells the slave unit to release the voice in the specified slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyChainNoteOff {
    /// Voice slot index (0--3).
    pub voice_slot: U2,
    /// Whether to mute the voice immediately.
    pub mute: bool,
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

/// Build a Poly Chain Note On SysEx message (function 0x60).
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 60, vv, kk, vv, hh, mm, ll, F7]`
///
/// The pitch is encoded as three 7-bit bytes:
/// - H = (pitch >> 14) & 0x7F
/// - M = (pitch >> 7) & 0x7F
/// - L = pitch & 0x7F
///
/// # Errors
///
/// Returns [`Error::OutOfRange`] if `note_on.pitch` exceeds 2^21 - 1.
pub fn build_note_on(channel: U4, note_on: &PolyChainNoteOn) -> Result<Vec<u8>> {
    if note_on.pitch > PITCH_MAX {
        return Err(Error::OutOfRange {
            type_name: "pitch",
            value: i64::from(note_on.pitch),
            min: 0,
            max: i64::from(PITCH_MAX),
        });
    }
    let pitch = note_on.pitch;
    let pitch_h = ((pitch >> 14) & 0x7F) as u8;
    let pitch_m = ((pitch >> 7) & 0x7F) as u8;
    let pitch_l = (pitch & 0x7F) as u8;

    Ok(vec![
        0xF0,
        KORG_ID,
        0x30 | channel.value(),
        DEVICE_ID[0],
        DEVICE_ID[1],
        DEVICE_ID[2],
        frame::POLY_CHAIN_NOTE_ON,
        note_on.voice_slot.value(),
        note_on.note.value(),
        note_on.velocity.value(),
        pitch_h,
        pitch_m,
        pitch_l,
        0xF7,
    ])
}

/// Build a Poly Chain Note Off SysEx message (function 0x61).
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 61, vv, mm, F7]`
pub fn build_note_off(channel: U4, note_off: &PolyChainNoteOff) -> Vec<u8> {
    vec![
        0xF0,
        KORG_ID,
        0x30 | channel.value(),
        DEVICE_ID[0],
        DEVICE_ID[1],
        DEVICE_ID[2],
        frame::POLY_CHAIN_NOTE_OFF,
        note_off.voice_slot.value(),
        u8::from(note_off.mute),
        0xF7,
    ]
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// Parse a Poly Chain Note On SysEx message (function 0x60).
///
/// Expects a raw SysEx message of exactly 14 bytes.
///
/// # Errors
///
/// Returns an error if the message is malformed, too short, or has the
/// wrong function ID.
pub fn parse_note_on(bytes: &[u8]) -> Result<PolyChainNoteOn> {
    // Expected: F0 42 3g 00 01 51 60 vv kk vv hh mm ll F7 = 14 bytes
    let expected_len = 14;
    if bytes.len() < expected_len {
        return Err(SysexError::PayloadTooShort {
            expected: expected_len,
            actual: bytes.len(),
        }
        .into());
    }

    // Validate header bytes.
    validate_header(bytes)?;

    if bytes[6] != frame::POLY_CHAIN_NOTE_ON {
        return Err(SysexError::WrongFunctionId {
            expected: frame::POLY_CHAIN_NOTE_ON,
            found: bytes[6],
        }
        .into());
    }

    if bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader("expected F7 end byte".to_string()).into());
    }

    let voice_slot = U2::new(bytes[7])?;
    let note = U7::new(bytes[8])?;
    let velocity = U7::new(bytes[9])?;

    let pitch_h = u32::from(bytes[10] & 0x7F);
    let pitch_m = u32::from(bytes[11] & 0x7F);
    let pitch_l = u32::from(bytes[12] & 0x7F);
    let pitch = (pitch_h << 14) | (pitch_m << 7) | pitch_l;

    Ok(PolyChainNoteOn {
        voice_slot,
        note,
        velocity,
        pitch,
    })
}

/// Parse a Poly Chain Note Off SysEx message (function 0x61).
///
/// Expects a raw SysEx message of exactly 10 bytes.
///
/// # Errors
///
/// Returns an error if the message is malformed, too short, or has the
/// wrong function ID.
pub fn parse_note_off(bytes: &[u8]) -> Result<PolyChainNoteOff> {
    // Expected: F0 42 3g 00 01 51 61 vv mm F7 = 10 bytes
    let expected_len = 10;
    if bytes.len() < expected_len {
        return Err(SysexError::PayloadTooShort {
            expected: expected_len,
            actual: bytes.len(),
        }
        .into());
    }

    validate_header(bytes)?;

    if bytes[6] != frame::POLY_CHAIN_NOTE_OFF {
        return Err(SysexError::WrongFunctionId {
            expected: frame::POLY_CHAIN_NOTE_OFF,
            found: bytes[6],
        }
        .into());
    }

    if bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader("expected F7 end byte".to_string()).into());
    }

    let voice_slot = U2::new(bytes[7])?;
    let mute = bytes[8] != 0;

    Ok(PolyChainNoteOff { voice_slot, mute })
}

// ---------------------------------------------------------------------------
// Internal header validation
// ---------------------------------------------------------------------------

/// Validate the Korg SysEx header (first 6 bytes).
fn validate_header(bytes: &[u8]) -> Result<()> {
    if bytes[0] != 0xF0 {
        return Err(SysexError::InvalidHeader(format!(
            "expected F0 start byte, got 0x{:02X}",
            bytes[0]
        ))
        .into());
    }
    if bytes[1] != KORG_ID {
        return Err(SysexError::InvalidHeader(format!(
            "expected Korg ID 0x{KORG_ID:02X}, got 0x{:02X}",
            bytes[1]
        ))
        .into());
    }
    if bytes[2] & 0xF0 != 0x30 {
        return Err(SysexError::InvalidHeader(format!(
            "expected channel byte 0x30..0x3F, got 0x{:02X}",
            bytes[2]
        ))
        .into());
    }
    if bytes[3..6] != DEVICE_ID {
        return Err(SysexError::InvalidHeader(format!(
            "expected device ID {:02X?}, got {:02X?}",
            DEVICE_ID,
            &bytes[3..6]
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    // ---------------------------------------------------------------
    // U2
    // ---------------------------------------------------------------

    #[test]
    fn u2_new_valid() {
        for v in 0..=3 {
            let u = U2::new(v).unwrap();
            assert_eq!(u.value(), v);
        }
    }

    #[test]
    fn u2_new_invalid() {
        assert!(U2::new(4).is_err());
        assert!(U2::new(128).is_err());
        assert!(U2::new(255).is_err());
    }

    #[test]
    fn u2_try_from() {
        assert!(U2::try_from(3u8).is_ok());
        assert!(U2::try_from(4u8).is_err());
    }

    #[test]
    fn u2_into_u8() {
        let u = U2::new(2).unwrap();
        let raw: u8 = u.into();
        assert_eq!(raw, 2);
    }

    #[test]
    fn u2_display() {
        let u = U2::new(1).unwrap();
        assert_eq!(format!("{u}"), "1");
    }

    #[test]
    fn u2_copy_hash_ord() {
        use std::collections::HashSet;
        let a = U2::new(0).unwrap();
        let b = U2::new(3).unwrap();
        let c = a; // Copy
        assert_eq!(a, c);
        assert!(a < b);
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&U2::new(0).unwrap()));
    }

    // ---------------------------------------------------------------
    // PolyChainNoteOn — build/parse roundtrip
    // ---------------------------------------------------------------

    #[test]
    fn note_on_roundtrip_basic() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: 0,
        };
        let msg = build_note_on(ch(0), &note_on).unwrap();
        assert_eq!(msg.len(), 14);
        let parsed = parse_note_on(&msg).unwrap();
        assert_eq!(parsed, note_on);
    }

    #[test]
    fn note_on_roundtrip_max_pitch() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(3).unwrap(),
            note: U7::new(127).unwrap(),
            velocity: U7::new(127).unwrap(),
            pitch: PITCH_MAX,
        };
        let msg = build_note_on(ch(15), &note_on).unwrap();
        let parsed = parse_note_on(&msg).unwrap();
        assert_eq!(parsed, note_on);
    }

    #[test]
    fn note_on_pitch_encoding() {
        // Verify the 21-bit pitch encoding explicitly.
        let pitch: u32 = 0b0_1010101_1001100_0110011; // 21 bits
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(1).unwrap(),
            note: U7::new(69).unwrap(),
            velocity: U7::new(64).unwrap(),
            pitch,
        };
        let msg = build_note_on(ch(0), &note_on).unwrap();
        // Check the three pitch bytes.
        let h = msg[10];
        let m = msg[11];
        let l = msg[12];
        assert_eq!(h, ((pitch >> 14) & 0x7F) as u8);
        assert_eq!(m, ((pitch >> 7) & 0x7F) as u8);
        assert_eq!(l, (pitch & 0x7F) as u8);

        let parsed = parse_note_on(&msg).unwrap();
        assert_eq!(parsed.pitch, pitch);
    }

    #[test]
    fn note_on_all_voice_slots() {
        for slot in 0..=3 {
            let note_on = PolyChainNoteOn {
                voice_slot: U2::new(slot).unwrap(),
                note: U7::new(60).unwrap(),
                velocity: U7::new(100).unwrap(),
                pitch: 12345,
            };
            let msg = build_note_on(ch(0), &note_on).unwrap();
            let parsed = parse_note_on(&msg).unwrap();
            assert_eq!(parsed.voice_slot.value(), slot);
        }
    }

    #[test]
    fn note_on_channel_preserved() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: 0,
        };
        for c in 0..=15 {
            let msg = build_note_on(ch(c), &note_on).unwrap();
            assert_eq!(msg[2], 0x30 | c);
        }
    }

    // ---------------------------------------------------------------
    // PolyChainNoteOff — build/parse roundtrip
    // ---------------------------------------------------------------

    #[test]
    fn note_off_roundtrip_mute_false() {
        let note_off = PolyChainNoteOff {
            voice_slot: U2::new(2).unwrap(),
            mute: false,
        };
        let msg = build_note_off(ch(0), &note_off);
        assert_eq!(msg.len(), 10);
        let parsed = parse_note_off(&msg).unwrap();
        assert_eq!(parsed, note_off);
    }

    #[test]
    fn note_off_roundtrip_mute_true() {
        let note_off = PolyChainNoteOff {
            voice_slot: U2::new(1).unwrap(),
            mute: true,
        };
        let msg = build_note_off(ch(7), &note_off);
        let parsed = parse_note_off(&msg).unwrap();
        assert_eq!(parsed, note_off);
    }

    #[test]
    fn note_off_all_voice_slots() {
        for slot in 0..=3 {
            let note_off = PolyChainNoteOff {
                voice_slot: U2::new(slot).unwrap(),
                mute: slot % 2 == 0,
            };
            let msg = build_note_off(ch(0), &note_off);
            let parsed = parse_note_off(&msg).unwrap();
            assert_eq!(parsed.voice_slot.value(), slot);
            assert_eq!(parsed.mute, slot % 2 == 0);
        }
    }

    // ---------------------------------------------------------------
    // Error cases
    // ---------------------------------------------------------------

    #[test]
    fn parse_note_on_too_short() {
        let msg = vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x60, 0xF7];
        assert!(parse_note_on(&msg).is_err());
    }

    #[test]
    fn parse_note_on_wrong_function_id() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: 0,
        };
        let mut msg = build_note_on(ch(0), &note_on).unwrap();
        msg[6] = frame::POLY_CHAIN_NOTE_OFF; // wrong
        assert!(parse_note_on(&msg).is_err());
    }

    #[test]
    fn parse_note_on_wrong_manufacturer() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: 0,
        };
        let mut msg = build_note_on(ch(0), &note_on).unwrap();
        msg[1] = 0x43; // Yamaha
        assert!(parse_note_on(&msg).is_err());
    }

    #[test]
    fn parse_note_on_missing_f7() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: 0,
        };
        let mut msg = build_note_on(ch(0), &note_on).unwrap();
        let last = msg.len() - 1;
        msg[last] = 0x00;
        assert!(parse_note_on(&msg).is_err());
    }

    #[test]
    fn parse_note_on_invalid_voice_slot() {
        let msg = vec![
            0xF0,
            KORG_ID,
            0x30,
            DEVICE_ID[0],
            DEVICE_ID[1],
            DEVICE_ID[2],
            frame::POLY_CHAIN_NOTE_ON,
            5, // invalid voice slot
            60,
            100,
            0,
            0,
            0,
            0xF7,
        ];
        assert!(parse_note_on(&msg).is_err());
    }

    #[test]
    fn parse_note_off_too_short() {
        let msg = vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x61, 0xF7];
        assert!(parse_note_off(&msg).is_err());
    }

    #[test]
    fn parse_note_off_wrong_function_id() {
        let note_off = PolyChainNoteOff {
            voice_slot: U2::new(0).unwrap(),
            mute: false,
        };
        let mut msg = build_note_off(ch(0), &note_off);
        msg[6] = frame::POLY_CHAIN_NOTE_ON; // wrong
        assert!(parse_note_off(&msg).is_err());
    }

    #[test]
    fn parse_note_off_missing_f7() {
        let note_off = PolyChainNoteOff {
            voice_slot: U2::new(0).unwrap(),
            mute: false,
        };
        let mut msg = build_note_off(ch(0), &note_off);
        let last = msg.len() - 1;
        msg[last] = 0x00;
        assert!(parse_note_off(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // Trait derivations
    // ---------------------------------------------------------------

    #[test]
    fn note_on_debug_clone_copy() {
        let n = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: 0,
        };
        let n2 = n; // Copy
        assert_eq!(n, n2);
        let _dbg = format!("{n:?}");
    }

    #[test]
    fn note_off_debug_clone_copy() {
        let n = PolyChainNoteOff {
            voice_slot: U2::new(0).unwrap(),
            mute: true,
        };
        let n2 = n; // Copy
        assert_eq!(n, n2);
        let _dbg = format!("{n:?}");
    }

    #[test]
    fn pitch_zero_encoding() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(0).unwrap(),
            velocity: U7::new(0).unwrap(),
            pitch: 0,
        };
        let msg = build_note_on(ch(0), &note_on).unwrap();
        assert_eq!(msg[10], 0); // H
        assert_eq!(msg[11], 0); // M
        assert_eq!(msg[12], 0); // L
    }

    #[test]
    fn pitch_max_encoding() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(0).unwrap(),
            velocity: U7::new(0).unwrap(),
            pitch: PITCH_MAX,
        };
        let msg = build_note_on(ch(0), &note_on).unwrap();
        assert_eq!(msg[10], 0x7F); // H: all 7 bits set
        assert_eq!(msg[11], 0x7F); // M: all 7 bits set
        assert_eq!(msg[12], 0x7F); // L: all 7 bits set
    }

    #[test]
    fn build_note_on_pitch_out_of_range() {
        let note_on = PolyChainNoteOn {
            voice_slot: U2::new(0).unwrap(),
            note: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
            pitch: PITCH_MAX + 1,
        };
        assert!(build_note_on(ch(0), &note_on).is_err());
    }
}
