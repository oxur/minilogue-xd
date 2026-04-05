//! MIDI message types for the Minilogue XD.
//!
//! This module provides strongly-typed representations of every MIDI message
//! in the Minilogue XD Implementation chart, along with serialization and
//! parsing through the [`ToMidiBytes`] and [`FromMidiBytes`] traits.

pub mod channel;
pub mod common;
pub mod realtime;
pub mod types;

pub use channel::*;
pub use common::*;
pub use realtime::*;
pub use types::*;

use crate::error::{Error, Result};

/// A parsed MIDI message covering all types recognized by the Minilogue XD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MidiMessage {
    /// Note On (status 0x90).
    NoteOn(NoteOn),
    /// Note Off (status 0x80, or 0x90 with velocity 0).
    NoteOff(NoteOff),
    /// Control Change (status 0xB0, generic CC).
    ControlChange(ControlChange),
    /// Program Change (status 0xC0).
    ProgramChange(ProgramChange),
    /// Pitch Bend (status 0xE0).
    PitchBend(PitchBend),
    /// Channel Pressure / Aftertouch (status 0xD0).
    ChannelPressure(ChannelPressure),
    /// All Sound Off (CC 120).
    AllSoundOff(AllSoundOff),
    /// All Notes Off (CC 123).
    AllNotesOff(AllNotesOff),
    /// Local Control (CC 122).
    LocalControl(LocalControl),
    /// Damper / Sustain pedal (CC 64).
    Damper(Damper),
    /// Timing Clock (0xF8).
    TimingClock(TimingClock),
    /// Start (0xFA).
    Start(Start),
    /// Continue (0xFB).
    Continue(Continue),
    /// Stop (0xFC).
    Stop(Stop),
    /// Active Sensing (0xFE).
    ActiveSensing(ActiveSensing),
    /// Song Position Pointer (0xF2).
    SongPositionPointer(SongPositionPointer),
}

impl ToMidiBytes for MidiMessage {
    fn to_midi_bytes(&self) -> Vec<u8> {
        match self {
            Self::NoteOn(m) => m.to_midi_bytes(),
            Self::NoteOff(m) => m.to_midi_bytes(),
            Self::ControlChange(m) => m.to_midi_bytes(),
            Self::ProgramChange(m) => m.to_midi_bytes(),
            Self::PitchBend(m) => m.to_midi_bytes(),
            Self::ChannelPressure(m) => m.to_midi_bytes(),
            Self::AllSoundOff(m) => m.to_midi_bytes(),
            Self::AllNotesOff(m) => m.to_midi_bytes(),
            Self::LocalControl(m) => m.to_midi_bytes(),
            Self::Damper(m) => m.to_midi_bytes(),
            Self::TimingClock(m) => m.to_midi_bytes(),
            Self::Start(m) => m.to_midi_bytes(),
            Self::Continue(m) => m.to_midi_bytes(),
            Self::Stop(m) => m.to_midi_bytes(),
            Self::ActiveSensing(m) => m.to_midi_bytes(),
            Self::SongPositionPointer(m) => m.to_midi_bytes(),
        }
    }
}

