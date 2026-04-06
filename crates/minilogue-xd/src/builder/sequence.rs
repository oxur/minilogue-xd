//! Fluent builder for step sequences.
//!
//! [`SequenceBuilder`] starts from [`SequencerParams::default()`] and provides
//! ergonomic methods to set BPM, step length, resolution, and individual
//! step events.

use crate::sysex::program::SequencerParams;

/// A consuming builder for [`SequencerParams`].
///
/// Values are clamped to valid ranges rather than returning errors, matching
/// the forgiving builder pattern used by [`PatchBuilder`](super::PatchBuilder).
///
/// # Examples
///
/// ```
/// use minilogue_xd::builder::SequenceBuilder;
///
/// let seq = SequenceBuilder::new()
///     .bpm(130.0)
///     .length(8)
///     .step(0, 60, 100)
///     .step(2, 64, 80)
///     .step(4, 67, 90)
///     .step(6, 72, 110)
///     .build();
///
/// assert_eq!(seq.bpm, 1300);
/// assert_eq!(seq.step_length, 8);
/// ```
pub struct SequenceBuilder {
    seq: SequencerParams,
}

impl SequenceBuilder {
    /// Creates a new builder starting from the default sequencer parameters
    /// (120 BPM, 16 steps, 1/16 resolution).
    pub fn new() -> Self {
        Self {
            seq: SequencerParams::default(),
        }
    }

    /// Sets the tempo in BPM (10.0--300.0).
    ///
    /// Stored internally as BPM x 10 (e.g., 120.0 -> 1200).
    /// Values are clamped to 10.0--300.0.
    pub fn bpm(mut self, bpm: f32) -> Self {
        self.seq.bpm = (bpm.clamp(10.0, 300.0) * 10.0).round() as u16;
        self
    }

    /// Sets the number of active steps (1--16).
    ///
    /// Values are clamped to 1--16.
    pub fn length(mut self, steps: u8) -> Self {
        self.seq.step_length = steps.clamp(1, 16);
        self
    }

    /// Sets the step resolution.
    ///
    /// Values: 0 = 1/16, 1 = 1/8, 2 = 1/4, 3 = 1/2, 4 = 1/1.
    /// Out-of-range values are clamped to 0--4.
    pub fn resolution(mut self, res: u8) -> Self {
        self.seq.step_resolution = res.min(4);
        self
    }

    /// Sets the swing amount (-75 to +75).
    ///
    /// Stored internally with a +75 offset (0 = -75, 75 = 0, 150 = +75).
    /// Values are clamped to -75..=75.
    pub fn swing(mut self, swing: i8) -> Self {
        let clamped = swing.clamp(-75, 75);
        self.seq.swing = (clamped as i16 + 75) as u8;
        self
    }

    /// Sets the default gate time (0--72, where 72 = 100%).
    pub fn default_gate_time(mut self, gate: u8) -> Self {
        self.seq.default_gate_time = gate.min(72);
        self
    }

    /// Sets a step event at `index` (0--15) with a note and velocity.
    ///
    /// This sets the first note slot of the step, enables the step in both
    /// `steps_on` and `active_steps` bitfields, and sets the default gate
    /// time with trigger enabled.
    ///
    /// If `index` is out of range (>= 16), this call is silently ignored.
    pub fn step(mut self, index: u8, note: u8, velocity: u8) -> Self {
        if let Some(step) = self.seq.steps.get_mut(index as usize) {
            step.notes[0] = note;
            step.velocities[0] = velocity;
            // Set gate time with trigger switch (bit 7) enabled.
            step.gate_times[0] = self.seq.default_gate_time | 0x80;
            self.seq.steps_on |= 1 << index;
            self.seq.active_steps |= 1 << index;
        }
        self
    }

