//! Global parameter blob (TABLE 1) for the Korg Minilogue XD.
//!
//! The global data dump is a 63-byte blob with the magic header `GLOB`.
//! It contains system-wide settings such as master tune, transpose, MIDI
//! configuration, favorites, and display options.
//!
//! ## Byte layout (offsets 0--62)
//!
//! | Offset | Field                  | Range           |
//! |--------|------------------------|-----------------|
//! | 0--3   | Magic `"GLOB"`         | ASCII           |
//! | 4      | Master Tune            | 0--100          |
//! | 5      | Transpose              | 0--24           |
//! | 6      | Metronome              | 0=Off, 1=On     |
//! | 7      | Damper Polarity        | 0--1            |
//! | 8      | Local SW               | 0=Off, 1=On     |
//! | 9      | Velocity Curve         | 0--8            |
//! | 10     | Knob Mode              | 0--2            |
//! | 11     | Sync In Unit           | 0--1            |
//! | 12     | Sync Out Unit          | 0--1            |
//! | 13     | Sync In Polarity       | 0--1            |
//! | 14     | Sync Out Polarity      | 0--1            |
//! | 15     | MIDI Route             | 0--1            |
//! | 16     | MIDI Channel           | 0--15           |
//! | 17     | Clock Source            | 0--2            |
//! | 18     | En Rx Transport        | 0=Off, 1=On     |
//! | 19     | MIDI Rx Prog Chg       | 0=Off, 1=On     |
//! | 20     | MIDI Rx CC             | 0=Off, 1=On     |
//! | 21     | MIDI Rx Pitch Bend     | 0=Off, 1=On     |
//! | 22     | MIDI Tx Prog Chg       | 0=Off, 1=On     |
//! | 23     | MIDI Tx CC             | 0=Off, 1=On     |
//! | 24     | MIDI Tx Pitch Bend     | 0=Off, 1=On     |
//! | 25     | Parameter Display      | 0--1            |
//! | 26     | Brightness             | 0--9            |
//! | 27     | Auto Power Off         | 0=Off, 1=On     |
//! | 28--59 | Favorites 1--16        | 2 bytes each    |
//! | 60     | Poly Chain Mode        | 0--2            |
//! | 61     | Oscilloscope           | 0=Off, 1=On     |
//! | 62     | Shift Function         | 0--1            |
//!
//! **Note on favorites:** The Korg spec lists range 0--499 for favorite
//! lower/upper values, but each field occupies a single byte (max 255).
//! This implementation stores raw `u8` values and documents the discrepancy.

use crate::error::{Result, SysexError};
use crate::message::types::U4;
use crate::sysex::enums::{
    ClockSource, DamperPolarity, KnobMode, MidiRoute, ParameterDisp, PolyChainMode, ShiftFunction,
    SyncPolarity, SyncUnit, VelocityCurve,
};
use crate::sysex::frame;

/// A single favorite slot entry (lower and upper program references).
///
/// Each value is stored as a raw `u8`. The Korg spec states the range is
/// 0--499, but each field is a single byte in the blob, so the actual
/// on-wire range is 0--255.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FavoriteEntry {
    /// Lower program reference.
    pub lower: u8,
    /// Upper program reference.
    pub upper: u8,
}

