# Phase 2 Implementation Plan — Parameter Layer

## Context

Phase 1 is complete (121 tests, all passing). Phase 2 builds typed representations of every CC and NRPN parameter the Minilogue XD sends/receives, including multi-byte encoding schemes for high-resolution parameters. The authoritative spec is `workbench/minilogue-xd-util/assets/minilogue_xd__MIDIImp.txt`.

---

## File Organization

```
src/param/
  mod.rs        — module declarations, SteppedParam trait, re-exports
  enums.rs      — declarative macro + all ~30 stepped-parameter enums (M2.1)
  encoding.rs   — TenBitParam, EightBitHighRes, FourteenBitParam, TenBitSysex, TenBitReceiver (M2.2)
  cc.rs         — CcParam enum, CcParamReceiver (M2.3)
  nrpn.rs       — NrpnParam enum, NrpnReceiver state machine (M2.4)
```

Add `pub mod param;` to `src/lib.rs`.

---

## Milestone 2.1 — Enum Types for Stepped Parameters

**File:** `src/param/enums.rs` (~30 enums, ~200+ variants)

### TX/RX Asymmetry

Every stepped enum has dual encoding:

- **TX**: synth sends exact canonical values (e.g., 0, 64, 127)
- **RX**: synth accepts band ranges (e.g., 0–42, 43–85, 86–127)
- **Program blob**: 0-based index (0, 1, 2)

### Macro Strategy

Use `stepped_param_enum!` declarative macro with explicit TX values, RX band boundaries, program indices, and display names per variant. Handles both regular (VcoWave) and irregular (DelaySubType) spacing uniformly. Generates:

- Enum with `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` + `#[non_exhaustive]`
- `to_tx_value()`, `from_rx_value()`, `to_program_value()`, `from_program_value()`
- `TryFrom<u8>` (TX exact values), `fmt::Display`
- Impl of `SteppedParam` trait (defined in `mod.rs`)

### SteppedParam Trait (in `src/param/mod.rs`)

```rust
pub trait SteppedParam: Sized + Copy {
    fn to_tx_value(&self) -> u8;
    fn from_rx_value(v: u8) -> Result<Self>;
    fn to_program_value(&self) -> u8;
    fn from_program_value(v: u8) -> Result<Self>;
}
```

### Enum Catalog

**2-variant (TX: 0,127 / RX: 0–63, 64–127):** `Sync`, `Ring`

**3-variant (TX: 0,64,127 / RX: 0–42, 43–85, 86–127):** `VcoWave`, `LfoWave`, `LfoMode`, `LfoTarget`, `EgTarget`, `MultiType`, `CutoffKeytrack`, `CutoffDrive`, `ModFxSubTypeEnsemble`

**4-variant (TX: 0,42,84,127 / RX: 0–31, 32–63, 64–95, 96–127):** `VcoOctave`, `MultiSelectNoise`, `LfoTargetOsc`

**5-variant (irregular TX):** `ModFxType` (0,38,64,84,127), `VoiceModeType`

**8-variant (TX: 0,16,32,48,64,80,96,127):** `ModFxSubTypeChorus`, `ModFxSubTypePhaser`, `ModFxSubTypeFlanger`

**16-variant (TX: 0,8,16,...,112,127):** `MultiSelectVpm`, `MultiSelectUser`, `ModFxSubTypeUser`

**20-variant (irregular):** `DelaySubType` (TX: 0,7,13,20,26,32,39,45,52,58,64,71,77,84,90,96,103,109,116,127)

**18-variant (irregular):** `ReverbSubType` (TX: 0,8,15,22,29,36,43,50,57,64,72,79,86,93,100,107,117,127)

**Large enums:** `MicroTuning` (39 variants), `ModAssignTarget` (29 variants)

**NRPN-only (simple index mapping):** `CvInMode`, `UserParamType`, `MultiRouting`, `PortamentoMode`

### Tests (M2.1)

- TX round-trip: `TryFrom(variant.to_tx_value()) == Ok(variant)` for all variants
- RX full coverage: every value 0–127 maps to a variant (test all 128 values per enum)
- RX band boundaries: first/last value of each band
- Program round-trip: `from_program_value(to_program_value()) == Ok(variant)`
- Display spot checks
- Invalid TX/program values return Err

---

## Milestone 2.2 — 10-bit Parameter Encoding

**File:** `src/param/encoding.rs`

### Types

- **`TenBitParam(u16)`** — 0–1023. CC63 sends bits 0–2, parameter CC sends bits 3–9. (*1-4,*5-4)
- **`EightBitHighRes(u8)`** — 0–200. CC63 sends bits 0–2, CC6 sends bits 3–7. (*3-2)
- **`FourteenBitParam(u16)`** — 0–16383. CC63 sends bits 0–6, CC6 sends bits 7–13. (*3-3)
- **`TenBitSysex(u16)`** — same encoding as TenBitParam, distinct type for NRPN context. (*3-1)

All derive `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord`. Each has `new()`, `value()`, `TryFrom`, `From`, `Display`.

