//! User module management for the logue SDK on the Minilogue XD.
//!
//! The Minilogue XD supports user-loadable modules for oscillators, modulation
//! effects, delay effects, and reverb effects via the logue SDK. This module
//! provides types and SysEx message builders/parsers for querying and managing
//! user module slots.
//!
//! ## Module types and slot counts
//!
//! | Module     | ID | Max slots |
//! |------------|:--:|:---------:|
//! | Mod FX     | 1  | 16        |
//! | Delay FX   | 2  | 8         |
//! | Reverb FX  | 3  | 8         |
//! | Oscillator | 4  | 16        |
//!
//! ## Data structures
//!
//! - **User Module Info** (TABLE 5): 9 bytes 8-bit, contains size limits and
//!   available slot count.
//! - **User Slot Status** (TABLE 6): 32 bytes 8-bit, describes the program
//!   loaded in a slot.
//! - **User Slot Data** (TABLE 7): variable length, contains the actual
//!   program binary with a CRC32 integrity check.

use std::fmt;

use crate::error::{Result, SysexError};
use crate::message::types::U4;
use crate::sysex::frame;
use crate::sysex::{DEVICE_ID, KORG_ID};

// ---------------------------------------------------------------------------
// UserModuleId
// ---------------------------------------------------------------------------

/// User module categories supported by the Minilogue XD logue SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UserModuleId {
    /// Modulation effect (ID 1, 16 slots).
    ModFx = 1,
    /// Delay effect (ID 2, 8 slots).
    DelayFx = 2,
    /// Reverb effect (ID 3, 8 slots).
    ReverbFx = 3,
    /// User oscillator (ID 4, 16 slots).
    Osc = 4,
}

impl UserModuleId {
    /// Returns the maximum number of slots available for this module type.
    pub fn max_slots(self) -> u8 {
        match self {
            Self::ModFx | Self::Osc => 16,
            Self::DelayFx | Self::ReverbFx => 8,
        }
    }

    /// Parse a module ID from a raw byte.
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::InvalidModuleId`] if the byte is not 1--4.
    pub fn from_byte(b: u8) -> Result<Self> {
        match b {
            1 => Ok(Self::ModFx),
            2 => Ok(Self::DelayFx),
            3 => Ok(Self::ReverbFx),
            4 => Ok(Self::Osc),
            _ => Err(SysexError::InvalidModuleId(b).into()),
        }
    }

    /// Convert this module ID to its wire byte value.
    pub fn to_byte(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for UserModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModFx => write!(f, "Mod FX"),
            Self::DelayFx => write!(f, "Delay FX"),
            Self::ReverbFx => write!(f, "Reverb FX"),
            Self::Osc => write!(f, "Oscillator"),
        }
    }
}

// ---------------------------------------------------------------------------
// UserModuleInfo (TABLE 5)
// ---------------------------------------------------------------------------

/// User module info returned by the device (TABLE 5).
///
/// Contains size limits and available slot count for a given module type.
/// The raw 8-bit blob is 9 bytes; on the wire it is 7-bit encoded to 11 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserModuleInfo {
    /// Maximum size in bytes for slot data.
    pub max_slot_size: u32,
    /// Maximum size in bytes for program data.
    pub max_program_size: u32,
    /// Number of available (empty) slots.
    pub available_slot_count: u8,
}

impl UserModuleInfo {
    /// Size of the raw 8-bit blob in bytes.
    pub const BLOB_SIZE: usize = 9;

    /// Parse from a 9-byte 8-bit data slice.
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::PayloadTooShort`] if `data` is shorter than 9 bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::BLOB_SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::BLOB_SIZE,
                actual: data.len(),
            }
            .into());
        }
        let max_slot_size = read_u32_le(data, 0);
        let max_program_size = read_u32_le(data, 4);
        let available_slot_count = data[8];
        Ok(Self {
            max_slot_size,
            max_program_size,
            available_slot_count,
        })
    }

    /// Serialize to a 9-byte blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; Self::BLOB_SIZE];
        write_u32_le(&mut out, 0, self.max_slot_size);
        write_u32_le(&mut out, 4, self.max_program_size);
        out[8] = self.available_slot_count;
        out
    }
}