/// Global parameters (TABLE 1) — 63-byte data blob.
///
/// Use [`GlobalParams::from_bytes`] to parse a raw blob and
/// [`GlobalParams::to_bytes`] to serialize back. For SysEx-level operations,
/// use [`build_global_request`], [`build_global_dump`], and
/// [`parse_global_dump`].
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalParams {
    /// Master tune offset. Range 0--100; center (50) = 0 cents.
    /// Each step is approximately 1 cent, so 0 = -50 cents, 100 = +50 cents.
    pub master_tune: u8,
    /// Transpose. Range 0--24; center (12) = 0 semitones.
    pub transpose: u8,
    /// Metronome on/off.
    pub metronome: bool,
    /// Damper pedal polarity.
    pub damper_polarity: DamperPolarity,
    /// Local sound on/off. When off, the keyboard does not trigger local sound.
    pub local_sw: bool,
    /// Velocity curve selection.
    pub velocity_curve: VelocityCurve,
    /// Knob behavior mode.
    pub knob_mode: KnobMode,
    /// Sync input clock unit.
    pub sync_in_unit: SyncUnit,
    /// Sync output clock unit.
    pub sync_out_unit: SyncUnit,
    /// Sync input polarity.
    pub sync_in_polarity: SyncPolarity,
    /// Sync output polarity.
    pub sync_out_polarity: SyncPolarity,
    /// MIDI routing mode.
    pub midi_route: MidiRoute,
    /// MIDI channel (0--15).
    pub midi_channel: U4,
    /// Clock source selection.
    pub clock_source: ClockSource,
    /// Enable receive transport (start/stop/continue).
    pub en_rx_transport: bool,
    /// Enable receive program change.
    pub midi_rx_prog_chg: bool,
    /// Enable receive control change.
    pub midi_rx_cc: bool,
    /// Enable receive pitch bend.
    pub midi_rx_pitch_bend: bool,
    /// Enable transmit program change.
    pub midi_tx_prog_chg: bool,
    /// Enable transmit control change.
    pub midi_tx_cc: bool,
    /// Enable transmit pitch bend.
    pub midi_tx_pitch_bend: bool,
    /// Parameter display mode.
    pub parameter_disp: ParameterDisp,
    /// Display brightness (0--9).
    pub brightness: u8,
    /// Auto power off on/off.
    pub auto_power_off: bool,
    /// 16 favorite slots.
    pub favorites: [FavoriteEntry; 16],
    /// Poly chain mode.
    pub poly_chain: PolyChainMode,
    /// Oscilloscope display on/off.
    pub oscilloscope: bool,
    /// Shift button function.
    pub shift_function: ShiftFunction,
}

impl GlobalParams {
    /// The magic header bytes at the start of the blob.
    pub const MAGIC: &'static [u8; 4] = b"GLOB";

    /// Total size of the raw global data blob in bytes.
    pub const BLOB_SIZE: usize = 63;

