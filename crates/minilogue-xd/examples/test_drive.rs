//! Test drive: connect to the Minilogue XD and play some notes!
//!
//! This example connects to the synth, tweaks some parameters,
//! and plays a short sequence to verify everything works.

use std::thread::sleep;
use std::time::Duration;

use minilogue_xd::controller::RealtimeController;
use minilogue_xd::device;
use minilogue_xd::message::U4;
use minilogue_xd::message::U7;
use minilogue_xd::param::enums::{
    CutoffDrive, CutoffKeytrack, DelaySubType, LfoMode, LfoTarget, LfoWave, ReverbSubType,
    VcoOctave, VcoWave,
};
use minilogue_xd::theory::chord::Chord;
use minilogue_xd::theory::note::Notes;
use minilogue_xd::transport::MidirOutput;

fn main() -> minilogue_xd::Result<()> {
    println!("Connecting to Minilogue XD...");
    let port_name =
        device::find_output_port()?.expect("Minilogue XD not found — is it connected via USB?");
    let output = MidirOutput::connect(&port_name)?;
    let channel = U4::new(0)?;
    let mut xd = RealtimeController::new(output, channel);

    println!("Connected! Setting up a pad sound...\n");

    // --- Set up a lush pad patch ---

    // VCO 1: Saw wave, 8' octave, centered pitch, moderate shape
    xd.set_vco1_wave(VcoWave::Saw)?;
    xd.set_vco1_octave(VcoOctave::Eight)?;
    xd.set_vco1_pitch(0.5)?;
    xd.set_vco1_shape(0.3)?;

    // VCO 2: Triangle wave, 8' octave, slightly detuned
    xd.set_vco2_wave(VcoWave::Tri)?;
    xd.set_vco2_octave(VcoOctave::Eight)?;
    xd.set_vco2_pitch(0.52)?; // slight detune for width
    xd.set_vco2_shape(0.5)?;

    // Mixer: both oscillators up
    xd.set_vco1_level(0.8)?;
    xd.set_vco2_level(0.7)?;

    // Filter: open-ish, with some resonance
    xd.set_cutoff(0.6)?;
    xd.set_resonance(0.25)?;
    xd.set_cutoff_drive(CutoffDrive::Off)?;
    xd.set_cutoff_keytrack(CutoffKeytrack::Half)?;

    // Amp EG: slow attack, long release for a pad
    xd.set_amp_eg_attack(0.4)?;
    xd.set_amp_eg_decay(0.5)?;
    xd.set_amp_eg_sustain(0.7)?;
    xd.set_amp_eg_release(0.6)?;

    // LFO: slow triangle modulating the cutoff
    xd.set_lfo_wave(LfoWave::Tri)?;
    xd.set_lfo_mode(LfoMode::Normal)?;
    xd.set_lfo_rate(0.15)?;
    xd.set_lfo_int(0.2)?;
    xd.set_lfo_target(LfoTarget::Cutoff)?;

    // Effects: tape delay + hall reverb
    xd.set_delay_on(true)?;
    xd.set_delay_sub_type(DelaySubType::Tape)?;
    xd.set_delay_time(0.4)?;
    xd.set_delay_depth(0.5)?;
    xd.set_delay_dry_wet(0.3)?;

    xd.set_reverb_on(true)?;
    xd.set_reverb_sub_type(ReverbSubType::Hall)?;
    xd.set_reverb_time(0.7)?;
    xd.set_reverb_depth(0.6)?;
    xd.set_reverb_dry_wet(0.4)?;

    println!("Patch configured! Playing a chord progression...\n");
    sleep(Duration::from_millis(200));

    // --- Play a Cm9 → Fm7 → Gm7 → Cm progression ---
    // Built from music theory — no hardcoded MIDI numbers!

    let chords: Vec<(&str, Chord)> = vec![
        ("Cm9", Chord::from_regex("Cm9").unwrap()),
        ("Fm7", Chord::from_regex("Fm7").unwrap()),
        ("Gm7", Chord::from_regex("Gm7").unwrap()),
        ("Cm", Chord::from_regex("Cm").unwrap()),
    ];

    for (name, chord) in &chords {
        println!("  Playing {name} ({})...", chord.format_notes());

        // Notes on — get MIDI pitches from the chord's notes
        let midi_notes: Vec<u8> = chord.notes().iter().map(|n| n.midi_pitch()).collect();
        for &note in &midi_notes {
            xd.play_note(U7::new(note)?, U7::new(90)?)?;
            sleep(Duration::from_millis(20)); // slight strum
        }

        sleep(Duration::from_millis(1500));

        // Notes off
        for &note in &midi_notes {
            xd.stop_note(U7::new(note)?)?;
        }

        sleep(Duration::from_millis(300));
    }

    // Let the reverb tail ring out
    println!("\n  Letting the reverb tail fade...");
    sleep(Duration::from_secs(3));

    println!("\nDone! The Minilogue XD is alive and well.");
    Ok(())
}
