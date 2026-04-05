//! System Common MIDI messages.
//!
//! Currently implements Song Position Pointer, which is the only System
//! Common message in the Minilogue XD MIDI Implementation chart.

use crate::error::{Error, Result};
use crate::message::channel::{FromMidiBytes, ToMidiBytes};
use crate::message::types::U14;

/// Song Position Pointer (0xF2).
///
/// The 14-bit beat count is encoded as two 7-bit bytes (LSB first):
/// `[0xF2, beats & 0x7F, (beats >> 7) & 0x7F]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SongPositionPointer {
    /// Beat position (0..=16383), where each beat is 6 MIDI clocks.
    pub beats: U14,
}

impl ToMidiBytes for SongPositionPointer {
    fn to_midi_bytes(&self) -> Vec<u8> {
        let v = self.beats.value();
        vec![0xF2, (v & 0x7F) as u8, ((v >> 7) & 0x7F) as u8]
    }
}

impl FromMidiBytes for SongPositionPointer {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "SongPositionPointer requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        if bytes[0] != 0xF2 {
            return Err(Error::InvalidMessage(format!(
                "expected SongPositionPointer status 0xF2, got 0x{:02X}",
                bytes[0]
            )));
        }
        let lsb = u16::from(bytes[1] & 0x7F);
        let msb = u16::from(bytes[2] & 0x7F);
        let value = (msb << 7) | lsb;
        Ok(Self {
            beats: U14::new(value)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spp_zero_round_trip() {
        let msg = SongPositionPointer {
            beats: U14::new(0).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xF2, 0x00, 0x00]);
        let parsed = SongPositionPointer::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn spp_midpoint_round_trip() {
        let msg = SongPositionPointer {
            beats: U14::new(8192).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        // 8192 = 0x2000 -> lsb = 0x00, msb = 0x40
        assert_eq!(bytes, vec![0xF2, 0x00, 0x40]);
        let parsed = SongPositionPointer::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn spp_max_round_trip() {
        let msg = SongPositionPointer {
            beats: U14::new(16383).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xF2, 0x7F, 0x7F]);
        let parsed = SongPositionPointer::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn spp_wrong_status() {
        assert!(SongPositionPointer::from_midi_bytes(&[0xF3, 0x00, 0x00]).is_err());
    }

    #[test]
    fn spp_too_short() {
        assert!(SongPositionPointer::from_midi_bytes(&[0xF2, 0x00]).is_err());
    }
}
