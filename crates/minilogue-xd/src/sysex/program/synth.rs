//! Synth parameters from TABLE 2 (offsets 0--155).
//!
//! This module provides [`SynthParams`] (the first 156 bytes of a 1024-byte
//! program blob) and the [`ProgramName`] newtype for 12-character names
//! restricted to the charset defined in note P1 of the spec.

use std::fmt;

use crate::error::{Error, Result, SysexError};
use crate::param::enums::{
    CutoffDrive, CutoffKeytrack, CvInMode, DelaySubType, EgTarget, LfoMode, LfoTarget,
    LfoTargetOsc, LfoWave, MicroTuning, ModAssignTarget, ModFxType, MultiRouting, MultiSelectNoise,
    MultiSelectUser, MultiSelectVpm, MultiType, PortamentoMode, ReverbSubType, UserParamType,
    VcoOctave, VcoWave, VoiceModeType,
};
use crate::param::SteppedParam;
use crate::sysex::helpers::{read_10bit, write_10bit};

// ---------------------------------------------------------------------------
// ProgramName
// ---------------------------------------------------------------------------

/// Valid bytes for a program name (note P1 of the spec).
const VALID_NAME_BYTES: &[u8] =
    b" !#$%&'()*,-./0123456789:?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Returns `true` if `b` is a valid program-name byte per note P1.
fn is_valid_name_byte(b: u8) -> bool {
    VALID_NAME_BYTES.contains(&b)
}

/// A 12-character program name using the restricted charset from note P1.
///
/// Valid characters are: space, `!`, `#`-`'`, `(`-`*`, `,`-`/`, `0`-`9`,
/// `:`, `?`, `A`-`Z`, `a`-`z`. The name is always exactly 12 bytes, padded
/// on the right with spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramName([u8; 12]);

impl ProgramName {
    /// Create a new `ProgramName` from a raw 12-byte array.
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::InvalidProgramNameChar`] if any byte is not in
    /// the valid charset.
    pub fn new(bytes: [u8; 12]) -> Result<Self> {
        for &b in &bytes {
            if !is_valid_name_byte(b) {
                return Err(SysexError::InvalidProgramNameChar(b).into());
            }
        }
        Ok(Self(bytes))
    }

    /// Create a `ProgramName` from a string slice.
    ///
    /// The string is truncated to 12 bytes and padded with spaces on the
    /// right if shorter. Only ASCII bytes are considered; the string must
    /// contain only valid name characters.
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::InvalidProgramNameChar`] if any byte is not in
    /// the valid charset.
    pub fn from_string(s: &str) -> Result<Self> {
        let mut bytes = [b' '; 12];
        let len = s.len().min(12);
        bytes[..len].copy_from_slice(&s.as_bytes()[..len]);
        Self::new(bytes)
    }

    /// Returns a reference to the raw 12-byte array.
    pub fn as_bytes(&self) -> &[u8; 12] {
        &self.0
    }

    /// Returns the name as a string with trailing spaces trimmed.
    pub fn as_str(&self) -> &str {
        // The bytes are all valid ASCII, so this is safe.
        let s = std::str::from_utf8(&self.0).unwrap_or("");
        s.trim_end()
    }
}

impl std::str::FromStr for ProgramName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s)
    }
}

impl fmt::Display for ProgramName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for ProgramName {
    fn default() -> Self {
        Self([b' '; 12])
    }
}

// ---------------------------------------------------------------------------
// SynthParams
// ---------------------------------------------------------------------------

