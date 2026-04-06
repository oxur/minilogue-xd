//! Umbrella crate for Korg Minilogue synthesizer libraries.
//!
//! This crate re-exports the [`minilogue_xd`] library, providing a
//! convenient single dependency for projects targeting Korg Minilogue
//! synthesizers. Future crates (e.g., for the original Minilogue or
//! Minilogue Bass) will also be re-exported here.
//!
//! # Usage
//!
//! ```toml
//! [dependencies]
//! minilogue = "0.1"
//! ```
//!
//! ```rust,ignore
//! use minilogue::xd::controller::RealtimeController;
//! ```

/// Korg Minilogue XD synthesizer library.
pub use minilogue_xd as xd;
