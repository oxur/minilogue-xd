---
number: 1
title: "`minilogue-xd` Rust Library — Project Plan"
author: "Claude Code"
component: All
tags: [change-me]
created: 2026-04-05
updated: 2026-04-05
state: Under Review
supersedes: null
superseded-by: null
version: 1.0
---

# `minilogue-xd` Rust Library — Project Plan

**Repository:** github.com/oxur/minilogue-xd
**Goal:** 100% coverage of the Korg Minilogue XD MIDI Implementation (Revision 1.01)
**Workbench references:** logue-sdk, mnlgxd.py (gekart gist), minilogue-xd-util (isnotinvain), loguetools (gazzar)

---

## Architectural Overview

The library is organized into four layers, each built on the one before it:

```
┌─────────────────────────────────────────────────────┐
│  Phase 4 · High-Level API & Ergonomics              │
│  (patch builder, realtime controller, sequencer)    │
├─────────────────────────────────────────────────────┤
│  Phase 3 · SysEx Layer                              │
│  (program/global blobs, user modules, tuning)       │
├─────────────────────────────────────────────────────┤
│  Phase 2 · Parameter Layer                          │
│  (CC, NRPN, 10-bit encoding, enum types)            │
├─────────────────────────────────────────────────────┤
│  Phase 1 · Foundation                               │
│  (project scaffold, codec, channel messages, I/O)   │
└─────────────────────────────────────────────────────┘
```

Each phase is broken into milestones. Each milestone is scoped to be executable by Claude Code in a single context window: focused, well-bounded, independently testable, and ending in a passing `cargo test`.

---

## Phase 1 — Foundation

*Establish the crate, the wire codec, and basic channel message types. Everything else depends on this.*

---

### Milestone 1.1 — Crate Scaffold & Workspace Layout

**Deliverables:**

- `Cargo.toml` with workspace structure and initial dependencies (`midir`, `thiserror`, `bitflags`)
- `src/lib.rs` with module declarations (stubs only)
- `src/error.rs` — unified `Error` enum and `Result<T>` alias
- `.gitignore` with `workbench/` excluded
- `README.md` skeleton with crate purpose and module map
- CI skeleton (`Cargo.lock` committed, `cargo test` passes on empty test suite)

**Acceptance:** `cargo build` and `cargo test` both pass cleanly.

---

### Milestone 1.2 — Korg 7-bit SysEx Codec

**Scope:** NOTE 1 of the MIDI implementation — the bijective encoding between 8-bit data bytes and 7-bit MIDI SysEx bytes. Every 7 bytes of real data become 8 bytes on the wire (high bits packed into a leading byte).

**Deliverables:**

- `src/codec.rs`
  - `encode_7bit(data: &[u8]) -> Vec<u8>` — encodes 8-bit buffer to MIDI 7-bit wire format
  - `decode_7bit(data: &[u8]) -> Result<Vec<u8>>` — decodes wire format back to 8-bit
  - Both functions handle arbitrary-length inputs including non-multiples of 7
- Unit tests covering:
  - Round-trip identity for all lengths 0–21
  - Known vectors cross-checked against the workbench Python implementations (mnlgxd.py, loguetools)
  - Error on malformed input (high bit set in data byte)

**Acceptance:** All codec tests pass. Round-trip property holds for 1000 random inputs.

---

### Milestone 1.3 — Channel Message Types (Transmit Side)

**Scope:** Section 1-1 of the implementation — all messages the XD *transmits*.

**Deliverables:**

- `src/message/channel.rs`
  - `NoteOn { channel: u4, key: u7, velocity: u7 }`
  - `NoteOff { channel: u4, key: u7, velocity: u7 }` (fixed velocity 64 on TX)
  - `ProgramChange { channel: u4, program: u7 }` (0–99)
  - `PitchBend { channel: u4, value: i14 }`
  - `ControlChange { channel: u4, controller: u7, value: u7 }`
  - `ChannelPressure { channel: u4, value: u7 }` (aftertouch, RX only, noted)
  - Trait `ToMidiBytes` with `fn to_bytes(&self) -> Vec<u8>`
  - Trait `FromMidiBytes` with `fn from_bytes(bytes: &[u8]) -> Result<Self>`
- Newtype wrappers for constrained integer ranges (`u4`, `u7`, `i14`) with range-checked constructors
- Unit tests: encode/decode round-trips, boundary values, error on out-of-range

**Acceptance:** All channel message types serialize to correct byte sequences per spec.

---

### Milestone 1.4 — Channel Message Types (Receive Additions)

**Scope:** Section 2-1 additions not present on transmit side — messages the XD *only receives*.

**Deliverables:**

- Added to `src/message/channel.rs`:
  - `AllSoundOff` (CC 120, value 0)
  - `AllNotesOff` (CC 123, value 0)
  - `LocalControl { on: bool }` (CC 122)
  - `Damper { value: u7 }` (CC 64 — on RX this is a full 0–127 damper, not a hold switch)
  - `BankSelect { msb: u7, lsb: u7 }` (CC 0 + CC 32 pair, with LSB 0–4 = 5 banks)
- `MidiMessage` enum wrapping all channel message variants
- `fn parse_midi_bytes(bytes: &[u8]) -> Result<MidiMessage>` dispatcher
- Tests: every message variant round-trips; bank select validated LSB 0–4 only

**Acceptance:** Parser correctly identifies all channel message types from raw bytes.

---

### Milestone 1.5 — System Realtime & Common Messages

**Scope:** Sections 1-2 (TX) and 2-3 (RX) — clock, transport, active sensing, song position pointer.

**Deliverables:**

- `src/message/realtime.rs`
  - `TimingClock`, `Start`, `Continue`, `Stop`, `ActiveSensing` — unit structs with `ToMidiBytes`
- `src/message/common.rs`
  - `SongPositionPointer { beats: u14 }` — with decoding note (pppp = (step *step_resolution* 16))
- Extended `MidiMessage` enum to include realtime and common variants
- Tests: byte sequences match spec exactly

**Acceptance:** All system message types serialize and parse correctly.

---

### Milestone 1.6 — MIDI I/O Transport Abstraction

**Scope:** A thin, testable abstraction over `midir` that the rest of the library uses for sending and receiving messages.

**Deliverables:**

- `src/transport.rs`
  - `trait MidiOutput: Send` with `fn send(&mut self, bytes: &[u8]) -> Result<()>`
  - `trait MidiInput: Send` with callback-based `fn listen<F: FnMut(MidiMessage)>(...)`
  - `MidirOutput` and `MidirInput` — concrete implementations wrapping `midir`
  - `MockOutput` — in-memory implementation for tests that captures sent bytes
- `src/connection.rs`
  - `MinilogueXd { output: Box<dyn MidiOutput>, channel: u4 }`
  - Constructor: `fn connect(port_name: &str, channel: u4) -> Result<Self>`
  - `fn send_message(&mut self, msg: &MidiMessage) -> Result<()>`
- Feature flag `midi-io` (default on) gates the `midir` dependency so the codec/message layers can be used without a MIDI runtime (useful for file manipulation)
- Tests using `MockOutput` to verify bytes sent match spec

