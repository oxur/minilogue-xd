use thiserror::Error;

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

    /// The underlying MIDI I/O layer failed.
    #[cfg(feature = "midi-io")]
    #[error("MIDI I/O error: {0}")]
    MidiIo(String),
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
}
