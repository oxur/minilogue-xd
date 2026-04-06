//! Fluent builder for synth patches (program data).
//!
//! [`PatchBuilder`] starts from [`SynthParams::default()`] and provides
//! ergonomic methods to set parameter groups. Float parameters are clamped
//! to 0.0..=1.0 and mapped to 10-bit (0--1023) values for a forgiving
//! builder experience.

use crate::error::Result;
use crate::param::enums::{
    CutoffDrive, CutoffKeytrack, DelaySubType, EgTarget, LfoMode, LfoTarget, LfoWave, ModFxType,
    MultiType, ReverbSubType, Ring, Sync, VcoOctave, VcoWave,
};
use crate::param::SteppedParam;
use crate::sysex::program::{ProgramData, ProgramName, SequencerParams, SynthParams};

/// Clamp a float to 0.0..=1.0 and scale to a 10-bit value (0--1023).
fn clamp_to_10bit(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1023.0).round() as u16
}

/// A consuming builder for [`ProgramData`].
///
/// All methods return `self` (or `Result<Self>` where validation is needed)
/// for chaining. Float parameters are clamped rather than rejected for a
/// more ergonomic builder experience.
///
/// # Examples
///
/// ```
/// use minilogue_xd::builder::PatchBuilder;
/// use minilogue_xd::param::enums::*;
///
/// let patch = PatchBuilder::new()
///     .name("MyPatch").unwrap()
///     .vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.3)
///     .filter(0.7, 0.2, CutoffDrive::Off, CutoffKeytrack::Off)
///     .build();
/// ```
pub struct PatchBuilder {
    synth: SynthParams,
}

impl PatchBuilder {
    /// Creates a new builder starting from the default synth parameters.
    pub fn new() -> Self {
        Self {
            synth: SynthParams::default(),
        }
    }

    /// Sets the program name (up to 12 characters).
    ///
    /// # Errors
    ///
    /// Returns an error if the name contains invalid characters.
    pub fn name(mut self, name: &str) -> Result<Self> {
        self.synth.name = ProgramName::from_string(name)?;
        Ok(self)
    }

    /// Sets VCO1 parameters: waveform, octave, pitch (0.0--1.0), shape (0.0--1.0).
    pub fn vco1(mut self, wave: VcoWave, octave: VcoOctave, pitch: f32, shape: f32) -> Self {
        self.synth.vco1_wave = wave;
        self.synth.vco1_octave = octave;
        self.synth.vco1_pitch = clamp_to_10bit(pitch);
        self.synth.vco1_shape = clamp_to_10bit(shape);
        self
    }

    /// Sets VCO2 parameters: waveform, octave, pitch (0.0--1.0), shape (0.0--1.0).
    pub fn vco2(mut self, wave: VcoWave, octave: VcoOctave, pitch: f32, shape: f32) -> Self {
        self.synth.vco2_wave = wave;
        self.synth.vco2_octave = octave;
        self.synth.vco2_pitch = clamp_to_10bit(pitch);
        self.synth.vco2_shape = clamp_to_10bit(shape);
        self
    }

    /// Sets VCO1 level (0.0--1.0).
    pub fn vco1_level(mut self, level: f32) -> Self {
        self.synth.vco1_level = clamp_to_10bit(level);
        self
    }

    /// Sets VCO2 level (0.0--1.0).
    pub fn vco2_level(mut self, level: f32) -> Self {
        self.synth.vco2_level = clamp_to_10bit(level);
        self
    }

    /// Sets oscillator sync and ring modulation.
    pub fn sync_ring(mut self, sync: Sync, ring: Ring) -> Self {
        self.synth.sync = sync == Sync::On;
        self.synth.ring = ring == Ring::On;
        self
    }

    /// Sets cross-modulation depth (0.0--1.0).
    pub fn cross_mod_depth(mut self, depth: f32) -> Self {
        self.synth.cross_mod_depth = clamp_to_10bit(depth);
        self
    }

    /// Sets multi-engine type and level (0.0--1.0).
    pub fn multi(mut self, multi_type: MultiType, level: f32) -> Self {
        self.synth.multi_type = multi_type;
        self.synth.multi_level = clamp_to_10bit(level);
        self
    }

    /// Sets filter parameters: cutoff (0.0--1.0), resonance (0.0--1.0),
    /// drive, and key-tracking.
    pub fn filter(
        mut self,
        cutoff: f32,
        resonance: f32,
        drive: CutoffDrive,
        keytrack: CutoffKeytrack,
    ) -> Self {
        self.synth.cutoff = clamp_to_10bit(cutoff);
        self.synth.resonance = clamp_to_10bit(resonance);
        self.synth.cutoff_drive = drive;
        self.synth.cutoff_keytrack = keytrack;
        self
    }

