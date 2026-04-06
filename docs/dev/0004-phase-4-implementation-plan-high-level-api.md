# Phase 4 Implementation Plan — High-Level API & Ergonomics

## Context

Phases 1–3 are complete (895 tests passing). Phase 4 builds typed, idiomatic Rust interfaces that make the library feel like a hardware synth module rather than a byte-wrangler. It covers 6 milestones: real-time controller, SysEx transaction manager, patch builder, sequence builder, integration tests, and publication prep.

---

## File Organization

```
src/
  controller.rs       — M4.1: RealtimeController
  builder/
    mod.rs            — module root
    patch.rs          — M4.3: PatchBuilder
    sequence.rs       — M4.4: SequenceBuilder
  sysex/
    transaction.rs    — M4.2: SysexTransaction
  transport.rs        — M4.2: MidiInput trait, MockMidiInput, MidirInput (additions)
tests/
  cc_coverage.rs      — M4.5: CC parameter map completeness
  nrpn_coverage.rs    — M4.5: NRPN round-trip coverage
  sysex_flows.rs      — M4.5: SysEx transaction end-to-end
  builder_roundtrip.rs — M4.5: Builder serialization round-trips
```

---

## M4.1 — Real-Time Parameter Controller

**File:** `src/controller.rs`

**`RealtimeController<O: MidiOutput>`** — generic over output backend, provides typed methods for every CC and NRPN parameter.

### Float-to-Integer Mapping
- 10-bit CC params: float 0.0–1.0 → 0–1023 via `(value * 1023.0).round()`
- Pitch bend: float -1.0..+1.0 → I14 -8192..8191 via `(value * 8191.0).round()`
- Program level (NRPN): float -18.0..+6.0 dB → u8 0–120 via `(db + 18) * 5`
- Out-of-range values return `Error::OutOfRange`

### Method Categories
- **31 high-res CC methods** (e.g., `set_cutoff`, `set_amp_eg_attack`): send CC63(lsb) + param_CC(msb)
- **16 stepped CC methods** (e.g., `set_vco1_wave`, `set_delay_sub_type`): send CC with `to_tx_value()`
- **3 on/off CC methods** (`set_mod_fx_on`, `set_delay_on`, `set_reverb_on`): send 0 or 127
- **Note/transport** (`play_note`, `stop_note`, `pitch_bend`, `program_change`, `all_notes_off`)
- **NRPN methods** (`set_bend_range_plus`, `set_micro_tuning`, `set_program_level`)

### Internal Helpers
- `send_cc(controller, value)` — constructs ControlChange + sends via ToMidiBytes
- `send_10bit_cc(cc_number, value: f32)` — float → TenBitParam → CC63 + CC_N
- `send_stepped_cc(cc_number, enum)` — enum → to_tx_value() → CC
- `send_on_off_cc(cc_number, on: bool)` — bool → 0/127 → CC
- `send_nrpn(param: NrpnParam)` — to_midi_sequence() → send each CC

**Tests (~88):** Every method category verified via MockOutput byte capture. Float boundaries, out-of-range rejection, stepped TX values, NRPN CC99/CC98/CC6 sequences.

---

## M4.2 — SysEx Transaction Manager

**Files:** `src/sysex/transaction.rs`, additions to `src/transport.rs`

### MidiInput Trait (added to transport.rs)
```rust
pub trait MidiInput: Send {
    fn receive(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>>;
}
```
- `MockMidiInput` — VecDeque-backed, queues pre-scripted responses, returns None when empty (simulates timeout)
- `MidirInput` (feature-gated) — wraps `midir::MidiInputConnection` with mpsc channel, `recv_timeout`

### SysexTransaction<'a, O: MidiOutput, I: MidiInput>
Borrows output + input for the transaction lifetime. Default 5-second timeout, configurable via `with_timeout()`.

**Core helpers:**
- `request_response(request, expected_fn)` — send, receive, check NAK, verify function ID
- `raw_request_response(request)` — for stored program dumps (program number bytes between header and 7-bit payload)
- `send_and_wait_ack(data)` — send, receive ACK/NAK status

**Public methods:**
- `request_current_program()` / `send_current_program(data)`
- `request_program(number)` / `send_program(number, data)`
- `request_global()` / `send_global(params)`
- `request_user_scale()` / `send_user_scale(scale)`
- `request_user_octave()` / `send_user_octave(octave)`
- `query_identity()`

**Error variants added to SysexError:**
- `Timeout(Duration)`, `NakReceived(SysexStatus)`, `UnexpectedResponse(u8)`

