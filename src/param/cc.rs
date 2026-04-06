//! CC parameter map for the Minilogue XD.
//!
//! Maps every Control Change number from the MIDI Implementation chart to a
//! strongly typed [`CcParam`] variant. The [`CcParamReceiver`] handles the
//! stateful CC63-preceded 10-bit protocol.

use crate::error::Result;
use crate::message::channel::ControlChange;
use crate::message::types::{U4, U7};
use crate::param::encoding::{TenBitParam, TenBitReceiver};
use crate::param::enums::{
    CutoffDrive, CutoffKeytrack, DelaySubType, EgTarget, LfoMode, LfoTarget, LfoWave, ModFxType,
    MultiType, ReverbSubType, Ring, Sync, VcoOctave, VcoWave,
};
use crate::param::SteppedParam;

// ---------------------------------------------------------------------------
// CcParam
// ---------------------------------------------------------------------------

/// A typed CC parameter from the Minilogue XD MIDI implementation.
///
/// Each variant corresponds to a specific CC number and carries the
/// appropriate value type (10-bit for high-resolution continuous parameters,
/// enum for stepped parameters, `bool` for on/off switches, or `U7` for
/// simple continuous / context-dependent / special parameters).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CcParam {
    // -- Special --
    /// CC 0: Bank Select MSB.
    BankSelectMsb(U7),
    /// CC 32: Bank Select LSB.
    BankSelectLsb(U7),
    /// CC 63: Data Entry LSB (buffered by receiver for 10-bit params).
    DataEntryLsb(U7),
    /// CC 64: Damper pedal (continuous 0-127 on receive).
    Damper(U7),

    // -- NRPN addressing --
    /// CC 6: Data Entry MSB (used by NRPN protocol).
    DataEntryMsb(U7),
    /// CC 98: NRPN LSB.
    NrpnLsb(U7),
    /// CC 99: NRPN MSB.
    NrpnMsb(U7),

    // -- Simple continuous --
    /// CC 1: Modulation wheel 1.
    Modulation1(U7),
    /// CC 2: Modulation wheel 2.
    Modulation2(U7),
    /// CC 5: Portamento time.
    PortamentoTime(U7),
    /// CC 118: CV input 1.
    CvIn1(U7),
    /// CC 119: CV input 2.
    CvIn2(U7),

    // -- High-res 10-bit (CC63-preceded) --
    /// CC 16: Amp EG attack.
    AmpEgAttack(TenBitParam),
    /// CC 17: Amp EG decay.
    AmpEgDecay(TenBitParam),
    /// CC 18: Amp EG sustain.
    AmpEgSustain(TenBitParam),
    /// CC 19: Amp EG release.
    AmpEgRelease(TenBitParam),
    /// CC 20: EG attack.
    EgAttack(TenBitParam),
    /// CC 21: EG decay.
    EgDecay(TenBitParam),
    /// CC 22: EG intensity.
    EgInt(TenBitParam),
    /// CC 24: LFO rate.
    LfoRate(TenBitParam),
    /// CC 26: LFO intensity.
    LfoInt(TenBitParam),
    /// CC 27: Voice mode depth.
    VoiceModeDepth(TenBitParam),
    /// CC 28: Mod-FX time.
    ModFxTime(TenBitParam),
    /// CC 29: Mod-FX depth.
    ModFxDepth(TenBitParam),
    /// CC 33: Multi-engine level.
    MultiLevel(TenBitParam),
    /// CC 34: VCO1 pitch.
    Vco1Pitch(TenBitParam),
    /// CC 35: VCO2 pitch.
    Vco2Pitch(TenBitParam),
    /// CC 36: VCO1 shape.
    Vco1Shape(TenBitParam),
    /// CC 37: VCO2 shape.
    Vco2Shape(TenBitParam),
    /// CC 39: VCO1 level.
    Vco1Level(TenBitParam),
    /// CC 40: VCO2 level.
    Vco2Level(TenBitParam),
    /// CC 41: Cross-modulation depth.
    CrossModDepth(TenBitParam),
    /// CC 43: Filter cutoff.
    Cutoff(TenBitParam),
    /// CC 44: Filter resonance.
    Resonance(TenBitParam),
    /// CC 54: Multi-engine shape.
    MultiShape(TenBitParam),
    /// CC 59: Voice mode depth (alternate, no display).
    VoiceModeDepthAlt(TenBitParam),
    /// CC 104: Multi-engine shift shape.
    MultiShiftShape(TenBitParam),
    /// CC 105: Delay time.
    DelayTime(TenBitParam),
    /// CC 106: Delay depth.
    DelayDepth(TenBitParam),
    /// CC 107: Delay dry/wet.
    DelayDryWet(TenBitParam),
    /// CC 108: Reverb time.
    ReverbTime(TenBitParam),
    /// CC 109: Reverb depth.
    ReverbDepth(TenBitParam),
    /// CC 110: Reverb dry/wet.
    ReverbDryWet(TenBitParam),

    // -- Stepped (enum-valued) --
    /// CC 23: EG target.
    EgTarget(EgTarget),
    /// CC 48: VCO1 octave.
    Vco1Octave(VcoOctave),
    /// CC 49: VCO2 octave.
    Vco2Octave(VcoOctave),
    /// CC 50: VCO1 waveform.
    Vco1Wave(VcoWave),
    /// CC 51: VCO2 waveform.
    Vco2Wave(VcoWave),
    /// CC 53: Multi-engine type.
    MultiType(MultiType),
    /// CC 56: LFO target.
    LfoTarget(LfoTarget),
    /// CC 57: LFO waveform.
    LfoWave(LfoWave),
    /// CC 58: LFO mode.
    LfoMode(LfoMode),
    /// CC 80: VCO sync on/off.
    Sync(Sync),
    /// CC 81: Ring modulation on/off.
    Ring(Ring),
    /// CC 83: Cutoff key-tracking.
    CutoffKeytrack(CutoffKeytrack),
    /// CC 84: Cutoff drive.
    CutoffDrive(CutoffDrive),
    /// CC 88: Mod-FX type.
    ModFxType(ModFxType),
    /// CC 89: Delay sub-type.
    DelaySubType(DelaySubType),
    /// CC 90: Reverb sub-type.
    ReverbSubType(ReverbSubType),

    // -- On/Off --
    /// CC 92: Mod-FX on/off.
    ModFxOnOff(bool),
    /// CC 93: Delay on/off.
    DelayOnOff(bool),
    /// CC 94: Reverb on/off.
    ReverbOnOff(bool),

    // -- Context-dependent (raw U7) --
    /// CC 96: Mod-FX sub-type (interpretation depends on current ModFxType).
    ModFxSubType(U7),
    /// CC 103: Multi-engine select (interpretation depends on current MultiType).
    MultiSelect(U7),
}

