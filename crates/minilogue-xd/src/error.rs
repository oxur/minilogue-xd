use thiserror::Error;

/// SysEx-layer errors.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SysexError {
    /// The SysEx header bytes were invalid or missing.
    #[error("invalid SysEx header: {0}")]
    InvalidHeader(String),

    /// The function ID in the message did not match what was expected.
    #[error("unexpected function ID 0x{found:02X}, expected 0x{expected:02X}")]
    WrongFunctionId {
        /// The function ID that was expected.
        expected: u8,
        /// The function ID that was found.
        found: u8,
    },

    /// The payload was shorter than required for the given message type.
    #[error("payload too short: expected {expected} bytes, got {actual}")]
    PayloadTooShort {
        /// Minimum number of bytes expected.
        expected: usize,
        /// Actual number of bytes received.
        actual: usize,
    },

    /// The magic bytes at the start of a data blob were wrong.
    #[error("invalid magic: expected {expected}, got {actual}")]
    InvalidMagic {
        /// Expected magic string.
        expected: String,
        /// Actual bytes found.
        actual: String,
    },

    /// A checksum in the data did not match the computed value.
    #[error("checksum mismatch: expected 0x{expected:02X}, got 0x{actual:02X}")]
    ChecksumMismatch {
        /// Expected checksum value.
        expected: u8,
        /// Actual checksum value.
        actual: u8,
    },

    /// A program number was outside the valid range 0..=499.
    #[error("invalid program number {0} (must be 0-499)")]
    InvalidProgramNumber(u16),

    /// A character in a program name was not a valid ASCII printable byte.
    #[error("invalid program name character: 0x{0:02X}")]
    InvalidProgramNameChar(u8),

    /// A CRC32 checksum in the data did not match the computed value.
    #[error("CRC32 mismatch: expected 0x{expected:08X}, got 0x{actual:08X}")]
    Crc32Mismatch {
        /// Expected CRC32 value.
        expected: u32,
        /// Actual CRC32 value.
        actual: u32,
    },

    /// A slot index was out of range for the given module type.
    #[error("slot index {slot} out of range for module (max {max})")]
    InvalidSlotIndex {
        /// The slot index that was provided.
        slot: u8,
        /// The maximum valid slot index (exclusive).
        max: u8,
    },

    /// A user module ID byte was not recognized.
    #[error("invalid user module ID: {0}")]
    InvalidModuleId(u8),

    /// A SysEx transaction timed out waiting for a response.
    #[error("transaction timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// The device returned a NAK (error) status code.
    #[error("device returned error status: {0}")]
    NakReceived(crate::sysex::frame::SysexStatus),

    /// The response had an unexpected function ID.
    #[error("unexpected response function ID 0x{0:02X}")]
    UnexpectedResponse(u8),
}