    /// Sets Amp EG parameters: attack, decay, sustain, release (all 0.0--1.0).
    pub fn amp_eg(mut self, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        self.synth.amp_eg_attack = clamp_to_10bit(attack);
        self.synth.amp_eg_decay = clamp_to_10bit(decay);
        self.synth.amp_eg_sustain = clamp_to_10bit(sustain);
        self.synth.amp_eg_release = clamp_to_10bit(release);
        self
    }

    /// Sets EG parameters: attack, decay, intensity (all 0.0--1.0), and target.
    pub fn eg(mut self, attack: f32, decay: f32, int: f32, target: EgTarget) -> Self {
        self.synth.eg_attack = clamp_to_10bit(attack);
        self.synth.eg_decay = clamp_to_10bit(decay);
        self.synth.eg_int = clamp_to_10bit(int);
        self.synth.eg_target = target;
        self
    }

    /// Sets LFO parameters: waveform, mode, rate (0.0--1.0), intensity (0.0--1.0),
    /// and target.
    pub fn lfo(
        mut self,
        wave: LfoWave,
        mode: LfoMode,
        rate: f32,
        int: f32,
        target: LfoTarget,
    ) -> Self {
        self.synth.lfo_wave = wave;
        self.synth.lfo_mode = mode;
        self.synth.lfo_rate = clamp_to_10bit(rate);
        self.synth.lfo_int = clamp_to_10bit(int);
        self.synth.lfo_target = target;
        self
    }

    /// Sets mod-FX on/off and type.
    pub fn mod_fx(mut self, on: bool, fx_type: ModFxType) -> Self {
        self.synth.mod_fx_on = on;
        // Store as 1-based raw value matching the blob format.
        self.synth.mod_fx_type = fx_type.to_program_value() + 1;
        self
    }

    /// Sets mod-FX time and depth (both 0.0--1.0).
    pub fn mod_fx_params(mut self, time: f32, depth: f32) -> Self {
        self.synth.mod_fx_time = clamp_to_10bit(time);
        self.synth.mod_fx_depth = clamp_to_10bit(depth);
        self
    }

    /// Sets delay parameters: on/off, sub-type, time (0.0--1.0),
    /// depth (0.0--1.0), dry/wet (0.0--1.0).
    pub fn delay(
        mut self,
        on: bool,
        sub_type: DelaySubType,
        time: f32,
        depth: f32,
        dry_wet: f32,
    ) -> Self {
        self.synth.delay_on = on;
        self.synth.delay_sub_type = sub_type;
        self.synth.delay_time = clamp_to_10bit(time);
        self.synth.delay_depth = clamp_to_10bit(depth);
        self.synth.delay_dry_wet = clamp_to_10bit(dry_wet);
        self
    }

    /// Sets reverb parameters: on/off, sub-type, time (0.0--1.0),
    /// depth (0.0--1.0), dry/wet (0.0--1.0).
    pub fn reverb(
        mut self,
        on: bool,
        sub_type: ReverbSubType,
        time: f32,
        depth: f32,
        dry_wet: f32,
    ) -> Self {
        self.synth.reverb_on = on;
        self.synth.reverb_sub_type = sub_type;
        self.synth.reverb_time = clamp_to_10bit(time);
        self.synth.reverb_depth = clamp_to_10bit(depth);
        self.synth.reverb_dry_wet = clamp_to_10bit(dry_wet);
        self
    }

    /// Sets portamento time (0--127).
    pub fn portamento(mut self, time: u8) -> Self {
        self.synth.portamento = time.min(127);
        self
    }

    /// Builds the final [`ProgramData`] with default sequencer parameters.
    pub fn build(self) -> ProgramData {
        ProgramData {
            synth: self.synth,
            sequencer: SequencerParams::default(),
        }
    }

    /// Builds the final [`ProgramData`] with a custom sequencer.
    pub fn build_with_sequencer(self, sequencer: SequencerParams) -> ProgramData {
        ProgramData {
            synth: self.synth,
            sequencer,
        }
    }
}

