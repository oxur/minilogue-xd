//! Device discovery, port types, and CC name lookup for the Korg Minilogue XD.
//!
//! The Minilogue XD exposes **four USB MIDI ports**, named from the device's
//! perspective:
//!
//! | Direction | Port | Purpose |
//! |-----------|------|---------|
//! | Output (we send to) | **SOUND** | Direct to the synth engine — notes, CCs, SysEx |
//! | Output (we send to) | **MIDI OUT** | Forwarded to the physical MIDI OUT jack |
//! | Input (we hear from) | **KBD/KNOB** | Keyboard, knobs, and SysEx responses |
//! | Input (we hear from) | **MIDI IN** | Data arriving on the physical MIDI IN jack |
//!
//! For **real-time control** (notes, CCs): send on SOUND.
//! For **SysEx transactions**: send on SOUND, listen on KBD/KNOB.
//! For **external MIDI routing**: use MIDI OUT / MIDI IN.
//!
//! Use [`OutputPort`] and [`InputPort`] enums to select ports without
//! hardcoding strings.

use std::fmt;

#[cfg(feature = "midi-io")]
use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Port enums
// ---------------------------------------------------------------------------

/// An output port on the Minilogue XD (a destination we send MIDI data to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutputPort {
    /// Direct to the synth engine — notes, CCs, and SysEx requests.
    /// This is the preferred port for real-time control and SysEx.
    Sound,
    /// Forwarded to the physical MIDI OUT jack on the back panel.
    /// Use this for routing to external MIDI devices.
    MidiOut,
}

impl OutputPort {
    /// The USB MIDI port name substring to search for.
    pub fn port_name_pattern(self) -> &'static str {
        match self {
            Self::Sound => "minilogue xd SOUND",
            Self::MidiOut => "minilogue xd MIDI OUT",
        }
    }

    /// All output port variants in preference order (Sound first).
    pub const ALL: &'static [OutputPort] = &[OutputPort::Sound, OutputPort::MidiOut];
}

impl fmt::Display for OutputPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sound => write!(f, "SOUND"),
            Self::MidiOut => write!(f, "MIDI OUT"),
        }
    }
}

/// An input port on the Minilogue XD (a source we receive MIDI data from).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputPort {
    /// Keyboard, knob, and SysEx responses from the synth engine.
    /// This is the port for receiving SysEx dump responses.
    KbdKnob,
    /// Data arriving on the physical MIDI IN jack.
    /// Use this for receiving from external MIDI devices.
    MidiIn,
}

impl InputPort {
    /// The USB MIDI port name substring to search for.
    pub fn port_name_pattern(self) -> &'static str {
        match self {
            Self::KbdKnob => "minilogue xd KBD/KNOB",
            Self::MidiIn => "minilogue xd MIDI IN",
        }
    }

    /// All input port variants in preference order (KbdKnob first).
    pub const ALL: &'static [InputPort] = &[InputPort::KbdKnob, InputPort::MidiIn];
}

impl fmt::Display for InputPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KbdKnob => write!(f, "KBD/KNOB"),
            Self::MidiIn => write!(f, "MIDI IN"),
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy constants (kept for backward compatibility, prefer the enums above)
// ---------------------------------------------------------------------------

/// The USB MIDI output port that drives the synth engine directly.
pub const OUTPUT_PORT_SOUND: &str = "minilogue xd SOUND";

/// The USB MIDI output port that routes to the physical MIDI OUT jack.
pub const OUTPUT_PORT_MIDI: &str = "minilogue xd MIDI OUT";

/// The USB MIDI input port for keyboard/knob data and SysEx responses.
pub const INPUT_PORT_KBD_KNOB: &str = "minilogue xd KBD/KNOB";

/// The USB MIDI input port for data from the physical MIDI IN jack.
pub const INPUT_PORT_MIDI: &str = "minilogue xd MIDI IN";

