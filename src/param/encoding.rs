//! High-resolution parameter encoding types for the Minilogue XD.
//!
//! The Korg MIDI implementation uses several multi-byte encoding schemes
//! for parameters that exceed 7-bit resolution. This module provides
//! strongly typed wrappers for each scheme, handling the bit-split
//! between LSB and MSB bytes.

use std::fmt;

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// TenBitParam — 10-bit (0..=1023)
// ---------------------------------------------------------------------------

/// A 10-bit parameter value (0..=1023).
///
/// Used for CC parameters where CC63 sends bits 0-2 (LSB) and the
/// parameter's own CC number sends bits 3-9 (MSB).
///
/// # Wire encoding
///
/// ```text
/// LSB (CC63): value & 0x07          (bits 0-2)
/// MSB (CCnn): (value >> 3) & 0x7F   (bits 3-9)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenBitParam(u16);

impl TenBitParam {
    /// The minimum value (0).
    pub const MIN: u16 = 0;
    /// The maximum value (1023).
    pub const MAX: u16 = 1023;

    /// Creates a new `TenBitParam` if `value` is in range 0..=1023.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 1023.
    pub fn new(value: u16) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "TenBitParam",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner value.
    pub fn value(self) -> u16 {
        self.0
    }

    /// Returns the LSB byte (bits 0-2) for CC63.
    pub fn lsb(self) -> u8 {
        (self.0 & 0x07) as u8
    }

    /// Returns the MSB byte (bits 3-9) for the parameter's CC.
    pub fn msb(self) -> u8 {
        ((self.0 >> 3) & 0x7F) as u8
    }

    /// Reconstructs a `TenBitParam` from its LSB and MSB wire parts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the reconstructed value exceeds 1023
    /// (which can happen if `msb` exceeds 127).
    pub fn from_parts(lsb: u8, msb: u8) -> Result<Self> {
        let value = (u16::from(msb) << 3) | u16::from(lsb & 0x07);
        Self::new(value)
    }
}

impl TryFrom<u16> for TenBitParam {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self> {
        Self::new(value)
    }
}

impl From<TenBitParam> for u16 {
    fn from(val: TenBitParam) -> Self {
        val.0
    }
}

impl fmt::Display for TenBitParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// EightBitHighRes — 8-bit high resolution (0..=200)
// ---------------------------------------------------------------------------

/// An 8-bit high-resolution parameter value (0..=200).
///
/// Used for CC parameters where CC63 sends bits 0-2 (LSB) and CC6
/// sends bits 3-7 (MSB).
///
/// # Wire encoding
///
/// ```text
/// LSB (CC63): value & 0x07         (bits 0-2)
/// MSB (CC6):  (value >> 3) & 0x1F  (bits 3-7)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EightBitHighRes(u8);

impl EightBitHighRes {
    /// The minimum value (0).
    pub const MIN: u8 = 0;
    /// The maximum value (200).
    pub const MAX: u8 = 200;

    /// Creates a new `EightBitHighRes` if `value` is in range 0..=200.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 200.
    pub fn new(value: u8) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "EightBitHighRes",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner value.
    pub fn value(self) -> u8 {
        self.0
    }

    /// Returns the LSB byte (bits 0-2) for CC63.
    pub fn lsb(self) -> u8 {
        self.0 & 0x07
    }

    /// Returns the MSB byte (bits 3-7) for CC6.
    pub fn msb(self) -> u8 {
        (self.0 >> 3) & 0x1F
    }

    /// Reconstructs an `EightBitHighRes` from its LSB and MSB wire parts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the reconstructed value exceeds 200.
    pub fn from_parts(lsb: u8, msb: u8) -> Result<Self> {
        let value = ((msb & 0x1F) << 3) | (lsb & 0x07);
        Self::new(value)
    }
}

impl TryFrom<u8> for EightBitHighRes {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<EightBitHighRes> for u8 {
    fn from(val: EightBitHighRes) -> Self {
        val.0
    }
}

impl fmt::Display for EightBitHighRes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// FourteenBitParam — 14-bit (0..=16383)
// ---------------------------------------------------------------------------

/// A 14-bit parameter value (0..=16383).
///
/// Used for CC parameters where CC63 sends bits 0-6 (LSB) and CC6
/// sends bits 7-13 (MSB).
///
/// # Wire encoding
///
/// ```text
/// LSB (CC63): value & 0x7F          (bits 0-6)
/// MSB (CC6):  (value >> 7) & 0x7F   (bits 7-13)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FourteenBitParam(u16);

impl FourteenBitParam {
    /// The minimum value (0).
    pub const MIN: u16 = 0;
    /// The maximum value (16383).
    pub const MAX: u16 = 16383;

