//! Fluent builders for program data and sequences.
//!
//! - [`PatchBuilder`] builds a [`ProgramData`](crate::sysex::program::ProgramData)
//!   from a [`SynthParams`](crate::sysex::program::SynthParams) starting point.
//! - [`SequenceBuilder`] builds a
//!   [`SequencerParams`](crate::sysex::program::SequencerParams).

pub mod patch;
pub mod sequence;

pub use patch::PatchBuilder;
pub use sequence::SequenceBuilder;