/// The product name used in file metadata (e.g., `.mnlgxdprog` XML headers).
pub const PRODUCT_NAME: &str = "minilogue xd";

// ---------------------------------------------------------------------------
// Internal client / connection names (used by midir)
// ---------------------------------------------------------------------------

/// Client name passed to `midir::MidiOutput::new` / `midir::MidiInput::new`.
#[cfg(feature = "midi-io")]
pub(crate) const MIDI_CLIENT_NAME: &str = "minilogue-xd";

/// Connection name used when opening an output port via `midir`.
#[cfg(feature = "midi-io")]
pub(crate) const MIDI_OUT_PORT_NAME: &str = "minilogue-xd-out";

/// Connection name used when opening an input port via `midir`.
#[cfg(feature = "midi-io")]
pub(crate) const MIDI_IN_PORT_NAME: &str = "minilogue-xd-in";

// ---------------------------------------------------------------------------
// Device discovery (feature-gated)
// ---------------------------------------------------------------------------

/// Lists all available MIDI output port names.
///
/// # Errors
///
/// Returns [`Error::MidiIo`] if the MIDI subsystem cannot be initialized.
#[cfg(feature = "midi-io")]
pub fn list_output_ports() -> Result<Vec<String>> {
    let midi_out =
        midir::MidiOutput::new(MIDI_CLIENT_NAME).map_err(|e| Error::MidiIo(e.to_string()))?;
    let ports = midi_out.ports();
    Ok(ports
        .iter()
        .filter_map(|p| midi_out.port_name(p).ok())
        .collect())
}

/// Lists all available MIDI input port names.
///
/// # Errors
///
/// Returns [`Error::MidiIo`] if the MIDI subsystem cannot be initialized.
#[cfg(feature = "midi-io")]
pub fn list_input_ports() -> Result<Vec<String>> {
    let midi_in =
        midir::MidiInput::new(MIDI_CLIENT_NAME).map_err(|e| Error::MidiIo(e.to_string()))?;
    let ports = midi_in.ports();
    Ok(ports
        .iter()
        .filter_map(|p| midi_in.port_name(p).ok())
        .collect())
}

/// Finds a Minilogue XD output port, preferring the given port type.
///
/// If the preferred port is not found, falls back to the other output port.
/// Returns `Ok(None)` if no Minilogue XD output port is found.
///
/// # Examples
///
/// ```rust,ignore
/// // Prefer SOUND for real-time control and SysEx
/// let port = device::find_output(OutputPort::Sound)?;
///
/// // Prefer MIDI OUT for routing to external gear
/// let port = device::find_output(OutputPort::MidiOut)?;
/// ```
#[cfg(feature = "midi-io")]
pub fn find_output(prefer: OutputPort) -> Result<Option<String>> {
    let ports = list_output_ports()?;
    // Try preferred first
    if let Some(p) = ports
        .iter()
        .find(|n| n.contains(prefer.port_name_pattern()))
    {
        return Ok(Some(p.clone()));
    }
    // Fall back to any other XD output port
    for variant in OutputPort::ALL {
        if *variant != prefer {
            if let Some(p) = ports
                .iter()
                .find(|n| n.contains(variant.port_name_pattern()))
            {
                return Ok(Some(p.clone()));
            }
        }
    }
    Ok(None)
}

/// Finds a Minilogue XD input port, preferring the given port type.
///
/// If the preferred port is not found, falls back to the other input port.
/// Returns `Ok(None)` if no Minilogue XD input port is found.
///
/// # Examples
///
/// ```rust,ignore
/// // Prefer KBD/KNOB for SysEx responses
/// let port = device::find_input(InputPort::KbdKnob)?;
///
/// // Prefer MIDI IN for external MIDI data
/// let port = device::find_input(InputPort::MidiIn)?;
/// ```
#[cfg(feature = "midi-io")]
pub fn find_input(prefer: InputPort) -> Result<Option<String>> {
    let ports = list_input_ports()?;
    // Try preferred first
    if let Some(p) = ports
        .iter()
        .find(|n| n.contains(prefer.port_name_pattern()))
    {
        return Ok(Some(p.clone()));
    }
    // Fall back to any other XD input port
    for variant in InputPort::ALL {
        if *variant != prefer {
            if let Some(p) = ports
                .iter()
                .find(|n| n.contains(variant.port_name_pattern()))
            {
                return Ok(Some(p.clone()));
            }
        }
    }
    Ok(None)
}

