---
name: minilogue-xd-sound-design
description: |
  Sound design and synth programming skill for the Korg Minilogue XD,
  using the minilogue-xd Rust library. Covers patch architecture,
  parameter relationships, classic synth sound recipes, sequencing
  patterns, and idiomatic library usage.
  Use when: designing patches, creating sequences, programming the
  XD via Rust, brainstorming electronica, exporting MIDI files, or
  answering questions about subtractive synthesis on this hardware.
---

# Minilogue XD Sound Design Skill

## Overview

This skill provides domain knowledge for programming the Korg Minilogue XD
synthesizer through the `minilogue-xd` Rust library. It covers the synth's
signal flow, parameter relationships, classic sound design patterns, and
idiomatic library usage — enabling expert-level patch creation and sequencing.

## When to Use This Skill

Activate when the task involves:

- Designing synth patches (pads, basses, leads, sequences, FX)
- Programming step sequences or motion sequences
- Real-time parameter control via MIDI
- Exporting performances as MIDI files
- Brainstorming electronica, ambient, or experimental music
- Answering "how do I make it sound like X?" questions
- Tuning or micro-tuning (alternative temperaments, user scales)

## Signal Flow — How the XD Makes Sound

Understanding the signal path is essential for effective sound design:

```
VCO 1 (Saw/Tri/Sqr) ─┐
VCO 2 (Saw/Tri/Sqr) ─┤── Mixer ── Filter (2-pole LPF) ── Amp ── Output
Multi Engine ────────┘     │            │                 │
  (Noise/VPM/User)         │       Cutoff Drive         Amp EG
                           │         Keytrack           (ADSR)
                           │            │
                       VCO Levels    EG → Cutoff/Pitch
                                    LFO → Cutoff/Shape/Pitch
                                        │
                                     Mod FX ── Delay ── Reverb
```

### Key Relationships

| If you want... | Control these... |
|----------------|-----------------|
| Brightness | Cutoff (CC 43), Resonance (CC 44), EG Int (CC 22) |
| Fatness/width | VCO 2 detune (pitch offset from center 512), VCO levels |
| Pulsing/rhythmic | Amp EG Sustain (low = percussive), Decay time |
| Movement | LFO Rate + Int + Target, or automate Cutoff over time |
| Space/depth | Delay Dry/Wet + Time, Reverb Dry/Wet + Time |
| Aggression | Cutoff Drive, Resonance, Cross Mod Depth |
| Warmth | Triangle waves, low resonance, moderate cutoff |

## Parameter Quick Reference

### 10-bit Parameters (0.0–1.0 in the API, 0–1023 raw)

All continuous knob parameters use 10-bit resolution via CC63 + parameter CC:

| Parameter | CC | Notes |
|-----------|-----|-------|
| Cutoff | 43 | The most expressive parameter. Automate this. |
| Resonance | 44 | Self-oscillates near max. 0.4–0.5 = musical sweet spot. |
| VCO 1/2 Pitch | 34/35 | Center = 512. ±1 semitone range near center, ±1 octave at extremes. |
| VCO 1/2 Shape | 36/37 | Pulse width for Sqr, wavefold for Saw/Tri. |
| VCO 1/2 Level | 39/40 | Mixer levels. Keep both < 1.0 to avoid clipping. |
| Amp EG ADSR | 16/17/18/19 | Attack/Decay/Sustain/Release. |
| EG ADSR + Int | 20/21/22 | Filter envelope. Int controls sweep depth. |
| LFO Rate | 24 | Very slow (0.02–0.08) for evolution, fast (0.4+) for vibrato. |
| LFO Int | 26 | Depth of modulation. Subtle (0.1–0.2) is usually better. |
| Delay Time/Depth/DW | 105/106/107 | |
| Reverb Time/Depth/DW | 108/109/110 | |

### Stepped Parameters (Enums)

