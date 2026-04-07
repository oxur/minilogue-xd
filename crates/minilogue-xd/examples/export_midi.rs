//! Export the Berlin School sequence to a Standard MIDI File.
//!
//! Builds a classic Tangerine Dream style 16th-note arpeggiation pattern
//! with a Minilogue XD patch setup encoded as CC events, then writes
//! the result to `berlin_school.mid`.

use minilogue_xd::midi_file::MidiFileBuilder;
use minilogue_xd::param::enums::*;
use minilogue_xd::sysex::program::SynthParams;

fn main() -> std::io::Result<()> {
    let tpq: u64 = 480;
    let step = tpq / 4; // 16th note = 120 ticks

    // Build a SynthParams for the patch setup CCs
    let synth = SynthParams {
        vco1_wave: VcoWave::Saw,
        vco1_octave: VcoOctave::Eight,
        vco2_wave: VcoWave::Saw,
        vco2_octave: VcoOctave::Eight,
        vco2_pitch: 527, // slight detune (~+18 cents from center 512)
        cutoff: 358,     // ~0.35 * 1023
        resonance: 460,  // ~0.45 * 1023
        amp_eg_attack: 0,
        amp_eg_decay: 358,
        amp_eg_sustain: 256,
        amp_eg_release: 307,
        delay_on: true,
        delay_sub_type: DelaySubType::Tape,
        reverb_on: true,
        reverb_sub_type: ReverbSubType::Hall,
        ..SynthParams::default()
    };

    let mut builder = MidiFileBuilder::new(120.0)
        .track_name("Berlin School -- Tangerine Dream style")
        .patch_ccs(0, &synth);

    // E minor pattern (16 steps, 16th notes)
    let pattern: &[(u8, u8, f32)] = &[
        (40, 100, 0.6),
        (47, 80, 0.4),
        (52, 90, 0.5),
        (47, 70, 0.3),
        (55, 95, 0.6),
        (47, 75, 0.4),
        (52, 85, 0.5),
        (50, 70, 0.3),
        (40, 100, 0.6),
        (47, 80, 0.4),
        (55, 90, 0.5),
        (52, 70, 0.3),
        (57, 95, 0.6),
        (55, 75, 0.4),
        (52, 85, 0.5),
        (50, 65, 0.3),
    ];

    // Performance structure: (transpose, repeats, base_cutoff, cutoff_drift, vel_scale)
    let passes: &[(i8, u32, f32, f32, f32)] = &[
        // E minor — establish the theme
        (0, 4, 0.30, 0.03, 1.0),
        (0, 4, 0.42, 0.04, 1.0),
        // C minor — darker, deeper
        (-4, 4, 0.38, 0.03, 1.0),
        (-4, 4, 0.50, 0.04, 1.0),
        // E minor — return
        (0, 4, 0.50, 0.04, 1.0),
        (0, 4, 0.55, 0.03, 1.0),
        // Fade out
        (0, 2, 0.45, -0.03, 0.85),
        (0, 2, 0.38, -0.03, 0.65),
        (0, 2, 0.30, -0.03, 0.45),
        (0, 2, 0.22, -0.02, 0.25),
    ];

    let mut tick: u64 = 0;

    for &(transpose, repeats, base_cutoff, cutoff_drift, vel_scale) in passes {
        for rep in 0..repeats {
            // Cutoff automation: set at the start of each repetition
            let cutoff = (base_cutoff + cutoff_drift * rep as f32).clamp(0.0, 1.0);
            builder = builder.set_cutoff(tick, cutoff);

            // Notes for this repetition
            for &(note, vel, gate) in pattern {
                let transposed = (note as i8 + transpose).clamp(0, 127) as u8;
                let scaled_vel = ((vel as f32 * vel_scale).round() as u8).clamp(1, 127);
                let dur = (step as f32 * gate) as u64;
                builder = builder.note(tick, transposed, scaled_vel, dur);
                tick += step;
            }
        }
    }

    let midi_data = builder.build();
    std::fs::write("berlin_school.mid", &midi_data)?;
    println!(
        "Wrote berlin_school.mid ({} bytes, {} ticks)",
        midi_data.len(),
        tick
    );
    Ok(())
}
