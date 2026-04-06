//! Sequencer parameters from TABLE 2 (offsets 156--1023).
//!
//! This module provides [`SequencerParams`], [`StepEvent`], and
//! [`MotionSlotConfig`] for the step sequencer section of the program blob.

use crate::error::{Result, SysexError};
use crate::sysex::helpers::{read_u16_le, write_u16_le};

// ---------------------------------------------------------------------------
// MotionSlotConfig
// ---------------------------------------------------------------------------

/// Configuration for a single motion sequence slot.
///
/// Each slot holds a parameter ID (u16) identifying which synth parameter
/// the motion sequence controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MotionSlotConfig {
    /// Raw motion parameter ID (see note S2 of the spec).
    pub parameter_id: u16,
}

// ---------------------------------------------------------------------------
// StepEvent
// ---------------------------------------------------------------------------

/// A single step in the 16-step sequencer.
///
/// Each step can hold up to 8 notes with independent velocities and gate
/// times, plus motion sequence data for up to 4 slots.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepEvent {
    /// MIDI note numbers for up to 8 notes (0 = unused).
    pub notes: [u8; 8],
    /// Velocity for each note (0--127).
    pub velocities: [u8; 8],
    /// Gate time (bits 0--6, 0--72 = 0%--100%) and trigger switch (bit 7).
    pub gate_times: [u8; 8],
    /// Raw motion data per slot: 4 slots x 7 bytes each.
    pub motion_data: [[u8; 7]; 4],
}

impl StepEvent {
    /// Size of a single step event in bytes.
    pub const SIZE: usize = 52;

    /// Parse a step event from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is too short.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::SIZE,
                actual: bytes.len(),
            }
            .into());
        }

        let mut notes = [0u8; 8];
        notes.copy_from_slice(&bytes[0..8]);

        let mut velocities = [0u8; 8];
        velocities.copy_from_slice(&bytes[8..16]);

        let mut gate_times = [0u8; 8];
        gate_times.copy_from_slice(&bytes[16..24]);

        let mut motion_data = [[0u8; 7]; 4];
        for (i, slot) in motion_data.iter_mut().enumerate() {
            let off = 24 + i * 7;
            slot.copy_from_slice(&bytes[off..off + 7]);
        }

        Ok(Self {
            notes,
            velocities,
            gate_times,
            motion_data,
        })
    }

    /// Serialize this step event to bytes, appending to `out`.
    pub fn write_to(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.notes);
        out.extend_from_slice(&self.velocities);
        out.extend_from_slice(&self.gate_times);
        for slot in &self.motion_data {
            out.extend_from_slice(slot);
        }
    }

    /// Returns the gate time for note `n` (0--7), in the range 0--72.
    pub fn gate_time(&self, n: usize) -> u8 {
        self.gate_times[n] & 0x7F
    }

    /// Returns whether the trigger switch is set for note `n` (0--7).
    pub fn trigger_switch(&self, n: usize) -> bool {
        self.gate_times[n] & 0x80 != 0
    }

    /// Set the gate time and trigger switch for note `n`.
    pub fn set_gate_time(&mut self, n: usize, gate: u8, trigger: bool) {
        self.gate_times[n] = (gate & 0x7F) | (u8::from(trigger) << 7);
    }
}

// ---------------------------------------------------------------------------
// SequencerParams
// ---------------------------------------------------------------------------

