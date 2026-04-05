//! Channel voice and channel mode MIDI message types.
//!
//! This module contains the structs for all channel messages documented in
//! the Minilogue XD MIDI Implementation chart, along with trait
//! implementations for serializing to and deserializing from raw MIDI bytes.

use crate::error::{Error, Result};
use crate::message::types::{I14, U4, U7};

/// Serialize a MIDI message to its wire bytes.
pub trait ToMidiBytes {
    /// Returns the raw MIDI byte representation of this message.
    fn to_midi_bytes(&self) -> Vec<u8>;
}

/// Deserialize a MIDI message from raw wire bytes.
pub trait FromMidiBytes: Sized {
    /// Parses a MIDI message from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMessage`] if the bytes do not represent a valid
    /// message of this type.
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self>;
}

// ---------------------------------------------------------------------------
// Channel Voice Messages (TX + RX)
// ---------------------------------------------------------------------------

/// Note On message (status 0x90).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteOn {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Note number (0..=127).
    pub key: U7,
    /// Velocity (1..=127; velocity 0 is interpreted as Note Off).
    pub velocity: U7,
}

impl ToMidiBytes for NoteOn {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![
            0x90 | self.channel.value(),
            self.key.value(),
            self.velocity.value(),
        ]
    }
}

impl FromMidiBytes for NoteOn {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "NoteOn requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0x90 {
            return Err(Error::InvalidMessage(format!(
                "expected NoteOn status 0x9n, got 0x{status:02X}"
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            key: U7::new(bytes[1])?,
            velocity: U7::new(bytes[2])?,
        })
    }
}

/// Note Off message (status 0x80).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteOff {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Note number (0..=127).
    pub key: U7,
    /// Release velocity (0..=127).
    pub velocity: U7,
}

impl ToMidiBytes for NoteOff {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![
            0x80 | self.channel.value(),
            self.key.value(),
            self.velocity.value(),
        ]
    }
}

impl FromMidiBytes for NoteOff {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "NoteOff requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0x80 {
            return Err(Error::InvalidMessage(format!(
                "expected NoteOff status 0x8n, got 0x{status:02X}"
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            key: U7::new(bytes[1])?,
            velocity: U7::new(bytes[2])?,
        })
    }
}

/// Program Change message (status 0xC0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramChange {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Program number (0..=127).
    pub program: U7,
}

impl ToMidiBytes for ProgramChange {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xC0 | self.channel.value(), self.program.value()]
    }
}

impl FromMidiBytes for ProgramChange {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 2 {
            return Err(Error::InvalidMessage(format!(
                "ProgramChange requires 2 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xC0 {
            return Err(Error::InvalidMessage(format!(
                "expected ProgramChange status 0xCn, got 0x{status:02X}"
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            program: U7::new(bytes[1])?,
        })
    }
}

/// Pitch Bend message (status 0xE0).
///
/// The signed value is encoded on the wire as an unsigned 14-bit integer
/// with center at 0x2000 (8192). Wire format: `[0xE0|ch, lsb, msb]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PitchBend {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Pitch bend value (-8192..=8191).
    pub value: I14,
}

impl PitchBend {
    /// The center offset added to the signed value for wire encoding.
    const CENTER: i32 = 8192;
}

impl ToMidiBytes for PitchBend {
    fn to_midi_bytes(&self) -> Vec<u8> {
        let wire = (i32::from(self.value.value()) + Self::CENTER) as u16;
        vec![
            0xE0 | self.channel.value(),
            (wire & 0x7F) as u8,
            ((wire >> 7) & 0x7F) as u8,
        ]
    }
}

impl FromMidiBytes for PitchBend {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "PitchBend requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xE0 {
            return Err(Error::InvalidMessage(format!(
                "expected PitchBend status 0xEn, got 0x{status:02X}"
            )));
        }
        let lsb = u16::from(bytes[1] & 0x7F);
        let msb = u16::from(bytes[2] & 0x7F);
        let wire = (msb << 7) | lsb;
        let signed = i16::try_from(i32::from(wire) - Self::CENTER).map_err(|_| {
            Error::InvalidMessage(format!("pitch bend wire value {wire} out of range"))
        })?;
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            value: I14::new(signed)?,
        })
    }
}