**Acceptance:** `MinilogueXd::send_message` sends correct bytes. Tests pass without hardware.

---

## Phase 2 — Parameter Layer

*Typed representations of every CC and NRPN parameter, including the multi-byte encoding schemes.*

---

### Milestone 2.1 — Enum Types for Stepped Parameters

**Scope:** All parameters that use non-linear or discrete value mappings on the CC and program data layers. These are the `*2-xx` notes in Section 1-1 and the `*note Pxx` / `*6-xx` footnotes in Section 2-1.

**Deliverables:**

- `src/param/enums.rs` — one Rust enum per distinct stepped parameter type:
  - `VcoOctave` — 16', 8', 4', 2' (TX: vv=0,42,84,127 / RX: vv ranges 0–31,32–63,64–95,96–127)
  - `VcoWave` — SQR, TRI, SAW
  - `LfoWave` — SQR, TRI, SAW
  - `LfoMode` — OnShot, Normal, Bpm
  - `LfoTarget` — Cutoff, Shape, Pitch
  - `EgTarget` — Cutoff, Pitch2, Pitch
  - `MultiType` — Noise, Vpm, User
  - `MultiSelectNoise` — High, Low, Peak, Decim
  - `MultiSelectVpm` — Sin1..Sin4, Saw1..Saw2, Squ1..Squ2, Fat1..Fat2, Air1..Air2, Decay1..Decay2, Creep, Throat
  - `MultiSelectUser` — User1..User16
  - `ModFxType` — Chorus, Ensemble, Phaser, Flanger, User
  - `ModFxSubTypeChorus` — Stereo, Light, Deep, Triphase, Harmonic, Mono, Feedback, Vibrato
  - `ModFxSubTypeEnsemble` — Stereo, Light, Mono
  - `ModFxSubTypePhaser` — Stereo, Fast, Orange, Small, SmallReso, Black, Formant, Twinkle
  - `ModFxSubTypeFlanger` — Stereo, Light, Mono, HighSweep, MidSweep, PanSweep, MonoSweep, Triphase
  - `ModFxSubTypeUser` — User1..User16
  - `DelaySubType` — Stereo..Doubling, User1..User8 (20 variants)
  - `ReverbSubType` — Hall..Horror, User1..User8 (18 variants)
  - `CutoffKeytrack` — Off (0%), Half (50%), Full (100%)
  - `CutoffDrive` — Off (0%), Half (50%), Full (100%)
  - `Sync` — Off, On
  - `Ring` — Off, On
  - `VoiceModeType` — ArpLatch, Arp, Chord, Unison, Poly
  - `MicroTuning` — all 27 named tunings + User Scale 1–6 + User Octave 1–6
  - `CvInMode` — Modulation, CvGatePlus, CvGateMinus
  - `ModAssignTarget` — all 29 targets (GateTime..DelayDepth) — used by joystick assign, CV assign, aftertouch assign
  - `UserParamType` — Percent, Bipolar, Select
- Each enum implements:
  - `TryFrom<u8>` using the TX value mapping (exact values)
  - `fn from_rx_value(v: u8) -> Result<Self>` using the RX range mapping (band decode)
  - `fn to_tx_value(&self) -> u8` — canonical TX wire value
  - `fn to_program_value(&self) -> u8` — value used in program blob (often 0-based index)
  - `fmt::Display` for human-readable names matching Korg's naming

**Acceptance:** Every enum variant correctly round-trips through both TX and RX encodings. Boundary values tested explicitly.

---

### Milestone 2.2 — 10-bit Parameter Encoding

**Scope:** The `*1-4` / `*5-4` / `*3-1` / `*3-2` / `*3-3` footnotes — the multi-CC encoding schemes for high-resolution parameters.

**Deliverables:**

- `src/param/encoding.rs`
  - `struct TenBitParam(u16)` — wraps a 0–1023 value
    - `fn encode_cc(&self) -> (ControlChange, ControlChange)` — returns (CC63 LSB msg, CC6 MSB msg) in correct send order
    - `fn decode_cc(lsb_msg: &ControlChange, msb_msg: &ControlChange) -> Result<Self>` — reconstructs from CC63 + CC6 pair
  - `struct EightBitHighRes(u8)` — the `*3-2` variant (5-bit MSB + 3-bit LSB via CC63/CC6)
  - `struct FourteenBitParam(u16)` — the `*3-3` variant for MASTER VOLUME (0–16383, 7-bit MSB + 7-bit LSB)
  - `struct TenBitSysex(u16)` — the `*3-1` variant (for NRPN SysEx parameters like MULTI SHAPE)
- State machine `TenBitReceiver` that buffers CC63 and waits for CC6 to complete a value:
  - `fn feed(&mut self, cc: &ControlChange) -> Option<TenBitParam>`
  - Handles the "or assumed 0" case where CC63 is never sent

**Acceptance:** All encoding schemes produce correct byte sequences. Receiver correctly assembles values from ordered pairs. The "assumed 0" path tested explicitly.

---

### Milestone 2.3 — CC Parameter Map

**Scope:** Section 1-1 TX and Section 2-1 RX — all Control Change parameters as a typed, discoverable map.

**Deliverables:**

- `src/param/cc.rs`
  - `CcParam` enum with one variant per CC mapping (50+ variants):
    - Simple continuous: `AmpEgAttack(u7)`, `AmpEgDecay(u7)`, `AmpEgSustain(u7)`, `AmpEgRelease(u7)`, `EgAttack(u7)`, `EgDecay(u7)`, `EgInt(u7)`, `LfoRate(u7)`, `LfoInt(u7)`, `VoiceModeDepth(u7)`, `ModFxTime(u7)`, `ModFxDepth(u7)`, `MultiLevel(u7)`, `Vco1Pitch(u7)`, `Vco2Pitch(u7)`, `Vco1Shape(u7)`, `Vco2Shape(u7)`, `Vco1Level(u7)`, `Vco2Level(u7)`, `CrossModDepth(u7)`, `Cutoff(u7)`, `Resonance(u7)`, `MultiShape(u7)`, `MultiShiftShape(u7)`, `DelayTime(u7)`, `DelayDepth(u7)`, `DelayDryWet(u7)`, `ReverbTime(u7)`, `ReverbDepth(u7)`, `ReverbDryWet(u7)`, `CvIn1(u7)`, `CvIn2(u7)`
    - Modulation axes: `Modulation1(u7)`, `Modulation2(u7)`, `PortamentoTime(u7)`
    - High-resolution (10-bit, CC63+CC6): `AmpEgAttackHR(TenBitParam)`, `AmpEgDecayHR(TenBitParam)`, `AmpEgSustainHR(TenBitParam)`, `AmpEgReleaseHR(TenBitParam)`, `EgAttackHR(TenBitParam)`, `EgDecayHR(TenBitParam)`, `EgIntHR(TenBitParam)`, `LfoRateHR(TenBitParam)`, `LfoIntHR(TenBitParam)`, `VoiceModeDepthHR(TenBitParam)`, `ModFxTimeHR(TenBitParam)`, `ModFxDepthHR(TenBitParam)`, and all other `*5-4` parameters
    - Stepped (enum-valued): `Vco1Octave(VcoOctave)`, `Vco2Octave(VcoOctave)`, `Vco1Wave(VcoWave)`, `Vco2Wave(VcoWave)`, `LfoWave(LfoWave)`, `LfoMode(LfoMode)`, `LfoTarget(LfoTarget)`, `EgTarget(EgTarget)`, `MultiType(MultiType)`, `MultiSubTypeSelect(MultiSubTypeSelect)`, `ModFxType(ModFxType)`, `ModFxSubType(ModFxSubTypeSelector)`, `DelaySubType(DelaySubType)`, `ReverbSubType(ReverbSubType)`, `ModFxOnOff(bool)`, `DelayOnOff(bool)`, `ReverbOnOff(bool)`, `Sync(Sync)`, `Ring(Ring)`, `CutoffKeytrack(CutoffKeytrack)`, `CutoffDrive(CutoffDrive)`, `VoiceModeDepthNoDisplay(u7)`
    - Switch: `Hold(bool)` (CC64 on TX), `Damper(u7)` (CC64 on RX — noted asymmetry)
    - Bank: `BankSelectMsb(u7)`, `BankSelectLsb(u7)` (0–4)
    - LSB accumulator: `LsbValue(u7)` (CC63)
  - `fn CcParam::from_cc(controller: u7, value: u7) -> Result<CcParam>` — RX decode using band ranges
  - `fn CcParam::to_cc(&self) -> ControlChange` (or pair for high-res) — TX encode using exact values
  - `CC_NUMBER: u7` associated constant per variant
  - Note asymmetry between TX (exact values) and RX (ranges) in doc comments