/// Sequencer parameters from offsets 156--1023 of the program blob.
///
/// The sequencer section contains the step sequencer configuration, 16
/// step events, and arpeggiator settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SequencerParams {
    /// BPM x 10: 100 = 10.0 BPM, 3000 = 300.0 BPM, stored as LE u16.
    pub bpm: u16,
    /// Number of active steps (1--16).
    pub step_length: u8,
    /// Step resolution (0--4).
    pub step_resolution: u8,
    /// Swing amount, stored as u8 with +75 offset (0 = −75, 75 = 0, 150 = +75).
    pub swing: u8,
    /// Default gate time (0--72 = 0%--100%).
    pub default_gate_time: u8,
    /// Bitfield: which of the 16 steps are active (bit N = step N).
    pub active_steps: u16,
    /// Bitfield: which of the 16 steps are enabled/on.
    pub steps_on: u16,
    /// Bitfield: which of the 16 steps have motion enabled.
    pub motion_on: u16,
    /// Configuration for the 4 motion sequence slots.
    pub motion_slots: [MotionSlotConfig; 4],
    /// Per-slot step enable bitfield (bit N = step N has motion data for this slot).
    pub motion_slot_steps: [u16; 4],
    /// The 16 step events.
    pub steps: [StepEvent; 16],
    /// Arpeggiator gate time (0--72).
    pub arp_gate_time: u8,
    /// Arpeggiator rate (0--10, see [`ArpRate`](crate::sysex::enums::ArpRate)).
    pub arp_rate: u8,
}

impl SequencerParams {
    /// Size of the sequencer parameter block in bytes.
    pub const SIZE: usize = 868;

    /// Offset within the 1024-byte program blob where the sequencer starts.
    pub const OFFSET: usize = 156;

    /// Parse sequencer parameters from a byte slice starting at offset 156.
    ///
    /// The input should be the sequencer portion of the blob (868 bytes).
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is too short.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::SIZE,
                actual: bytes.len(),
            }
            .into());
        }

        // Offsets are relative to the start of the sequencer section (blob offset 156).
        //
        // Layout (34 bytes header + 832 bytes steps + 2 bytes arp = 868):
        //   0-5:   reserved (6 bytes)
        //   6-7:   BPM (LE u16, x10)
        //   8:     step_length
        //   9:     step_resolution
        //   10:    swing
        //   11:    default_gate_time
        //   12-13: active_steps (LE u16 bitfield)
        //   14-15: steps_on (LE u16 bitfield)
        //   16-17: motion_on (LE u16 bitfield)
        //   18-25: motion_slot param IDs (4 x LE u16)
        //   26-33: motion_slot_steps (4 x LE u16)
        //   34-865: 16 step events (16 x 52 bytes)
        //   866:   arp_gate_time
        //   867:   arp_rate
        let bpm = read_u16_le(bytes, 6);
        let step_length = bytes[8];
        let step_resolution = bytes[9];
        let swing = bytes[10];
        let default_gate_time = bytes[11];
        let active_steps = read_u16_le(bytes, 12);
        let steps_on = read_u16_le(bytes, 14);
        let motion_on = read_u16_le(bytes, 16);

        // Motion slots: parameter IDs at offsets 18-25 (4 x u16 LE).
        let mut motion_slots = [MotionSlotConfig::default(); 4];
        for (i, slot) in motion_slots.iter_mut().enumerate() {
            slot.parameter_id = read_u16_le(bytes, 18 + i * 2);
        }

        // Motion slot step enables at offsets 26-33 (4 x u16 LE).
        let mut motion_slot_steps = [0u16; 4];
        for (i, step_bits) in motion_slot_steps.iter_mut().enumerate() {
            *step_bits = read_u16_le(bytes, 26 + i * 2);
        }

        // 16 step events start at relative offset 34 (blob offset 190).
        let step_base = 34;
        let mut steps: [StepEvent; 16] = Default::default();
        for (i, step) in steps.iter_mut().enumerate() {
            let off = step_base + i * StepEvent::SIZE;
            *step = StepEvent::from_bytes(&bytes[off..])?;
        }

        // After 16 steps: offset = 34 + 16*52 = 34 + 832 = 866.
        let arp_gate_time = bytes[866];
        let arp_rate = bytes[867];

        Ok(Self {
            bpm,
            step_length,
            step_resolution,
            swing,
            default_gate_time,
            active_steps,
            steps_on,
            motion_on,
            motion_slots,
            motion_slot_steps,
            steps,
            arp_gate_time,
            arp_rate,
        })
    }

    /// Serialize sequencer parameters to bytes (868 bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; Self::SIZE];

        // First 6 bytes: reserved / header (zeros).
        write_u16_le(&mut out, 6, self.bpm);
        out[8] = self.step_length;
        out[9] = self.step_resolution;
        out[10] = self.swing;
        out[11] = self.default_gate_time;
        write_u16_le(&mut out, 12, self.active_steps);
        write_u16_le(&mut out, 14, self.steps_on);
        write_u16_le(&mut out, 16, self.motion_on);

        for i in 0..4 {
            write_u16_le(&mut out, 18 + i * 2, self.motion_slots[i].parameter_id);
        }

        for i in 0..4 {
            write_u16_le(&mut out, 26 + i * 2, self.motion_slot_steps[i]);
        }

        let step_base = 34;
        for (i, step) in self.steps.iter().enumerate() {
            let off = step_base + i * StepEvent::SIZE;
            // Write step in-place.
            out[off..off + 8].copy_from_slice(&step.notes);
            out[off + 8..off + 16].copy_from_slice(&step.velocities);
            out[off + 16..off + 24].copy_from_slice(&step.gate_times);
            for (j, slot) in step.motion_data.iter().enumerate() {
                let slot_off = off + 24 + j * 7;
                out[slot_off..slot_off + 7].copy_from_slice(slot);
            }
        }

        out[866] = self.arp_gate_time;
        out[867] = self.arp_rate;

        out
    }

    /// Returns the BPM as a floating-point value (e.g. 120.0).
    pub fn bpm_f32(&self) -> f32 {
        f32::from(self.bpm) / 10.0
    }

    /// Returns the swing as a signed value (−75 to +75).
    pub fn swing_signed(&self) -> i8 {
        (i16::from(self.swing) - 75) as i8
    }
}

