# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust library providing 100% coverage of the Korg Minilogue XD MIDI Implementation (Revision 1.01). Covers CC parameters, NRPN, SysEx program/global data blobs, 16-step sequencer with motion sequences, sub-cent tuning tables, and logue SDK user module management. Works with both keyboard and desktop module variants.

The goal: treat the XD desktop module as a voice in an imaginary MIDI-based Eurorack system, with every parameter reachable from Rust in real time.

## Build Commands

```bash
make build            # Debug build
make build-release    # Optimized release build
make test             # Run all tests
make lint             # Clippy + format check
make format           # Auto-format with rustfmt
make check            # Build + lint + test
make check-all        # Build + lint + coverage
make coverage         # Text coverage report (cargo llvm-cov)
make coverage-html    # HTML coverage report -> target/llvm-cov/html/index.html
make docs             # Generate API docs -> target/doc/minilogue-xd/index.html
```

Running a single test:

```bash
cargo test test_name
cargo test module::tests::specific_test
```

## Architecture

Four-layer design, each built on the one below:

```
Phase 4 · High-Level API     (PatchBuilder, RealtimeController, SequenceBuilder)
Phase 3 · SysEx Layer         (program/global blobs, tuning, user modules)
Phase 2 · Parameter Layer     (CC map, NRPN map, 10-bit encoding, typed enums)
Phase 1 · Foundation          (crate scaffold, 7-bit codec, channel messages, I/O)
```

Planned module layout:

- `transport` -- MIDI I/O abstraction (midir backend, MockOutput for tests)
- `message` -- channel, realtime, common message types
- `codec` -- Korg 7-bit <-> 8-bit SysEx encoding (NOTE 1 of the spec)
- `param/enums` -- typed enums for all stepped/discrete parameters
- `param/encoding` -- 10-bit, 8-bit high-res, 14-bit parameter encoding
- `param/cc` -- 50+ CC parameter map
- `param/nrpn` -- 29 NRPN parameters, state machine receiver
- `sysex/program` -- 1024-byte program data blob (TABLE 2)
- `sysex/global` -- 63-byte global parameter blob (TABLE 1)
- `sysex/tuning` -- user scale, user octave, MIDI Tuning Standard
- `sysex/user_module` -- logue SDK slot management
- `sysex/transaction` -- request/response with ACK/NAK handling
- `controller` -- real-time parameter controller (fluent API)
- `builder` -- PatchBuilder and SequenceBuilder

Feature flags: `midi-io` (default, midir), `file-formats` (default, zip), `std` (default; disable for no_std).

## Writing Code

### Sound Design

**`assets/ai/SOUND_DESIGN.md`** -- Minilogue XD sound design skill: patch architecture, parameter relationships, classic sound recipes (pads, bass, leads, Berlin School, ambient, acid), sequencing patterns, genre quick reference, and idiomatic library API usage. **Read this when designing sounds, creating patches, or brainstorming electronica.**

### Rust Quality Guidelines

1. **`assets/ai/ai-rust/skills/claude/SKILL.md`** -- advanced Rust programming skill (**use this**)
2. **`assets/ai/ai-rust/guides/*.md`** -- comprehensive Rust guidelines referenced by the skill
3. **`assets/ai/CLAUDE-CODE-COVERAGE.md`** -- test coverage guide (95%+ target)

**Important:** `assets/ai/ai-rust` may be a symlink. If it doesn't resolve, clone it:

```bash
git clone https://github.com/oxur/ai-rust assets/ai/ai-rust
```

### Key Conventions

- Target 95%+ test coverage; never accept broken or ignored tests
- Always run `make format` after changes and `make lint` before testing
- Validate implementations against real `.mnlgxdlib`/`.mnlgxdprog` files and the workbench Python implementations, not just the Korg spec (which has known errata)
- The `workbench/` directory (gitignored) contains cloned reference implementations for cross-validation

### User Guide

The official Korg product user guide (PDF) for the Minilogue has been converted to Markdown, with images included -- and most importantly, images have all been analysed and annotated with captions for easy AI-reading, here:

- `./docs/korg-user-guide/book.md`

### Known Spec Errata

- Bank select and program number encoding differs from spec in practice -- validate against real files
- `.mnlgxdpreset` files from firmware v2.10+ use 448-byte program blobs, not 1024-byte; both formats must be supported

## Project Plan

The detailed project plan with milestone breakdowns lives at:
`docs/design/02-under-review/0001-minilogue-xd-rust-library-project-plan.md`

## Git Remotes

Pushes go to: macpro, github, codeberg (via `make push`)