**Acceptance:** Every CC number in the spec maps to exactly one `CcParam` variant. `from_cc` covers all RX ranges without gaps or overlaps. All 7-bit and enum variants serialize to correct CC numbers and values.

---

### Milestone 2.4 — NRPN Parameter Map

**Scope:** Section `*3` — all Non-Registered Parameter Number parameters.

**Deliverables:**

- `src/param/nrpn.rs`
  - `NrpnParam` enum (one variant per NRPN):
    - Program name chars: `ProgramName1(u7)` .. `ProgramName12(u7)` (ASCII, see note P1)
    - Voice: `VoiceModeType(VoiceModeType)`
    - Multi selects: `MultiSelectNoise(MultiSelectNoise)`, `MultiSelectVpm(MultiSelectVpm)`, `MultiSelectUser(MultiSelectUser)`
    - Multi shapes (10-bit): `MultiShapeNoise(TenBitSysex)`, `MultiShapeVpm(TenBitSysex)`, `MultiShapeUser(TenBitSysex)`, `MultiShiftShapeNoise(TenBitSysex)`, `MultiShiftShapeVpm(TenBitSysex)`, `MultiShiftShapeUser(TenBitSysex)`
    - Performance: `BendRangePlus(u8)`, `BendRangeMinus(u8)` (0–12), `JoystickAssignPlus(ModAssignTarget)`, `JoystickRangePlus(i8)`, `JoystickAssignMinus(ModAssignTarget)`, `JoystickRangeMinus(i8)` (0–200 = -100%..+100%)
    - CV: `CvInMode(CvInMode)`, `CvIn1Assign(ModAssignTarget)`, `CvIn1Range(i8)`, `CvIn2Assign(ModAssignTarget)`, `CvIn2Range(i8)`
    - Tuning: `MicroTuning(MicroTuning)`, `ScaleKey(i8)` (-12..+12), `ProgramTuning(i8)` (-50..+50 cents)
    - LFO: `LfoKeySync(bool)`, `LfoVoiceSync(bool)`, `LfoTargetOsc(LfoTargetOsc)`
    - EG/Amp: `EgVelocity(u7)`, `AmpVelocity(u7)`
    - Multi: `MultiOctave(VcoOctave)`, `MultiRouting(MultiRouting)`
    - Articulation: `EgLegato(bool)`, `PortamentoMode(PortamentoMode)`, `PortamentoBpmSync(bool)`
    - Level: `ProgramLevel(i8)` (0–120 = -18dB..+6dB)
    - VPM params (8-bit hi-res): `VpmParam1(EightBitHighRes)` .. `VpmParam6(EightBitHighRes)`
    - User params (8-bit hi-res): `UserParam1(EightBitHighRes)` .. `UserParam6(EightBitHighRes)`
    - Transpose: `ProgramTranspose(i8)` (1–25 = -12..+12)
    - Master volume: `MasterVolume(FourteenBitParam)` (poly-chain master only)
  - `fn NrpnParam::to_midi_sequence(&self) -> Vec<ControlChange>` — emits CC99 (MSB), CC98 (LSB), CC63 (data LSB if needed), CC6 (data MSB) in correct order
  - `struct NrpnReceiver` state machine that buffers CC99→CC98→(CC63)→CC6 and emits `NrpnParam` on completion
  - New enums introduced: `LfoTargetOsc` (All, Vco1Vco2, Vco2, Multi), `MultiRouting` (PreVcf, PostVcf), `PortamentoMode` (Auto, On)

**Acceptance:** Every NRPN in the spec (29 entries in the table, with sub-parameters) is covered. `to_midi_sequence` + `NrpnReceiver::feed` round-trip all variants. 8-bit and 14-bit encoding schemes tested.

---

## Phase 3 — SysEx Layer

*The Korg proprietary System Exclusive protocol: device identity, program/global data blobs, tuning, user modules, poly chain.*

---

### Milestone 3.1 — SysEx Frame Builder & Parser

**Scope:** The Korg SysEx header format and the ACK/NAK status codes — the wrapper used by all Korg SysEx messages.

**Deliverables:**

- `src/sysex/frame.rs`
  - `const KORG_MANUFACTURER_ID: u8 = 0x42`
  - `const MINILOGUE_XD_FAMILY_ID: u8 = 0x51`
  - `struct KorgSysexHeader { global_channel: u4 }` — serializes to `[F0, 42, 3g, 00, 01, 51]`
  - `fn build_sysex(channel: u4, function_id: u8, payload: &[u8]) -> Vec<u8>` — wraps payload in Korg frame with F7 terminator
  - `fn parse_sysex(bytes: &[u8]) -> Result<(u8, Vec<u8>)>` — returns (function_id, raw_payload); validates header, strips F7
  - `enum SysexStatus` — full ACK/NAK table: `DataLoadCompleted (0x23)`, `DataLoadError (0x24)`, `DataFormatError (0x26)`, `UserDataSizeError (0x27)`, `UserDataCrcError (0x28)`, `UserTargetError (0x29)`, `UserApiError (0x2A)`, `UserLoadSizeError (0x2B)`, `UserModuleError (0x2C)`, `UserSlotError (0x2D)`, `UserFormatError (0x2E)`, `UserInternalError (0x2F)`
  - `fn build_status_sysex(channel: u4, status: SysexStatus) -> Vec<u8>`
  - `fn parse_status_sysex(bytes: &[u8]) -> Result<SysexStatus>`
