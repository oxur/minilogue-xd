//! SysEx frame builder and parser for the Korg Minilogue XD.
//!
//! The Korg SysEx header format is:
//!
//! ```text
//! [F0, 42, 3g, 00, 01, 51, function_id, ...payload..., F7]
//! ```
//!
//! where `g` = global channel (0--15), so byte 2 = `0x30 | channel`.

use std::fmt;

use crate::codec;
use crate::error::{Result, SysexError};
use crate::message::types::U4;
use crate::sysex::{DEVICE_ID, KORG_ID};

// ---------------------------------------------------------------------------
// Function ID constants — requests (host -> device)
// ---------------------------------------------------------------------------

/// Request the global data blob (TABLE 1).
pub const GLOBAL_DATA_REQUEST: u8 = 0x0E;
/// Request the current program data.
pub const CURRENT_PROGRAM_REQUEST: u8 = 0x10;
/// Request the user scale data.
pub const USER_SCALE_REQUEST: u8 = 0x14;
/// Request the user octave data.
pub const USER_OCTAVE_REQUEST: u8 = 0x15;
/// Request the user API version.
pub const USER_API_VERSION_REQUEST: u8 = 0x17;
/// Request user module info.
pub const USER_MODULE_INFO_REQUEST: u8 = 0x18;
/// Request user slot status.
pub const USER_SLOT_STATUS_REQUEST: u8 = 0x19;
/// Request user slot data.
pub const USER_SLOT_DATA_REQUEST: u8 = 0x1A;
/// Clear a user slot.
pub const CLEAR_USER_SLOT: u8 = 0x1B;
/// Request a specific program data dump by number.
pub const PROGRAM_DATA_REQUEST: u8 = 0x1C;
/// Clear a user module.
pub const CLEAR_USER_MODULE: u8 = 0x1D;
/// Swap user data.
pub const SWAP_USER_DATA: u8 = 0x1E;

// ---------------------------------------------------------------------------
// Function ID constants — dumps / replies (device -> host)
// ---------------------------------------------------------------------------

/// Current program data dump.
pub const CURRENT_PROGRAM_DUMP: u8 = 0x40;
/// User scale data dump.
pub const USER_SCALE_DUMP: u8 = 0x44;
/// User octave data dump.
pub const USER_OCTAVE_DUMP: u8 = 0x45;
/// User API version reply.
pub const USER_API_VERSION_REPLY: u8 = 0x47;
/// User module info reply.
pub const USER_MODULE_INFO_REPLY: u8 = 0x48;
/// User slot status reply.
pub const USER_SLOT_STATUS_REPLY: u8 = 0x49;
/// User slot data reply.
pub const USER_SLOT_DATA_REPLY: u8 = 0x4A;
/// Program data dump (specific program number).
pub const PROGRAM_DATA_DUMP: u8 = 0x4C;
/// Global data dump (TABLE 1).
pub const GLOBAL_DATA_DUMP: u8 = 0x51;

// ---------------------------------------------------------------------------
// Function ID constants — poly chain
// ---------------------------------------------------------------------------

/// Poly chain note-on.
pub const POLY_CHAIN_NOTE_ON: u8 = 0x60;
/// Poly chain note-off.
pub const POLY_CHAIN_NOTE_OFF: u8 = 0x61;

// ---------------------------------------------------------------------------
// SysexStatus — ACK / NAK codes (NOTE 2 of the spec)
// ---------------------------------------------------------------------------

/// Status codes returned by the device as ACK or error responses.
///
/// These appear as the function-ID byte in a minimal SysEx frame
/// (`[F0, 42, 3g, 00, 01, 51, status_byte, F7]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SysexStatus {
    /// Data load completed successfully (0x23).
    DataLoadCompleted,
    /// Data load error (0x24).
    DataLoadError,
    /// Data format error (0x26).
    DataFormatError,
    /// User data size error (0x27).
    UserDataSizeError,
    /// User data CRC error (0x28).
    UserDataCrcError,
    /// User target error (0x29).
    UserTargetError,
    /// User API error (0x2A).
    UserApiError,
    /// User load size error (0x2B).
    UserLoadSizeError,
    /// User module error (0x2C).
    UserModuleError,
    /// User slot error (0x2D).
    UserSlotError,
    /// User format error (0x2E).
    UserFormatError,
    /// User internal error (0x2F).
    UserInternalError,
}

