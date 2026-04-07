//! A comprehensive Rust library for the Korg Minilogue XD synthesizer,
//! covering 100% of the MIDI Implementation chart (Revision 1.01). Provides
//! strongly typed CC and NRPN parameter maps, SysEx program and global data
//! blobs, a 16-step sequencer with motion sequences, sub-cent tuning tables,
//! user module management for logue SDK units, and high-level builder and
//! controller APIs for real-time performance and patch construction.
//!
//! # Architecture
//!
//! The crate is organized in four layers, each building on the one below:
//!
//! ```text
//!  +------------------------------------------------------------------+
//!  |  Layer 4 - High-Level API                                        |
//!  |  PatchBuilder, SequenceBuilder, RealtimeController               |
//!  +------------------------------------------------------------------+
//!  |  Layer 3 - SysEx Layer                                           |
//!  |  Program/Global blobs, tuning tables, user modules,              |
//!  |  SysexTransaction (request/response with ACK/NAK)                |
//!  +------------------------------------------------------------------+
//!  |  Layer 2 - Parameter Layer                                       |
//!  |  CC map (50+ params), NRPN map (29 params), 10-bit encoding,     |
//!  |  typed enums for all stepped/discrete parameters                 |
//!  +------------------------------------------------------------------+
//!  |  Layer 1 - Foundation                                            |
//!  |  7-bit codec, channel/realtime/common messages, MIDI I/O         |
//!  +------------------------------------------------------------------+
//! ```
//!
//! # Quick Start
//!
//! ## Building a patch
//!
//! ```
//! use minilogue_xd::builder::PatchBuilder;
//! use minilogue_xd::param::enums::*;
//!
//! let patch = PatchBuilder::new()
//!     .name("MyPad").unwrap()
//!     .vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.3)
//!     .filter(0.7, 0.2, CutoffDrive::Off, CutoffKeytrack::Off)
//!     .amp_eg(0.1, 0.5, 0.8, 0.4)
//!     .delay(true, DelaySubType::Stereo, 0.5, 0.5, 0.5)
//!     .build();
//!
//! assert_eq!(patch.synth.name.as_str(), "MyPad");
//! ```
//!
//! ## Real-time control
//!
//! ```
//! use minilogue_xd::controller::RealtimeController;
//! use minilogue_xd::message::types::U4;
//! use minilogue_xd::transport::MockOutput;
//!
//! let output = MockOutput::new();
//! let channel = U4::new(0).unwrap();
//! let mut ctrl = RealtimeController::new(output, channel);
//!
//! ctrl.set_cutoff(0.75).unwrap();
//! ctrl.set_vco1_wave(minilogue_xd::param::enums::VcoWave::Saw).unwrap();
//! ctrl.set_delay_on(true).unwrap();
//! ```
//!
//! # Feature Flags
//!
//! | Feature        | Default | Description                                     |
//! |----------------|---------|-------------------------------------------------|
//! | `midi-io`      | Yes     | Real MIDI I/O via `midir`                       |
//! | `file-formats` | Yes     | `.mnlgxdprog` / `.mnlgxdlib` file support       |
//! | `std`          | Yes     | Standard library support (disable for `no_std`) |
//!
//! # MIDI Implementation Version
//!
//! This crate targets the Korg Minilogue XD MIDI Implementation **Revision 1.01**,
//! supporting both the keyboard and desktop module variants.

pub mod builder;
pub mod codec;
pub mod controller;
pub mod device;
pub mod error;
pub mod message;
pub mod midi_file;
pub mod param;
pub mod sysex;

pub mod connection;
pub mod transport;

pub use error::{Error, Result};