- `src/sysex/identity.rs`
  - Device Inquiry Request builder (universal non-realtime, function 0x06/0x01)
  - Device Inquiry Reply parser — extracts firmware version (major/minor LSB/MSB pairs)
  - `struct DeviceIdentity { firmware_major: u8, firmware_minor: u8 }`
  - Search Device Request builder (`F0, 42, 50, 00, echo_id, F7`)
  - Search Device Reply parser — extracts device ID, echo ID, firmware version

**Acceptance:** Frame builder produces byte-exact output per spec. Parser correctly identifies all function IDs. Status codes cover the full 0x23–0x2F range.

---

### Milestone 3.2 — Global Parameter Blob (TABLE 1)

**Scope:** The Global Data Dump (SysEx function 0x51) — the 63-byte global settings structure.

**Deliverables:**

- `src/sysex/global.rs`
  - `struct GlobalParams` — one field per TABLE 1 entry:
    - `master_tune: i8` (-50..+50 cents)
    - `transpose: i8` (-12..+12)
    - `metronome: bool`
    - `damper_pedal_polarity: DamperPolarity`
    - `local_sw: bool`
    - `velocity_curve: VelocityCurve` (Type1..Type8, Const127 — 9 variants from note G1)
    - `knob_mode: KnobMode` (Jump, Catch, Scale — note G2)
    - `sync_in_unit: SyncUnit`, `sync_out_unit: SyncUnit` (16th, 8th)
    - `sync_in_polarity: SyncPolarity`, `sync_out_polarity: SyncPolarity` (Rise, Fall)
    - `midi_route: MidiRoute` (UsbAndMidi, UsbOnly)
    - `midi_channel: u4` (0–15)
    - `clock_source: ClockSource` (AutoUsb, AutoMidi, Internal — note G3)
    - `en_rx_transport: bool`
    - `midi_rx_prog_chg: bool`, `midi_rx_cc: bool`, `midi_rx_pitch_bend: bool`
    - `midi_tx_prog_chg: bool`, `midi_tx_cc: bool`, `midi_tx_pitch_bend: bool`
    - `parameter_disp: ParameterDisp` (Normal, All)
    - `brightness: u8` (0–9)
    - `auto_power_off: bool`
    - `favorites: [FavoriteRange; 16]` where `FavoriteRange { lower: u16, upper: u16 }` (0–499)
    - `poly_chain: PolyChain` (Off, Master, Slave)
    - `oscilloscope: bool`
    - `shift_function: ShiftFunction` (Favorite, ActiveStep)
  - New enums: `DamperPolarity`, `VelocityCurve`, `KnobMode`, `SyncUnit`, `SyncPolarity`, `MidiRoute`, `ClockSource`, `ParameterDisp`, `PolyChain`, `ShiftFunction`
  - `fn GlobalParams::from_bytes(bytes: &[u8]) -> Result<Self>` — parses 63 8-bit bytes (post 7-bit decode)
  - `fn GlobalParams::to_bytes(&self) -> Vec<u8>` — serializes to 63 8-bit bytes (pre 7-bit encode)
  - `fn GlobalDataDumpRequest::build(channel: u4) -> Vec<u8>` — function 0x0E request
  - `fn GlobalDataDump::build(channel: u4, params: &GlobalParams) -> Vec<u8>` — function 0x51 with encoded payload
  - `fn GlobalDataDump::parse(bytes: &[u8]) -> Result<GlobalParams>` — validates header, decodes payload

**Acceptance:** `from_bytes(to_bytes(params)) == params` for all field combinations. Byte layout matches TABLE 1 offsets exactly (cross-checked against loguetools source).

---

### Milestone 3.3 — Program Parameter Blob, Part A: Synth Parameters (TABLE 2, offsets 0–155)

**Scope:** The first half of TABLE 2 — the 155 bytes of synth engine parameters. This is the most complex parsing task in the library due to 10-bit fields packed across byte pairs.

**Deliverables:**

- `src/sysex/program/synth.rs`
  - `struct SynthParams` covering TABLE 2 offsets 0–155:
    - Header magic `'PROG'` (bytes 0–3)
    - `name: ProgramName` — 12-char name with restricted charset (note P1)
    - `octave: i8` (0–4 = -2..+2)
    - `portamento: u8` (0–127)
    - `key_trig: bool`
    - `voice_mode_depth: VoiceModeDepthValue` — 10-bit, context-dependent interpretation (note P2):
      - Poly sub-enum: Poly vs Duo (with detune value)
      - Unison: detune cents
      - Chord: 14 chord types by range
      - Arp: 13 arp patterns by range
    - `voice_mode_type: VoiceModeType`
    - VCO1: `vco1_wave: VcoWave`, `vco1_octave: VcoOctave`, `vco1_pitch: u16` (10-bit, note P5), `vco1_shape: u16` (10-bit)
    - VCO2: same fields
    - `sync: bool`, `ring: bool`
    - `cross_mod_depth: u16` (10-bit)
    - Multi engine: `multi_type: MultiType`, `select_noise: MultiSelectNoise`, `select_vpm: MultiSelectVpm`, `select_user: MultiSelectUser`
    - Multi shapes (all 10-bit): `shape_noise`, `shape_vpm`, `shape_user`, `shift_shape_noise`, `shift_shape_vpm`, `shift_shape_user`
    - Mixer: `vco1_level: u16`, `vco2_level: u16`, `multi_level: u16` (all 10-bit)
    - Filter: `cutoff: u16`, `resonance: u16` (both 10-bit), `cutoff_drive: CutoffDrive`, `cutoff_keyboard_track: CutoffKeytrack`
    - Amp EG: `amp_eg_attack: u16`, `amp_eg_decay: u16`, `amp_eg_sustain: u16`, `amp_eg_release: u16` (all 10-bit)
    - EG: `eg_attack: u16`, `eg_decay: u16`, `eg_int: u16` (10-bit, with quadratic mapping note P10), `eg_target: EgTarget`
    - LFO: `lfo_wave: LfoWave`, `lfo_mode: LfoMode`, `lfo_rate: u16` (10-bit, with BPM sync interpretation note P11), `lfo_int: u16` (10-bit), `lfo_target: LfoTarget`
    - FX: `mod_fx_on: bool`, `mod_fx_type: ModFxType`, `mod_fx_sub: ModFxSubTypeSelector`, `mod_fx_time: u16`, `mod_fx_depth: u16` (10-bit)
    - Delay: `delay_on: bool`, `delay_sub_type: DelaySubType`, `delay_time: u16`, `delay_depth: u16`, `delay_dry_wet: u16` (10-bit)
    - Reverb: `reverb_on: bool`, `reverb_sub_type: ReverbSubType`, `reverb_time: u16`, `reverb_depth: u16`, `reverb_dry_wet: u16` (10-bit)
    - Mod routing: `bend_range_plus: u8`, `bend_range_minus: u8`, `joystick_assign_plus: ModAssignTarget`, `joystick_range_plus: i8`, `joystick_assign_minus: ModAssignTarget`, `joystick_range_minus: i8`
    - CV: `cv_in_mode: CvInMode`, `cv_in1_assign: ModAssignTarget`, `cv_in1_range: i8`, `cv_in2_assign: ModAssignTarget`, `cv_in2_range: i8`
    - Tuning: `micro_tuning: MicroTuning`, `scale_key: i8`, `program_tuning: i8`
    - LFO detail: `lfo_key_sync: bool`, `lfo_voice_sync: bool`, `lfo_target_osc: LfoTargetOsc`
    - Velocity: `cutoff_velocity: u8`, `amp_velocity: u8`
    - Multi routing: `multi_octave: VcoOctave`, `multi_routing: MultiRouting`
    - Articulation: `eg_legato: bool`, `portamento_mode: PortamentoMode`, `portamento_bpm_sync: bool`, `program_level: i8`
    - VPM params: `vpm_param1..vpm_param6: i16` (-100..+100%)
    - User params: `user_param1..user_param6: u16`, `user_param_types: [UserParamType; 6]`
    - `program_transpose: i8` (-12..+12)
    - `delay_dry_wet: u16`, `reverb_dry_wet: u16` (10-bit, bytes 151–154)
    - `midi_after_touch_assign: ModAssignTarget`
  - `fn SynthParams::from_bytes(bytes: &[u8]) -> Result<Self>` parsing bytes 0–155
  - `fn SynthParams::to_bytes(&self) -> [u8; 156]`
  - `struct ProgramName` with `TryFrom<[u8; 12]>`, `to_bytes`, `Display` using note P1 charset

