//! NRPN parameter map for the Minilogue XD.
//!
//! Maps every NRPN address from the MIDI Implementation chart (section \*3)
//! to a strongly typed [`NrpnParam`] variant. The [`NrpnReceiver`] implements
//! a 4-state FSM to reassemble NRPN messages from individual CC messages.

use crate::error::Result;
use crate::message::channel::ControlChange;
use crate::message::types::{U4, U7};
use crate::param::encoding::{EightBitHighRes, FourteenBitParam, TenBitSysex};
use crate::param::enums::{
    CvInMode, LfoTargetOsc, MicroTuning, ModAssignTarget, MultiRouting, MultiSelectNoise,
    MultiSelectUser, MultiSelectVpm, PortamentoMode, VcoOctave, VoiceModeType,
};
use crate::param::SteppedParam;

// ---------------------------------------------------------------------------
// NrpnParam
// ---------------------------------------------------------------------------

/// A typed NRPN parameter from the Minilogue XD MIDI implementation.
///
/// Each variant carries the appropriate value type for its parameter.
/// NRPN addresses use MSB=0 for all standard parameters, except
/// `MasterVolume` which uses MSB=1 when PolyChain is set to Master.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum NrpnParam {
    /// NRPN (0, 0x00..0x0B): Program name character.
    /// First field is the character index (0-11), second is the ASCII value.
    ProgramName(u8, U7),
    /// NRPN (0, 0x0C): Voice mode type.
    VoiceModeType(VoiceModeType),
    /// NRPN (0, 0x0D): Multi-engine noise type selector.
    MultiSelectNoise(MultiSelectNoise),
    /// NRPN (0, 0x0E): Multi-engine VPM type selector.
    MultiSelectVpm(MultiSelectVpm),
    /// NRPN (0, 0x0F): Multi-engine user oscillator selector.
    MultiSelectUser(MultiSelectUser),
    /// NRPN (0, 0x10): Multi-engine shape for Noise.
    MultiShapeNoise(TenBitSysex),
    /// NRPN (0, 0x11): Multi-engine shape for VPM.
    MultiShapeVpm(TenBitSysex),
    /// NRPN (0, 0x12): Multi-engine shape for User.
    MultiShapeUser(TenBitSysex),
    /// NRPN (0, 0x13): Multi-engine shift-shape for Noise.
    MultiShiftShapeNoise(TenBitSysex),
    /// NRPN (0, 0x14): Multi-engine shift-shape for VPM.
    MultiShiftShapeVpm(TenBitSysex),
    /// NRPN (0, 0x15): Multi-engine shift-shape for User.
    MultiShiftShapeUser(TenBitSysex),
    /// NRPN (0, 0x16): Pitch bend range positive (0-12 semitones).
    BendRangePlus(u8),
    /// NRPN (0, 0x17): Pitch bend range negative (0-12 semitones).
    BendRangeMinus(u8),
    /// NRPN (0, 0x18): Joystick assign positive.
    JoystickAssignPlus(ModAssignTarget),
    /// NRPN (0, 0x19): Joystick range positive (0-200).
    JoystickRangePlus(EightBitHighRes),
    /// NRPN (0, 0x1A): Joystick assign negative.
    JoystickAssignMinus(ModAssignTarget),
    /// NRPN (0, 0x1B): Joystick range negative (0-200).
    JoystickRangeMinus(EightBitHighRes),
    /// NRPN (0, 0x1C): CV input mode.
    CvInMode(CvInMode),
    /// NRPN (0, 0x1D): CV input 1 assign target.
    CvIn1Assign(ModAssignTarget),
    /// NRPN (0, 0x1E): CV input 1 range (0-200).
    CvIn1Range(EightBitHighRes),
    /// NRPN (0, 0x1F): CV input 2 assign target.
    CvIn2Assign(ModAssignTarget),
    /// NRPN (0, 0x20): CV input 2 range (0-200).
    CvIn2Range(EightBitHighRes),
    /// NRPN (0, 0x30): Micro-tuning scale.
    MicroTuning(MicroTuning),
    /// NRPN (0, 0x31): Scale key (0-11).
    ScaleKey(u8),
    /// NRPN (0, 0x32): Program tuning (0-100, center=50).
    ProgramTuning(u8),
    /// NRPN (0, 0x33): LFO key sync on/off.
    LfoKeySync(bool),
    /// NRPN (0, 0x34): LFO voice sync on/off.
    LfoVoiceSync(bool),
    /// NRPN (0, 0x35): LFO target oscillator.
    LfoTargetOsc(LfoTargetOsc),
    /// NRPN (0, 0x36): Cutoff velocity sensitivity.
    CutoffVelocity(U7),
    /// NRPN (0, 0x37): Amp velocity sensitivity.
    AmpVelocity(U7),
    /// NRPN (0, 0x38): Multi-engine octave.
    MultiOctave(VcoOctave),
    /// NRPN (0, 0x39): Multi-engine audio routing.
    MultiRouting(MultiRouting),
    /// NRPN (0, 0x3A): EG legato on/off.
    EgLegato(bool),
    /// NRPN (0, 0x3B): Portamento mode.
    PortamentoMode(PortamentoMode),
    /// NRPN (0, 0x3C): Portamento BPM sync on/off.
    PortamentoBpmSync(bool),
    /// NRPN (0, 0x3F): Program level (0-120 = -18dB..+6dB).
    ProgramLevel(u8),
    /// NRPN (0, 0x40): VPM parameter 1 (0-200).
    VpmParam1(EightBitHighRes),
    /// NRPN (0, 0x41): VPM parameter 2 (0-200).
    VpmParam2(EightBitHighRes),
    /// NRPN (0, 0x42): VPM parameter 3 (0-200).
    VpmParam3(EightBitHighRes),
    /// NRPN (0, 0x43): VPM parameter 4 (0-200).
    VpmParam4(EightBitHighRes),
    /// NRPN (0, 0x44): VPM parameter 5 (0-200).
    VpmParam5(EightBitHighRes),
    /// NRPN (0, 0x45): VPM parameter 6 (0-200).
    VpmParam6(EightBitHighRes),
    /// NRPN (0, 0x48): User parameter 1 (0-200).
    UserParam1(EightBitHighRes),
    /// NRPN (0, 0x49): User parameter 2 (0-200).
    UserParam2(EightBitHighRes),
    /// NRPN (0, 0x4A): User parameter 3 (0-200).
    UserParam3(EightBitHighRes),
    /// NRPN (0, 0x4B): User parameter 4 (0-200).
    UserParam4(EightBitHighRes),
    /// NRPN (0, 0x4C): User parameter 5 (0-200).
    UserParam5(EightBitHighRes),
    /// NRPN (0, 0x4D): User parameter 6 (0-200).
    UserParam6(EightBitHighRes),
    /// NRPN (0, 0x50): Program transpose (1-25 = -12..+12).
    ProgramTranspose(u8),
    /// NRPN (0, 0x51): MIDI after-touch assign target.
    MidiAfterTouchAssign(ModAssignTarget),
    /// NRPN (0, 0x7F): Master volume (14-bit, poly-chain master).
    MasterVolume(FourteenBitParam),
}