| Parameter | CC | Values |
|-----------|-----|--------|
| VCO 1/2 Wave | 50/51 | `Sqr`, `Tri`, `Saw` |
| VCO 1/2 Octave | 48/49 | `Sixteen` (16'), `Eight` (8'), `Four` (4'), `Two` (2') |
| LFO Wave | 57 | `Sqr`, `Tri`, `Saw` |
| LFO Mode | 58 | `OneShot`, `Normal`, `Bpm` |
| LFO Target | 56 | `Cutoff`, `Shape`, `Pitch` |
| EG Target | 23 | `Cutoff`, `Pitch2`, `Pitch` |
| Multi Type | 53 | `Noise`, `Vpm`, `User` |
| Mod FX Type | 88 | `Chorus`, `Ensemble`, `Phaser`, `Flanger`, `User` |
| Delay Sub Type | 89 | `Stereo`, `Mono`, `PingPong`, `Tape`, `StereoBpm`, ... |
| Reverb Sub Type | 90 | `Hall`, `Plate`, `Room`, `Space`, ... |

### On/Off Switches

| Parameter | CC | |
|-----------|-----|---|
| Mod FX | 92 | |
| Delay | 93 | |
| Reverb | 94 | |
| Sync | 80 | Hard sync VCO2 to VCO1 — aggressive harmonics |
| Ring | 81 | Ring modulation — metallic/bell tones |

## Sound Design Recipes

### Pad (Lush, Slow)

```rust
.vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.3)
.vco2(VcoWave::Tri, VcoOctave::Eight, 0.52, 0.5)  // slight detune
.vco1_level(0.8).vco2_level(0.7)
.filter(0.6, 0.25, CutoffDrive::Off, CutoffKeytrack::Half)
.amp_eg(0.4, 0.5, 0.7, 0.6)                        // slow attack, high sustain
.lfo(LfoWave::Tri, LfoMode::Normal, 0.15, 0.2, LfoTarget::Cutoff)
.delay(true, DelaySubType::Tape, 0.4, 0.5, 0.3)
.reverb(true, ReverbSubType::Hall, 0.7, 0.6, 0.4)
```

**Why it works:** Detuned oscillators create width. Slow attack lets notes bloom.
High sustain keeps the sound alive. Slow LFO on cutoff adds gentle movement.
Tape delay + hall reverb create depth without washing out.

### Bass (Acid/Squelchy)

```rust
.vco1(VcoWave::Saw, VcoOctave::Sixteen, 0.5, 0.0)  // 16' = low
.vco2(VcoWave::Sqr, VcoOctave::Sixteen, 0.5, 0.3)
.filter(0.35, 0.7, CutoffDrive::Full, CutoffKeytrack::Off)  // resonant!
.amp_eg(0.0, 0.3, 0.0, 0.1)                         // super percussive
.eg(0.0, 0.4, 0.7, EgTarget::Cutoff)                // filter sweep on each note
```

**Why it works:** Low octave + full drive = grit. High resonance makes the filter
sing on each note. Zero sustain + fast decay = punchy. EG sweeps the filter open
and closed on every trigger — the classic acid sound.

### Lead (Cutting, Mono-style)

```rust
.vco1(VcoWave::Saw, VcoOctave::Four, 0.5, 0.0)     // 4' = mid-high
.vco2(VcoWave::Saw, VcoOctave::Four, 0.507, 0.0)    // tiny detune
.sync(true)                                           // hard sync for edge
.filter(0.55, 0.35, CutoffDrive::Half, CutoffKeytrack::Full)
.amp_eg(0.0, 0.4, 0.6, 0.3)
.eg(0.0, 0.3, 0.5, EgTarget::Cutoff)
.portamento(0.15, PortamentoMode::Auto, false)        // glide between notes
```

**Why it works:** Hard sync creates harmonically rich, cutting overtones.
Full keytrack keeps brightness consistent across the keyboard. Portamento
gives the classic mono-synth slide.

### Berlin School Sequence

