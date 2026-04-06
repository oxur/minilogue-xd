# Phase 3 Implementation Plan — SysEx Layer

## Context

Phases 1 (foundation, 121 tests) and 2 (parameter layer, 379 tests) are complete — 500 total tests passing. Phase 3 implements the Korg proprietary SysEx protocol: device identity, program/global data blobs, tuning tables, user module management, and poly chain. This is the largest phase with 8 milestones.

The spec's "384 Bytes (7bit) -> 336 Bytes (8bit)" for program dumps is a documentation error — the actual program blob is 1024 bytes (8-bit), confirmed by the workbench Python code (`xd_prog_bin.py`). The full TABLE 2 covers offsets 0–1023.

---

## File Organization

```
src/sysex/
  mod.rs              — module root, function ID constants, re-exports
  helpers.rs          — 10-bit read/write helpers for blob parsing
  frame.rs            — M3.1: header build/parse, SysexStatus enum
  identity.rs         — M3.1: Device Inquiry, Search Device
  enums.rs            — M3.2/3.4: SysEx-only enums (global + sequencer)
  global.rs           — M3.2: GlobalParams (TABLE 1, 63 bytes)
  program/
    mod.rs            — M3.5: ProgramData assembly, ProgramNumber, SysEx builders
    synth.rs          — M3.3: SynthParams (TABLE 2, 0–155), ProgramName
    sequencer.rs      — M3.4: SequencerParams (TABLE 2, 156–1023)
    file.rs           — M3.5: .mnlgxdprog/.mnlgxdlib/.mnlgxdpreset (feature-gated)
  tuning.rs           — M3.6: CentOffset, UserScale, UserOctave, MTS
  user_module.rs      — M3.7: logue SDK slot management, CRC32
  poly_chain.rs       — M3.8: PolyChainNoteOn/Off
```

Add `pub mod sysex;` to `src/lib.rs`.

---

## Error Strategy

Add to `src/error.rs`:
```rust
#[error("SysEx error: {0}")]
Sysex(#[from] SysexError),

#[cfg(feature = "file-formats")]
#[error(transparent)]
Io(#[from] std::io::Error),
```

New `SysexError` enum (in `src/error.rs` or `src/sysex/error.rs`):
- `InvalidHeader(String)`, `WrongFunctionId { expected, found }`, `PayloadTooShort { expected, actual }`
- `ChecksumMismatch { expected, actual }`, `InvalidProgramNumber(u16)`, `InvalidProgramNameChar(u8)`
- `Nak(SysexStatus)`, `Crc32Mismatch { expected, actual }`, `InvalidSlot { module_type, slot }`

---

## Cargo.toml Changes

```toml
[dependencies]
zip = { version = "2", optional = true, default-features = false, features = ["deflate"] }
crc32fast = { version = "1", optional = true }

[features]
file-formats = ["dep:zip", "dep:crc32fast"]
```

---

## M3.1 — SysEx Frame Builder & Parser

**Files:** `src/sysex/mod.rs`, `src/sysex/frame.rs`, `src/sysex/identity.rs`, `src/sysex/helpers.rs`

**Key types:**
- `pub mod function` — all SysEx function ID constants (0x0E, 0x10, 0x14, ..., 0x51, etc.)
- `SysexStatus` — 12 ACK/NAK codes (0x23–0x2F) with `is_error()`, `from_function_id()`, `to_function_id()`
- `SysexFrame { channel: U4, function_id: u8, data: Vec<u8> }` — parsed frame with 8-bit decoded payload
- `build_sysex(channel, function_id, data_8bit) -> Vec<u8>` — encodes payload via `codec::encode_7bit`, wraps with `[F0, 42, 3g, 00, 01, 51, function_id, ..., F7]`
- `build_sysex_request(channel, function_id) -> Vec<u8>` — no-data request
- `parse_sysex(bytes) -> Result<SysexFrame>` — validates header, decodes 7-bit payload
- Identity: `build_identity_request()`, `parse_identity_reply()` (universal non-realtime 0x7E)
- Search Device: `build_search_device(echo_id)`, `parse_search_device_reply()`

**7-bit boundary:** `build_sysex` calls `encode_7bit` internally. `parse_sysex` calls `decode_7bit`. All downstream `from_bytes()` methods work on 8-bit data exclusively.

**Helpers** (`src/sysex/helpers.rs`):
```rust
pub fn read_10bit(bytes: &[u8], offset: usize) -> u16  // big-endian H:L
pub fn write_10bit(bytes: &mut [u8], offset: usize, value: u16)
pub fn read_u16_be(bytes: &[u8], offset: usize) -> u16
pub fn write_u16_be(bytes: &mut [u8], offset: usize, value: u16)
pub fn read_motion_step(bytes: &[u8; 7]) -> [u16; 5]   // 5×10-bit in 7 bytes
pub fn write_motion_step(values: &[u16; 5]) -> [u8; 7]
```