impl NrpnParam {
    /// Returns the NRPN address as `(MSB, LSB)`.
    pub fn address(&self) -> (u8, u8) {
        match self {
            Self::ProgramName(idx, _) => (0, *idx),
            Self::VoiceModeType(_) => (0, 0x0C),
            Self::MultiSelectNoise(_) => (0, 0x0D),
            Self::MultiSelectVpm(_) => (0, 0x0E),
            Self::MultiSelectUser(_) => (0, 0x0F),
            Self::MultiShapeNoise(_) => (0, 0x10),
            Self::MultiShapeVpm(_) => (0, 0x11),
            Self::MultiShapeUser(_) => (0, 0x12),
            Self::MultiShiftShapeNoise(_) => (0, 0x13),
            Self::MultiShiftShapeVpm(_) => (0, 0x14),
            Self::MultiShiftShapeUser(_) => (0, 0x15),
            Self::BendRangePlus(_) => (0, 0x16),
            Self::BendRangeMinus(_) => (0, 0x17),
            Self::JoystickAssignPlus(_) => (0, 0x18),
            Self::JoystickRangePlus(_) => (0, 0x19),
            Self::JoystickAssignMinus(_) => (0, 0x1A),
            Self::JoystickRangeMinus(_) => (0, 0x1B),
            Self::CvInMode(_) => (0, 0x1C),
            Self::CvIn1Assign(_) => (0, 0x1D),
            Self::CvIn1Range(_) => (0, 0x1E),
            Self::CvIn2Assign(_) => (0, 0x1F),
            Self::CvIn2Range(_) => (0, 0x20),
            Self::MicroTuning(_) => (0, 0x30),
            Self::ScaleKey(_) => (0, 0x31),
            Self::ProgramTuning(_) => (0, 0x32),
            Self::LfoKeySync(_) => (0, 0x33),
            Self::LfoVoiceSync(_) => (0, 0x34),
            Self::LfoTargetOsc(_) => (0, 0x35),
            Self::CutoffVelocity(_) => (0, 0x36),
            Self::AmpVelocity(_) => (0, 0x37),
            Self::MultiOctave(_) => (0, 0x38),
            Self::MultiRouting(_) => (0, 0x39),
            Self::EgLegato(_) => (0, 0x3A),
            Self::PortamentoMode(_) => (0, 0x3B),
            Self::PortamentoBpmSync(_) => (0, 0x3C),
            Self::ProgramLevel(_) => (0, 0x3F),
            Self::VpmParam1(_) => (0, 0x40),
            Self::VpmParam2(_) => (0, 0x41),
            Self::VpmParam3(_) => (0, 0x42),
            Self::VpmParam4(_) => (0, 0x43),
            Self::VpmParam5(_) => (0, 0x44),
            Self::VpmParam6(_) => (0, 0x45),
            Self::UserParam1(_) => (0, 0x48),
            Self::UserParam2(_) => (0, 0x49),
            Self::UserParam3(_) => (0, 0x4A),
            Self::UserParam4(_) => (0, 0x4B),
            Self::UserParam5(_) => (0, 0x4C),
            Self::UserParam6(_) => (0, 0x4D),
            Self::ProgramTranspose(_) => (0, 0x50),
            Self::MidiAfterTouchAssign(_) => (0, 0x51),
            Self::MasterVolume(_) => (0, 0x7F),
        }
    }