impl CcParam {
    /// Returns the CC number for this parameter.
    pub fn cc_number(&self) -> u8 {
        match self {
            Self::BankSelectMsb(_) => 0,
            Self::Modulation1(_) => 1,
            Self::Modulation2(_) => 2,
            Self::PortamentoTime(_) => 5,
            Self::DataEntryMsb(_) => 6,
            Self::AmpEgAttack(_) => 16,
            Self::AmpEgDecay(_) => 17,
            Self::AmpEgSustain(_) => 18,
            Self::AmpEgRelease(_) => 19,
            Self::EgAttack(_) => 20,
            Self::EgDecay(_) => 21,
            Self::EgInt(_) => 22,
            Self::EgTarget(_) => 23,
            Self::LfoRate(_) => 24,
            Self::LfoInt(_) => 26,
            Self::VoiceModeDepth(_) => 27,
            Self::ModFxTime(_) => 28,
            Self::ModFxDepth(_) => 29,
            Self::BankSelectLsb(_) => 32,
            Self::MultiLevel(_) => 33,
            Self::Vco1Pitch(_) => 34,
            Self::Vco2Pitch(_) => 35,
            Self::Vco1Shape(_) => 36,
            Self::Vco2Shape(_) => 37,
            Self::Vco1Level(_) => 39,
            Self::Vco2Level(_) => 40,
            Self::CrossModDepth(_) => 41,
            Self::Cutoff(_) => 43,
            Self::Resonance(_) => 44,
            Self::Vco1Octave(_) => 48,
            Self::Vco2Octave(_) => 49,
            Self::Vco1Wave(_) => 50,
            Self::Vco2Wave(_) => 51,
            Self::MultiType(_) => 53,
            Self::MultiShape(_) => 54,
            Self::LfoTarget(_) => 56,
            Self::LfoWave(_) => 57,
            Self::LfoMode(_) => 58,
            Self::VoiceModeDepthAlt(_) => 59,
            Self::DataEntryLsb(_) => 63,
            Self::Damper(_) => 64,
            Self::Sync(_) => 80,
            Self::Ring(_) => 81,
            Self::CutoffKeytrack(_) => 83,
            Self::CutoffDrive(_) => 84,
            Self::ModFxType(_) => 88,
            Self::DelaySubType(_) => 89,
            Self::ReverbSubType(_) => 90,
            Self::ModFxOnOff(_) => 92,
            Self::DelayOnOff(_) => 93,
            Self::ReverbOnOff(_) => 94,
            Self::ModFxSubType(_) => 96,
            Self::NrpnLsb(_) => 98,
            Self::NrpnMsb(_) => 99,
            Self::MultiSelect(_) => 103,
            Self::MultiShiftShape(_) => 104,
            Self::DelayTime(_) => 105,
            Self::DelayDepth(_) => 106,
            Self::DelayDryWet(_) => 107,
            Self::ReverbTime(_) => 108,
            Self::ReverbDepth(_) => 109,
            Self::ReverbDryWet(_) => 110,
            Self::CvIn1(_) => 118,
            Self::CvIn2(_) => 119,
        }
    }

