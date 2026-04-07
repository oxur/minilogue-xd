//! Standard MIDI File (Format 0) export.
//!
//! A self-contained builder for producing SMF Format 0 binary data with
//! **no external dependencies** — only [`std::io::Write`] is required for
//! streaming output.
//!
//! # Quick Start
//!
//! ```
//! use minilogue_xd::midi_file::MidiFileBuilder;
//!
//! let midi = MidiFileBuilder::new(120.0)
//!     .track_name("My Track")
//!     .note(0, 60, 100, 240)
//!     .note(480, 64, 90, 240)
//!     .build();
//!
//! assert_eq!(&midi[..4], b"MThd");
//! ```

use std::io::Write;

use crate::param::enums::MultiType;
use std::collections::BTreeSet;

use crate::param::SteppedParam;
use crate::sysex::program::SynthParams;

// ---------------------------------------------------------------------------
// CC legend — human-readable names for Minilogue XD CC numbers
// ---------------------------------------------------------------------------

/// Returns the Minilogue XD parameter name for a CC number, if known.
fn cc_name(cc: u8) -> Option<&'static str> {
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

// ---------------------------------------------------------------------------
// VLQ encoding
// ---------------------------------------------------------------------------

/// Encodes `value` as a MIDI variable-length quantity.
///
/// The VLQ format uses 7 data bits per byte with bit 7 as a continuation
/// flag. The continuation bit is set on every byte **except** the last.
fn encode_vlq(mut value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }
    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0x7F) as u8);
        value >>= 7;
    }
    bytes.reverse();
    let len = bytes.len();
    for b in &mut bytes[..len - 1] {
        *b |= 0x80;
    }
    bytes
}

/// Converts a BPM tempo to microseconds per quarter note.
fn tempo_microseconds(bpm: f64) -> u32 {
    (60_000_000.0 / bpm).round() as u32
}

// ---------------------------------------------------------------------------
// MidiFileEvent (internal)
// ---------------------------------------------------------------------------

/// A single raw MIDI event positioned at an absolute tick.
struct MidiFileEvent {
    tick: u64,
    bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// MidiFileBuilder
// ---------------------------------------------------------------------------

/// A builder for Standard MIDI File (Format 0) binary data.
///
/// Events are accumulated via the builder methods and then serialised by
/// [`build`](Self::build) into a complete SMF byte vector ready to be
/// written to disk or streamed.
///
/// All builder methods consume and return `self` for fluent chaining.
pub struct MidiFileBuilder {
    ticks_per_quarter: u16,
    tempo_bpm: f64,
    channel: u8,
    events: Vec<MidiFileEvent>,
    /// CC numbers used in this file (for legend generation).
    used_ccs: BTreeSet<u8>,
}

impl MidiFileBuilder {
    /// Creates a new builder at the given tempo.
    ///
    /// Defaults: ticks-per-quarter = 480, channel = 0.
    pub fn new(tempo_bpm: f64) -> Self {
        Self {
            ticks_per_quarter: 480,
            tempo_bpm,
            channel: 0,
            events: Vec::new(),
            used_ccs: BTreeSet::new(),
        }
    }

    /// Sets the MIDI channel (0--15). Values above 15 are clamped.
    pub fn channel(mut self, ch: u8) -> Self {
        self.channel = ch.min(15);
        self
    }

    /// Sets the ticks-per-quarter-note resolution of the file.
    pub fn ticks_per_quarter(mut self, tpq: u16) -> Self {
        self.ticks_per_quarter = tpq;
        self
    }

    /// Adds a Track Name meta event (FF 03) at tick 0.
    pub fn track_name(mut self, name: &str) -> Self {
        let name_bytes = name.as_bytes();
        let mut ev = vec![0xFF, 0x03];
        ev.extend_from_slice(&encode_vlq(name_bytes.len() as u64));
        ev.extend_from_slice(name_bytes);
        self.events.push(MidiFileEvent { tick: 0, bytes: ev });
        self
    }