    /// Encodes this NRPN parameter as a sequence of CC messages for transmission.
    ///
    /// The sequence is: `CC99(addr_msb), CC98(addr_lsb), [CC63(data_lsb)], CC6(data_msb)`.
    /// Parameters with multi-byte data (10-bit, 8-bit high-res, 14-bit) include
    /// the CC63 data LSB message; single-byte parameters omit it.
    ///
    /// # Errors
    ///
    /// Returns an error if any U7 value construction fails.
    pub fn to_midi_sequence(&self, channel: U4) -> Result<Vec<ControlChange>> {
        let (addr_msb, addr_lsb) = self.address();
        let mut msgs = vec![
            ControlChange {
                channel,
                controller: U7::new(99)?,
                value: U7::new(addr_msb)?,
            },
            ControlChange {
                channel,
                controller: U7::new(98)?,
                value: U7::new(addr_lsb)?,
            },
        ];

        match self {
            // Multi-byte: TenBitSysex (CC63 + CC6)
            Self::MultiShapeNoise(v)
            | Self::MultiShapeVpm(v)
            | Self::MultiShapeUser(v)
            | Self::MultiShiftShapeNoise(v)
            | Self::MultiShiftShapeVpm(v)
            | Self::MultiShiftShapeUser(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(63)?,
                    value: U7::new(v.lsb())?,
                });
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.msb())?,
                });
            }

            // Multi-byte: EightBitHighRes (CC63 + CC6)
            Self::JoystickRangePlus(v)
            | Self::JoystickRangeMinus(v)
            | Self::CvIn1Range(v)
            | Self::CvIn2Range(v)
            | Self::VpmParam1(v)
            | Self::VpmParam2(v)
            | Self::VpmParam3(v)
            | Self::VpmParam4(v)
            | Self::VpmParam5(v)
            | Self::VpmParam6(v)
            | Self::UserParam1(v)
            | Self::UserParam2(v)
            | Self::UserParam3(v)
            | Self::UserParam4(v)
            | Self::UserParam5(v)
            | Self::UserParam6(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(63)?,
                    value: U7::new(v.lsb())?,
                });
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.msb())?,
                });
            }

            // Multi-byte: FourteenBitParam (CC63 + CC6)
            Self::MasterVolume(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(63)?,
                    value: U7::new(v.lsb())?,
                });
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.msb())?,
                });
            }

            // Single-byte: enum program values
            Self::VoiceModeType(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::MultiSelectNoise(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::MultiSelectVpm(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::MultiSelectUser(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::JoystickAssignPlus(v)
            | Self::JoystickAssignMinus(v)
            | Self::CvIn1Assign(v)
            | Self::CvIn2Assign(v)
            | Self::MidiAfterTouchAssign(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::CvInMode(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::MicroTuning(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::LfoTargetOsc(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::MultiOctave(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::MultiRouting(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }
            Self::PortamentoMode(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(v.to_program_value())?,
                });
            }

            // Single-byte: bool (0=off, 1=on)
            Self::LfoKeySync(on)
            | Self::LfoVoiceSync(on)
            | Self::EgLegato(on)
            | Self::PortamentoBpmSync(on) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(u8::from(*on))?,
                });
            }

            // Single-byte: raw values
            Self::ProgramName(_, v) | Self::CutoffVelocity(v) | Self::AmpVelocity(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: *v,
                });
            }
            Self::BendRangePlus(v)
            | Self::BendRangeMinus(v)
            | Self::ScaleKey(v)
            | Self::ProgramTuning(v)
            | Self::ProgramLevel(v)
            | Self::ProgramTranspose(v) => {
                msgs.push(ControlChange {
                    channel,
                    controller: U7::new(6)?,
                    value: U7::new(*v)?,
                });
            }
        }

        Ok(msgs)
    }
}

// ---------------------------------------------------------------------------
// NrpnReceiver
// ---------------------------------------------------------------------------

/// Internal FSM state for the NRPN receiver.
#[derive(Debug, Default)]
enum NrpnState {
    /// Waiting for CC99 (NRPN MSB).
    #[default]
    Idle,
    /// Received CC99; waiting for CC98 (NRPN LSB).
    HaveMsb(u8),
    /// Received CC99+CC98; waiting for CC63 or CC6.
    Addressed { msb: u8, lsb: u8 },
    /// Received CC99+CC98+CC63; waiting for CC6.
    HaveDataLsb { msb: u8, lsb: u8, data_lsb: u8 },
}

/// Stateful receiver that reassembles NRPN parameter messages from
/// individual [`ControlChange`] messages.
///
/// The NRPN protocol requires 3-4 CC messages in sequence:
/// 1. CC 99 — NRPN address MSB
/// 2. CC 98 — NRPN address LSB
/// 3. CC 63 — Data LSB (optional, for multi-byte params)
/// 4. CC 6  — Data MSB (completes the message)
#[derive(Debug, Default)]
pub struct NrpnReceiver {
    state: NrpnState,
}

impl NrpnReceiver {
    /// Creates a new receiver in the idle state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes an incoming [`ControlChange`] and returns an [`NrpnParam`]
    /// if a complete NRPN message has been assembled.
    ///
    /// State transitions:
    /// - CC 99 → `HaveMsb`
    /// - CC 98 (when `HaveMsb`) → `Addressed`
    /// - CC 63 (when `Addressed`) → `HaveDataLsb`
    /// - CC 6  (when `Addressed`) → resolve with no data LSB
    /// - CC 6  (when `HaveDataLsb`) → resolve with data LSB
    /// - Anything else → `Idle`
    pub fn feed(&mut self, cc: &ControlChange) -> Option<NrpnParam> {
        let cc_num = cc.controller.value();
        let val = cc.value.value();

        match cc_num {
            // CC 99: NRPN MSB — always transitions to HaveMsb
            99 => {
                self.state = NrpnState::HaveMsb(val);
                None
            }
            // CC 98: NRPN LSB — only valid after CC99
            98 => {
                if let NrpnState::HaveMsb(msb) = self.state {
                    self.state = NrpnState::Addressed { msb, lsb: val };
                } else {
                    self.state = NrpnState::Idle;
                }
                None
            }
            // CC 63: Data LSB — only valid when addressed
            63 => {
                if let NrpnState::Addressed { msb, lsb } = self.state {
                    self.state = NrpnState::HaveDataLsb {
                        msb,
                        lsb,
                        data_lsb: val,
                    };
                } else {
                    self.state = NrpnState::Idle;
                }
                None
            }
            // CC 6: Data MSB — resolves if addressed (with or without data LSB)
            6 => {
                let result = match self.state {
                    NrpnState::Addressed { msb, lsb } => Self::resolve(msb, lsb, None, val),
                    NrpnState::HaveDataLsb { msb, lsb, data_lsb } => {
                        Self::resolve(msb, lsb, Some(data_lsb), val)
                    }
                    _ => None,
                };
                self.state = NrpnState::Idle;
                result
            }
            // Anything else: reset
            _ => {
                self.state = NrpnState::Idle;
                None
            }
        }
    }