/// Parse raw MIDI bytes into a [`MidiMessage`].
///
/// Dispatches on the status byte (or status nibble for channel messages).
/// For Note On messages with velocity 0, returns a [`MidiMessage::NoteOff`].
/// For CC messages, inspects the controller number to produce specialized
/// types (AllSoundOff, AllNotesOff, LocalControl, Damper).
///
/// # Errors
///
/// Returns [`Error::InvalidMessage`] if the status byte is unrecognized or
/// the message bytes are malformed.
pub fn parse_midi_bytes(bytes: &[u8]) -> Result<MidiMessage> {
    if bytes.is_empty() {
        return Err(Error::InvalidMessage("empty message".to_string()));
    }

    let status = bytes[0];

    // System messages (0xF0+) are identified by the full byte.
    if status >= 0xF0 {
        return match status {
            0xF2 => Ok(MidiMessage::SongPositionPointer(
                SongPositionPointer::from_midi_bytes(bytes)?,
            )),
            0xF8 => Ok(MidiMessage::TimingClock(TimingClock::from_midi_bytes(
                bytes,
            )?)),
            0xFA => Ok(MidiMessage::Start(Start::from_midi_bytes(bytes)?)),
            0xFB => Ok(MidiMessage::Continue(Continue::from_midi_bytes(bytes)?)),
            0xFC => Ok(MidiMessage::Stop(Stop::from_midi_bytes(bytes)?)),
            0xFE => Ok(MidiMessage::ActiveSensing(ActiveSensing::from_midi_bytes(
                bytes,
            )?)),
            _ => Err(Error::InvalidMessage(format!(
                "unrecognized status byte 0x{status:02X}"
            ))),
        };
    }

    // Channel messages are identified by the upper nibble.
    match status & 0xF0 {
        0x80 => Ok(MidiMessage::NoteOff(NoteOff::from_midi_bytes(bytes)?)),
        0x90 => {
            // NoteOn with velocity 0 is conventionally NoteOff.
            if bytes.len() >= 3 && bytes[2] == 0 {
                Ok(MidiMessage::NoteOff(NoteOff {
                    channel: U4::new(status & 0x0F)?,
                    key: U7::new(bytes[1])?,
                    velocity: U7::new(0)?,
                }))
            } else {
                Ok(MidiMessage::NoteOn(NoteOn::from_midi_bytes(bytes)?))
            }
        }
        0xB0 => {
            // Inspect controller number for specialized CC types.
            if bytes.len() < 3 {
                return Err(Error::InvalidMessage(format!(
                    "CC message requires 3 bytes, got {}",
                    bytes.len()
                )));
            }
            match bytes[1] {
                120 => Ok(MidiMessage::AllSoundOff(AllSoundOff::from_midi_bytes(
                    bytes,
                )?)),
                122 => Ok(MidiMessage::LocalControl(LocalControl::from_midi_bytes(
                    bytes,
                )?)),
                123 => Ok(MidiMessage::AllNotesOff(AllNotesOff::from_midi_bytes(
                    bytes,
                )?)),
                64 => Ok(MidiMessage::Damper(Damper::from_midi_bytes(bytes)?)),
                _ => Ok(MidiMessage::ControlChange(ControlChange::from_midi_bytes(
                    bytes,
                )?)),
            }
        }
        0xC0 => Ok(MidiMessage::ProgramChange(ProgramChange::from_midi_bytes(
            bytes,
        )?)),
        0xD0 => Ok(MidiMessage::ChannelPressure(
            ChannelPressure::from_midi_bytes(bytes)?,
        )),
        0xE0 => Ok(MidiMessage::PitchBend(PitchBend::from_midi_bytes(bytes)?)),
        _ => Err(Error::InvalidMessage(format!(
            "unrecognized status byte 0x{status:02X}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Basic variant parsing ---

    #[test]
    fn parse_note_on() {
        let msg = parse_midi_bytes(&[0x90, 60, 100]).unwrap();
        assert!(matches!(msg, MidiMessage::NoteOn(_)));
    }

    #[test]
    fn parse_note_off() {
        let msg = parse_midi_bytes(&[0x80, 60, 64]).unwrap();
        assert!(matches!(msg, MidiMessage::NoteOff(_)));
    }

    #[test]
    fn parse_note_on_velocity_zero_becomes_note_off() {
        let msg = parse_midi_bytes(&[0x90, 60, 0]).unwrap();
        match msg {
            MidiMessage::NoteOff(n) => {
                assert_eq!(n.key.value(), 60);
                assert_eq!(n.velocity.value(), 0);
            }
            other => panic!("expected NoteOff, got {other:?}"),
        }
    }

    #[test]
    fn parse_control_change_generic() {
        let msg = parse_midi_bytes(&[0xB0, 74, 100]).unwrap();
        assert!(matches!(msg, MidiMessage::ControlChange(_)));
    }

    #[test]
    fn parse_cc_120_all_sound_off() {
        let msg = parse_midi_bytes(&[0xB0, 120, 0]).unwrap();
        assert!(matches!(msg, MidiMessage::AllSoundOff(_)));
    }

    #[test]
    fn parse_cc_122_local_control() {
        let msg = parse_midi_bytes(&[0xB0, 122, 127]).unwrap();
        match msg {
            MidiMessage::LocalControl(lc) => {
                assert_eq!(lc.state, LocalControlState::On);
            }
            other => panic!("expected LocalControl, got {other:?}"),
        }
    }

    #[test]
    fn parse_cc_123_all_notes_off() {
        let msg = parse_midi_bytes(&[0xB0, 123, 0]).unwrap();
        assert!(matches!(msg, MidiMessage::AllNotesOff(_)));
    }

    #[test]
    fn parse_cc_64_damper() {
        let msg = parse_midi_bytes(&[0xB0, 64, 127]).unwrap();
        assert!(matches!(msg, MidiMessage::Damper(_)));
    }

    #[test]
    fn parse_program_change() {
        let msg = parse_midi_bytes(&[0xC0, 42]).unwrap();
        assert!(matches!(msg, MidiMessage::ProgramChange(_)));
    }

    #[test]
    fn parse_channel_pressure() {
        let msg = parse_midi_bytes(&[0xD0, 80]).unwrap();
        assert!(matches!(msg, MidiMessage::ChannelPressure(_)));
    }

    #[test]
    fn parse_pitch_bend() {
        let msg = parse_midi_bytes(&[0xE0, 0x00, 0x40]).unwrap();
        assert!(matches!(msg, MidiMessage::PitchBend(_)));
    }

    #[test]
    fn parse_song_position_pointer() {
        let msg = parse_midi_bytes(&[0xF2, 0x7F, 0x7F]).unwrap();
        assert!(matches!(msg, MidiMessage::SongPositionPointer(_)));
    }

    #[test]
    fn parse_timing_clock() {
        let msg = parse_midi_bytes(&[0xF8]).unwrap();
        assert!(matches!(msg, MidiMessage::TimingClock(_)));
    }

    #[test]
    fn parse_start() {
        let msg = parse_midi_bytes(&[0xFA]).unwrap();
        assert!(matches!(msg, MidiMessage::Start(_)));
    }

    #[test]
    fn parse_continue() {
        let msg = parse_midi_bytes(&[0xFB]).unwrap();
        assert!(matches!(msg, MidiMessage::Continue(_)));
    }

    #[test]
    fn parse_stop() {
        let msg = parse_midi_bytes(&[0xFC]).unwrap();
        assert!(matches!(msg, MidiMessage::Stop(_)));
    }

    #[test]
    fn parse_active_sensing() {
        let msg = parse_midi_bytes(&[0xFE]).unwrap();
        assert!(matches!(msg, MidiMessage::ActiveSensing(_)));
    }

    // --- Error cases ---

    #[test]
    fn parse_empty_is_error() {
        assert!(parse_midi_bytes(&[]).is_err());
    }

    #[test]
    fn parse_unknown_status_is_error() {
        // 0x00..0x7F are data bytes, not status bytes.
        assert!(parse_midi_bytes(&[0x00]).is_err());
    }

    #[test]
    fn parse_unrecognized_system_status_is_error() {
        assert!(parse_midi_bytes(&[0xF1]).is_err());
    }

    #[test]
    fn parse_cc_too_short_is_error() {
        assert!(parse_midi_bytes(&[0xB0, 74]).is_err());
    }

    // --- ToMidiBytes delegation ---

    #[test]
    fn midi_message_to_bytes_delegates() {
        let msg = MidiMessage::NoteOn(NoteOn {
            channel: U4::new(0).unwrap(),
            key: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
        });
        assert_eq!(msg.to_midi_bytes(), vec![0x90, 60, 100]);

        let msg = MidiMessage::TimingClock(TimingClock);
        assert_eq!(msg.to_midi_bytes(), vec![0xF8]);
    }
}