**Acceptance:** Round-trip identity for all fields. 10-bit field extraction tested at boundaries (0, 511, 512, 1023). Cross-checked byte offsets against `mnlgxd.py` and `minilogue-xd-util` for known patches.

---

### Milestone 3.4 — Program Parameter Blob, Part B: Sequencer & Motion (TABLE 2, offsets 156–1023)

**Scope:** The sequencer portion of TABLE 2 — active steps, BPM, step configuration, step on/off, motion slot assignments, step events, and ARP settings.

**Deliverables:**

- `src/sysex/program/sequencer.rs`
  - `struct SequencerParams`:
    - Header: `'PRED'` (bytes 156–159), `'SQ'` (bytes 160–161, with backward compat note S1)
    - `active_steps: [bool; 16]` (bytes 162–163, bitfield)
    - `bpm: f32` (bytes 164–165, 100–3000 = 10.0–300.0 BPM, 10-bit packed)
    - `step_length: u8` (1–16)
    - `step_resolution: StepResolution` (1/16, 1/8, 1/4, 1/2, 1/1)
    - `swing: i8` (-75..+75)
    - `default_gate_time: u8` (0–72 = 0%..100%)
    - `steps_enabled: [bool; 16]` (bytes 170–171, bitfield)
    - `motion_steps_enabled: [bool; 16]` (bytes 172–173, bitfield)
    - `motion_slots: [MotionSlot; 4]` (bytes 174–181)
    - `motion_slot_steps: [[bool; 16]; 4]` (bytes 182–189, 4 × 2 bytes of bitfields)
    - `step_events: [StepEvent; 16]` (bytes 190–1021, 52 bytes each)
    - `arp_gate_time: u8` (byte 1022, 0–72)
    - `arp_rate: ArpRate` (byte 1023 — 11 variants, note S4)
  - `struct MotionSlot`:
    - `enabled: bool`, `smooth: bool`
    - `parameter: MotionParameter` — full enum of 45 assignable motion parameter IDs (note S2-1)
  - `enum MotionParameter` — all 45 variants from note S2-1 (None, Portamento, VoiceModeDepth, ... GateTime)
  - `struct StepEvent`:
    - `notes: [Option<u7>; 8]` — up to 8 notes per step (velocity 0 = NoEvent)
    - `velocities: [Option<u7>; 8]` — paired with notes
    - `gate_times: [GateTime; 8]` — 0–72 = 0%..100%, 73–127 = TIE
    - `trigger_switches: [bool; 8]`
    - `motion_data: [MotionStepData; 4]` — one per slot, 7 bytes each
  - `struct MotionStepData`: 5 × 10-bit data points packed per note S3-2
  - `enum GateTime`: `Percent(u8)` (0–100), `Tie`
  - `enum StepResolution`: Sixteenth, Eighth, Quarter, Half, Whole
  - `enum ArpRate`: Sixtyfourth..Fourth (11 variants, note S4)
  - `fn SequencerParams::from_bytes(bytes: &[u8]) -> Result<Self>` parsing bytes 156–1023
  - `fn SequencerParams::to_bytes(&self) -> [u8; 868]`

**Acceptance:** Round-trip identity. Motion data 10-bit pack/unpack (note S3-2) tested for all 5 data points. Step event parsing validated against sample patches from the workbench.

---

### Milestone 3.5 — Program Blob Assembly & SysEx Transactions

**Scope:** Combining SynthParams + SequencerParams into the full program blob, and implementing all program-related SysEx transactions.

**Deliverables:**

- `src/sysex/program/mod.rs`
  - `struct ProgramData { synth: SynthParams, sequencer: SequencerParams }`
  - `fn ProgramData::from_bytes(bytes: &[u8]) -> Result<Self>` — parses full 1024-byte (8-bit) / 1170-byte (7-bit) blob
  - `fn ProgramData::to_bytes(&self) -> Vec<u8>` — produces full 1024-byte blob
  - `struct ProgramNumber(u16)` — 0–499 with `TryFrom`, bank/program decomposition (5 banks × 100)
  - `fn CurrentProgramDumpRequest::build(channel: u4) -> Vec<u8>` (function 0x10)
  - `fn ProgramDumpRequest::build(channel: u4, program: ProgramNumber) -> Vec<u8>` (function 0x1C, with LSB/MSB encoding)
  - `fn CurrentProgramDataDump::build(channel: u4, data: &ProgramData) -> Vec<u8>` (function 0x40)
  - `fn CurrentProgramDataDump::parse(bytes: &[u8]) -> Result<ProgramData>` — handles 7-bit → 8-bit decode
  - `fn ProgramDataDump::build(channel: u4, program: ProgramNumber, data: &ProgramData) -> Vec<u8>` (function 0x4C)
  - `fn ProgramDataDump::parse(bytes: &[u8]) -> Result<(ProgramNumber, ProgramData)>`
- File format support:
  - `.mnlgxdprog` — zip archive containing `Prog_000.prog_bin` (raw 1024-byte blob)
  - `.mnlgxdlib` — zip archive containing multiple prog_bin entries + metadata
  - `.mnlgxdpreset` — zip archive, firmware v2+ format (448-byte blobs per KnobKraft findings)
  - `fn load_prog_file(path: &Path) -> Result<ProgramData>`
  - `fn save_prog_file(path: &Path, data: &ProgramData) -> Result<()>`
  - `fn load_lib_file(path: &Path) -> Result<Vec<(ProgramNumber, ProgramData)>>`

**Acceptance:** Load actual factory preset files from the workbench (`workbench/minilogue-xd-util/` and `workbench/loguetools/`), parse without error, round-trip back to identical bytes. Validate against `mnlgxd.py` output for at least 10 factory patches.

---

### Milestone 3.6 — Tuning Data (User Scale & User Octave)

**Scope:** Section 2-7 (Bulk Tuning Dump), Section 2-8 (Single Note Tuning Change), SysEx functions 0x14, 0x15, 0x44, 0x45, and TABLEs 3 and 4.

