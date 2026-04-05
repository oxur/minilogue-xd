# Phase 1 Implementation Plan — minilogue-xd Foundation

## Context

This is a greenfield Rust library for the Korg Minilogue XD synthesizer. No `Cargo.toml` or `src/` exists yet. Phase 1 establishes the crate scaffold, the Korg 7-bit SysEx wire codec, all MIDI message types (channel, system realtime, system common), and the MIDI I/O transport abstraction. Everything in Phases 2–4 depends on this foundation.

The authoritative MIDI spec lives at `workbench/minilogue-xd-util/assets/minilogue_xd__MIDIImp.txt`.

---

## Milestone 1.1 — Crate Scaffold

**Files to create:**
- `Cargo.toml`
- `src/lib.rs`
- `src/error.rs`
- `src/codec.rs` (empty stub)
- `src/message/mod.rs` (empty stub)
- `src/transport.rs` (empty stub)
- `src/connection.rs` (empty stub)

**Cargo.toml design:**
```toml
[package]
name = "minilogue-xd"
version = "0.1.0"
edition = "2021"
description = "Rust library for the Korg Minilogue XD synthesizer MIDI implementation"
license = "MIT OR Apache-2.0"
repository = "https://github.com/oxur/minilogue-xd"

[features]
default = ["midi-io", "file-formats", "std"]
std = []
midi-io = ["dep:midir"]
file-formats = []   # placeholder for Phase 3 zip support

[dependencies]
thiserror = "2"
bitflags = "2"
midir = { version = "0.10", optional = true }
```

Features are additive (PS-06). `dep:midir` syntax makes midir truly optional.

**src/error.rs** — Centralized error module (PS-04):
- `#[non_exhaustive]` `Error` enum (ID-01) with variants:
  - `OutOfRange { type_name, value, min, max }` — for newtype validation
  - `Codec(String)` — 7-bit codec errors
  - `InvalidMessage(String)` — MIDI parse errors
  - `#[cfg(feature = "midi-io")] MidiIo(String)` — transport errors
  - `#[cfg(feature = "midi-io")] Midir(#[from] midir::SendError)` — midir errors
- `pub type Result<T> = std::result::Result<T, Error>;`

**src/lib.rs** — Module declarations with stubs, re-exports `Error` and `Result`.

**Acceptance:** `make build`, `make test`, `make lint` all pass.

---

## Milestone 1.2 — Korg 7-bit SysEx Codec

**File:** `src/codec.rs`

**Algorithm** (from NOTE 1 of the MIDI spec, line 1113):
- Each group of 7 data bytes (8-bit) becomes 8 wire bytes (7-bit)
- Wire byte 0 = packed MSBs: bit i holds bit 7 of data byte i (i=0..6)
- Wire bytes 1–7 = data bytes with bit 7 cleared (& 0x7F)
- Final partial group (<7 bytes) uses the same logic, emitting group_len + 1 wire bytes

**Functions:**
```rust
pub fn encode_7bit(data: &[u8]) -> Vec<u8>     // infallible
pub fn decode_7bit(data: &[u8]) -> Result<Vec<u8>>  // errors on high bits set
```

**Add to Cargo.toml:** `proptest = "1"` in `[dev-dependencies]`

**Tests:**
- Round-trip identity for lengths 0–21
- Known size vectors: 336 bytes -> 384 wire bytes, 32 -> 37, 9 -> 11
- Error on any wire byte with bit 7 set
- Empty input -> empty output
- 1000 random round-trips via proptest
- Wire length formula: `(N/7)*8 + if N%7 > 0 { N%7 + 1 } else { 0 }`

**Acceptance:** All tests pass. 95%+ coverage of codec.rs.

---

## Milestone 1.3 — Channel Messages (TX)

**Files:**
- `src/message/mod.rs` — module declarations
- `src/message/types.rs` — newtype wrappers
- `src/message/channel.rs` — TX channel messages + traits

**Newtype wrappers** (all derive `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord`):
- `U4(u8)` — 0..=15 (MIDI channel)
- `U7(u8)` — 0..=127 (note, velocity, CC value)
- `U14(u16)` — 0..=16383 (Song Position Pointer)
- `I14(i16)` — -8192..=8191 (Pitch Bend)

Each gets: `new(val) -> Result<Self>`, `value(self) -> {prim}`, `TryFrom<{prim}>`, `From<Self> for {prim}`, `Display`.

**Traits:**
```rust
pub trait ToMidiBytes { fn to_midi_bytes(&self) -> Vec<u8>; }
pub trait FromMidiBytes: Sized { fn from_midi_bytes(bytes: &[u8]) -> Result<Self>; }
```

**Channel message structs:** `NoteOn`, `NoteOff`, `ProgramChange`, `PitchBend`, `ControlChange`, `ChannelPressure` — each with channel + relevant fields. All implement both traits.

