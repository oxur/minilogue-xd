# minilogue-xd

[![][build-badge]][build]
[![Crates.io](https://img.shields.io/crates/v/minilogue-xd)](https://crates.io/crates/minilogue-xd)
[![Docs.rs](https://img.shields.io/docsrs/minilogue-xd)](https://docs.rs/minilogue-xd)
[![MIT / Apache-2.0](https://img.shields.io/crates/l/minilogue-xd)](LICENSE-MIT)

A complete Rust library for the Korg Minilogue XD synthesizer, covering 100% of the
[MIDI Implementation (Revision 1.01)](https://www.korg.com/us/support/download/manual/0/811/4440/) —
CC parameters, NRPN, SysEx program and global data blobs, the 16-step sequencer with motion
sequences, sub-cent tuning tables, and user module management for logue SDK units.

The goal: treat the XD desktop module as a voice in an imaginary MIDI-based Eurorack system,
with every parameter reachable from Rust in real time.

---

## Install

```toml
# Cargo.toml
[dependencies]
minilogue-xd = "0.1"
```

Requires Rust 2021 edition. The `midi-io` feature (default on) pulls in `midir` for live
MIDI port access. Disable it for pure file manipulation or `no_std` targets:

```toml
minilogue-xd = { version = "0.1", default-features = false, features = ["file-formats"] }
```

---

## Quick start

```rust
use minilogue_xd::{
    controller::RealtimeController,
    param::enums::{VcoWave, VcoOctave, LfoMode, EgTarget, MicroTuning},
    transport::MidirOutput,
};

fn main() -> minilogue_xd::Result<()> {
    let output = MidirOutput::connect("minilogue xd", 0)?;
    let mut xd = RealtimeController::new(output, 0);

    // VCO control — natural units, typed enums
    xd.set_vco1_wave(VcoWave::Saw)?;
    xd.set_cutoff(0.6)?;        // 0.0 – 1.0
    xd.set_resonance(0.4)?;
    xd.set_lfo_rate(0.3)?;
    xd.set_lfo_mode(LfoMode::Bpm)?;
    xd.set_eg_target(EgTarget::Cutoff)?;

    // NRPN: deep parameters not on the CC surface
    xd.set_micro_tuning(MicroTuning::Pythagorean)?;
    xd.set_bend_range_plus(7)?;

    xd.play_note(60, 100)?;     // middle C, velocity 100
    Ok(())
}
```

---

## Patch manipulation

```rust
use minilogue_xd::{
    builder::PatchBuilder,
    param::enums::{VcoWave, VcoOctave, CutoffDrive, CutoffKeytrack, DelaySubType},
    sysex::transaction::SysexTransaction,
};

// Build a patch programmatically
let patch = PatchBuilder::new()
    .name("Acid Bass")
    .vco1(VcoWave::Saw, VcoOctave::Eight, 0.0, 0.5)
    .filter(0.35, 0.8, CutoffDrive::Full, CutoffKeytrack::Full)
    .amp_eg(0.0, 0.3, 0.0, 0.1)
    .delay(DelaySubType::Tape, 0.4, 0.6, 0.3)
    .build()?;

// Send it to the edit buffer over SysEx
let mut tx = SysexTransaction::new(output, input, channel);
tx.send_current_program(&patch)?;

// Load a .mnlgxdlib file from disk
let patches = minilogue_xd::load_lib_file("factory.mnlgxdlib")?;
println!("{} patches loaded", patches.len());
```

---

## Sequencer & motion

```rust
use minilogue_xd::builder::SequenceBuilder;
use minilogue_xd::param::enums::{StepResolution, ArpRate, MotionParameter};

let seq = SequenceBuilder::new()
    .bpm(120.0)
    .length(8)
    .resolution(StepResolution::Sixteenth)
    .swing(15)
    .step(0, &[60], &[100], GateTime::Percent(50))
    .step(2, &[63], &[80],  GateTime::Percent(50))
    .step(4, &[60], &[110], GateTime::Percent(75))
    .step(6, &[67], &[90],  GateTime::Tie)
    // Motion: automate cutoff across all 8 steps
    .motion_slot(0, MotionParameter::Cutoff, true)
    .motion_step(0, 0, &[0.2, 0.4, 0.6, 0.8, 1.0])
    .build()?;
```

---

## Micro-tuning

```rust
use minilogue_xd::sysex::tuning::{UserScale, CentOffset};
use minilogue_xd::sysex::transaction::SysexTransaction;

// Build a 128-note user scale
let mut scale = UserScale::equal_temperament();
// Flatten the third by 14 cents (just intonation approximation)
scale.set_note(64, CentOffset(-14.0));

let mut tx = SysexTransaction::new(output, input, channel);
tx.send_user_scale(&scale)?;
```

---

## Features

| What | Coverage |
|------|----------|
| CC parameters | All 50+ with typed enums and 10-bit high-resolution encoding |
| NRPN parameters | All 29 entries, including VPM params and user oscillator params |
| Program blob | Full 1024-byte parse/serialize (TABLE 2), all bit-packed fields |
| Global settings | Full 63-byte parse/serialize (TABLE 1) |
| Sequencer | 16 steps, active steps, motion slots, step events, ARP settings |
| Motion sequences | 4 slots × 16 steps × 5-point interpolation, 45 assignable parameters |
| Micro-tuning | User scale (128 notes, 0.0061-cent precision), user octave (12 notes) |
| MIDI Tuning Standard | Bulk dump and single-note tuning change |
| User modules | Query, upload, download, clear, swap logue SDK OSC/FX slots |
| File formats | `.mnlgxdprog`, `.mnlgxdlib`, `.mnlgxdpreset`, `.mnlgxdunit` |
| Poly chain | Note on/off SysEx with sub-semitone pitch encoding |
| Transport | Clock, Start, Continue, Stop, Song Position Pointer |
| Device identity | Inquiry request/reply, search device request/reply |

---

## Architecture

```
minilogue-xd
├── transport       MIDI I/O abstraction (midir backend, MockOutput for tests)
├── message         Channel, realtime, and common message types
├── codec           Korg 7-bit ↔ 8-bit SysEx encoding (NOTE 1)
├── param
│   ├── enums       Typed enums for all stepped/discrete parameters
│   ├── encoding    10-bit, 8-bit high-res, and 14-bit parameter encoding
│   ├── cc          CC parameter map (50+ parameters)
│   └── nrpn        NRPN parameter map and state machine receiver
├── sysex
│   ├── frame       Korg SysEx header builder/parser, ACK/NAK status codes
│   ├── identity    Device inquiry and search device messages
│   ├── global      Global parameter blob (TABLE 1)
│   ├── program     Program data blob (TABLE 2): synth + sequencer
│   ├── tuning      User scale, user octave, MIDI Tuning Standard
│   ├── user_module logue SDK slot management
│   ├── poly_chain  Poly chain note on/off
│   └── transaction Request/response manager with timeout and ACK/NAK handling
├── controller      Real-time parameter controller (fluent API)
└── builder         PatchBuilder and SequenceBuilder
```

---

## Feature flags

| Flag | Default | Description |
|------|---------|-------------|
| `midi-io` | on | Live MIDI I/O via `midir` |
| `file-formats` | on | `.mnlgxd*` file format support (requires `zip`) |
| `std` | on | Disable for `no_std` — codec, message, and param layers remain available |

---

## Community references

This library is built on top of prior community work that validated the spec against
real hardware. The `workbench/` directory (not tracked in git) contains clones of:

| Repo | Language | What it contributes |
|------|----------|---------------------|
| [korginc/logue-sdk](https://github.com/korginc/logue-sdk) | C/C++ | Official Korg SDK — user module binary format, CRC32 |
| [gekart/mnlgxd.py](https://gist.github.com/gekart/b187d3c16e6160571ccfcf6c597fea3f) | Python | Program blob parser — ground truth for TABLE 2 byte offsets |
| [isnotinvain/minilogue-xd-util](https://github.com/isnotinvain/minilogue-xd-util) | Python | Patch object model — sequencer and motion byte layout |
| [gazzar/loguetools](https://github.com/gazzar/loguetools) | Python | Cross-synth patch tools — `.mnlgxd*` file container formats |

### Known documentation errata

The Korg MIDI implementation document (rev 1.01) contains a small number of errors
discovered by the community:

- Bank select and program number encoding differs from the spec in practice — the library
  is validated against real files rather than the documented encoding alone.
- `.mnlgxdpreset` files from firmware v2.10+ use 448-byte program blobs, not the 1024-byte
  format described in the spec. Both formats are supported.

---

## MIDI implementation version

Covers **Korg Minilogue XD MIDI Implementation Revision 1.01 (2020.02.10)**.
Works with both the keyboard variant and the desktop module variant.

---

## Examples

The [`examples/`](crates/minilogue-xd/examples/) directory contains runnable demos:

- **[`list_ports`](crates/minilogue-xd/examples/list_ports.rs)** — Enumerate available MIDI output ports
- **[`test_drive`](crates/minilogue-xd/examples/test_drive.rs)** — Connect to the XD, set up a pad sound, and play a chord progression
- **[`berlin_school`](crates/minilogue-xd/examples/berlin_school.rs)** — Tangerine Dream–style Berlin School sequencer with evolving filter sweeps, tape echo, key modulation (E minor → C minor → E minor), and velocity fade

Run them with:

```bash
cargo run -p minilogue-xd --example berlin_school
```

---

## Workspace structure

```
crates/
  minilogue-xd/   — Core library (this crate)
  minilogue/       — Umbrella crate re-exporting all Minilogue variants
```

---

## Contributing

Issues and PRs welcome. The `MIDI_COVERAGE.md` file documents the explicit mapping from
every item in the MIDI implementation document to the Rust type or function that covers it —
a useful starting point for identifying gaps.

---

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

*Not affiliated with Korg Inc.*

[//]: ---Named-Links---

[build]: https://github.com/oxur/minilogue-xd/actions/workflows/ci.yml
[build-badge]: https://github.com/oxur/minilogue-xd/actions/workflows/ci.yml/badge.svg