/// All errors produced by the minilogue-xd crate.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A value was outside the valid range for its type.
    #[error("{type_name} value {value} out of range {min}..={max}")]
    OutOfRange {
        type_name: &'static str,
        value: i64,
        min: i64,
        max: i64,
    },

    /// The 7-bit codec received malformed input.
    #[error("codec error: {0}")]
    Codec(String),

    /// A MIDI message had an invalid status byte or structure.
    #[error("invalid MIDI message: {0}")]
    InvalidMessage(String),

    /// A SysEx protocol error.
    #[error("SysEx error: {0}")]
    Sysex(#[from] SysexError),

    /// The underlying MIDI I/O layer failed.
    #[cfg(feature = "midi-io")]
    #[error("MIDI I/O error: {0}")]
    MidiIo(String),

    /// A file I/O or zip archive error.
    #[cfg(feature = "file-formats")]
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A zip archive format error.
    #[cfg(feature = "file-formats")]
    #[error("zip error: {0}")]
    Zip(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_out_of_range() {
        let e = Error::OutOfRange {
            type_name: "U7",
            value: 128,
            min: 0,
            max: 127,
        };
        let msg = e.to_string();
        assert!(msg.contains("128"));
        assert!(msg.contains("U7"));
        assert!(msg.contains("0..=127"));
    }

    #[test]
    fn error_display_codec() {
        let e = Error::Codec("high bit set".to_string());
        assert_eq!(e.to_string(), "codec error: high bit set");
    }

    #[test]
    fn error_display_invalid_message() {
        let e = Error::InvalidMessage("bad status byte".to_string());
        assert_eq!(e.to_string(), "invalid MIDI message: bad status byte");
    }

    // ---------------------------------------------------------------
    // SysexError display tests
    // ---------------------------------------------------------------

    #[test]
    fn sysex_error_display_invalid_header() {
        let e = SysexError::InvalidHeader("missing F0".to_string());
        assert_eq!(e.to_string(), "invalid SysEx header: missing F0");
    }

    #[test]
    fn sysex_error_display_wrong_function_id() {
        let e = SysexError::WrongFunctionId {
            expected: 0x51,
            found: 0x40,
        };
        let msg = e.to_string();
        assert!(msg.contains("0x40"));
        assert!(msg.contains("0x51"));
    }

    #[test]
    fn sysex_error_display_payload_too_short() {
        let e = SysexError::PayloadTooShort {
            expected: 63,
            actual: 10,
        };
        let msg = e.to_string();
        assert!(msg.contains("63"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn sysex_error_display_invalid_magic() {
        let e = SysexError::InvalidMagic {
            expected: "GLOB".to_string(),
            actual: "PROG".to_string(),
        };
        let msg = e.to_string();
        assert!(msg.contains("GLOB"));
        assert!(msg.contains("PROG"));
    }

    #[test]
    fn sysex_error_display_checksum_mismatch() {
        let e = SysexError::ChecksumMismatch {
            expected: 0xAB,
            actual: 0xCD,
        };
        let msg = e.to_string();
        assert!(msg.contains("0xAB"));
        assert!(msg.contains("0xCD"));
    }

    #[test]
    fn sysex_error_display_invalid_program_number() {
        let e = SysexError::InvalidProgramNumber(500);
        assert!(e.to_string().contains("500"));
    }

    #[test]
    fn sysex_error_display_invalid_program_name_char() {
        let e = SysexError::InvalidProgramNameChar(0xFF);
        assert!(e.to_string().contains("0xFF"));
    }

    #[test]
    fn sysex_error_display_crc32_mismatch() {
        let e = SysexError::Crc32Mismatch {
            expected: 0xDEADBEEF,
            actual: 0xCAFEBABE,
        };
        let msg = e.to_string();
        assert!(msg.contains("DEADBEEF"));
        assert!(msg.contains("CAFEBABE"));
    }

    #[test]
    fn sysex_error_display_invalid_slot_index() {
        let e = SysexError::InvalidSlotIndex { slot: 20, max: 16 };
        let msg = e.to_string();
        assert!(msg.contains("20"));
        assert!(msg.contains("16"));
    }

    #[test]
    fn sysex_error_display_invalid_module_id() {
        let e = SysexError::InvalidModuleId(99);
        assert!(e.to_string().contains("99"));
    }

    #[test]
    fn sysex_error_converts_to_error() {
        let sysex_err = SysexError::InvalidHeader("test".to_string());
        let err: Error = sysex_err.into();
        assert!(matches!(err, Error::Sysex(_)));
        assert!(err.to_string().contains("SysEx error"));
    }

    #[test]
    fn sysex_error_display_timeout() {
        let e = SysexError::Timeout(std::time::Duration::from_secs(5));
        let msg = e.to_string();
        assert!(msg.contains("timed out"));
        assert!(msg.contains("5s"));
    }

    #[test]
    fn sysex_error_display_nak_received() {
        use crate::sysex::frame::SysexStatus;
        let e = SysexError::NakReceived(SysexStatus::DataFormatError);
        let msg = e.to_string();
        assert!(msg.contains("error status"));
        assert!(msg.contains("Data Format Error"));
    }

    #[test]
    fn sysex_error_display_unexpected_response() {
        let e = SysexError::UnexpectedResponse(0x40);
        let msg = e.to_string();
        assert!(msg.contains("unexpected response"));
        assert!(msg.contains("0x40"));
    }
}