**Pitch Bend encoding:** center=0x2000, `wire = i14 + 8192`, split into 7-bit halves: `[0xE0|ch, wire & 0x7F, (wire >> 7) & 0x7F]`.

**Tests:** Round-trips for each type, boundary values for all newtypes, PitchBend center/extremes, invalid inputs.

**Acceptance:** All channel messages serialize to correct byte sequences per spec Section 1-1.

---

## Milestone 1.4 — Channel Messages (RX Additions)

**Files modified:** `src/message/channel.rs`, `src/message/mod.rs`

**New types:**
- `AllSoundOff { channel }` — CC 120, value 0
- `AllNotesOff { channel }` — CC 123, value 0
- `LocalControl { channel, state: LocalControlState }` — CC 122 (use enum, not bool per AP-10)
- `Damper { channel, value: U7 }` — CC 64 on RX (full 0–127, asymmetric with TX Hold)
- `BankSelect { channel, msb: U7, lsb: U7 }` — CC 0 + CC 32 pair, lsb validated 0–4

**`MidiMessage` enum** (in `src/message/mod.rs`):
- `#[non_exhaustive]` with variants for all channel message types
- BankSelect is NOT in MidiMessage (requires two MIDI messages to compose); exists as separate construct
- NoteOn with velocity 0 parses as NoteOff per MIDI convention

**`parse_midi_bytes(bytes: &[u8]) -> Result<MidiMessage>`:**
- Dispatches on status nibble (0x80..=0xE0)
- CC messages inspected by controller number: 120->AllSoundOff, 122->LocalControl, 123->AllNotesOff, 64->Damper, others->ControlChange
- 0xF0..=0xFF reserved for M1.5

**Tests:** Every variant round-trips, BankSelect LSB=5 fails, NoteOn velocity=0 -> NoteOff.

---

## Milestone 1.5 — System Realtime & Common Messages

**Files:**
- `src/message/realtime.rs` (new)
- `src/message/common.rs` (new)
- `src/message/mod.rs` (extend MidiMessage + parse_midi_bytes)

**Realtime messages** (single-byte, unit structs implementing ToMidiBytes/FromMidiBytes):
- `TimingClock` (0xF8), `Start` (0xFA), `Continue` (0xFB), `Stop` (0xFC), `ActiveSensing` (0xFE)

**Common messages:**
- `SongPositionPointer { beats: U14 }` — 0xF2, standard 14-bit encoding

**Extended parse_midi_bytes:** Add 0xF2, 0xF8, 0xFA, 0xFB, 0xFC, 0xFE dispatch. 0xF0 (SysEx) returns error for now (Phase 3).

**Tests:** Exact byte values, SPP round-trip at 0/8192/16383, unsupported status bytes error.

---

## Milestone 1.6 — MIDI I/O Transport

**Files:**
- `src/transport.rs` — traits + MockOutput + midir impls
- `src/connection.rs` — MinilogueXd struct

**Transport traits** (always available, NOT feature-gated):
```rust
pub trait MidiOutput: Send { fn send(&mut self, bytes: &[u8]) -> Result<()>; }
pub trait MidiInput: Send { fn listen<F: FnMut(&[u8]) + Send + 'static>(&mut self, callback: F) -> Result<()>; }
```

**MockOutput** (always available, for tests):
```rust
pub struct MockOutput { messages: Vec<Vec<u8>> }
// methods: new(), messages(), last_message(), clear()
// implements MidiOutput
```

**midir implementations** (`#[cfg(feature = "midi-io")]`):
- `MidirOutput` — wraps `midir::MidiOutputConnection`, with `connect(port_name)` and `available_ports()`
- `MidirInput` — wraps `midir::MidiInputConnection`, with `connect(port_name)` and `available_ports()`

**MinilogueXd** (generic over output type):
```rust
pub struct MinilogueXd<O: MidiOutput = Box<dyn MidiOutput>> {
    output: O,
    channel: U4,
}
```
- `new(output, channel)` — always available
- `connect(port_name, channel)` — `#[cfg(feature = "midi-io")]`, convenience constructor
- `send_message(&mut self, msg: &MidiMessage) -> Result<()>`
- `channel(&self) -> U4`

Also implement `ToMidiBytes` for `MidiMessage` (delegates to inner variant).

**Tests:**
- MockOutput captures sent bytes
- MinilogueXd::send_message produces correct wire bytes for all MidiMessage variants
- `cargo build --no-default-features` compiles (codec + messages + MockOutput work without midir)

**Acceptance:** Tests pass without hardware. Feature flag isolation verified.

---

## Verification

After all 6 milestones:

```bash
make check                                    # build + lint + test
cargo test --all-features                     # everything on
cargo build --no-default-features             # core only (no midir)
cargo test --no-default-features --features std  # core + std
make coverage                                 # verify 95%+ target
```

Every milestone ends with `make check` passing before moving to the next.
