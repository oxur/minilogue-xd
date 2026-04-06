//! SysEx layer for the Korg Minilogue XD.
//!
//! This module handles the framing, parsing, and construction of Korg SysEx
//! messages, as well as the structured data blobs (global parameters, program
//! data, tuning tables, etc.) that travel inside those frames.

pub mod enums;
pub mod frame;
pub mod global;
pub mod helpers;
pub mod identity;
pub mod program;

/// Korg manufacturer SysEx ID (0x42).
pub const KORG_ID: u8 = 0x42;

/// Minilogue XD device ID bytes `[0x00, 0x01, 0x51]`.
pub const DEVICE_ID: [u8; 3] = [0x00, 0x01, 0x51];
