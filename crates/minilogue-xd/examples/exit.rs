//! Tangerine Dream "Exit" (1981) style — two-voice performance.
//!
//! Voice A ("The Machine"): aggressive square-wave sequence in D minor,
//! with tight filter, high resonance, and stereo delay.
//!
//! Voice B ("The Atmosphere"): slow-evolving triangle pad with deep
//! space reverb, entering gradually above the sequence.
//!
//! The XD's 4-voice polyphony lets both voices play simultaneously —
//! the sequence uses 1 voice per note while the pad holds chords with
//! the remaining 3 voices.

use std::thread::sleep;
use std::time::Duration;

use minilogue_xd::controller::RealtimeController;
use minilogue_xd::device;
use minilogue_xd::message::{U4, U7};
use minilogue_xd::param::enums::*;
use minilogue_xd::theory::chord::Chord;
use minilogue_xd::theory::note::{Note, Notes, Pitch, PitchSymbol};
use minilogue_xd::transport::MidirOutput;

fn n(sym: PitchSymbol, oct: u8) -> u8 {
    Note::new(Pitch::from(sym), oct).midi_pitch()
}

/// Configure Voice A — "The Machine" (aggressive sequencer voice).
fn setup_voice_a<O: minilogue_xd::transport::MidiOutput>(
    xd: &mut RealtimeController<O>,
) -> minilogue_xd::Result<()> {
    // Square wave — harder, more aggressive than saw
    xd.set_vco1_wave(VcoWave::Sqr)?;
    xd.set_vco1_octave(VcoOctave::Eight)?;
    xd.set_vco1_pitch(0.5)?;
    xd.set_vco1_shape(0.35)?; // pulse width slightly off center

    // Second square, detuned for thickness
    xd.set_vco2_wave(VcoWave::Sqr)?;
    xd.set_vco2_octave(VcoOctave::Eight)?;
    xd.set_vco2_pitch(0.512)?; // ~15 cents sharp
    xd.set_vco2_shape(0.4)?;

    xd.set_vco1_level(0.85)?;
    xd.set_vco2_level(0.6)?;

    // Tight filter, prominent resonance — menacing
    xd.set_cutoff(0.30)?;
    xd.set_resonance(0.55)?; // singing, almost dangerous
    xd.set_cutoff_drive(CutoffDrive::Half)?;
    xd.set_cutoff_keytrack(CutoffKeytrack::Half)?;

    // Percussive amp — sharp attack, low sustain for rhythmic pulse
    xd.set_amp_eg_attack(0.0)?;
    xd.set_amp_eg_decay(0.30)?;
    xd.set_amp_eg_sustain(0.15)?; // very low — staccato machine
    xd.set_amp_eg_release(0.20)?;

    // Filter EG — aggressive snap on each note
    xd.set_eg_attack(0.0)?;
    xd.set_eg_decay(0.25)?;
    xd.set_eg_int(0.65)?; // deep sweep
    xd.set_eg_target(EgTarget::Cutoff)?;

    // LFO — slow, subtle pitch wobble (analog drift simulation)
    xd.set_lfo_wave(LfoWave::Tri)?;
    xd.set_lfo_mode(LfoMode::Normal)?;
    xd.set_lfo_rate(0.08)?;
    xd.set_lfo_int(0.06)?; // barely perceptible
    xd.set_lfo_target(LfoTarget::Pitch)?;

    // Stereo delay — mechanical, rhythmic
    xd.set_delay_on(true)?;
    xd.set_delay_sub_type(DelaySubType::Stereo)?;
    xd.set_delay_time(0.33)?;
    xd.set_delay_depth(0.5)?;
    xd.set_delay_dry_wet(0.30)?;

    // Reverb — small room, not too wet
    xd.set_reverb_on(true)?;
    xd.set_reverb_sub_type(ReverbSubType::Room)?;
    xd.set_reverb_time(0.35)?;
    xd.set_reverb_depth(0.3)?;
    xd.set_reverb_dry_wet(0.15)?;

    Ok(())
}