impl SysexStatus {
    /// Parse a status byte into a [`SysexStatus`], if recognized.
    ///
    /// Returns `None` for unrecognized bytes.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x23 => Some(Self::DataLoadCompleted),
            0x24 => Some(Self::DataLoadError),
            0x26 => Some(Self::DataFormatError),
            0x27 => Some(Self::UserDataSizeError),
            0x28 => Some(Self::UserDataCrcError),
            0x29 => Some(Self::UserTargetError),
            0x2A => Some(Self::UserApiError),
            0x2B => Some(Self::UserLoadSizeError),
            0x2C => Some(Self::UserModuleError),
            0x2D => Some(Self::UserSlotError),
            0x2E => Some(Self::UserFormatError),
            0x2F => Some(Self::UserInternalError),
            _ => None,
        }
    }

    /// Convert this status to its wire byte value.
    pub fn to_byte(self) -> u8 {
        match self {
            Self::DataLoadCompleted => 0x23,
            Self::DataLoadError => 0x24,
            Self::DataFormatError => 0x26,
            Self::UserDataSizeError => 0x27,
            Self::UserDataCrcError => 0x28,
            Self::UserTargetError => 0x29,
            Self::UserApiError => 0x2A,
            Self::UserLoadSizeError => 0x2B,
            Self::UserModuleError => 0x2C,
            Self::UserSlotError => 0x2D,
            Self::UserFormatError => 0x2E,
            Self::UserInternalError => 0x2F,
        }
    }

    /// Returns `true` if this status represents a successful acknowledgement.
    pub fn is_ack(self) -> bool {
        matches!(self, Self::DataLoadCompleted)
    }

    /// Returns `true` if this status represents an error condition.
    pub fn is_error(self) -> bool {
        !self.is_ack()
    }
}

impl fmt::Display for SysexStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataLoadCompleted => write!(f, "Data Load Completed"),
            Self::DataLoadError => write!(f, "Data Load Error"),
            Self::DataFormatError => write!(f, "Data Format Error"),
            Self::UserDataSizeError => write!(f, "User Data Size Error"),
            Self::UserDataCrcError => write!(f, "User Data CRC Error"),
            Self::UserTargetError => write!(f, "User Target Error"),
            Self::UserApiError => write!(f, "User API Error"),
            Self::UserLoadSizeError => write!(f, "User Load Size Error"),
            Self::UserModuleError => write!(f, "User Module Error"),
            Self::UserSlotError => write!(f, "User Slot Error"),
            Self::UserFormatError => write!(f, "User Format Error"),
            Self::UserInternalError => write!(f, "User Internal Error"),
        }
    }
}

// ---------------------------------------------------------------------------
// SysexFrame
// ---------------------------------------------------------------------------

