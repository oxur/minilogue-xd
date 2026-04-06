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

    // 4 repetitions in E minor
    let mut tick: u64 = 0;
    for _ in 0..4 {
        for &(note, vel, gate) in pattern {
            let dur = (step as f32 * gate) as u64;
            builder = builder.note(tick, note, vel, dur);
            tick += step;
        }
    }

    // 4 repetitions transposed to C minor (-4 semitones)
    for _ in 0..4 {
        for &(note, vel, gate) in pattern {
            let dur = (step as f32 * gate) as u64;
            builder = builder.note(tick, note.saturating_sub(4), vel, dur);
            tick += step;
        }
    }

    // 4 repetitions back in E minor
    for _ in 0..4 {
        for &(note, vel, gate) in pattern {
            let dur = (step as f32 * gate) as u64;
            builder = builder.note(tick, note, vel, dur);
            tick += step;
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