/// Configure Voice B — "The Atmosphere" (evolving pad, standalone).
fn setup_voice_b<O: minilogue_xd::transport::MidiOutput>(
    xd: &mut RealtimeController<O>,
) -> minilogue_xd::Result<()> {
    xd.set_vco1_wave(VcoWave::Tri)?;
    xd.set_vco1_octave(VcoOctave::Eight)?;
    xd.set_vco1_pitch(0.5)?;
    xd.set_vco1_shape(0.6)?;
    xd.set_vco2_wave(VcoWave::Tri)?;
    xd.set_vco2_octave(VcoOctave::Four)?;
    xd.set_vco2_pitch(0.504)?;
    xd.set_vco2_shape(0.5)?;
    xd.set_vco1_level(0.75)?;
    xd.set_vco2_level(0.5)?;
    xd.set_cutoff(0.55)?;
    xd.set_resonance(0.15)?;
    xd.set_cutoff_drive(CutoffDrive::Off)?;
    xd.set_cutoff_keytrack(CutoffKeytrack::Half)?;
    xd.set_amp_eg_attack(0.6)?;
    xd.set_amp_eg_decay(0.3)?;
    xd.set_amp_eg_sustain(0.8)?;
    xd.set_amp_eg_release(0.7)?;
    xd.set_eg_attack(0.4)?;
    xd.set_eg_decay(0.5)?;
    xd.set_eg_int(0.2)?;
    xd.set_eg_target(EgTarget::Cutoff)?;
    xd.set_lfo_wave(LfoWave::Tri)?;
    xd.set_lfo_mode(LfoMode::Normal)?;
    xd.set_lfo_rate(0.03)?;
    xd.set_lfo_int(0.2)?;
    xd.set_lfo_target(LfoTarget::Cutoff)?;
    xd.set_delay_on(false)?;
    xd.set_reverb_on(true)?;
    xd.set_reverb_sub_type(ReverbSubType::Space)?;
    xd.set_reverb_time(0.85)?;
    xd.set_reverb_depth(0.7)?;
    xd.set_reverb_dry_wet(0.5)?;
    Ok(())
}

/// Configure Hybrid voice — sequence + pad from one patch.
///
/// The trick: moderate amp sustain (0.4) lets short-gate notes pulse
/// while held notes sustain. Full keytrack makes low notes dark (sequence)
/// and high notes bright (pad). Saw + Triangle mix gives both edge and warmth.
fn setup_hybrid<O: minilogue_xd::transport::MidiOutput>(
    xd: &mut RealtimeController<O>,
) -> minilogue_xd::Result<()> {
    // Saw for edge (sequence), Triangle for warmth (pad)
    xd.set_vco1_wave(VcoWave::Saw)?;
    xd.set_vco1_octave(VcoOctave::Eight)?;
    xd.set_vco1_pitch(0.5)?;
    xd.set_vco1_shape(0.2)?;

    xd.set_vco2_wave(VcoWave::Tri)?;
    xd.set_vco2_octave(VcoOctave::Eight)?;
    xd.set_vco2_pitch(0.508)?; // slight detune
    xd.set_vco2_shape(0.5)?;

    xd.set_vco1_level(0.7)?;
    xd.set_vco2_level(0.65)?;

    // Filter: moderate, FULL keytrack — low notes dark, high notes bright
    xd.set_cutoff(0.38)?;
    xd.set_resonance(0.40)?;
    xd.set_cutoff_drive(CutoffDrive::Off)?;
    xd.set_cutoff_keytrack(CutoffKeytrack::Full)?; // key to two-voice illusion

    // Amp EG: moderate sustain — pulses for short gates, sustains for holds
    xd.set_amp_eg_attack(0.02)?; // near-instant for sequence articulation
    xd.set_amp_eg_decay(0.35)?;
    xd.set_amp_eg_sustain(0.4)?; // the sweet spot: pulse AND sustain
    xd.set_amp_eg_release(0.5)?; // enough tail for pad notes to overlap

    // Filter EG — moderate, adds articulation to sequence hits
    xd.set_eg_attack(0.0)?;
    xd.set_eg_decay(0.30)?;
    xd.set_eg_int(0.45)?;
    xd.set_eg_target(EgTarget::Cutoff)?;

    // Slow LFO on cutoff — evolves both voices together
    xd.set_lfo_wave(LfoWave::Tri)?;
    xd.set_lfo_mode(LfoMode::Normal)?;
    xd.set_lfo_rate(0.04)?;
    xd.set_lfo_int(0.15)?;
    xd.set_lfo_target(LfoTarget::Cutoff)?;

    // Stereo delay — rhythmic echoes on the sequence
    xd.set_delay_on(true)?;
    xd.set_delay_sub_type(DelaySubType::Stereo)?;
    xd.set_delay_time(0.33)?;
    xd.set_delay_depth(0.45)?;
    xd.set_delay_dry_wet(0.25)?;

    // Space reverb — cinematic depth for the pad
    xd.set_reverb_on(true)?;
    xd.set_reverb_sub_type(ReverbSubType::Space)?;
    xd.set_reverb_time(0.7)?;
    xd.set_reverb_depth(0.6)?;
    xd.set_reverb_dry_wet(0.35)?;

    Ok(())
}