    /// Parse a global parameter blob from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is too short, the magic header is wrong,
    /// or any field value is out of range.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::BLOB_SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::BLOB_SIZE,
                actual: data.len(),
            }
            .into());
        }

        // Validate magic.
        if &data[0..4] != Self::MAGIC.as_slice() {
            return Err(SysexError::InvalidMagic {
                expected: "GLOB".to_string(),
                actual: format!("{:02X?}", &data[0..4]),
            }
            .into());
        }

        let master_tune = data[4];
        let transpose = data[5];
        let metronome = data[6] != 0;
        let damper_polarity = DamperPolarity::from_byte(data[7])?;
        let local_sw = data[8] != 0;
        let velocity_curve = VelocityCurve::from_byte(data[9])?;
        let knob_mode = KnobMode::from_byte(data[10])?;
        let sync_in_unit = SyncUnit::from_byte(data[11])?;
        let sync_out_unit = SyncUnit::from_byte(data[12])?;
        let sync_in_polarity = SyncPolarity::from_byte(data[13])?;
        let sync_out_polarity = SyncPolarity::from_byte(data[14])?;
        let midi_route = MidiRoute::from_byte(data[15])?;
        let midi_channel = U4::new(data[16])?;
        let clock_source = ClockSource::from_byte(data[17])?;
        let en_rx_transport = data[18] != 0;
        let midi_rx_prog_chg = data[19] != 0;
        let midi_rx_cc = data[20] != 0;
        let midi_rx_pitch_bend = data[21] != 0;
        let midi_tx_prog_chg = data[22] != 0;
        let midi_tx_cc = data[23] != 0;
        let midi_tx_pitch_bend = data[24] != 0;
        let parameter_disp = ParameterDisp::from_byte(data[25])?;
        let brightness = data[26];
        let auto_power_off = data[27] != 0;

        let mut favorites = [FavoriteEntry::default(); 16];
        for (i, fav) in favorites.iter_mut().enumerate() {
            let base = 28 + i * 2;
            *fav = FavoriteEntry {
                lower: data[base],
                upper: data[base + 1],
            };
        }

        let poly_chain = PolyChainMode::from_byte(data[60])?;
        let oscilloscope = data[61] != 0;
        let shift_function = ShiftFunction::from_byte(data[62])?;

        Ok(Self {
            master_tune,
            transpose,
            metronome,
            damper_polarity,
            local_sw,
            velocity_curve,
            knob_mode,
            sync_in_unit,
            sync_out_unit,
            sync_in_polarity,
            sync_out_polarity,
            midi_route,
            midi_channel,
            clock_source,
            en_rx_transport,
            midi_rx_prog_chg,
            midi_rx_cc,
            midi_rx_pitch_bend,
            midi_tx_prog_chg,
            midi_tx_cc,
            midi_tx_pitch_bend,
            parameter_disp,
            brightness,
            auto_power_off,
            favorites,
            poly_chain,
            oscilloscope,
            shift_function,
        })
    }

    /// Serialize the global parameters to a 63-byte blob.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; Self::BLOB_SIZE];

        out[0..4].copy_from_slice(Self::MAGIC);
        out[4] = self.master_tune;
        out[5] = self.transpose;
        out[6] = u8::from(self.metronome);
        out[7] = self.damper_polarity.to_byte();
        out[8] = u8::from(self.local_sw);
        out[9] = self.velocity_curve.to_byte();
        out[10] = self.knob_mode.to_byte();
        out[11] = self.sync_in_unit.to_byte();
        out[12] = self.sync_out_unit.to_byte();
        out[13] = self.sync_in_polarity.to_byte();
        out[14] = self.sync_out_polarity.to_byte();
        out[15] = self.midi_route.to_byte();
        out[16] = self.midi_channel.value();
        out[17] = self.clock_source.to_byte();
        out[18] = u8::from(self.en_rx_transport);
        out[19] = u8::from(self.midi_rx_prog_chg);
        out[20] = u8::from(self.midi_rx_cc);
        out[21] = u8::from(self.midi_rx_pitch_bend);
        out[22] = u8::from(self.midi_tx_prog_chg);
        out[23] = u8::from(self.midi_tx_cc);
        out[24] = u8::from(self.midi_tx_pitch_bend);
        out[25] = self.parameter_disp.to_byte();
        out[26] = self.brightness;
        out[27] = u8::from(self.auto_power_off);

        for i in 0..16 {
            let base = 28 + i * 2;
            out[base] = self.favorites[i].lower;
            out[base + 1] = self.favorites[i].upper;
        }

        out[60] = self.poly_chain.to_byte();
        out[61] = u8::from(self.oscilloscope);
        out[62] = self.shift_function.to_byte();

        out
    }
}

impl Default for GlobalParams {
    /// Returns factory-default global parameters.
    fn default() -> Self {
        Self {
            master_tune: 50, // center = 0 cents
            transpose: 12,   // center = 0 semitones
            metronome: false,
            damper_polarity: DamperPolarity::Normal,
            local_sw: true,
            velocity_curve: VelocityCurve::Type4,
            knob_mode: KnobMode::Jump,
            sync_in_unit: SyncUnit::Sixteenth,
            sync_out_unit: SyncUnit::Sixteenth,
            sync_in_polarity: SyncPolarity::Rise,
            sync_out_polarity: SyncPolarity::Rise,
            midi_route: MidiRoute::UsbAndMidi,
            midi_channel: U4::new(0).expect("0 is valid for U4"),
            clock_source: ClockSource::AutoUsb,
            en_rx_transport: true,
            midi_rx_prog_chg: true,
            midi_rx_cc: true,
            midi_rx_pitch_bend: true,
            midi_tx_prog_chg: true,
            midi_tx_cc: true,
            midi_tx_pitch_bend: true,
            parameter_disp: ParameterDisp::Normal,
            brightness: 5,
            auto_power_off: false,
            favorites: [FavoriteEntry::default(); 16],
            poly_chain: PolyChainMode::Off,
            oscilloscope: true,
            shift_function: ShiftFunction::Favorite,
        }
    }
}

// ---------------------------------------------------------------------------
// SysEx convenience functions
// ---------------------------------------------------------------------------