**Deliverables:**

- `src/sysex/tuning.rs`
  - `struct CentOffset(f32)` — a pitch offset in cents, with sub-cent precision via the 14-bit yyzz fraction format (1 unit = 0.0061 cents)
  - `fn encode_cent_offset(cents: f32) -> (u8, u8, u8)` — semitone, fraction MSB, fraction LSB
  - `fn decode_cent_offset(semitone: u8, frac_msb: u8, frac_lsb: u8) -> CentOffset`
  - `struct UserScale([CentOffset; 128])` — 128-note full tuning table (TABLE 3, 384 bytes)
  - `struct UserOctave([CentOffset; 12])` — 12-note octave tuning (TABLE 4, 36 bytes), with ±12 semitone range
  - `fn UserScaleDumpRequest::build(channel: u4) -> Vec<u8>` (function 0x14)
  - `fn UserOctaveDumpRequest::build(channel: u4) -> Vec<u8>` (function 0x15)
  - `fn UserScaleDump::build(channel: u4, scale: &UserScale) -> Vec<u8>` (function 0x44)
  - `fn UserScaleDump::parse(bytes: &[u8]) -> Result<UserScale>`
  - `fn UserOctaveDump::build(channel: u4, octave: &UserOctave) -> Vec<u8>` (function 0x45)
  - `fn UserOctaveDump::parse(bytes: &[u8]) -> Result<UserOctave>`
  - MIDI Tuning Standard support (universal non-realtime):
    - `fn BulkTuningDump::build(scale: &UserScale) -> Vec<u8>` — MTS format with checksum (XOR of all bytes except F0, checksum, F7)
    - `fn BulkTuningDump::parse(bytes: &[u8]) -> Result<UserScale>` — validates checksum, ignores tt/mm fields per spec
    - `fn SingleNoteTuningChange::build(changes: &[(u8, CentOffset)]) -> Vec<u8>` — up to 127 note changes per message
    - `fn SingleNoteTuningChange::parse(bytes: &[u8]) -> Result<Vec<(u8, CentOffset)>>`
  - Helper: `UserScale::equal_temperament() -> Self` — 12-TET reference
  - Helper: `UserOctave::just_major() -> Self`, `UserOctave::pythagorean() -> Self` — example non-ET tunings

**Acceptance:** Round-trip on all tuning data types. Checksum computation validated. Equal temperament helper produces 0-cent offsets for all semitones. MTS format matches the standard spec byte layout.

---

### Milestone 3.7 — User Module Management

**Scope:** SysEx functions 0x17–0x1E and 0x47–0x4A — querying and managing the user oscillator/FX slots (logue SDK units).

**Deliverables:**

- `src/sysex/user_module.rs`
  - `enum UserModuleId` — ModFx (1), DelayFx (2), ReverbFx (3), Osc (4)
  - `struct UserApiVersion { platform_id: u8, major: u8, minor: u8, patch: u8 }`
    - Platform ID 2 = minilogue xd (per function 0x47 spec)
  - `struct UserModuleInfo { max_slot_size: u32, max_program_size: u32, available_slot_count: u16 }` (TABLE 5)
  - `struct UserSlotStatus { platform_id: u8, module_id: UserModuleId, api_version: UserApiVersion, developer_id: u32, program_id: u32, program_version: (u8, u8, u8), program_name: String }` (TABLE 6)
  - `struct UserSlotData { payload_size: u32, payload_crc32: u32, payload: Vec<u8> }` (TABLE 7)
  - Request builders:
    - `fn UserApiVersionRequest::build(channel: u4) -> Vec<u8>` (function 0x17)
    - `fn UserModuleInfoRequest::build(channel: u4, module: UserModuleId) -> Vec<u8>` (function 0x18)
    - `fn UserSlotStatusRequest::build(channel: u4, module: UserModuleId, slot: u8) -> Vec<u8>` (function 0x19)
    - `fn UserSlotDataRequest::build(channel: u4, module: UserModuleId, slot: u8) -> Vec<u8>` (function 0x1A)
    - `fn ClearUserSlot::build(channel: u4, module: UserModuleId, slot: u8) -> Vec<u8>` (function 0x1B)
    - `fn ClearUserModule::build(channel: u4, module: UserModuleId) -> Vec<u8>` (function 0x1D)
    - `fn SwapUserData::build(channel: u4, module: UserModuleId, slot_a: u8, slot_b: u8) -> Vec<u8>` (function 0x1E)
  - Response parsers:
    - `fn UserApiVersion::parse(bytes: &[u8]) -> Result<UserApiVersion>` (function 0x47)
    - `fn UserModuleInfo::parse(bytes: &[u8]) -> Result<UserModuleInfo>` (function 0x48)
    - `fn UserSlotStatus::parse(bytes: &[u8]) -> Result<UserSlotStatus>` (function 0x49)
    - `fn UserSlotData::parse(bytes: &[u8]) -> Result<UserSlotData>` (function 0x4A)
  - Slot count limits: ModFx/Osc = 0–15 (16 slots), DelayFx/ReverbFx = 0–7 (8 slots) — validated in constructors
  - `.mnlgxdunit` file support: zip archive containing binary payload + metadata JSON
    - `fn load_unit_file(path: &Path) -> Result<UserSlotData>`

**Acceptance:** All request/response pairs round-trip. Slot count constraints validated with appropriate errors. CRC32 computation matches logue-sdk reference.

---

### Milestone 3.8 — Poly Chain SysEx

**Scope:** SysEx functions 0x60 and 0x61 — the Poly Chain Note On/Off messages used for multi-unit chaining.

**Deliverables:**

- `src/sysex/poly_chain.rs`
  - `struct PolyChainNoteOn { voice_slot: u2, note: u7, velocity: u7, pitch: u21 }` — pitch is 21-bit (hh:mm:ll) for sub-semitone precision
  - `struct PolyChainNoteOff { voice_slot: u2, mute: bool }`
  - `fn PolyChainNoteOn::build(channel: u4, note_on: &PolyChainNoteOn) -> Vec<u8>` (function 0x60)
  - `fn PolyChainNoteOn::parse(bytes: &[u8]) -> Result<PolyChainNoteOn>`
  - `fn PolyChainNoteOff::build(channel: u4, note_off: &PolyChainNoteOff) -> Vec<u8>` (function 0x61)
  - `fn PolyChainNoteOff::parse(bytes: &[u8]) -> Result<PolyChainNoteOff>`
  - Doc notes: voice slot 0–3 (2 bits), pitch field breakdown (H=bits 14–20, M=bits 7–13, L=bits 0–6)

**Acceptance:** Pitch field packing/unpacking round-trips. Voice slot range validated (0–3).

---

## Phase 4 — High-Level API & Ergonomics

*Typed, idiomatic Rust interfaces that make the library feel like a hardware synth module, not a byte-wrangler.*

---

### Milestone 4.1 — Real-Time Parameter Controller

**Scope:** A fluent API for sending individual parameters to the synth over a live MIDI connection, with correct encoding automatically selected (CC vs NRPN, 7-bit vs 10-bit).

**Deliverables:**