/// Synth parameters from the first 156 bytes of a program blob (TABLE 2).
///
/// Fields are stored with their natural Rust representations. Boolean fields
/// like `sync` and `ring` are stored as logical values (`true` = ON), even
/// though the blob uses inverted encoding (0 = ON, 1 = OFF) for those fields.
///
/// 10-bit continuous parameters are stored as raw `u16` values (0--1023).
/// Enum fields that map cleanly to the param enums are stored as their enum
/// type. Fields with ambiguous or firmware-specific ranges are stored as raw
/// `u8`.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthParams {
    /// Program name (12 characters, note P1 charset).
    pub name: ProgramName,
    /// Octave shift: 0=−2, 1=−1, 2=0, 3=+1, 4=+2.
    pub octave: u8,
    /// Portamento time (0--127).
    pub portamento: u8,
    /// Key trigger on/off.
    pub key_trig: bool,
    /// Voice mode depth (10-bit, 0--1023).
    pub voice_mode_depth: u16,
    /// Voice mode type, raw 1-based value (1--4 in blob; see note P3).
    pub voice_mode_type: u8,
    /// VCO 1 waveform.
    pub vco1_wave: VcoWave,
    /// VCO 1 octave.
    pub vco1_octave: VcoOctave,
    /// VCO 1 pitch (10-bit, 0--1023).
    pub vco1_pitch: u16,
    /// VCO 1 shape (10-bit, 0--1023).
    pub vco1_shape: u16,
    /// VCO 2 waveform.
    pub vco2_wave: VcoWave,
    /// VCO 2 octave.
    pub vco2_octave: VcoOctave,
    /// VCO 2 pitch (10-bit, 0--1023).
    pub vco2_pitch: u16,
    /// VCO 2 shape (10-bit, 0--1023).
    pub vco2_shape: u16,
    /// Oscillator sync on/off (true = ON).
    ///
    /// **Note:** the blob encodes this inverted: 0 = ON, 1 = OFF.
    pub sync: bool,
    /// Ring modulation on/off (true = ON).
    ///
    /// **Note:** the blob encodes this inverted: 0 = ON, 1 = OFF.
    pub ring: bool,
    /// Cross-modulation depth (10-bit, 0--1023).
    pub cross_mod_depth: u16,
    /// Multi-engine oscillator type.
    pub multi_type: MultiType,
    /// Multi-engine noise sub-type selector.
    pub select_noise: MultiSelectNoise,
    /// Multi-engine VPM sub-type selector (0--15).
    pub select_vpm: u8,
    /// Multi-engine user sub-type selector (0--15).
    pub select_user: u8,
    /// Multi-engine noise shape (10-bit).
    pub shape_noise: u16,
    /// Multi-engine VPM shape (10-bit).
    pub shape_vpm: u16,
    /// Multi-engine user shape (10-bit).
    pub shape_user: u16,
    /// Multi-engine noise shift-shape (10-bit).
    pub shift_shape_noise: u16,
    /// Multi-engine VPM shift-shape (10-bit).
    pub shift_shape_vpm: u16,
    /// Multi-engine user shift-shape (10-bit).
    pub shift_shape_user: u16,
    /// VCO 1 level (10-bit).
    pub vco1_level: u16,
    /// VCO 2 level (10-bit).
    pub vco2_level: u16,
    /// Multi-engine level (10-bit).
    pub multi_level: u16,
    /// Filter cutoff (10-bit).
    pub cutoff: u16,
    /// Filter resonance (10-bit).
    pub resonance: u16,
    /// Cutoff drive amount.
    pub cutoff_drive: CutoffDrive,
    /// Cutoff key-tracking amount.
    pub cutoff_keytrack: CutoffKeytrack,
    /// Amp EG attack (10-bit).
    pub amp_eg_attack: u16,
    /// Amp EG decay (10-bit).
    pub amp_eg_decay: u16,
    /// Amp EG sustain (10-bit).
    pub amp_eg_sustain: u16,
    /// Amp EG release (10-bit).
    pub amp_eg_release: u16,
    /// EG attack (10-bit).
    pub eg_attack: u16,
    /// EG decay (10-bit).
    pub eg_decay: u16,
    /// EG intensity (10-bit, quadratic scaling per note P10).
    pub eg_int: u16,
    /// EG target.
    pub eg_target: EgTarget,
    /// LFO waveform.
    pub lfo_wave: LfoWave,
    /// LFO mode.
    pub lfo_mode: LfoMode,
    /// LFO rate (10-bit, BPM-synced per note P11).
    pub lfo_rate: u16,
    /// LFO intensity (10-bit).
    pub lfo_int: u16,
    /// LFO target.
    pub lfo_target: LfoTarget,
    /// Mod-FX on/off.
    pub mod_fx_on: bool,
    /// Mod-FX type, raw 1-based value (1--5 in blob; see note P12).
    pub mod_fx_type: u8,
    /// Mod-FX Chorus sub-type (0--7).
    pub mod_fx_chorus: u8,
    /// Mod-FX Ensemble sub-type (0--2).
    pub mod_fx_ensemble: u8,
    /// Mod-FX Phaser sub-type (0--7).
    pub mod_fx_phaser: u8,
    /// Mod-FX Flanger sub-type (0--7).
    pub mod_fx_flanger: u8,
    /// Mod-FX User sub-type (0--15).
    pub mod_fx_user: u8,
    /// Mod-FX time (10-bit).
    pub mod_fx_time: u16,
    /// Mod-FX depth (10-bit).
    pub mod_fx_depth: u16,
    /// Delay on/off.
    pub delay_on: bool,
    /// Delay sub-type.
    pub delay_sub_type: DelaySubType,
    /// Delay time (10-bit).
    pub delay_time: u16,
    /// Delay depth (10-bit).
    pub delay_depth: u16,
    /// Reverb on/off.
    pub reverb_on: bool,
    /// Reverb sub-type.
    pub reverb_sub_type: ReverbSubType,
    /// Reverb time (10-bit).
    pub reverb_time: u16,
    /// Reverb depth (10-bit).
    pub reverb_depth: u16,
    /// Pitch bend range positive (0--12 semitones).
    pub bend_range_plus: u8,
    /// Pitch bend range negative (0--12 semitones).
    pub bend_range_minus: u8,
    /// Joystick assign positive target.
    pub joystick_assign_plus: ModAssignTarget,
    /// Joystick range positive (0--200).
    pub joystick_range_plus: u8,
    /// Joystick assign negative target.
    pub joystick_assign_minus: ModAssignTarget,
    /// Joystick range negative (0--200).
    pub joystick_range_minus: u8,
    /// CV input mode.
    pub cv_in_mode: CvInMode,
    /// CV input 1 assign target.
    pub cv_in1_assign: ModAssignTarget,
    /// CV input 1 range (0--200).
    pub cv_in1_range: u8,
    /// CV input 2 assign target.
    pub cv_in2_assign: ModAssignTarget,
    /// CV input 2 range (0--200).
    pub cv_in2_range: u8,
    /// Micro-tuning selection, raw byte (0--139, see note P21).
    pub micro_tuning: u8,
    /// Scale key (0--24, maps to −12..+12).
    pub scale_key: u8,
    /// Program tuning (0--100, maps to −50..+50 cents).
    pub program_tuning: u8,
    /// LFO key sync on/off.
    pub lfo_key_sync: bool,
    /// LFO voice sync on/off.
    pub lfo_voice_sync: bool,
    /// LFO target oscillator.
    pub lfo_target_osc: LfoTargetOsc,
    /// Cutoff velocity sensitivity (0--127).
    pub cutoff_velocity: u8,
    /// Amp velocity sensitivity (0--127).
    pub amp_velocity: u8,
    /// Multi-engine octave.
    pub multi_octave: VcoOctave,
    /// Multi-engine audio routing.
    pub multi_routing: MultiRouting,
    /// EG legato on/off.
    pub eg_legato: bool,
    /// Portamento mode.
    pub portamento_mode: PortamentoMode,
    /// Portamento BPM sync on/off.
    pub portamento_bpm_sync: bool,
    /// Program level (12--132, maps to −18dB..+6dB).
    pub program_level: u8,
    /// VPM parameter 1 (0--200).
    pub vpm_param1: u8,
    /// VPM parameter 2 (0--200).
    pub vpm_param2: u8,
    /// VPM parameter 3 (0--200).
    pub vpm_param3: u8,
    /// VPM parameter 4 (0--200).
    pub vpm_param4: u8,
    /// VPM parameter 5 (0--200).
    pub vpm_param5: u8,
    /// VPM parameter 6 (0--200).
    pub vpm_param6: u8,
    /// User parameter 1 (0--200, see note P23).
    pub user_param1: u8,
    /// User parameter 2 (0--200).
    pub user_param2: u8,
    /// User parameter 3 (0--200).
    pub user_param3: u8,
    /// User parameter 4 (0--200).
    pub user_param4: u8,
    /// User parameter 5 (0--200).
    pub user_param5: u8,
    /// User parameter 6 (0--200).
    pub user_param6: u8,
    /// User parameter 5 display type.
    pub user_param5_type: UserParamType,
    /// User parameter 6 display type.
    pub user_param6_type: UserParamType,
    /// User parameter 1 display type.
    pub user_param1_type: UserParamType,
    /// User parameter 2 display type.
    pub user_param2_type: UserParamType,
    /// User parameter 3 display type.
    pub user_param3_type: UserParamType,
    /// User parameter 4 display type.
    pub user_param4_type: UserParamType,
    /// Program transpose (1--25, maps to −12..+12).
    pub program_transpose: u8,
    /// Delay dry/wet (10-bit).
    pub delay_dry_wet: u16,
    /// Reverb dry/wet (10-bit).
    pub reverb_dry_wet: u16,
    /// MIDI aftertouch assign target.
    pub midi_after_touch_assign: ModAssignTarget,
}

impl SynthParams {
    /// Size of the synth parameter block in bytes.
    pub const SIZE: usize = 156;

    /// Expected magic bytes at the start of a program blob.
    pub const MAGIC: &[u8; 4] = b"PROG";

