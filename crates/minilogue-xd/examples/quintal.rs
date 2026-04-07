//! Quintal Harmony — Vangelis-inspired crystalline pad with pentatonic melody.
//!
//! Explores quintal chord inversions using Tymoczko's interscalar transposition,
//! navigating from C quintal through Eb and Ab before returning home via a
//! descending inversion coda.
//!
//! The pad arpeggiates four-note quintal chords as slow half notes while a
//! C-pentatonic melody floats above, incorporating passing tones (Bb, Eb, Ab)
//! as the harmony modulates.
//!
//! Harmonic arc:
//!   C root → C 1st inv → (passing) → C 2nd inv → Eb 1st inv
//!   → Ab root → C 3rd inv → coda (2nd → 1st → root + fade)
//!
//! Theory: see "Quintal Harmony as a Fiber Bundle" (McGreggor, 2026)
//! for the voice-leading geometry and inversion framework.
//!
//! Usage:
//!   cargo run -p minilogue-xd --example quintal              # play on hardware
//!   cargo run -p minilogue-xd --example quintal -- --export  # export MIDI file
//!   cargo run -p minilogue-xd --example quintal -- --save    # load patch into edit buffer
//!   cargo run -p minilogue-xd --example quintal -- --save N  # save patch to slot N+1

use std::collections::HashMap;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use minilogue_xd::controller::RealtimeController;
use minilogue_xd::device;
use minilogue_xd::message::{U4, U7};
use minilogue_xd::midi_file::MidiFileBuilder;
use minilogue_xd::param::enums::*;
use minilogue_xd::sysex::program::SynthParams;
use minilogue_xd::theory::note::{Note, Pitch, PitchSymbol};
use minilogue_xd::transport::MidirOutput;

/// MIDI note number from pitch symbol and octave.
fn n(sym: PitchSymbol, oct: u8) -> u8 {
    Note::new(Pitch::from(sym), oct).midi_pitch()
}

// -----------------------------------------------------------------------
// Score event system — build the timeline declaratively, then play or export
// -----------------------------------------------------------------------

#[derive(Clone)]
enum Event {
    NoteOn { time: u64, note: u8, vel: u8 },
    NoteOff { time: u64, note: u8 },
    Cutoff { time: u64, val: f32 },
    Label { time: u64, text: &'static str },
}

impl Event {
    fn time(&self) -> u64 {
        match self {
            Event::NoteOn { time, .. }
            | Event::NoteOff { time, .. }
            | Event::Cutoff { time, .. }
            | Event::Label { time, .. } => *time,
        }
    }