    /// Creates a new `FourteenBitParam` if `value` is in range 0..=16383.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 16383.
    pub fn new(value: u16) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "FourteenBitParam",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner value.
    pub fn value(self) -> u16 {
        self.0
    }

    /// Returns the LSB byte (bits 0-6) for CC63.
    pub fn lsb(self) -> u8 {
        (self.0 & 0x7F) as u8
    }

    /// Returns the MSB byte (bits 7-13) for CC6.
    pub fn msb(self) -> u8 {
        ((self.0 >> 7) & 0x7F) as u8
    }

    /// Reconstructs a `FourteenBitParam` from its LSB and MSB wire parts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the reconstructed value exceeds 16383.
    pub fn from_parts(lsb: u8, msb: u8) -> Result<Self> {
        let value = (u16::from(msb & 0x7F) << 7) | u16::from(lsb & 0x7F);
        Self::new(value)
    }
}

impl TryFrom<u16> for FourteenBitParam {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self> {
        Self::new(value)
    }
}

impl From<FourteenBitParam> for u16 {
    fn from(val: FourteenBitParam) -> Self {
        val.0
    }
}

impl fmt::Display for FourteenBitParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// TenBitSysex — 10-bit in NRPN context (0..=1023)
// ---------------------------------------------------------------------------

/// A 10-bit value in NRPN/SysEx context (0..=1023).
///
/// Same encoding as [`TenBitParam`] but semantically distinct — used for
/// NRPN data values rather than CC parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenBitSysex(u16);

impl TenBitSysex {
    /// The minimum value (0).
    pub const MIN: u16 = 0;
    /// The maximum value (1023).
    pub const MAX: u16 = 1023;

    /// Creates a new `TenBitSysex` if `value` is in range 0..=1023.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 1023.
    pub fn new(value: u16) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "TenBitSysex",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner value.
    pub fn value(self) -> u16 {
        self.0
    }

    /// Returns the LSB byte (bits 0-2).
    pub fn lsb(self) -> u8 {
        (self.0 & 0x07) as u8
    }

    /// Returns the MSB byte (bits 3-9).
    pub fn msb(self) -> u8 {
        ((self.0 >> 3) & 0x7F) as u8
    }

    /// Reconstructs a `TenBitSysex` from its LSB and MSB wire parts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the reconstructed value exceeds 1023.
    pub fn from_parts(lsb: u8, msb: u8) -> Result<Self> {
        let value = (u16::from(msb) << 3) | u16::from(lsb & 0x07);
        Self::new(value)
    }
}

impl TryFrom<u16> for TenBitSysex {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self> {
        Self::new(value)
    }
}

impl From<TenBitSysex> for u16 {
    fn from(val: TenBitSysex) -> Self {
        val.0
    }
}

impl fmt::Display for TenBitSysex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// TenBitReceiver — stateful CC63-preceded value accumulator
// ---------------------------------------------------------------------------

/// Stateful receiver for 10-bit CC parameters.
///
/// In the Minilogue XD protocol, a 10-bit parameter is sent as two
/// consecutive CC messages: CC63 carries the 3-bit LSB, then the
/// parameter's own CC carries the 7-bit MSB.
///
/// If the MSB arrives without a preceding CC63, the LSB is assumed to be 0.
///
/// # Example
///
/// ```
/// use minilogue_xd::param::encoding::TenBitReceiver;
///
/// let mut rx = TenBitReceiver::new();
///
/// // CC63 arrives with LSB = 5
/// rx.feed_lsb(5);
///
/// // Parameter CC arrives with MSB = 64
/// let param = rx.take_value(64);
/// assert_eq!(param.value(), (64 << 3) | 5);
/// ```
#[derive(Debug, Default)]
pub struct TenBitReceiver {
    pending_lsb: Option<u8>,
}

impl TenBitReceiver {
    /// Creates a new receiver with no pending LSB.
    pub fn new() -> Self {
        Self { pending_lsb: None }
    }