```rust
// Patch: percussive, resonant, with echo
.vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.0)
.vco2(VcoWave::Saw, VcoOctave::Eight, 0.515, 0.0)  // ~18 cents detune
.filter(0.35, 0.45, CutoffDrive::Off, CutoffKeytrack::Half)
.amp_eg(0.0, 0.35, 0.25, 0.30)                      // LOW sustain = pulse
.eg(0.0, 0.40, 0.55, EgTarget::Cutoff)              // filter opens on each note
.lfo(LfoWave::Tri, LfoMode::Normal, 0.05, 0.15, LfoTarget::Cutoff)  // very slow
.delay(true, DelaySubType::Tape, 0.37, 0.55, 0.35)  // offset echo
.reverb(true, ReverbSubType::Hall, 0.65, 0.5, 0.25)
```

**The secret sauce:**

- **Low amp sustain (0.25)** makes each 16th note pulse distinctly
- **Filter EG on cutoff** gives each note a "pew" — opens then closes
- **Very slow LFO (0.05)** creates the long-term filter sweep over many bars
- **Tape delay at 0.37** (not 0.5) creates a dotted-eighth polyrhythm against 16ths
- **Automate cutoff across repetitions** — gradually open the filter, then close for fade

### Ambient Drone

```rust
.vco1(VcoWave::Tri, VcoOctave::Sixteen, 0.5, 0.7)   // shaped triangle
.vco2(VcoWave::Tri, VcoOctave::Eight, 0.503, 0.6)    // octave above, detuned
.multi_type(MultiType::Noise)                          // add noise texture
.filter(0.45, 0.15, CutoffDrive::Off, CutoffKeytrack::Off)
.amp_eg(0.8, 0.0, 1.0, 0.9)                          // very slow attack, full sustain
.lfo(LfoWave::Tri, LfoMode::Normal, 0.03, 0.25, LfoTarget::Cutoff)
.reverb(true, ReverbSubType::Space, 0.9, 0.8, 0.6)   // deep space reverb
```

**Why it works:** Triangles are warm and harmonic-poor — good foundation for drones.
Noise adds texture without pitch. Slow attack means the sound emerges gradually.
Full sustain = infinite hold. Deep reverb creates the space.

## Sequencing Patterns

### Classic 16th-Note Arpeggio (E minor)

Root-fifth-octave interplay with passing tones:

```
E2 → B2 → E3 → B2 → G3 → B2 → E3 → D3
E2 → B2 → G3 → E3 → A3 → G3 → E3 → D3
```

**Why it works:** The alternating low root (E2) and high notes create the
illusion of two voices. The fifth (B2) acts as a rhythmic anchor. Passing
tones (D3, A3) add harmonic interest without leaving the key.

### Key Modulation

Transpose the same pattern to create sections:

- **Down a minor 3rd** (E→C#): darker, more intense
- **Down a major 3rd** (E→C): distinctly different color, classic TD move
- **Up a 4th** (E→A): brighter, lifting
- **Return to root**: homecoming resolution

### Performance Automation

The filter sweep is the primary performance tool:

```rust
// Gradually open the filter across repetitions
for rep in 0..reps {
    let cutoff = base + drift * rep as f32;
    builder = builder.set_cutoff(tick, cutoff);
    // ... play pattern ...
}
```

Combine with velocity scaling for fades:

```rust
let vel = (step.vel as f32 * vel_scale).round() as u8;
```

## Library API Patterns

### Real-Time Hardware Control

```rust
use minilogue_xd::controller::RealtimeController;
use minilogue_xd::transport::MidirOutput;
use minilogue_xd::message::{U4, U7};

let output = MidirOutput::connect("minilogue xd SOUND")?;
let mut xd = RealtimeController::new(output, U4::new(0)?);

xd.set_cutoff(0.6)?;           // float 0.0–1.0 → 10-bit
xd.set_vco1_wave(VcoWave::Saw)?; // typed enum
xd.play_note(U7::new(60)?, U7::new(100)?)?;
```

**Important:** Use the "SOUND" port, not "MIDI OUT" — SOUND goes directly
to the synth engine.

### Building Patches Programmatically

```rust
use minilogue_xd::builder::patch::PatchBuilder;

let patch = PatchBuilder::new()
    .name("My Patch")?
    .vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.3)
    .filter(0.5, 0.4, CutoffDrive::Off, CutoffKeytrack::Half)
    .amp_eg(0.1, 0.4, 0.6, 0.3)
    .delay(true, DelaySubType::Tape, 0.4, 0.5, 0.3)
    .build();
```

Float parameters **clamp** (don't error) — the builder is forgiving.

### Exporting to MIDI File

```rust
use minilogue_xd::midi_file::MidiFileBuilder;

let builder = MidiFileBuilder::new(120.0)
    .track_name("My Sequence")
    .patch_ccs(0, &patch.synth)          // all CCs at tick 0
    .set_cutoff(480, 0.7)                // automate cutoff at tick 480
    .note(0, 60, 100, 240)              // note events
    .note(480, 64, 90, 240);

// Write the legend first (documents all CCs used)
std::fs::write("legend.txt", builder.legend())?;
let midi = builder.build();
std::fs::write("output.mid", &midi)?;
```

### SysEx Transactions (Program Dump/Load)

```rust
use minilogue_xd::sysex::transaction::SysexTransaction;

let mut tx = SysexTransaction::new(&mut output, &mut input, channel);
let current = tx.request_current_program()?;   // read from synth
tx.send_current_program(&modified_patch)?;      // write to synth
```

## Sound Design Principles for the XD

1. **Detune is your friend.** Even 5–18 cents between VCO1 and VCO2 creates
   width and warmth. Center pitch = 512 (0.5 in float). Try 515–530.

2. **Amp EG sustain controls "pulse."** Low sustain (0.2–0.3) = sequences pulse.
   High sustain (0.7–0.9) = pads and drones sustain.

3. **EG → Cutoff is the most musical modulation.** It gives each note its own
   filter sweep. Int controls depth. Keep it moderate (0.3–0.6) for musicality.

4. **LFO rate < 0.1 for evolution, > 0.3 for vibrato.** The sweet spot for
   Berlin School long-term sweeps is 0.03–0.08.

5. **Tape delay creates polyrhythm.** Set delay time slightly off from the
   note grid (e.g., 0.37 against 16th notes) for the classic TD echo pattern.

6. **Automate the filter, not the volume.** Closing the filter gradually is
   a more musical fade than reducing velocity alone. Combine both for best results.

7. **The XD's resonance self-oscillates near max.** Use 0.4–0.5 for "singing"
   resonance without runaway. Above 0.7, it screams.

8. **Hard sync (VCO2 synced to VCO1) + sweeping VCO2 pitch = classic lead tone.**
   Aggressive, harmonically rich. Great for solo lines.

9. **Noise in the multi engine adds texture to anything.** Even at low levels
   (multi_level 0.1–0.2), it adds analog-like grit and air.

10. **The reverb "Space" and "Submarine" types are the most atmospheric.**
    Use "Hall" for general-purpose, "Plate" for drums, "Space" for ambient.

## Genre Quick Reference

| Genre | VCO | Filter | Amp EG | FX | Notes |
|-------|-----|--------|--------|-----|-------|
| Berlin School | 2× Saw, detuned | Closed, res 0.45, EG sweep | Fast attack, low sustain | Tape delay, Hall reverb | Automate cutoff |
| Ambient | Tri + Noise | Open, low res | Slow attack, full sustain | Space reverb, deep | Hold notes, let LFO evolve |
| Acid | Saw 16' | Closed, res 0.7, drive full | Zero sustain, fast decay | Optional delay | EG → Cutoff is everything |
| Synthwave | 2× Saw, chorus | Mid-open, res 0.3 | Medium attack, high sustain | Chorus + delay | Lush chords, arp patterns |
| Industrial | Sqr + Ring mod | Variable, drive full | Percussive | Distorted delay | Cross mod, ring mod, noise |
| Minimal techno | Single Saw/Sqr | Sweeping, moderate res | Percussive | Ping pong delay | Repetitive, filter is the groove |