    /// Parse synth parameters from a byte slice (offsets 0--155).
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is too short, has wrong magic bytes,
    /// or contains invalid enum values.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::SIZE,
                actual: bytes.len(),
            }
            .into());
        }

        // Validate magic.
        if &bytes[0..4] != Self::MAGIC {
            return Err(SysexError::InvalidMagic {
                expected: "PROG".to_string(),
                actual: String::from_utf8_lossy(&bytes[0..4]).to_string(),
            }
            .into());
        }

        // Name (offsets 4--15).
        let mut name_bytes = [0u8; 12];
        name_bytes.copy_from_slice(&bytes[4..16]);
        let name = ProgramName::new(name_bytes)?;

        Ok(SynthParams {
            name,
            octave: bytes[16],
            portamento: bytes[17],
            key_trig: bytes[18] != 0,
            voice_mode_depth: read_10bit(bytes, 19),
            voice_mode_type: bytes[21],
            vco1_wave: VcoWave::from_program_value(bytes[22])?,
            vco1_octave: VcoOctave::from_program_value(bytes[23])?,
            vco1_pitch: read_10bit(bytes, 24),
            vco1_shape: read_10bit(bytes, 26),
            vco2_wave: VcoWave::from_program_value(bytes[28])?,
            vco2_octave: VcoOctave::from_program_value(bytes[29])?,
            vco2_pitch: read_10bit(bytes, 30),
            vco2_shape: read_10bit(bytes, 32),
            // Inverted: 0 = ON, 1 = OFF in blob.
            sync: bytes[34] == 0,
            ring: bytes[35] == 0,
            cross_mod_depth: read_10bit(bytes, 36),
            multi_type: MultiType::from_program_value(bytes[38])?,
            select_noise: MultiSelectNoise::from_program_value(bytes[39])?,
            select_vpm: bytes[40],
            select_user: bytes[41],
            shape_noise: read_10bit(bytes, 42),
            shape_vpm: read_10bit(bytes, 44),
            shape_user: read_10bit(bytes, 46),
            shift_shape_noise: read_10bit(bytes, 48),
            shift_shape_vpm: read_10bit(bytes, 50),
            shift_shape_user: read_10bit(bytes, 52),
            vco1_level: read_10bit(bytes, 54),
            vco2_level: read_10bit(bytes, 56),
            multi_level: read_10bit(bytes, 58),
            cutoff: read_10bit(bytes, 60),
            resonance: read_10bit(bytes, 62),
            cutoff_drive: CutoffDrive::from_program_value(bytes[64])?,
            cutoff_keytrack: CutoffKeytrack::from_program_value(bytes[65])?,
            amp_eg_attack: read_10bit(bytes, 66),
            amp_eg_decay: read_10bit(bytes, 68),
            amp_eg_sustain: read_10bit(bytes, 70),
            amp_eg_release: read_10bit(bytes, 72),
            eg_attack: read_10bit(bytes, 74),
            eg_decay: read_10bit(bytes, 76),
            eg_int: read_10bit(bytes, 78),
            eg_target: EgTarget::from_program_value(bytes[80])?,
            lfo_wave: LfoWave::from_program_value(bytes[81])?,
            lfo_mode: LfoMode::from_program_value(bytes[82])?,
            lfo_rate: read_10bit(bytes, 83),
            lfo_int: read_10bit(bytes, 85),
            lfo_target: LfoTarget::from_program_value(bytes[87])?,
            mod_fx_on: bytes[88] != 0,
            mod_fx_type: bytes[89],
            mod_fx_chorus: bytes[90],
            mod_fx_ensemble: bytes[91],
            mod_fx_phaser: bytes[92],
            mod_fx_flanger: bytes[93],
            mod_fx_user: bytes[94],
            mod_fx_time: read_10bit(bytes, 95),
            mod_fx_depth: read_10bit(bytes, 97),
            delay_on: bytes[99] != 0,
            delay_sub_type: DelaySubType::from_program_value(bytes[100])?,
            delay_time: read_10bit(bytes, 101),
            delay_depth: read_10bit(bytes, 103),
            reverb_on: bytes[105] != 0,
            reverb_sub_type: ReverbSubType::from_program_value(bytes[106])?,
            reverb_time: read_10bit(bytes, 107),
            reverb_depth: read_10bit(bytes, 109),
            bend_range_plus: bytes[111],
            bend_range_minus: bytes[112],
            joystick_assign_plus: ModAssignTarget::from_program_value(bytes[113])?,
            joystick_range_plus: bytes[114],
            joystick_assign_minus: ModAssignTarget::from_program_value(bytes[115])?,
            joystick_range_minus: bytes[116],
            cv_in_mode: CvInMode::from_program_value(bytes[117])?,
            cv_in1_assign: ModAssignTarget::from_program_value(bytes[118])?,
            cv_in1_range: bytes[119],
            cv_in2_assign: ModAssignTarget::from_program_value(bytes[120])?,
            cv_in2_range: bytes[121],
            micro_tuning: bytes[122],
            scale_key: bytes[123],
            program_tuning: bytes[124],
            lfo_key_sync: bytes[125] != 0,
            lfo_voice_sync: bytes[126] != 0,
            lfo_target_osc: LfoTargetOsc::from_program_value(bytes[127])?,
            cutoff_velocity: bytes[128],
            amp_velocity: bytes[129],
            multi_octave: VcoOctave::from_program_value(bytes[130])?,
            multi_routing: MultiRouting::from_program_value(bytes[131])?,
            eg_legato: bytes[132] != 0,
            portamento_mode: PortamentoMode::from_program_value(bytes[133])?,
            portamento_bpm_sync: bytes[134] != 0,
            program_level: bytes[135],
            vpm_param1: bytes[136],
            vpm_param2: bytes[137],
            vpm_param3: bytes[138],
            vpm_param4: bytes[139],
            vpm_param5: bytes[140],
            vpm_param6: bytes[141],
            user_param1: bytes[142],
            user_param2: bytes[143],
            user_param3: bytes[144],
            user_param4: bytes[145],
            user_param5: bytes[146],
            user_param6: bytes[147],
            // Byte 148: bits 0-1 = user_param5_type, bits 2-3 = user_param6_type.
            user_param5_type: UserParamType::from_program_value(bytes[148] & 0x03)?,
            user_param6_type: UserParamType::from_program_value((bytes[148] >> 2) & 0x03)?,
            // Byte 149: bits 0-1 = user_param1_type, bits 2-3 = user_param2_type,
            //           bits 4-5 = user_param3_type, bits 6-7 = user_param4_type.
            user_param1_type: UserParamType::from_program_value(bytes[149] & 0x03)?,
            user_param2_type: UserParamType::from_program_value((bytes[149] >> 2) & 0x03)?,
            user_param3_type: UserParamType::from_program_value((bytes[149] >> 4) & 0x03)?,
            user_param4_type: UserParamType::from_program_value((bytes[149] >> 6) & 0x03)?,
            program_transpose: bytes[150],
            delay_dry_wet: read_10bit(bytes, 151),
            reverb_dry_wet: read_10bit(bytes, 153),
            midi_after_touch_assign: ModAssignTarget::from_program_value(bytes[155])?,
        })
    }

    /// Serialize synth parameters to a 156-byte array.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];

        out[0..4].copy_from_slice(Self::MAGIC);
        out[4..16].copy_from_slice(self.name.as_bytes());
        out[16] = self.octave;
        out[17] = self.portamento;
        out[18] = u8::from(self.key_trig);
        write_10bit(&mut out, 19, self.voice_mode_depth);
        out[21] = self.voice_mode_type;
        out[22] = self.vco1_wave.to_program_value();
        out[23] = self.vco1_octave.to_program_value();
        write_10bit(&mut out, 24, self.vco1_pitch);
        write_10bit(&mut out, 26, self.vco1_shape);
        out[28] = self.vco2_wave.to_program_value();
        out[29] = self.vco2_octave.to_program_value();
        write_10bit(&mut out, 30, self.vco2_pitch);
        write_10bit(&mut out, 32, self.vco2_shape);
        // Inverted: true(ON) -> 0, false(OFF) -> 1.
        out[34] = u8::from(!self.sync);
        out[35] = u8::from(!self.ring);
        write_10bit(&mut out, 36, self.cross_mod_depth);
        out[38] = self.multi_type.to_program_value();
        out[39] = self.select_noise.to_program_value();
        out[40] = self.select_vpm;
        out[41] = self.select_user;
        write_10bit(&mut out, 42, self.shape_noise);
        write_10bit(&mut out, 44, self.shape_vpm);
        write_10bit(&mut out, 46, self.shape_user);
        write_10bit(&mut out, 48, self.shift_shape_noise);
        write_10bit(&mut out, 50, self.shift_shape_vpm);
        write_10bit(&mut out, 52, self.shift_shape_user);
        write_10bit(&mut out, 54, self.vco1_level);
        write_10bit(&mut out, 56, self.vco2_level);
        write_10bit(&mut out, 58, self.multi_level);
        write_10bit(&mut out, 60, self.cutoff);
        write_10bit(&mut out, 62, self.resonance);
        out[64] = self.cutoff_drive.to_program_value();
        out[65] = self.cutoff_keytrack.to_program_value();
        write_10bit(&mut out, 66, self.amp_eg_attack);
        write_10bit(&mut out, 68, self.amp_eg_decay);
        write_10bit(&mut out, 70, self.amp_eg_sustain);
        write_10bit(&mut out, 72, self.amp_eg_release);
        write_10bit(&mut out, 74, self.eg_attack);
        write_10bit(&mut out, 76, self.eg_decay);
        write_10bit(&mut out, 78, self.eg_int);
        out[80] = self.eg_target.to_program_value();
        out[81] = self.lfo_wave.to_program_value();
        out[82] = self.lfo_mode.to_program_value();
        write_10bit(&mut out, 83, self.lfo_rate);
        write_10bit(&mut out, 85, self.lfo_int);
        out[87] = self.lfo_target.to_program_value();
        out[88] = u8::from(self.mod_fx_on);
        out[89] = self.mod_fx_type;
        out[90] = self.mod_fx_chorus;
        out[91] = self.mod_fx_ensemble;
        out[92] = self.mod_fx_phaser;
        out[93] = self.mod_fx_flanger;
        out[94] = self.mod_fx_user;
        write_10bit(&mut out, 95, self.mod_fx_time);
        write_10bit(&mut out, 97, self.mod_fx_depth);
        out[99] = u8::from(self.delay_on);
        out[100] = self.delay_sub_type.to_program_value();
        write_10bit(&mut out, 101, self.delay_time);
        write_10bit(&mut out, 103, self.delay_depth);
        out[105] = u8::from(self.reverb_on);
        out[106] = self.reverb_sub_type.to_program_value();
        write_10bit(&mut out, 107, self.reverb_time);
        write_10bit(&mut out, 109, self.reverb_depth);
        out[111] = self.bend_range_plus;
        out[112] = self.bend_range_minus;
        out[113] = self.joystick_assign_plus.to_program_value();
        out[114] = self.joystick_range_plus;
        out[115] = self.joystick_assign_minus.to_program_value();
        out[116] = self.joystick_range_minus;
        out[117] = self.cv_in_mode.to_program_value();
        out[118] = self.cv_in1_assign.to_program_value();
        out[119] = self.cv_in1_range;
        out[120] = self.cv_in2_assign.to_program_value();
        out[121] = self.cv_in2_range;
        out[122] = self.micro_tuning;
        out[123] = self.scale_key;
        out[124] = self.program_tuning;
        out[125] = u8::from(self.lfo_key_sync);
        out[126] = u8::from(self.lfo_voice_sync);
        out[127] = self.lfo_target_osc.to_program_value();
        out[128] = self.cutoff_velocity;
        out[129] = self.amp_velocity;
        out[130] = self.multi_octave.to_program_value();
        out[131] = self.multi_routing.to_program_value();
        out[132] = u8::from(self.eg_legato);
        out[133] = self.portamento_mode.to_program_value();
        out[134] = u8::from(self.portamento_bpm_sync);
        out[135] = self.program_level;
        out[136] = self.vpm_param1;
        out[137] = self.vpm_param2;
        out[138] = self.vpm_param3;
        out[139] = self.vpm_param4;
        out[140] = self.vpm_param5;
        out[141] = self.vpm_param6;
        out[142] = self.user_param1;
        out[143] = self.user_param2;
        out[144] = self.user_param3;
        out[145] = self.user_param4;
        out[146] = self.user_param5;
        out[147] = self.user_param6;
        // Byte 148: bits 0-1 = user_param5_type, bits 2-3 = user_param6_type.
        out[148] = (self.user_param5_type.to_program_value() & 0x03)
            | ((self.user_param6_type.to_program_value() & 0x03) << 2);
        // Byte 149: bits 0-1 = param1, 2-3 = param2, 4-5 = param3, 6-7 = param4.
        out[149] = (self.user_param1_type.to_program_value() & 0x03)
            | ((self.user_param2_type.to_program_value() & 0x03) << 2)
            | ((self.user_param3_type.to_program_value() & 0x03) << 4)
            | ((self.user_param4_type.to_program_value() & 0x03) << 6);
        out[150] = self.program_transpose;
        write_10bit(&mut out, 151, self.delay_dry_wet);
        write_10bit(&mut out, 153, self.reverb_dry_wet);
        out[155] = self.midi_after_touch_assign.to_program_value();

        out
    }

    /// Attempt to interpret `voice_mode_type` as a [`VoiceModeType`].
    ///
    /// The blob stores this as a 1-based value (1=Arp, 2=Chord, 3=Poly, 4=ArpLatch).
    /// This returns the enum variant if the raw value can be converted.
    pub fn voice_mode_type_enum(&self) -> Result<VoiceModeType> {
        // Blob stores 1-based; our enum prog values are 0-based.
        if self.voice_mode_type == 0 {
            return Err(Error::OutOfRange {
                type_name: "VoiceModeType",
                value: 0,
                min: 1,
                max: 4,
            });
        }
        VoiceModeType::from_program_value(self.voice_mode_type.saturating_sub(1))
    }

    /// Attempt to interpret `mod_fx_type` as a [`ModFxType`].
    ///
    /// The blob stores this as a 1-based value (1=Chorus, 2=Ensemble, etc.).
    pub fn mod_fx_type_enum(&self) -> Result<ModFxType> {
        if self.mod_fx_type == 0 {
            return Err(Error::OutOfRange {
                type_name: "ModFxType",
                value: 0,
                min: 1,
                max: 5,
            });
        }
        ModFxType::from_program_value(self.mod_fx_type.saturating_sub(1))
    }

    /// Attempt to interpret `micro_tuning` as a [`MicroTuning`] enum.
    ///
    /// Values above 38 are firmware-specific and will return an error.
    pub fn micro_tuning_enum(&self) -> Result<MicroTuning> {
        MicroTuning::from_program_value(self.micro_tuning)
    }

    /// Attempt to interpret `select_vpm` as a [`MultiSelectVpm`] enum.
    pub fn select_vpm_enum(&self) -> Result<MultiSelectVpm> {
        MultiSelectVpm::from_program_value(self.select_vpm)
    }

    /// Attempt to interpret `select_user` as a [`MultiSelectUser`] enum.
    pub fn select_user_enum(&self) -> Result<MultiSelectUser> {
        MultiSelectUser::from_program_value(self.select_user)
    }
}