**Reuse:** `U4`, `codec::encode_7bit`/`decode_7bit`

**Tests (~40):** Header for all 16 channels, roundtrip, reject wrong manufacturer/device ID, missing F0/F7, all SysexStatus codes, identity/search device format, proptest arbitrary payloads.

---

## M3.2 — Global Parameter Blob (TABLE 1)

**Files:** `src/sysex/global.rs`, `src/sysex/enums.rs`

**New enums** (in `src/sysex/enums.rs`, simpler `sysex_enum!` macro — no TX/RX bands, just value mapping):
- `DamperPolarity(2)`, `VelocityCurve(9)`, `KnobMode(3)`, `SyncUnit(2)`, `SyncPolarity(2)`, `MidiRoute(2)`, `ClockSource(3)`, `ParameterDisp(2)`, `PolyChainMode(3)`, `ShiftFunction(2)`

**`GlobalParams`** — 63 bytes (offsets 0–62):
- Bytes 0–3: 'GLOB' magic
- Bytes 4–27: scalar fields (master_tune, transpose, bools, enums)
- Bytes 28–59: 16 favorites × 2 bytes each (u16 program number pairs)
- Bytes 60–62: poly_chain, oscilloscope, shift_function

Note: favorites stored as 16-bit values. Byte 28=Fav1_lower_lo, byte 29=Fav1_lower_hi (little-endian? Need to verify against workbench). The spec says range 0–499.

**SysEx wrappers:** `build_global_request(ch)`, `build_global_dump(ch, params)`, `parse_global_dump(bytes)`

**Spec discrepancy:** Spec says "32 Bytes (8bit) -> 37 Bytes (7bit)" but TABLE 1 is 63 bytes. Likely 63 is correct (32 may be an old firmware subset). Implementation handles 63 bytes; if a 32-byte dump is received, parse what's available and default the rest.

**Tests (~35):** Roundtrip, magic validation, each enum variant, favorites encoding, wire-level roundtrip through frame layer, reject short blob.

---

## M3.3 — Program Blob Part A: Synth Params (TABLE 2, 0–155)

**File:** `src/sysex/program/synth.rs`