    /// Adds a Note On at `tick` and a corresponding Note Off at
    /// `tick + duration_ticks`.
    pub fn note(mut self, tick: u64, note: u8, velocity: u8, duration_ticks: u64) -> Self {
        let ch = self.channel;
        // Note On
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0x90 | ch, note & 0x7F, velocity & 0x7F],
        });
        // Note Off (velocity 64)
        self.events.push(MidiFileEvent {
            tick: tick + duration_ticks,
            bytes: vec![0x80 | ch, note & 0x7F, 64],
        });
        self
    }

    /// Adds a Control Change event at `tick`.
    pub fn cc(mut self, tick: u64, controller: u8, value: u8) -> Self {
        let ch = self.channel;
        self.used_ccs.insert(controller & 0x7F);
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xB0 | ch, controller & 0x7F, value & 0x7F],
        });
        self
    }

    /// Adds a Program Change event at `tick`.
    pub fn program_change(mut self, tick: u64, program: u8) -> Self {
        let ch = self.channel;
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xC0 | ch, program & 0x7F],
        });
        self
    }

    /// Adds a Pitch Bend event at `tick`.
    ///
    /// `value` is a signed 14-bit range: -8192..=8191.
    /// Center (no bend) is 0, which encodes as 0x2000.
    pub fn pitch_bend(mut self, tick: u64, value: i16) -> Self {
        let ch = self.channel;
        let clamped = value.clamp(-8192, 8191);
        let unsigned = (clamped as i32 + 0x2000) as u16;
        let lsb = (unsigned & 0x7F) as u8;
        let msb = ((unsigned >> 7) & 0x7F) as u8;
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xE0 | ch, lsb, msb],
        });
        self
    }

    /// Adds a System Exclusive event at `tick`.
    ///
    /// `data` should contain the payload bytes **after** the leading F0.
    /// The builder writes `F0 <VLQ length> <data bytes>`.
    pub fn sysex(mut self, tick: u64, data: &[u8]) -> Self {
        let mut ev = vec![0xF0];
        ev.extend_from_slice(&encode_vlq(data.len() as u64));
        ev.extend_from_slice(data);
        self.events.push(MidiFileEvent { tick, bytes: ev });
        self
    }

    // -----------------------------------------------------------------
    // High-level CC helpers (matching RealtimeController patterns)
    // -----------------------------------------------------------------

    /// Add a 10-bit CC parameter at the given tick.
    ///
    /// Emits two CC events: CC 63 with the lower 3 bits (LSB), then
    /// `cc_number` with the upper 7 bits (MSB). This matches how the
    /// Minilogue XD sends high-resolution parameters (cutoff, resonance,
    /// EG attack/decay/sustain/release, LFO rate, VCO pitch/shape, etc.).
    ///
    /// `value` is clamped to 0..=1023.
    ///
    /// # Common CC numbers
    ///
    /// | CC | Parameter |
    /// |----|-----------|
    /// | 43 | Cutoff |
    /// | 44 | Resonance |
    /// | 16 | Amp EG Attack |
    /// | 17 | Amp EG Decay |
    /// | 18 | Amp EG Sustain |
    /// | 19 | Amp EG Release |
    /// | 24 | LFO Rate |
    /// | 26 | LFO Int |
    /// | 34 | VCO 1 Pitch |
    /// | 36 | VCO 1 Shape |
    pub fn ten_bit_cc(mut self, tick: u64, cc_number: u8, value: u16) -> Self {
        let ch = self.channel;
        let clamped = value.min(1023);
        let lsb = (clamped & 0x07) as u8;
        let msb = ((clamped >> 3) & 0x7F) as u8;
        self.used_ccs.insert(63);
        self.used_ccs.insert(cc_number);
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xB0 | ch, 63, lsb],
        });
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xB0 | ch, cc_number, msb],
        });
        self
    }

    /// Add a 10-bit CC parameter using a float (0.0–1.0) mapped to 0–1023.
    ///
    /// Convenience wrapper around [`ten_bit_cc`](Self::ten_bit_cc) for
    /// natural-unit parameters. The value is clamped to 0.0..=1.0.
    pub fn ten_bit_cc_f32(self, tick: u64, cc_number: u8, value: f32) -> Self {
        let raw = (value.clamp(0.0, 1.0) * 1023.0).round() as u16;
        self.ten_bit_cc(tick, cc_number, raw)
    }

    /// Add a stepped (enum) CC parameter at the given tick.
    ///
    /// Uses the enum's TX wire value via [`SteppedParam::to_tx_value`].
    pub fn stepped_cc<T: SteppedParam>(mut self, tick: u64, cc_number: u8, value: T) -> Self {
        let ch = self.channel;
        self.used_ccs.insert(cc_number);
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xB0 | ch, cc_number, value.to_tx_value()],
        });
        self
    }

    /// Add an on/off CC parameter at the given tick.
    ///
    /// `true` sends value 127, `false` sends 0.
    pub fn on_off_cc(mut self, tick: u64, cc_number: u8, on: bool) -> Self {
        let ch = self.channel;
        self.used_ccs.insert(cc_number);
        self.events.push(MidiFileEvent {
            tick,
            bytes: vec![0xB0 | ch, cc_number, if on { 127 } else { 0 }],
        });
        self
    }

    // -----------------------------------------------------------------
    // Named parameter methods (mirrors RealtimeController)
    // -----------------------------------------------------------------

    /// Set the cutoff filter at the given tick (0.0–1.0 → 10-bit via CC 43).
    pub fn set_cutoff(self, tick: u64, value: f32) -> Self {
        self.ten_bit_cc_f32(tick, 43, value)
    }

    /// Set the resonance at the given tick (0.0–1.0 → 10-bit via CC 44).
    pub fn set_resonance(self, tick: u64, value: f32) -> Self {
        self.ten_bit_cc_f32(tick, 44, value)
    }

    /// Set the LFO rate at the given tick (0.0–1.0 → 10-bit via CC 24).
    pub fn set_lfo_rate(self, tick: u64, value: f32) -> Self {
        self.ten_bit_cc_f32(tick, 24, value)
    }

    /// Set the LFO intensity at the given tick (0.0–1.0 → 10-bit via CC 26).
    pub fn set_lfo_int(self, tick: u64, value: f32) -> Self {
        self.ten_bit_cc_f32(tick, 26, value)
    }

    /// Set the delay dry/wet at the given tick (0.0–1.0 → 10-bit via CC 107).
    pub fn set_delay_dry_wet(self, tick: u64, value: f32) -> Self {
        self.ten_bit_cc_f32(tick, 107, value)
    }

    /// Set the reverb dry/wet at the given tick (0.0–1.0 → 10-bit via CC 110).
    pub fn set_reverb_dry_wet(self, tick: u64, value: f32) -> Self {
        self.ten_bit_cc_f32(tick, 110, value)
    }

    // -----------------------------------------------------------------
    // Patch snapshot
    // -----------------------------------------------------------------

    /// Emits CC events for all key synthesizer parameters from a
    /// [`SynthParams`] snapshot.
    ///
    /// This covers every parameter that [`RealtimeController`] supports,
    /// using the same CC numbers and encoding (including the CC63-preceded
    /// 10-bit protocol for high-resolution parameters).
    ///
    /// [`RealtimeController`]: crate::controller::RealtimeController
    pub fn patch_ccs(mut self, tick: u64, synth: &SynthParams) -> Self {
        let ch = self.channel;
        let t = tick;

        // ----- helpers -----

        // Pushes a 10-bit CC pair: CC63 with the 3 LSBs, then the main CC
        // with the 7 MSBs.
        fn emit_10bit(
            events: &mut Vec<MidiFileEvent>,
            used: &mut BTreeSet<u8>,
            ch: u8,
            cc_num: u8,
            value: u16,
            t: u64,
        ) {
            let lsb = (value & 0x07) as u8;
            let msb = ((value >> 3) & 0x7F) as u8;
            used.insert(63);
            used.insert(cc_num);
            events.push(MidiFileEvent {
                tick: t,
                bytes: vec![0xB0 | ch, 63, lsb],
            });
            events.push(MidiFileEvent {
                tick: t,
                bytes: vec![0xB0 | ch, cc_num, msb],
            });
        }

        fn emit_stepped<T: SteppedParam>(
            events: &mut Vec<MidiFileEvent>,
            used: &mut BTreeSet<u8>,
            ch: u8,
            cc_num: u8,
            val: T,
            t: u64,
        ) {
            used.insert(cc_num);
            events.push(MidiFileEvent {
                tick: t,
                bytes: vec![0xB0 | ch, cc_num, val.to_tx_value()],
            });
        }

        fn emit_bool(
            events: &mut Vec<MidiFileEvent>,
            used: &mut BTreeSet<u8>,
            ch: u8,
            cc_num: u8,
            on: bool,
            t: u64,
        ) {
            used.insert(cc_num);
            events.push(MidiFileEvent {
                tick: t,
                bytes: vec![0xB0 | ch, cc_num, if on { 127 } else { 0 }],
            });
        }

        fn emit_cc(
            events: &mut Vec<MidiFileEvent>,
            used: &mut BTreeSet<u8>,
            ch: u8,
            cc_num: u8,
            value: u8,
            t: u64,
        ) {
            used.insert(cc_num);
            events.push(MidiFileEvent {
                tick: t,
                bytes: vec![0xB0 | ch, cc_num, value & 0x7F],
            });
        }

        // ----- Stepped enum parameters -----

        // VCO1
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            50,
            synth.vco1_wave,
            t,
        ); // CC50: VCO1 Wave
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            48,
            synth.vco1_octave,
            t,
        ); // CC48: VCO1 Octave

        // VCO2
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            51,
            synth.vco2_wave,
            t,
        ); // CC51: VCO2 Wave
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            49,
            synth.vco2_octave,
            t,
        ); // CC49: VCO2 Octave

        // Sync/Ring (Sync: CC80, Ring: CC81)
        emit_bool(&mut self.events, &mut self.used_ccs, ch, 80, synth.sync, t);
        emit_bool(&mut self.events, &mut self.used_ccs, ch, 81, synth.ring, t);

        // Multi-engine type (CC53)
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            53,
            synth.multi_type,
            t,
        );

        // EG target (CC23)
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            23,
            synth.eg_target,
            t,
        );

        // LFO
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            57,
            synth.lfo_wave,
            t,
        ); // CC57: LFO Wave
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            58,
            synth.lfo_mode,
            t,
        ); // CC58: LFO Mode
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            56,
            synth.lfo_target,
            t,
        ); // CC56: LFO Target

        // Filter drive/keytrack
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            84,
            synth.cutoff_drive,
            t,
        ); // CC84
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            83,
            synth.cutoff_keytrack,
            t,
        ); // CC83

        // FX types
        // mod_fx_type is stored as raw u8 (1-based in blob; see note P12).
        // Use the helper to convert to the typed enum.
        if let Ok(fx) = synth.mod_fx_type_enum() {
            emit_stepped(&mut self.events, &mut self.used_ccs, ch, 88, fx, t); // CC88: Mod FX Type
        } else {
            emit_cc(&mut self.events, &mut self.used_ccs, ch, 88, 0, t);
        }

        // Mod FX sub-type (CC96) — context-dependent, emit the relevant sub-type
        // based on the current mod_fx_type.
        let mod_fx_sub = match synth.mod_fx_type {
            1 => synth.mod_fx_chorus,
            2 => synth.mod_fx_ensemble,
            3 => synth.mod_fx_phaser,
            4 => synth.mod_fx_flanger,
            5 => synth.mod_fx_user,
            _ => 0,
        };
        emit_cc(&mut self.events, &mut self.used_ccs, ch, 96, mod_fx_sub, t);

        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            89,
            synth.delay_sub_type,
            t,
        ); // CC89
        emit_stepped(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            90,
            synth.reverb_sub_type,
            t,
        ); // CC90

        // ----- 10-bit continuous parameters -----

        // VCO1
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            34,
            synth.vco1_pitch,
            t,
        ); // CC34
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            36,
            synth.vco1_shape,
            t,
        ); // CC36
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            39,
            synth.vco1_level,
            t,
        ); // CC39

        // VCO2
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            35,
            synth.vco2_pitch,
            t,
        ); // CC35
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            37,
            synth.vco2_shape,
            t,
        ); // CC37
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            40,
            synth.vco2_level,
            t,
        ); // CC40

        // Cross-mod depth (CC41)
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            41,
            synth.cross_mod_depth,
            t,
        );

        // Multi-engine: shape is context-dependent on multi_type
        let multi_shape = match synth.multi_type {
            MultiType::Noise => synth.shape_noise,
            MultiType::Vpm => synth.shape_vpm,
            MultiType::User => synth.shape_user,
        };
        emit_10bit(&mut self.events, &mut self.used_ccs, ch, 54, multi_shape, t); // CC54

        let multi_shift = match synth.multi_type {
            MultiType::Noise => synth.shift_shape_noise,
            MultiType::Vpm => synth.shift_shape_vpm,
            MultiType::User => synth.shift_shape_user,
        };
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            104,
            multi_shift,
            t,
        ); // CC104

        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            33,
            synth.multi_level,
            t,
        ); // CC33

        // Filter
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            43,
            synth.cutoff,
            t,
        ); // CC43
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            44,
            synth.resonance,
            t,
        ); // CC44

        // Amp EG
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            16,
            synth.amp_eg_attack,
            t,
        ); // CC16
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            17,
            synth.amp_eg_decay,
            t,
        ); // CC17
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            18,
            synth.amp_eg_sustain,
            t,
        ); // CC18
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            19,
            synth.amp_eg_release,
            t,
        ); // CC19

        // EG
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            20,
            synth.eg_attack,
            t,
        ); // CC20
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            21,
            synth.eg_decay,
            t,
        ); // CC21
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            22,
            synth.eg_int,
            t,
        ); // CC22

        // LFO
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            24,
            synth.lfo_rate,
            t,
        ); // CC24
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            26,
            synth.lfo_int,
            t,
        ); // CC26

        // Mod FX
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            28,
            synth.mod_fx_time,
            t,
        ); // CC28
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            29,
            synth.mod_fx_depth,
            t,
        ); // CC29

        // Delay
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            105,
            synth.delay_time,
            t,
        ); // CC105
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            106,
            synth.delay_depth,
            t,
        ); // CC106
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            107,
            synth.delay_dry_wet,
            t,
        ); // CC107

        // Reverb
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            108,
            synth.reverb_time,
            t,
        ); // CC108
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            109,
            synth.reverb_depth,
            t,
        ); // CC109
        emit_10bit(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            110,
            synth.reverb_dry_wet,
            t,
        ); // CC110

        // ----- On/Off switches -----

        emit_bool(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            92,
            synth.mod_fx_on,
            t,
        ); // CC92
        emit_bool(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            93,
            synth.delay_on,
            t,
        ); // CC93
        emit_bool(
            &mut self.events,
            &mut self.used_ccs,
            ch,
            94,
            synth.reverb_on,
            t,
        ); // CC94

        self
    }

    /// Generates a human-readable legend of all CC numbers used in this
    /// MIDI file, with their Minilogue XD parameter names.
    ///
    /// Useful for documenting what each CC automation lane controls.
    /// Returns a multi-line string sorted by CC number.
    ///
    /// # Example output
    ///
    /// ```text
    /// Minilogue XD MIDI CC Legend
    /// ===========================
    /// CC  16  Amp EG Attack
    /// CC  17  Amp EG Decay
    /// CC  43  Cutoff
    /// CC  44  Resonance
    /// CC  63  Data Entry LSB (10-bit low 3 bits)
    /// ```
    pub fn legend(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Minilogue XD MIDI CC Legend".to_string());
        lines.push("===========================".to_string());
        for &cc in &self.used_ccs {
            let name = cc_name(cc).unwrap_or("(unknown)");
            lines.push(format!("CC {:>3}  {}", cc, name));
        }
        lines.join("\n")
    }

    /// Produces the complete Standard MIDI File (Format 0) as a byte vector.
    pub fn build(mut self) -> Vec<u8> {
        // --- Sort events by tick (stable preserves insertion order) ---
        self.events.sort_by_key(|e| e.tick);

        // --- Build track data ---
        let mut track_data = Vec::new();

        // Meta events at delta 0: track name is already in events if set,
        // so we just need tempo and time signature.
        // Tempo: FF 51 03 tt tt tt
        let tempo_us = tempo_microseconds(self.tempo_bpm);
        track_data.push(0x00); // delta = 0
        track_data.extend_from_slice(&[0xFF, 0x51, 0x03]);
        track_data.push((tempo_us >> 16) as u8);
        track_data.push((tempo_us >> 8) as u8);
        track_data.push(tempo_us as u8);

        // Time signature: FF 58 04 04 02 18 08 (4/4)
        track_data.push(0x00); // delta = 0
        track_data.extend_from_slice(&[0xFF, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08]);

        // Channel events with delta-time encoding
        let mut prev_tick: u64 = 0;
        for event in &self.events {
            let delta = event.tick.saturating_sub(prev_tick);
            track_data.extend_from_slice(&encode_vlq(delta));
            track_data.extend_from_slice(&event.bytes);
            prev_tick = event.tick;
        }

        // End of Track: FF 2F 00
        track_data.push(0x00); // delta = 0
        track_data.extend_from_slice(&[0xFF, 0x2F, 0x00]);

        // --- Build complete file ---
        let mut out = Vec::new();

        // Header chunk: MThd + length(6) + format(0) + tracks(1) + tpq
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&0x00000006u32.to_be_bytes());
        out.extend_from_slice(&0x0000u16.to_be_bytes()); // format 0
        out.extend_from_slice(&0x0001u16.to_be_bytes()); // 1 track
        out.extend_from_slice(&self.ticks_per_quarter.to_be_bytes());

        // Track chunk: MTrk + length + data
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&(track_data.len() as u32).to_be_bytes());
        out.extend_from_slice(&track_data);

        out
    }

    /// Writes the complete SMF to the given writer.
    ///
    /// This is a convenience wrapper around [`build`](Self::build) that
    /// streams the result directly to a [`Write`] implementor.
    pub fn write_to(self, writer: &mut impl Write) -> std::io::Result<()> {
        let data = self.build();
        writer.write_all(&data)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // VLQ encoding
    // ---------------------------------------------------------------

    #[test]
    fn test_vlq_zero() {
        assert_eq!(encode_vlq(0), vec![0x00]);
    }

    #[test]
    fn test_vlq_127() {
        assert_eq!(encode_vlq(127), vec![0x7F]);
    }

    #[test]
    fn test_vlq_128() {
        assert_eq!(encode_vlq(128), vec![0x81, 0x00]);
    }

    #[test]
    fn test_vlq_16383() {
        assert_eq!(encode_vlq(16383), vec![0xFF, 0x7F]);
    }

    #[test]
    fn test_vlq_large() {
        // 0x200000 = 2097152
        assert_eq!(encode_vlq(0x200000), vec![0x81, 0x80, 0x80, 0x00]);
    }

    // ---------------------------------------------------------------
    // Tempo
    // ---------------------------------------------------------------

    #[test]
    fn test_tempo_120_bpm() {
        assert_eq!(tempo_microseconds(120.0), 500_000);
    }

    #[test]
    fn test_tempo_60_bpm() {
        assert_eq!(tempo_microseconds(60.0), 1_000_000);
    }

    // ---------------------------------------------------------------
    // Header
    // ---------------------------------------------------------------

    #[test]
    fn test_header_valid_mthd() {
        let data = MidiFileBuilder::new(120.0).build();
        assert_eq!(&data[..4], b"MThd");
        // Header length = 6
        assert_eq!(&data[4..8], &[0, 0, 0, 6]);
        // Format 0
        assert_eq!(&data[8..10], &[0, 0]);
        // 1 track
        assert_eq!(&data[10..12], &[0, 1]);
    }

    #[test]
    fn test_header_tpq_encoding() {
        let data = MidiFileBuilder::new(120.0).ticks_per_quarter(960).build();
        // TPQ = 960 = 0x03C0
        assert_eq!(data[12], 0x03);
        assert_eq!(data[13], 0xC0);
    }

    // ---------------------------------------------------------------
    // Track structure
    // ---------------------------------------------------------------

    #[test]
    fn test_track_has_mtrk_and_correct_length() {
        let data = MidiFileBuilder::new(120.0).build();
        assert_eq!(&data[14..18], b"MTrk");
        let track_len = u32::from_be_bytes([data[18], data[19], data[20], data[21]]) as usize;
        // Everything after the track length field should be track_len bytes
        assert_eq!(data.len() - 22, track_len);
    }

    #[test]
    fn test_track_ends_with_end_of_track() {
        let data = MidiFileBuilder::new(120.0).build();
        let len = data.len();
        assert_eq!(&data[len - 3..], &[0xFF, 0x2F, 0x00]);
    }

    // ---------------------------------------------------------------
    // Notes
    // ---------------------------------------------------------------

    #[test]
    fn test_single_note_on_off() {
        let data = MidiFileBuilder::new(120.0).note(0, 60, 100, 240).build();
        // The track data (after header) should contain a Note On and Note Off
        let track_start = 22; // after MThd(14) + MTrk header(8)
        let track_bytes = &data[track_start..];
        // Find Note On (0x90, 60, 100)
        assert!(track_bytes.windows(3).any(|w| w == [0x90, 60, 100]));
        // Find Note Off (0x80, 60, 64)
        assert!(track_bytes.windows(3).any(|w| w == [0x80, 60, 64]));
    }

    #[test]
    fn test_note_duration_delta_encoding() {
        // A note at tick 0, duration 480 should produce a Note Off with delta 480
        let data = MidiFileBuilder::new(120.0).note(0, 60, 100, 480).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // 480 in VLQ = 0x83, 0x60
        let vlq_480 = encode_vlq(480);
        assert_eq!(vlq_480, vec![0x83, 0x60]);
        // The Note Off should be preceded by VLQ(480)
        let note_off = [0x80, 60, 64];
        for i in 0..track_bytes.len() - 4 {
            if track_bytes[i..i + 2] == vlq_480[..]
                && i + 2 + 3 <= track_bytes.len()
                && track_bytes[i + 2..i + 5] == note_off
            {
                return; // found it
            }
        }
        panic!("Expected Note Off with delta 480 not found");
    }

    #[test]
    fn test_multiple_notes_ordering() {
        let data = MidiFileBuilder::new(120.0)
            .note(0, 60, 100, 120)
            .note(240, 64, 90, 120)
            .build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Both Note On events should appear (order: note 60 first, note 64 later)
        let pos_60 = track_bytes
            .windows(3)
            .position(|w| w == [0x90, 60, 100])
            .expect("Note On 60 not found");
        let pos_64 = track_bytes
            .windows(3)
            .position(|w| w == [0x90, 64, 90])
            .expect("Note On 64 not found");
        assert!(pos_60 < pos_64);
    }

    #[test]
    fn test_note_velocity() {
        let data = MidiFileBuilder::new(120.0).note(0, 72, 127, 120).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        assert!(track_bytes.windows(3).any(|w| w == [0x90, 72, 127]));
    }

    // ---------------------------------------------------------------
    // CCs
    // ---------------------------------------------------------------

    #[test]
    fn test_single_cc_bytes() {
        let data = MidiFileBuilder::new(120.0).cc(0, 74, 64).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 74, 64]));
    }

    #[test]
    fn test_multiple_ccs_same_tick_delta_zero() {
        let data = MidiFileBuilder::new(120.0)
            .cc(0, 74, 64)
            .cc(0, 71, 100)
            .build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Both CCs should be present
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 74, 64]));
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 71, 100]));
        // Find second CC and verify it's preceded by delta 0
        let first_pos = track_bytes
            .windows(3)
            .position(|w| w == [0xB0, 74, 64])
            .unwrap();
        let after_first = first_pos + 3;
        // Next byte should be delta=0 followed by the second CC
        assert_eq!(track_bytes[after_first], 0x00); // delta 0
        assert_eq!(
            &track_bytes[after_first + 1..after_first + 4],
            &[0xB0, 71, 100]
        );
    }

    #[test]
    fn test_cc_value_clamped_to_7bit() {
        let data = MidiFileBuilder::new(120.0).cc(0, 200, 200).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Values should be masked to 7 bits: 200 & 0x7F = 72
        assert!(track_bytes
            .windows(3)
            .any(|w| w == [0xB0, 200 & 0x7F, 200 & 0x7F]));
    }

    // ---------------------------------------------------------------
    // Pitch bend
    // ---------------------------------------------------------------

    #[test]
    fn test_pitch_bend_center() {
        let data = MidiFileBuilder::new(120.0).pitch_bend(0, 0).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Center = 0x2000 => LSB=0x00, MSB=0x40
        assert!(track_bytes.windows(3).any(|w| w == [0xE0, 0x00, 0x40]));
    }

    // ---------------------------------------------------------------
    // Program change
    // ---------------------------------------------------------------

    #[test]
    fn test_program_change_encoding() {
        let data = MidiFileBuilder::new(120.0).program_change(0, 42).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        assert!(track_bytes.windows(2).any(|w| w == [0xC0, 42]));
    }

    // ---------------------------------------------------------------
    // SysEx
    // ---------------------------------------------------------------

    #[test]
    fn test_sysex_event() {
        let sysex_data = &[0x42, 0x30, 0x00, 0x01, 0x51, 0xF7];
        let data = MidiFileBuilder::new(120.0).sysex(0, sysex_data).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Should contain F0, VLQ(6), then the data bytes
        assert!(track_bytes.windows(1).any(|w| w == [0xF0]));
        assert!(track_bytes
            .windows(sysex_data.len())
            .any(|w| w == sysex_data));
    }

    // ---------------------------------------------------------------
    // patch_ccs
    // ---------------------------------------------------------------

    #[test]
    fn test_patch_ccs_default_synth_params() {
        let synth = SynthParams::default();
        let data = MidiFileBuilder::new(120.0).patch_ccs(0, &synth).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];

        // Should contain CC50 for VCO1 wave (Saw => tx=0 for Square? No, let's
        // check: VcoWave::Saw tx value)
        // Default is Saw. Saw tx value — we just verify the CC number is
        // present.
        // CC50 byte should appear as part of a CC message on channel 0
        assert!(track_bytes.windows(2).any(|w| w[0] == 0xB0 && w[1] == 50));
        // CC43 (cutoff) should be present (via 10-bit: CC63 then CC43)
        assert!(track_bytes.windows(2).any(|w| w[0] == 0xB0 && w[1] == 43));
        // On/off switches: CC92 (mod_fx_on) should be present
        assert!(track_bytes.windows(2).any(|w| w[0] == 0xB0 && w[1] == 92));
    }

    #[test]
    fn test_patch_ccs_specific_values() {
        use crate::param::enums::{DelaySubType, VcoOctave, VcoWave};

        let synth = SynthParams {
            vco1_wave: VcoWave::Sqr,
            vco1_octave: VcoOctave::Sixteen,
            delay_on: true,
            delay_sub_type: DelaySubType::Tape,
            ..SynthParams::default()
        };

        let data = MidiFileBuilder::new(120.0).patch_ccs(0, &synth).build();
        let track_start = 22;
        let track_bytes = &data[track_start..];

        // VCO1 Wave (CC50): Sqr tx value
        let sqr_tx = VcoWave::Sqr.to_tx_value();
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 50, sqr_tx]));

        // VCO1 Octave (CC48): Sixteen tx value
        let oct_tx = VcoOctave::Sixteen.to_tx_value();
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 48, oct_tx]));

        // Delay on (CC93): should be 127
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 93, 127]));

        // Delay sub-type (CC89): Tape tx value
        let tape_tx = DelaySubType::Tape.to_tx_value();
        assert!(track_bytes.windows(3).any(|w| w == [0xB0, 89, tape_tx]));
    }

    // ---------------------------------------------------------------
    // Integration
    // ---------------------------------------------------------------

    #[test]
    fn test_build_starts_with_mthd() {
        let data = MidiFileBuilder::new(120.0)
            .track_name("Test")
            .note(0, 60, 100, 480)
            .build();
        assert_eq!(&data[..4], b"MThd");
    }

    #[test]
    fn test_round_trip_length_consistency() {
        let builder = MidiFileBuilder::new(120.0)
            .track_name("Length Test")
            .note(0, 60, 100, 240)
            .note(480, 64, 90, 240)
            .cc(0, 74, 64)
            .pitch_bend(960, 1000);
        let data = builder.build();

        // Verify the file length matches header + track header + track data
        let header_size = 14; // MThd(4) + len(4) + format(2) + tracks(2) + tpq(2)
        let track_header_size = 8; // MTrk(4) + len(4)
        let track_data_len = u32::from_be_bytes([data[18], data[19], data[20], data[21]]) as usize;
        assert_eq!(data.len(), header_size + track_header_size + track_data_len);
    }

    // ---------------------------------------------------------------
    // write_to
    // ---------------------------------------------------------------

    #[test]
    fn test_write_to_produces_same_bytes_as_build() {
        let data1 = MidiFileBuilder::new(120.0).note(0, 60, 100, 480).build();

        let mut buf = Vec::new();
        MidiFileBuilder::new(120.0)
            .note(0, 60, 100, 480)
            .write_to(&mut buf)
            .unwrap();

        assert_eq!(data1, buf);
    }

    // ---------------------------------------------------------------
    // Channel selection
    // ---------------------------------------------------------------

    #[test]
    fn test_channel_selection() {
        let data = MidiFileBuilder::new(120.0)
            .channel(5)
            .note(0, 60, 100, 240)
            .build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Note On on channel 5 = 0x95
        assert!(track_bytes.windows(3).any(|w| w == [0x95, 60, 100]));
    }

    #[test]
    fn test_channel_clamped_to_15() {
        let data = MidiFileBuilder::new(120.0)
            .channel(255)
            .note(0, 60, 100, 240)
            .build();
        let track_start = 22;
        let track_bytes = &data[track_start..];
        // Should be channel 15 = 0x9F
        assert!(track_bytes.windows(3).any(|w| w == [0x9F, 60, 100]));
    }
}
