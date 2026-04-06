//! Device identity messages (Universal Non-Realtime and Korg Search Device).
//!
//! These messages are used to discover and identify connected Minilogue XD
//! devices on the MIDI bus.

use crate::error::{Result, SysexError};
use crate::message::types::U4;

// ---------------------------------------------------------------------------
// Universal Non-Realtime Identity Request / Reply
// ---------------------------------------------------------------------------

/// Build a Universal Non-Realtime Identity Request message.
///
/// Format: `[F0, 7E, channel, 06, 01, F7]`
///
/// The `channel` parameter is the MIDI channel (0--15). To address all
/// devices, use channel 0x7F (127), but since [`U4`] is limited to 0--15,
/// this function accepts a `U4` for type safety.
pub fn build_identity_request(channel: U4) -> Vec<u8> {
    vec![0xF0, 0x7E, channel.value(), 0x06, 0x01, 0xF7]
}

/// A parsed Universal Non-Realtime Identity Reply.
///
/// Format: `[F0, 7E, channel, 06, 02, manufacturer_id, family_lo, family_hi,
///           member_lo, member_hi, ver0, ver1, ver2, ver3, F7]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityReply {
    /// MIDI channel from the reply.
    pub channel: U4,
    /// Manufacturer ID (single byte; 0x42 for Korg).
    pub manufacturer_id: u8,
    /// Device family ID (little-endian u16).
    pub family_id: u16,
    /// Device member ID (little-endian u16).
    pub member_id: u16,
    /// Firmware version bytes `[major, minor, micro, build]`.
    pub version: [u8; 4],
}

/// Expected length of a Universal Identity Reply message.
const IDENTITY_REPLY_LEN: usize = 15;

/// Parse a Universal Non-Realtime Identity Reply message.
///
/// # Errors
///
/// Returns an error if the message is malformed or too short.
pub fn parse_identity_reply(bytes: &[u8]) -> Result<IdentityReply> {
    if bytes.len() < IDENTITY_REPLY_LEN {
        return Err(SysexError::PayloadTooShort {
            expected: IDENTITY_REPLY_LEN,
            actual: bytes.len(),
        }
        .into());
    }

    if bytes[0] != 0xF0 {
        return Err(SysexError::InvalidHeader(format!(
            "expected F0 start, got 0x{:02X}",
            bytes[0]
        ))
        .into());
    }
    if bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader(format!(
            "expected F7 end, got 0x{:02X}",
            bytes[bytes.len() - 1]
        ))
        .into());
    }
    if bytes[1] != 0x7E {
        return Err(SysexError::InvalidHeader(format!(
            "expected 0x7E (non-realtime), got 0x{:02X}",
            bytes[1]
        ))
        .into());
    }
    if bytes[3] != 0x06 || bytes[4] != 0x02 {
        return Err(SysexError::InvalidHeader(format!(
            "expected Identity Reply sub-IDs [06, 02], got [0x{:02X}, 0x{:02X}]",
            bytes[3], bytes[4]
        ))
        .into());
    }

    let channel = U4::new(bytes[2]).map_err(|_| {
        SysexError::InvalidHeader(format!("channel byte out of range: 0x{:02X}", bytes[2]))
    })?;

    let manufacturer_id = bytes[5];
    let family_id = u16::from(bytes[6]) | (u16::from(bytes[7]) << 8);
    let member_id = u16::from(bytes[8]) | (u16::from(bytes[9]) << 8);
    let version = [bytes[10], bytes[11], bytes[12], bytes[13]];

    Ok(IdentityReply {
        channel,
        manufacturer_id,
        family_id,
        member_id,
        version,
    })
}

// ---------------------------------------------------------------------------
// Korg Search Device Request / Reply
// ---------------------------------------------------------------------------

/// Build a Korg Search Device Request message.
///
/// Format: `[F0, 42, 50, 00, echo_id, F7]`
///
/// The `echo_id` is echoed back in the reply, allowing the host to correlate
/// requests with responses.
pub fn build_search_device(echo_id: u8) -> Vec<u8> {
    vec![0xF0, 0x42, 0x50, 0x00, echo_id, 0xF7]
}

/// A parsed Korg Search Device Reply.
///
/// Format: `[F0, 42, 50, 01, echo_id, channel, family_lo, family_hi,
///           member_lo, member_hi, ver0, ver1, ver2, ver3, F7]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDeviceReply {
    /// The echo ID from the original request.
    pub echo_id: u8,
    /// MIDI channel the device is configured to use.
    pub channel: U4,
    /// Device family ID (little-endian u16).
    pub family_id: u16,
    /// Device member ID (little-endian u16).
    pub member_id: u16,
    /// Firmware version bytes `[major, minor, micro, build]`.
    pub version: [u8; 4],
}