**`ProgramName`** newtype — 12 chars from restricted charset (note P1: space, !, #, $, %, &, ', (, ), *, comma, -, ., /, 0-9, :, ?, A-Z, a-z). `new()`, `from_str()` (pads with spaces), `as_str()` (trims trailing spaces), `Display`.

**`SynthParams`** — 156 bytes, ~60 fields. Key patterns:
- Enum fields at single-byte offsets: `VcoWave::from_program_value(bytes[22])`, `VcoOctave::from_program_value(bytes[23])`, etc.
- 10-bit fields at consecutive byte pairs: `helpers::read_10bit(bytes, 24)` for VCO1 pitch, etc.
- Bit-packed user param types: 2 bits each in bytes 148–149
- NOTE P2: VoiceModeDepth is a 10-bit value with context-dependent interpretation (Poly/Duo, Unison, Chord, Arp) — store as raw u16, provide interpretation helpers
- NOTE P3: VoiceModeType uses values 1–4 in blob (not 0-based!) — need offset

**Reuse:** All Phase 2 enums via `from_program_value()`, `helpers::read_10bit()`

**Tests (~50):** ProgramName valid/invalid chars, from_str padding, roundtrip, each enum field, each 10-bit field at boundaries, user param type bit extraction, known blob from workbench (golden test).

---

## M3.4 — Program Blob Part B: Sequencer (TABLE 2, 156–1023)

**Files:** `src/sysex/program/sequencer.rs`, additions to `src/sysex/enums.rs`

**New enums:** `StepResolution(5)`, `ArpRate(11)`, `MotionParameter(45)`

**`SequencerParams`** — 868 bytes:
- Headers: 'PRED' (156–159), 'SQ' (160–161), backward compat for old 'SEQD'
- BPM: u16 in bytes 164–165 (little-endian per Python code), range 100–3000 = 10.0–300.0
- Bitfield arrays: active_steps (162–163), step_on_off (170–171), motion_on_off (172–173) — each 16 bits
- Motion slots: 4 × 2 bytes (174–181), parameter ID + flags
- Motion slot step enables: 4 × 2 bytes (182–189)
- Step events: 16 × 52 bytes (190–1021)
- ARP: gate_time (1022), arp_rate (1023)

**`StepEvent`** — 52 bytes:
- Notes (8 bytes), velocities (8 bytes), gate_times+trigger_switches (8 bytes, 7 bits gate + 1 bit trigger per byte)
- Motion data: 4 slots × 7 bytes = 28 bytes (5 × 10-bit data points packed in 7 bytes per NOTE S3-2)

**Tests (~45):** Roundtrip, header magic (both 'PRED' and 'SEQD'), BPM boundaries, bitfield manipulation, step event with max notes, motion data packing unit tests, empty sequencer.

---

## M3.5 — Program Blob Assembly & File Formats

**Files:** `src/sysex/program/mod.rs`, `src/sysex/program/file.rs`

**`ProgramData { synth: SynthParams, sequencer: SequencerParams }`** — 1024 bytes total.
- `from_bytes(data)` splits at offset 156
- `to_bytes()` concatenates synth + sequencer

**`ProgramNumber(u16)`** — 0–499, with `bank()` (0–4, 100 programs each) and `slot_in_bank()` (0–99).

**SysEx wrappers:** Current program request/dump (0x10/0x40), stored program request/dump (0x1C/0x4C with LSB/MSB program number encoding).

**File formats** (behind `file-formats` feature):
- `.mnlgxdprog` — zip with `Prog_000.prog_bin` (1024 raw bytes)
- `.mnlgxdlib` — zip with multiple `Prog_NNN.prog_bin` entries + `FileInformation.xml`
- `.mnlgxdpreset` — zip, firmware v2+ (448-byte blobs — likely synth params only, no sequencer)
- Functions accept `impl Read + Seek` for flexibility (API-04)

**Tests (~40):** ProgramData roundtrip, ProgramNumber valid/invalid, bank decomposition, SysEx roundtrip, file format create/read in memory, golden test with workbench fixture.

---

## M3.6 — Tuning Data

**File:** `src/sysex/tuning.rs`

**`CentOffset(f32)`** — 14-bit fraction encoding (1 unit = 0.0061 cents). Encode/decode as 3 bytes: semitone + 14-bit fraction.

**`UserScale([CentOffset; 128])`** — TABLE 3, 384 bytes (128 × 3).
**`UserOctave([CentOffset; 12])`** — TABLE 4, 36 bytes (12 × 3).

**SysEx:** request/dump for scale (0x14/0x44) and octave (0x15/0x45).
**MTS:** Bulk dump with XOR checksum, single note tuning change.
**Helpers:** `equal_temperament()`, `just_major()`, `pythagorean()`.

**Tests (~35):** CentOffset precision, blob roundtrips, SysEx roundtrips, MTS checksum, temperament helpers produce known values.

---

## M3.7 — User Module Management

**File:** `src/sysex/user_module.rs`

**`UserModuleId`** — ModFx(1), DelayFx(2), ReverbFx(3), Osc(4) with `max_slots()`.
**Structs:** `ApiVersion`, `ModuleInfo` (TABLE 5), `SlotStatus` (TABLE 6), `SlotData` (TABLE 7).
**Builders:** 6 request types (0x17–0x1E). **Parsers:** 4 response types (0x47–0x4A).
**CRC32:** via `crc32fast` crate. Slot limits: ModFx/Osc=0–15, DelayFx/ReverbFx=0–7.
**File:** `.mnlgxdunit` (zip, feature-gated).

**Tests (~30):** Slot validation, request/response format, CRC32 test vectors, file roundtrip.

---

## M3.8 — Poly Chain SysEx

**File:** `src/sysex/poly_chain.rs`

**`U2(u8)`** — 0–3 (voice slot). **`PolyChainNoteOn`** — voice_slot, note, velocity, 21-bit pitch. **`PolyChainNoteOff`** — voice_slot, mute.
Build/parse for functions 0x60/0x61. 21-bit pitch = 3 × 7-bit bytes.

**Tests (~20):** U2 validation, roundtrips, pitch boundaries, mute flag.

---

## Implementation Order

```
M3.1 (frame + identity + helpers)     FIRST
  ├─→ M3.2 (global)                   validates blob pattern
  ├─→ M3.3 (synth params)             largest struct
  │     └─→ M3.4 (sequencer)          contiguous blob
  │           └─→ M3.5 (assembly + files)  combines + adds zip
  ├─→ M3.6 (tuning)                   independent
  ├─→ M3.7 (user modules)             independent
  └─→ M3.8 (poly chain)               smallest, independent
```

Recommended: M3.1 → M3.2 → M3.3 → M3.4 → M3.5 → M3.6 → M3.7 → M3.8

---

## Verification

After all milestones:
```bash
make check                         # build + lint + test
cargo test --all-features          # ~800+ tests expected
cargo build --no-default-features  # core compiles without zip/crc32/midir
make coverage                      # 95%+ target
```

Each milestone ends with `make check` passing.