    /// Resolves an NRPN address + data into a typed parameter.
    fn resolve(
        addr_msb: u8,
        addr_lsb: u8,
        data_lsb: Option<u8>,
        data_msb: u8,
    ) -> Option<NrpnParam> {
        match (addr_msb, addr_lsb) {
            // Program name characters (0, 0x00..0x0B)
            (0, idx @ 0x00..=0x0B) => U7::new(data_msb)
                .ok()
                .map(|v| NrpnParam::ProgramName(idx, v)),

            // Enum-valued (program values)
            (0, 0x0C) => VoiceModeType::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::VoiceModeType),
            (0, 0x0D) => MultiSelectNoise::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MultiSelectNoise),
            (0, 0x0E) => MultiSelectVpm::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MultiSelectVpm),
            (0, 0x0F) => MultiSelectUser::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MultiSelectUser),

            // TenBitSysex (multi shape / shift shape)
            (0, 0x10) => ten_bit_sysex_from(data_lsb, data_msb).map(NrpnParam::MultiShapeNoise),
            (0, 0x11) => ten_bit_sysex_from(data_lsb, data_msb).map(NrpnParam::MultiShapeVpm),
            (0, 0x12) => ten_bit_sysex_from(data_lsb, data_msb).map(NrpnParam::MultiShapeUser),
            (0, 0x13) => {
                ten_bit_sysex_from(data_lsb, data_msb).map(NrpnParam::MultiShiftShapeNoise)
            }
            (0, 0x14) => ten_bit_sysex_from(data_lsb, data_msb).map(NrpnParam::MultiShiftShapeVpm),
            (0, 0x15) => ten_bit_sysex_from(data_lsb, data_msb).map(NrpnParam::MultiShiftShapeUser),

            // Bend range
            (0, 0x16) => Some(NrpnParam::BendRangePlus(data_msb)),
            (0, 0x17) => Some(NrpnParam::BendRangeMinus(data_msb)),

            // Joystick assign/range
            (0, 0x18) => ModAssignTarget::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::JoystickAssignPlus),
            (0, 0x19) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::JoystickRangePlus),
            (0, 0x1A) => ModAssignTarget::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::JoystickAssignMinus),
            (0, 0x1B) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::JoystickRangeMinus),

            // CV input
            (0, 0x1C) => CvInMode::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::CvInMode),
            (0, 0x1D) => ModAssignTarget::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::CvIn1Assign),
            (0, 0x1E) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::CvIn1Range),
            (0, 0x1F) => ModAssignTarget::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::CvIn2Assign),
            (0, 0x20) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::CvIn2Range),

            // Tuning / scale
            (0, 0x30) => MicroTuning::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MicroTuning),
            (0, 0x31) => Some(NrpnParam::ScaleKey(data_msb)),
            (0, 0x32) => Some(NrpnParam::ProgramTuning(data_msb)),

            // LFO sync / target osc
            (0, 0x33) => Some(NrpnParam::LfoKeySync(data_msb != 0)),
            (0, 0x34) => Some(NrpnParam::LfoVoiceSync(data_msb != 0)),
            (0, 0x35) => LfoTargetOsc::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::LfoTargetOsc),

            // Velocity
            (0, 0x36) => U7::new(data_msb).ok().map(NrpnParam::CutoffVelocity),
            (0, 0x37) => U7::new(data_msb).ok().map(NrpnParam::AmpVelocity),

            // Multi octave / routing
            (0, 0x38) => VcoOctave::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MultiOctave),
            (0, 0x39) => MultiRouting::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MultiRouting),

            // EG legato, portamento
            (0, 0x3A) => Some(NrpnParam::EgLegato(data_msb != 0)),
            (0, 0x3B) => PortamentoMode::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::PortamentoMode),
            (0, 0x3C) => Some(NrpnParam::PortamentoBpmSync(data_msb != 0)),

            // Program level
            (0, 0x3F) => Some(NrpnParam::ProgramLevel(data_msb)),

            // VPM params
            (0, 0x40) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::VpmParam1),
            (0, 0x41) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::VpmParam2),
            (0, 0x42) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::VpmParam3),
            (0, 0x43) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::VpmParam4),
            (0, 0x44) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::VpmParam5),
            (0, 0x45) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::VpmParam6),

            // User params
            (0, 0x48) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::UserParam1),
            (0, 0x49) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::UserParam2),
            (0, 0x4A) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::UserParam3),
            (0, 0x4B) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::UserParam4),
            (0, 0x4C) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::UserParam5),
            (0, 0x4D) => eight_bit_from(data_lsb, data_msb).map(NrpnParam::UserParam6),

            // Program transpose
            (0, 0x50) => Some(NrpnParam::ProgramTranspose(data_msb)),

            // MIDI aftertouch assign
            (0, 0x51) => ModAssignTarget::from_program_value(data_msb)
                .ok()
                .map(NrpnParam::MidiAfterTouchAssign),

            // Master volume (14-bit)
            (0, 0x7F) => fourteen_bit_from(data_lsb, data_msb).map(NrpnParam::MasterVolume),

            // Unknown address
            _ => None,
        }
    }
}