    /// Buffers a CC63 LSB value. Only the lowest 3 bits are used.
    pub fn feed_lsb(&mut self, value: u8) {
        self.pending_lsb = Some(value & 0x07);
    }

    /// Combines the buffered LSB (or 0 if none) with the given MSB to
    /// produce a [`TenBitParam`]. Consumes the pending LSB.
    ///
    /// The MSB is masked to 7 bits. The resulting value is always valid
    /// (0..=1023) because `(0x7F << 3) | 0x07 = 1023`.
    pub fn take_value(&mut self, msb: u8) -> TenBitParam {
        let lsb = self.pending_lsb.take().unwrap_or(0);
        let value = (u16::from(msb & 0x7F) << 3) | u16::from(lsb);
        // SAFETY: max value is (127 << 3) | 7 = 1023, always in range.
        TenBitParam(value)
    }

    /// Clears any pending LSB without producing a value.
    pub fn reset(&mut self) {
        self.pending_lsb = None;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // === TenBitParam ===

    #[test]
    fn ten_bit_new_min() {
        let p = TenBitParam::new(0).unwrap();
        assert_eq!(p.value(), 0);
    }

    #[test]
    fn ten_bit_new_max() {
        let p = TenBitParam::new(1023).unwrap();
        assert_eq!(p.value(), 1023);
    }

    #[test]
    fn ten_bit_new_out_of_range() {
        assert!(TenBitParam::new(1024).is_err());
    }

    #[test]
    fn ten_bit_lsb_msb() {
        // value = 517 = 0b10_0000_101
        // lsb = 0b101 = 5, msb = 0b1000000 = 64
        let p = TenBitParam::new(517).unwrap();
        assert_eq!(p.lsb(), 5);
        assert_eq!(p.msb(), 64);
    }

    #[test]
    fn ten_bit_from_parts_roundtrip() {
        for v in [0u16, 1, 7, 8, 100, 511, 512, 1023] {
            let p = TenBitParam::new(v).unwrap();
            let reconstructed = TenBitParam::from_parts(p.lsb(), p.msb()).unwrap();
            assert_eq!(reconstructed, p, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn ten_bit_from_parts_masks_lsb() {
        // Extra bits in LSB beyond bit 2 should be ignored.
        let p = TenBitParam::from_parts(0xFF, 0).unwrap();
        assert_eq!(p.value(), 7); // only bottom 3 bits
    }

    #[test]
    fn ten_bit_try_from() {
        assert!(TenBitParam::try_from(1023u16).is_ok());
        assert!(TenBitParam::try_from(1024u16).is_err());
    }

    #[test]
    fn ten_bit_into_u16() {
        let p = TenBitParam::new(500).unwrap();
        let raw: u16 = p.into();
        assert_eq!(raw, 500);
    }

    #[test]
    fn ten_bit_display() {
        let p = TenBitParam::new(42).unwrap();
        assert_eq!(format!("{p}"), "42");
    }

    #[test]
    fn ten_bit_ord() {
        let a = TenBitParam::new(100).unwrap();
        let b = TenBitParam::new(200).unwrap();
        assert!(a < b);
    }

    // === EightBitHighRes ===

    #[test]
    fn eight_bit_new_min() {
        let p = EightBitHighRes::new(0).unwrap();
        assert_eq!(p.value(), 0);
    }

    #[test]
    fn eight_bit_new_max() {
        let p = EightBitHighRes::new(200).unwrap();
        assert_eq!(p.value(), 200);
    }

    #[test]
    fn eight_bit_new_out_of_range() {
        assert!(EightBitHighRes::new(201).is_err());
    }

    #[test]
    fn eight_bit_lsb_msb() {
        // value = 200 = 0b11001000
        // lsb = 0b000 = 0, msb = 0b11001 = 25
        let p = EightBitHighRes::new(200).unwrap();
        assert_eq!(p.lsb(), 0);
        assert_eq!(p.msb(), 25);
    }

    #[test]
    fn eight_bit_from_parts_roundtrip() {
        for v in [0u8, 1, 7, 8, 100, 199, 200] {
            let p = EightBitHighRes::new(v).unwrap();
            let reconstructed = EightBitHighRes::from_parts(p.lsb(), p.msb()).unwrap();
            assert_eq!(reconstructed, p, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn eight_bit_from_parts_masks_bits() {
        // MSB is masked to 5 bits: 0xFF & 0x1F = 31.
        // (31 << 3) | 7 = 255, which exceeds 200 -- should error.
        assert!(EightBitHighRes::from_parts(0x07, 0xFF).is_err());

        // With valid MSB: (25 << 3) | 0 = 200, the maximum value.
        let p = EightBitHighRes::from_parts(0, 25).unwrap();
        assert_eq!(p.value(), 200);
    }

    #[test]
    fn eight_bit_try_from() {
        assert!(EightBitHighRes::try_from(200u8).is_ok());
        assert!(EightBitHighRes::try_from(201u8).is_err());
    }

    #[test]
    fn eight_bit_into_u8() {
        let p = EightBitHighRes::new(150).unwrap();
        let raw: u8 = p.into();
        assert_eq!(raw, 150);
    }

    #[test]
    fn eight_bit_display() {
        let p = EightBitHighRes::new(100).unwrap();
        assert_eq!(format!("{p}"), "100");
    }

    // === FourteenBitParam ===

    #[test]
    fn fourteen_bit_new_min() {
        let p = FourteenBitParam::new(0).unwrap();
        assert_eq!(p.value(), 0);
    }

    #[test]
    fn fourteen_bit_new_max() {
        let p = FourteenBitParam::new(16383).unwrap();
        assert_eq!(p.value(), 16383);
    }

    #[test]
    fn fourteen_bit_new_out_of_range() {
        assert!(FourteenBitParam::new(16384).is_err());
    }

    #[test]
    fn fourteen_bit_lsb_msb() {
        // value = 16383 = 0b11_1111_1111_1111
        // lsb = 0b1111111 = 127, msb = 0b1111111 = 127
        let p = FourteenBitParam::new(16383).unwrap();
        assert_eq!(p.lsb(), 127);
        assert_eq!(p.msb(), 127);
    }

    #[test]
    fn fourteen_bit_from_parts_roundtrip() {
        for v in [0u16, 1, 127, 128, 8192, 16382, 16383] {
            let p = FourteenBitParam::new(v).unwrap();
            let reconstructed = FourteenBitParam::from_parts(p.lsb(), p.msb()).unwrap();
            assert_eq!(reconstructed, p, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn fourteen_bit_from_parts_masks_bits() {
        // Both LSB and MSB masked to 7 bits.
        let p = FourteenBitParam::from_parts(0xFF, 0xFF).unwrap();
        assert_eq!(p.value(), 16383); // (127 << 7) | 127
    }

    #[test]
    fn fourteen_bit_try_from() {
        assert!(FourteenBitParam::try_from(16383u16).is_ok());
        assert!(FourteenBitParam::try_from(16384u16).is_err());
    }

    #[test]
    fn fourteen_bit_into_u16() {
        let p = FourteenBitParam::new(8192).unwrap();
        let raw: u16 = p.into();
        assert_eq!(raw, 8192);
    }

    #[test]
    fn fourteen_bit_display() {
        let p = FourteenBitParam::new(1000).unwrap();
        assert_eq!(format!("{p}"), "1000");
    }

    // === TenBitSysex ===

    #[test]
    fn ten_bit_sysex_new_min() {
        let p = TenBitSysex::new(0).unwrap();
        assert_eq!(p.value(), 0);
    }

    #[test]
    fn ten_bit_sysex_new_max() {
        let p = TenBitSysex::new(1023).unwrap();
        assert_eq!(p.value(), 1023);
    }

    #[test]
    fn ten_bit_sysex_new_out_of_range() {
        assert!(TenBitSysex::new(1024).is_err());
    }

    #[test]
    fn ten_bit_sysex_from_parts_roundtrip() {
        for v in [0u16, 1, 7, 8, 100, 511, 512, 1023] {
            let p = TenBitSysex::new(v).unwrap();
            let reconstructed = TenBitSysex::from_parts(p.lsb(), p.msb()).unwrap();
            assert_eq!(reconstructed, p, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn ten_bit_sysex_lsb_msb() {
        let p = TenBitSysex::new(517).unwrap();
        assert_eq!(p.lsb(), 5);
        assert_eq!(p.msb(), 64);
    }

    #[test]
    fn ten_bit_sysex_try_from() {
        assert!(TenBitSysex::try_from(1023u16).is_ok());
        assert!(TenBitSysex::try_from(1024u16).is_err());
    }

    #[test]
    fn ten_bit_sysex_into_u16() {
        let p = TenBitSysex::new(500).unwrap();
        let raw: u16 = p.into();
        assert_eq!(raw, 500);
    }

    #[test]
    fn ten_bit_sysex_display() {
        let p = TenBitSysex::new(42).unwrap();
        assert_eq!(format!("{p}"), "42");
    }

    // === TenBitReceiver ===

    #[test]
    fn receiver_normal_flow() {
        let mut rx = TenBitReceiver::new();
        rx.feed_lsb(5);
        let p = rx.take_value(64);
        assert_eq!(p.value(), (64 << 3) | 5);
    }

    #[test]
    fn receiver_assumed_zero() {
        let mut rx = TenBitReceiver::new();
        // No feed_lsb, so LSB defaults to 0.
        let p = rx.take_value(64);
        assert_eq!(p.value(), 64 << 3);
    }

    #[test]
    fn receiver_reset() {
        let mut rx = TenBitReceiver::new();
        rx.feed_lsb(7);
        rx.reset();
        let p = rx.take_value(10);
        assert_eq!(p.value(), 10 << 3); // LSB was cleared
    }

    #[test]
    fn receiver_lsb_consumed() {
        let mut rx = TenBitReceiver::new();
        rx.feed_lsb(3);
        let p1 = rx.take_value(1);
        assert_eq!(p1.value(), (1 << 3) | 3);
        // Second take without new feed_lsb should use 0.
        let p2 = rx.take_value(1);
        assert_eq!(p2.value(), 1 << 3);
    }

    #[test]
    fn receiver_lsb_only_low_3_bits() {
        let mut rx = TenBitReceiver::new();
        rx.feed_lsb(0xFF); // should mask to 7
        let p = rx.take_value(0);
        assert_eq!(p.value(), 7);
    }

    #[test]
    fn receiver_msb_masked() {
        let mut rx = TenBitReceiver::new();
        rx.feed_lsb(0);
        // MSB 0xFF masked to 0x7F = 127
        let p = rx.take_value(0xFF);
        assert_eq!(p.value(), 127 << 3);
    }

    #[test]
    fn receiver_max_value() {
        let mut rx = TenBitReceiver::new();
        rx.feed_lsb(7);
        let p = rx.take_value(127);
        assert_eq!(p.value(), 1023);
    }

    #[test]
    fn receiver_default() {
        let rx = TenBitReceiver::default();
        assert!(format!("{rx:?}").contains("None"));
    }

    // === Proptest ===

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn ten_bit_roundtrip(v in 0u16..=1023) {
                let p = TenBitParam::new(v).unwrap();
                let r = TenBitParam::from_parts(p.lsb(), p.msb()).unwrap();
                prop_assert_eq!(p, r);
            }

            #[test]
            fn ten_bit_sysex_roundtrip(v in 0u16..=1023) {
                let p = TenBitSysex::new(v).unwrap();
                let r = TenBitSysex::from_parts(p.lsb(), p.msb()).unwrap();
                prop_assert_eq!(p, r);
            }

            #[test]
            fn eight_bit_roundtrip(v in 0u8..=200) {
                let p = EightBitHighRes::new(v).unwrap();
                let r = EightBitHighRes::from_parts(p.lsb(), p.msb()).unwrap();
                prop_assert_eq!(p, r);
            }

            #[test]
            fn fourteen_bit_roundtrip(v in 0u16..=16383) {
                let p = FourteenBitParam::new(v).unwrap();
                let r = FourteenBitParam::from_parts(p.lsb(), p.msb()).unwrap();
                prop_assert_eq!(p, r);
            }

            #[test]
            fn ten_bit_reject_out_of_range(v in 1024u16..=65535) {
                prop_assert!(TenBitParam::new(v).is_err());
            }

            #[test]
            fn eight_bit_reject_out_of_range(v in 201u8..=255) {
                prop_assert!(EightBitHighRes::new(v).is_err());
            }

            #[test]
            fn fourteen_bit_reject_out_of_range(v in 16384u16..=65535) {
                prop_assert!(FourteenBitParam::new(v).is_err());
            }

            #[test]
            fn receiver_always_valid(lsb in 0u8..=255, msb in 0u8..=255) {
                let mut rx = TenBitReceiver::new();
                rx.feed_lsb(lsb);
                let p = rx.take_value(msb);
                prop_assert!(p.value() <= 1023);
            }
        }
    }
}