/// A parsed Korg Minilogue XD SysEx frame.
///
/// The `data` field contains the **decoded 8-bit payload** (after 7-bit
/// decoding). For request frames (no payload), `data` will be empty.
#[derive(Debug, Clone, PartialEq)]
pub struct SysexFrame {
    /// MIDI channel (0--15).
    pub channel: U4,
    /// The function ID byte from the frame header.
    pub function_id: u8,
    /// Decoded 8-bit payload data (may be empty for requests/status messages).
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Minimum frame length: F0 42 3g 00 01 51 FN F7 = 8 bytes
// ---------------------------------------------------------------------------
const MIN_FRAME_LEN: usize = 8;

/// Build a complete SysEx message with an 8-bit data payload.
///
/// The payload is encoded via the Korg 7-bit codec before being placed in
/// the frame. The result is a complete MIDI SysEx message:
///
/// ```text
/// [F0, 42, 3g, 00, 01, 51, function_id, ...7-bit payload..., F7]
/// ```
///
/// If `data_8bit` is empty, only the header and trailer are emitted (same
/// as [`build_sysex_request`]).
pub fn build_sysex(channel: U4, function_id: u8, data_8bit: &[u8]) -> Vec<u8> {
    let encoded = codec::encode_7bit(data_8bit);
    let mut msg = Vec::with_capacity(7 + encoded.len() + 1);
    msg.push(0xF0);
    msg.push(KORG_ID);
    msg.push(0x30 | channel.value());
    msg.extend_from_slice(&DEVICE_ID);
    msg.push(function_id);
    msg.extend_from_slice(&encoded);
    msg.push(0xF7);
    msg
}

/// Build a SysEx request message (no data payload).
///
/// ```text
/// [F0, 42, 3g, 00, 01, 51, function_id, F7]
/// ```
pub fn build_sysex_request(channel: U4, function_id: u8) -> Vec<u8> {
    vec![
        0xF0,
        KORG_ID,
        0x30 | channel.value(),
        DEVICE_ID[0],
        DEVICE_ID[1],
        DEVICE_ID[2],
        function_id,
        0xF7,
    ]
}

/// Build a SysEx status message (ACK/NAK).
///
/// ```text
/// [F0, 42, 3g, 00, 01, 51, status_byte, F7]
/// ```
pub fn build_status(channel: U4, status: SysexStatus) -> Vec<u8> {
    build_sysex_request(channel, status.to_byte())
}

/// Parse raw SysEx bytes into a [`SysexFrame`].
///
/// Validates the Korg Minilogue XD header structure and decodes the 7-bit
/// payload back to 8-bit data.
///
/// # Errors
///
/// Returns [`SysexError`](crate::error::SysexError) (wrapped in
/// [`Error::Sysex`](crate::Error::Sysex)) if the message is malformed.
pub fn parse_sysex(bytes: &[u8]) -> Result<SysexFrame> {
    // Check minimum length.
    if bytes.len() < MIN_FRAME_LEN {
        return Err(SysexError::InvalidHeader(format!(
            "message too short: {} bytes (minimum {})",
            bytes.len(),
            MIN_FRAME_LEN
        ))
        .into());
    }

    // Check start/end markers.
    if bytes[0] != 0xF0 {
        return Err(SysexError::InvalidHeader(format!(
            "expected F0 start byte, got 0x{:02X}",
            bytes[0]
        ))
        .into());
    }
    let last = bytes[bytes.len() - 1];
    if last != 0xF7 {
        return Err(
            SysexError::InvalidHeader(format!("expected F7 end byte, got 0x{last:02X}")).into(),
        );
    }

    // Check Korg manufacturer ID.
    if bytes[1] != KORG_ID {
        return Err(SysexError::InvalidHeader(format!(
            "expected Korg ID 0x{KORG_ID:02X}, got 0x{:02X}",
            bytes[1]
        ))
        .into());
    }

    // Extract channel: byte 2 must be 0x30..=0x3F.
    let ch_byte = bytes[2];
    if ch_byte & 0xF0 != 0x30 {
        return Err(SysexError::InvalidHeader(format!(
            "expected channel byte 0x30..0x3F, got 0x{ch_byte:02X}"
        ))
        .into());
    }
    // Safety: ch_byte & 0x0F is always 0..=15.
    let channel = U4::new(ch_byte & 0x0F).map_err(|_| {
        SysexError::InvalidHeader(format!("channel nibble out of range: 0x{ch_byte:02X}"))
    })?;

    // Check device ID bytes.
    if bytes[3..6] != DEVICE_ID {
        return Err(SysexError::InvalidHeader(format!(
            "expected device ID {:02X?}, got {:02X?}",
            DEVICE_ID,
            &bytes[3..6]
        ))
        .into());
    }

    let function_id = bytes[6];

    // Payload is everything between the function ID and the trailing F7.
    let payload_wire = &bytes[7..bytes.len() - 1];

    // Decode the 7-bit payload. If the payload is empty (request/status),
    // decode_7bit will return an empty Vec.
    let data = codec::decode_7bit(payload_wire)?;

    Ok(SysexFrame {
        channel,
        function_id,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    // ---------------------------------------------------------------
    // SysexStatus
    // ---------------------------------------------------------------

    #[test]
    fn status_round_trip_all_variants() {
        let variants = [
            (0x23, SysexStatus::DataLoadCompleted),
            (0x24, SysexStatus::DataLoadError),
            (0x26, SysexStatus::DataFormatError),
            (0x27, SysexStatus::UserDataSizeError),
            (0x28, SysexStatus::UserDataCrcError),
            (0x29, SysexStatus::UserTargetError),
            (0x2A, SysexStatus::UserApiError),
            (0x2B, SysexStatus::UserLoadSizeError),
            (0x2C, SysexStatus::UserModuleError),
            (0x2D, SysexStatus::UserSlotError),
            (0x2E, SysexStatus::UserFormatError),
            (0x2F, SysexStatus::UserInternalError),
        ];
        for (byte, expected) in &variants {
            let parsed = SysexStatus::from_byte(*byte);
            assert_eq!(parsed, Some(*expected), "from_byte(0x{byte:02X})");
            assert_eq!(expected.to_byte(), *byte, "to_byte for {expected}");
        }
    }

    #[test]
    fn status_from_byte_unknown() {
        assert_eq!(SysexStatus::from_byte(0x00), None);
        assert_eq!(SysexStatus::from_byte(0x25), None);
        assert_eq!(SysexStatus::from_byte(0x30), None);
        assert_eq!(SysexStatus::from_byte(0xFF), None);
    }

    #[test]
    fn status_is_ack() {
        assert!(SysexStatus::DataLoadCompleted.is_ack());
        assert!(!SysexStatus::DataLoadCompleted.is_error());
    }

    #[test]
    fn status_is_error() {
        let errors = [
            SysexStatus::DataLoadError,
            SysexStatus::DataFormatError,
            SysexStatus::UserDataSizeError,
            SysexStatus::UserDataCrcError,
            SysexStatus::UserTargetError,
            SysexStatus::UserApiError,
            SysexStatus::UserLoadSizeError,
            SysexStatus::UserModuleError,
            SysexStatus::UserSlotError,
            SysexStatus::UserFormatError,
            SysexStatus::UserInternalError,
        ];
        for s in &errors {
            assert!(s.is_error(), "{s} should be an error");
            assert!(!s.is_ack(), "{s} should not be ACK");
        }
    }

    #[test]
    fn status_display() {
        assert_eq!(
            SysexStatus::DataLoadCompleted.to_string(),
            "Data Load Completed"
        );
        assert_eq!(SysexStatus::DataLoadError.to_string(), "Data Load Error");
        assert_eq!(
            SysexStatus::DataFormatError.to_string(),
            "Data Format Error"
        );
        assert_eq!(
            SysexStatus::UserDataSizeError.to_string(),
            "User Data Size Error"
        );
        assert_eq!(
            SysexStatus::UserDataCrcError.to_string(),
            "User Data CRC Error"
        );
        assert_eq!(
            SysexStatus::UserTargetError.to_string(),
            "User Target Error"
        );
        assert_eq!(SysexStatus::UserApiError.to_string(), "User API Error");
        assert_eq!(
            SysexStatus::UserLoadSizeError.to_string(),
            "User Load Size Error"
        );
        assert_eq!(
            SysexStatus::UserModuleError.to_string(),
            "User Module Error"
        );
        assert_eq!(SysexStatus::UserSlotError.to_string(), "User Slot Error");
        assert_eq!(
            SysexStatus::UserFormatError.to_string(),
            "User Format Error"
        );
        assert_eq!(
            SysexStatus::UserInternalError.to_string(),
            "User Internal Error"
        );
    }

    // ---------------------------------------------------------------
    // build_sysex_request
    // ---------------------------------------------------------------

    #[test]
    fn build_request_global_ch0() {
        let msg = build_sysex_request(ch(0), GLOBAL_DATA_REQUEST);
        assert_eq!(msg, vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x0E, 0xF7]);
    }

    #[test]
    fn build_request_current_program_ch15() {
        let msg = build_sysex_request(ch(15), CURRENT_PROGRAM_REQUEST);
        assert_eq!(msg, vec![0xF0, 0x42, 0x3F, 0x00, 0x01, 0x51, 0x10, 0xF7]);
    }

    #[test]
    fn build_request_various_function_ids() {
        let ids = [
            USER_SCALE_REQUEST,
            USER_OCTAVE_REQUEST,
            USER_API_VERSION_REQUEST,
            USER_MODULE_INFO_REQUEST,
            USER_SLOT_STATUS_REQUEST,
            USER_SLOT_DATA_REQUEST,
            CLEAR_USER_SLOT,
            PROGRAM_DATA_REQUEST,
            CLEAR_USER_MODULE,
            SWAP_USER_DATA,
        ];
        for id in ids {
            let msg = build_sysex_request(ch(5), id);
            assert_eq!(msg.len(), 8);
            assert_eq!(msg[0], 0xF0);
            assert_eq!(msg[6], id);
            assert_eq!(msg[7], 0xF7);
        }
    }

    // ---------------------------------------------------------------
    // build_status
    // ---------------------------------------------------------------

    #[test]
    fn build_status_ack() {
        let msg = build_status(ch(0), SysexStatus::DataLoadCompleted);
        assert_eq!(msg, vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x23, 0xF7]);
    }

    #[test]
    fn build_status_error() {
        let msg = build_status(ch(3), SysexStatus::DataFormatError);
        assert_eq!(msg[6], 0x26);
    }

    // ---------------------------------------------------------------
    // build_sysex with payload
    // ---------------------------------------------------------------

    #[test]
    fn build_sysex_empty_payload() {
        let msg = build_sysex(ch(0), 0x51, &[]);
        // Should be identical to a request frame.
        assert_eq!(msg, vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x51, 0xF7]);
    }