/// Control Change message (status 0xB0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlChange {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Controller number (0..=127).
    pub controller: U7,
    /// Controller value (0..=127).
    pub value: U7,
}

impl ToMidiBytes for ControlChange {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![
            0xB0 | self.channel.value(),
            self.controller.value(),
            self.value.value(),
        ]
    }
}

impl FromMidiBytes for ControlChange {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "ControlChange requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected ControlChange status 0xBn, got 0x{status:02X}"
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            controller: U7::new(bytes[1])?,
            value: U7::new(bytes[2])?,
        })
    }
}

/// Channel Pressure (Aftertouch) message (status 0xD0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelPressure {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Pressure value (0..=127).
    pub value: U7,
}

impl ToMidiBytes for ChannelPressure {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xD0 | self.channel.value(), self.value.value()]
    }
}

impl FromMidiBytes for ChannelPressure {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 2 {
            return Err(Error::InvalidMessage(format!(
                "ChannelPressure requires 2 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xD0 {
            return Err(Error::InvalidMessage(format!(
                "expected ChannelPressure status 0xDn, got 0x{status:02X}"
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            value: U7::new(bytes[1])?,
        })
    }
}

// ---------------------------------------------------------------------------
// Channel Mode / RX-only Messages
// ---------------------------------------------------------------------------

/// All Sound Off (CC 120, value 0) — RX only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AllSoundOff {
    /// MIDI channel (0..=15).
    pub channel: U4,
}

impl ToMidiBytes for AllSoundOff {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xB0 | self.channel.value(), 120, 0]
    }
}

impl FromMidiBytes for AllSoundOff {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "AllSoundOff requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC status 0xBn, got 0x{status:02X}"
            )));
        }
        if bytes[1] != 120 {
            return Err(Error::InvalidMessage(format!(
                "expected CC 120 for AllSoundOff, got {}",
                bytes[1]
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
        })
    }
}

/// All Notes Off (CC 123, value 0) — RX only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AllNotesOff {
    /// MIDI channel (0..=15).
    pub channel: U4,
}

impl ToMidiBytes for AllNotesOff {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xB0 | self.channel.value(), 123, 0]
    }
}

impl FromMidiBytes for AllNotesOff {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "AllNotesOff requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC status 0xBn, got 0x{status:02X}"
            )));
        }
        if bytes[1] != 123 {
            return Err(Error::InvalidMessage(format!(
                "expected CC 123 for AllNotesOff, got {}",
                bytes[1]
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
        })
    }
}

/// Local Control state (per AP-10: use an enum, not a boolean).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LocalControlState {
    /// Local control disabled (CC 122, value 0).
    Off,
    /// Local control enabled (CC 122, value 127).
    On,
}

/// Local Control message (CC 122) — RX only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalControl {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Whether local control is on or off.
    pub state: LocalControlState,
}

impl ToMidiBytes for LocalControl {
    fn to_midi_bytes(&self) -> Vec<u8> {
        let value = match self.state {
            LocalControlState::Off => 0,
            LocalControlState::On => 127,
        };
        vec![0xB0 | self.channel.value(), 122, value]
    }
}

impl FromMidiBytes for LocalControl {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "LocalControl requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC status 0xBn, got 0x{status:02X}"
            )));
        }
        if bytes[1] != 122 {
            return Err(Error::InvalidMessage(format!(
                "expected CC 122 for LocalControl, got {}",
                bytes[1]
            )));
        }
        let state = if bytes[2] == 0 {
            LocalControlState::Off
        } else {
            LocalControlState::On
        };
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            state,
        })
    }
}

/// Damper pedal (sustain) message (CC 64) — RX only, full 0..=127 range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Damper {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Damper value (0..=127).
    pub value: U7,
}

