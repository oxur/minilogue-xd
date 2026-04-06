//! Typed parameters for the Korg Minilogue XD.
//!
//! This module contains:
//! - [`SteppedParam`] — trait for discrete/stepped parameter enums
//! - [`enums`] — all stepped-parameter enums (oscillator waves, effects, etc.)
//! - [`encoding`] — high-resolution parameter types (10-bit, 14-bit, etc.)
//! - [`cc`] — CC parameter map and stateful receiver
//! - [`nrpn`] — NRPN parameter map, address table, and FSM receiver

pub mod cc;
pub mod encoding;
pub mod enums;
pub mod nrpn;

/// Trait for discrete (stepped) parameters that map between TX values,
/// RX value bands, and program-data indices.
///
/// Every stepped parameter on the Minilogue XD has three representations:
/// - **TX value**: the exact CC data byte sent when the knob is at that step.
/// - **RX band**: a range of incoming CC values that all resolve to this step.
/// - **Program value**: the index used inside SysEx program data blobs.
pub trait SteppedParam: Sized + Copy {
    /// Returns the exact CC data value transmitted for this variant.
    fn to_tx_value(&self) -> u8;

    /// Resolves an incoming CC data value to the matching variant.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::OutOfRange`] if `v` exceeds 127 or does not
    /// fall within any defined RX band.
    fn from_rx_value(v: u8) -> crate::Result<Self>;

    /// Returns the program-data index for this variant.
    fn to_program_value(&self) -> u8;

    /// Resolves a program-data index to the matching variant.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::OutOfRange`] if `v` is not a valid program index.
    fn from_program_value(v: u8) -> crate::Result<Self>;
}

pub use cc::{CcParam, CcParamReceiver};
pub use encoding::{EightBitHighRes, FourteenBitParam, TenBitParam, TenBitReceiver, TenBitSysex};
pub use enums::*;
pub use nrpn::{NrpnParam, NrpnReceiver};