    /// Sets a step event with up to 8 notes (polyphonic step).
    ///
    /// Notes beyond position 7 are ignored. Empty (0) notes are skipped.
    pub fn step_poly(mut self, index: u8, notes: &[(u8, u8)]) -> Self {
        if let Some(step) = self.seq.steps.get_mut(index as usize) {
            for (i, &(note, vel)) in notes.iter().take(8).enumerate() {
                step.notes[i] = note;
                step.velocities[i] = vel;
                step.gate_times[i] = self.seq.default_gate_time | 0x80;
            }
            self.seq.steps_on |= 1 << index;
            self.seq.active_steps |= 1 << index;
        }
        self
    }

    /// Sets the arpeggiator gate time (0--72).
    pub fn arp_gate_time(mut self, gate: u8) -> Self {
        self.seq.arp_gate_time = gate.min(72);
        self
    }

    /// Sets the arpeggiator rate (0--10).
    pub fn arp_rate(mut self, rate: u8) -> Self {
        self.seq.arp_rate = rate.min(10);
        self
    }

    /// Builds the final [`SequencerParams`].
    pub fn build(self) -> SequencerParams {
        self.seq
    }
}

impl Default for SequenceBuilder {
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
    fn default_build_has_120_bpm() {
        let seq = SequenceBuilder::new().build();
        assert_eq!(seq.bpm, 1200);
    }

    #[test]
    fn default_build_has_16_steps() {
        let seq = SequenceBuilder::new().build();
        assert_eq!(seq.step_length, 16);
    }

    #[test]
    fn default_build_has_sixteenth_resolution() {
        let seq = SequenceBuilder::new().build();
        assert_eq!(seq.step_resolution, 0);
    }

    #[test]
    fn bpm_sets_correctly() {
        let seq = SequenceBuilder::new().bpm(130.0).build();
        assert_eq!(seq.bpm, 1300);
    }

    #[test]
    fn bpm_clamps_low() {
        let seq = SequenceBuilder::new().bpm(5.0).build();
        assert_eq!(seq.bpm, 100); // 10.0 * 10
    }

    #[test]
    fn bpm_clamps_high() {
        let seq = SequenceBuilder::new().bpm(500.0).build();
        assert_eq!(seq.bpm, 3000); // 300.0 * 10
    }

    #[test]
    fn length_sets_correctly() {
        let seq = SequenceBuilder::new().length(8).build();
        assert_eq!(seq.step_length, 8);
    }

    #[test]
    fn length_clamps_zero_to_one() {
        let seq = SequenceBuilder::new().length(0).build();
        assert_eq!(seq.step_length, 1);
    }

    #[test]
    fn length_clamps_17_to_16() {
        let seq = SequenceBuilder::new().length(17).build();
        assert_eq!(seq.step_length, 16);
    }

    #[test]
    fn resolution_sets_correctly() {
        let seq = SequenceBuilder::new().resolution(2).build();
        assert_eq!(seq.step_resolution, 2);
    }

    #[test]
    fn resolution_clamps_high() {
        let seq = SequenceBuilder::new().resolution(10).build();
        assert_eq!(seq.step_resolution, 4);
    }

    #[test]
    fn step_sets_note_velocity_and_enables() {
        let seq = SequenceBuilder::new().step(0, 60, 100).build();
        assert_eq!(seq.steps[0].notes[0], 60);
        assert_eq!(seq.steps[0].velocities[0], 100);
        assert!(seq.steps_on & 1 != 0);
        assert!(seq.active_steps & 1 != 0);
    }

    #[test]
    fn step_sets_gate_time_with_trigger() {
        let seq = SequenceBuilder::new().step(0, 60, 100).build();
        // Default gate time is 54, plus trigger bit 0x80
        assert_eq!(seq.steps[0].gate_times[0], 54 | 0x80);
    }

    #[test]
    fn step_out_of_range_ignored() {
        let seq = SequenceBuilder::new().step(16, 60, 100).build();
        // Should not panic; steps_on should still be 0 (default)
        assert_eq!(seq.steps_on, 0);
    }