/// Create a default `SynthParams` representing an initialized program.
impl Default for SynthParams {
    fn default() -> Self {
        Self {
            name: ProgramName::default(),
            octave: 2, // center (0)
            portamento: 0,
            key_trig: false,
            voice_mode_depth: 0,
            voice_mode_type: 3, // Poly (1-based)
            vco1_wave: VcoWave::Saw,
            vco1_octave: VcoOctave::Eight,
            vco1_pitch: 512,
            vco1_shape: 0,
            vco2_wave: VcoWave::Saw,
            vco2_octave: VcoOctave::Eight,
            vco2_pitch: 512,
            vco2_shape: 0,
            sync: false,
            ring: false,
            cross_mod_depth: 0,
            multi_type: MultiType::Noise,
            select_noise: MultiSelectNoise::High,
            select_vpm: 0,
            select_user: 0,
            shape_noise: 0,
            shape_vpm: 0,
            shape_user: 0,
            shift_shape_noise: 0,
            shift_shape_vpm: 0,
            shift_shape_user: 0,
            vco1_level: 1023,
            vco2_level: 0,
            multi_level: 0,
            cutoff: 1023,
            resonance: 0,
            cutoff_drive: CutoffDrive::Off,
            cutoff_keytrack: CutoffKeytrack::Off,
            amp_eg_attack: 0,
            amp_eg_decay: 1023,
            amp_eg_sustain: 1023,
            amp_eg_release: 0,
            eg_attack: 0,
            eg_decay: 1023,
            eg_int: 512,
            eg_target: EgTarget::Cutoff,
            lfo_wave: LfoWave::Tri,
            lfo_mode: LfoMode::Normal,
            lfo_rate: 512,
            lfo_int: 0,
            lfo_target: LfoTarget::Cutoff,
            mod_fx_on: false,
            mod_fx_type: 1,
            mod_fx_chorus: 0,
            mod_fx_ensemble: 0,
            mod_fx_phaser: 0,
            mod_fx_flanger: 0,
            mod_fx_user: 0,
            mod_fx_time: 0,
            mod_fx_depth: 0,
            delay_on: false,
            delay_sub_type: DelaySubType::Stereo,
            delay_time: 0,
            delay_depth: 0,
            reverb_on: false,
            reverb_sub_type: ReverbSubType::Hall,
            reverb_time: 0,
            reverb_depth: 0,
            bend_range_plus: 2,
            bend_range_minus: 2,
            joystick_assign_plus: ModAssignTarget::None,
            joystick_range_plus: 100,
            joystick_assign_minus: ModAssignTarget::None,
            joystick_range_minus: 100,
            cv_in_mode: CvInMode::Modulation,
            cv_in1_assign: ModAssignTarget::None,
            cv_in1_range: 100,
            cv_in2_assign: ModAssignTarget::None,
            cv_in2_range: 100,
            micro_tuning: 0,
            scale_key: 12,      // center (0)
            program_tuning: 50, // center (0 cents)
            lfo_key_sync: false,
            lfo_voice_sync: false,
            lfo_target_osc: LfoTargetOsc::All,
            cutoff_velocity: 0,
            amp_velocity: 0,
            multi_octave: VcoOctave::Eight,
            multi_routing: MultiRouting::PreVcf,
            eg_legato: false,
            portamento_mode: PortamentoMode::Auto,
            portamento_bpm_sync: false,
            program_level: 102, // 0dB
            vpm_param1: 0,
            vpm_param2: 0,
            vpm_param3: 0,
            vpm_param4: 0,
            vpm_param5: 0,
            vpm_param6: 0,
            user_param1: 0,
            user_param2: 0,
            user_param3: 0,
            user_param4: 0,
            user_param5: 0,
            user_param6: 0,
            user_param5_type: UserParamType::Percent,
            user_param6_type: UserParamType::Percent,
            user_param1_type: UserParamType::Percent,
            user_param2_type: UserParamType::Percent,
            user_param3_type: UserParamType::Percent,
            user_param4_type: UserParamType::Percent,
            program_transpose: 13, // center (0)
            delay_dry_wet: 512,
            reverb_dry_wet: 512,
            midi_after_touch_assign: ModAssignTarget::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // ProgramName
    // ---------------------------------------------------------------

    #[test]
    fn name_valid_all_spaces() {
        let name = ProgramName::new([b' '; 12]).unwrap();
        assert_eq!(name.as_str(), "");
        assert_eq!(name.to_string(), "");
    }

    #[test]
    fn name_valid_alphanumeric() {
        let bytes = *b"Hello World!";
        let name = ProgramName::new(bytes).unwrap();
        assert_eq!(name.as_str(), "Hello World!");
        assert_eq!(name.as_bytes(), &bytes);
    }

    #[test]
    fn name_valid_special_chars() {
        let bytes = *b"#$%&'()*,-./";
        let name = ProgramName::new(bytes).unwrap();
        assert_eq!(name.as_str(), "#$%&'()*,-./");
    }

    #[test]
    fn name_valid_digits_colon_question() {
        let bytes = *b"0123456789:?";
        let name = ProgramName::new(bytes).unwrap();
        assert_eq!(name.as_str(), "0123456789:?");
    }

    #[test]
    fn name_invalid_at_sign() {
        let mut bytes = *b"Hello World ";
        bytes[5] = b'@';
        assert!(ProgramName::new(bytes).is_err());
    }

    #[test]
    fn name_invalid_semicolon() {
        let mut bytes = [b' '; 12];
        bytes[0] = b';';
        assert!(ProgramName::new(bytes).is_err());
    }

    #[test]
    fn name_invalid_backslash() {
        let mut bytes = [b' '; 12];
        bytes[0] = b'\\';
        assert!(ProgramName::new(bytes).is_err());
    }

    #[test]
    fn name_invalid_control_char() {
        let mut bytes = [b' '; 12];
        bytes[0] = 0x01;
        assert!(ProgramName::new(bytes).is_err());
    }

    #[test]
    fn name_invalid_high_ascii() {
        let mut bytes = [b' '; 12];
        bytes[0] = 0x80;
        assert!(ProgramName::new(bytes).is_err());
    }

    #[test]
    fn name_from_string_exact() {
        let name = ProgramName::from_string("Hello World!").unwrap();
        assert_eq!(name.as_str(), "Hello World!");
    }

    #[test]
    fn name_from_string_short_pads() {
        let name = ProgramName::from_string("Hi").unwrap();
        assert_eq!(name.as_bytes(), b"Hi          ");
        assert_eq!(name.as_str(), "Hi");
    }

    #[test]
    fn name_from_string_long_truncates() {
        let name = ProgramName::from_string("This Is Too Long!").unwrap();
        // Truncated to 12 bytes: "This Is Too " -> as_str trims trailing spaces.
        assert_eq!(name.as_str(), "This Is Too");
        assert_eq!(name.as_bytes(), b"This Is Too ");
    }

    #[test]
    fn name_from_string_empty() {
        let name = ProgramName::from_string("").unwrap();
        assert_eq!(name.as_str(), "");
        assert_eq!(name.as_bytes(), &[b' '; 12]);
    }

    #[test]
    fn name_display() {
        let name = ProgramName::from_string("Test").unwrap();
        assert_eq!(format!("{name}"), "Test");
    }

    #[test]
    fn name_default() {
        let name = ProgramName::default();
        assert_eq!(name.as_bytes(), &[b' '; 12]);
    }

    #[test]
    fn name_clone_eq() {
        let a = ProgramName::from_string("Test").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ---------------------------------------------------------------
    // SynthParams from_bytes / to_bytes round-trip
    // ---------------------------------------------------------------

    /// Build a valid 156-byte synth blob with known values.
    fn make_synth_blob() -> [u8; 156] {
        let mut buf = [0u8; 156];
        buf[0..4].copy_from_slice(b"PROG");
        buf[4..16].copy_from_slice(b"TestProgram!");
        buf[16] = 2; // octave = center
        buf[17] = 64; // portamento
        buf[18] = 1; // key_trig = on
        write_10bit(&mut buf, 19, 500); // voice_mode_depth
        buf[21] = 3; // voice_mode_type (Poly, 1-based)
        buf[22] = 2; // vco1_wave = Saw
        buf[23] = 1; // vco1_octave = 8'
        write_10bit(&mut buf, 24, 512); // vco1_pitch
        write_10bit(&mut buf, 26, 300); // vco1_shape
        buf[28] = 1; // vco2_wave = Tri
        buf[29] = 2; // vco2_octave = 4'
        write_10bit(&mut buf, 30, 256); // vco2_pitch
        write_10bit(&mut buf, 32, 100); // vco2_shape
        buf[34] = 0; // sync ON (inverted)
        buf[35] = 1; // ring OFF (inverted)
        write_10bit(&mut buf, 36, 0); // cross_mod_depth
        buf[38] = 1; // multi_type = VPM
        buf[39] = 2; // select_noise = Peak
        buf[40] = 5; // select_vpm
        buf[41] = 3; // select_user
        write_10bit(&mut buf, 42, 100); // shape_noise
        write_10bit(&mut buf, 44, 200); // shape_vpm
        write_10bit(&mut buf, 46, 300); // shape_user
        write_10bit(&mut buf, 48, 400); // shift_shape_noise
        write_10bit(&mut buf, 50, 500); // shift_shape_vpm
        write_10bit(&mut buf, 52, 600); // shift_shape_user
        write_10bit(&mut buf, 54, 700); // vco1_level
        write_10bit(&mut buf, 56, 800); // vco2_level
        write_10bit(&mut buf, 58, 900); // multi_level
        write_10bit(&mut buf, 60, 1023); // cutoff
        write_10bit(&mut buf, 62, 512); // resonance
        buf[64] = 1; // cutoff_drive = 50%
        buf[65] = 2; // cutoff_keytrack = 100%
        write_10bit(&mut buf, 66, 100); // amp_eg_attack
        write_10bit(&mut buf, 68, 200); // amp_eg_decay
        write_10bit(&mut buf, 70, 800); // amp_eg_sustain
        write_10bit(&mut buf, 72, 300); // amp_eg_release
        write_10bit(&mut buf, 74, 50); // eg_attack
        write_10bit(&mut buf, 76, 100); // eg_decay
        write_10bit(&mut buf, 78, 512); // eg_int
        buf[80] = 0; // eg_target = Cutoff
        buf[81] = 1; // lfo_wave = Tri
        buf[82] = 1; // lfo_mode = Normal
        write_10bit(&mut buf, 83, 400); // lfo_rate
        write_10bit(&mut buf, 85, 300); // lfo_int
        buf[87] = 0; // lfo_target = Cutoff
        buf[88] = 1; // mod_fx_on
        buf[89] = 1; // mod_fx_type (1-based = Chorus)
        buf[90] = 3; // mod_fx_chorus sub-type
        buf[91] = 1; // mod_fx_ensemble sub-type
        buf[92] = 5; // mod_fx_phaser sub-type
        buf[93] = 2; // mod_fx_flanger sub-type
        buf[94] = 7; // mod_fx_user sub-type
        write_10bit(&mut buf, 95, 600); // mod_fx_time
        write_10bit(&mut buf, 97, 700); // mod_fx_depth
        buf[99] = 1; // delay_on
        buf[100] = 4; // delay_sub_type = Tape
        write_10bit(&mut buf, 101, 500); // delay_time
        write_10bit(&mut buf, 103, 400); // delay_depth
        buf[105] = 1; // reverb_on
        buf[106] = 3; // reverb_sub_type = Plate
        write_10bit(&mut buf, 107, 600); // reverb_time
        write_10bit(&mut buf, 109, 300); // reverb_depth
        buf[111] = 12; // bend_range_plus
        buf[112] = 2; // bend_range_minus
        buf[113] = 16; // joystick_assign_plus = Cutoff
        buf[114] = 200; // joystick_range_plus
        buf[115] = 0; // joystick_assign_minus = None
        buf[116] = 100; // joystick_range_minus
        buf[117] = 0; // cv_in_mode = Modulation
        buf[118] = 1; // cv_in1_assign = GateTime
        buf[119] = 150; // cv_in1_range
        buf[120] = 2; // cv_in2_assign = PitchBend
        buf[121] = 80; // cv_in2_range
        buf[122] = 5; // micro_tuning = Kirnberger
        buf[123] = 12; // scale_key = 0
        buf[124] = 50; // program_tuning = 0 cents
        buf[125] = 1; // lfo_key_sync = on
        buf[126] = 0; // lfo_voice_sync = off
        buf[127] = 2; // lfo_target_osc = VCO2
        buf[128] = 64; // cutoff_velocity
        buf[129] = 127; // amp_velocity
        buf[130] = 1; // multi_octave = 8'
        buf[131] = 0; // multi_routing = PreVCF
        buf[132] = 0; // eg_legato = off
        buf[133] = 1; // portamento_mode = On
        buf[134] = 0; // portamento_bpm_sync = off
        buf[135] = 102; // program_level
        buf[136] = 100; // vpm_param1
        buf[137] = 50; // vpm_param2
        buf[138] = 150; // vpm_param3
        buf[139] = 0; // vpm_param4
        buf[140] = 200; // vpm_param5
        buf[141] = 75; // vpm_param6
        buf[142] = 10; // user_param1
        buf[143] = 20; // user_param2
        buf[144] = 30; // user_param3
        buf[145] = 40; // user_param4
        buf[146] = 50; // user_param5
        buf[147] = 60; // user_param6
                       // Byte 148: param5_type=1(Bipolar) bits 0-1, param6_type=2(Select) bits 2-3
        buf[148] = 0x01 | (0x02 << 2);
        // Byte 149: param1_type=0, param2_type=1, param3_type=2, param4_type=0
        buf[149] = (0x01 << 2) | (0x02 << 4);
        buf[150] = 13; // program_transpose = 0
        write_10bit(&mut buf, 151, 512); // delay_dry_wet
        write_10bit(&mut buf, 153, 768); // reverb_dry_wet
        buf[155] = 28; // midi_after_touch_assign = PortamentoTime
        buf
    }

    #[test]
    fn synth_params_from_bytes_valid() {
        let blob = make_synth_blob();
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.name.as_str(), "TestProgram!");
        assert_eq!(params.octave, 2);
        assert_eq!(params.portamento, 64);
        assert!(params.key_trig);
        assert_eq!(params.voice_mode_depth, 500);
        assert_eq!(params.voice_mode_type, 3);
        assert_eq!(params.vco1_wave, VcoWave::Saw);
        assert_eq!(params.vco1_octave, VcoOctave::Eight);
        assert_eq!(params.vco1_pitch, 512);
        assert_eq!(params.vco1_shape, 300);
        assert_eq!(params.vco2_wave, VcoWave::Tri);
        assert_eq!(params.vco2_octave, VcoOctave::Four);
        // sync: blob 0 = ON
        assert!(params.sync);
        // ring: blob 1 = OFF
        assert!(!params.ring);
        assert_eq!(params.cutoff, 1023);
        assert_eq!(params.cutoff_drive, CutoffDrive::Half);
        assert_eq!(params.cutoff_keytrack, CutoffKeytrack::Full);
        assert!(params.mod_fx_on);
        assert_eq!(params.mod_fx_type, 1);
        assert_eq!(params.delay_sub_type, DelaySubType::Tape);
        assert_eq!(params.reverb_sub_type, ReverbSubType::Plate);
        assert_eq!(params.bend_range_plus, 12);
        assert_eq!(params.joystick_assign_plus, ModAssignTarget::Cutoff);
        assert_eq!(params.user_param5_type, UserParamType::Bipolar);
        assert_eq!(params.user_param6_type, UserParamType::Select);
        assert_eq!(params.user_param1_type, UserParamType::Percent);
        assert_eq!(params.user_param2_type, UserParamType::Bipolar);
        assert_eq!(params.user_param3_type, UserParamType::Select);
        assert_eq!(params.user_param4_type, UserParamType::Percent);
        assert_eq!(
            params.midi_after_touch_assign,
            ModAssignTarget::PortamentoTime
        );
    }

    #[test]
    fn synth_params_round_trip() {
        let blob = make_synth_blob();
        let params = SynthParams::from_bytes(&blob).unwrap();
        let out = params.to_bytes();
        assert_eq!(&out[..], &blob[..]);
    }

    #[test]
    fn synth_params_too_short() {
        let short = [0u8; 100];
        assert!(SynthParams::from_bytes(&short).is_err());
    }

    #[test]
    fn synth_params_wrong_magic() {
        let mut blob = make_synth_blob();
        blob[0..4].copy_from_slice(b"GLOB");
        assert!(SynthParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn synth_params_sync_ring_inversion() {
        let mut blob = make_synth_blob();
        // Set sync=1 (OFF), ring=0 (ON) in blob
        blob[34] = 1;
        blob[35] = 0;
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert!(!params.sync);
        assert!(params.ring);

        // Round-trip preserves the inversion.
        let out = params.to_bytes();
        assert_eq!(out[34], 1);
        assert_eq!(out[35], 0);
    }

    #[test]
    fn synth_params_10bit_boundaries() {
        let mut blob = make_synth_blob();
        // Set cutoff to 0 and resonance to 1023.
        write_10bit(&mut blob, 60, 0);
        write_10bit(&mut blob, 62, 1023);
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.cutoff, 0);
        assert_eq!(params.resonance, 1023);
        let out = params.to_bytes();
        assert_eq!(read_10bit(&out, 60), 0);
        assert_eq!(read_10bit(&out, 62), 1023);
    }

    #[test]
    fn synth_params_voice_mode_type_enum() {
        let blob = make_synth_blob();
        let params = SynthParams::from_bytes(&blob).unwrap();
        // voice_mode_type=3, which is 1-based, so 3-1=2 => Chord.
        let vmt = params.voice_mode_type_enum().unwrap();
        assert_eq!(vmt, VoiceModeType::Chord);
    }

    #[test]
    fn synth_params_mod_fx_type_enum() {
        let blob = make_synth_blob();
        let params = SynthParams::from_bytes(&blob).unwrap();
        // mod_fx_type=1, 1-based => 0 => Chorus.
        let mft = params.mod_fx_type_enum().unwrap();
        assert_eq!(mft, ModFxType::Chorus);
    }

    #[test]
    fn synth_params_mod_fx_type_all_values() {
        let mut blob = make_synth_blob();
        for (raw, expected) in [
            (1, ModFxType::Chorus),
            (2, ModFxType::Ensemble),
            (3, ModFxType::Phaser),
            (4, ModFxType::Flanger),
            (5, ModFxType::User),
        ] {
            blob[89] = raw;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.mod_fx_type_enum().unwrap(), expected);
        }
    }

    #[test]
    fn synth_params_voice_mode_type_all_values() {
        let mut blob = make_synth_blob();
        for (raw, expected) in [
            (1, VoiceModeType::Poly),
            (2, VoiceModeType::Unison),
            (3, VoiceModeType::Chord),
            (4, VoiceModeType::Arp),
        ] {
            blob[21] = raw;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.voice_mode_type_enum().unwrap(), expected);
        }
    }

    #[test]
    fn synth_params_voice_mode_type_zero_is_err() {
        let mut blob = make_synth_blob();
        blob[21] = 0;
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert!(params.voice_mode_type_enum().is_err());
    }

    #[test]
    fn synth_params_mod_fx_type_zero_is_err() {
        let mut blob = make_synth_blob();
        blob[89] = 0;
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert!(params.mod_fx_type_enum().is_err());
    }

    #[test]
    fn synth_params_user_param_type_bit_packing() {
        let mut blob = make_synth_blob();
        // Set all user param types to Select (2).
        buf_set_all_user_param_types(&mut blob, 2);
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.user_param1_type, UserParamType::Select);
        assert_eq!(params.user_param2_type, UserParamType::Select);
        assert_eq!(params.user_param3_type, UserParamType::Select);
        assert_eq!(params.user_param4_type, UserParamType::Select);
        assert_eq!(params.user_param5_type, UserParamType::Select);
        assert_eq!(params.user_param6_type, UserParamType::Select);
        let out = params.to_bytes();
        assert_eq!(out[148], 0x02 | (0x02 << 2));
        assert_eq!(out[149], 0x02 | (0x02 << 2) | (0x02 << 4) | (0x02 << 6));
    }