/// Build a SysEx global data request message.
///
/// Sends function ID [`GLOBAL_DATA_REQUEST`](frame::GLOBAL_DATA_REQUEST)
/// with no payload.
pub fn build_global_request(channel: U4) -> Vec<u8> {
    frame::build_sysex_request(channel, frame::GLOBAL_DATA_REQUEST)
}

/// Build a SysEx global data dump message.
///
/// Wraps the 63-byte global blob in a Korg SysEx frame with function ID
/// [`GLOBAL_DATA_DUMP`](frame::GLOBAL_DATA_DUMP).
pub fn build_global_dump(channel: U4, params: &GlobalParams) -> Vec<u8> {
    let blob = params.to_bytes();
    frame::build_sysex(channel, frame::GLOBAL_DATA_DUMP, &blob)
}

/// Parse a SysEx global data dump message into [`GlobalParams`].
///
/// Expects a complete SysEx message (F0...F7). Validates the frame header,
/// checks the function ID is [`GLOBAL_DATA_DUMP`](frame::GLOBAL_DATA_DUMP),
/// and decodes the blob.
///
/// # Errors
///
/// Returns an error if the frame is malformed, the function ID is wrong,
/// or the blob data is invalid.
pub fn parse_global_dump(bytes: &[u8]) -> Result<GlobalParams> {
    let parsed = frame::parse_sysex(bytes)?;
    if parsed.function_id != frame::GLOBAL_DATA_DUMP {
        return Err(SysexError::WrongFunctionId {
            expected: frame::GLOBAL_DATA_DUMP,
            found: parsed.function_id,
        }
        .into());
    }
    GlobalParams::from_bytes(&parsed.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    /// Build a minimal valid GLOB blob with all zeros except the magic.
    fn minimal_glob_blob() -> Vec<u8> {
        let mut blob = vec![0u8; GlobalParams::BLOB_SIZE];
        blob[0..4].copy_from_slice(b"GLOB");
        blob
    }

    // ---------------------------------------------------------------
    // FavoriteEntry
    // ---------------------------------------------------------------

    #[test]
    fn favorite_entry_default() {
        let f = FavoriteEntry::default();
        assert_eq!(f.lower, 0);
        assert_eq!(f.upper, 0);
    }

    #[test]
    fn favorite_entry_clone_copy_eq() {
        let f = FavoriteEntry {
            lower: 10,
            upper: 20,
        };
        let f2 = f; // Copy
        assert_eq!(f, f2);
        let _dbg = format!("{f:?}");
    }

    // ---------------------------------------------------------------
    // GlobalParams::from_bytes — valid
    // ---------------------------------------------------------------

    #[test]
    fn from_bytes_minimal() {
        let blob = minimal_glob_blob();
        let params = GlobalParams::from_bytes(&blob).unwrap();
        assert_eq!(params.master_tune, 0);
        assert_eq!(params.transpose, 0);
        assert!(!params.metronome);
        assert_eq!(params.damper_polarity, DamperPolarity::Normal);
        assert!(!params.local_sw);
        assert_eq!(params.velocity_curve, VelocityCurve::Type1);
        assert_eq!(params.knob_mode, KnobMode::Jump);
        assert_eq!(params.midi_channel, ch(0));
        assert_eq!(params.brightness, 0);
        assert_eq!(params.poly_chain, PolyChainMode::Off);
        assert!(!params.oscilloscope);
        assert_eq!(params.shift_function, ShiftFunction::Favorite);
    }

    #[test]
    fn from_bytes_all_fields() {
        let mut blob = minimal_glob_blob();
        blob[4] = 100; // master_tune max
        blob[5] = 24; // transpose max
        blob[6] = 1; // metronome on
        blob[7] = 1; // damper reversed
        blob[8] = 1; // local on
        blob[9] = 8; // Const127
        blob[10] = 2; // Scale
        blob[11] = 1; // Eighth
        blob[12] = 1; // Eighth
        blob[13] = 1; // Fall
        blob[14] = 1; // Fall
        blob[15] = 1; // USB Only
        blob[16] = 15; // channel 15
        blob[17] = 2; // Internal
        blob[18] = 1;
        blob[19] = 1;
        blob[20] = 1;
        blob[21] = 1;
        blob[22] = 1;
        blob[23] = 1;
        blob[24] = 1;
        blob[25] = 1; // All
        blob[26] = 9; // brightness max
        blob[27] = 1; // auto power off
                      // favorites
        for i in 0..16 {
            blob[28 + i * 2] = (i + 1) as u8;
            blob[28 + i * 2 + 1] = (i + 100) as u8;
        }
        blob[60] = 2; // Slave
        blob[61] = 1; // oscilloscope on
        blob[62] = 1; // ActiveStep

        let params = GlobalParams::from_bytes(&blob).unwrap();
        assert_eq!(params.master_tune, 100);
        assert_eq!(params.transpose, 24);
        assert!(params.metronome);
        assert_eq!(params.damper_polarity, DamperPolarity::Reversed);
        assert!(params.local_sw);
        assert_eq!(params.velocity_curve, VelocityCurve::Const127);
        assert_eq!(params.knob_mode, KnobMode::Scale);
        assert_eq!(params.sync_in_unit, SyncUnit::Eighth);
        assert_eq!(params.sync_out_unit, SyncUnit::Eighth);
        assert_eq!(params.sync_in_polarity, SyncPolarity::Fall);
        assert_eq!(params.sync_out_polarity, SyncPolarity::Fall);
        assert_eq!(params.midi_route, MidiRoute::UsbOnly);
        assert_eq!(params.midi_channel, ch(15));
        assert_eq!(params.clock_source, ClockSource::Internal);
        assert!(params.en_rx_transport);
        assert!(params.midi_rx_prog_chg);
        assert!(params.midi_rx_cc);
        assert!(params.midi_rx_pitch_bend);
        assert!(params.midi_tx_prog_chg);
        assert!(params.midi_tx_cc);
        assert!(params.midi_tx_pitch_bend);
        assert_eq!(params.parameter_disp, ParameterDisp::All);
        assert_eq!(params.brightness, 9);
        assert!(params.auto_power_off);
        for i in 0..16 {
            assert_eq!(params.favorites[i].lower, (i + 1) as u8);
            assert_eq!(params.favorites[i].upper, (i + 100) as u8);
        }
        assert_eq!(params.poly_chain, PolyChainMode::Slave);
        assert!(params.oscilloscope);
        assert_eq!(params.shift_function, ShiftFunction::ActiveStep);
    }

    // ---------------------------------------------------------------
    // GlobalParams::from_bytes — errors
    // ---------------------------------------------------------------

    #[test]
    fn from_bytes_too_short() {
        let blob = vec![0u8; 10];
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_wrong_magic() {
        let mut blob = vec![0u8; GlobalParams::BLOB_SIZE];
        blob[0..4].copy_from_slice(b"PROG");
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_enum_velocity() {
        let mut blob = minimal_glob_blob();
        blob[9] = 99; // invalid VelocityCurve
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_enum_knob_mode() {
        let mut blob = minimal_glob_blob();
        blob[10] = 5; // invalid KnobMode
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_channel() {
        let mut blob = minimal_glob_blob();
        blob[16] = 16; // invalid U4
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_clock_source() {
        let mut blob = minimal_glob_blob();
        blob[17] = 5; // invalid ClockSource
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_poly_chain() {
        let mut blob = minimal_glob_blob();
        blob[60] = 10; // invalid PolyChainMode
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_shift_function() {
        let mut blob = minimal_glob_blob();
        blob[62] = 5; // invalid ShiftFunction
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_damper_polarity() {
        let mut blob = minimal_glob_blob();
        blob[7] = 3;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_sync_in_unit() {
        let mut blob = minimal_glob_blob();
        blob[11] = 5;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_sync_out_unit() {
        let mut blob = minimal_glob_blob();
        blob[12] = 5;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_sync_in_polarity() {
        let mut blob = minimal_glob_blob();
        blob[13] = 5;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_sync_out_polarity() {
        let mut blob = minimal_glob_blob();
        blob[14] = 5;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_midi_route() {
        let mut blob = minimal_glob_blob();
        blob[15] = 5;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    #[test]
    fn from_bytes_invalid_parameter_disp() {
        let mut blob = minimal_glob_blob();
        blob[25] = 5;
        assert!(GlobalParams::from_bytes(&blob).is_err());
    }

    // ---------------------------------------------------------------
    // GlobalParams::to_bytes / round-trip
    // ---------------------------------------------------------------

    #[test]
    fn to_bytes_size() {
        let params = GlobalParams::default();
        let blob = params.to_bytes();
        assert_eq!(blob.len(), GlobalParams::BLOB_SIZE);
    }

    #[test]
    fn to_bytes_magic() {
        let params = GlobalParams::default();
        let blob = params.to_bytes();
        assert_eq!(&blob[0..4], b"GLOB");
    }

    #[test]
    fn round_trip_default() {
        let params = GlobalParams::default();
        let blob = params.to_bytes();
        let parsed = GlobalParams::from_bytes(&blob).unwrap();
        assert_eq!(parsed, params);
    }

    #[test]
    fn round_trip_all_max_values() {
        let params = GlobalParams {
            master_tune: 100,
            transpose: 24,
            metronome: true,
            damper_polarity: DamperPolarity::Reversed,
            local_sw: true,
            velocity_curve: VelocityCurve::Const127,
            knob_mode: KnobMode::Scale,
            sync_in_unit: SyncUnit::Eighth,
            sync_out_unit: SyncUnit::Eighth,
            sync_in_polarity: SyncPolarity::Fall,
            sync_out_polarity: SyncPolarity::Fall,
            midi_route: MidiRoute::UsbOnly,
            midi_channel: ch(15),
            clock_source: ClockSource::Internal,
            en_rx_transport: true,
            midi_rx_prog_chg: true,
            midi_rx_cc: true,
            midi_rx_pitch_bend: true,
            midi_tx_prog_chg: true,
            midi_tx_cc: true,
            midi_tx_pitch_bend: true,
            parameter_disp: ParameterDisp::All,
            brightness: 9,
            auto_power_off: true,
            favorites: [FavoriteEntry {
                lower: 255,
                upper: 255,
            }; 16],
            poly_chain: PolyChainMode::Slave,
            oscilloscope: true,
            shift_function: ShiftFunction::ActiveStep,
        };
        let blob = params.to_bytes();
        let parsed = GlobalParams::from_bytes(&blob).unwrap();
        assert_eq!(parsed, params);
    }

    #[test]
    fn round_trip_with_favorites() {
        let mut params = GlobalParams::default();
        for i in 0..16 {
            params.favorites[i] = FavoriteEntry {
                lower: (i * 10) as u8,
                upper: (i * 10 + 5) as u8,
            };
        }
        let blob = params.to_bytes();
        let parsed = GlobalParams::from_bytes(&blob).unwrap();
        assert_eq!(parsed.favorites, params.favorites);
    }

    // ---------------------------------------------------------------
    // Default
    // ---------------------------------------------------------------

    #[test]
    fn default_values() {
        let d = GlobalParams::default();
        assert_eq!(d.master_tune, 50);
        assert_eq!(d.transpose, 12);
        assert!(!d.metronome);
        assert_eq!(d.damper_polarity, DamperPolarity::Normal);
        assert!(d.local_sw);
        assert_eq!(d.velocity_curve, VelocityCurve::Type4);
        assert_eq!(d.knob_mode, KnobMode::Jump);
        assert_eq!(d.sync_in_unit, SyncUnit::Sixteenth);
        assert_eq!(d.sync_out_unit, SyncUnit::Sixteenth);
        assert_eq!(d.sync_in_polarity, SyncPolarity::Rise);
        assert_eq!(d.sync_out_polarity, SyncPolarity::Rise);
        assert_eq!(d.midi_route, MidiRoute::UsbAndMidi);
        assert_eq!(d.midi_channel, ch(0));
        assert_eq!(d.clock_source, ClockSource::AutoUsb);
        assert!(d.en_rx_transport);
        assert!(d.midi_rx_prog_chg);
        assert!(d.midi_rx_cc);
        assert!(d.midi_rx_pitch_bend);
        assert!(d.midi_tx_prog_chg);
        assert!(d.midi_tx_cc);
        assert!(d.midi_tx_pitch_bend);
        assert_eq!(d.parameter_disp, ParameterDisp::Normal);
        assert_eq!(d.brightness, 5);
        assert!(!d.auto_power_off);
        assert_eq!(d.poly_chain, PolyChainMode::Off);
        assert!(d.oscilloscope);
        assert_eq!(d.shift_function, ShiftFunction::Favorite);
    }

    // ---------------------------------------------------------------
    // SysEx convenience functions
    // ---------------------------------------------------------------

    #[test]
    fn build_global_request_ch0() {
        let msg = build_global_request(ch(0));
        assert_eq!(msg, vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x0E, 0xF7]);
    }

    #[test]
    fn build_global_request_ch15() {
        let msg = build_global_request(ch(15));
        assert_eq!(msg[2], 0x3F);
        assert_eq!(msg[6], frame::GLOBAL_DATA_REQUEST);
    }

    #[test]
    fn build_and_parse_global_dump_round_trip() {
        let params = GlobalParams::default();
        let msg = build_global_dump(ch(5), &params);
        let parsed = parse_global_dump(&msg).unwrap();
        assert_eq!(parsed, params);
    }

    #[test]
    fn build_and_parse_global_dump_custom() {
        let mut favorites = [FavoriteEntry::default(); 16];
        favorites[0] = FavoriteEntry {
            lower: 42,
            upper: 99,
        };
        let params = GlobalParams {
            master_tune: 75,
            brightness: 9,
            metronome: true,
            favorites,
            ..GlobalParams::default()
        };

        let msg = build_global_dump(ch(0), &params);
        let parsed = parse_global_dump(&msg).unwrap();
        assert_eq!(parsed.master_tune, 75);
        assert_eq!(parsed.brightness, 9);
        assert!(parsed.metronome);
        assert_eq!(parsed.favorites[0].lower, 42);
        assert_eq!(parsed.favorites[0].upper, 99);
    }

    #[test]
    fn parse_global_dump_wrong_function_id() {
        // Build a message with the wrong function ID.
        let params = GlobalParams::default();
        let blob = params.to_bytes();
        let msg = frame::build_sysex(ch(0), frame::CURRENT_PROGRAM_DUMP, &blob);
        let err = parse_global_dump(&msg);
        assert!(err.is_err());
    }

    #[test]
    fn parse_global_dump_malformed_frame() {
        // Not even valid SysEx.
        assert!(parse_global_dump(&[0x00, 0x01, 0x02]).is_err());
    }

    // ---------------------------------------------------------------
    // GlobalParams Debug, Clone
    // ---------------------------------------------------------------

    #[test]
    fn global_params_debug_clone() {
        let params = GlobalParams::default();
        let cloned = params.clone();
        assert_eq!(params, cloned);
        let _dbg = format!("{params:?}");
    }

    // ---------------------------------------------------------------
    // Extra bytes beyond BLOB_SIZE are ignored
    // ---------------------------------------------------------------

    #[test]
    fn from_bytes_extra_bytes_ignored() {
        let mut blob = minimal_glob_blob();
        blob.extend_from_slice(&[0xFF; 10]);
        let params = GlobalParams::from_bytes(&blob).unwrap();
        assert_eq!(params.master_tune, 0);
    }

    // ---------------------------------------------------------------
    // Bool fields nonzero treated as true
    // ---------------------------------------------------------------

    #[test]
    fn bool_fields_nonzero_is_true() {
        let mut blob = minimal_glob_blob();
        blob[6] = 2; // metronome: any nonzero = true
        blob[8] = 255; // local_sw
        blob[18] = 42; // en_rx_transport
        blob[27] = 100; // auto_power_off
        blob[61] = 5; // oscilloscope
        let params = GlobalParams::from_bytes(&blob).unwrap();
        assert!(params.metronome);
        assert!(params.local_sw);
        assert!(params.en_rx_transport);
        assert!(params.auto_power_off);
        assert!(params.oscilloscope);
    }

    // ---------------------------------------------------------------
    // Blob constants
    // ---------------------------------------------------------------

    #[test]
    fn blob_size_constant() {
        assert_eq!(GlobalParams::BLOB_SIZE, 63);
    }

    #[test]
    fn magic_constant() {
        assert_eq!(GlobalParams::MAGIC, b"GLOB");
    }
}