    /// Sort priority: labels first, then automation, then note-offs, then note-ons.
    fn priority(&self) -> u8 {
        match self {
            Event::Label { .. } => 0,
            Event::Cutoff { .. } => 1,
            Event::NoteOff { .. } => 2,
            Event::NoteOn { .. } => 3,
        }
    }
}

/// Add a pad arp cycle: 4 notes as half notes, bottom to top.
/// Returns the time after the cycle.
fn add_arp(events: &mut Vec<Event>, start: u64, chord: &[u8; 4], half: u64, vel: u8) -> u64 {
    let gate = (half as f64 * 0.8) as u64;
    let mut t = start;
    for &note in chord {
        events.push(Event::NoteOn { time: t, note, vel });
        events.push(Event::NoteOff {
            time: t + gate,
            note,
        });
        t += half;
    }
    t
}

/// Add a fast arp cycle: 4 notes as quarter notes (for passing chords).
fn add_fast_arp(
    events: &mut Vec<Event>,
    start: u64,
    chord: &[u8; 4],
    quarter: u64,
    vel: u8,
) -> u64 {
    let gate = (quarter as f64 * 0.75) as u64;
    let mut t = start;
    for &note in chord {
        events.push(Event::NoteOn { time: t, note, vel });
        events.push(Event::NoteOff {
            time: t + gate,
            note,
        });
        t += quarter;
    }
    t
}

/// Add a descending arp cycle: 4 notes as half notes, top to bottom.
fn add_arp_desc(events: &mut Vec<Event>, start: u64, chord: &[u8; 4], half: u64, vel: u8) -> u64 {
    let gate = (half as f64 * 0.8) as u64;
    let mut t = start;
    for &note in chord.iter().rev() {
        events.push(Event::NoteOn { time: t, note, vel });
        events.push(Event::NoteOff {
            time: t + gate,
            note,
        });
        t += half;
    }
    t
}

/// Add a melody note.
fn add_mel(events: &mut Vec<Event>, start: u64, note: u8, vel: u8, dur: u64) {
    events.push(Event::NoteOn {
        time: start,
        note,
        vel,
    });
    events.push(Event::NoteOff {
        time: start + dur,
        note,
    });
}

// -----------------------------------------------------------------------
// Score: build the complete event timeline
// -----------------------------------------------------------------------

fn build_score(bpm: f64) -> Vec<Event> {
    let quarter = (60_000.0 / bpm) as u64;
    let half = quarter * 2;
    let whole = quarter * 4;
    let dotted_half = quarter * 3;

    use PitchSymbol::*;

    // ---------------------------------------------------------------
    // Quintal chord voicings — Tymoczko interscalar transposition
    //
    // C quintal {C, D, G, A}, chord-scale steps [2, 5, 2, 3]
    //   Root:    C2–G2–D3–A3   (7, 7, 7)    stacked P5s
    //   1st inv: D2–A2–G3–C4   (7, 10, 5)   wide spread
    //   2nd inv: G2–C3–A3–D4   (5, 9, 5)    compressed
    //   3rd inv: A2–D3–C4–G4   (5, 10, 7)   weighted bass
    //
    // Eb quintal {Eb, F, Bb, C}, steps [2, 5, 2, 3]
    //   1st inv: F2–C3–Bb3–Eb4 (7, 10, 5)
    //
    // Ab quintal {Ab, Bb, Eb, F}, steps [2, 5, 2, 3]
    //   Root:    Ab2–Eb3–Bb3–F4 (7, 7, 7)
    // ---------------------------------------------------------------
    let c_root = [n(C, 1), n(G, 1), n(D, 2), n(A, 2)];
    let c_inv1 = [n(D, 1), n(A, 1), n(G, 2), n(C, 3)];
    let c_inv2 = [n(G, 1), n(C, 2), n(A, 2), n(D, 3)];
    let c_inv3 = [n(A, 1), n(D, 2), n(C, 3), n(G, 3)];

    // Voice leading: C 2nd → Eb 1st = L1 of 4 (G→F, C→C, A→Bb, D→Eb)
    let eb_inv1 = [n(F, 1), n(C, 2), n(Bb, 2), n(Eb, 3)];

    // Voice leading: Eb 1st → Ab root = L1 of 8 (F→Ab, C→Eb, Bb→Bb, Eb→F)
    let ab_root = [n(Ab, 1), n(Eb, 2), n(Bb, 2), n(F, 3)];

    // Passing chord: C 1st → C 2nd (L1 = 12)
    // Move the two upper voices first: G3→A3, C4→D4
    // Result: D–A bare quintal doubled across octaves
    let pass_da = [n(D, 1), n(A, 1), n(A, 2), n(D, 3)];

    let mut events: Vec<Event> = Vec::new();
    let mut t: u64 = 0;

    // === Section 1: C quintal root — open stacked fifths (4 bars) ===
    events.push(Event::Label {
        time: t,
        text: "C quintal root — open stacked fifths",
    });
    events.push(Event::Cutoff { time: t, val: 0.62 });
    t = add_arp(&mut events, t, &c_root, half, 68);
    t = add_arp(&mut events, t, &c_root, half, 70);

    // === Section 2: C 1st inversion + melody enters (4 bars) ===
    let s2 = t;
    events.push(Event::Label {
        time: t,
        text: "C 1st inversion — melody enters",
    });
    events.push(Event::Cutoff { time: t, val: 0.64 });
    t = add_arp(&mut events, t, &c_inv1, half, 65);
    t = add_arp(&mut events, t, &c_inv1, half, 67);

    // Melody phrase 1 (base rhythm: whole, half, half)
    add_mel(&mut events, s2, n(G, 4), 78, whole);
    add_mel(&mut events, s2 + whole, n(E, 4), 75, half);
    add_mel(&mut events, s2 + whole + half, n(D, 4), 72, half);
    // Melody phrase 2 (variation 1: dotted half, quarter, whole)
    add_mel(&mut events, s2 + 2 * whole, n(C, 5), 80, dotted_half);
    add_mel(
        &mut events,
        s2 + 2 * whole + dotted_half,
        n(A, 4),
        70,
        quarter,
    );
    add_mel(&mut events, s2 + 3 * whole, n(G, 4), 76, whole);

    // === Section 3: Passing chord D–A (1 bar, fast arp) ===
    let s3 = t;
    events.push(Event::Label {
        time: t,
        text: "  passing — D–A bare quintal",
    });
    t = add_fast_arp(&mut events, t, &pass_da, quarter, 62);
    // Melody: sustained E4 bridges the transition
    add_mel(&mut events, s3, n(E, 4), 72, whole);

    // === Section 4: C 2nd inversion — compressed (4 bars) ===
    let s4 = t;
    events.push(Event::Label {
        time: t,
        text: "C 2nd inversion — compressed voicing",
    });
    events.push(Event::Cutoff { time: t, val: 0.60 });
    t = add_arp(&mut events, t, &c_inv2, half, 66);
    t = add_arp(&mut events, t, &c_inv2, half, 68);

    // Melody phrase 3 (base rhythm inverted: half, half, whole)
    add_mel(&mut events, s4, n(D, 4), 75, half);
    add_mel(&mut events, s4 + half, n(E, 4), 72, half);
    add_mel(&mut events, s4 + whole, n(G, 4), 78, whole);
    // Melody phrase 4 (variation 1 descending: dotted half, quarter, whole)
    add_mel(&mut events, s4 + 2 * whole, n(A, 4), 80, dotted_half);
    add_mel(
        &mut events,
        s4 + 2 * whole + dotted_half,
        n(G, 4),
        72,
        quarter,
    );
    add_mel(&mut events, s4 + 3 * whole, n(E, 4), 76, whole);

    // === Section 5: Eb 1st inversion — new harmonic center (4 bars) ===
    // Direct voice leading from C 2nd: L1 = 4 (smooth!)
    let s5 = t;
    events.push(Event::Label {
        time: t,
        text: "Eb 1st inversion — new harmonic center (L1=4)",
    });
    events.push(Event::Cutoff { time: t, val: 0.66 });
    t = add_arp(&mut events, t, &eb_inv1, half, 66);
    t = add_arp(&mut events, t, &eb_inv1, half, 68);

    // Melody: Bb and Eb appear as the harmony shifts
    // Phrase 5 (base rhythm with Eb tones)
    add_mel(&mut events, s5, n(C, 5), 80, whole);
    add_mel(&mut events, s5 + whole, n(Bb, 4), 75, half);
    add_mel(&mut events, s5 + whole + half, n(Eb, 5), 78, half);
    // Phrase 6 (variation 1)
    add_mel(&mut events, s5 + 2 * whole, n(Bb, 4), 76, dotted_half);
    add_mel(
        &mut events,
        s5 + 2 * whole + dotted_half,
        n(C, 5),
        72,
        quarter,
    );
    add_mel(&mut events, s5 + 3 * whole, n(Eb, 5), 80, whole);

    // === Section 6: Ab quintal root — the far point (4 bars) ===
    // Direct voice leading from Eb 1st: L1 = 8
    let s6 = t;
    events.push(Event::Label {
        time: t,
        text: "Ab quintal root — the far point (L1=8)",
    });
    events.push(Event::Cutoff { time: t, val: 0.68 });
    t = add_arp(&mut events, t, &ab_root, half, 68);
    t = add_arp(&mut events, t, &ab_root, half, 70);

    // Melody: peak intensity, Ab-pentatonic tones
    // Phrase 7 (base rhythm)
    add_mel(&mut events, s6, n(Eb, 5), 82, whole);
    add_mel(&mut events, s6 + whole, n(C, 5), 76, half);
    add_mel(&mut events, s6 + whole + half, n(Bb, 4), 72, half);
    // Phrase 8 (ascending variation, resolving back)
    add_mel(&mut events, s6 + 2 * whole, n(Ab, 4), 75, dotted_half);
    add_mel(
        &mut events,
        s6 + 2 * whole + dotted_half,
        n(Bb, 4),
        70,
        quarter,
    );
    add_mel(&mut events, s6 + 3 * whole, n(C, 5), 78, whole);

    // === Section 7: C 3rd inversion — approaching home (4 bars) ===
    // Direct voice leading from Ab root: L1 = 6
    // Ab→A(+1), Eb→D(-1), Bb→C(+2), F→G(+2)
    let s7 = t;
    events.push(Event::Label {
        time: t,
        text: "C 3rd inversion — approaching home (L1=6)",
    });
    events.push(Event::Cutoff { time: t, val: 0.63 });
    t = add_arp(&mut events, t, &c_inv3, half, 66);
    t = add_arp(&mut events, t, &c_inv3, half, 68);

    // Melody: callback to opening phrases, C pentatonic again
    // Phrase 9 (echoes phrase 1)
    add_mel(&mut events, s7, n(G, 4), 78, whole);
    add_mel(&mut events, s7 + whole, n(E, 4), 75, half);
    add_mel(&mut events, s7 + whole + half, n(D, 4), 72, half);
    // Phrase 10 (settling, winding down)
    add_mel(&mut events, s7 + 2 * whole, n(C, 5), 76, dotted_half);
    // half rest
    add_mel(&mut events, s7 + 3 * whole, n(G, 4), 70, whole);

    // === Coda: descending through inversions back to root ===
    events.push(Event::Label {
        time: t,
        text: "Coda — descending inversions to root",
    });
    events.push(Event::Cutoff { time: t, val: 0.58 });

    // 2nd inversion — descending arp (2 bars)
    t = add_arp_desc(&mut events, t, &c_inv2, half, 64);

    // 1st inversion — descending arp (2 bars)
    events.push(Event::Cutoff { time: t, val: 0.54 });
    t = add_arp_desc(&mut events, t, &c_inv1, half, 62);

    // Root position — ascending arp (2 bars)
    events.push(Event::Label {
        time: t,
        text: "  root position — home",
    });
    events.push(Event::Cutoff { time: t, val: 0.50 });
    t = add_arp(&mut events, t, &c_root, half, 60);

    // Root again — ascending arp with gradual fade (2 bars)
    events.push(Event::Label {
        time: t,
        text: "  fading...",
    });
    // Fade: each note softer than the last
    let fade_vels: [u8; 4] = [50, 42, 34, 26];
    let gate = (half as f64 * 0.8) as u64;
    for (i, &note) in c_root.iter().enumerate() {
        events.push(Event::NoteOn {
            time: t,
            note,
            vel: fade_vels[i],
        });
        events.push(Event::NoteOff {
            time: t + gate,
            note,
        });
        let cutoff = 0.45 - i as f32 * 0.10;
        events.push(Event::Cutoff {
            time: t,
            val: cutoff.max(0.10),
        });
        t += half;
    }
    let _end = t;

    // Stable sort: (time, priority) ensures note-offs before note-ons
    events.sort_by(|a, b| {
        a.time()
            .cmp(&b.time())
            .then(a.priority().cmp(&b.priority()))
    });

    events
}

// -----------------------------------------------------------------------
// Patch: crystalline glass — Vangelis CS-80 territory
// -----------------------------------------------------------------------

fn setup_patch(xd: &mut RealtimeController<MidirOutput>) -> minilogue_xd::Result<()> {
    // Triangle waves for purity, slight detune for crystalline shimmer
    xd.set_vco1_wave(VcoWave::Tri)?;
    xd.set_vco1_octave(VcoOctave::Eight)?;
    xd.set_vco1_pitch(0.5)?;
    xd.set_vco1_shape(0.55)?;

    xd.set_vco2_wave(VcoWave::Tri)?;
    xd.set_vco2_octave(VcoOctave::Eight)?;
    xd.set_vco2_pitch(0.506)?; // ~8 cents detune — glass shimmer
    xd.set_vco2_shape(0.45)?;

    xd.set_vco1_level(0.75)?;
    xd.set_vco2_level(0.65)?;

    // No sync, no ring — clean and open
    xd.set_sync(Sync::Off)?;
    xd.set_ring(Ring::Off)?;

    // Filter: bright, clean, full keytrack so melody register sparkles
    xd.set_cutoff(0.65)?;
    xd.set_resonance(0.12)?;
    xd.set_cutoff_drive(CutoffDrive::Off)?;
    xd.set_cutoff_keytrack(CutoffKeytrack::Full)?;

    // Amp EG: pad-like — slow bloom, high sustain, long release
    xd.set_amp_eg_attack(0.30)?;
    xd.set_amp_eg_decay(0.25)?;
    xd.set_amp_eg_sustain(0.80)?;
    xd.set_amp_eg_release(0.65)?;

    // Filter EG: gentle brightness bloom on each note
    xd.set_eg_attack(0.20)?;
    xd.set_eg_decay(0.35)?;
    xd.set_eg_int(0.25)?;
    xd.set_eg_target(EgTarget::Cutoff)?;

    // LFO: glacial cutoff movement
    xd.set_lfo_wave(LfoWave::Tri)?;
    xd.set_lfo_mode(LfoMode::Normal)?;
    xd.set_lfo_rate(0.03)?;
    xd.set_lfo_int(0.12)?;
    xd.set_lfo_target(LfoTarget::Cutoff)?;

    // Chorus for crystalline width
    xd.set_mod_fx_on(true)?;
    xd.set_mod_fx_type(ModFxType::Chorus)?;
    xd.set_mod_fx_time(0.45)?;
    xd.set_mod_fx_depth(0.35)?;

    // Delay: stereo, spatial but not rhythmic
    xd.set_delay_on(true)?;
    xd.set_delay_sub_type(DelaySubType::Stereo)?;
    xd.set_delay_time(0.45)?;
    xd.set_delay_depth(0.40)?;
    xd.set_delay_dry_wet(0.25)?;

    // Reverb: deep space — the Vangelis signature
    xd.set_reverb_on(true)?;
    xd.set_reverb_sub_type(ReverbSubType::Space)?;
    xd.set_reverb_time(0.85)?;
    xd.set_reverb_depth(0.75)?;
    xd.set_reverb_dry_wet(0.45)?;

    Ok(())
}

/// Patch parameters as raw SynthParams for MIDI export.
fn patch_synth_params() -> SynthParams {
    let mut s = SynthParams::default();
    apply_patch_params(&mut s);
    s
}

// -----------------------------------------------------------------------
// Play mode: send to hardware in real time
// -----------------------------------------------------------------------

fn play(events: &[Event]) -> Result<(), Box<dyn Error>> {
    let Some(port_name) = device::find_output_port()? else {
        eprintln!("Minilogue XD not found — is it connected via USB?");
        std::process::exit(1);
    };
    let output = MidirOutput::connect(&port_name)?;
    let channel = U4::new(0)?;
    let mut xd = RealtimeController::new(output, channel);

    println!("Setting up crystalline patch...");
    setup_patch(&mut xd)?;
    sleep(Duration::from_millis(300));

    println!("\nPlaying...\n");

    let mut now: u64 = 0;
    for event in events {
        let event_time = event.time();
        if event_time > now {
            sleep(Duration::from_millis(event_time - now));
            now = event_time;
        }
        match event {
            Event::NoteOn { note, vel, .. } => {
                xd.play_note(U7::new(*note)?, U7::new(*vel)?)?;
            }
            Event::NoteOff { note, .. } => {
                xd.stop_note(U7::new(*note)?)?;
            }
            Event::Cutoff { val, .. } => {
                xd.set_cutoff(*val)?;
            }
            Event::Label { text, .. } => {
                println!("  {text}");
            }
        }
    }

    xd.all_notes_off()?;
    println!("\n  Reverb tail...");
    sleep(Duration::from_secs(5));

    println!("\n=== Silence ===");
    Ok(())
}

// -----------------------------------------------------------------------
// Export mode: write Standard MIDI File
// -----------------------------------------------------------------------

fn export(events: &[Event], bpm: f64) -> Result<(), Box<dyn Error>> {
    let tpq: u64 = 480;
    let ms_per_tick = 60_000.0 / (bpm * tpq as f64);

    let synth = patch_synth_params();
    let mut builder = MidiFileBuilder::new(bpm)
        .track_name("Quintal Harmony — Crystalline Vangelis")
        .patch_ccs(0, &synth);

    // Pair NoteOn/NoteOff events to compute durations.
    // Key: MIDI note number → (start_tick, velocity)
    let mut active: HashMap<u8, (u64, u8)> = HashMap::new();

    for event in events {
        let tick = (event.time() as f64 / ms_per_tick).round() as u64;
        match event {
            Event::NoteOn { note, vel, .. } => {
                // If this note is already active (re-trigger), close the previous one
                if let Some((start, prev_vel)) = active.remove(note) {
                    let dur = tick.saturating_sub(start).max(1);
                    builder = builder.note(start, *note, prev_vel, dur);
                }
                active.insert(*note, (tick, *vel));
            }
            Event::NoteOff { note, .. } => {
                if let Some((start, vel)) = active.remove(note) {
                    let dur = tick.saturating_sub(start).max(1);
                    builder = builder.note(start, *note, vel, dur);
                }
            }
            Event::Cutoff { val, .. } => {
                builder = builder.set_cutoff(tick, *val);
            }
            Event::Label { .. } => {}
        }
    }

    // Close any notes still active at the end
    if let Some(last_time) = events.last().map(|e| e.time()) {
        let end_tick = (last_time as f64 / ms_per_tick).round() as u64;
        for (note, (start, vel)) in &active {
            let dur = end_tick.saturating_sub(*start).max(1);
            builder = builder.note(*start, *note, *vel, dur);
        }
    }

    let legend = builder.legend();
    let midi_data = builder.build();

    std::fs::write("quintal-legend.txt", &legend)?;
    println!("Wrote quintal-legend.txt");

    std::fs::write("quintal.mid", &midi_data)?;
    println!(
        "Wrote quintal.mid ({} bytes, {:.1} sec at {:.0} BPM)",
        midi_data.len(),
        events.last().map_or(0, |e| e.time()) as f64 / 1000.0,
        bpm,
    );

    Ok(())
}

// -----------------------------------------------------------------------
// Save mode: write patch to a program slot via SysEx
// -----------------------------------------------------------------------

fn save(_slot: Option<u16>) -> Result<(), Box<dyn Error>> {
    // NOTE: send_current_program / send_program currently triggers a
    // DataFormatError NAK from the XD — likely a from_bytes → to_bytes
    // round-trip fidelity issue in the library. Until that's fixed, we
    // fall back to setting the patch via real-time CCs, which works
    // reliably and is immediately audible.
    let Some(port_name) = device::find_output_port()? else {
        eprintln!("Minilogue XD not found — is it connected via USB?");
        std::process::exit(1);
    };
    let output = MidirOutput::connect(&port_name)?;
    let channel = U4::new(0)?;
    let mut xd = RealtimeController::new(output, channel);

    println!("Loading \"Quintal xd\" via real-time CCs...");
    setup_patch(&mut xd)?;

    println!("Done! The patch is now active — play some keys.");
    println!("To keep it, WRITE to a program slot on the XD (SHIFT + key).");
    Ok(())
}

/// Apply our patch settings to an existing SynthParams (raw 10-bit values).
fn apply_patch_params(s: &mut SynthParams) {
    use minilogue_xd::sysex::program::ProgramName;
    s.name = ProgramName::from_string("Quintal xd").unwrap();
    s.vco1_wave = VcoWave::Tri;
    s.vco1_octave = VcoOctave::Eight;
    s.vco1_pitch = 512;
    s.vco1_shape = 563;
    s.vco2_wave = VcoWave::Tri;
    s.vco2_octave = VcoOctave::Eight;
    s.vco2_pitch = 518;
    s.vco2_shape = 460;
    s.vco1_level = 767;
    s.vco2_level = 665;
    s.sync = false;
    s.ring = false;
    s.cross_mod_depth = 0;
    s.cutoff = 665;
    s.resonance = 123;
    s.cutoff_drive = CutoffDrive::Off;
    s.cutoff_keytrack = CutoffKeytrack::Full;
    s.amp_eg_attack = 307;
    s.amp_eg_decay = 256;
    s.amp_eg_sustain = 818;
    s.amp_eg_release = 665;
    s.eg_attack = 205;
    s.eg_decay = 358;
    s.eg_int = 256;
    s.eg_target = EgTarget::Cutoff;
    s.lfo_wave = LfoWave::Tri;
    s.lfo_mode = LfoMode::Normal;
    s.lfo_rate = 31;
    s.lfo_int = 123;
    s.lfo_target = LfoTarget::Cutoff;
    s.mod_fx_on = true;
    s.mod_fx_type = 0; // Chorus (raw program value)
    s.delay_on = true;
    s.delay_sub_type = DelaySubType::Stereo;
    s.delay_time = 460;
    s.delay_depth = 409;
    s.delay_dry_wet = 256;
    s.reverb_on = true;
    s.reverb_sub_type = ReverbSubType::Space;
    s.reverb_time = 869;
    s.reverb_depth = 767;
    s.reverb_dry_wet = 460;
}

// -----------------------------------------------------------------------
// Main: build score once, then play, export, or save
// -----------------------------------------------------------------------

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let export_mode = args.iter().any(|a| a == "--export");
    let save_mode = args.iter().any(|a| a == "--save");
    let save_slot = args
        .iter()
        .position(|a| a == "--save")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<u16>().ok());

    println!("=== Quintal Harmony — Crystalline Vangelis ===");
    println!("    Tymoczko inversions + C-pentatonic melody\n");

    if save_mode {
        save(save_slot)?;
    } else if export_mode {
        let bpm = 60.0;
        let events = build_score(bpm);
        export(&events, bpm)?;
    } else {
        let bpm = 60.0;
        let events = build_score(bpm);
        play(&events)?;
    }

    Ok(())
}
