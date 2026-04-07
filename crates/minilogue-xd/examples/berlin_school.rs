//! Berlin School sequencer — Tangerine Dream style.
//!
//! Sets up a classic analog-sounding patch (resonant saw, slow filter sweep,
//! tape echo) and drives it with a pulsing 16th-note sequence in E minor
//! reminiscent of Phaedra / Rubycon / Stratosfear era TD.
//!
//! The sequence evolves over multiple passes, gradually opening the filter
//! and shifting octaves — the hallmark Berlin School technique of building
//! tension through subtle repetitive variation.

use std::thread::sleep;
use std::time::Duration;

use minilogue_xd::controller::RealtimeController;
use minilogue_xd::device;
use minilogue_xd::message::{U4, U7};
use minilogue_xd::param::enums::{
    CutoffDrive, CutoffKeytrack, DelaySubType, EgTarget, LfoMode, LfoTarget, LfoWave,
    ReverbSubType, VcoOctave, VcoWave,
};
use minilogue_xd::theory::note::{Note, Pitch, PitchSymbol};
use minilogue_xd::transport::MidirOutput;

/// Shorthand: create a MIDI note number from a pitch symbol and octave.
fn n(sym: PitchSymbol, oct: u8) -> u8 {
    Note::new(Pitch::from(sym), oct).midi_pitch()
}

/// A single step in the sequence: MIDI note number and velocity.
/// Velocity 0 = rest (silent step).
struct Step {
    note: u8,
    vel: u8,
    /// Gate time as fraction of step duration (0.0–1.0).
    gate: f32,
}

impl Step {
    fn note(note: u8, vel: u8, gate: f32) -> Self {
        Self { note, vel, gate }
    }
    fn rest() -> Self {
        Self {
            note: 0,
            vel: 0,
            gate: 0.0,
        }
    }
}