fn main() -> minilogue_xd::Result<()> {
    println!("=== Tangerine Dream — \"Exit\" Style ===");
    println!("    Two voices: The Machine + The Atmosphere\n");

    let Some(port_name) = device::find_output_port()? else {
        eprintln!("Minilogue XD not found — is it connected via USB?");
        std::process::exit(1);
    };
    let output = MidirOutput::connect(&port_name)?;
    let channel = U4::new(0)?;
    let mut xd = RealtimeController::new(output, channel);

    let bpm: f64 = 130.0; // slightly faster than Phaedra — Exit is more driven
    let step_dur = Duration::from_secs_f64(60.0 / bpm / 4.0);

    use PitchSymbol::*;

    // D minor sequence pattern — angular, mechanical, Kiew Mission-inspired
    // Root-fifth pattern with chromatic tension (Bb, C#)
    let sequence: Vec<(u8, u8, f32)> = vec![
        (n(D, 2), 110, 0.5), // root — hard
        (n(A, 2), 75, 0.3),  // fifth
        (n(D, 3), 95, 0.5),  // octave
        (n(F, 3), 80, 0.3),  // minor third
        (n(A, 2), 100, 0.5), // fifth — accent
        (n(Bb, 2), 70, 0.3), // b6 — dark tension
        (n(D, 3), 90, 0.4),  // octave
        (n(Cs, 3), 65, 0.3), // leading tone — chromatic edge
        (n(D, 2), 110, 0.5), // root
        (n(A, 2), 75, 0.3),  // fifth
        (n(F, 3), 95, 0.5),  // minor third — higher
        (n(D, 3), 70, 0.3),  // octave
        (n(E, 3), 100, 0.5), // second — pushing forward
        (n(F, 3), 80, 0.4),  // minor third
        (n(D, 3), 85, 0.4),  // home
        (n(A, 2), 60, 0.2),  // fifth — ghostly
    ];

    // Pad chords — Dm → Bbmaj → Gm → Dm (cinematic minor progression)
    let pad_chords: Vec<(&str, Chord)> = vec![
        ("Dm", Chord::from_regex("Dm").unwrap()),
        ("Bb", Chord::from_regex("Bb").unwrap()),
        ("Gm", Chord::from_regex("Gm").unwrap()),
        ("Dm", Chord::from_regex("Dm").unwrap()),
    ];

    // === Act I: The Machine Awakens ===
    println!("  Act I: The Machine awakens...");
    setup_voice_a(&mut xd)?;
    sleep(Duration::from_millis(200));

    // Sequence alone — 4 passes with rising filter
    for rep in 0..4u32 {
        let cutoff = 0.25 + 0.04 * rep as f32;
        xd.set_cutoff(cutoff)?;

        for &(note, vel, gate) in &sequence {
            xd.play_note(U7::new(note)?, U7::new(vel)?)?;
            sleep(step_dur.mul_f32(gate));
            xd.stop_note(U7::new(note)?)?;
            sleep(step_dur.mul_f32(1.0 - gate));
        }
    }

    // === Act II: The Atmosphere Enters (standalone pad) ===
    println!("  Act II: The Atmosphere enters...");
    xd.all_notes_off()?;
    sleep(Duration::from_millis(500));

    setup_voice_b(&mut xd)?;
    sleep(Duration::from_millis(300));

    for (name, chord) in &pad_chords {
        let midi_notes: Vec<u8> = chord.notes().iter().map(|nn| nn.midi_pitch()).collect();
        println!("    {name}...");

        for &note in &midi_notes {
            xd.play_note(U7::new(note)?, U7::new(70)?)?;
            sleep(Duration::from_millis(30));
        }

        sleep(Duration::from_secs(3));

        for &note in &midi_notes {
            xd.stop_note(U7::new(note)?)?;
        }
        sleep(Duration::from_millis(500));
    }

    // === Act III: Both Voices Together ===
    // Hybrid patch: sequence in low register + pad chords held in high register.
    // Full keytrack makes low notes dark (percussive) and high notes bright (airy).
    // The XD's 4-voice poly handles both — sequence uses 1 voice, pad uses 3.
    println!("  Act III: Both voices converge...");
    xd.all_notes_off()?;
    sleep(Duration::from_millis(300));

    setup_hybrid(&mut xd)?;
    sleep(Duration::from_millis(200));

    // Play 4 chord changes, each lasting 2 sequence passes.
    // Pad chord is held while sequence runs underneath.
    for (chord_idx, (name, chord)) in pad_chords.iter().enumerate() {
        // Shift pad chords up an octave so they sit above the sequence
        let pad_notes: Vec<u8> = chord
            .notes()
            .iter()
            .map(|nn| nn.midi_pitch() + 12) // one octave up
            .collect();

        println!("    {name} (sequence + pad)...");

        // Start the pad chord (held)
        for &note in &pad_notes {
            xd.play_note(U7::new(note.min(127))?, U7::new(55)?)?; // soft pad
            sleep(Duration::from_millis(20));
        }

        // Run 2 sequence passes over the held pad chord
        for rep in 0..2u32 {
            let cutoff = (0.35 + 0.04 * (chord_idx * 2 + rep as usize) as f32).min(0.65);
            xd.set_cutoff(cutoff)?;

            for &(note, vel, gate) in &sequence {
                xd.play_note(U7::new(note)?, U7::new(vel)?)?;
                sleep(step_dur.mul_f32(gate));
                xd.stop_note(U7::new(note)?)?;
                sleep(step_dur.mul_f32(1.0 - gate));
            }
        }

        // Release the pad chord
        for &note in &pad_notes {
            xd.stop_note(U7::new(note.min(127))?)?;
        }
        sleep(Duration::from_millis(200));
    }

    // === Fadeout: The Machine winds down ===
    println!("  Fading...");
    for pass in 0..4u32 {
        let vel_scale = 1.0 - (pass as f32 * 0.25);
        let cutoff = 0.35 - (pass as f32 * 0.07);
        xd.set_cutoff(cutoff.max(0.0))?;

        for &(note, vel, gate) in &sequence {
            let scaled = ((vel as f32 * vel_scale).round() as u8).clamp(1, 127);
            xd.play_note(U7::new(note)?, U7::new(scaled)?)?;
            sleep(step_dur.mul_f32(gate));
            xd.stop_note(U7::new(note)?)?;
            sleep(step_dur.mul_f32(1.0 - gate));
        }
    }

    xd.all_notes_off()?;
    println!("\n  Reverb tail...");
    sleep(Duration::from_secs(4));

    println!("\n=== Exit ===");
    Ok(())
}