/// Expected length of a Korg Search Device Reply message.
const SEARCH_REPLY_LEN: usize = 15;

/// Parse a Korg Search Device Reply message.
///
/// # Errors
///
/// Returns an error if the message is malformed or too short.
pub fn parse_search_device_reply(bytes: &[u8]) -> Result<SearchDeviceReply> {
    if bytes.len() < SEARCH_REPLY_LEN {
        return Err(SysexError::PayloadTooShort {
            expected: SEARCH_REPLY_LEN,
            actual: bytes.len(),
        }
        .into());
    }

    if bytes[0] != 0xF0 {
        return Err(SysexError::InvalidHeader(format!(
            "expected F0 start, got 0x{:02X}",
            bytes[0]
        ))
        .into());
    }
    if bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader(format!(
            "expected F7 end, got 0x{:02X}",
            bytes[bytes.len() - 1]
        ))
        .into());
    }
    if bytes[1] != 0x42 {
        return Err(SysexError::InvalidHeader(format!(
            "expected Korg ID 0x42, got 0x{:02X}",
            bytes[1]
        ))
        .into());
    }
    if bytes[2] != 0x50 {
        return Err(SysexError::InvalidHeader(format!(
            "expected 0x50 (search device), got 0x{:02X}",
            bytes[2]
        ))
        .into());
    }
    if bytes[3] != 0x01 {
        return Err(SysexError::InvalidHeader(format!(
            "expected 0x01 (reply), got 0x{:02X}",
            bytes[3]
        ))
        .into());
    }

    let echo_id = bytes[4];

    let channel = U4::new(bytes[5]).map_err(|_| {
        SysexError::InvalidHeader(format!("channel byte out of range: 0x{:02X}", bytes[5]))
    })?;

    let family_id = u16::from(bytes[6]) | (u16::from(bytes[7]) << 8);
    let member_id = u16::from(bytes[8]) | (u16::from(bytes[9]) << 8);
    let version = [bytes[10], bytes[11], bytes[12], bytes[13]];

    Ok(SearchDeviceReply {
        echo_id,
        channel,
        family_id,
        member_id,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    // ---------------------------------------------------------------
    // Identity Request
    // ---------------------------------------------------------------

    #[test]
    fn build_identity_request_ch0() {
        let msg = build_identity_request(ch(0));
        assert_eq!(msg, vec![0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7]);
    }

    #[test]
    fn build_identity_request_ch15() {
        let msg = build_identity_request(ch(15));
        assert_eq!(msg, vec![0xF0, 0x7E, 0x0F, 0x06, 0x01, 0xF7]);
    }

    // ---------------------------------------------------------------
    // Identity Reply parsing
    // ---------------------------------------------------------------

    #[test]
    fn parse_identity_reply_valid() {
        let msg = vec![
            0xF0, 0x7E, 0x00, 0x06, 0x02, // header
            0x42, // manufacturer (Korg)
            0x51, 0x01, // family ID = 0x0151
            0x00, 0x00, // member ID = 0x0000
            0x02, 0x01, 0x00, 0x00, // version
            0xF7,
        ];
        let reply = parse_identity_reply(&msg).unwrap();
        assert_eq!(reply.channel, ch(0));
        assert_eq!(reply.manufacturer_id, 0x42);
        assert_eq!(reply.family_id, 0x0151);
        assert_eq!(reply.member_id, 0x0000);
        assert_eq!(reply.version, [0x02, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn parse_identity_reply_channel_5() {
        let msg = vec![
            0xF0, 0x7E, 0x05, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        let reply = parse_identity_reply(&msg).unwrap();
        assert_eq!(reply.channel, ch(5));
    }

    #[test]
    fn parse_identity_reply_too_short() {
        let msg = vec![0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0xF7];
        assert!(parse_identity_reply(&msg).is_err());
    }

    #[test]
    fn parse_identity_reply_bad_start() {
        let msg = vec![
            0x00, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        assert!(parse_identity_reply(&msg).is_err());
    }

    #[test]
    fn parse_identity_reply_bad_end() {
        let msg = vec![
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00,
        ];
        assert!(parse_identity_reply(&msg).is_err());
    }

    #[test]
    fn parse_identity_reply_bad_sub_id() {
        let msg = vec![
            0xF0, 0x7E, 0x00, 0x06, 0x01, // sub-ID 01 (request, not reply)
            0x42, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xF7,
        ];
        assert!(parse_identity_reply(&msg).is_err());
    }

    #[test]
    fn parse_identity_reply_bad_nonrealtime_id() {
        let msg = vec![
            0xF0, 0x7F, 0x00, 0x06, 0x02, 0x42, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        assert!(parse_identity_reply(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // Search Device Request
    // ---------------------------------------------------------------

    #[test]
    fn build_search_device_echo_0() {
        let msg = build_search_device(0x00);
        assert_eq!(msg, vec![0xF0, 0x42, 0x50, 0x00, 0x00, 0xF7]);
    }

    #[test]
    fn build_search_device_echo_42() {
        let msg = build_search_device(0x42);
        assert_eq!(msg, vec![0xF0, 0x42, 0x50, 0x00, 0x42, 0xF7]);
    }

    // ---------------------------------------------------------------
    // Search Device Reply parsing
    // ---------------------------------------------------------------

    #[test]
    fn parse_search_device_reply_valid() {
        let msg = vec![
            0xF0, 0x42, 0x50, 0x01, // header
            0x7F, // echo_id
            0x00, // channel
            0x51, 0x01, // family
            0x00, 0x00, // member
            0x02, 0x01, 0x00, 0x00, // version
            0xF7,
        ];
        let reply = parse_search_device_reply(&msg).unwrap();
        assert_eq!(reply.echo_id, 0x7F);
        assert_eq!(reply.channel, ch(0));
        assert_eq!(reply.family_id, 0x0151);
        assert_eq!(reply.member_id, 0x0000);
        assert_eq!(reply.version, [0x02, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn parse_search_device_reply_channel_10() {
        let msg = vec![
            0xF0, 0x42, 0x50, 0x01, 0x00, 0x0A, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        let reply = parse_search_device_reply(&msg).unwrap();
        assert_eq!(reply.channel, ch(10));
    }

    #[test]
    fn parse_search_device_reply_too_short() {
        let msg = vec![0xF0, 0x42, 0x50, 0x01, 0x00, 0xF7];
        assert!(parse_search_device_reply(&msg).is_err());
    }

    #[test]
    fn parse_search_device_reply_bad_start() {
        let msg = vec![
            0x00, 0x42, 0x50, 0x01, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        assert!(parse_search_device_reply(&msg).is_err());
    }

    #[test]
    fn parse_search_device_reply_bad_end() {
        let msg = vec![
            0xF0, 0x42, 0x50, 0x01, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x00,
        ];
        assert!(parse_search_device_reply(&msg).is_err());
    }

    #[test]
    fn parse_search_device_reply_wrong_korg_id() {
        let msg = vec![
            0xF0, 0x43, 0x50, 0x01, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        assert!(parse_search_device_reply(&msg).is_err());
    }

    #[test]
    fn parse_search_device_reply_wrong_search_byte() {
        let msg = vec![
            0xF0, 0x42, 0x51, 0x01, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        assert!(parse_search_device_reply(&msg).is_err());
    }

    #[test]
    fn parse_search_device_reply_wrong_reply_byte() {
        let msg = vec![
            0xF0, 0x42, 0x50, 0x00, 0x00, 0x00, 0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0xF7,
        ];
        assert!(parse_search_device_reply(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // IdentityReply Debug + Clone + Eq
    // ---------------------------------------------------------------

    #[test]
    fn identity_reply_debug_clone_eq() {
        let reply = IdentityReply {
            channel: ch(0),
            manufacturer_id: 0x42,
            family_id: 0x0151,
            member_id: 0x0000,
            version: [1, 0, 0, 0],
        };
        let cloned = reply.clone();
        assert_eq!(reply, cloned);
        let _dbg = format!("{reply:?}");
    }

    #[test]
    fn search_device_reply_debug_clone_eq() {
        let reply = SearchDeviceReply {
            echo_id: 0x42,
            channel: ch(5),
            family_id: 0x0151,
            member_id: 0x0000,
            version: [2, 1, 0, 0],
        };
        let cloned = reply.clone();
        assert_eq!(reply, cloned);
        let _dbg = format!("{reply:?}");
    }

    // ---------------------------------------------------------------
    // Search device reply with channel out of range for U4
    // ---------------------------------------------------------------

    #[test]
    fn parse_search_device_reply_channel_out_of_range() {
        let msg = vec![
            0xF0, 0x42, 0x50, 0x01, 0x00, 0x10, // channel = 16, out of U4 range
            0x51, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xF7,
        ];
        assert!(parse_search_device_reply(&msg).is_err());
    }
}