impl ToMidiBytes for Damper {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xB0 | self.channel.value(), 64, self.value.value()]
    }
}

impl FromMidiBytes for Damper {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 3 {
            return Err(Error::InvalidMessage(format!(
                "Damper requires 3 bytes, got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC status 0xBn, got 0x{status:02X}"
            )));
        }
        if bytes[1] != 64 {
            return Err(Error::InvalidMessage(format!(
                "expected CC 64 for Damper, got {}",
                bytes[1]
            )));
        }
        Ok(Self {
            channel: U4::new(status & 0x0F)?,
            value: U7::new(bytes[2])?,
        })
    }
}

/// Bank Select message (CC 0 MSB + CC 32 LSB) — RX only.
///
/// The LSB is validated to the range 0..=4 per the Minilogue XD spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BankSelect {
    /// MIDI channel (0..=15).
    pub channel: U4,
    /// Bank MSB (CC 0 value, 0..=127).
    pub msb: U7,
    /// Bank LSB (CC 32 value, 0..=4 for the Minilogue XD).
    pub lsb: U7,
}

impl BankSelect {
    /// Maximum valid LSB for the Minilogue XD.
    const MAX_LSB: u8 = 4;

    /// Creates a new `BankSelect`, validating that `lsb` is in range 0..=4.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `lsb` exceeds 4.
    pub fn new(channel: U4, msb: U7, lsb: U7) -> Result<Self> {
        if lsb.value() > Self::MAX_LSB {
            return Err(Error::OutOfRange {
                type_name: "BankSelect LSB",
                value: i64::from(lsb.value()),
                min: 0,
                max: i64::from(Self::MAX_LSB),
            });
        }
        Ok(Self { channel, msb, lsb })
    }
}

impl ToMidiBytes for BankSelect {
    /// Serializes as two CC messages: CC 0 (MSB) followed by CC 32 (LSB).
    fn to_midi_bytes(&self) -> Vec<u8> {
        let status = 0xB0 | self.channel.value();
        vec![status, 0, self.msb.value(), status, 32, self.lsb.value()]
    }
}

