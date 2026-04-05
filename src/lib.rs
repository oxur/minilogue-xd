//! Rust library for the Korg Minilogue XD synthesizer.
//!
//! Covers 100% of the MIDI Implementation (Revision 1.01) — CC parameters,
//! NRPN, SysEx program and global data blobs, the 16-step sequencer with
//! motion sequences, sub-cent tuning tables, and user module management
//! for logue SDK units.

pub mod codec;
pub mod error;
pub mod message;

pub mod connection;
pub mod transport;

pub use error::{Error, Result};