- `src/controller.rs`
  - `struct RealtimeController<O: MidiOutput> { output: O, channel: u4 }`
  - One method per CC parameter, e.g.:
    - `fn set_cutoff(&mut self, value: f32) -> Result<()>` (0.0–1.0 mapped to 10-bit, sent as CC63+CC6)
    - `fn set_vco1_wave(&mut self, wave: VcoWave) -> Result<()>` (sends enum TX value via CC)
    - `fn set_lfo_rate(&mut self, value: f32) -> Result<()>`
    - `fn set_amp_eg_attack(&mut self, value: f32) -> Result<()>` ... (all 10-bit CC params)
    - `fn set_mod_fx_type(&mut self, fx: ModFxType) -> Result<()>`
    - `fn set_delay_sub_type(&mut self, sub: DelaySubType) -> Result<()>`
    - `fn set_mod_fx_on(&mut self, on: bool) -> Result<()>` ... (all on/off switches)
    - `fn play_note(&mut self, note: u7, velocity: u7) -> Result<()>`
    - `fn stop_note(&mut self, note: u7) -> Result<()>`
    - `fn pitch_bend(&mut self, value: f32) -> Result<()>` (-1.0..+1.0)
    - `fn program_change(&mut self, program: ProgramNumber) -> Result<()>`
    - `fn modulation(&mut self, axis: JoystickAxis, value: f32) -> Result<()>`
    - `fn all_notes_off(&mut self) -> Result<()>`
  - One method per NRPN parameter, e.g.:
    - `fn set_bend_range_plus(&mut self, semitones: u8) -> Result<()>`
    - `fn set_micro_tuning(&mut self, tuning: MicroTuning) -> Result<()>`
    - `fn set_joystick_assign_plus(&mut self, target: ModAssignTarget) -> Result<()>`
    - `fn set_program_level(&mut self, db: f32) -> Result<()>` (-18.0..+6.0)
    - ... (all NRPN params)
  - `enum JoystickAxis { Plus, Minus }`
  - All float-valued methods use natural units (cents, dB, %, semitones) and document the mapping

**Acceptance:** `MockOutput` verifies byte sequences for each method. Float-to-integer mapping edge cases tested. No panics on any valid input.

---

### Milestone 4.2 — SysEx Transaction Manager

**Scope:** A stateful layer for managing the request→response pattern of SysEx conversations, with timeout handling and ACK/NAK processing.

**Deliverables:**

- `src/transaction.rs`
  - `struct SysexTransaction<O: MidiOutput> { output: O, channel: u4, timeout: Duration }`
  - `fn request_current_program(&mut self, input: &mut dyn MidiInput) -> Result<ProgramData>`
  - `fn request_program(&mut self, input: &mut dyn MidiInput, program: ProgramNumber) -> Result<ProgramData>`
  - `fn send_program(&mut self, input: &mut dyn MidiInput, program: ProgramNumber, data: &ProgramData) -> Result<()>` — sends, awaits ACK/NAK
  - `fn send_current_program(&mut self, input: &mut dyn MidiInput, data: &ProgramData) -> Result<()>`
  - `fn request_global(&mut self, input: &mut dyn MidiInput) -> Result<GlobalParams>`
  - `fn send_global(&mut self, input: &mut dyn MidiInput, params: &GlobalParams) -> Result<()>`
  - `fn request_user_scale(&mut self, input: &mut dyn MidiInput) -> Result<UserScale>`
  - `fn send_user_scale(&mut self, input: &mut dyn MidiInput, scale: &UserScale) -> Result<()>`
  - `fn request_user_octave(&mut self, input: &mut dyn MidiInput) -> Result<UserOctave>`
  - `fn send_user_octave(&mut self, input: &mut dyn MidiInput, octave: &UserOctave) -> Result<()>`
  - `fn request_user_slot_status(&mut self, input: &mut dyn MidiInput, module: UserModuleId, slot: u8) -> Result<UserSlotStatus>`
  - `fn send_user_slot(&mut self, input: &mut dyn MidiInput, module: UserModuleId, slot: u8, data: &UserSlotData) -> Result<()>`
  - `fn query_device_identity(&mut self, input: &mut dyn MidiInput) -> Result<DeviceIdentity>`
  - Error variants: `TransactionError::Timeout`, `TransactionError::NakReceived(SysexStatus)`, `TransactionError::UnexpectedResponse`

**Acceptance:** Full request/response flows tested using a `MockMidiPair` (mock output + scripted input). All error paths reachable and tested.

---

### Milestone 4.3 — Patch Builder

**Scope:** A high-level, ergonomic builder for constructing `ProgramData` values without manually setting every field.

**Deliverables:**

- `src/builder/patch.rs`
  - `struct PatchBuilder` with builder-pattern methods:
    - `fn name(self, name: &str) -> Self`
    - `fn voice_mode(self, mode: VoiceModeType) -> Self`
    - `fn vco1(self, wave: VcoWave, octave: VcoOctave, pitch_cents: f32, shape: f32) -> Self`
    - `fn vco2(self, wave: VcoWave, octave: VcoOctave, pitch_cents: f32, shape: f32) -> Self`
    - `fn sync(self, on: bool) -> Self`
    - `fn ring(self, on: bool) -> Self`
    - `fn cross_mod(self, depth: f32) -> Self`
    - `fn multi_noise(self, variant: MultiSelectNoise, shape: f32, shift_shape: f32) -> Self`
    - `fn multi_vpm(self, variant: MultiSelectVpm, shape: f32, shift_shape: f32) -> Self`
    - `fn multi_user(self, slot: u8, shape: f32, shift_shape: f32) -> Self`
    - `fn mixer(self, vco1: f32, vco2: f32, multi: f32) -> Self`
    - `fn filter(self, cutoff: f32, resonance: f32, drive: CutoffDrive, keytrack: CutoffKeytrack) -> Self`
    - `fn amp_eg(self, attack: f32, decay: f32, sustain: f32, release: f32) -> Self`
    - `fn eg(self, attack: f32, decay: f32, intensity: f32, target: EgTarget) -> Self`
    - `fn lfo(self, wave: LfoWave, mode: LfoMode, rate: f32, intensity: f32, target: LfoTarget) -> Self`
    - `fn mod_fx(self, fx_type: ModFxType, time: f32, depth: f32) -> Self`
    - `fn delay(self, sub_type: DelaySubType, time: f32, depth: f32, dry_wet: f32) -> Self`
    - `fn reverb(self, sub_type: ReverbSubType, time: f32, depth: f32, dry_wet: f32) -> Self`
    - `fn tuning(self, micro: MicroTuning, key: i8, program_cents: i8) -> Self`
    - `fn portamento(self, time: f32, mode: PortamentoMode, bpm_sync: bool) -> Self`
    - `fn build(self) -> Result<ProgramData>`
  - Default values matching the XD's factory init patch
  - `fn ProgramData::default() -> Self` — init patch equivalent

**Acceptance:** Builder produces valid `ProgramData` for common use cases. All float parameters clamp with a descriptive error (not panic) on out-of-range.

---

### Milestone 4.4 — Sequence Builder

**Scope:** A high-level builder for constructing `SequencerParams`, including the motion sequence system.

**Deliverables:**