    #[test]
    fn multiple_steps() {
        let seq = SequenceBuilder::new()
            .step(0, 60, 100)
            .step(4, 64, 80)
            .step(8, 67, 90)
            .build();
        assert_eq!(seq.steps[0].notes[0], 60);
        assert_eq!(seq.steps[4].notes[0], 64);
        assert_eq!(seq.steps[8].notes[0], 67);
        assert_eq!(seq.steps_on, (1 << 0) | (1 << 4) | (1 << 8));
    }

    #[test]
    fn swing_zero() {
        let seq = SequenceBuilder::new().swing(0).build();
        assert_eq!(seq.swing, 75);
    }

    #[test]
    fn swing_positive() {
        let seq = SequenceBuilder::new().swing(50).build();
        assert_eq!(seq.swing, 125);
    }

    #[test]
    fn swing_negative() {
        let seq = SequenceBuilder::new().swing(-30).build();
        assert_eq!(seq.swing, 45);
    }

    #[test]
    fn swing_clamps() {
        let seq_hi = SequenceBuilder::new().swing(100).build();
        assert_eq!(seq_hi.swing, 150);
        let seq_lo = SequenceBuilder::new().swing(-100).build();
        assert_eq!(seq_lo.swing, 0);
    }

    #[test]
    fn default_gate_time_sets() {
        let seq = SequenceBuilder::new().default_gate_time(36).build();
        assert_eq!(seq.default_gate_time, 36);
    }

    #[test]
    fn default_gate_time_clamps() {
        let seq = SequenceBuilder::new().default_gate_time(100).build();
        assert_eq!(seq.default_gate_time, 72);
    }

    #[test]
    fn arp_gate_time_sets() {
        let seq = SequenceBuilder::new().arp_gate_time(36).build();
        assert_eq!(seq.arp_gate_time, 36);
    }

    #[test]
    fn arp_rate_sets() {
        let seq = SequenceBuilder::new().arp_rate(5).build();
        assert_eq!(seq.arp_rate, 5);
    }

    #[test]
    fn arp_rate_clamps() {
        let seq = SequenceBuilder::new().arp_rate(15).build();
        assert_eq!(seq.arp_rate, 10);
    }

    #[test]
    fn step_poly_sets_multiple_notes() {
        let seq = SequenceBuilder::new()
            .step_poly(0, &[(60, 100), (64, 80), (67, 90)])
            .build();
        assert_eq!(seq.steps[0].notes[0], 60);
        assert_eq!(seq.steps[0].notes[1], 64);
        assert_eq!(seq.steps[0].notes[2], 67);
        assert_eq!(seq.steps[0].velocities[0], 100);
        assert_eq!(seq.steps[0].velocities[1], 80);
        assert_eq!(seq.steps[0].velocities[2], 90);
    }

    #[test]
    fn chained_builder() {
        let seq = SequenceBuilder::new()
            .bpm(140.0)
            .length(8)
            .resolution(1)
            .swing(10)
            .step(0, 60, 100)
            .step(2, 64, 80)
            .build();
        assert_eq!(seq.bpm, 1400);
        assert_eq!(seq.step_length, 8);
        assert_eq!(seq.step_resolution, 1);
        assert_eq!(seq.swing, 85);
        assert_eq!(seq.steps[0].notes[0], 60);
        assert_eq!(seq.steps[2].notes[0], 64);
    }

    #[test]
    fn default_impl_matches_new() {
        let a = SequenceBuilder::new().build();
        let b = SequenceBuilder::default().build();
        assert_eq!(a.bpm, b.bpm);
        assert_eq!(a.step_length, b.step_length);
    }

    #[test]
    fn bpm_fractional() {
        let seq = SequenceBuilder::new().bpm(120.5).build();
        assert_eq!(seq.bpm, 1205);
    }
}
