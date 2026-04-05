//! System Real-Time MIDI messages.
//!
//! These single-byte messages are used for timing and transport control.
//! The Minilogue XD recognizes all five of these on receive.

use crate::error::{Error, Result};
use crate::message::channel::{FromMidiBytes, ToMidiBytes};

/// Timing Clock (0xF8) — sent 24 times per quarter note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimingClock;

impl ToMidiBytes for TimingClock {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xF8]
    }
}

impl FromMidiBytes for TimingClock {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes[0] != 0xF8 {
            return Err(Error::InvalidMessage(
                "expected TimingClock (0xF8)".to_string(),
            ));
        }
        Ok(Self)
    }
}

/// Start (0xFA) — starts playback from the beginning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Start;

impl ToMidiBytes for Start {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xFA]
    }
}

impl FromMidiBytes for Start {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes[0] != 0xFA {
            return Err(Error::InvalidMessage("expected Start (0xFA)".to_string()));
        }
        Ok(Self)
    }
}

/// Continue (0xFB) — resumes playback from the current position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Continue;

impl ToMidiBytes for Continue {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xFB]
    }
}

impl FromMidiBytes for Continue {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes[0] != 0xFB {
            return Err(Error::InvalidMessage(
                "expected Continue (0xFB)".to_string(),
            ));
        }
        Ok(Self)
    }
}

/// Stop (0xFC) — stops playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stop;

impl ToMidiBytes for Stop {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xFC]
    }
}

impl FromMidiBytes for Stop {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes[0] != 0xFC {
            return Err(Error::InvalidMessage("expected Stop (0xFC)".to_string()));
        }
        Ok(Self)
    }
}

/// Active Sensing (0xFE) — keep-alive heartbeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveSensing;

impl ToMidiBytes for ActiveSensing {
    fn to_midi_bytes(&self) -> Vec<u8> {
        vec![0xFE]
    }
}

impl FromMidiBytes for ActiveSensing {
    fn from_midi_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes[0] != 0xFE {
            return Err(Error::InvalidMessage(
                "expected ActiveSensing (0xFE)".to_string(),
            ));
        }
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_clock_byte() {
        assert_eq!(TimingClock.to_midi_bytes(), vec![0xF8]);
        assert!(TimingClock::from_midi_bytes(&[0xF8]).is_ok());
        assert!(TimingClock::from_midi_bytes(&[0xF9]).is_err());
        assert!(TimingClock::from_midi_bytes(&[]).is_err());
    }

    #[test]
    fn start_byte() {
        assert_eq!(Start.to_midi_bytes(), vec![0xFA]);
        assert!(Start::from_midi_bytes(&[0xFA]).is_ok());
        assert!(Start::from_midi_bytes(&[0xFB]).is_err());
        assert!(Start::from_midi_bytes(&[]).is_err());
    }

    #[test]
    fn continue_byte() {
        assert_eq!(Continue.to_midi_bytes(), vec![0xFB]);
        assert!(Continue::from_midi_bytes(&[0xFB]).is_ok());
        assert!(Continue::from_midi_bytes(&[0xFA]).is_err());
        assert!(Continue::from_midi_bytes(&[]).is_err());
    }

    #[test]
    fn stop_byte() {
        assert_eq!(Stop.to_midi_bytes(), vec![0xFC]);
        assert!(Stop::from_midi_bytes(&[0xFC]).is_ok());
        assert!(Stop::from_midi_bytes(&[0xFD]).is_err());
        assert!(Stop::from_midi_bytes(&[]).is_err());
    }

    #[test]
    fn active_sensing_byte() {
        assert_eq!(ActiveSensing.to_midi_bytes(), vec![0xFE]);
        assert!(ActiveSensing::from_midi_bytes(&[0xFE]).is_ok());
        assert!(ActiveSensing::from_midi_bytes(&[0xFF]).is_err());
        assert!(ActiveSensing::from_midi_bytes(&[]).is_err());
    }
}