impl FromMidiBytes for BankSelect {
    /// Parses from 6 bytes: `[Bn, 00, msb, Bn, 20, lsb]`.
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 6 {
            return Err(Error::InvalidMessage(format!(
                "BankSelect requires 6 bytes (CC0 + CC32), got {}",
                bytes.len()
            )));
        }
        let status = bytes[0];
        if status & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC status 0xBn, got 0x{status:02X}"
            )));
        }
        if bytes[1] != 0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC 0 for BankSelect MSB, got {}",
                bytes[1]
            )));
        }
        if bytes[3] & 0xF0 != 0xB0 {
            return Err(Error::InvalidMessage(format!(
                "expected CC status for LSB, got 0x{:02X}",
                bytes[3]
            )));
        }
        if bytes[4] != 32 {
            return Err(Error::InvalidMessage(format!(
                "expected CC 32 for BankSelect LSB, got {}",
                bytes[4]
            )));
        }
        let channel = U4::new(status & 0x0F)?;
        let msb = U7::new(bytes[2])?;
        let lsb = U7::new(bytes[5])?;
        Self::new(channel, msb, lsb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- NoteOn ---

    #[test]
    fn note_on_round_trip() {
        let msg = NoteOn {
            channel: U4::new(0).unwrap(),
            key: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0x90, 60, 100]);
        let parsed = NoteOn::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn note_on_channel_15() {
        let msg = NoteOn {
            channel: U4::new(15).unwrap(),
            key: U7::new(0).unwrap(),
            velocity: U7::new(127).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes[0], 0x9F);
    }

    #[test]
    fn note_on_wrong_status() {
        assert!(NoteOn::from_midi_bytes(&[0x80, 60, 100]).is_err());
    }

    #[test]
    fn note_on_too_short() {
        assert!(NoteOn::from_midi_bytes(&[0x90, 60]).is_err());
    }

    // --- NoteOff ---

    #[test]
    fn note_off_round_trip() {
        let msg = NoteOff {
            channel: U4::new(3).unwrap(),
            key: U7::new(72).unwrap(),
            velocity: U7::new(64).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0x83, 72, 64]);
        let parsed = NoteOff::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn note_off_wrong_status() {
        assert!(NoteOff::from_midi_bytes(&[0x90, 60, 100]).is_err());
    }

    #[test]
    fn note_off_too_short() {
        assert!(NoteOff::from_midi_bytes(&[0x80]).is_err());
    }

    // --- ProgramChange ---

    #[test]
    fn program_change_round_trip() {
        let msg = ProgramChange {
            channel: U4::new(5).unwrap(),
            program: U7::new(42).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xC5, 42]);
        let parsed = ProgramChange::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn program_change_two_byte_encoding() {
        let msg = ProgramChange {
            channel: U4::new(0).unwrap(),
            program: U7::new(0).unwrap(),
        };
        assert_eq!(msg.to_midi_bytes().len(), 2);
    }

    #[test]
    fn program_change_wrong_status() {
        assert!(ProgramChange::from_midi_bytes(&[0xB0, 42]).is_err());
    }

    #[test]
    fn program_change_too_short() {
        assert!(ProgramChange::from_midi_bytes(&[0xC0]).is_err());
    }

    // --- PitchBend ---

    #[test]
    fn pitch_bend_center() {
        let msg = PitchBend {
            channel: U4::new(0).unwrap(),
            value: I14::new(0).unwrap(),
        };
        // center = 8192 = 0x2000, lsb = 0x00, msb = 0x40
        assert_eq!(msg.to_midi_bytes(), vec![0xE0, 0x00, 0x40]);
    }

    #[test]
    fn pitch_bend_min() {
        let msg = PitchBend {
            channel: U4::new(0).unwrap(),
            value: I14::new(-8192).unwrap(),
        };
        // wire = 0, lsb = 0, msb = 0
        assert_eq!(msg.to_midi_bytes(), vec![0xE0, 0x00, 0x00]);
    }

    #[test]
    fn pitch_bend_max() {
        let msg = PitchBend {
            channel: U4::new(0).unwrap(),
            value: I14::new(8191).unwrap(),
        };
        // wire = 16383 = 0x3FFF, lsb = 0x7F, msb = 0x7F
        assert_eq!(msg.to_midi_bytes(), vec![0xE0, 0x7F, 0x7F]);
    }

    #[test]
    fn pitch_bend_round_trip() {
        let msg = PitchBend {
            channel: U4::new(7).unwrap(),
            value: I14::new(1000).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        let parsed = PitchBend::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn pitch_bend_wrong_status() {
        assert!(PitchBend::from_midi_bytes(&[0x90, 0x00, 0x40]).is_err());
    }

    #[test]
    fn pitch_bend_too_short() {
        assert!(PitchBend::from_midi_bytes(&[0xE0, 0x00]).is_err());
    }

    // --- ControlChange ---

    #[test]
    fn control_change_round_trip() {
        let msg = ControlChange {
            channel: U4::new(2).unwrap(),
            controller: U7::new(74).unwrap(),
            value: U7::new(100).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB2, 74, 100]);
        let parsed = ControlChange::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn control_change_wrong_status() {
        assert!(ControlChange::from_midi_bytes(&[0x90, 74, 100]).is_err());
    }

    #[test]
    fn control_change_too_short() {
        assert!(ControlChange::from_midi_bytes(&[0xB0, 74]).is_err());
    }

    // --- ChannelPressure ---

    #[test]
    fn channel_pressure_round_trip() {
        let msg = ChannelPressure {
            channel: U4::new(0).unwrap(),
            value: U7::new(80).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xD0, 80]);
        let parsed = ChannelPressure::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn channel_pressure_two_byte_encoding() {
        let msg = ChannelPressure {
            channel: U4::new(0).unwrap(),
            value: U7::new(0).unwrap(),
        };
        assert_eq!(msg.to_midi_bytes().len(), 2);
    }

    #[test]
    fn channel_pressure_wrong_status() {
        assert!(ChannelPressure::from_midi_bytes(&[0xC0, 80]).is_err());
    }

    #[test]
    fn channel_pressure_too_short() {
        assert!(ChannelPressure::from_midi_bytes(&[0xD0]).is_err());
    }

    // --- AllSoundOff ---

    #[test]
    fn all_sound_off_round_trip() {
        let msg = AllSoundOff {
            channel: U4::new(0).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB0, 120, 0]);
        let parsed = AllSoundOff::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn all_sound_off_wrong_cc() {
        assert!(AllSoundOff::from_midi_bytes(&[0xB0, 121, 0]).is_err());
    }

    // --- AllNotesOff ---

    #[test]
    fn all_notes_off_round_trip() {
        let msg = AllNotesOff {
            channel: U4::new(4).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB4, 123, 0]);
        let parsed = AllNotesOff::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn all_notes_off_wrong_cc() {
        assert!(AllNotesOff::from_midi_bytes(&[0xB0, 120, 0]).is_err());
    }

    // --- LocalControl ---

    #[test]
    fn local_control_off_round_trip() {
        let msg = LocalControl {
            channel: U4::new(0).unwrap(),
            state: LocalControlState::Off,
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB0, 122, 0]);
        let parsed = LocalControl::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn local_control_on_round_trip() {
        let msg = LocalControl {
            channel: U4::new(0).unwrap(),
            state: LocalControlState::On,
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB0, 122, 127]);
        let parsed = LocalControl::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn local_control_nonzero_is_on() {
        // Any nonzero value should be interpreted as On.
        let parsed = LocalControl::from_midi_bytes(&[0xB0, 122, 64]).unwrap();
        assert_eq!(parsed.state, LocalControlState::On);
    }

    #[test]
    fn local_control_wrong_cc() {
        assert!(LocalControl::from_midi_bytes(&[0xB0, 123, 0]).is_err());
    }

    // --- Damper ---

    #[test]
    fn damper_round_trip() {
        let msg = Damper {
            channel: U4::new(0).unwrap(),
            value: U7::new(127).unwrap(),
        };
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB0, 64, 127]);
        let parsed = Damper::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn damper_wrong_cc() {
        assert!(Damper::from_midi_bytes(&[0xB0, 65, 0]).is_err());
    }

    // --- BankSelect ---

    #[test]
    fn bank_select_round_trip() {
        let msg = BankSelect::new(
            U4::new(0).unwrap(),
            U7::new(0).unwrap(),
            U7::new(2).unwrap(),
        )
        .unwrap();
        let bytes = msg.to_midi_bytes();
        assert_eq!(bytes, vec![0xB0, 0, 0, 0xB0, 32, 2]);
        let parsed = BankSelect::from_midi_bytes(&bytes).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn bank_select_lsb_max_valid() {
        assert!(BankSelect::new(
            U4::new(0).unwrap(),
            U7::new(0).unwrap(),
            U7::new(4).unwrap(),
        )
        .is_ok());
    }

    #[test]
    fn bank_select_lsb_out_of_range() {
        assert!(BankSelect::new(
            U4::new(0).unwrap(),
            U7::new(0).unwrap(),
            U7::new(5).unwrap(),
        )
        .is_err());
    }

    #[test]
    fn bank_select_too_short() {
        assert!(BankSelect::from_midi_bytes(&[0xB0, 0, 0, 0xB0, 32]).is_err());
    }

    #[test]
    fn bank_select_wrong_first_cc() {
        assert!(BankSelect::from_midi_bytes(&[0xB0, 1, 0, 0xB0, 32, 0]).is_err());
    }

    #[test]
    fn bank_select_wrong_second_cc() {
        assert!(BankSelect::from_midi_bytes(&[0xB0, 0, 0, 0xB0, 33, 0]).is_err());
    }
}