- `src/builder/sequence.rs`
  - `struct SequenceBuilder`:
    - `fn bpm(self, bpm: f32) -> Self` (10.0–300.0)
    - `fn length(self, steps: u8) -> Self` (1–16)
    - `fn resolution(self, res: StepResolution) -> Self`
    - `fn swing(self, percent: i8) -> Self` (-75..+75)
    - `fn gate_time(self, percent: u8) -> Self` (0–100)
    - `fn step(self, index: u8, notes: &[u7], velocities: &[u7], gate: GateTime) -> Self`
    - `fn step_on(self, index: u8, on: bool) -> Self`
    - `fn active_step(self, index: u8, active: bool) -> Self`
    - `fn motion_slot(self, slot: u8, param: MotionParameter, smooth: bool) -> Self`
    - `fn motion_step(self, slot: u8, step: u8, values: &[f32; 5]) -> Self` (5 data points for smooth)
    - `fn motion_step_on(self, slot: u8, step: u8, on: bool) -> Self`
    - `fn arp_gate_time(self, percent: u8) -> Self`
    - `fn arp_rate(self, rate: ArpRate) -> Self`
    - `fn build(self) -> Result<SequencerParams>`
  - `fn SequencerParams::default() -> Self` — 16 steps, 120 BPM, 1/16 resolution

**Acceptance:** Builder produces valid `SequencerParams`. Motion data 10-bit packing tested from the builder path.

---

### Milestone 4.5 — Integration Tests & Workbench Validation

**Scope:** End-to-end tests using real data from the workbench clones, ensuring the library handles actual files from the community.

**Deliverables:**

- `tests/integration/` directory (not behind feature flag)
- Test fixtures from workbench (representative factory patches, a .mnlgxdlib file, a .mnlgxdunit file) — copied to `tests/fixtures/` and committed
- `tests/integration/round_trip.rs`:
  - Load each fixture .mnlgxdprog → parse → re-serialize → compare bytes
  - Load .mnlgxdlib → parse all programs → verify program count and spot-check named patches
  - Verify byte offsets for known parameters in known patches (cross-checked against mnlgxd.py output)
- `tests/integration/sysex_flows.rs`:
  - Full program dump request → mock response → parse → verify (using MockMidiPair)
  - Global dump request → mock response → parse → verify
  - Tuning scale dump → parse → verify equal temperament values
- `tests/integration/cc_coverage.rs`:
  - Every CC number in the spec produces a valid `CcParam` from `from_cc`
  - Every `CcParam` variant serializes to the correct CC number
- `tests/integration/nrpn_coverage.rs`:
  - Every NRPN number produces a valid sequence from `to_midi_sequence`
  - `NrpnReceiver` correctly assembles every NRPN variant from the sequence
- Documentation examples: every public type has a `# Examples` block that compiles via `cargo test --doc`

**Acceptance:** All integration tests pass. `cargo test --doc` passes. Zero panics under any valid input combination.

---

### Milestone 4.6 — Polish, Docs & Crate Publication Prep

**Scope:** Final pass for publication quality — no new functionality, only completeness and polish.

**Deliverables:**

- `src/lib.rs` top-level rustdoc: crate overview, architecture diagram (ASCII), quick-start example
- `CHANGELOG.md` — v0.1.0 entry covering all implemented features
- `MIDI_COVERAGE.md` — explicit mapping of every item in the MIDI implementation document to the Rust type/function that covers it (the definitive "100% coverage" checklist)
- `README.md` — full README with: purpose, install, quick-start code, feature flags, MIDI implementation version covered, reference to workbench
- Feature flags finalized:
  - `midi-io` (default on) — enables `midir` transport
  - `file-formats` (default on) — enables zip/file I/O for .mnlgxd* formats
  - `std` (default on) — allows `no_std` builds of the codec/message/param layers
- `cargo clippy -- -D warnings` passes
- `cargo fmt --check` passes
- All `pub` items have doc comments
- `Cargo.toml` metadata complete (authors, description, license, keywords, categories, repository)

**Acceptance:** `cargo publish --dry-run` succeeds. All clippy lints pass. Doc tests pass.

---

## Summary Table

| Phase | Milestone | Scope |
|-------|-----------|-------|
| 1 | 1.1 | Crate scaffold |
| 1 | 1.2 | 7-bit SysEx codec |
| 1 | 1.3 | Channel messages (TX) |
| 1 | 1.4 | Channel messages (RX additions) |
| 1 | 1.5 | System realtime & common messages |
| 1 | 1.6 | MIDI I/O transport abstraction |
| 2 | 2.1 | Enum types for all stepped parameters |
| 2 | 2.2 | 10-bit parameter encoding schemes |
| 2 | 2.3 | CC parameter map (50+ parameters) |
| 2 | 2.4 | NRPN parameter map (29+ parameters) |
| 3 | 3.1 | SysEx frame builder/parser + ACK/NAK |
| 3 | 3.2 | Global parameter blob (TABLE 1) |
| 3 | 3.3 | Program blob Part A: synth params (TABLE 2, 0–155) |
| 3 | 3.4 | Program blob Part B: sequencer & motion (TABLE 2, 156–1023) |
| 3 | 3.5 | Program blob assembly + file format support |
| 3 | 3.6 | Tuning data: user scale, user octave, MTS |
| 3 | 3.7 | User module management (logue SDK slots) |
| 3 | 3.8 | Poly chain SysEx |
| 4 | 4.1 | Real-time parameter controller |
| 4 | 4.2 | SysEx transaction manager |
| 4 | 4.3 | Patch builder |
| 4 | 4.4 | Sequence builder |
| 4 | 4.5 | Integration tests & workbench validation |
| 4 | 4.6 | Polish, docs & crate publication prep |

**Total: 4 phases, 24 milestones**

---

## Cross-Cutting Notes for Claude Code

**User Guide**

The official Korg product user guide (PDF) for the Minilogue has been converted to Markdown, with images included -- and most importantly, images have all been analysed and annotated with captions for easy AI-reading, here:

- `./docs/korg-user-guide/book.md`

**The workbench is ground truth.** Before implementing any parsing:

- `workbench/logue-sdk` — reference for user module binary format and CRC32 computation
- `workbench/gekart-mnlgxd.py` — field-by-field program blob parser; cross-check byte offsets
- `workbench/minilogue-xd-util` — higher-level Python model; useful for sequencer byte layout
- `workbench/loguetools` — cross-synth patch tool; useful for the .mnlgxd* file container formats

**Known documentation errata** (from KnobKraft Orm community findings):

- The MIDI implementation document has errors in the bank select/program number encoding — test against actual files, not only the spec
- `.mnlgxdpreset` files from firmware v2.10+ use 448-byte blobs, not the 1024-byte blobs described in the spec
- The `*note S1` backward-compat path (old 'SEQD' header → new 'SQ' header) must be handled on parse

**Dependency order is strict:** each milestone depends only on milestones before it. Claude Code should never be asked to implement Phase 3 before Phase 1 is complete and tested.

**Test-first on codecs.** The 7-bit codec (1.2) and 10-bit encoding (2.2) are the most failure-prone components. Write tests before implementation for these milestones.