fn main() -> minilogue_xd::Result<()> {
    println!("=== Berlin School Sequencer ===");
    println!("    Tangerine Dream style\n");

    let port_name =
        device::find_output_port()?.expect("Minilogue XD not found — is it connected via USB?");
    let output = MidirOutput::connect(&port_name)?;
    let channel = U4::new(0)?;
    let mut xd = RealtimeController::new(output, channel);

    // ------------------------------------------------------------------
    // Patch: classic Berlin School lead
    // ------------------------------------------------------------------
    println!("Setting up patch...");

    // VCO 1: Sawtooth, the bread and butter of Berlin School
    xd.set_vco1_wave(VcoWave::Saw)?;
    xd.set_vco1_octave(VcoOctave::Eight)?;
    xd.set_vco1_pitch(0.5)?; // centered
    xd.set_vco1_shape(0.0)?; // pure saw

    // VCO 2: Slightly detuned saw for fatness
    xd.set_vco2_wave(VcoWave::Saw)?;
    xd.set_vco2_octave(VcoOctave::Eight)?;
    xd.set_vco2_pitch(0.515)?; // ~+18 cents detune — classic analog drift
    xd.set_vco2_shape(0.0)?;

    // Mixer: VCO1 dominant, VCO2 adds body
    xd.set_vco1_level(0.85)?;
    xd.set_vco2_level(0.55)?;

    // Filter: slightly closed, moderate resonance — will sweep this live
    xd.set_cutoff(0.35)?;
    xd.set_resonance(0.45)?; // enough to sing, not enough to squeal
    xd.set_cutoff_drive(CutoffDrive::Off)?;
    xd.set_cutoff_keytrack(CutoffKeytrack::Half)?;

    // Amp EG: snappy attack, moderate decay, low sustain — percussive
    xd.set_amp_eg_attack(0.0)?; // instant
    xd.set_amp_eg_decay(0.35)?; // moderate
    xd.set_amp_eg_sustain(0.25)?; // low — lets the sequence pulse
    xd.set_amp_eg_release(0.30)?; // medium release for legato feel

    // Filter EG: opens the filter on each note, then closes
    xd.set_eg_attack(0.0)?;
    xd.set_eg_decay(0.40)?;
    xd.set_eg_int(0.55)?; // moderate sweep depth
    xd.set_eg_target(EgTarget::Cutoff)?;

    // LFO: very slow triangle on cutoff — the long-term sweep
    xd.set_lfo_wave(LfoWave::Tri)?;
    xd.set_lfo_mode(LfoMode::Normal)?;
    xd.set_lfo_rate(0.05)?; // very slow — one cycle over many bars
    xd.set_lfo_int(0.15)?; // subtle
    xd.set_lfo_target(LfoTarget::Cutoff)?;

    // Delay: tape echo — essential Berlin School
    xd.set_delay_on(true)?;
    xd.set_delay_sub_type(DelaySubType::Tape)?;
    xd.set_delay_time(0.37)?; // dotted-eighth feel against 16ths
    xd.set_delay_depth(0.55)?; // prominent echoes
    xd.set_delay_dry_wet(0.35)?;

    // Reverb: spacious hall
    xd.set_reverb_on(true)?;
    xd.set_reverb_sub_type(ReverbSubType::Hall)?;
    xd.set_reverb_time(0.65)?;
    xd.set_reverb_depth(0.5)?;
    xd.set_reverb_dry_wet(0.25)?;

    sleep(Duration::from_millis(300));

    // ------------------------------------------------------------------
    // Sequence: E minor, 16th notes, classic TD pattern
    // ------------------------------------------------------------------

    // BPM and timing
    let bpm: f64 = 120.0;
    let step_dur = Duration::from_secs_f64(60.0 / bpm / 4.0); // 16th notes

    // Classic Berlin School pattern in E minor (E2–E4 range)
    // The pattern has a hypnotic, interlocking quality — root and fifth
    // with passing tones and octave leaps that create the illusion of
    // multiple voices from a single sequence line.
    // Named notes — no magic MIDI numbers!
    use PitchSymbol::*;

    let pattern_a: Vec<Step> = vec![
        Step::note(n(E, 2), 100, 0.6), // root anchor
        Step::note(n(B, 2), 80, 0.4),  // fifth
        Step::note(n(E, 3), 90, 0.5),  // octave
        Step::note(n(B, 2), 70, 0.3),  // fifth echo
        Step::note(n(G, 3), 95, 0.6),  // minor third
        Step::note(n(B, 2), 75, 0.4),  // fifth
        Step::note(n(E, 3), 85, 0.5),  // octave
        Step::note(n(D, 3), 70, 0.3),  // passing tone (VII)
        Step::note(n(E, 2), 100, 0.6), // root
        Step::note(n(B, 2), 80, 0.4),  // fifth
        Step::note(n(G, 3), 90, 0.5),  // minor third
        Step::note(n(E, 3), 70, 0.3),  // octave
        Step::note(n(A, 3), 95, 0.6),  // fourth (tension)
        Step::note(n(G, 3), 75, 0.4),  // resolve
        Step::note(n(E, 3), 85, 0.5),  // home
        Step::note(n(D, 3), 65, 0.3),  // leading back
    ];

    // Second pattern: higher register, more open
    let pattern_b: Vec<Step> = vec![
        Step::note(n(E, 3), 100, 0.6),
        Step::note(n(B, 3), 80, 0.4),
        Step::note(n(E, 4), 90, 0.5),
        Step::note(n(B, 3), 70, 0.3),
        Step::note(n(G, 4), 95, 0.6),
        Step::note(n(B, 3), 75, 0.4),
        Step::note(n(E, 4), 85, 0.5),
        Step::note(n(D, 4), 70, 0.3),
        Step::note(n(E, 3), 100, 0.6),
        Step::note(n(B, 3), 80, 0.4),
        Step::note(n(G, 4), 90, 0.5),
        Step::note(n(E, 4), 70, 0.3),
        Step::note(n(A, 4), 95, 0.6),
        Step::note(n(G, 4), 75, 0.4),
        Step::note(n(E, 4), 85, 0.5),
        Step::rest(), // breathing room
    ];

    // ------------------------------------------------------------------
    // Performance: evolve the sound over multiple passes
    // ------------------------------------------------------------------

    println!("Sequencing...\n");
    println!("  Pattern A — low register, filter closed");

    // Transpose a pattern down by `semitones` (creates a new Vec).
    let transpose = |pattern: &[Step], semitones: i8| -> Vec<Step> {
        pattern
            .iter()
            .map(|s| {
                if s.vel > 0 {
                    Step::note(
                        (s.note as i8 + semitones).clamp(0, 127) as u8,
                        s.vel,
                        s.gate,
                    )
                } else {
                    Step::rest()
                }
            })
            .collect()
    };

    // C minor variants: down a major third (-4 semitones) from E minor
    let pattern_a_cm = transpose(&pattern_a, -4);
    let pattern_b_cm = transpose(&pattern_b, -4);

    // Each pass: pattern, repeats, base_cutoff, cutoff_drift, velocity_scale, label
    // velocity_scale is multiplied against each step's velocity (1.0 = full)
    struct Pass<'a> {
        pattern: &'a [Step],
        repeats: u32,
        base_cutoff: f32,
        cutoff_drift: f32,
        vel_scale: f32,
        label: &'static str,
    }

    let passes: Vec<Pass> = vec![
        // E minor — establish the theme
        Pass {
            pattern: &pattern_a,
            repeats: 4,
            base_cutoff: 0.30,
            cutoff_drift: 0.03,
            vel_scale: 1.0,
            label: "E minor — low register, filter rising",
        },
        Pass {
            pattern: &pattern_a,
            repeats: 4,
            base_cutoff: 0.42,
            cutoff_drift: 0.04,
            vel_scale: 1.0,
            label: "E minor — low, filter opening",
        },
        Pass {
            pattern: &pattern_b,
            repeats: 4,
            base_cutoff: 0.45,
            cutoff_drift: 0.03,
            vel_scale: 1.0,
            label: "E minor — high register enters",
        },
        Pass {
            pattern: &pattern_b,
            repeats: 4,
            base_cutoff: 0.55,
            cutoff_drift: 0.05,
            vel_scale: 1.0,
            label: "E minor — high, filter wide",
        },
        // Modulate to C minor — darker, deeper
        Pass {
            pattern: &pattern_a_cm,
            repeats: 4,
            base_cutoff: 0.38,
            cutoff_drift: 0.03,
            vel_scale: 1.0,
            label: "C minor — the key drops, new territory",
        },
        Pass {
            pattern: &pattern_a_cm,
            repeats: 4,
            base_cutoff: 0.50,
            cutoff_drift: 0.04,
            vel_scale: 1.0,
            label: "C minor — low, filter opening",
        },
        Pass {
            pattern: &pattern_b_cm,
            repeats: 4,
            base_cutoff: 0.48,
            cutoff_drift: 0.03,
            vel_scale: 1.0,
            label: "C minor — high register",
        },
        Pass {
            pattern: &pattern_b_cm,
            repeats: 4,
            base_cutoff: 0.58,
            cutoff_drift: 0.05,
            vel_scale: 1.0,
            label: "C minor — high, filter wide",
        },
        // Return to E minor — homecoming
        Pass {
            pattern: &pattern_a,
            repeats: 4,
            base_cutoff: 0.50,
            cutoff_drift: 0.04,
            vel_scale: 1.0,
            label: "E minor — return, filter bright",
        },
        Pass {
            pattern: &pattern_b,
            repeats: 4,
            base_cutoff: 0.55,
            cutoff_drift: 0.03,
            vel_scale: 1.0,
            label: "E minor — high, triumphant",
        },
        // Fade: velocity drops across 6 reps while filter closes
        Pass {
            pattern: &pattern_a,
            repeats: 2,
            base_cutoff: 0.45,
            cutoff_drift: -0.03,
            vel_scale: 0.85,
            label: "fading — pulling back...",
        },
        Pass {
            pattern: &pattern_a,
            repeats: 2,
            base_cutoff: 0.38,
            cutoff_drift: -0.03,
            vel_scale: 0.65,
            label: "fading — quieter...",
        },
        Pass {
            pattern: &pattern_a,
            repeats: 2,
            base_cutoff: 0.30,
            cutoff_drift: -0.03,
            vel_scale: 0.45,
            label: "fading — distant...",
        },
        Pass {
            pattern: &pattern_a,
            repeats: 2,
            base_cutoff: 0.22,
            cutoff_drift: -0.02,
            vel_scale: 0.25,
            label: "fading — almost gone...",
        },
    ];

    for pass in &passes {
        println!("  {}", pass.label);

        for rep in 0..pass.repeats {
            let cutoff = (pass.base_cutoff + pass.cutoff_drift * rep as f32).clamp(0.0, 1.0);
            xd.set_cutoff(cutoff)?;

            for step in pass.pattern {
                if step.vel > 0 {
                    // Scale velocity for fade — clamp to 1..127
                    let vel = ((step.vel as f32 * pass.vel_scale).round() as u8).clamp(1, 127);
                    xd.play_note(U7::new(step.note)?, U7::new(vel)?)?;

                    let gate_time = step_dur.mul_f32(step.gate);
                    sleep(gate_time);
                    xd.stop_note(U7::new(step.note)?)?;

                    let rest_time = step_dur.saturating_sub(gate_time);
                    if !rest_time.is_zero() {
                        sleep(rest_time);
                    }
                } else {
                    sleep(step_dur);
                }
            }
        }
    }

    // Let the delay/reverb tail ring out naturally
    xd.all_notes_off()?;
    println!("\n  Reverb tail...");
    sleep(Duration::from_secs(4));

    println!("\n=== Sequence complete ===");
    Ok(())
}