### Key Design: CC63 Precedes Different Targets

- **For CC params (*1-4, *5-4):** CC63 precedes the *parameter's own CC number* (not CC6)
- **For NRPN params (*3-1, *3-2, *3-3):** CC63 precedes CC6 within the NRPN sequence

This means `TenBitReceiver` is only for CC context. NRPN context is handled by `NrpnReceiver`.

### TenBitReceiver State Machine (for CC context)

```rust
pub struct TenBitReceiver { pending_lsb: Option<u8> }
```

- CC63 → buffers LSB, returns None
- Any high-res CC → combines with buffered LSB (or 0), returns `Some(TenBitParam)`
- Other CC → resets, returns None

### Tests (M2.2)

- Boundary values: 0, 7, 8, 511, 512, 1023 for TenBitParam; 0, 7, 200 for EightBitHighRes; 0, 127, 16383 for FourteenBitParam
- Out-of-range: 1024, 201, 16384
- TenBitReceiver: normal flow, assumed-zero, reset on foreign CC, double CC63
- Proptest round-trips

---

## Milestone 2.3 — CC Parameter Map

**File:** `src/param/cc.rs`

### CcParam Enum (flat, ~50 variants)

```rust
#[non_exhaustive]
pub enum CcParam {
    // High-res 10-bit (CC63 + own CC): AmpEgAttack(TenBitParam), ...
    // Stepped (enum): Vco1Octave(VcoOctave), Vco1Wave(VcoWave), ...
    // Simple continuous: Modulation1(U7), PortamentoTime(U7), ...
    // Context-dependent (raw U7): MultiSelect(U7), ModFxSubType(U7)
    // Special: DataEntryLsb(U7), Hold/Damper asymmetry
}
```

### CcParamReceiver

Stateful receiver that buffers CC63 and resolves high-res CC params:

```rust
pub struct CcParamReceiver { pending_lsb: Option<u8> }
fn feed(&mut self, cc: &ControlChange) -> Option<CcParam>
```

Dispatches on `cc.controller.value()` to classify as high-res, stepped, or simple.

### CcParam::to_cc_messages()

Returns `Vec<ControlChange>` — 1 CC for simple/stepped, 2 CCs (CC63 + param CC) for high-res.

### Context-Dependent CCs

CC 53 (Multi Select) and CC 59 (Mod FX Sub Type) depend on current synth state. Stored as raw `U7` in `CcParam` — caller resolves context using `MultiSelectVpm::from_rx_value()` etc.

### Tests (M2.3)

- Every spec CC number maps to correct variant
- to_cc + CcParamReceiver::feed round-trip
- High-res round-trip with/without CC63
- Stepped TX/RX values match spec
- Unknown CC returns None

---

## Milestone 2.4 — NRPN Parameter Map

**File:** `src/param/nrpn.rs`

### NrpnParam Enum (~60 variants)

Each variant maps to an (MSB, LSB) NRPN address:

- Program name chars: (0,0)–(0,11)
- VoiceModeType: (0,12)
- Multi selects/shapes: (0,13)–(0,21)
- Performance/CV/tuning/LFO/velocity/multi/articulation: (0,22)–(0,46)
- VPM params: (0,47)–(0,52)
- User params: (0,53)–(0,58)
- Transpose: (0,59), Aftertouch: (0,60)
- Master Volume: (1,0)

### NrpnParam::to_midi_sequence()

Emits CC99(MSB), CC98(LSB), [CC63(data LSB)], CC6(data MSB) in correct order.

### NrpnReceiver State Machine

4-state FSM: Idle → HaveMsb(CC99) → Addressed(CC99+CC98) → HaveDataLsb(+CC63) → resolved(+CC6).

```rust
enum NrpnState { Idle, HaveMsb(u8), Addressed { msb, lsb }, HaveDataLsb { msb, lsb, data_lsb } }
fn feed(&mut self, cc: &ControlChange) -> Option<NrpnParam>
fn resolve(msb, lsb, data_lsb, data_msb) -> Option<NrpnParam>
```

### Tests (M2.4)

- to_midi_sequence + NrpnReceiver::feed round-trip for every variant
- All NRPN addresses match spec table
- 10-bit, 8-bit, 14-bit boundary values
- State machine edge cases: out-of-order, interruption, double CC99, no CC63

---

## Implementation Sequence

1. **M2.1** — enums.rs (macro + all enums + tests) — largest milestone
2. **M2.2** — encoding.rs (4 types + TenBitReceiver + tests) — small, independent
3. **M2.3** — cc.rs (CcParam + CcParamReceiver + tests) — depends on M2.1 + M2.2
4. **M2.4** — nrpn.rs (NrpnParam + NrpnReceiver + tests) — depends on M2.1 + M2.2

---

## Verification

After all milestones:

```bash
make check                    # build + lint + test
cargo test --all-features     # ~400+ tests expected
make coverage                 # verify 95%+ target
cargo build --no-default-features  # core still compiles
```

Each milestone ends with `make check` passing before proceeding.