impl Default for SequencerParams {
    fn default() -> Self {
        Self {
            bpm: 1200, // 120.0 BPM
            step_length: 16,
            step_resolution: 0,    // 1/16
            swing: 75,             // 0 (center)
            default_gate_time: 54, // ~75%
            active_steps: 0xFFFF,  // all 16 active
            steps_on: 0,
            motion_on: 0,
            motion_slots: [MotionSlotConfig::default(); 4],
            motion_slot_steps: [0; 4],
            steps: Default::default(),
            arp_gate_time: 54,
            arp_rate: 9, // 1/8
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid sequencer blob (868 bytes).
    fn make_seq_blob() -> Vec<u8> {
        let mut buf = vec![0u8; SequencerParams::SIZE];
        // BPM = 1200 (120.0 BPM).
        write_u16_le(&mut buf, 6, 1200);
        buf[8] = 16; // step_length
        buf[9] = 0; // step_resolution = 1/16
        buf[10] = 75; // swing = 0
        buf[11] = 54; // default_gate_time = ~75%
        write_u16_le(&mut buf, 12, 0xFFFF); // all steps active
        write_u16_le(&mut buf, 14, 0x0001); // step 0 on
        write_u16_le(&mut buf, 16, 0x0001); // motion on for step 0
                                            // Motion slot 0 parameter ID.
        write_u16_le(&mut buf, 18, 0x0010);
        write_u16_le(&mut buf, 26, 0x0003); // motion slot 0 steps 0,1

        // Step 0: note C4 (60) with velocity 100, gate 54.
        let step_off = 34;
        buf[step_off] = 60; // note 0
        buf[step_off + 8] = 100; // velocity 0
        buf[step_off + 16] = 54; // gate time 0

        buf[866] = 36; // arp_gate_time
        buf[867] = 6; // arp_rate = 1/4
        buf
    }

    #[test]
    fn seq_params_from_bytes_valid() {
        let blob = make_seq_blob();
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.bpm, 1200);
        assert_eq!(params.step_length, 16);
        assert_eq!(params.step_resolution, 0);
        assert_eq!(params.swing, 75);
        assert_eq!(params.default_gate_time, 54);
        assert_eq!(params.active_steps, 0xFFFF);
        assert_eq!(params.steps_on, 0x0001);
        assert_eq!(params.motion_on, 0x0001);
        assert_eq!(params.motion_slots[0].parameter_id, 0x0010);
        assert_eq!(params.motion_slot_steps[0], 0x0003);
        assert_eq!(params.steps[0].notes[0], 60);
        assert_eq!(params.steps[0].velocities[0], 100);
        assert_eq!(params.steps[0].gate_time(0), 54);
        assert_eq!(params.arp_gate_time, 36);
        assert_eq!(params.arp_rate, 6);
    }

    #[test]
    fn seq_params_round_trip() {
        let blob = make_seq_blob();
        let params = SequencerParams::from_bytes(&blob).unwrap();
        let out = params.to_bytes();
        assert_eq!(&out[..], &blob[..]);
    }

    #[test]
    fn seq_params_too_short() {
        let short = vec![0u8; 100];
        assert!(SequencerParams::from_bytes(&short).is_err());
    }

    #[test]
    fn seq_params_bpm_display() {
        let blob = make_seq_blob();
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert!((params.bpm_f32() - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn seq_params_bpm_min() {
        let mut blob = make_seq_blob();
        write_u16_le(&mut blob, 6, 100);
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert!((params.bpm_f32() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn seq_params_bpm_max() {
        let mut blob = make_seq_blob();
        write_u16_le(&mut blob, 6, 3000);
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert!((params.bpm_f32() - 300.0).abs() < f32::EPSILON);
    }

    #[test]
    fn seq_params_swing_offset() {
        let mut blob = make_seq_blob();
        blob[10] = 0; // -75
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.swing_signed(), -75);

        blob[10] = 75; // 0
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.swing_signed(), 0);

        blob[10] = 150; // +75
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.swing_signed(), 75);
    }

    #[test]
    fn seq_params_bitfields() {
        let mut blob = make_seq_blob();
        write_u16_le(&mut blob, 12, 0b1010_1010_1010_1010);
        write_u16_le(&mut blob, 14, 0b0101_0101_0101_0101);
        write_u16_le(&mut blob, 16, 0b1111_0000_1111_0000);
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.active_steps, 0b1010_1010_1010_1010);
        assert_eq!(params.steps_on, 0b0101_0101_0101_0101);
        assert_eq!(params.motion_on, 0b1111_0000_1111_0000);
    }

    #[test]
    fn step_event_notes_and_velocities() {
        let mut blob = make_seq_blob();
        let step_off = 34;
        // Fill step 0 with 3 notes.
        blob[step_off] = 60; // C4
        blob[step_off + 1] = 64; // E4
        blob[step_off + 2] = 67; // G4
        blob[step_off + 8] = 100;
        blob[step_off + 9] = 80;
        blob[step_off + 10] = 60;
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.steps[0].notes[0], 60);
        assert_eq!(params.steps[0].notes[1], 64);
        assert_eq!(params.steps[0].notes[2], 67);
        assert_eq!(params.steps[0].velocities[0], 100);
        assert_eq!(params.steps[0].velocities[1], 80);
        assert_eq!(params.steps[0].velocities[2], 60);
    }

    #[test]
    fn step_event_gate_time_trigger_switch() {
        let mut step = StepEvent::default();
        step.set_gate_time(0, 54, false);
        assert_eq!(step.gate_time(0), 54);
        assert!(!step.trigger_switch(0));

        step.set_gate_time(0, 54, true);
        assert_eq!(step.gate_time(0), 54);
        assert!(step.trigger_switch(0));

        // Verify raw byte.
        assert_eq!(step.gate_times[0], 54 | 0x80);
    }

    #[test]
    fn step_event_gate_time_bit_packing() {
        let mut blob = make_seq_blob();
        let step_off = 34;
        // Gate time 72 with trigger on.
        blob[step_off + 16] = 72 | 0x80;
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.steps[0].gate_time(0), 72);
        assert!(params.steps[0].trigger_switch(0));

        // Round-trip.
        let out = params.to_bytes();
        assert_eq!(out[step_off + 16], 72 | 0x80);
    }

    #[test]
    fn step_event_motion_data_round_trip() {
        let mut blob = make_seq_blob();
        let step_off = 34;
        // Write some motion data to slot 0 of step 0.
        let motion_off = step_off + 24;
        for i in 0..7 {
            blob[motion_off + i] = (i + 1) as u8;
        }
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.steps[0].motion_data[0], [1, 2, 3, 4, 5, 6, 7]);
        let out = params.to_bytes();
        assert_eq!(&out[motion_off..motion_off + 7], &[1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn seq_params_empty_sequencer() {
        let blob = vec![0u8; SequencerParams::SIZE];
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.bpm, 0);
        assert_eq!(params.step_length, 0);
        assert_eq!(params.active_steps, 0);
        assert_eq!(params.steps_on, 0);
        for step in &params.steps {
            assert_eq!(step.notes, [0; 8]);
            assert_eq!(step.velocities, [0; 8]);
        }
        // Round-trip.
        let out = params.to_bytes();
        assert_eq!(&out[..], &blob[..]);
    }

    #[test]
    fn seq_params_default_round_trips() {
        let params = SequencerParams::default();
        let bytes = params.to_bytes();
        let recovered = SequencerParams::from_bytes(&bytes).unwrap();
        assert_eq!(params, recovered);
    }

    #[test]
    fn step_event_too_short() {
        let short = [0u8; 10];
        assert!(StepEvent::from_bytes(&short).is_err());
    }

    #[test]
    fn step_event_write_to() {
        let step = StepEvent {
            notes: [60, 64, 67, 0, 0, 0, 0, 0],
            velocities: [100, 80, 60, 0, 0, 0, 0, 0],
            gate_times: [54, 54, 54, 0, 0, 0, 0, 0],
            motion_data: [[0; 7]; 4],
        };
        let mut buf = Vec::new();
        step.write_to(&mut buf);
        assert_eq!(buf.len(), StepEvent::SIZE);
        assert_eq!(buf[0], 60);
        assert_eq!(buf[8], 100);
        assert_eq!(buf[16], 54);
    }

    #[test]
    fn step_event_default() {
        let step = StepEvent::default();
        assert_eq!(step.notes, [0; 8]);
        assert_eq!(step.velocities, [0; 8]);
        assert_eq!(step.gate_times, [0; 8]);
        assert_eq!(step.motion_data, [[0; 7]; 4]);
    }

    #[test]
    fn motion_slot_config_default() {
        let slot = MotionSlotConfig::default();
        assert_eq!(slot.parameter_id, 0);
    }

    #[test]
    fn seq_params_multiple_steps_with_data() {
        let mut blob = make_seq_blob();
        // Put notes in steps 0, 7, and 15.
        for (step_idx, note) in [(0, 60u8), (7, 72), (15, 84)] {
            let off = 34 + step_idx * StepEvent::SIZE;
            blob[off] = note;
            blob[off + 8] = 127; // max velocity
        }
        let params = SequencerParams::from_bytes(&blob).unwrap();
        assert_eq!(params.steps[0].notes[0], 60);
        assert_eq!(params.steps[7].notes[0], 72);
        assert_eq!(params.steps[15].notes[0], 84);
        assert_eq!(params.steps[0].velocities[0], 127);
    }

    #[test]
    fn seq_params_all_motion_slots() {
        let mut blob = make_seq_blob();
        for i in 0..4 {
            write_u16_le(&mut blob, 18 + i * 2, (i as u16 + 1) * 0x10);
            write_u16_le(&mut blob, 26 + i * 2, 0xFFFF);
        }
        let params = SequencerParams::from_bytes(&blob).unwrap();
        for i in 0..4 {
            assert_eq!(params.motion_slots[i].parameter_id, (i as u16 + 1) * 0x10);
            assert_eq!(params.motion_slot_steps[i], 0xFFFF);
        }
    }
}