/// Finds the preferred output port (SOUND) for real-time control.
///
/// Convenience wrapper for `find_output(OutputPort::Sound)`.
#[cfg(feature = "midi-io")]
pub fn find_output_port() -> Result<Option<String>> {
    find_output(OutputPort::Sound)
}

/// Finds the preferred input port (KBD/KNOB) for SysEx responses.
///
/// Convenience wrapper for `find_input(InputPort::KbdKnob)`.
#[cfg(feature = "midi-io")]
pub fn find_input_port() -> Result<Option<String>> {
    find_input(InputPort::KbdKnob)
}

// ---------------------------------------------------------------------------
// CC name lookup
// ---------------------------------------------------------------------------

/// Returns the Minilogue XD parameter name for a MIDI CC number, if known.
///
/// The Minilogue XD uses 63 CC numbers to expose its front-panel parameters
/// over MIDI. This function maps each recognized CC to a human-readable
/// parameter name.
///
/// Returns `None` for unrecognized CC numbers.
pub fn cc_name(cc: u8) -> Option<&'static str> {
    match cc {
        0 => Some("Bank Select MSB"),
        1 => Some("Modulation 1 (Joystick +Y)"),
        2 => Some("Modulation 2 (Joystick -Y)"),
        5 => Some("Portamento Time"),
        6 => Some("Data Entry MSB"),
        16 => Some("Amp EG Attack"),
        17 => Some("Amp EG Decay"),
        18 => Some("Amp EG Sustain"),
        19 => Some("Amp EG Release"),
        20 => Some("EG Attack"),
        21 => Some("EG Decay"),
        22 => Some("EG Int"),
        23 => Some("EG Target"),
        24 => Some("LFO Rate"),
        26 => Some("LFO Int"),
        27 => Some("Voice Mode Depth"),
        28 => Some("Mod FX Time"),
        29 => Some("Mod FX Depth"),
        32 => Some("Bank Select LSB"),
        33 => Some("Multi Level"),
        34 => Some("VCO 1 Pitch"),
        35 => Some("VCO 2 Pitch"),
        36 => Some("VCO 1 Shape"),
        37 => Some("VCO 2 Shape"),
        39 => Some("VCO 1 Level"),
        40 => Some("VCO 2 Level"),
        41 => Some("Cross Mod Depth"),
        43 => Some("Cutoff"),
        44 => Some("Resonance"),
        48 => Some("VCO 1 Octave"),
        49 => Some("VCO 2 Octave"),
        50 => Some("VCO 1 Wave"),
        51 => Some("VCO 2 Wave"),
        53 => Some("Multi Type"),
        54 => Some("Multi Shape"),
        56 => Some("LFO Target"),
        57 => Some("LFO Wave"),
        58 => Some("LFO Mode"),
        59 => Some("Voice Mode Depth (alt)"),
        63 => Some("Data Entry LSB (10-bit low 3 bits)"),
        64 => Some("Damper / Hold"),
        80 => Some("Sync"),
        81 => Some("Ring"),
        83 => Some("Cutoff Keytrack"),
        84 => Some("Cutoff Drive"),
        88 => Some("Mod FX Type"),
        89 => Some("Delay Sub Type"),
        90 => Some("Reverb Sub Type"),
        92 => Some("Mod FX On/Off"),
        93 => Some("Delay On/Off"),
        94 => Some("Reverb On/Off"),
        96 => Some("Mod FX Sub Type"),
        98 => Some("NRPN LSB"),
        99 => Some("NRPN MSB"),
        103 => Some("Multi Select"),
        104 => Some("Multi Shift Shape"),
        105 => Some("Delay Time"),
        106 => Some("Delay Depth"),
        107 => Some("Delay Dry/Wet"),
        108 => Some("Reverb Time"),
        109 => Some("Reverb Depth"),
        110 => Some("Reverb Dry/Wet"),
        118 => Some("CV In 1"),
        119 => Some("CV In 2"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Port enums ---

    #[test]
    fn output_port_sound_pattern() {
        assert!(OutputPort::Sound.port_name_pattern().contains("SOUND"));
    }

    #[test]
    fn output_port_midi_pattern() {
        assert!(OutputPort::MidiOut.port_name_pattern().contains("MIDI OUT"));
    }

    #[test]
    fn input_port_kbd_knob_pattern() {
        assert!(InputPort::KbdKnob.port_name_pattern().contains("KBD/KNOB"));
    }

    #[test]
    fn input_port_midi_pattern() {
        assert!(InputPort::MidiIn.port_name_pattern().contains("MIDI IN"));
    }

    #[test]
    fn output_port_display() {
        assert_eq!(OutputPort::Sound.to_string(), "SOUND");
        assert_eq!(OutputPort::MidiOut.to_string(), "MIDI OUT");
    }

    #[test]
    fn input_port_display() {
        assert_eq!(InputPort::KbdKnob.to_string(), "KBD/KNOB");
        assert_eq!(InputPort::MidiIn.to_string(), "MIDI IN");
    }

    #[test]
    fn output_port_all_ordering() {
        assert_eq!(OutputPort::ALL, &[OutputPort::Sound, OutputPort::MidiOut]);
    }

    #[test]
    fn input_port_all_ordering() {
        assert_eq!(InputPort::ALL, &[InputPort::KbdKnob, InputPort::MidiIn]);
    }

    // --- Constants ---

    #[test]
    fn constants_non_empty() {
        assert!(!OUTPUT_PORT_SOUND.is_empty());
        assert!(!OUTPUT_PORT_MIDI.is_empty());
        assert!(!INPUT_PORT_KBD_KNOB.is_empty());
        assert!(!INPUT_PORT_MIDI.is_empty());
        assert!(!PRODUCT_NAME.is_empty());
    }

    #[cfg(feature = "midi-io")]
    #[test]
    fn midi_io_constants_non_empty() {
        assert!(!MIDI_CLIENT_NAME.is_empty());
        assert!(!MIDI_OUT_PORT_NAME.is_empty());
        assert!(!MIDI_IN_PORT_NAME.is_empty());
    }

    // --- Discovery (ignored on CI — no hardware) ---

    #[cfg(feature = "midi-io")]
    #[test]
    #[ignore]
    fn list_output_ports_returns_vec() {
        let ports = list_output_ports().unwrap();
        println!("Output ports: {ports:?}");
    }

    #[cfg(feature = "midi-io")]
    #[test]
    #[ignore]
    fn list_input_ports_returns_vec() {
        let ports = list_input_ports().unwrap();
        println!("Input ports: {ports:?}");
    }

    // --- CC names ---

    #[test]
    fn cc_name_known() {
        assert_eq!(cc_name(43), Some("Cutoff"));
        assert_eq!(cc_name(44), Some("Resonance"));
        assert_eq!(cc_name(50), Some("VCO 1 Wave"));
        assert_eq!(cc_name(93), Some("Delay On/Off"));
    }

    #[test]
    fn cc_name_unknown() {
        assert_eq!(cc_name(3), None);
        assert_eq!(cc_name(100), None);
        assert_eq!(cc_name(127), None);
    }

    #[test]
    fn cc_name_count() {
        let count = (0..=127).filter(|&cc| cc_name(cc).is_some()).count();
        assert_eq!(count, 64, "expected 64 known CCs");
    }
}