impl Default for PatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_build_produces_valid_program_data() {
        let data = PatchBuilder::new().build();
        assert_eq!(data.synth.name.as_str(), "");
        assert_eq!(data.synth.vco1_wave, VcoWave::Saw);
        assert_eq!(data.sequencer.bpm, 1200);
    }

    #[test]
    fn name_sets_program_name() {
        let data = PatchBuilder::new().name("TestPatch").unwrap().build();
        assert_eq!(data.synth.name.as_str(), "TestPatch");
    }

    #[test]
    fn name_invalid_char_returns_error() {
        let result = PatchBuilder::new().name("Bad@Name");
        assert!(result.is_err());
    }

    #[test]
    fn vco1_sets_all_fields() {
        let data = PatchBuilder::new()
            .vco1(VcoWave::Tri, VcoOctave::Four, 0.5, 0.75)
            .build();
        assert_eq!(data.synth.vco1_wave, VcoWave::Tri);
        assert_eq!(data.synth.vco1_octave, VcoOctave::Four);
        assert_eq!(data.synth.vco1_pitch, 512);
        assert_eq!(data.synth.vco1_shape, 767);
    }

    #[test]
    fn vco2_sets_all_fields() {
        let data = PatchBuilder::new()
            .vco2(VcoWave::Sqr, VcoOctave::Two, 1.0, 0.0)
            .build();
        assert_eq!(data.synth.vco2_wave, VcoWave::Sqr);
        assert_eq!(data.synth.vco2_octave, VcoOctave::Two);
        assert_eq!(data.synth.vco2_pitch, 1023);
        assert_eq!(data.synth.vco2_shape, 0);
    }

    #[test]
    fn filter_sets_all_fields() {
        let data = PatchBuilder::new()
            .filter(0.8, 0.3, CutoffDrive::Half, CutoffKeytrack::Full)
            .build();
        assert_eq!(data.synth.cutoff, 818);
        assert_eq!(data.synth.resonance, 307);
        assert_eq!(data.synth.cutoff_drive, CutoffDrive::Half);
        assert_eq!(data.synth.cutoff_keytrack, CutoffKeytrack::Full);
    }

    #[test]
    fn amp_eg_sets_all_fields() {
        let data = PatchBuilder::new().amp_eg(0.1, 0.2, 0.8, 0.4).build();
        assert_eq!(data.synth.amp_eg_attack, 102);
        assert_eq!(data.synth.amp_eg_decay, 205);
        assert_eq!(data.synth.amp_eg_sustain, 818);
        assert_eq!(data.synth.amp_eg_release, 409);
    }

    #[test]
    fn eg_sets_all_fields() {
        let data = PatchBuilder::new()
            .eg(0.3, 0.6, 0.5, EgTarget::Pitch)
            .build();
        assert_eq!(data.synth.eg_attack, 307);
        assert_eq!(data.synth.eg_decay, 614);
        assert_eq!(data.synth.eg_int, 512);
        assert_eq!(data.synth.eg_target, EgTarget::Pitch);
    }

    #[test]
    fn lfo_sets_all_fields() {
        let data = PatchBuilder::new()
            .lfo(LfoWave::Saw, LfoMode::Bpm, 0.5, 0.7, LfoTarget::Pitch)
            .build();
        assert_eq!(data.synth.lfo_wave, LfoWave::Saw);
        assert_eq!(data.synth.lfo_mode, LfoMode::Bpm);
        assert_eq!(data.synth.lfo_rate, 512);
        assert_eq!(data.synth.lfo_int, 716);
        assert_eq!(data.synth.lfo_target, LfoTarget::Pitch);
    }

    #[test]
    fn delay_sets_all_fields() {
        let data = PatchBuilder::new()
            .delay(true, DelaySubType::Stereo, 0.5, 0.6, 0.4)
            .build();
        assert!(data.synth.delay_on);
        assert_eq!(data.synth.delay_sub_type, DelaySubType::Stereo);
        assert_eq!(data.synth.delay_time, 512);
        assert_eq!(data.synth.delay_depth, 614);
        assert_eq!(data.synth.delay_dry_wet, 409);
    }

    #[test]
    fn reverb_sets_all_fields() {
        let data = PatchBuilder::new()
            .reverb(true, ReverbSubType::Hall, 0.7, 0.3, 0.5)
            .build();
        assert!(data.synth.reverb_on);
        assert_eq!(data.synth.reverb_sub_type, ReverbSubType::Hall);
        assert_eq!(data.synth.reverb_time, 716);
        assert_eq!(data.synth.reverb_depth, 307);
        assert_eq!(data.synth.reverb_dry_wet, 512);
    }

    #[test]
    fn float_clamping_high() {
        let data = PatchBuilder::new()
            .vco1(VcoWave::Saw, VcoOctave::Eight, 1.5, 2.0)
            .build();
        assert_eq!(data.synth.vco1_pitch, 1023);
        assert_eq!(data.synth.vco1_shape, 1023);
    }

    #[test]
    fn float_clamping_low() {
        let data = PatchBuilder::new()
            .vco1(VcoWave::Saw, VcoOctave::Eight, -0.5, -1.0)
            .build();
        assert_eq!(data.synth.vco1_pitch, 0);
        assert_eq!(data.synth.vco1_shape, 0);
    }

    #[test]
    fn chained_builder() {
        let data = PatchBuilder::new()
            .name("Chain")
            .unwrap()
            .vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.0)
            .vco2(VcoWave::Sqr, VcoOctave::Four, 0.5, 0.5)
            .filter(0.7, 0.2, CutoffDrive::Off, CutoffKeytrack::Half)
            .amp_eg(0.0, 0.5, 1.0, 0.3)
            .lfo(LfoWave::Tri, LfoMode::Normal, 0.5, 0.0, LfoTarget::Cutoff)
            .delay(true, DelaySubType::Stereo, 0.5, 0.5, 0.5)
            .build();

        assert_eq!(data.synth.name.as_str(), "Chain");
        assert_eq!(data.synth.vco1_wave, VcoWave::Saw);
        assert_eq!(data.synth.vco2_wave, VcoWave::Sqr);
        assert_eq!(data.synth.cutoff, 716);
        assert_eq!(data.synth.amp_eg_attack, 0);
        assert!(data.synth.delay_on);
    }

    #[test]
    fn sync_ring_sets_booleans() {
        let data = PatchBuilder::new().sync_ring(Sync::On, Ring::On).build();
        assert!(data.synth.sync);
        assert!(data.synth.ring);
    }

    #[test]
    fn sync_ring_off() {
        let data = PatchBuilder::new().sync_ring(Sync::Off, Ring::Off).build();
        assert!(!data.synth.sync);
        assert!(!data.synth.ring);
    }

    #[test]
    fn multi_sets_type_and_level() {
        let data = PatchBuilder::new().multi(MultiType::Vpm, 0.5).build();
        assert_eq!(data.synth.multi_type, MultiType::Vpm);
        assert_eq!(data.synth.multi_level, 512);
    }

    #[test]
    fn mod_fx_sets_on_and_type() {
        let data = PatchBuilder::new().mod_fx(true, ModFxType::Phaser).build();
        assert!(data.synth.mod_fx_on);
        // Phaser program value is 2, blob stores 1-based -> 3
        assert_eq!(data.synth.mod_fx_type, 3);
    }

    #[test]
    fn mod_fx_params_sets_time_and_depth() {
        let data = PatchBuilder::new().mod_fx_params(0.3, 0.7).build();
        assert_eq!(data.synth.mod_fx_time, 307);
        assert_eq!(data.synth.mod_fx_depth, 716);
    }

    #[test]
    fn vco1_level_sets_value() {
        let data = PatchBuilder::new().vco1_level(0.5).build();
        assert_eq!(data.synth.vco1_level, 512);
    }

    #[test]
    fn vco2_level_sets_value() {
        let data = PatchBuilder::new().vco2_level(0.75).build();
        assert_eq!(data.synth.vco2_level, 767);
    }

    #[test]
    fn cross_mod_depth_sets_value() {
        let data = PatchBuilder::new().cross_mod_depth(0.5).build();
        assert_eq!(data.synth.cross_mod_depth, 512);
    }

    #[test]
    fn portamento_sets_value() {
        let data = PatchBuilder::new().portamento(64).build();
        assert_eq!(data.synth.portamento, 64);
    }

    #[test]
    fn portamento_clamps_to_127() {
        let data = PatchBuilder::new().portamento(200).build();
        assert_eq!(data.synth.portamento, 127);
    }

    #[test]
    fn build_with_sequencer_uses_custom_seq() {
        let seq = SequencerParams {
            bpm: 1400,
            ..SequencerParams::default()
        };
        let data = PatchBuilder::new().build_with_sequencer(seq);
        assert_eq!(data.sequencer.bpm, 1400);
    }

    #[test]
    fn default_impl_matches_new() {
        let a = PatchBuilder::new().build();
        let b = PatchBuilder::default().build();
        assert_eq!(a.synth.vco1_wave, b.synth.vco1_wave);
        assert_eq!(a.synth.cutoff, b.synth.cutoff);
    }
}
