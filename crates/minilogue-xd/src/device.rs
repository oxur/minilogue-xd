//! Device discovery, port constants, and CC name lookup for the Korg
//! Minilogue XD.
//!
//! This module provides:
//! - **Constants** for known USB MIDI port names and internal client strings
//! - **Discovery functions** to enumerate MIDI ports and locate the Minilogue
//!   XD automatically (behind the `midi-io` feature flag)
//! - **CC name lookup** mapping MIDI CC numbers to Minilogue XD parameter names

#[cfg(feature = "midi-io")]
use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Port name patterns (USB MIDI)
// ---------------------------------------------------------------------------

/// The USB MIDI output port that drives the Minilogue XD sound engine
/// directly (bypasses the internal MIDI router).
pub const OUTPUT_PORT_SOUND: &str = "minilogue xd SOUND";

/// The USB MIDI output port that routes through the Minilogue XD's MIDI
/// implementation (channel filtering, local on/off, etc.).
pub const OUTPUT_PORT_MIDI: &str = "minilogue xd MIDI OUT";

/// The USB MIDI input port for receiving data from the Minilogue XD.
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

/// Finds the Minilogue XD's preferred output port.
///
/// Searches for the SOUND port first (direct to sound engine), falling back
/// to the MIDI OUT port. Returns `Ok(None)` if no matching port is found.
///
/// # Errors
///
/// Returns [`Error::MidiIo`] if the MIDI subsystem cannot be initialized.
#[cfg(feature = "midi-io")]
pub fn find_output_port() -> Result<Option<String>> {
    let ports = list_output_ports()?;
    // Prefer SOUND port (direct to engine)
    if let Some(p) = ports.iter().find(|n| n.contains(OUTPUT_PORT_SOUND)) {
        return Ok(Some(p.clone()));
    }
    // Fall back to MIDI OUT
    if let Some(p) = ports.iter().find(|n| n.contains(OUTPUT_PORT_MIDI)) {
        return Ok(Some(p.clone()));
    }
    Ok(None)
}

/// Finds the Minilogue XD's input port.
///
/// Returns `Ok(None)` if no matching port is found.
///
/// # Errors
///
/// Returns [`Error::MidiIo`] if the MIDI subsystem cannot be initialized.
#[cfg(feature = "midi-io")]
pub fn find_input_port() -> Result<Option<String>> {
    let ports = list_input_ports()?;
    if let Some(p) = ports.iter().find(|n| n.contains(INPUT_PORT_MIDI)) {
        return Ok(Some(p.clone()));
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// CC name lookup
// ---------------------------------------------------------------------------

/// Returns the Minilogue XD parameter name for a MIDI CC number, if known.
///
/// The Minilogue XD uses 49 CC numbers to expose its front-panel parameters
/// over MIDI. This function maps each recognized CC to a human-readable
/// parameter name.
///
/// # Examples
///
/// ```
/// use minilogue_xd::device::cc_name;
///
/// assert_eq!(cc_name(43), Some("Cutoff"));
/// assert_eq!(cc_name(44), Some("Resonance"));
/// assert_eq!(cc_name(3), None); // not used by the XD
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // CC name lookup
    // ---------------------------------------------------------------

    #[test]
    fn test_cc_name_cutoff() {
        assert_eq!(cc_name(43), Some("Cutoff"));
    }

    #[test]
    fn test_cc_name_resonance() {
        assert_eq!(cc_name(44), Some("Resonance"));
    }

    #[test]
    fn test_cc_name_vco1_wave() {
        assert_eq!(cc_name(50), Some("VCO 1 Wave"));
    }

    #[test]
    fn test_cc_name_unknown_3() {
        assert_eq!(cc_name(3), None);
    }

    #[test]
    fn test_cc_name_unknown_100() {
        assert_eq!(cc_name(100), None);
    }

    #[test]
    fn test_cc_name_unknown_127() {
        assert_eq!(cc_name(127), None);
    }

    #[test]
    fn test_cc_name_exhaustive_count() {
        let count = (0..=127).filter(|&cc| cc_name(cc).is_some()).count();
        assert_eq!(count, 63, "expected exactly 63 known CC mappings");
    }

    // ---------------------------------------------------------------
    // Constants
    // ---------------------------------------------------------------

    #[test]
    fn test_constants_non_empty() {
        assert!(!OUTPUT_PORT_SOUND.is_empty());
        assert!(!OUTPUT_PORT_MIDI.is_empty());
        assert!(!INPUT_PORT_MIDI.is_empty());
        assert!(!PRODUCT_NAME.is_empty());
    }

    #[cfg(feature = "midi-io")]
    #[test]
    fn test_midi_io_constants_non_empty() {
        assert!(!MIDI_CLIENT_NAME.is_empty());
        assert!(!MIDI_OUT_PORT_NAME.is_empty());
        assert!(!MIDI_IN_PORT_NAME.is_empty());
    }

    #[test]
    fn test_output_port_sound_contains_sound() {
        assert!(
            OUTPUT_PORT_SOUND.contains("SOUND"),
            "OUTPUT_PORT_SOUND should contain 'SOUND'"
        );
    }

    #[test]
    fn test_output_port_midi_contains_midi() {
        assert!(
            OUTPUT_PORT_MIDI.contains("MIDI"),
            "OUTPUT_PORT_MIDI should contain 'MIDI'"
        );
    }

    #[test]
    fn test_input_port_midi_contains_midi() {
        assert!(
            INPUT_PORT_MIDI.contains("MIDI"),
            "INPUT_PORT_MIDI should contain 'MIDI'"
        );
    }

    // ---------------------------------------------------------------
    // Discovery (requires hardware — ignored in CI)
    // ---------------------------------------------------------------

    #[cfg(feature = "midi-io")]
    #[test]
    #[ignore]
    fn test_list_output_ports_succeeds() {
        let ports = list_output_ports().expect("list_output_ports should not error");
        // On CI without hardware this will be empty; that's fine.
        let _ = ports;
    }

    #[cfg(feature = "midi-io")]
    #[test]
    #[ignore]
    fn test_list_input_ports_succeeds() {
        let ports = list_input_ports().expect("list_input_ports should not error");
        let _ = ports;
    }
}
