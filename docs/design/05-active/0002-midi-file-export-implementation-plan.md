---
number: 2
title: "MIDI File Export — Implementation Plan"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-04-06
updated: 2026-04-06
state: Active
supersedes: null
superseded-by: null
version: 1.0
---

# MIDI File Export — Implementation Plan

## Context

The library can control a Minilogue XD in real time, but there's no way to save a performance as a standard MIDI file (.mid). Adding SMF Format 0 export lets users capture patch setups and sequences as portable MIDI files — playable in DAWs, shareable, and replayable through any MIDI player connected to the XD.

No external dependencies needed — we already have all the byte-level primitives.

---

## File: `crates/minilogue-xd/src/midi_file.rs` (NEW)

### Core Types

```rust
pub struct MidiFileBuilder {
    ticks_per_quarter: u16,  // default 480
    tempo_bpm: f64,
    channel: u8,             // 0-15
    events: Vec<MidiFileEvent>,
}

struct MidiFileEvent {
    tick: u64,       // absolute tick position
    bytes: Vec<u8>,  // raw MIDI event bytes
}
```

### Public API

- `MidiFileBuilder::new(tempo_bpm: f64) -> Self`
- `.channel(ch: u8) -> Self`
- `.ticks_per_quarter(tpq: u16) -> Self`
- `.track_name(name: &str) -> Self` — meta event FF 03
- `.note(tick, note, velocity, duration_ticks) -> Self` — emits NoteOn + NoteOff
- `.cc(tick, controller, value) -> Self`
- `.program_change(tick, program) -> Self`
- `.pitch_bend(tick, value: i16) -> Self`
- `.sysex(tick, data: &[u8]) -> Self`
- `.patch_ccs(tick, synth: &SynthParams) -> Self` — emits ~45 CCs matching RealtimeController
- `.build() -> Vec<u8>` — produces complete SMF bytes
- `.write_to(writer: &mut impl Write) -> io::Result<()>`

### Internal Helpers

- `encode_vlq(value: u64) -> Vec<u8>` — Variable-Length Quantity for delta times
- `write_header(tpq: u16) -> Vec<u8>` — "MThd" chunk
- `write_track(events: &[MidiFileEvent]) -> Vec<u8>` — "MTrk" chunk with sorted events and delta times
- `tempo_to_microseconds(bpm: f64) -> [u8; 3]` — FF 51 03 conversion

### patch_ccs Implementation

Emits CCs for all major SynthParams fields using the CC number mappings from `controller.rs`:
- **10-bit fields** (cutoff, resonance, VCO pitch/shape/level, EG params, LFO, FX params): CC63(lsb) + param_CC(msb)
- **Stepped enums** (VCO wave/octave, LFO wave/mode/target, EG target, FX types): CC with `to_tx_value()`
- **On/off** (mod_fx_on, delay_on, reverb_on): CC with 0/127

Imports `SteppedParam` trait from `param/mod.rs` for `to_tx_value()`.

### SMF Structure

```
"MThd" [0,0,0,6] [0,0] [0,1] [tpq_hi,tpq_lo]  — Format 0, 1 track
"MTrk" [length_4bytes]
  delta=0  FF 03 len "track name"     — Track Name
  delta=0  FF 51 03 tt tt tt          — Tempo
  delta=0  FF 58 04 04 02 18 08       — Time Sig (4/4)
  delta=0  Bn cc vv                   — CC events (patch setup)
  delta=N  9n kk vv                   — Note On
  delta=D  8n kk 40                   — Note Off
  ...
  delta=0  FF 2F 00                   — End of Track
```

---

## File: `crates/minilogue-xd/examples/export_midi.rs` (NEW)

Exports the Berlin School sequence as a .mid file. Uses `MidiFileBuilder` to:
1. Set tempo to 120 BPM
2. Add patch setup CCs (saw waves, filter, EG, LFO, delay, reverb)
3. Add the E minor → C minor → E minor note sequence with 16th note timing
4. Write to `berlin_school.mid`

Requires `midi-io` feature (for consistency with other examples, though midi_file itself doesn't need it).

Actually — `export_midi` doesn't need `midi-io` at all since it just writes a file, no hardware. Don't add `required-features`.

---

## Files Modified

- `crates/minilogue-xd/src/lib.rs` — add `pub mod midi_file;`
- `crates/minilogue-xd/Cargo.toml` — add `[[example]] name = "export_midi"` (no required-features)

---

## Tests (~25)

**VLQ encoding:** 0, 127, 128, 16383, large values
**Header:** valid "MThd", format 0, 1 track, correct TPQ
**Track:** valid "MTrk", correct length, ends with End of Track meta
**Tempo:** BPM to microseconds conversion (120 BPM = 500000 us)
**Notes:** NoteOn + NoteOff at correct ticks with proper delta times
**CCs:** correct status byte + controller + value
**Delta times:** events at same tick have delta=0, ordering preserved
**SysEx:** F0 + length + data + F7
**patch_ccs:** emits CCs for key SynthParams fields
**Integration:** build produces parseable SMF bytes

---

## Verification

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run -p minilogue-xd --example export_midi
# Then open berlin_school.mid in a DAW or MIDI player to verify
```