    #[test]
    fn build_sysex_with_payload() {
        let data = [0x01, 0x02, 0x03];
        let msg = build_sysex(ch(0), GLOBAL_DATA_DUMP, &data);
        // Header: F0 42 30 00 01 51 51 = 7 bytes
        // Payload: encode_7bit([01, 02, 03]) = [0x00, 0x01, 0x02, 0x03] (4 bytes, no high bits)
        // Trailer: F7
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[6], GLOBAL_DATA_DUMP);
        assert_eq!(msg[msg.len() - 1], 0xF7);
    }

    // ---------------------------------------------------------------
    // parse_sysex
    // ---------------------------------------------------------------

    #[test]
    fn parse_request_round_trip() {
        let msg = build_sysex_request(ch(7), GLOBAL_DATA_REQUEST);
        let frame = parse_sysex(&msg).unwrap();
        assert_eq!(frame.channel, ch(7));
        assert_eq!(frame.function_id, GLOBAL_DATA_REQUEST);
        assert!(frame.data.is_empty());
    }

    #[test]
    fn parse_sysex_with_payload_round_trip() {
        let data = vec![0xFF, 0x80, 0x00, 0x7F, 0x01, 0x55, 0xAA];
        let msg = build_sysex(ch(3), CURRENT_PROGRAM_DUMP, &data);
        let frame = parse_sysex(&msg).unwrap();
        assert_eq!(frame.channel, ch(3));
        assert_eq!(frame.function_id, CURRENT_PROGRAM_DUMP);
        assert_eq!(frame.data, data);
    }

    #[test]
    fn parse_sysex_large_payload_round_trip() {
        let data: Vec<u8> = (0..=255).collect();
        let msg = build_sysex(ch(0), PROGRAM_DATA_DUMP, &data);
        let frame = parse_sysex(&msg).unwrap();
        assert_eq!(frame.data, data);
    }

    #[test]
    fn parse_status_round_trip() {
        let msg = build_status(ch(10), SysexStatus::UserDataCrcError);
        let frame = parse_sysex(&msg).unwrap();
        assert_eq!(frame.channel, ch(10));
        assert_eq!(frame.function_id, 0x28);
        assert!(frame.data.is_empty());
    }

    #[test]
    fn parse_all_channels() {
        for c in 0..=15 {
            let msg = build_sysex_request(ch(c), GLOBAL_DATA_REQUEST);
            let frame = parse_sysex(&msg).unwrap();
            assert_eq!(frame.channel, ch(c));
        }
    }

    // ---------------------------------------------------------------
    // parse_sysex error cases
    // ---------------------------------------------------------------

    #[test]
    fn parse_too_short() {
        let short = vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0xF7];
        assert!(parse_sysex(&short).is_err());
    }

    #[test]
    fn parse_empty() {
        assert!(parse_sysex(&[]).is_err());
    }

    #[test]
    fn parse_missing_f0() {
        let mut msg = build_sysex_request(ch(0), GLOBAL_DATA_REQUEST);
        msg[0] = 0x00;
        assert!(parse_sysex(&msg).is_err());
    }

    #[test]
    fn parse_missing_f7() {
        let mut msg = build_sysex_request(ch(0), GLOBAL_DATA_REQUEST);
        let last = msg.len() - 1;
        msg[last] = 0x00;
        assert!(parse_sysex(&msg).is_err());
    }

    #[test]
    fn parse_wrong_manufacturer() {
        let mut msg = build_sysex_request(ch(0), GLOBAL_DATA_REQUEST);
        msg[1] = 0x43; // Yamaha, not Korg
        assert!(parse_sysex(&msg).is_err());
    }

    #[test]
    fn parse_wrong_channel_nibble() {
        let mut msg = build_sysex_request(ch(0), GLOBAL_DATA_REQUEST);
        msg[2] = 0x40; // Should be 0x30..0x3F
        assert!(parse_sysex(&msg).is_err());
    }

    #[test]
    fn parse_wrong_device_id() {
        let mut msg = build_sysex_request(ch(0), GLOBAL_DATA_REQUEST);
        msg[4] = 0xFF; // Corrupt device ID
        assert!(parse_sysex(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // Function ID constant value verification
    // ---------------------------------------------------------------

    #[test]
    fn function_id_request_values() {
        assert_eq!(GLOBAL_DATA_REQUEST, 0x0E);
        assert_eq!(CURRENT_PROGRAM_REQUEST, 0x10);
        assert_eq!(USER_SCALE_REQUEST, 0x14);
        assert_eq!(USER_OCTAVE_REQUEST, 0x15);
        assert_eq!(USER_API_VERSION_REQUEST, 0x17);
        assert_eq!(USER_MODULE_INFO_REQUEST, 0x18);
        assert_eq!(USER_SLOT_STATUS_REQUEST, 0x19);
        assert_eq!(USER_SLOT_DATA_REQUEST, 0x1A);
        assert_eq!(CLEAR_USER_SLOT, 0x1B);
        assert_eq!(PROGRAM_DATA_REQUEST, 0x1C);
        assert_eq!(CLEAR_USER_MODULE, 0x1D);
        assert_eq!(SWAP_USER_DATA, 0x1E);
    }

    #[test]
    fn function_id_dump_values() {
        assert_eq!(CURRENT_PROGRAM_DUMP, 0x40);
        assert_eq!(USER_SCALE_DUMP, 0x44);
        assert_eq!(USER_OCTAVE_DUMP, 0x45);
        assert_eq!(USER_API_VERSION_REPLY, 0x47);
        assert_eq!(USER_MODULE_INFO_REPLY, 0x48);
        assert_eq!(USER_SLOT_STATUS_REPLY, 0x49);
        assert_eq!(USER_SLOT_DATA_REPLY, 0x4A);
        assert_eq!(PROGRAM_DATA_DUMP, 0x4C);
        assert_eq!(GLOBAL_DATA_DUMP, 0x51);
    }

    #[test]
    fn function_id_poly_chain_values() {
        assert_eq!(POLY_CHAIN_NOTE_ON, 0x60);
        assert_eq!(POLY_CHAIN_NOTE_OFF, 0x61);
    }

    // ---------------------------------------------------------------
    // SysexFrame Debug + Clone
    // ---------------------------------------------------------------

    #[test]
    fn frame_debug_and_clone() {
        let frame = SysexFrame {
            channel: ch(0),
            function_id: 0x51,
            data: vec![1, 2, 3],
        };
        let cloned = frame.clone();
        assert_eq!(frame, cloned);
        // Verify Debug is implemented.
        let _dbg = format!("{frame:?}");
    }

    // ---------------------------------------------------------------
    // SysexStatus clone, copy, hash
    // ---------------------------------------------------------------

    #[test]
    fn status_clone_copy_hash() {
        use std::collections::HashSet;
        let s = SysexStatus::DataLoadCompleted;
        let s2 = s; // Copy
        assert_eq!(s, s2);
        let mut set = HashSet::new();
        set.insert(s);
        assert!(set.contains(&SysexStatus::DataLoadCompleted));
    }
}
