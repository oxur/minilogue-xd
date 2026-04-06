//! Real-time parameter controller for the Minilogue XD.
//!
//! [`RealtimeController`] wraps any [`MidiOutput`] and provides typed methods
//! for every parameter, handling the low-level CC encoding (including the
//! CC63-preceded 10-bit protocol) automatically.

use crate::error::{Error, Result};
use crate::message::channel::{ControlChange, ToMidiBytes};
use crate::message::types::{I14, U4, U7};
use crate::param::encoding::TenBitParam;
use crate::param::enums::{
    CutoffDrive, CutoffKeytrack, DelaySubType, EgTarget, LfoMode, LfoTarget, LfoWave, MicroTuning,
    ModFxType, MultiType, ReverbSubType, Ring, Sync, VcoOctave, VcoWave,
};
use crate::param::nrpn::NrpnParam;
use crate::param::SteppedParam;
use crate::sysex::program::ProgramNumber;
use crate::transport::MidiOutput;

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Converts a float in 0.0..=1.0 to a 10-bit parameter value (0..=1023).
///
/// # Errors
///
/// Returns [`Error::OutOfRange`] if `value` is outside 0.0..=1.0.
fn f32_to_10bit(value: f32) -> Result<TenBitParam> {
    if !(0.0..=1.0).contains(&value) {
        return Err(Error::OutOfRange {
            type_name: "f32 (0.0-1.0)",
            value: (value * 1000.0) as i64,
            min: 0,
            max: 1000,
        });
    }
    TenBitParam::new((value * 1023.0).round() as u16)
}

/// Converts a float in -1.0..=1.0 to a signed 14-bit pitch bend value
/// (-8192..=8191).
///
/// # Errors
///
/// Returns [`Error::OutOfRange`] if `value` is outside -1.0..=1.0.
fn f32_to_pitch_bend(value: f32) -> Result<I14> {
    if !(-1.0..=1.0).contains(&value) {
        return Err(Error::OutOfRange {
            type_name: "f32 (-1.0-1.0)",
            value: (value * 1000.0) as i64,
            min: -1000,
            max: 1000,
        });
    }
    I14::new((value * 8191.0).round() as i16)
}

// ---------------------------------------------------------------------------
// RealtimeController
// ---------------------------------------------------------------------------

/// A typed real-time controller for the Minilogue XD.
///
/// Provides ergonomic methods for every parameter, handling the encoding
/// details (10-bit CC63 protocol, stepped enums, on/off switches, NRPN
/// sequences) internally. All methods that send MIDI return `Result`.
///
/// # Examples
///
/// ```
/// use minilogue_xd::controller::RealtimeController;
/// use minilogue_xd::message::types::U4;
/// use minilogue_xd::transport::MockOutput;
///
/// let output = MockOutput::new();
/// let channel = U4::new(0).unwrap();
/// let mut ctrl = RealtimeController::new(output, channel);
///
/// ctrl.set_cutoff(0.75).unwrap();
/// ctrl.set_vco1_wave(minilogue_xd::param::enums::VcoWave::Saw).unwrap();
/// ctrl.set_mod_fx_on(true).unwrap();
/// ```
pub struct RealtimeController<O: MidiOutput> {
    output: O,
    channel: U4,
}

impl<O: MidiOutput> RealtimeController<O> {
    /// Creates a new controller using the given output and MIDI channel.
    pub fn new(output: O, channel: U4) -> Self {
        Self { output, channel }
    }

    /// Returns the MIDI channel this controller is configured for.
    pub fn channel(&self) -> U4 {
        self.channel
    }

    /// Returns a reference to the underlying output.
    pub fn output(&self) -> &O {
        &self.output
    }

    /// Returns a mutable reference to the underlying output.
    pub fn output_mut(&mut self) -> &mut O {
        &mut self.output
    }

    // -- Internal helpers --

    /// Sends a single CC message.
    fn send_cc(&mut self, controller: u8, value: u8) -> Result<()> {
        let msg = ControlChange {
            channel: self.channel,
            controller: U7::new(controller)?,
            value: U7::new(value)?,
        };
        self.output.send(&msg.to_midi_bytes())
    }

    /// Sends a 10-bit value as CC63(lsb) + CC_N(msb).
    fn send_10bit_cc(&mut self, cc_number: u8, value: f32) -> Result<()> {
        let ten = f32_to_10bit(value)?;
        self.send_cc(63, ten.lsb())?;
        self.send_cc(cc_number, ten.msb())
    }

    /// Sends a stepped enum param via CC.
    fn send_stepped_cc<P: SteppedParam>(&mut self, cc_number: u8, param: P) -> Result<()> {
        self.send_cc(cc_number, param.to_tx_value())
    }

    /// Sends an on/off param via CC (0 = off, 127 = on).
    fn send_on_off_cc(&mut self, cc_number: u8, on: bool) -> Result<()> {
        self.send_cc(cc_number, if on { 127 } else { 0 })
    }

