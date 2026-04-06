//! Constrained integer newtypes for MIDI values.
//!
//! These types enforce valid ranges at construction time, making it impossible
//! to represent out-of-spec values in the type system.

use std::fmt;

use crate::error::{Error, Result};

/// A 4-bit unsigned integer (0..=15), used for MIDI channel numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct U4(u8);

impl U4 {
    /// The minimum value (0).
    pub const MIN: u8 = 0;
    /// The maximum value (15).
    pub const MAX: u8 = 15;

    /// Creates a new `U4` if `value` is in range 0..=15.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 15.
    pub fn new(value: u8) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "U4",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner `u8` value.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for U4 {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<U4> for u8 {
    fn from(val: U4) -> Self {
        val.0
    }
}

impl fmt::Display for U4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A 7-bit unsigned integer (0..=127), used for note numbers, velocities, and CC values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct U7(u8);

impl U7 {
    /// The minimum value (0).
    pub const MIN: u8 = 0;
    /// The maximum value (127).
    pub const MAX: u8 = 127;

    /// Creates a new `U7` if `value` is in range 0..=127.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 127.
    pub fn new(value: u8) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "U7",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner `u8` value.
    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for U7 {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<U7> for u8 {
    fn from(val: U7) -> Self {
        val.0
    }
}

impl fmt::Display for U7 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A 14-bit unsigned integer (0..=16383), used for Song Position Pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct U14(u16);

impl U14 {
    /// The minimum value (0).
    pub const MIN: u16 = 0;
    /// The maximum value (16383).
    pub const MAX: u16 = 16383;

    /// Creates a new `U14` if `value` is in range 0..=16383.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` exceeds 16383.
    pub fn new(value: u16) -> Result<Self> {
        if value > Self::MAX {
            return Err(Error::OutOfRange {
                type_name: "U14",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner `u16` value.
    pub fn value(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for U14 {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self> {
        Self::new(value)
    }
}

impl From<U14> for u16 {
    fn from(val: U14) -> Self {
        val.0
    }
}

impl fmt::Display for U14 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A signed 14-bit integer (-8192..=8191), used for Pitch Bend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct I14(i16);

impl I14 {
    /// The minimum value (-8192).
    pub const MIN: i16 = -8192;
    /// The maximum value (8191).
    pub const MAX: i16 = 8191;

    /// Creates a new `I14` if `value` is in range -8192..=8191.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if `value` is outside the valid range.
    pub fn new(value: i16) -> Result<Self> {
        if !(Self::MIN..=Self::MAX).contains(&value) {
            return Err(Error::OutOfRange {
                type_name: "I14",
                value: i64::from(value),
                min: i64::from(Self::MIN),
                max: i64::from(Self::MAX),
            });
        }
        Ok(Self(value))
    }

    /// Returns the inner `i16` value.
    pub fn value(self) -> i16 {
        self.0
    }
}

impl TryFrom<i16> for I14 {
    type Error = Error;

    fn try_from(value: i16) -> Result<Self> {
        Self::new(value)
    }
}

impl From<I14> for i16 {
    fn from(val: I14) -> Self {
        val.0
    }
}

impl fmt::Display for I14 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- U4 tests ---

    #[test]
    fn u4_new_min() {
        let v = U4::new(0).unwrap();
        assert_eq!(v.value(), 0);
    }

    #[test]
    fn u4_new_max() {
        let v = U4::new(15).unwrap();
        assert_eq!(v.value(), 15);
    }

    #[test]
    fn u4_new_out_of_range() {
        assert!(U4::new(16).is_err());
    }

    #[test]
    fn u4_try_from() {
        assert!(U4::try_from(15u8).is_ok());
        assert!(U4::try_from(16u8).is_err());
    }

    #[test]
    fn u4_into_u8() {
        let v = U4::new(10).unwrap();
        let raw: u8 = v.into();
        assert_eq!(raw, 10);
    }

    #[test]
    fn u4_display() {
        let v = U4::new(7).unwrap();
        assert_eq!(format!("{v}"), "7");
    }

    // --- U7 tests ---

    #[test]
    fn u7_new_min() {
        let v = U7::new(0).unwrap();
        assert_eq!(v.value(), 0);
    }

    #[test]
    fn u7_new_max() {
        let v = U7::new(127).unwrap();
        assert_eq!(v.value(), 127);
    }

    #[test]
    fn u7_new_out_of_range() {
        assert!(U7::new(128).is_err());
    }

    #[test]
    fn u7_try_from() {
        assert!(U7::try_from(127u8).is_ok());
        assert!(U7::try_from(128u8).is_err());
    }

    #[test]
    fn u7_into_u8() {
        let v = U7::new(64).unwrap();
        let raw: u8 = v.into();
        assert_eq!(raw, 64);
    }

    #[test]
    fn u7_display() {
        let v = U7::new(42).unwrap();
        assert_eq!(format!("{v}"), "42");
    }

    // --- U14 tests ---

    #[test]
    fn u14_new_min() {
        let v = U14::new(0).unwrap();
        assert_eq!(v.value(), 0);
    }

    #[test]
    fn u14_new_max() {
        let v = U14::new(16383).unwrap();
        assert_eq!(v.value(), 16383);
    }

    #[test]
    fn u14_new_out_of_range() {
        assert!(U14::new(16384).is_err());
    }

    #[test]
    fn u14_try_from() {
        assert!(U14::try_from(16383u16).is_ok());
        assert!(U14::try_from(16384u16).is_err());
    }

    #[test]
    fn u14_into_u16() {
        let v = U14::new(8192).unwrap();
        let raw: u16 = v.into();
        assert_eq!(raw, 8192);
    }

    #[test]
    fn u14_display() {
        let v = U14::new(1000).unwrap();
        assert_eq!(format!("{v}"), "1000");
    }

    // --- I14 tests ---

    #[test]
    fn i14_new_min() {
        let v = I14::new(-8192).unwrap();
        assert_eq!(v.value(), -8192);
    }

    #[test]
    fn i14_new_max() {
        let v = I14::new(8191).unwrap();
        assert_eq!(v.value(), 8191);
    }

    #[test]
    fn i14_new_zero() {
        let v = I14::new(0).unwrap();
        assert_eq!(v.value(), 0);
    }

    #[test]
    fn i14_new_below_min() {
        assert!(I14::new(-8193).is_err());
    }

    #[test]
    fn i14_new_above_max() {
        assert!(I14::new(8192).is_err());
    }

    #[test]
    fn i14_try_from() {
        assert!(I14::try_from(-8192i16).is_ok());
        assert!(I14::try_from(8191i16).is_ok());
        assert!(I14::try_from(-8193i16).is_err());
        assert!(I14::try_from(8192i16).is_err());
    }

    #[test]
    fn i14_into_i16() {
        let v = I14::new(-100).unwrap();
        let raw: i16 = v.into();
        assert_eq!(raw, -100);
    }

    #[test]
    fn i14_display() {
        let v = I14::new(-42).unwrap();
        assert_eq!(format!("{v}"), "-42");
    }
}