// ---------------------------------------------------------------------------
// UserSlotStatus (TABLE 6)
// ---------------------------------------------------------------------------

/// Status of a user module slot (TABLE 6).
///
/// Describes the program loaded in a particular slot, including version
/// info, developer ID, program name, etc. The raw 8-bit blob is 32 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSlotStatus {
    /// Platform identifier.
    pub platform_id: u8,
    /// Module type.
    pub module_id: UserModuleId,
    /// API version (major, minor, patch).
    pub api_version: (u8, u8, u8),
    /// Developer ID (u32 LE).
    pub developer_id: u32,
    /// Program ID (u32 LE).
    pub program_id: u32,
    /// Program version (major, minor, patch).
    pub program_version: (u8, u8, u8),
    /// Program name (up to 16 characters, null-terminated ASCII).
    pub program_name: String,
}

impl UserSlotStatus {
    /// Size of the raw 8-bit blob in bytes.
    pub const BLOB_SIZE: usize = 32;

    /// Parse from a 32-byte 8-bit data slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is too short or the module ID is invalid.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::BLOB_SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::BLOB_SIZE,
                actual: data.len(),
            }
            .into());
        }
        let platform_id = data[0];
        let module_id = UserModuleId::from_byte(data[1])?;
        let api_version = (data[2], data[3], data[4]);
        let developer_id = read_u32_le(data, 5);
        let program_id = read_u32_le(data, 9);
        let program_version = (data[13], data[14], data[15]);

        // Extract null-terminated ASCII name from bytes 16..32.
        let name_bytes = &data[16..32];
        let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(16);
        let program_name = String::from_utf8_lossy(&name_bytes[..name_len]).to_string();

        Ok(Self {
            platform_id,
            module_id,
            api_version,
            developer_id,
            program_id,
            program_version,
            program_name,
        })
    }

    /// Serialize to a 32-byte blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; Self::BLOB_SIZE];
        out[0] = self.platform_id;
        out[1] = self.module_id.to_byte();
        out[2] = self.api_version.0;
        out[3] = self.api_version.1;
        out[4] = self.api_version.2;
        write_u32_le(&mut out, 5, self.developer_id);
        write_u32_le(&mut out, 9, self.program_id);
        out[13] = self.program_version.0;
        out[14] = self.program_version.1;
        out[15] = self.program_version.2;

        let name_bytes = self.program_name.as_bytes();
        let copy_len = name_bytes.len().min(16);
        out[16..16 + copy_len].copy_from_slice(&name_bytes[..copy_len]);
        // Remaining bytes are already zero (null termination).
        out
    }
}

// ---------------------------------------------------------------------------
// UserSlotData (TABLE 7)
// ---------------------------------------------------------------------------

/// User slot data payload (TABLE 7).
///
/// Contains the actual user module binary with a CRC32 integrity check.
/// The header is 8 bytes (size + CRC32), followed by the payload data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSlotData {
    /// Declared payload data size in bytes (u32 LE at offset 0).
    pub payload_size: u32,
    /// CRC32 of the payload data (u32 LE at offset 4).
    pub crc32: u32,
    /// The raw payload data.
    pub payload: Vec<u8>,
}

impl UserSlotData {
    /// Minimum header size (payload_size + crc32 = 8 bytes).
    pub const HEADER_SIZE: usize = 8;