    /// Sends an NRPN parameter as a CC sequence.
    fn send_nrpn(&mut self, param: &NrpnParam) -> Result<()> {
        let msgs = param.to_midi_sequence(self.channel)?;
        for msg in &msgs {
            self.output.send(&msg.to_midi_bytes())?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 10-bit CC parameters (float 0.0..=1.0)
    // -----------------------------------------------------------------------

    /// Sets the Amp EG attack time (CC 16, 10-bit).
    ///
    /// `value` is 0.0 (instant) to 1.0 (maximum), mapped to 0--1023.
    pub fn set_amp_eg_attack(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(16, value)
    }

    /// Sets the Amp EG decay time (CC 17, 10-bit).
    pub fn set_amp_eg_decay(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(17, value)
    }

    /// Sets the Amp EG sustain level (CC 18, 10-bit).
    pub fn set_amp_eg_sustain(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(18, value)
    }

    /// Sets the Amp EG release time (CC 19, 10-bit).
    pub fn set_amp_eg_release(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(19, value)
    }

    /// Sets the EG attack time (CC 20, 10-bit).
    pub fn set_eg_attack(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(20, value)
    }

    /// Sets the EG decay time (CC 21, 10-bit).
    pub fn set_eg_decay(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(21, value)
    }

    /// Sets the EG intensity (CC 22, 10-bit).
    pub fn set_eg_int(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(22, value)
    }

    /// Sets the LFO rate (CC 24, 10-bit).
    pub fn set_lfo_rate(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(24, value)
    }

    /// Sets the LFO intensity (CC 26, 10-bit).
    pub fn set_lfo_int(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(26, value)
    }

    /// Sets the voice mode depth (CC 27, 10-bit).
    pub fn set_voice_mode_depth(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(27, value)
    }

    /// Sets the mod-FX time (CC 28, 10-bit).
    pub fn set_mod_fx_time(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(28, value)
    }

    /// Sets the mod-FX depth (CC 29, 10-bit).
    pub fn set_mod_fx_depth(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(29, value)
    }

    /// Sets the multi-engine level (CC 33, 10-bit).
    pub fn set_multi_level(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(33, value)
    }

    /// Sets VCO1 pitch (CC 34, 10-bit).
    pub fn set_vco1_pitch(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(34, value)
    }

    /// Sets VCO2 pitch (CC 35, 10-bit).
    pub fn set_vco2_pitch(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(35, value)
    }

    /// Sets VCO1 shape (CC 36, 10-bit).
    pub fn set_vco1_shape(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(36, value)
    }

    /// Sets VCO2 shape (CC 37, 10-bit).
    pub fn set_vco2_shape(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(37, value)
    }

    /// Sets VCO1 level (CC 39, 10-bit).
    pub fn set_vco1_level(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(39, value)
    }

    /// Sets VCO2 level (CC 40, 10-bit).
    pub fn set_vco2_level(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(40, value)
    }

    /// Sets cross-modulation depth (CC 41, 10-bit).
    pub fn set_cross_mod_depth(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(41, value)
    }

    /// Sets filter cutoff (CC 43, 10-bit).
    ///
    /// `value` is 0.0 (fully closed) to 1.0 (fully open), mapped to 0--1023.
    pub fn set_cutoff(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(43, value)
    }

    /// Sets filter resonance (CC 44, 10-bit).
    pub fn set_resonance(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(44, value)
    }

    /// Sets multi-engine shape (CC 54, 10-bit).
    pub fn set_multi_shape(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(54, value)
    }

    /// Sets voice mode depth alternate (CC 59, 10-bit, no display update).
    pub fn set_voice_mode_depth_alt(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(59, value)
    }

    /// Sets multi-engine shift shape (CC 104, 10-bit).
    pub fn set_multi_shift_shape(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(104, value)
    }

    /// Sets delay time (CC 105, 10-bit).
    pub fn set_delay_time(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(105, value)
    }

    /// Sets delay depth (CC 106, 10-bit).
    pub fn set_delay_depth(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(106, value)
    }

    /// Sets delay dry/wet (CC 107, 10-bit).
    pub fn set_delay_dry_wet(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(107, value)
    }

    /// Sets reverb time (CC 108, 10-bit).
    pub fn set_reverb_time(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(108, value)
    }

    /// Sets reverb depth (CC 109, 10-bit).
    pub fn set_reverb_depth(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(109, value)
    }

    /// Sets reverb dry/wet (CC 110, 10-bit).
    pub fn set_reverb_dry_wet(&mut self, value: f32) -> Result<()> {
        self.send_10bit_cc(110, value)
    }

    // -----------------------------------------------------------------------
    // Stepped CC parameters (enum-valued)
    // -----------------------------------------------------------------------

    /// Sets the EG target (CC 23).
    pub fn set_eg_target(&mut self, target: EgTarget) -> Result<()> {
        self.send_stepped_cc(23, target)
    }

    /// Sets VCO1 octave (CC 48).
    pub fn set_vco1_octave(&mut self, octave: VcoOctave) -> Result<()> {
        self.send_stepped_cc(48, octave)
    }

    /// Sets VCO2 octave (CC 49).
    pub fn set_vco2_octave(&mut self, octave: VcoOctave) -> Result<()> {
        self.send_stepped_cc(49, octave)
    }

    /// Sets VCO1 waveform (CC 50).
    pub fn set_vco1_wave(&mut self, wave: VcoWave) -> Result<()> {
        self.send_stepped_cc(50, wave)
    }

    /// Sets VCO2 waveform (CC 51).
    pub fn set_vco2_wave(&mut self, wave: VcoWave) -> Result<()> {
        self.send_stepped_cc(51, wave)
    }

    /// Sets multi-engine type (CC 53).
    pub fn set_multi_type(&mut self, multi: MultiType) -> Result<()> {
        self.send_stepped_cc(53, multi)
    }

    /// Sets LFO target (CC 56).
    pub fn set_lfo_target(&mut self, target: LfoTarget) -> Result<()> {
        self.send_stepped_cc(56, target)
    }

    /// Sets LFO waveform (CC 57).
    pub fn set_lfo_wave(&mut self, wave: LfoWave) -> Result<()> {
        self.send_stepped_cc(57, wave)
    }

    /// Sets LFO mode (CC 58).
    pub fn set_lfo_mode(&mut self, mode: LfoMode) -> Result<()> {
        self.send_stepped_cc(58, mode)
    }

    /// Sets oscillator sync on/off (CC 80).
    pub fn set_sync(&mut self, sync: Sync) -> Result<()> {
        self.send_stepped_cc(80, sync)
    }

    /// Sets ring modulation on/off (CC 81).
    pub fn set_ring(&mut self, ring: Ring) -> Result<()> {
        self.send_stepped_cc(81, ring)
    }

    /// Sets cutoff key-tracking (CC 83).
    pub fn set_cutoff_keytrack(&mut self, keytrack: CutoffKeytrack) -> Result<()> {
        self.send_stepped_cc(83, keytrack)
    }

    /// Sets cutoff drive (CC 84).
    pub fn set_cutoff_drive(&mut self, drive: CutoffDrive) -> Result<()> {
        self.send_stepped_cc(84, drive)
    }

    /// Sets mod-FX type (CC 88).
    pub fn set_mod_fx_type(&mut self, fx_type: ModFxType) -> Result<()> {
        self.send_stepped_cc(88, fx_type)
    }

    /// Sets delay sub-type (CC 89).
    pub fn set_delay_sub_type(&mut self, sub_type: DelaySubType) -> Result<()> {
        self.send_stepped_cc(89, sub_type)
    }

    /// Sets reverb sub-type (CC 90).
    pub fn set_reverb_sub_type(&mut self, sub_type: ReverbSubType) -> Result<()> {
        self.send_stepped_cc(90, sub_type)
    }

    // -----------------------------------------------------------------------
    // On/Off CC parameters
    // -----------------------------------------------------------------------

    /// Sets mod-FX on/off (CC 92).
    pub fn set_mod_fx_on(&mut self, on: bool) -> Result<()> {
        self.send_on_off_cc(92, on)
    }

    /// Sets delay on/off (CC 93).
    pub fn set_delay_on(&mut self, on: bool) -> Result<()> {
        self.send_on_off_cc(93, on)
    }

    /// Sets reverb on/off (CC 94).
    pub fn set_reverb_on(&mut self, on: bool) -> Result<()> {
        self.send_on_off_cc(94, on)
    }

    // -----------------------------------------------------------------------
    // Simple continuous parameters
    // -----------------------------------------------------------------------

    /// Sets modulation wheel 1 (CC 1, 0--127).
    pub fn set_modulation1(&mut self, value: U7) -> Result<()> {
        self.send_cc(1, value.value())
    }

    /// Sets modulation wheel 2 (CC 2, 0--127).
    pub fn set_modulation2(&mut self, value: U7) -> Result<()> {
        self.send_cc(2, value.value())
    }

    /// Sets portamento time (CC 5, 0--127).
    pub fn set_portamento_time(&mut self, value: U7) -> Result<()> {
        self.send_cc(5, value.value())
    }

    // -----------------------------------------------------------------------
    // Note and transport methods
    // -----------------------------------------------------------------------

    /// Sends a Note On message.
    pub fn play_note(&mut self, note: U7, velocity: U7) -> Result<()> {
        let bytes = [0x90 | self.channel.value(), note.value(), velocity.value()];
        self.output.send(&bytes)
    }

    /// Sends a Note Off message (velocity 0).
    pub fn stop_note(&mut self, note: U7) -> Result<()> {
        let bytes = [0x80 | self.channel.value(), note.value(), 0];
        self.output.send(&bytes)
    }

    /// Sends a pitch bend message.
    ///
    /// `value` is -1.0 (maximum down) to +1.0 (maximum up), with 0.0 at center.
    pub fn pitch_bend(&mut self, value: f32) -> Result<()> {
        let bend = f32_to_pitch_bend(value)?;
        let wire = (i32::from(bend.value()) + 8192) as u16;
        let lsb = (wire & 0x7F) as u8;
        let msb = ((wire >> 7) & 0x7F) as u8;
        let bytes = [0xE0 | self.channel.value(), lsb, msb];
        self.output.send(&bytes)
    }

    /// Sends a program change with bank select.
    ///
    /// Sends CC0 (Bank MSB), CC32 (Bank LSB), then Program Change.
    pub fn program_change(&mut self, program: ProgramNumber) -> Result<()> {
        self.send_cc(0, program.bank())?;
        self.send_cc(32, program.slot_in_bank())?;
        let bytes = [0xC0 | self.channel.value(), program.slot_in_bank()];
        self.output.send(&bytes)
    }

    /// Sends All Notes Off (CC 123).
    pub fn all_notes_off(&mut self) -> Result<()> {
        self.send_cc(123, 0)
    }

    // -----------------------------------------------------------------------
    // NRPN methods
    // -----------------------------------------------------------------------

    /// Sets the pitch bend range (positive direction, 0--12 semitones).
    ///
    /// # Errors
    ///
    /// Returns an error if `semitones` exceeds 12.
    pub fn set_bend_range_plus(&mut self, semitones: u8) -> Result<()> {
        if semitones > 12 {
            return Err(Error::OutOfRange {
                type_name: "bend_range_plus",
                value: i64::from(semitones),
                min: 0,
                max: 12,
            });
        }
        self.send_nrpn(&NrpnParam::BendRangePlus(semitones))
    }

    /// Sets the pitch bend range (negative direction, 0--12 semitones).
    ///
    /// # Errors
    ///
    /// Returns an error if `semitones` exceeds 12.
    pub fn set_bend_range_minus(&mut self, semitones: u8) -> Result<()> {
        if semitones > 12 {
            return Err(Error::OutOfRange {
                type_name: "bend_range_minus",
                value: i64::from(semitones),
                min: 0,
                max: 12,
            });
        }
        self.send_nrpn(&NrpnParam::BendRangeMinus(semitones))
    }

    /// Sets the micro-tuning scale.
    pub fn set_micro_tuning(&mut self, tuning: MicroTuning) -> Result<()> {
        self.send_nrpn(&NrpnParam::MicroTuning(tuning))
    }

    /// Sets the program level.
    ///
    /// `db` is -18.0 to +6.0 dB, mapped to NRPN raw values 0--120.
    ///
    /// # Errors
    ///
    /// Returns an error if `db` is outside -18.0..=6.0.
    pub fn set_program_level(&mut self, db: f32) -> Result<()> {
        if !(-18.0..=6.0).contains(&db) {
            return Err(Error::OutOfRange {
                type_name: "program_level_db",
                value: (db * 10.0) as i64,
                min: -180,
                max: 60,
            });
        }
        // -18.0 dB = 0, +6.0 dB = 120. Linear mapping: raw = (db + 18) * 5.
        let raw = ((db + 18.0) * 5.0).round() as u8;
        self.send_nrpn(&NrpnParam::ProgramLevel(raw))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockOutput;

    fn make_ctrl() -> RealtimeController<MockOutput> {
        let output = MockOutput::new();
        let channel = U4::new(0).unwrap();
        RealtimeController::new(output, channel)
    }

    fn make_ctrl_ch(ch: u8) -> RealtimeController<MockOutput> {
        let output = MockOutput::new();
        let channel = U4::new(ch).unwrap();
        RealtimeController::new(output, channel)
    }

    // -- Accessors --

    #[test]
    fn channel_returns_configured_channel() {
        let ctrl = make_ctrl_ch(5);
        assert_eq!(ctrl.channel().value(), 5);
    }

    #[test]
    fn output_ref_accessible() {
        let ctrl = make_ctrl();
        assert!(ctrl.output().messages().is_empty());
    }

    #[test]
    fn output_mut_accessible() {
        let mut ctrl = make_ctrl();
        ctrl.output_mut().clear();
        assert!(ctrl.output().messages().is_empty());
    }

    // -- 10-bit CC methods --

    /// Helper: verify a 10-bit CC sends CC63(lsb) then CC_N(msb).
    fn assert_10bit_cc(ctrl: &RealtimeController<MockOutput>, cc_number: u8, value: f32) {
        let ten = f32_to_10bit(value).unwrap();
        let msgs = ctrl.output().messages();
        let len = msgs.len();
        assert!(len >= 2, "expected at least 2 messages, got {len}");
        // CC63 message
        assert_eq!(msgs[len - 2], vec![0xB0, 63, ten.lsb()]);
        // Param CC message
        assert_eq!(msgs[len - 1], vec![0xB0, cc_number, ten.msb()]);
    }

    #[test]
    fn set_cutoff_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_cutoff(0.5).unwrap();
        assert_10bit_cc(&ctrl, 43, 0.5);
    }

    #[test]
    fn set_cutoff_zero() {
        let mut ctrl = make_ctrl();
        ctrl.set_cutoff(0.0).unwrap();
        assert_10bit_cc(&ctrl, 43, 0.0);
    }

    #[test]
    fn set_cutoff_one() {
        let mut ctrl = make_ctrl();
        ctrl.set_cutoff(1.0).unwrap();
        assert_10bit_cc(&ctrl, 43, 1.0);
    }

    #[test]
    fn set_cutoff_out_of_range_high() {
        let mut ctrl = make_ctrl();
        assert!(ctrl.set_cutoff(1.1).is_err());
    }

    #[test]
    fn set_cutoff_out_of_range_low() {
        let mut ctrl = make_ctrl();
        assert!(ctrl.set_cutoff(-0.1).is_err());
    }

    #[test]
    fn set_resonance_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_resonance(0.75).unwrap();
        assert_10bit_cc(&ctrl, 44, 0.75);
    }

    #[test]
    fn set_amp_eg_attack_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_amp_eg_attack(0.25).unwrap();
        assert_10bit_cc(&ctrl, 16, 0.25);
    }