**Tests (~44):** Happy paths for all operations, timeout, NAK, unexpected response, send failure, custom timeout, channel propagation, sequential operations.

---

## M4.3 — Patch Builder

**File:** `src/builder/patch.rs`

**`PatchBuilder`** — consuming builder starting from `SynthParams::default()`.

- Float params **clamp** (don't error) for ergonomic builder UX: `value.clamp(0.0, 1.0) * 1023.0`
- `name(&str)` returns `Result<Self>` (name validation can fail)
- `build()` is infallible, pairs synth with default sequencer
- `build_with_sequencer(SequencerParams)` for custom sequencer

**Methods:** `name`, `vco1`, `vco2`, `sync`, `ring`, `cross_mod`, `multi_noise`, `multi_vpm`, `multi_user`, `mixer`, `filter`, `amp_eg`, `eg`, `lfo`, `mod_fx`, `delay`, `reverb`, `tuning`, `portamento`

**Tests (~26):** Default round-trip, named patch, VCO/filter/EG/LFO/FX configuration, float clamping behavior, chained builder.

---

## M4.4 — Sequence Builder

**File:** `src/builder/sequence.rs`

**`SequenceBuilder`** — consuming builder starting from `SequencerParams::default()`.

- Values **clamp** to valid ranges (BPM 10.0–300.0, step_length 1–16, swing -75..+75)
- Out-of-range step indices silently ignored
- `step(index, note, velocity)` enables the step in both `steps_on` and `active_steps` bitfields

**Methods:** `bpm`, `length`, `resolution`, `swing`, `gate_time`, `step`, `step_on`, `active_step`, `arp_gate_time`, `arp_rate`

**Tests (~28):** Default values (120 BPM, 16 steps, 1/16), BPM encoding, length clamping, step enable, chained builder.

---

## M4.5 — Integration Tests

**Directory:** `tests/`

### tests/cc_coverage.rs (~5 tests)
- All 63 recognized CC numbers produce CcParam via CcParamReceiver
- 30 unrecognized CC numbers return None
- CC number round-trip (cc_number() matches what was fed)
- 10-bit without/with CC63 preamble

### tests/nrpn_coverage.rs (~8 tests)
- Round-trip for each NrpnParam data type: ProgramName, VoiceModeType, TenBitSysex, EightBitHighRes, FourteenBitParam, bool, raw u8
- NrpnReceiver FSM reset on unexpected CC

### tests/sysex_flows.rs (~8 tests)
- Request/response round-trips (global, program) via MockMidiInput
- ACK/NAK handling
- Timeout behavior
- ProgramNumber boundary verification

### tests/builder_roundtrip.rs (~6 tests)
- Default patch serialization round-trip
- Named and complex patch round-trips
- Custom sequencer integration
- Float clamping verification
- ProgramData::SIZE constant check

---

## M4.6 — Polish & Publication Prep

### src/lib.rs Top-Level Rustdoc
- Crate overview paragraph
- ASCII architecture diagram (4-layer stack)
- Quick-start examples (PatchBuilder, RealtimeController)
- Feature flags table (midi-io, file-formats, std)
- MIDI Implementation version (Rev 1.01)

### Cargo.toml Metadata
Already complete: name, version, edition, description, license, repository, keywords, categories.

### Verification
- `cargo publish --dry-run` succeeds
- `cargo clippy --all-features -- -D warnings` clean
- `cargo test --all-features` — all 1127 tests pass
- `cargo build --no-default-features` — core compiles

---

## Implementation Order

```
M4.1 (controller)          — independent
M4.2 (transaction manager) — needs MidiInput trait
M4.3 (patch builder)       — independent
M4.4 (sequence builder)    — independent
M4.5 (integration tests)   — after M4.1–M4.4
M4.6 (polish)              — after M4.5
```

M4.1, M4.3, and M4.4 were implemented in parallel. M4.2 required adding MidiInput to transport.rs first. M4.5 and M4.6 were the finishing touches.

---

## Final Test Counts

| Milestone | Unit Tests | Integration | Doc Tests | Total |
|-----------|-----------|-------------|-----------|-------|
| M4.1 | 88 | — | 1 | 89 |
| M4.2 | 52 | — | 2 | 54 |
| M4.3 | 26 | — | 1 | 27 |
| M4.4 | 28 | — | 1 | 29 |
| M4.5 | — | 27 | — | 27 |
| M4.6 | — | — | 6 (lib.rs examples) | 6 |
| **Phase 4 Total** | 194 | 27 | 11 | 232 |

**Grand total across all phases: 1127 tests**