    /// Parse from 8-bit data (header + payload).
    ///
    /// # Errors
    ///
    /// Returns an error if the data is too short or the CRC32 does not match.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::HEADER_SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::HEADER_SIZE,
                actual: data.len(),
            }
            .into());
        }
        let payload_size = read_u32_le(data, 0);
        let crc32 = read_u32_le(data, 4);

        let payload_start = Self::HEADER_SIZE;
        let payload_end = payload_start + payload_size as usize;
        if data.len() < payload_end {
            return Err(SysexError::PayloadTooShort {
                expected: payload_end,
                actual: data.len(),
            }
            .into());
        }

        let payload = data[payload_start..payload_end].to_vec();

        // Verify CRC32.
        let computed = compute_crc32(&payload);
        if computed != crc32 {
            return Err(SysexError::Crc32Mismatch {
                expected: crc32,
                actual: computed,
            }
            .into());
        }

        Ok(Self {
            payload_size,
            crc32,
            payload,
        })
    }

    /// Serialize to bytes (header + payload).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::HEADER_SIZE + self.payload.len());
        let mut header = [0u8; 8];
        write_u32_le(&mut header, 0, self.payload_size);
        write_u32_le(&mut header, 4, self.crc32);
        out.extend_from_slice(&header);
        out.extend_from_slice(&self.payload);
        out
    }

    /// Create a new `UserSlotData` from a payload, computing the CRC32.
    pub fn from_payload(payload: Vec<u8>) -> Self {
        let crc32 = compute_crc32(&payload);
        let payload_size = payload.len() as u32;
        Self {
            payload_size,
            crc32,
            payload,
        }
    }
}

// ---------------------------------------------------------------------------
// CRC32
// ---------------------------------------------------------------------------

/// Compute CRC32 of user module data (IEEE polynomial, as used by logue SDK).
pub fn compute_crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

// ---------------------------------------------------------------------------
// Slot validation
// ---------------------------------------------------------------------------