    #[test]
    fn set_amp_eg_decay_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_amp_eg_decay(0.5).unwrap();
        assert_10bit_cc(&ctrl, 17, 0.5);
    }

    #[test]
    fn set_amp_eg_sustain_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_amp_eg_sustain(1.0).unwrap();
        assert_10bit_cc(&ctrl, 18, 1.0);
    }

    #[test]
    fn set_amp_eg_release_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_amp_eg_release(0.0).unwrap();
        assert_10bit_cc(&ctrl, 19, 0.0);
    }

    #[test]
    fn set_eg_attack_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_eg_attack(0.3).unwrap();
        assert_10bit_cc(&ctrl, 20, 0.3);
    }

    #[test]
    fn set_eg_decay_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_eg_decay(0.7).unwrap();
        assert_10bit_cc(&ctrl, 21, 0.7);
    }

    #[test]
    fn set_eg_int_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_eg_int(0.5).unwrap();
        assert_10bit_cc(&ctrl, 22, 0.5);
    }

    #[test]
    fn set_lfo_rate_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_lfo_rate(0.5).unwrap();
        assert_10bit_cc(&ctrl, 24, 0.5);
    }

    #[test]
    fn set_lfo_int_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_lfo_int(0.8).unwrap();
        assert_10bit_cc(&ctrl, 26, 0.8);
    }

    #[test]
    fn set_voice_mode_depth_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_voice_mode_depth(0.5).unwrap();
        assert_10bit_cc(&ctrl, 27, 0.5);
    }

    #[test]
    fn set_mod_fx_time_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_mod_fx_time(0.4).unwrap();
        assert_10bit_cc(&ctrl, 28, 0.4);
    }

    #[test]
    fn set_mod_fx_depth_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_mod_fx_depth(0.6).unwrap();
        assert_10bit_cc(&ctrl, 29, 0.6);
    }

    #[test]
    fn set_multi_level_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_multi_level(0.5).unwrap();
        assert_10bit_cc(&ctrl, 33, 0.5);
    }

    #[test]
    fn set_vco1_pitch_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco1_pitch(0.5).unwrap();
        assert_10bit_cc(&ctrl, 34, 0.5);
    }

    #[test]
    fn set_vco2_pitch_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco2_pitch(0.5).unwrap();
        assert_10bit_cc(&ctrl, 35, 0.5);
    }

    #[test]
    fn set_vco1_shape_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco1_shape(0.5).unwrap();
        assert_10bit_cc(&ctrl, 36, 0.5);
    }

    #[test]
    fn set_vco2_shape_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco2_shape(0.5).unwrap();
        assert_10bit_cc(&ctrl, 37, 0.5);
    }

    #[test]
    fn set_vco1_level_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco1_level(0.5).unwrap();
        assert_10bit_cc(&ctrl, 39, 0.5);
    }

    #[test]
    fn set_vco2_level_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco2_level(0.5).unwrap();
        assert_10bit_cc(&ctrl, 40, 0.5);
    }

    #[test]
    fn set_cross_mod_depth_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_cross_mod_depth(0.5).unwrap();
        assert_10bit_cc(&ctrl, 41, 0.5);
    }

    #[test]
    fn set_multi_shape_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_multi_shape(0.5).unwrap();
        assert_10bit_cc(&ctrl, 54, 0.5);
    }

    #[test]
    fn set_voice_mode_depth_alt_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_voice_mode_depth_alt(0.5).unwrap();
        assert_10bit_cc(&ctrl, 59, 0.5);
    }

    #[test]
    fn set_multi_shift_shape_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_multi_shift_shape(0.5).unwrap();
        assert_10bit_cc(&ctrl, 104, 0.5);
    }

    #[test]
    fn set_delay_time_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_delay_time(0.5).unwrap();
        assert_10bit_cc(&ctrl, 105, 0.5);
    }

    #[test]
    fn set_delay_depth_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_delay_depth(0.5).unwrap();
        assert_10bit_cc(&ctrl, 106, 0.5);
    }

    #[test]
    fn set_delay_dry_wet_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_delay_dry_wet(0.5).unwrap();
        assert_10bit_cc(&ctrl, 107, 0.5);
    }

    #[test]
    fn set_reverb_time_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_reverb_time(0.5).unwrap();
        assert_10bit_cc(&ctrl, 108, 0.5);
    }

    #[test]
    fn set_reverb_depth_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_reverb_depth(0.5).unwrap();
        assert_10bit_cc(&ctrl, 109, 0.5);
    }

    #[test]
    fn set_reverb_dry_wet_sends_10bit() {
        let mut ctrl = make_ctrl();
        ctrl.set_reverb_dry_wet(0.5).unwrap();
        assert_10bit_cc(&ctrl, 110, 0.5);
    }

    // -- Stepped CC methods --

    #[test]
    fn set_vco1_wave_saw() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco1_wave(VcoWave::Saw).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 50, VcoWave::Saw.to_tx_value()]);
    }

    #[test]
    fn set_vco1_wave_sqr() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco1_wave(VcoWave::Sqr).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 50, VcoWave::Sqr.to_tx_value()]);
    }

    #[test]
    fn set_vco2_wave_tri() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco2_wave(VcoWave::Tri).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 51, VcoWave::Tri.to_tx_value()]);
    }

    #[test]
    fn set_vco1_octave_four() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco1_octave(VcoOctave::Four).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 48, VcoOctave::Four.to_tx_value()]);
    }

    #[test]
    fn set_vco2_octave_two() {
        let mut ctrl = make_ctrl();
        ctrl.set_vco2_octave(VcoOctave::Two).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 49, VcoOctave::Two.to_tx_value()]);
    }

    #[test]
    fn set_eg_target_cutoff() {
        let mut ctrl = make_ctrl();
        ctrl.set_eg_target(EgTarget::Cutoff).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 23, EgTarget::Cutoff.to_tx_value()]);
    }

    #[test]
    fn set_multi_type_vpm() {
        let mut ctrl = make_ctrl();
        ctrl.set_multi_type(MultiType::Vpm).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 53, MultiType::Vpm.to_tx_value()]);
    }

    #[test]
    fn set_lfo_target_shape() {
        let mut ctrl = make_ctrl();
        ctrl.set_lfo_target(LfoTarget::Shape).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 56, LfoTarget::Shape.to_tx_value()]);
    }

    #[test]
    fn set_lfo_wave_saw() {
        let mut ctrl = make_ctrl();
        ctrl.set_lfo_wave(LfoWave::Saw).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 57, LfoWave::Saw.to_tx_value()]);
    }

    #[test]
    fn set_lfo_mode_bpm() {
        let mut ctrl = make_ctrl();
        ctrl.set_lfo_mode(LfoMode::Bpm).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 58, LfoMode::Bpm.to_tx_value()]);
    }

    #[test]
    fn set_sync_on() {
        let mut ctrl = make_ctrl();
        ctrl.set_sync(Sync::On).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 80, Sync::On.to_tx_value()]);
    }

    #[test]
    fn set_ring_off() {
        let mut ctrl = make_ctrl();
        ctrl.set_ring(Ring::Off).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 81, Ring::Off.to_tx_value()]);
    }

    #[test]
    fn set_cutoff_keytrack_half() {
        let mut ctrl = make_ctrl();
        ctrl.set_cutoff_keytrack(CutoffKeytrack::Half).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 83, CutoffKeytrack::Half.to_tx_value()]);
    }

    #[test]
    fn set_cutoff_drive_full() {
        let mut ctrl = make_ctrl();
        ctrl.set_cutoff_drive(CutoffDrive::Full).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 84, CutoffDrive::Full.to_tx_value()]);
    }

    #[test]
    fn set_mod_fx_type_flanger() {
        let mut ctrl = make_ctrl();
        ctrl.set_mod_fx_type(ModFxType::Flanger).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 88, ModFxType::Flanger.to_tx_value()]);
    }

    #[test]
    fn set_delay_sub_type_stereo() {
        let mut ctrl = make_ctrl();
        ctrl.set_delay_sub_type(DelaySubType::Stereo).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 89, DelaySubType::Stereo.to_tx_value()]);
    }

    #[test]
    fn set_reverb_sub_type_hall() {
        let mut ctrl = make_ctrl();
        ctrl.set_reverb_sub_type(ReverbSubType::Hall).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 90, ReverbSubType::Hall.to_tx_value()]);
    }

    // -- On/Off CC methods --

    #[test]
    fn set_mod_fx_on_true() {
        let mut ctrl = make_ctrl();
        ctrl.set_mod_fx_on(true).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 92, 127]);
    }

    #[test]
    fn set_mod_fx_on_false() {
        let mut ctrl = make_ctrl();
        ctrl.set_mod_fx_on(false).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 92, 0]);
    }

    #[test]
    fn set_delay_on_true() {
        let mut ctrl = make_ctrl();
        ctrl.set_delay_on(true).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 93, 127]);
    }

    #[test]
    fn set_delay_on_false() {
        let mut ctrl = make_ctrl();
        ctrl.set_delay_on(false).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 93, 0]);
    }

    #[test]
    fn set_reverb_on_true() {
        let mut ctrl = make_ctrl();
        ctrl.set_reverb_on(true).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 94, 127]);
    }

    #[test]
    fn set_reverb_on_false() {
        let mut ctrl = make_ctrl();
        ctrl.set_reverb_on(false).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 94, 0]);
    }

    // -- Simple continuous --

    #[test]
    fn set_modulation1() {
        let mut ctrl = make_ctrl();
        ctrl.set_modulation1(U7::new(64).unwrap()).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 1, 64]);
    }

    #[test]
    fn set_modulation2() {
        let mut ctrl = make_ctrl();
        ctrl.set_modulation2(U7::new(100).unwrap()).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 2, 100]);
    }

    #[test]
    fn set_portamento_time() {
        let mut ctrl = make_ctrl();
        ctrl.set_portamento_time(U7::new(50).unwrap()).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 5, 50]);
    }

    // -- Note / transport --

    #[test]
    fn play_note_sends_note_on() {
        let mut ctrl = make_ctrl();
        ctrl.play_note(U7::new(60).unwrap(), U7::new(100).unwrap())
            .unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0x90, 60, 100]);
    }

    #[test]
    fn stop_note_sends_note_off() {
        let mut ctrl = make_ctrl();
        ctrl.stop_note(U7::new(60).unwrap()).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0x80, 60, 0]);
    }

    #[test]
    fn play_note_respects_channel() {
        let mut ctrl = make_ctrl_ch(3);
        ctrl.play_note(U7::new(60).unwrap(), U7::new(100).unwrap())
            .unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0][0], 0x93);
    }

    #[test]
    fn pitch_bend_center() {
        let mut ctrl = make_ctrl();
        ctrl.pitch_bend(0.0).unwrap();
        let msgs = ctrl.output().messages();
        // Center: wire value 8192 = 0x2000 -> lsb=0x00, msb=0x40
        assert_eq!(msgs[0], vec![0xE0, 0x00, 0x40]);
    }

    #[test]
    fn pitch_bend_max_up() {
        let mut ctrl = make_ctrl();
        ctrl.pitch_bend(1.0).unwrap();
        let msgs = ctrl.output().messages();
        // Max up: wire value 16383 = 0x3FFF -> lsb=0x7F, msb=0x7F
        assert_eq!(msgs[0], vec![0xE0, 0x7F, 0x7F]);
    }

    #[test]
    fn pitch_bend_max_down() {
        let mut ctrl = make_ctrl();
        ctrl.pitch_bend(-1.0).unwrap();
        let msgs = ctrl.output().messages();
        // Max down: wire value 1 -> lsb=0x01, msb=0x00
        // -8191 + 8192 = 1
        assert_eq!(msgs[0], vec![0xE0, 0x01, 0x00]);
    }

    #[test]
    fn pitch_bend_out_of_range() {
        let mut ctrl = make_ctrl();
        assert!(ctrl.pitch_bend(1.1).is_err());
        assert!(ctrl.pitch_bend(-1.1).is_err());
    }

    #[test]
    fn program_change_sends_bank_and_program() {
        let mut ctrl = make_ctrl();
        let prog = ProgramNumber::new(250).unwrap(); // bank 2, slot 50
        ctrl.program_change(prog).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs.len(), 3);
        // CC0 (Bank MSB) = 2
        assert_eq!(msgs[0], vec![0xB0, 0, 2]);
        // CC32 (Bank LSB) = 50
        assert_eq!(msgs[1], vec![0xB0, 32, 50]);
        // Program Change = 50
        assert_eq!(msgs[2], vec![0xC0, 50]);
    }

    #[test]
    fn all_notes_off_sends_cc123() {
        let mut ctrl = make_ctrl();
        ctrl.all_notes_off().unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0], vec![0xB0, 123, 0]);
    }

    // -- NRPN methods --

    #[test]
    fn set_bend_range_plus_sends_nrpn() {
        let mut ctrl = make_ctrl();
        ctrl.set_bend_range_plus(7).unwrap();
        let msgs = ctrl.output().messages();
        // CC99=0, CC98=0x16, CC6=7
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], vec![0xB0, 99, 0]);
        assert_eq!(msgs[1], vec![0xB0, 98, 0x16]);
        assert_eq!(msgs[2], vec![0xB0, 6, 7]);
    }

    #[test]
    fn set_bend_range_plus_rejects_over_12() {
        let mut ctrl = make_ctrl();
        assert!(ctrl.set_bend_range_plus(13).is_err());
    }

    #[test]
    fn set_bend_range_minus_sends_nrpn() {
        let mut ctrl = make_ctrl();
        ctrl.set_bend_range_minus(5).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], vec![0xB0, 99, 0]);
        assert_eq!(msgs[1], vec![0xB0, 98, 0x17]);
        assert_eq!(msgs[2], vec![0xB0, 6, 5]);
    }

    #[test]
    fn set_bend_range_minus_rejects_over_12() {
        let mut ctrl = make_ctrl();
        assert!(ctrl.set_bend_range_minus(13).is_err());
    }

    #[test]
    fn set_micro_tuning_sends_nrpn() {
        let mut ctrl = make_ctrl();
        ctrl.set_micro_tuning(MicroTuning::PureMajor).unwrap();
        let msgs = ctrl.output().messages();
        // CC99=0, CC98=0x30, CC6=program_value
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], vec![0xB0, 99, 0]);
        assert_eq!(msgs[1], vec![0xB0, 98, 0x30]);
        assert_eq!(
            msgs[2],
            vec![0xB0, 6, MicroTuning::PureMajor.to_program_value()]
        );
    }

    #[test]
    fn set_program_level_zero_db() {
        let mut ctrl = make_ctrl();
        ctrl.set_program_level(0.0).unwrap();
        let msgs = ctrl.output().messages();
        // 0 dB -> raw = (0 + 18) * 5 = 90
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], vec![0xB0, 99, 0]);
        assert_eq!(msgs[1], vec![0xB0, 98, 0x3F]);
        assert_eq!(msgs[2], vec![0xB0, 6, 90]);
    }

    #[test]
    fn set_program_level_min() {
        let mut ctrl = make_ctrl();
        ctrl.set_program_level(-18.0).unwrap();
        let msgs = ctrl.output().messages();
        // -18 dB -> raw = 0
        assert_eq!(msgs[2], vec![0xB0, 6, 0]);
    }

    #[test]
    fn set_program_level_max() {
        let mut ctrl = make_ctrl();
        ctrl.set_program_level(6.0).unwrap();
        let msgs = ctrl.output().messages();
        // +6 dB -> raw = 120
        assert_eq!(msgs[2], vec![0xB0, 6, 120]);
    }

    #[test]
    fn set_program_level_out_of_range() {
        let mut ctrl = make_ctrl();
        assert!(ctrl.set_program_level(-19.0).is_err());
        assert!(ctrl.set_program_level(7.0).is_err());
    }

    // -- f32 conversion edge cases --

    #[test]
    fn f32_to_10bit_boundaries() {
        let zero = f32_to_10bit(0.0).unwrap();
        assert_eq!(zero.value(), 0);

        let half = f32_to_10bit(0.5).unwrap();
        assert_eq!(half.value(), 512);

        let full = f32_to_10bit(1.0).unwrap();
        assert_eq!(full.value(), 1023);
    }

    #[test]
    fn f32_to_10bit_rejects_negative() {
        assert!(f32_to_10bit(-0.001).is_err());
    }

    #[test]
    fn f32_to_10bit_rejects_over_one() {
        assert!(f32_to_10bit(1.001).is_err());
    }

    #[test]
    fn f32_to_pitch_bend_boundaries() {
        let down = f32_to_pitch_bend(-1.0).unwrap();
        assert_eq!(down.value(), -8191);

        let center = f32_to_pitch_bend(0.0).unwrap();
        assert_eq!(center.value(), 0);

        let up = f32_to_pitch_bend(1.0).unwrap();
        assert_eq!(up.value(), 8191);
    }

    #[test]
    fn f32_to_pitch_bend_rejects_out_of_range() {
        assert!(f32_to_pitch_bend(-1.1).is_err());
        assert!(f32_to_pitch_bend(1.1).is_err());
    }

    // -- Channel respects non-zero channels for CC --

    #[test]
    fn cc_respects_channel() {
        let mut ctrl = make_ctrl_ch(5);
        ctrl.set_mod_fx_on(true).unwrap();
        let msgs = ctrl.output().messages();
        assert_eq!(msgs[0][0], 0xB5);
    }
}