    /// Encodes this parameter as CC messages for transmission.
    ///
    /// High-resolution 10-bit parameters return two messages: `[CC63(lsb), CC_N(msb)]`.
    /// All other parameters return a single message: `[CC_N(value)]`.
    ///
    /// # Errors
    ///
    /// Returns an error if any U7 value construction fails (should not happen
    /// for valid parameter values).
    pub fn to_cc_messages(&self, channel: U4) -> Result<Vec<ControlChange>> {
        match self {
            // High-res 10-bit: emit CC63 then parameter CC
            Self::AmpEgAttack(v)
            | Self::AmpEgDecay(v)
            | Self::AmpEgSustain(v)
            | Self::AmpEgRelease(v)
            | Self::EgAttack(v)
            | Self::EgDecay(v)
            | Self::EgInt(v)
            | Self::LfoRate(v)
            | Self::LfoInt(v)
            | Self::VoiceModeDepth(v)
            | Self::ModFxTime(v)
            | Self::ModFxDepth(v)
            | Self::MultiLevel(v)
            | Self::Vco1Pitch(v)
            | Self::Vco2Pitch(v)
            | Self::Vco1Shape(v)
            | Self::Vco2Shape(v)
            | Self::Vco1Level(v)
            | Self::Vco2Level(v)
            | Self::CrossModDepth(v)
            | Self::Cutoff(v)
            | Self::Resonance(v)
            | Self::MultiShape(v)
            | Self::VoiceModeDepthAlt(v)
            | Self::MultiShiftShape(v)
            | Self::DelayTime(v)
            | Self::DelayDepth(v)
            | Self::DelayDryWet(v)
            | Self::ReverbTime(v)
            | Self::ReverbDepth(v)
            | Self::ReverbDryWet(v) => Ok(vec![
                ControlChange {
                    channel,
                    controller: U7::new(63)?,
                    value: U7::new(v.lsb())?,
                },
                ControlChange {
                    channel,
                    controller: U7::new(self.cc_number())?,
                    value: U7::new(v.msb())?,
                },
            ]),

            // Stepped enums: use to_tx_value()
            Self::EgTarget(v) => self.single_cc(channel, v.to_tx_value()),
            Self::Vco1Octave(v) => self.single_cc(channel, v.to_tx_value()),
            Self::Vco2Octave(v) => self.single_cc(channel, v.to_tx_value()),
            Self::Vco1Wave(v) => self.single_cc(channel, v.to_tx_value()),
            Self::Vco2Wave(v) => self.single_cc(channel, v.to_tx_value()),
            Self::MultiType(v) => self.single_cc(channel, v.to_tx_value()),
            Self::LfoTarget(v) => self.single_cc(channel, v.to_tx_value()),
            Self::LfoWave(v) => self.single_cc(channel, v.to_tx_value()),
            Self::LfoMode(v) => self.single_cc(channel, v.to_tx_value()),
            Self::Sync(v) => self.single_cc(channel, v.to_tx_value()),
            Self::Ring(v) => self.single_cc(channel, v.to_tx_value()),
            Self::CutoffKeytrack(v) => self.single_cc(channel, v.to_tx_value()),
            Self::CutoffDrive(v) => self.single_cc(channel, v.to_tx_value()),
            Self::ModFxType(v) => self.single_cc(channel, v.to_tx_value()),
            Self::DelaySubType(v) => self.single_cc(channel, v.to_tx_value()),
            Self::ReverbSubType(v) => self.single_cc(channel, v.to_tx_value()),

            // On/off: true=127, false=0
            Self::ModFxOnOff(on) => self.single_cc(channel, if *on { 127 } else { 0 }),
            Self::DelayOnOff(on) => self.single_cc(channel, if *on { 127 } else { 0 }),
            Self::ReverbOnOff(on) => self.single_cc(channel, if *on { 127 } else { 0 }),

            // Simple / special / context-dependent: direct U7
            Self::BankSelectMsb(v)
            | Self::BankSelectLsb(v)
            | Self::DataEntryLsb(v)
            | Self::Damper(v)
            | Self::DataEntryMsb(v)
            | Self::NrpnLsb(v)
            | Self::NrpnMsb(v)
            | Self::Modulation1(v)
            | Self::Modulation2(v)
            | Self::PortamentoTime(v)
            | Self::CvIn1(v)
            | Self::CvIn2(v)
            | Self::ModFxSubType(v)
            | Self::MultiSelect(v) => Ok(vec![ControlChange {
                channel,
                controller: U7::new(self.cc_number())?,
                value: *v,
            }]),
        }
    }

    /// Helper: construct a single CC message.
    fn single_cc(&self, channel: U4, value: u8) -> Result<Vec<ControlChange>> {
        Ok(vec![ControlChange {
            channel,
            controller: U7::new(self.cc_number())?,
            value: U7::new(value)?,
        }])
    }
}

// ---------------------------------------------------------------------------
// CcParamReceiver
// ---------------------------------------------------------------------------

/// Stateful receiver that converts raw [`ControlChange`] messages into
/// typed [`CcParam`] values.
///
/// The Minilogue XD protocol sends 10-bit parameters as two CC messages:
/// CC63 (LSB) followed by the parameter's own CC (MSB). This receiver
/// buffers the CC63 value and combines it with the next high-res CC.
#[derive(Debug, Default)]
pub struct CcParamReceiver {
    ten_bit: TenBitReceiver,
}

