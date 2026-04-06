//! SysEx-only enums for values that appear in global and program data blobs.
//!
//! These enums are not CC/NRPN wire parameters; they exist only inside the
//! binary data blobs transferred via SysEx dumps.

use std::fmt;

use crate::error::{Error, Result};

// ---------------------------------------------------------------------------
// Macro for SysEx-only value enums
// ---------------------------------------------------------------------------

/// Generates a SysEx blob value enum with `from_byte`, `to_byte`, `Display`,
/// and `TryFrom<u8>`.
///
/// Unlike [`stepped_param_enum!`](crate::param::enums), these enums do not
/// have TX/RX bands or program-value mappings — they map 1:1 between a `u8`
/// and a variant.
macro_rules! sysex_value_enum {
    (
        $( #[doc = $enum_doc:literal] )*
        $name:ident {
            $(
                $( #[doc = $var_doc:literal] )*
                $variant:ident => { value: $val:expr $(, display: $disp:literal)? }
            ),+ $(,)?
        }
    ) => {
        $( #[doc = $enum_doc] )*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $(
                $( #[doc = $var_doc] )*
                $variant,
            )+
        }

        impl $name {
            /// Convert a raw byte value to this enum variant.
            ///
            /// # Errors
            ///
            /// Returns [`Error::OutOfRange`] if the value does not match any variant.
            pub fn from_byte(v: u8) -> Result<Self> {
                match v {
                    $( x if x == $val => Ok(Self::$variant), )+
                    _ => Err(Error::OutOfRange {
                        type_name: stringify!($name),
                        value: i64::from(v),
                        min: 0,
                        max: sysex_value_enum!(@max_val $( $val ),+),
                    }),
                }
            }

            /// Convert this variant to its raw byte value.
            pub fn to_byte(self) -> u8 {
                match self {
                    $( Self::$variant => $val, )+
                }
            }
        }

        impl TryFrom<u8> for $name {
            type Error = Error;

            fn try_from(value: u8) -> Result<Self> {
                Self::from_byte(value)
            }
        }

        impl From<$name> for u8 {
            fn from(val: $name) -> Self {
                val.to_byte()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $( Self::$variant => write!(f, "{}", sysex_value_enum!(@display $variant $(, $disp)?)), )+
                }
            }
        }
    };

    // Helper: extract display string or default to variant name.
    (@display $variant:ident, $disp:literal) => { $disp };
    (@display $variant:ident) => { stringify!($variant) };

    // Helper: compute the maximum value for error messages.
    (@max_val $single:expr) => { $single as i64 };
    (@max_val $first:expr, $( $rest:expr ),+) => {
        {
            let rest_max = sysex_value_enum!(@max_val $( $rest ),+);
            if ($first as i64) > rest_max { $first as i64 } else { rest_max }
        }
    };
}

// ---------------------------------------------------------------------------
// Global-blob enums (TABLE 1)
// ---------------------------------------------------------------------------

sysex_value_enum! {
    /// Damper pedal polarity setting.
    DamperPolarity {
        /// Normal polarity (closed = sustain on).
        Normal => { value: 0, display: "-" },
        /// Reversed polarity (open = sustain on).
        Reversed => { value: 1, display: "+" },
    }
}

sysex_value_enum! {
    /// Velocity curve selection.
    VelocityCurve {
        /// Velocity curve type 1.
        Type1 => { value: 0, display: "Type1" },
        /// Velocity curve type 2.
        Type2 => { value: 1, display: "Type2" },
        /// Velocity curve type 3.
        Type3 => { value: 2, display: "Type3" },
        /// Velocity curve type 4.
        Type4 => { value: 3, display: "Type4" },
        /// Velocity curve type 5.
        Type5 => { value: 4, display: "Type5" },
        /// Velocity curve type 6.
        Type6 => { value: 5, display: "Type6" },
        /// Velocity curve type 7.
        Type7 => { value: 6, display: "Type7" },
        /// Velocity curve type 8.
        Type8 => { value: 7, display: "Type8" },
        /// Constant velocity 127.
        Const127 => { value: 8, display: "Const127" },
    }
}

sysex_value_enum! {
    /// Knob behavior mode.
    KnobMode {
        /// Jump: parameter immediately jumps to the knob position.
        Jump => { value: 0 },
        /// Catch: parameter changes only after the knob passes the stored value.
        Catch => { value: 1 },
        /// Scale: parameter scales proportionally from the stored value.
        Scale => { value: 2 },
    }
}

sysex_value_enum! {
    /// Sync clock unit (applies to both sync in and sync out).
    SyncUnit {
        /// 1/16th note (sixteenth).
        Sixteenth => { value: 0, display: "1/16" },
        /// 1/8th note (eighth).
        Eighth => { value: 1, display: "1/8" },
    }
}

sysex_value_enum! {
    /// Sync clock polarity (applies to both sync in and sync out).
    SyncPolarity {
        /// Rising edge triggers the clock.
        Rise => { value: 0, display: "Rise" },
        /// Falling edge triggers the clock.
        Fall => { value: 1, display: "Fall" },
    }
}

sysex_value_enum! {
    /// MIDI routing mode.
    MidiRoute {
        /// Route MIDI to both USB and 5-pin DIN MIDI.
        UsbAndMidi => { value: 0, display: "USB+MIDI" },
        /// Route MIDI to USB only.
        UsbOnly => { value: 1, display: "USB Only" },
    }
}

sysex_value_enum! {
    /// Clock source selection.
    ClockSource {
        /// Auto-detect clock from USB.
        AutoUsb => { value: 0, display: "Auto (USB)" },
        /// Auto-detect clock from MIDI.
        AutoMidi => { value: 1, display: "Auto (MIDI)" },
        /// Use internal clock.
        Internal => { value: 2, display: "Internal" },
    }
}

sysex_value_enum! {
    /// Parameter display mode.
    ParameterDisp {
        /// Show parameter changes only for edited parameters.
        Normal => { value: 0 },
        /// Show all parameter values.
        All => { value: 1 },
    }
}

sysex_value_enum! {
    /// Poly chain mode.
    PolyChainMode {
        /// Poly chain disabled.
        Off => { value: 0 },
        /// This unit is the poly chain master.
        Master => { value: 1 },
        /// This unit is a poly chain slave.
        Slave => { value: 2 },
    }
}

sysex_value_enum! {
    /// Shift button function assignment.
    ShiftFunction {
        /// Shift accesses favorites.
        Favorite => { value: 0 },
        /// Shift accesses active step editing.
        ActiveStep => { value: 1, display: "Active Step" },
    }
}

// ---------------------------------------------------------------------------
// Program-blob enums (TABLE 2 — sequencer section)
// ---------------------------------------------------------------------------

sysex_value_enum! {
    /// Step sequencer resolution.
    StepResolution {
        /// 1/16th note.
        Sixteenth => { value: 0, display: "1/16" },
        /// 1/8th note.
        Eighth => { value: 1, display: "1/8" },
        /// 1/4 note.
        Quarter => { value: 2, display: "1/4" },
        /// 1/2 note.
        Half => { value: 3, display: "1/2" },
        /// Whole note.
        Whole => { value: 4, display: "1/1" },
    }
}

sysex_value_enum! {
    /// Arpeggiator rate division (note S4 of the spec).
    ArpRate {
        /// 1/1 (whole note).
        Full => { value: 0, display: "1/1" },
        /// 3/4 (dotted half).
        ThreeQuarter => { value: 1, display: "3/4" },
        /// 2/3 (triplet half).
        TwoThird => { value: 2, display: "2/3" },
        /// 1/2 (half note).
        Half => { value: 3, display: "1/2" },
        /// 3/8 (dotted quarter).
        ThreeEighth => { value: 4, display: "3/8" },
        /// 1/3 (triplet quarter).
        OneThird => { value: 5, display: "1/3" },
        /// 1/4 (quarter note).
        Quarter => { value: 6, display: "1/4" },
        /// 3/16 (dotted eighth).
        ThreeSixteenth => { value: 7, display: "3/16" },
        /// 1/6 (triplet eighth).
        OneSixth => { value: 8, display: "1/6" },
        /// 1/8 (eighth note).
        Eighth => { value: 9, display: "1/8" },
        /// 1/12 (triplet sixteenth).
        OneTwelfth => { value: 10, display: "1/12" },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Macro: per-enum round-trip and boundary tests
    // ---------------------------------------------------------------

    macro_rules! test_sysex_enum {
        ($name:ident, $enum_ty:ty, [ ($first_variant:expr, $first_val:expr, $first_display:expr) $(, ($variant:expr, $val:expr, $display:expr) )* $(,)? ]) => {
            mod $name {
                use super::*;

                #[test]
                fn round_trip_all_variants() {
                    {
                        let v: $enum_ty = $first_variant;
                        assert_eq!(v.to_byte(), $first_val, "to_byte for {}", stringify!($first_variant));
                        let parsed = <$enum_ty>::from_byte($first_val).unwrap();
                        assert_eq!(parsed, v, "from_byte({}) for {}", $first_val, stringify!($first_variant));
                    }
                    $(
                        {
                            let v: $enum_ty = $variant;
                            assert_eq!(v.to_byte(), $val, "to_byte for {}", stringify!($variant));
                            let parsed = <$enum_ty>::from_byte($val).unwrap();
                            assert_eq!(parsed, v, "from_byte({}) for {}", $val, stringify!($variant));
                        }
                    )*
                }

                #[test]
                fn display_all_variants() {
                    {
                        let v: $enum_ty = $first_variant;
                        assert_eq!(v.to_string(), $first_display, "display for {}", stringify!($first_variant));
                    }
                    $(
                        {
                            let v: $enum_ty = $variant;
                            assert_eq!(v.to_string(), $display, "display for {}", stringify!($variant));
                        }
                    )*
                }

                #[test]
                fn try_from_round_trip() {
                    {
                        let v = <$enum_ty>::try_from($first_val as u8).unwrap();
                        let byte: u8 = v.into();
                        assert_eq!(byte, $first_val);
                    }
                    $(
                        {
                            let v = <$enum_ty>::try_from($val as u8).unwrap();
                            let byte: u8 = v.into();
                            assert_eq!(byte, $val);
                        }
                    )*
                }

                #[test]
                fn from_byte_out_of_range() {
                    let max_val = $first_val $( .max($val) )*;
                    if max_val < 255 {
                        assert!(<$enum_ty>::from_byte(max_val + 1).is_err());
                    }
                    assert!(<$enum_ty>::from_byte(255).is_err());
                }

                #[test]
                fn clone_copy_eq_hash() {
                    use std::collections::HashSet;
                    let v: $enum_ty = $first_variant;
                    let v2 = v; // Copy
                    assert_eq!(v, v2);
                    let mut set = HashSet::new();
                    set.insert(v);
                    assert!(set.contains(&v2));
                    let _dbg = format!("{v:?}");
                }
            }
        };
    }

    test_sysex_enum!(
        damper_polarity,
        DamperPolarity,
        [
            (DamperPolarity::Normal, 0, "-"),
            (DamperPolarity::Reversed, 1, "+"),
        ]
    );

    test_sysex_enum!(
        velocity_curve,
        VelocityCurve,
        [
            (VelocityCurve::Type1, 0, "Type1"),
            (VelocityCurve::Type2, 1, "Type2"),
            (VelocityCurve::Type3, 2, "Type3"),
            (VelocityCurve::Type4, 3, "Type4"),
            (VelocityCurve::Type5, 4, "Type5"),
            (VelocityCurve::Type6, 5, "Type6"),
            (VelocityCurve::Type7, 6, "Type7"),
            (VelocityCurve::Type8, 7, "Type8"),
            (VelocityCurve::Const127, 8, "Const127"),
        ]
    );

    test_sysex_enum!(
        knob_mode,
        KnobMode,
        [
            (KnobMode::Jump, 0, "Jump"),
            (KnobMode::Catch, 1, "Catch"),
            (KnobMode::Scale, 2, "Scale"),
        ]
    );

    test_sysex_enum!(
        sync_unit,
        SyncUnit,
        [
            (SyncUnit::Sixteenth, 0, "1/16"),
            (SyncUnit::Eighth, 1, "1/8"),
        ]
    );

    test_sysex_enum!(
        sync_polarity,
        SyncPolarity,
        [
            (SyncPolarity::Rise, 0, "Rise"),
            (SyncPolarity::Fall, 1, "Fall"),
        ]
    );

    test_sysex_enum!(
        midi_route,
        MidiRoute,
        [
            (MidiRoute::UsbAndMidi, 0, "USB+MIDI"),
            (MidiRoute::UsbOnly, 1, "USB Only"),
        ]
    );

    test_sysex_enum!(
        clock_source,
        ClockSource,
        [
            (ClockSource::AutoUsb, 0, "Auto (USB)"),
            (ClockSource::AutoMidi, 1, "Auto (MIDI)"),
            (ClockSource::Internal, 2, "Internal"),
        ]
    );

    test_sysex_enum!(
        parameter_disp,
        ParameterDisp,
        [
            (ParameterDisp::Normal, 0, "Normal"),
            (ParameterDisp::All, 1, "All"),
        ]
    );

    test_sysex_enum!(
        poly_chain_mode,
        PolyChainMode,
        [
            (PolyChainMode::Off, 0, "Off"),
            (PolyChainMode::Master, 1, "Master"),
            (PolyChainMode::Slave, 2, "Slave"),
        ]
    );

    test_sysex_enum!(
        shift_function,
        ShiftFunction,
        [
            (ShiftFunction::Favorite, 0, "Favorite"),
            (ShiftFunction::ActiveStep, 1, "Active Step"),
        ]
    );

    // ---------------------------------------------------------------
    // Gap-in-values test (DamperPolarity has no value 2)
    // ---------------------------------------------------------------

    #[test]
    fn damper_polarity_gap_value_2() {
        assert!(DamperPolarity::from_byte(2).is_err());
    }

    #[test]
    fn velocity_curve_gap_value_9() {
        assert!(VelocityCurve::from_byte(9).is_err());
    }

    // ---------------------------------------------------------------
    // Sequencer enums
    // ---------------------------------------------------------------

    test_sysex_enum!(
        step_resolution,
        StepResolution,
        [
            (StepResolution::Sixteenth, 0, "1/16"),
            (StepResolution::Eighth, 1, "1/8"),
            (StepResolution::Quarter, 2, "1/4"),
            (StepResolution::Half, 3, "1/2"),
            (StepResolution::Whole, 4, "1/1"),
        ]
    );

    test_sysex_enum!(
        arp_rate,
        ArpRate,
        [
            (ArpRate::Full, 0, "1/1"),
            (ArpRate::ThreeQuarter, 1, "3/4"),
            (ArpRate::TwoThird, 2, "2/3"),
            (ArpRate::Half, 3, "1/2"),
            (ArpRate::ThreeEighth, 4, "3/8"),
            (ArpRate::OneThird, 5, "1/3"),
            (ArpRate::Quarter, 6, "1/4"),
            (ArpRate::ThreeSixteenth, 7, "3/16"),
            (ArpRate::OneSixth, 8, "1/6"),
            (ArpRate::Eighth, 9, "1/8"),
            (ArpRate::OneTwelfth, 10, "1/12"),
        ]
    );
}