    fn buf_set_all_user_param_types(buf: &mut [u8], val: u8) {
        buf[148] = (val & 0x03) | ((val & 0x03) << 2);
        buf[149] = (val & 0x03) | ((val & 0x03) << 2) | ((val & 0x03) << 4) | ((val & 0x03) << 6);
    }

    #[test]
    fn synth_params_all_vco_waves() {
        let mut blob = make_synth_blob();
        for (prog_val, expected) in [(0, VcoWave::Sqr), (1, VcoWave::Tri), (2, VcoWave::Saw)] {
            blob[22] = prog_val;
            blob[28] = prog_val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.vco1_wave, expected);
            assert_eq!(params.vco2_wave, expected);
        }
    }

    #[test]
    fn synth_params_all_vco_octaves() {
        let mut blob = make_synth_blob();
        for (prog_val, expected) in [
            (0, VcoOctave::Sixteen),
            (1, VcoOctave::Eight),
            (2, VcoOctave::Four),
            (3, VcoOctave::Two),
        ] {
            blob[23] = prog_val;
            blob[29] = prog_val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.vco1_octave, expected);
            assert_eq!(params.vco2_octave, expected);
        }
    }

    #[test]
    fn synth_params_micro_tuning_enum() {
        let blob = make_synth_blob();
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.micro_tuning_enum().unwrap(), MicroTuning::Kirnberger);
    }

    #[test]
    fn synth_params_micro_tuning_out_of_range() {
        let mut blob = make_synth_blob();
        blob[122] = 100; // beyond enum range
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert!(params.micro_tuning_enum().is_err());
    }

    #[test]
    fn synth_params_boolean_fields() {
        let mut blob = make_synth_blob();
        // Test all boolean fields at both states.
        blob[18] = 0; // key_trig off
        blob[88] = 0; // mod_fx off
        blob[99] = 0; // delay off
        blob[105] = 0; // reverb off
        blob[125] = 0; // lfo_key_sync off
        blob[126] = 0; // lfo_voice_sync off
        blob[132] = 0; // eg_legato off
        blob[134] = 0; // portamento_bpm_sync off
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert!(!params.key_trig);
        assert!(!params.mod_fx_on);
        assert!(!params.delay_on);
        assert!(!params.reverb_on);
        assert!(!params.lfo_key_sync);
        assert!(!params.lfo_voice_sync);
        assert!(!params.eg_legato);
        assert!(!params.portamento_bpm_sync);

        blob[18] = 1;
        blob[88] = 1;
        blob[99] = 1;
        blob[105] = 1;
        blob[125] = 1;
        blob[126] = 1;
        blob[132] = 1;
        blob[134] = 1;
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert!(params.key_trig);
        assert!(params.mod_fx_on);
        assert!(params.delay_on);
        assert!(params.reverb_on);
        assert!(params.lfo_key_sync);
        assert!(params.lfo_voice_sync);
        assert!(params.eg_legato);
        assert!(params.portamento_bpm_sync);
    }

    #[test]
    fn synth_params_default_round_trips() {
        let params = SynthParams::default();
        let bytes = params.to_bytes();
        let recovered = SynthParams::from_bytes(&bytes).unwrap();
        assert_eq!(params, recovered);
    }

    #[test]
    fn synth_params_eg_targets() {
        let mut blob = make_synth_blob();
        for (val, expected) in [
            (0, EgTarget::Cutoff),
            (1, EgTarget::Pitch2),
            (2, EgTarget::Pitch),
        ] {
            blob[80] = val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.eg_target, expected);
        }
    }

    #[test]
    fn synth_params_lfo_variants() {
        let mut blob = make_synth_blob();
        for (val, expected) in [(0, LfoWave::Sqr), (1, LfoWave::Tri), (2, LfoWave::Saw)] {
            blob[81] = val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.lfo_wave, expected);
        }
        for (val, expected) in [
            (0, LfoMode::OneShot),
            (1, LfoMode::Normal),
            (2, LfoMode::Bpm),
        ] {
            blob[82] = val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.lfo_mode, expected);
        }
        for (val, expected) in [
            (0, LfoTarget::Cutoff),
            (1, LfoTarget::Shape),
            (2, LfoTarget::Pitch),
        ] {
            blob[87] = val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.lfo_target, expected);
        }
    }

    #[test]
    fn synth_params_mod_assign_targets() {
        let mut blob = make_synth_blob();
        // Test a few assign targets at different offsets.
        blob[113] = 0; // joystick_assign_plus = None
        blob[115] = 28; // joystick_assign_minus = PortamentoTime
        blob[155] = 17; // midi_after_touch_assign = Resonance
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.joystick_assign_plus, ModAssignTarget::None);
        assert_eq!(
            params.joystick_assign_minus,
            ModAssignTarget::PortamentoTime
        );
        assert_eq!(params.midi_after_touch_assign, ModAssignTarget::Resonance);
    }

    #[test]
    fn synth_params_delay_sub_types() {
        let mut blob = make_synth_blob();
        for prog_val in 0..=19u8 {
            blob[100] = prog_val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.delay_sub_type.to_program_value(), prog_val);
        }
    }

    #[test]
    fn synth_params_reverb_sub_types() {
        let mut blob = make_synth_blob();
        for prog_val in 0..=17u8 {
            blob[106] = prog_val;
            let params = SynthParams::from_bytes(&blob).unwrap();
            assert_eq!(params.reverb_sub_type.to_program_value(), prog_val);
        }
    }

    #[test]
    fn synth_params_select_vpm_enum() {
        let mut blob = make_synth_blob();
        blob[40] = 15;
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.select_vpm_enum().unwrap(), MultiSelectVpm::Throat);
    }

    #[test]
    fn synth_params_select_user_enum() {
        let mut blob = make_synth_blob();
        blob[41] = 0;
        let params = SynthParams::from_bytes(&blob).unwrap();
        assert_eq!(params.select_user_enum().unwrap(), MultiSelectUser::User1);
    }
}