impl CcParamReceiver {
    /// Creates a new receiver with no pending LSB.
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes an incoming [`ControlChange`] and returns a [`CcParam`] if
    /// the message maps to a known parameter.
    ///
    /// - CC 63 buffers the LSB and also returns `Some(CcParam::DataEntryLsb(..))`.
    /// - High-res CCs consume the pending LSB (or assume 0) and return
    ///   the 10-bit variant.
    /// - Stepped CCs decode via `from_rx_value`.
    /// - On/off CCs use the >= 64 threshold.
    /// - Unknown CCs reset pending LSB and return `None`.
    pub fn feed(&mut self, cc: &ControlChange) -> Option<CcParam> {
        let cc_num = cc.controller.value();
        let val = cc.value.value();

        match cc_num {
            // CC 63: buffer LSB, also return it as a param
            63 => {
                self.ten_bit.feed_lsb(val);
                Some(CcParam::DataEntryLsb(cc.value))
            }

            // High-res 10-bit parameters (CC63 precedes, or LSB assumed 0)
            16 => Some(CcParam::AmpEgAttack(self.ten_bit.take_value(val))),
            17 => Some(CcParam::AmpEgDecay(self.ten_bit.take_value(val))),
            18 => Some(CcParam::AmpEgSustain(self.ten_bit.take_value(val))),
            19 => Some(CcParam::AmpEgRelease(self.ten_bit.take_value(val))),
            20 => Some(CcParam::EgAttack(self.ten_bit.take_value(val))),
            21 => Some(CcParam::EgDecay(self.ten_bit.take_value(val))),
            22 => Some(CcParam::EgInt(self.ten_bit.take_value(val))),
            24 => Some(CcParam::LfoRate(self.ten_bit.take_value(val))),
            26 => Some(CcParam::LfoInt(self.ten_bit.take_value(val))),
            27 => Some(CcParam::VoiceModeDepth(self.ten_bit.take_value(val))),
            28 => Some(CcParam::ModFxTime(self.ten_bit.take_value(val))),
            29 => Some(CcParam::ModFxDepth(self.ten_bit.take_value(val))),
            33 => Some(CcParam::MultiLevel(self.ten_bit.take_value(val))),
            34 => Some(CcParam::Vco1Pitch(self.ten_bit.take_value(val))),
            35 => Some(CcParam::Vco2Pitch(self.ten_bit.take_value(val))),
            36 => Some(CcParam::Vco1Shape(self.ten_bit.take_value(val))),
            37 => Some(CcParam::Vco2Shape(self.ten_bit.take_value(val))),
            39 => Some(CcParam::Vco1Level(self.ten_bit.take_value(val))),
            40 => Some(CcParam::Vco2Level(self.ten_bit.take_value(val))),
            41 => Some(CcParam::CrossModDepth(self.ten_bit.take_value(val))),
            43 => Some(CcParam::Cutoff(self.ten_bit.take_value(val))),
            44 => Some(CcParam::Resonance(self.ten_bit.take_value(val))),
            54 => Some(CcParam::MultiShape(self.ten_bit.take_value(val))),
            59 => Some(CcParam::VoiceModeDepthAlt(self.ten_bit.take_value(val))),
            104 => Some(CcParam::MultiShiftShape(self.ten_bit.take_value(val))),
            105 => Some(CcParam::DelayTime(self.ten_bit.take_value(val))),
            106 => Some(CcParam::DelayDepth(self.ten_bit.take_value(val))),
            107 => Some(CcParam::DelayDryWet(self.ten_bit.take_value(val))),
            108 => Some(CcParam::ReverbTime(self.ten_bit.take_value(val))),
            109 => Some(CcParam::ReverbDepth(self.ten_bit.take_value(val))),
            110 => Some(CcParam::ReverbDryWet(self.ten_bit.take_value(val))),

            // Stepped enums (decode via from_rx_value)
            23 => EgTarget::from_rx_value(val).ok().map(CcParam::EgTarget),
            48 => VcoOctave::from_rx_value(val).ok().map(CcParam::Vco1Octave),
            49 => VcoOctave::from_rx_value(val).ok().map(CcParam::Vco2Octave),
            50 => VcoWave::from_rx_value(val).ok().map(CcParam::Vco1Wave),
            51 => VcoWave::from_rx_value(val).ok().map(CcParam::Vco2Wave),
            53 => MultiType::from_rx_value(val).ok().map(CcParam::MultiType),
            56 => LfoTarget::from_rx_value(val).ok().map(CcParam::LfoTarget),
            57 => LfoWave::from_rx_value(val).ok().map(CcParam::LfoWave),
            58 => LfoMode::from_rx_value(val).ok().map(CcParam::LfoMode),
            80 => Sync::from_rx_value(val).ok().map(CcParam::Sync),
            81 => Ring::from_rx_value(val).ok().map(CcParam::Ring),
            83 => CutoffKeytrack::from_rx_value(val)
                .ok()
                .map(CcParam::CutoffKeytrack),
            84 => CutoffDrive::from_rx_value(val)
                .ok()
                .map(CcParam::CutoffDrive),
            88 => ModFxType::from_rx_value(val).ok().map(CcParam::ModFxType),
            89 => DelaySubType::from_rx_value(val)
                .ok()
                .map(CcParam::DelaySubType),
            90 => ReverbSubType::from_rx_value(val)
                .ok()
                .map(CcParam::ReverbSubType),

            // On/off (>=64 = true)
            92 => Some(CcParam::ModFxOnOff(val >= 64)),
            93 => Some(CcParam::DelayOnOff(val >= 64)),
            94 => Some(CcParam::ReverbOnOff(val >= 64)),

            // Context-dependent (raw U7)
            96 => Some(CcParam::ModFxSubType(cc.value)),
            103 => Some(CcParam::MultiSelect(cc.value)),

            // Simple continuous
            1 => Some(CcParam::Modulation1(cc.value)),
            2 => Some(CcParam::Modulation2(cc.value)),
            5 => Some(CcParam::PortamentoTime(cc.value)),
            118 => Some(CcParam::CvIn1(cc.value)),
            119 => Some(CcParam::CvIn2(cc.value)),

            // Special
            0 => Some(CcParam::BankSelectMsb(cc.value)),
            32 => Some(CcParam::BankSelectLsb(cc.value)),
            64 => Some(CcParam::Damper(cc.value)),

            // NRPN addressing pass-through
            6 => Some(CcParam::DataEntryMsb(cc.value)),
            98 => Some(CcParam::NrpnLsb(cc.value)),
            99 => Some(CcParam::NrpnMsb(cc.value)),

            // Unknown CC: reset pending LSB, return None
            _ => {
                self.ten_bit.reset();
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: make a ControlChange on channel 0.
    fn cc(controller: u8, value: u8) -> ControlChange {
        ControlChange {
            channel: U4::new(0).unwrap(),
            controller: U7::new(controller).unwrap(),
            value: U7::new(value).unwrap(),
        }
    }

    // === CC number mapping ===

    #[test]
    fn cc_number_bank_select_msb() {
        let p = CcParam::BankSelectMsb(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 0);
    }

    #[test]
    fn cc_number_modulation1() {
        let p = CcParam::Modulation1(U7::new(64).unwrap());
        assert_eq!(p.cc_number(), 1);
    }

    #[test]
    fn cc_number_modulation2() {
        let p = CcParam::Modulation2(U7::new(64).unwrap());
        assert_eq!(p.cc_number(), 2);
    }

    #[test]
    fn cc_number_portamento_time() {
        let p = CcParam::PortamentoTime(U7::new(64).unwrap());
        assert_eq!(p.cc_number(), 5);
    }

    #[test]
    fn cc_number_data_entry_msb() {
        let p = CcParam::DataEntryMsb(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 6);
    }

    #[test]
    fn cc_number_amp_eg_attack() {
        let p = CcParam::AmpEgAttack(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 16);
    }

    #[test]
    fn cc_number_amp_eg_decay() {
        let p = CcParam::AmpEgDecay(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 17);
    }

    #[test]
    fn cc_number_amp_eg_sustain() {
        let p = CcParam::AmpEgSustain(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 18);
    }

    #[test]
    fn cc_number_amp_eg_release() {
        let p = CcParam::AmpEgRelease(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 19);
    }

    #[test]
    fn cc_number_eg_attack() {
        let p = CcParam::EgAttack(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 20);
    }

    #[test]
    fn cc_number_eg_decay() {
        let p = CcParam::EgDecay(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 21);
    }

    #[test]
    fn cc_number_eg_int() {
        let p = CcParam::EgInt(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 22);
    }

    #[test]
    fn cc_number_eg_target() {
        let p = CcParam::EgTarget(EgTarget::Cutoff);
        assert_eq!(p.cc_number(), 23);
    }

    #[test]
    fn cc_number_lfo_rate() {
        let p = CcParam::LfoRate(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 24);
    }

    #[test]
    fn cc_number_lfo_int() {
        let p = CcParam::LfoInt(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 26);
    }

    #[test]
    fn cc_number_voice_mode_depth() {
        let p = CcParam::VoiceModeDepth(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 27);
    }

    #[test]
    fn cc_number_mod_fx_time() {
        let p = CcParam::ModFxTime(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 28);
    }

    #[test]
    fn cc_number_mod_fx_depth() {
        let p = CcParam::ModFxDepth(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 29);
    }

    #[test]
    fn cc_number_bank_select_lsb() {
        let p = CcParam::BankSelectLsb(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 32);
    }

    #[test]
    fn cc_number_multi_level() {
        let p = CcParam::MultiLevel(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 33);
    }

    #[test]
    fn cc_number_cutoff() {
        let p = CcParam::Cutoff(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 43);
    }

    #[test]
    fn cc_number_resonance() {
        let p = CcParam::Resonance(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 44);
    }

    #[test]
    fn cc_number_vco1_octave() {
        let p = CcParam::Vco1Octave(VcoOctave::Eight);
        assert_eq!(p.cc_number(), 48);
    }

    #[test]
    fn cc_number_multi_shape() {
        let p = CcParam::MultiShape(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 54);
    }

    #[test]
    fn cc_number_lfo_target() {
        let p = CcParam::LfoTarget(LfoTarget::Cutoff);
        assert_eq!(p.cc_number(), 56);
    }

    #[test]
    fn cc_number_voice_mode_depth_alt() {
        let p = CcParam::VoiceModeDepthAlt(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 59);
    }

    #[test]
    fn cc_number_data_entry_lsb() {
        let p = CcParam::DataEntryLsb(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 63);
    }

    #[test]
    fn cc_number_damper() {
        let p = CcParam::Damper(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 64);
    }

    #[test]
    fn cc_number_sync() {
        let p = CcParam::Sync(Sync::On);
        assert_eq!(p.cc_number(), 80);
    }

    #[test]
    fn cc_number_ring() {
        let p = CcParam::Ring(Ring::Off);
        assert_eq!(p.cc_number(), 81);
    }

    #[test]
    fn cc_number_cutoff_keytrack() {
        let p = CcParam::CutoffKeytrack(CutoffKeytrack::Full);
        assert_eq!(p.cc_number(), 83);
    }

    #[test]
    fn cc_number_cutoff_drive() {
        let p = CcParam::CutoffDrive(CutoffDrive::Half);
        assert_eq!(p.cc_number(), 84);
    }

    #[test]
    fn cc_number_mod_fx_type() {
        let p = CcParam::ModFxType(ModFxType::Chorus);
        assert_eq!(p.cc_number(), 88);
    }

    #[test]
    fn cc_number_delay_sub_type() {
        let p = CcParam::DelaySubType(DelaySubType::Stereo);
        assert_eq!(p.cc_number(), 89);
    }

    #[test]
    fn cc_number_reverb_sub_type() {
        let p = CcParam::ReverbSubType(ReverbSubType::Hall);
        assert_eq!(p.cc_number(), 90);
    }

    #[test]
    fn cc_number_mod_fx_on_off() {
        let p = CcParam::ModFxOnOff(true);
        assert_eq!(p.cc_number(), 92);
    }

    #[test]
    fn cc_number_delay_on_off() {
        let p = CcParam::DelayOnOff(false);
        assert_eq!(p.cc_number(), 93);
    }

    #[test]
    fn cc_number_reverb_on_off() {
        let p = CcParam::ReverbOnOff(true);
        assert_eq!(p.cc_number(), 94);
    }

    #[test]
    fn cc_number_mod_fx_sub_type() {
        let p = CcParam::ModFxSubType(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 96);
    }

    #[test]
    fn cc_number_nrpn_lsb() {
        let p = CcParam::NrpnLsb(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 98);
    }

    #[test]
    fn cc_number_nrpn_msb() {
        let p = CcParam::NrpnMsb(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 99);
    }

    #[test]
    fn cc_number_multi_select() {
        let p = CcParam::MultiSelect(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 103);
    }

    #[test]
    fn cc_number_multi_shift_shape() {
        let p = CcParam::MultiShiftShape(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 104);
    }

    #[test]
    fn cc_number_delay_time() {
        let p = CcParam::DelayTime(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 105);
    }

    #[test]
    fn cc_number_reverb_dry_wet() {
        let p = CcParam::ReverbDryWet(TenBitParam::new(0).unwrap());
        assert_eq!(p.cc_number(), 110);
    }

    #[test]
    fn cc_number_cv_in1() {
        let p = CcParam::CvIn1(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 118);
    }

    #[test]
    fn cc_number_cv_in2() {
        let p = CcParam::CvIn2(U7::new(0).unwrap());
        assert_eq!(p.cc_number(), 119);
    }

    // === CcParamReceiver: high-res with CC63 ===

    #[test]
    fn receiver_high_res_with_lsb() {
        let mut rx = CcParamReceiver::new();
        rx.feed(&cc(63, 5)); // LSB = 5
        let result = rx.feed(&cc(43, 64)); // Cutoff MSB = 64
        match result {
            Some(CcParam::Cutoff(v)) => {
                assert_eq!(v.value(), (64 << 3) | 5);
            }
            other => panic!("expected Cutoff, got {other:?}"),
        }
    }

    #[test]
    fn receiver_high_res_assumed_zero() {
        let mut rx = CcParamReceiver::new();
        // No CC63, LSB defaults to 0
        let result = rx.feed(&cc(43, 64));
        match result {
            Some(CcParam::Cutoff(v)) => {
                assert_eq!(v.value(), 64 << 3);
            }
            other => panic!("expected Cutoff, got {other:?}"),
        }
    }

    #[test]
    fn receiver_high_res_lsb_consumed() {
        let mut rx = CcParamReceiver::new();
        rx.feed(&cc(63, 7));
        let r1 = rx.feed(&cc(16, 10)); // AmpEgAttack
        assert!(matches!(r1, Some(CcParam::AmpEgAttack(v)) if v.value() == (10 << 3) | 7));

        // Second param without CC63 → LSB = 0
        let r2 = rx.feed(&cc(17, 10)); // AmpEgDecay
        assert!(matches!(r2, Some(CcParam::AmpEgDecay(v)) if v.value() == 10 << 3));
    }

    #[test]
    fn receiver_high_res_max_value() {
        let mut rx = CcParamReceiver::new();
        rx.feed(&cc(63, 7)); // LSB = 7 (max 3-bit)
        let result = rx.feed(&cc(43, 127)); // MSB = 127 (max 7-bit)
        match result {
            Some(CcParam::Cutoff(v)) => assert_eq!(v.value(), 1023),
            other => panic!("expected Cutoff(1023), got {other:?}"),
        }
    }

    #[test]
    fn receiver_high_res_min_value() {
        let mut rx = CcParamReceiver::new();
        rx.feed(&cc(63, 0));
        let result = rx.feed(&cc(43, 0));
        match result {
            Some(CcParam::Cutoff(v)) => assert_eq!(v.value(), 0),
            other => panic!("expected Cutoff(0), got {other:?}"),
        }
    }

    // === CcParamReceiver: all high-res CCs ===

    #[test]
    fn receiver_all_high_res_cc_numbers() {
        let high_res_ccs: &[(u8, &str)] = &[
            (16, "AmpEgAttack"),
            (17, "AmpEgDecay"),
            (18, "AmpEgSustain"),
            (19, "AmpEgRelease"),
            (20, "EgAttack"),
            (21, "EgDecay"),
            (22, "EgInt"),
            (24, "LfoRate"),
            (26, "LfoInt"),
            (27, "VoiceModeDepth"),
            (28, "ModFxTime"),
            (29, "ModFxDepth"),
            (33, "MultiLevel"),
            (34, "Vco1Pitch"),
            (35, "Vco2Pitch"),
            (36, "Vco1Shape"),
            (37, "Vco2Shape"),
            (39, "Vco1Level"),
            (40, "Vco2Level"),
            (41, "CrossModDepth"),
            (43, "Cutoff"),
            (44, "Resonance"),
            (54, "MultiShape"),
            (59, "VoiceModeDepthAlt"),
            (104, "MultiShiftShape"),
            (105, "DelayTime"),
            (106, "DelayDepth"),
            (107, "DelayDryWet"),
            (108, "ReverbTime"),
            (109, "ReverbDepth"),
            (110, "ReverbDryWet"),
        ];

        for &(cc_num, name) in high_res_ccs {
            let mut rx = CcParamReceiver::new();
            rx.feed(&cc(63, 3));
            let result = rx.feed(&cc(cc_num, 10));
            assert!(
                result.is_some(),
                "CC {cc_num} ({name}) should produce a CcParam"
            );
            let param = result.unwrap();
            assert_eq!(param.cc_number(), cc_num, "CC number mismatch for {name}");
        }
    }

    // === CcParamReceiver: stepped enums ===

    #[test]
    fn receiver_eg_target() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(23, 0));
        assert!(matches!(result, Some(CcParam::EgTarget(EgTarget::Cutoff))));
    }

    #[test]
    fn receiver_vco1_octave() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(48, 42));
        assert!(matches!(
            result,
            Some(CcParam::Vco1Octave(VcoOctave::Eight))
        ));
    }

    #[test]
    fn receiver_vco2_octave() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(49, 127));
        assert!(matches!(result, Some(CcParam::Vco2Octave(VcoOctave::Two))));
    }

    #[test]
    fn receiver_vco1_wave() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(50, 64));
        assert!(matches!(result, Some(CcParam::Vco1Wave(VcoWave::Tri))));
    }

    #[test]
    fn receiver_vco2_wave() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(51, 127));
        assert!(matches!(result, Some(CcParam::Vco2Wave(VcoWave::Saw))));
    }

    #[test]
    fn receiver_multi_type() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(53, 64));
        assert!(matches!(result, Some(CcParam::MultiType(MultiType::Vpm))));
    }

    #[test]
    fn receiver_lfo_target() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(56, 127));
        assert!(matches!(result, Some(CcParam::LfoTarget(LfoTarget::Pitch))));
    }

    #[test]
    fn receiver_lfo_wave() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(57, 0));
        assert!(matches!(result, Some(CcParam::LfoWave(LfoWave::Sqr))));
    }

    #[test]
    fn receiver_lfo_mode() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(58, 64));
        assert!(matches!(result, Some(CcParam::LfoMode(LfoMode::Normal))));
    }

    #[test]
    fn receiver_sync() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(80, 127));
        assert!(matches!(result, Some(CcParam::Sync(Sync::On))));
    }

    #[test]
    fn receiver_ring() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(81, 0));
        assert!(matches!(result, Some(CcParam::Ring(Ring::Off))));
    }

    #[test]
    fn receiver_cutoff_keytrack() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(83, 64));
        assert!(matches!(
            result,
            Some(CcParam::CutoffKeytrack(CutoffKeytrack::Half))
        ));
    }

    #[test]
    fn receiver_cutoff_drive() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(84, 127));
        assert!(matches!(
            result,
            Some(CcParam::CutoffDrive(CutoffDrive::Full))
        ));
    }

    #[test]
    fn receiver_mod_fx_type() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(88, 64));
        assert!(matches!(
            result,
            Some(CcParam::ModFxType(ModFxType::Phaser))
        ));
    }

    #[test]
    fn receiver_delay_sub_type() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(89, 0));
        assert!(matches!(
            result,
            Some(CcParam::DelaySubType(DelaySubType::Stereo))
        ));
    }

    #[test]
    fn receiver_reverb_sub_type() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(90, 0));
        assert!(matches!(
            result,
            Some(CcParam::ReverbSubType(ReverbSubType::Hall))
        ));
    }

    // === CcParamReceiver: on/off ===

    #[test]
    fn receiver_mod_fx_on_off_on() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(92, 127));
        assert!(matches!(result, Some(CcParam::ModFxOnOff(true))));
    }

    #[test]
    fn receiver_mod_fx_on_off_off() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(92, 0));
        assert!(matches!(result, Some(CcParam::ModFxOnOff(false))));
    }

    #[test]
    fn receiver_mod_fx_on_off_threshold() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(
            rx.feed(&cc(92, 64)),
            Some(CcParam::ModFxOnOff(true))
        ));
        assert!(matches!(
            rx.feed(&cc(92, 63)),
            Some(CcParam::ModFxOnOff(false))
        ));
    }

    #[test]
    fn receiver_delay_on_off() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(
            rx.feed(&cc(93, 127)),
            Some(CcParam::DelayOnOff(true))
        ));
        assert!(matches!(
            rx.feed(&cc(93, 0)),
            Some(CcParam::DelayOnOff(false))
        ));
    }

    #[test]
    fn receiver_reverb_on_off() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(
            rx.feed(&cc(94, 64)),
            Some(CcParam::ReverbOnOff(true))
        ));
        assert!(matches!(
            rx.feed(&cc(94, 63)),
            Some(CcParam::ReverbOnOff(false))
        ));
    }

    // === CcParamReceiver: simple / special / context-dependent ===

    #[test]
    fn receiver_simple_continuous() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(rx.feed(&cc(1, 64)), Some(CcParam::Modulation1(_))));
        assert!(matches!(rx.feed(&cc(2, 32)), Some(CcParam::Modulation2(_))));
        assert!(matches!(
            rx.feed(&cc(5, 100)),
            Some(CcParam::PortamentoTime(_))
        ));
        assert!(matches!(rx.feed(&cc(118, 50)), Some(CcParam::CvIn1(_))));
        assert!(matches!(rx.feed(&cc(119, 50)), Some(CcParam::CvIn2(_))));
    }

    #[test]
    fn receiver_special() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(
            rx.feed(&cc(0, 0)),
            Some(CcParam::BankSelectMsb(_))
        ));
        assert!(matches!(
            rx.feed(&cc(32, 2)),
            Some(CcParam::BankSelectLsb(_))
        ));
        assert!(matches!(rx.feed(&cc(64, 127)), Some(CcParam::Damper(_))));
    }

    #[test]
    fn receiver_nrpn_addressing() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(
            rx.feed(&cc(6, 42)),
            Some(CcParam::DataEntryMsb(_))
        ));
        assert!(matches!(rx.feed(&cc(98, 12)), Some(CcParam::NrpnLsb(_))));
        assert!(matches!(rx.feed(&cc(99, 0)), Some(CcParam::NrpnMsb(_))));
    }

    #[test]
    fn receiver_context_dependent() {
        let mut rx = CcParamReceiver::new();
        assert!(matches!(
            rx.feed(&cc(96, 42)),
            Some(CcParam::ModFxSubType(_))
        ));
        assert!(matches!(
            rx.feed(&cc(103, 8)),
            Some(CcParam::MultiSelect(_))
        ));
    }

    #[test]
    fn receiver_cc63_returns_data_entry_lsb() {
        let mut rx = CcParamReceiver::new();
        let result = rx.feed(&cc(63, 5));
        assert!(matches!(result, Some(CcParam::DataEntryLsb(_))));
        if let Some(CcParam::DataEntryLsb(v)) = result {
            assert_eq!(v.value(), 5);
        }
    }

    // === Unknown CC ===

    #[test]
    fn receiver_unknown_cc_returns_none() {
        let mut rx = CcParamReceiver::new();
        assert!(rx.feed(&cc(3, 0)).is_none());
        assert!(rx.feed(&cc(4, 0)).is_none());
        assert!(rx.feed(&cc(7, 0)).is_none());
        assert!(rx.feed(&cc(120, 0)).is_none());
        assert!(rx.feed(&cc(127, 0)).is_none());
    }

    #[test]
    fn receiver_unknown_cc_resets_pending_lsb() {
        let mut rx = CcParamReceiver::new();
        rx.feed(&cc(63, 7)); // Buffer LSB
        rx.feed(&cc(3, 0)); // Unknown CC resets LSB
        let result = rx.feed(&cc(43, 10)); // Cutoff should use LSB = 0
        match result {
            Some(CcParam::Cutoff(v)) => assert_eq!(v.value(), 10 << 3),
            other => panic!("expected Cutoff, got {other:?}"),
        }
    }

    // === to_cc_messages round-trip ===

    #[test]
    fn roundtrip_high_res() {
        let channel = U4::new(0).unwrap();
        let original = CcParam::Cutoff(TenBitParam::new(517).unwrap());
        let messages = original.to_cc_messages(channel).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].controller.value(), 63);
        assert_eq!(messages[1].controller.value(), 43);

        // Feed through receiver
        let mut rx = CcParamReceiver::new();
        rx.feed(&messages[0]);
        let recovered = rx.feed(&messages[1]);
        assert_eq!(recovered, Some(original));
    }

    #[test]
    fn roundtrip_all_high_res_boundary_values() {
        let channel = U4::new(0).unwrap();
        for value in [0u16, 1, 7, 8, 511, 512, 1023] {
            let original = CcParam::Cutoff(TenBitParam::new(value).unwrap());
            let messages = original.to_cc_messages(channel).unwrap();
            let mut rx = CcParamReceiver::new();
            rx.feed(&messages[0]);
            let recovered = rx.feed(&messages[1]);
            assert_eq!(
                recovered,
                Some(original),
                "roundtrip failed for value {value}"
            );
        }
    }

    #[test]
    fn roundtrip_stepped_eg_target() {
        let channel = U4::new(3).unwrap();
        for variant in [EgTarget::Cutoff, EgTarget::Pitch2, EgTarget::Pitch] {
            let original = CcParam::EgTarget(variant);
            let messages = original.to_cc_messages(channel).unwrap();
            assert_eq!(messages.len(), 1);
            let mut rx = CcParamReceiver::new();
            let recovered = rx.feed(&messages[0]);
            assert_eq!(
                recovered,
                Some(original),
                "roundtrip failed for {variant:?}"
            );
        }
    }

    #[test]
    fn roundtrip_stepped_vco_octave() {
        let channel = U4::new(0).unwrap();
        for variant in [
            VcoOctave::Sixteen,
            VcoOctave::Eight,
            VcoOctave::Four,
            VcoOctave::Two,
        ] {
            let original = CcParam::Vco1Octave(variant);
            let messages = original.to_cc_messages(channel).unwrap();
            let mut rx = CcParamReceiver::new();
            let recovered = rx.feed(&messages[0]);
            assert_eq!(
                recovered,
                Some(original),
                "roundtrip failed for {variant:?}"
            );
        }
    }

    #[test]
    fn roundtrip_on_off() {
        let channel = U4::new(0).unwrap();
        for on in [true, false] {
            let original = CcParam::ModFxOnOff(on);
            let messages = original.to_cc_messages(channel).unwrap();
            assert_eq!(messages.len(), 1);
            let mut rx = CcParamReceiver::new();
            let recovered = rx.feed(&messages[0]);
            assert_eq!(recovered, Some(original), "roundtrip failed for on={on}");
        }
    }

    #[test]
    fn roundtrip_simple_continuous() {
        let channel = U4::new(0).unwrap();
        let original = CcParam::Modulation1(U7::new(100).unwrap());
        let messages = original.to_cc_messages(channel).unwrap();
        assert_eq!(messages.len(), 1);
        let mut rx = CcParamReceiver::new();
        let recovered = rx.feed(&messages[0]);
        assert_eq!(recovered, Some(original));
    }

    #[test]
    fn roundtrip_context_dependent() {
        let channel = U4::new(0).unwrap();
        let original = CcParam::ModFxSubType(U7::new(42).unwrap());
        let messages = original.to_cc_messages(channel).unwrap();
        let mut rx = CcParamReceiver::new();
        let recovered = rx.feed(&messages[0]);
        assert_eq!(recovered, Some(original));
    }

    #[test]
    fn to_cc_messages_channel_preserved() {
        let channel = U4::new(7).unwrap();
        let param = CcParam::Cutoff(TenBitParam::new(100).unwrap());
        let messages = param.to_cc_messages(channel).unwrap();
        for msg in &messages {
            assert_eq!(msg.channel.value(), 7);
        }
    }

    // === Specific TX values for stepped params ===

    #[test]
    fn stepped_tx_values() {
        let channel = U4::new(0).unwrap();

        // Sync On should send 127
        let msgs = CcParam::Sync(Sync::On).to_cc_messages(channel).unwrap();
        assert_eq!(msgs[0].value.value(), 127);

        // Sync Off should send 0
        let msgs = CcParam::Sync(Sync::Off).to_cc_messages(channel).unwrap();
        assert_eq!(msgs[0].value.value(), 0);

        // VcoOctave Eight should send 42
        let msgs = CcParam::Vco1Octave(VcoOctave::Eight)
            .to_cc_messages(channel)
            .unwrap();
        assert_eq!(msgs[0].value.value(), 42);
    }

    // === Default receiver ===

    #[test]
    fn receiver_default() {
        let rx = CcParamReceiver::default();
        assert!(format!("{rx:?}").contains("None"));
    }

    // === CC63 LSB masking ===

    #[test]
    fn receiver_cc63_masks_to_3_bits() {
        let mut rx = CcParamReceiver::new();
        // CC63 value 0xFF would be invalid for U7, but 0x7F (127) is valid.
        // Only bottom 3 bits should be used: 127 & 7 = 7
        rx.feed(&cc(63, 127));
        let result = rx.feed(&cc(43, 0)); // Cutoff
        match result {
            Some(CcParam::Cutoff(v)) => assert_eq!(v.value(), 7),
            other => panic!("expected Cutoff(7), got {other:?}"),
        }
    }
}