/// Helper: construct a `TenBitSysex` from optional LSB and MSB.
fn ten_bit_sysex_from(data_lsb: Option<u8>, data_msb: u8) -> Option<TenBitSysex> {
    TenBitSysex::from_parts(data_lsb.unwrap_or(0), data_msb).ok()
}

/// Helper: construct an `EightBitHighRes` from optional LSB and MSB.
fn eight_bit_from(data_lsb: Option<u8>, data_msb: u8) -> Option<EightBitHighRes> {
    EightBitHighRes::from_parts(data_lsb.unwrap_or(0), data_msb).ok()
}

/// Helper: construct a `FourteenBitParam` from optional LSB and MSB.
fn fourteen_bit_from(data_lsb: Option<u8>, data_msb: u8) -> Option<FourteenBitParam> {
    FourteenBitParam::from_parts(data_lsb.unwrap_or(0), data_msb).ok()
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

    /// Helper: feed a full NRPN sequence (CC99, CC98, [CC63], CC6) to a receiver.
    fn feed_nrpn(
        rx: &mut NrpnReceiver,
        addr_msb: u8,
        addr_lsb: u8,
        data_lsb: Option<u8>,
        data_msb: u8,
    ) -> Option<NrpnParam> {
        rx.feed(&cc(99, addr_msb));
        rx.feed(&cc(98, addr_lsb));
        if let Some(lsb) = data_lsb {
            rx.feed(&cc(63, lsb));
        }
        rx.feed(&cc(6, data_msb))
    }

    // === Address mapping ===

    #[test]
    fn address_program_name() {
        for idx in 0..12u8 {
            let p = NrpnParam::ProgramName(idx, U7::new(65).unwrap());
            assert_eq!(p.address(), (0, idx));
        }
    }

    #[test]
    fn address_voice_mode_type() {
        let p = NrpnParam::VoiceModeType(VoiceModeType::Poly);
        assert_eq!(p.address(), (0, 0x0C));
    }

    #[test]
    fn address_multi_select_noise() {
        let p = NrpnParam::MultiSelectNoise(MultiSelectNoise::High);
        assert_eq!(p.address(), (0, 0x0D));
    }

    #[test]
    fn address_multi_select_vpm() {
        let p = NrpnParam::MultiSelectVpm(MultiSelectVpm::Sin1);
        assert_eq!(p.address(), (0, 0x0E));
    }

    #[test]
    fn address_multi_select_user() {
        let p = NrpnParam::MultiSelectUser(MultiSelectUser::User1);
        assert_eq!(p.address(), (0, 0x0F));
    }

    #[test]
    fn address_multi_shapes() {
        assert_eq!(
            NrpnParam::MultiShapeNoise(TenBitSysex::new(0).unwrap()).address(),
            (0, 0x10)
        );
        assert_eq!(
            NrpnParam::MultiShapeVpm(TenBitSysex::new(0).unwrap()).address(),
            (0, 0x11)
        );
        assert_eq!(
            NrpnParam::MultiShapeUser(TenBitSysex::new(0).unwrap()).address(),
            (0, 0x12)
        );
        assert_eq!(
            NrpnParam::MultiShiftShapeNoise(TenBitSysex::new(0).unwrap()).address(),
            (0, 0x13)
        );
        assert_eq!(
            NrpnParam::MultiShiftShapeVpm(TenBitSysex::new(0).unwrap()).address(),
            (0, 0x14)
        );
        assert_eq!(
            NrpnParam::MultiShiftShapeUser(TenBitSysex::new(0).unwrap()).address(),
            (0, 0x15)
        );
    }

    #[test]
    fn address_bend_range() {
        assert_eq!(NrpnParam::BendRangePlus(12).address(), (0, 0x16));
        assert_eq!(NrpnParam::BendRangeMinus(12).address(), (0, 0x17));
    }

    #[test]
    fn address_joystick() {
        assert_eq!(
            NrpnParam::JoystickAssignPlus(ModAssignTarget::None).address(),
            (0, 0x18)
        );
        assert_eq!(
            NrpnParam::JoystickRangePlus(EightBitHighRes::new(0).unwrap()).address(),
            (0, 0x19)
        );
        assert_eq!(
            NrpnParam::JoystickAssignMinus(ModAssignTarget::None).address(),
            (0, 0x1A)
        );
        assert_eq!(
            NrpnParam::JoystickRangeMinus(EightBitHighRes::new(0).unwrap()).address(),
            (0, 0x1B)
        );
    }

    #[test]
    fn address_cv_in() {
        assert_eq!(
            NrpnParam::CvInMode(CvInMode::Modulation).address(),
            (0, 0x1C)
        );
        assert_eq!(
            NrpnParam::CvIn1Assign(ModAssignTarget::None).address(),
            (0, 0x1D)
        );
        assert_eq!(
            NrpnParam::CvIn1Range(EightBitHighRes::new(0).unwrap()).address(),
            (0, 0x1E)
        );
        assert_eq!(
            NrpnParam::CvIn2Assign(ModAssignTarget::None).address(),
            (0, 0x1F)
        );
        assert_eq!(
            NrpnParam::CvIn2Range(EightBitHighRes::new(0).unwrap()).address(),
            (0, 0x20)
        );
    }

    #[test]
    fn address_tuning() {
        assert_eq!(
            NrpnParam::MicroTuning(MicroTuning::EqualTemp).address(),
            (0, 0x30)
        );
        assert_eq!(NrpnParam::ScaleKey(0).address(), (0, 0x31));
        assert_eq!(NrpnParam::ProgramTuning(50).address(), (0, 0x32));
    }

    #[test]
    fn address_lfo() {
        assert_eq!(NrpnParam::LfoKeySync(false).address(), (0, 0x33));
        assert_eq!(NrpnParam::LfoVoiceSync(false).address(), (0, 0x34));
        assert_eq!(
            NrpnParam::LfoTargetOsc(LfoTargetOsc::All).address(),
            (0, 0x35)
        );
    }

    #[test]
    fn address_velocity() {
        assert_eq!(
            NrpnParam::CutoffVelocity(U7::new(0).unwrap()).address(),
            (0, 0x36)
        );
        assert_eq!(
            NrpnParam::AmpVelocity(U7::new(0).unwrap()).address(),
            (0, 0x37)
        );
    }

    #[test]
    fn address_multi_octave_routing() {
        assert_eq!(
            NrpnParam::MultiOctave(VcoOctave::Eight).address(),
            (0, 0x38)
        );
        assert_eq!(
            NrpnParam::MultiRouting(MultiRouting::PreVcf).address(),
            (0, 0x39)
        );
    }

    #[test]
    fn address_eg_portamento() {
        assert_eq!(NrpnParam::EgLegato(false).address(), (0, 0x3A));
        assert_eq!(
            NrpnParam::PortamentoMode(PortamentoMode::Auto).address(),
            (0, 0x3B)
        );
        assert_eq!(NrpnParam::PortamentoBpmSync(false).address(), (0, 0x3C));
    }

    #[test]
    fn address_program_level() {
        assert_eq!(NrpnParam::ProgramLevel(60).address(), (0, 0x3F));
    }

    #[test]
    fn address_vpm_params() {
        for (i, expected_lsb) in (0x40..=0x45).enumerate() {
            let v = EightBitHighRes::new(0).unwrap();
            let p = match i {
                0 => NrpnParam::VpmParam1(v),
                1 => NrpnParam::VpmParam2(v),
                2 => NrpnParam::VpmParam3(v),
                3 => NrpnParam::VpmParam4(v),
                4 => NrpnParam::VpmParam5(v),
                5 => NrpnParam::VpmParam6(v),
                _ => unreachable!(),
            };
            assert_eq!(p.address(), (0, expected_lsb));
        }
    }

    #[test]
    fn address_user_params() {
        for (i, expected_lsb) in (0x48..=0x4D).enumerate() {
            let v = EightBitHighRes::new(0).unwrap();
            let p = match i {
                0 => NrpnParam::UserParam1(v),
                1 => NrpnParam::UserParam2(v),
                2 => NrpnParam::UserParam3(v),
                3 => NrpnParam::UserParam4(v),
                4 => NrpnParam::UserParam5(v),
                5 => NrpnParam::UserParam6(v),
                _ => unreachable!(),
            };
            assert_eq!(p.address(), (0, expected_lsb));
        }
    }

    #[test]
    fn address_program_transpose() {
        assert_eq!(NrpnParam::ProgramTranspose(13).address(), (0, 0x50));
    }

    #[test]
    fn address_midi_aftertouch_assign() {
        assert_eq!(
            NrpnParam::MidiAfterTouchAssign(ModAssignTarget::None).address(),
            (0, 0x51)
        );
    }

    #[test]
    fn address_master_volume() {
        assert_eq!(
            NrpnParam::MasterVolume(FourteenBitParam::new(0).unwrap()).address(),
            (0, 0x7F)
        );
    }

    // === Round-trip: to_midi_sequence + NrpnReceiver::feed ===

    #[test]
    fn roundtrip_program_name() {
        let channel = U4::new(0).unwrap();
        for idx in 0..12u8 {
            let original = NrpnParam::ProgramName(idx, U7::new(65 + idx).unwrap());
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for idx={idx}");
        }
    }

    #[test]
    fn roundtrip_voice_mode_type() {
        let channel = U4::new(0).unwrap();
        for variant in [
            VoiceModeType::Poly,
            VoiceModeType::Unison,
            VoiceModeType::Chord,
            VoiceModeType::Arp,
            VoiceModeType::ArpLatch,
        ] {
            let original = NrpnParam::VoiceModeType(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_multi_select_noise() {
        let channel = U4::new(0).unwrap();
        for variant in [
            MultiSelectNoise::High,
            MultiSelectNoise::Low,
            MultiSelectNoise::Peak,
            MultiSelectNoise::Decim,
        ] {
            let original = NrpnParam::MultiSelectNoise(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_ten_bit_sysex_boundary() {
        let channel = U4::new(0).unwrap();
        for value in [0u16, 1, 7, 8, 511, 512, 1023] {
            let original = NrpnParam::MultiShapeNoise(TenBitSysex::new(value).unwrap());
            let msgs = original.to_midi_sequence(channel).unwrap();
            assert_eq!(msgs.len(), 4); // CC99, CC98, CC63, CC6
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for value={value}");
        }
    }

    #[test]
    fn roundtrip_eight_bit_boundary() {
        let channel = U4::new(0).unwrap();
        for value in [0u8, 1, 7, 8, 100, 199, 200] {
            let original = NrpnParam::JoystickRangePlus(EightBitHighRes::new(value).unwrap());
            let msgs = original.to_midi_sequence(channel).unwrap();
            assert_eq!(msgs.len(), 4); // CC99, CC98, CC63, CC6
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for value={value}");
        }
    }

    #[test]
    fn roundtrip_fourteen_bit_boundary() {
        let channel = U4::new(0).unwrap();
        for value in [0u16, 1, 127, 128, 8192, 16382, 16383] {
            let original = NrpnParam::MasterVolume(FourteenBitParam::new(value).unwrap());
            let msgs = original.to_midi_sequence(channel).unwrap();
            assert_eq!(msgs.len(), 4);
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for value={value}");
        }
    }

    #[test]
    fn roundtrip_bool_params() {
        let channel = U4::new(0).unwrap();
        for on in [true, false] {
            for make_param in [
                NrpnParam::LfoKeySync as fn(bool) -> NrpnParam,
                NrpnParam::LfoVoiceSync,
                NrpnParam::EgLegato,
                NrpnParam::PortamentoBpmSync,
            ] {
                let original = make_param(on);
                let msgs = original.to_midi_sequence(channel).unwrap();
                assert_eq!(msgs.len(), 3); // No CC63 for bools
                let mut rx = NrpnReceiver::new();
                let mut result = None;
                for msg in &msgs {
                    result = rx.feed(msg);
                }
                assert_eq!(result, Some(original), "roundtrip failed for {original:?}");
            }
        }
    }

    #[test]
    fn roundtrip_bend_range() {
        let channel = U4::new(0).unwrap();
        for v in [0u8, 6, 12] {
            let original = NrpnParam::BendRangePlus(v);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for v={v}");
        }
    }

    #[test]
    fn roundtrip_micro_tuning() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::MicroTuning(MicroTuning::Pythagorean);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_lfo_target_osc() {
        let channel = U4::new(0).unwrap();
        for variant in [
            LfoTargetOsc::All,
            LfoTargetOsc::Vco1Vco2,
            LfoTargetOsc::Vco2,
            LfoTargetOsc::Multi,
        ] {
            let original = NrpnParam::LfoTargetOsc(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_multi_octave() {
        let channel = U4::new(0).unwrap();
        for variant in [
            VcoOctave::Sixteen,
            VcoOctave::Eight,
            VcoOctave::Four,
            VcoOctave::Two,
        ] {
            let original = NrpnParam::MultiOctave(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_multi_routing() {
        let channel = U4::new(0).unwrap();
        for variant in [MultiRouting::PreVcf, MultiRouting::PostVcf] {
            let original = NrpnParam::MultiRouting(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_portamento_mode() {
        let channel = U4::new(0).unwrap();
        for variant in [PortamentoMode::Auto, PortamentoMode::On] {
            let original = NrpnParam::PortamentoMode(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_cv_in_mode() {
        let channel = U4::new(0).unwrap();
        for variant in [
            CvInMode::Modulation,
            CvInMode::CvGatePlus,
            CvInMode::CvGateMinus,
        ] {
            let original = NrpnParam::CvInMode(variant);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn roundtrip_program_level() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::ProgramLevel(60);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_program_transpose() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::ProgramTranspose(13);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_midi_aftertouch_assign() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::MidiAfterTouchAssign(ModAssignTarget::Cutoff);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_vpm_params() {
        let channel = U4::new(0).unwrap();
        let v = EightBitHighRes::new(150).unwrap();
        for make_param in [
            NrpnParam::VpmParam1 as fn(EightBitHighRes) -> NrpnParam,
            NrpnParam::VpmParam2,
            NrpnParam::VpmParam3,
            NrpnParam::VpmParam4,
            NrpnParam::VpmParam5,
            NrpnParam::VpmParam6,
        ] {
            let original = make_param(v);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {original:?}");
        }
    }

    #[test]
    fn roundtrip_user_params() {
        let channel = U4::new(0).unwrap();
        let v = EightBitHighRes::new(100).unwrap();
        for make_param in [
            NrpnParam::UserParam1 as fn(EightBitHighRes) -> NrpnParam,
            NrpnParam::UserParam2,
            NrpnParam::UserParam3,
            NrpnParam::UserParam4,
            NrpnParam::UserParam5,
            NrpnParam::UserParam6,
        ] {
            let original = make_param(v);
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {original:?}");
        }
    }

    #[test]
    fn roundtrip_velocity() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::CutoffVelocity(U7::new(100).unwrap());
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_scale_key() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::ScaleKey(7);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_program_tuning() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::ProgramTuning(50);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    // === FSM state machine edge cases ===

    #[test]
    fn fsm_out_of_order_cc98_without_cc99() {
        let mut rx = NrpnReceiver::new();
        // CC98 without prior CC99 → Idle
        assert!(rx.feed(&cc(98, 0x0C)).is_none());
        // Subsequent CC6 should not resolve anything
        assert!(rx.feed(&cc(6, 0)).is_none());
    }

    #[test]
    fn fsm_interruption_by_unknown_cc() {
        let mut rx = NrpnReceiver::new();
        rx.feed(&cc(99, 0));
        rx.feed(&cc(98, 0x0C));
        // Interrupted by an unrelated CC
        rx.feed(&cc(1, 64)); // Modulation — resets state
                             // CC6 should not resolve
        assert!(rx.feed(&cc(6, 0)).is_none());
    }

    #[test]
    fn fsm_double_cc99() {
        let mut rx = NrpnReceiver::new();
        rx.feed(&cc(99, 0));
        rx.feed(&cc(99, 0)); // Second CC99 restarts
        rx.feed(&cc(98, 0x0C));
        let result = rx.feed(&cc(6, 0));
        assert!(matches!(
            result,
            Some(NrpnParam::VoiceModeType(VoiceModeType::Poly))
        ));
    }

    #[test]
    fn fsm_cc63_without_address() {
        let mut rx = NrpnReceiver::new();
        // CC63 without being addressed → Idle
        rx.feed(&cc(63, 5));
        // Full sequence after should still work
        rx.feed(&cc(99, 0));
        rx.feed(&cc(98, 0x0C));
        let result = rx.feed(&cc(6, 0));
        assert!(matches!(
            result,
            Some(NrpnParam::VoiceModeType(VoiceModeType::Poly))
        ));
    }

    #[test]
    fn fsm_no_cc63_for_multi_byte() {
        // If CC63 is omitted for a multi-byte param, data_lsb=None → 0
        let mut rx = NrpnReceiver::new();
        let result = feed_nrpn(&mut rx, 0, 0x10, None, 10);
        match result {
            Some(NrpnParam::MultiShapeNoise(v)) => {
                // MSB=10, LSB=0 → (10 << 3) | 0 = 80
                assert_eq!(v.value(), 80);
            }
            other => panic!("expected MultiShapeNoise, got {other:?}"),
        }
    }

    #[test]
    fn fsm_cc6_in_idle_returns_none() {
        let mut rx = NrpnReceiver::new();
        assert!(rx.feed(&cc(6, 42)).is_none());
    }

    #[test]
    fn fsm_cc6_after_only_cc99_returns_none() {
        let mut rx = NrpnReceiver::new();
        rx.feed(&cc(99, 0));
        // CC6 without CC98 → not Addressed, so Idle
        assert!(rx.feed(&cc(6, 0)).is_none());
    }

    #[test]
    fn fsm_resets_after_resolve() {
        let mut rx = NrpnReceiver::new();
        let result = feed_nrpn(&mut rx, 0, 0x0C, None, 0);
        assert!(result.is_some());
        // Receiver should be idle now — CC6 alone should not resolve
        assert!(rx.feed(&cc(6, 1)).is_none());
    }

    // === Unknown address ===

    #[test]
    fn unknown_address_returns_none() {
        let mut rx = NrpnReceiver::new();
        // Address (0, 0x3D) is not used
        let result = feed_nrpn(&mut rx, 0, 0x3D, None, 0);
        assert!(result.is_none());
    }

    #[test]
    fn unknown_msb_returns_none() {
        let mut rx = NrpnReceiver::new();
        // MSB=2 is not used
        let result = feed_nrpn(&mut rx, 2, 0x00, None, 0);
        assert!(result.is_none());
    }

    // === Default ===

    #[test]
    fn receiver_default() {
        let rx = NrpnReceiver::default();
        assert!(format!("{rx:?}").contains("Idle"));
    }

    // === Channel preservation ===

    #[test]
    fn to_midi_sequence_preserves_channel() {
        let channel = U4::new(5).unwrap();
        let param = NrpnParam::BendRangePlus(12);
        let msgs = param.to_midi_sequence(channel).unwrap();
        for msg in &msgs {
            assert_eq!(msg.channel.value(), 5);
        }
    }

    // === Multi-byte message counts ===

    #[test]
    fn ten_bit_sysex_produces_4_messages() {
        let channel = U4::new(0).unwrap();
        let p = NrpnParam::MultiShapeNoise(TenBitSysex::new(500).unwrap());
        let msgs = p.to_midi_sequence(channel).unwrap();
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[0].controller.value(), 99);
        assert_eq!(msgs[1].controller.value(), 98);
        assert_eq!(msgs[2].controller.value(), 63);
        assert_eq!(msgs[3].controller.value(), 6);
    }

    #[test]
    fn single_byte_produces_3_messages() {
        let channel = U4::new(0).unwrap();
        let p = NrpnParam::BendRangePlus(12);
        let msgs = p.to_midi_sequence(channel).unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].controller.value(), 99);
        assert_eq!(msgs[1].controller.value(), 98);
        assert_eq!(msgs[2].controller.value(), 6);
    }

    // === Joystick/CV assign roundtrip ===

    #[test]
    fn roundtrip_joystick_assign() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::JoystickAssignPlus(ModAssignTarget::Cutoff);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_cv_assign() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::CvIn1Assign(ModAssignTarget::LfoRate);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_cv2_range() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::CvIn2Range(EightBitHighRes::new(200).unwrap());
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    // === All shift-shape variants roundtrip ===

    #[test]
    fn roundtrip_all_shift_shapes() {
        let channel = U4::new(0).unwrap();
        for make_param in [
            NrpnParam::MultiShiftShapeNoise as fn(TenBitSysex) -> NrpnParam,
            NrpnParam::MultiShiftShapeVpm,
            NrpnParam::MultiShiftShapeUser,
        ] {
            let original = make_param(TenBitSysex::new(777).unwrap());
            let msgs = original.to_midi_sequence(channel).unwrap();
            let mut rx = NrpnReceiver::new();
            let mut result = None;
            for msg in &msgs {
                result = rx.feed(msg);
            }
            assert_eq!(result, Some(original), "roundtrip failed for {original:?}");
        }
    }

    #[test]
    fn roundtrip_multi_select_vpm() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::MultiSelectVpm(MultiSelectVpm::Fat1);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_multi_select_user() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::MultiSelectUser(MultiSelectUser::User16);
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }

    #[test]
    fn roundtrip_amp_velocity() {
        let channel = U4::new(0).unwrap();
        let original = NrpnParam::AmpVelocity(U7::new(127).unwrap());
        let msgs = original.to_midi_sequence(channel).unwrap();
        let mut rx = NrpnReceiver::new();
        let mut result = None;
        for msg in &msgs {
            result = rx.feed(msg);
        }
        assert_eq!(result, Some(original));
    }
}