/// Validate that a slot index is within the allowed range for the given module.
///
/// # Errors
///
/// Returns [`SysexError::InvalidSlotIndex`] if `slot >= module.max_slots()`.
fn validate_slot(module: UserModuleId, slot: u8) -> Result<()> {
    if slot >= module.max_slots() {
        return Err(SysexError::InvalidSlotIndex {
            slot,
            max: module.max_slots(),
        }
        .into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SysEx header helpers (raw, not 7-bit encoded)
// ---------------------------------------------------------------------------

/// Build a raw Korg SysEx header: `[F0, 42, 3g, 00, 01, 51]`.
fn sysex_header(channel: U4) -> Vec<u8> {
    vec![
        0xF0,
        KORG_ID,
        0x30 | channel.value(),
        DEVICE_ID[0],
        DEVICE_ID[1],
        DEVICE_ID[2],
    ]
}

// ---------------------------------------------------------------------------
// Request builders
// ---------------------------------------------------------------------------

/// Build a User API Version Request (function 0x17).
pub fn build_api_version_request(channel: U4) -> Vec<u8> {
    frame::build_sysex_request(channel, frame::USER_API_VERSION_REQUEST)
}

/// Build a User Module Info Request (function 0x18).
///
/// The module ID is sent as a raw data byte (not 7-bit encoded).
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 18, mm, F7]`
pub fn build_module_info_request(channel: U4, module: UserModuleId) -> Vec<u8> {
    let mut msg = sysex_header(channel);
    msg.push(frame::USER_MODULE_INFO_REQUEST);
    msg.push(module.to_byte());
    msg.push(0xF7);
    msg
}

/// Build a User Slot Status Request (function 0x19).
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 19, mm, ss, F7]`
///
/// # Errors
///
/// Returns an error if the slot index is out of range.
pub fn build_slot_status_request(channel: U4, module: UserModuleId, slot: u8) -> Result<Vec<u8>> {
    validate_slot(module, slot)?;
    let mut msg = sysex_header(channel);
    msg.push(frame::USER_SLOT_STATUS_REQUEST);
    msg.push(module.to_byte());
    msg.push(slot);
    msg.push(0xF7);
    Ok(msg)
}

/// Build a User Slot Data Request (function 0x1A).
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 1A, mm, ss, F7]`
///
/// # Errors
///
/// Returns an error if the slot index is out of range.
pub fn build_slot_data_request(channel: U4, module: UserModuleId, slot: u8) -> Result<Vec<u8>> {
    validate_slot(module, slot)?;
    let mut msg = sysex_header(channel);
    msg.push(frame::USER_SLOT_DATA_REQUEST);
    msg.push(module.to_byte());
    msg.push(slot);
    msg.push(0xF7);
    Ok(msg)
}

/// Build a Clear User Slot command (function 0x1B).
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 1B, mm, ss, F7]`
///
/// # Errors
///
/// Returns an error if the slot index is out of range.
pub fn build_clear_slot(channel: U4, module: UserModuleId, slot: u8) -> Result<Vec<u8>> {
    validate_slot(module, slot)?;
    let mut msg = sysex_header(channel);
    msg.push(frame::CLEAR_USER_SLOT);
    msg.push(module.to_byte());
    msg.push(slot);
    msg.push(0xF7);
    Ok(msg)
}

/// Build a Clear User Module command (function 0x1D).
///
/// Clears all slots for the given module type.
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 1D, mm, F7]`
pub fn build_clear_module(channel: U4, module: UserModuleId) -> Vec<u8> {
    let mut msg = sysex_header(channel);
    msg.push(frame::CLEAR_USER_MODULE);
    msg.push(module.to_byte());
    msg.push(0xF7);
    msg
}

/// Build a Swap User Data command (function 0x1E).
///
/// Swaps the contents of two slots within the same module type.
///
/// Format: `[F0, 42, 3g, 00, 01, 51, 1E, mm, sa, sb, F7]`
///
/// # Errors
///
/// Returns an error if either slot index is out of range.
pub fn build_swap_slots(
    channel: U4,
    module: UserModuleId,
    slot_a: u8,
    slot_b: u8,
) -> Result<Vec<u8>> {
    validate_slot(module, slot_a)?;
    validate_slot(module, slot_b)?;
    let mut msg = sysex_header(channel);
    msg.push(frame::SWAP_USER_DATA);
    msg.push(module.to_byte());
    msg.push(slot_a);
    msg.push(slot_b);
    msg.push(0xF7);
    Ok(msg)
}

// ---------------------------------------------------------------------------
// Response parsers
// ---------------------------------------------------------------------------

/// Parse a User Module Info Reply (function 0x48).
///
/// The reply contains 7-bit encoded data which is decoded via `parse_sysex`,
/// then the 9-byte 8-bit blob is parsed into a [`UserModuleInfo`].
///
/// # Errors
///
/// Returns an error if the frame is malformed or the function ID is wrong.
pub fn parse_module_info_reply(bytes: &[u8]) -> Result<UserModuleInfo> {
    let parsed = frame::parse_sysex(bytes)?;
    if parsed.function_id != frame::USER_MODULE_INFO_REPLY {
        return Err(SysexError::WrongFunctionId {
            expected: frame::USER_MODULE_INFO_REPLY,
            found: parsed.function_id,
        }
        .into());
    }
    UserModuleInfo::from_bytes(&parsed.data)
}

/// Parse a User Slot Status Reply (function 0x49).
///
/// # Errors
///
/// Returns an error if the frame is malformed or the function ID is wrong.
pub fn parse_slot_status_reply(bytes: &[u8]) -> Result<UserSlotStatus> {
    let parsed = frame::parse_sysex(bytes)?;
    if parsed.function_id != frame::USER_SLOT_STATUS_REPLY {
        return Err(SysexError::WrongFunctionId {
            expected: frame::USER_SLOT_STATUS_REPLY,
            found: parsed.function_id,
        }
        .into());
    }
    UserSlotStatus::from_bytes(&parsed.data)
}

/// Parse a User Slot Data Reply (function 0x4A).
///
/// # Errors
///
/// Returns an error if the frame is malformed, the function ID is wrong,
/// or the CRC32 does not match.
pub fn parse_slot_data_reply(bytes: &[u8]) -> Result<UserSlotData> {
    let parsed = frame::parse_sysex(bytes)?;
    if parsed.function_id != frame::USER_SLOT_DATA_REPLY {
        return Err(SysexError::WrongFunctionId {
            expected: frame::USER_SLOT_DATA_REPLY,
            found: parsed.function_id,
        }
        .into());
    }
    UserSlotData::from_bytes(&parsed.data)
}

// ---------------------------------------------------------------------------
// Helpers for u32 LE
// ---------------------------------------------------------------------------

/// Read a little-endian u32 at `offset`.
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from(data[offset])
        | (u32::from(data[offset + 1]) << 8)
        | (u32::from(data[offset + 2]) << 16)
        | (u32::from(data[offset + 3]) << 24)
}

/// Write a little-endian u32 at `offset`.
fn write_u32_le(data: &mut [u8], offset: usize, value: u32) {
    data[offset] = (value & 0xFF) as u8;
    data[offset + 1] = ((value >> 8) & 0xFF) as u8;
    data[offset + 2] = ((value >> 16) & 0xFF) as u8;
    data[offset + 3] = ((value >> 24) & 0xFF) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    // ---------------------------------------------------------------
    // UserModuleId
    // ---------------------------------------------------------------

    #[test]
    fn module_id_from_byte_valid() {
        assert_eq!(UserModuleId::from_byte(1).unwrap(), UserModuleId::ModFx);
        assert_eq!(UserModuleId::from_byte(2).unwrap(), UserModuleId::DelayFx);
        assert_eq!(UserModuleId::from_byte(3).unwrap(), UserModuleId::ReverbFx);
        assert_eq!(UserModuleId::from_byte(4).unwrap(), UserModuleId::Osc);
    }

    #[test]
    fn module_id_from_byte_invalid() {
        assert!(UserModuleId::from_byte(0).is_err());
        assert!(UserModuleId::from_byte(5).is_err());
        assert!(UserModuleId::from_byte(255).is_err());
    }

    #[test]
    fn module_id_to_byte_roundtrip() {
        for id in [
            UserModuleId::ModFx,
            UserModuleId::DelayFx,
            UserModuleId::ReverbFx,
            UserModuleId::Osc,
        ] {
            let b = id.to_byte();
            assert_eq!(UserModuleId::from_byte(b).unwrap(), id);
        }
    }

    #[test]
    fn module_id_max_slots() {
        assert_eq!(UserModuleId::ModFx.max_slots(), 16);
        assert_eq!(UserModuleId::DelayFx.max_slots(), 8);
        assert_eq!(UserModuleId::ReverbFx.max_slots(), 8);
        assert_eq!(UserModuleId::Osc.max_slots(), 16);
    }

    #[test]
    fn module_id_display() {
        assert_eq!(UserModuleId::ModFx.to_string(), "Mod FX");
        assert_eq!(UserModuleId::DelayFx.to_string(), "Delay FX");
        assert_eq!(UserModuleId::ReverbFx.to_string(), "Reverb FX");
        assert_eq!(UserModuleId::Osc.to_string(), "Oscillator");
    }

    #[test]
    fn module_id_copy_hash() {
        use std::collections::HashSet;
        let id = UserModuleId::ModFx;
        let id2 = id; // Copy
        assert_eq!(id, id2);
        let mut set = HashSet::new();
        set.insert(id);
        assert!(set.contains(&UserModuleId::ModFx));
    }

    // ---------------------------------------------------------------
    // Slot validation
    // ---------------------------------------------------------------

    #[test]
    fn validate_slot_modfx_valid() {
        assert!(validate_slot(UserModuleId::ModFx, 0).is_ok());
        assert!(validate_slot(UserModuleId::ModFx, 15).is_ok());
    }

    #[test]
    fn validate_slot_modfx_invalid() {
        assert!(validate_slot(UserModuleId::ModFx, 16).is_err());
        assert!(validate_slot(UserModuleId::ModFx, 255).is_err());
    }

    #[test]
    fn validate_slot_delay_valid() {
        assert!(validate_slot(UserModuleId::DelayFx, 0).is_ok());
        assert!(validate_slot(UserModuleId::DelayFx, 7).is_ok());
    }

    #[test]
    fn validate_slot_delay_invalid() {
        assert!(validate_slot(UserModuleId::DelayFx, 8).is_err());
    }

    #[test]
    fn validate_slot_reverb_boundary() {
        assert!(validate_slot(UserModuleId::ReverbFx, 7).is_ok());
        assert!(validate_slot(UserModuleId::ReverbFx, 8).is_err());
    }

    #[test]
    fn validate_slot_osc_boundary() {
        assert!(validate_slot(UserModuleId::Osc, 15).is_ok());
        assert!(validate_slot(UserModuleId::Osc, 16).is_err());
    }

    // ---------------------------------------------------------------
    // UserModuleInfo
    // ---------------------------------------------------------------

    #[test]
    fn module_info_roundtrip() {
        let info = UserModuleInfo {
            max_slot_size: 0x0001_0000,
            max_program_size: 0x0000_8000,
            available_slot_count: 12,
        };
        let blob = info.to_bytes();
        assert_eq!(blob.len(), UserModuleInfo::BLOB_SIZE);
        let info2 = UserModuleInfo::from_bytes(&blob).unwrap();
        assert_eq!(info, info2);
    }

    #[test]
    fn module_info_from_bytes_too_short() {
        assert!(UserModuleInfo::from_bytes(&[0u8; 8]).is_err());
    }

    #[test]
    fn module_info_le_byte_order() {
        // max_slot_size = 0x04030201
        let blob = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x0A];
        let info = UserModuleInfo::from_bytes(&blob).unwrap();
        assert_eq!(info.max_slot_size, 0x04030201);
        assert_eq!(info.max_program_size, 0x08070605);
        assert_eq!(info.available_slot_count, 0x0A);
    }

    // ---------------------------------------------------------------
    // UserSlotStatus
    // ---------------------------------------------------------------

    #[test]
    fn slot_status_roundtrip() {
        let status = UserSlotStatus {
            platform_id: 7,
            module_id: UserModuleId::Osc,
            api_version: (1, 2, 0),
            developer_id: 0xDEAD,
            program_id: 0xBEEF,
            program_version: (3, 1, 4),
            program_name: "TestOsc".to_string(),
        };
        let blob = status.to_bytes();
        assert_eq!(blob.len(), UserSlotStatus::BLOB_SIZE);
        let status2 = UserSlotStatus::from_bytes(&blob).unwrap();
        assert_eq!(status, status2);
    }

    #[test]
    fn slot_status_from_bytes_too_short() {
        assert!(UserSlotStatus::from_bytes(&[0u8; 31]).is_err());
    }

    #[test]
    fn slot_status_invalid_module_id() {
        let mut blob = vec![0u8; 32];
        blob[1] = 99; // invalid module ID
        assert!(UserSlotStatus::from_bytes(&blob).is_err());
    }

    #[test]
    fn slot_status_name_null_terminated() {
        let mut blob = vec![0u8; 32];
        blob[1] = 1; // ModFx
        blob[16] = b'H';
        blob[17] = b'i';
        blob[18] = 0; // null terminator
        blob[19] = b'X'; // should be ignored
        let status = UserSlotStatus::from_bytes(&blob).unwrap();
        assert_eq!(status.program_name, "Hi");
    }

    #[test]
    fn slot_status_name_full_16_chars() {
        let mut blob = vec![0u8; 32];
        blob[1] = 2; // DelayFx
        for i in 0..16 {
            blob[16 + i] = b'A' + (i as u8);
        }
        let status = UserSlotStatus::from_bytes(&blob).unwrap();
        assert_eq!(status.program_name, "ABCDEFGHIJKLMNOP");
    }

    // ---------------------------------------------------------------
    // UserSlotData
    // ---------------------------------------------------------------

    #[test]
    fn slot_data_from_payload() {
        let payload = vec![0x01, 0x02, 0x03, 0x04];
        let data = UserSlotData::from_payload(payload.clone());
        assert_eq!(data.payload_size, 4);
        assert_eq!(data.payload, payload);
        assert_eq!(data.crc32, compute_crc32(&payload));
    }

    #[test]
    fn slot_data_roundtrip() {
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];
        let data = UserSlotData::from_payload(payload);
        let blob = data.to_bytes();
        let data2 = UserSlotData::from_bytes(&blob).unwrap();
        assert_eq!(data, data2);
    }

    #[test]
    fn slot_data_crc_mismatch() {
        let payload = vec![0x01, 0x02, 0x03];
        let mut data = UserSlotData::from_payload(payload);
        data.crc32 ^= 1; // corrupt CRC
        let blob = data.to_bytes();
        assert!(UserSlotData::from_bytes(&blob).is_err());
    }

    #[test]
    fn slot_data_from_bytes_too_short() {
        assert!(UserSlotData::from_bytes(&[0u8; 7]).is_err());
    }

    #[test]
    fn slot_data_from_bytes_payload_too_short() {
        // Header claims 100 bytes but only 2 follow.
        let mut blob = vec![0u8; 10];
        write_u32_le(&mut blob, 0, 100);
        assert!(UserSlotData::from_bytes(&blob).is_err());
    }

    // ---------------------------------------------------------------
    // CRC32
    // ---------------------------------------------------------------

    #[test]
    fn crc32_empty() {
        assert_eq!(compute_crc32(&[]), 0x00000000);
    }

    #[test]
    fn crc32_known_value() {
        // "123456789" -> CRC32 = 0xCBF43926 (standard IEEE)
        let data = b"123456789";
        assert_eq!(compute_crc32(data), 0xCBF43926);
    }

    #[test]
    fn crc32_single_byte() {
        // CRC32 of a single zero byte.
        let crc = compute_crc32(&[0x00]);
        assert_eq!(crc, 0xD202EF8D);
    }

    // ---------------------------------------------------------------
    // Request builders
    // ---------------------------------------------------------------

    #[test]
    fn build_api_version_request_format() {
        let msg = build_api_version_request(ch(0));
        assert_eq!(msg.len(), 8);
        assert_eq!(msg[6], frame::USER_API_VERSION_REQUEST);
    }

    #[test]
    fn build_module_info_request_format() {
        let msg = build_module_info_request(ch(0), UserModuleId::ModFx);
        assert_eq!(msg.len(), 9);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[1], KORG_ID);
        assert_eq!(msg[2], 0x30);
        assert_eq!(msg[6], frame::USER_MODULE_INFO_REQUEST);
        assert_eq!(msg[7], 1); // ModFx
        assert_eq!(msg[8], 0xF7);
    }

    #[test]
    fn build_module_info_request_all_modules() {
        for module in [
            UserModuleId::ModFx,
            UserModuleId::DelayFx,
            UserModuleId::ReverbFx,
            UserModuleId::Osc,
        ] {
            let msg = build_module_info_request(ch(5), module);
            assert_eq!(msg[7], module.to_byte());
        }
    }

    #[test]
    fn build_slot_status_request_format() {
        let msg = build_slot_status_request(ch(3), UserModuleId::Osc, 5).unwrap();
        assert_eq!(msg.len(), 10);
        assert_eq!(msg[6], frame::USER_SLOT_STATUS_REQUEST);
        assert_eq!(msg[7], 4); // Osc
        assert_eq!(msg[8], 5); // slot
        assert_eq!(msg[9], 0xF7);
    }

    #[test]
    fn build_slot_status_request_invalid_slot() {
        assert!(build_slot_status_request(ch(0), UserModuleId::DelayFx, 8).is_err());
    }

    #[test]
    fn build_slot_data_request_format() {
        let msg = build_slot_data_request(ch(0), UserModuleId::ReverbFx, 3).unwrap();
        assert_eq!(msg[6], frame::USER_SLOT_DATA_REQUEST);
        assert_eq!(msg[7], 3); // ReverbFx
        assert_eq!(msg[8], 3); // slot
    }

    #[test]
    fn build_slot_data_request_invalid_slot() {
        assert!(build_slot_data_request(ch(0), UserModuleId::ReverbFx, 8).is_err());
    }

    #[test]
    fn build_clear_slot_format() {
        let msg = build_clear_slot(ch(0), UserModuleId::ModFx, 10).unwrap();
        assert_eq!(msg[6], frame::CLEAR_USER_SLOT);
        assert_eq!(msg[7], 1); // ModFx
        assert_eq!(msg[8], 10); // slot
    }

    #[test]
    fn build_clear_slot_invalid_slot() {
        assert!(build_clear_slot(ch(0), UserModuleId::ModFx, 16).is_err());
    }

    #[test]
    fn build_clear_module_format() {
        let msg = build_clear_module(ch(15), UserModuleId::DelayFx);
        assert_eq!(msg.len(), 9);
        assert_eq!(msg[2], 0x3F); // channel 15
        assert_eq!(msg[6], frame::CLEAR_USER_MODULE);
        assert_eq!(msg[7], 2); // DelayFx
        assert_eq!(msg[8], 0xF7);
    }

    #[test]
    fn build_swap_slots_format() {
        let msg = build_swap_slots(ch(0), UserModuleId::Osc, 3, 7).unwrap();
        assert_eq!(msg.len(), 11);
        assert_eq!(msg[6], frame::SWAP_USER_DATA);
        assert_eq!(msg[7], 4); // Osc
        assert_eq!(msg[8], 3); // slot_a
        assert_eq!(msg[9], 7); // slot_b
        assert_eq!(msg[10], 0xF7);
    }

    #[test]
    fn build_swap_slots_invalid_slot_a() {
        assert!(build_swap_slots(ch(0), UserModuleId::DelayFx, 8, 0).is_err());
    }

    #[test]
    fn build_swap_slots_invalid_slot_b() {
        assert!(build_swap_slots(ch(0), UserModuleId::DelayFx, 0, 8).is_err());
    }

    // ---------------------------------------------------------------
    // Response parsers
    // ---------------------------------------------------------------

    #[test]
    fn parse_module_info_reply_roundtrip() {
        let info = UserModuleInfo {
            max_slot_size: 65536,
            max_program_size: 32768,
            available_slot_count: 10,
        };
        let blob = info.to_bytes();
        let msg = frame::build_sysex(ch(0), frame::USER_MODULE_INFO_REPLY, &blob);
        let info2 = parse_module_info_reply(&msg).unwrap();
        assert_eq!(info, info2);
    }

    #[test]
    fn parse_module_info_reply_wrong_function_id() {
        let blob = vec![0u8; 9];
        let msg = frame::build_sysex(ch(0), frame::USER_SLOT_STATUS_REPLY, &blob);
        assert!(parse_module_info_reply(&msg).is_err());
    }

    #[test]
    fn parse_slot_status_reply_roundtrip() {
        let status = UserSlotStatus {
            platform_id: 1,
            module_id: UserModuleId::ModFx,
            api_version: (1, 1, 0),
            developer_id: 42,
            program_id: 100,
            program_version: (2, 0, 1),
            program_name: "MyMod".to_string(),
        };
        let blob = status.to_bytes();
        let msg = frame::build_sysex(ch(0), frame::USER_SLOT_STATUS_REPLY, &blob);
        let status2 = parse_slot_status_reply(&msg).unwrap();
        assert_eq!(status, status2);
    }

    #[test]
    fn parse_slot_status_reply_wrong_function_id() {
        let blob = vec![0u8; 32];
        let msg = frame::build_sysex(ch(0), frame::USER_MODULE_INFO_REPLY, &blob);
        assert!(parse_slot_status_reply(&msg).is_err());
    }

    #[test]
    fn parse_slot_data_reply_roundtrip() {
        let payload = vec![0x42; 128];
        let data = UserSlotData::from_payload(payload);
        let blob = data.to_bytes();
        let msg = frame::build_sysex(ch(0), frame::USER_SLOT_DATA_REPLY, &blob);
        let data2 = parse_slot_data_reply(&msg).unwrap();
        assert_eq!(data, data2);
    }

    #[test]
    fn parse_slot_data_reply_wrong_function_id() {
        let payload = vec![0x00; 4];
        let data = UserSlotData::from_payload(payload);
        let blob = data.to_bytes();
        let msg = frame::build_sysex(ch(0), frame::USER_MODULE_INFO_REPLY, &blob);
        assert!(parse_slot_data_reply(&msg).is_err());
    }

    // ---------------------------------------------------------------
    // u32 LE helpers
    // ---------------------------------------------------------------

    #[test]
    fn u32_le_roundtrip() {
        for v in [0u32, 1, 0xFF, 0xFFFF, 0xFF_FFFF, 0xFFFF_FFFF, 0xDEADBEEF] {
            let mut buf = [0u8; 4];
            write_u32_le(&mut buf, 0, v);
            assert_eq!(read_u32_le(&buf, 0), v);
        }
    }
}
