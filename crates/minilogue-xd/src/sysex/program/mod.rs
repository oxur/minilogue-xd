//! Program data blob (TABLE 2) and SysEx message builders.
//!
//! This module assembles the complete 1024-byte program blob from its two
//! halves ([`SynthParams`] and [`SequencerParams`]), provides
//! [`ProgramNumber`] for addressing stored programs, and offers builders
//! for the SysEx current-program and program-data messages.

#[cfg(feature = "file-formats")]
pub mod file;
pub mod sequencer;
pub mod synth;

pub use sequencer::{MotionSlotConfig, SequencerParams, StepEvent};
pub use synth::{ProgramName, SynthParams};

use crate::error::{Result, SysexError};
use crate::message::types::U4;
use crate::sysex::frame::{
    build_sysex, build_sysex_request, parse_sysex, CURRENT_PROGRAM_DUMP, CURRENT_PROGRAM_REQUEST,
    PROGRAM_DATA_DUMP, PROGRAM_DATA_REQUEST,
};

// ---------------------------------------------------------------------------
// ProgramNumber
// ---------------------------------------------------------------------------

/// A valid program number (0--499) addressing a stored program slot.
///
/// The Minilogue XD has 500 program slots organized in 5 banks of 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramNumber(u16);

impl ProgramNumber {
    /// Maximum valid program number.
    pub const MAX: u16 = 499;

    /// Create a new `ProgramNumber` if `n` is in range 0--499.
    ///
    /// # Errors
    ///
    /// Returns [`SysexError::InvalidProgramNumber`] if `n` exceeds 499.
    pub fn new(n: u16) -> Result<Self> {
        if n > Self::MAX {
            return Err(SysexError::InvalidProgramNumber(n).into());
        }
        Ok(Self(n))
    }

    /// Returns the raw program number (0--499).
    pub fn value(self) -> u16 {
        self.0
    }

    /// Returns the bank number (0--4), where each bank has 100 programs.
    pub fn bank(self) -> u8 {
        (self.0 / 100) as u8
    }

    /// Returns the slot index within the bank (0--99).
    pub fn slot_in_bank(self) -> u8 {
        (self.0 % 100) as u8
    }
}

impl std::fmt::Display for ProgramNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// ProgramData
// ---------------------------------------------------------------------------

/// A complete 1024-byte program data blob (TABLE 2).
///
/// Composed of [`SynthParams`] (offsets 0--155) and [`SequencerParams`]
/// (offsets 156--1023).
///
/// When parsed from hardware bytes via [`from_bytes`](Self::from_bytes), the
/// original raw blob is preserved internally. Subsequent calls to
/// [`to_bytes`](Self::to_bytes) use the raw blob as a base buffer, ensuring
/// that hardware-specific flag bits in unused bit positions (e.g. the upper
/// 6 bits of 10-bit parameter high bytes) survive a round-trip.
#[derive(Debug, Clone, Default)]
pub struct ProgramData {
    /// Synth parameters (offsets 0--155).
    pub synth: SynthParams,
    /// Sequencer parameters (offsets 156--1023).
    pub sequencer: SequencerParams,
    /// Raw blob bytes preserved for round-trip fidelity.
    ///
    /// When present, [`to_bytes`](Self::to_bytes) starts from these bytes
    /// instead of zeros, ensuring hardware-specific flags in unused bit
    /// positions are preserved.
    raw_blob: Option<Box<[u8; Self::SIZE]>>,
}

impl PartialEq for ProgramData {
    fn eq(&self, other: &Self) -> bool {
        self.synth == other.synth && self.sequencer == other.sequencer
    }
}

impl ProgramData {
    /// Total size of a program blob in bytes.
    pub const SIZE: usize = 1024;

    /// Create a new `ProgramData` from synth and sequencer parameters.
    ///
    /// The resulting blob will not have a raw base, so [`to_bytes`](Self::to_bytes)
    /// will produce a clean output starting from zeros.
    pub fn new(synth: SynthParams, sequencer: SequencerParams) -> Self {
        Self {
            synth,
            sequencer,
            raw_blob: None,
        }
    }

    /// Parse a complete program blob from a byte slice.
    ///
    /// The raw bytes are preserved internally so that [`to_bytes`](Self::to_bytes)
    /// can reproduce them faithfully, including hardware-specific flag bits
    /// in unused bit positions.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is too short or if sub-section
    /// parsing fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(SysexError::PayloadTooShort {
                expected: Self::SIZE,
                actual: bytes.len(),
            }
            .into());
        }
        let synth = SynthParams::from_bytes(&bytes[..SynthParams::SIZE])?;
        let sequencer = SequencerParams::from_bytes(&bytes[SequencerParams::OFFSET..Self::SIZE])?;

        // Preserve the raw blob for round-trip fidelity.
        let mut raw = Box::new([0u8; Self::SIZE]);
        raw.copy_from_slice(&bytes[..Self::SIZE]);

        Ok(Self {
            synth,
            sequencer,
            raw_blob: Some(raw),
        })
    }

    /// Serialize to a 1024-byte vector.
    ///
    /// If this `ProgramData` was created via [`from_bytes`](Self::from_bytes),
    /// the original raw blob is used as the base buffer, preserving
    /// hardware-specific flag bits that are not modeled by the typed fields.
    /// Otherwise a zeroed buffer is used.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);

        if let Some(ref raw) = self.raw_blob {
            // Use original blob bytes as the base for both sections.
            let synth_base: &[u8; SynthParams::SIZE] = raw[..SynthParams::SIZE].try_into().unwrap();
            let synth_bytes = self.synth.to_bytes_with_base(synth_base);
            out.extend_from_slice(&synth_bytes);

            let seq_base = &raw[SequencerParams::OFFSET..Self::SIZE];
            let seq_bytes = self.sequencer.to_bytes_with_base(seq_base);
            out.extend_from_slice(&seq_bytes);
        } else {
            let synth_bytes = self.synth.to_bytes();
            out.extend_from_slice(&synth_bytes);
            let seq_bytes = self.sequencer.to_bytes();
            out.extend_from_slice(&seq_bytes);
        }

        debug_assert_eq!(out.len(), Self::SIZE);
        out
    }
}

// ---------------------------------------------------------------------------
// SysEx builders and parsers
// ---------------------------------------------------------------------------

/// Build a SysEx message requesting the current program data.
///
/// ```text
/// [F0, 42, 3g, 00, 01, 51, 0x10, F7]
/// ```
pub fn build_current_program_request(channel: U4) -> Vec<u8> {
    build_sysex_request(channel, CURRENT_PROGRAM_REQUEST)
}

/// Build a SysEx message containing a current program dump.
///
/// The 1024-byte program data is encoded with the Korg 7-bit codec.
pub fn build_current_program_dump(channel: U4, data: &ProgramData) -> Vec<u8> {
    let raw = data.to_bytes();
    build_sysex(channel, CURRENT_PROGRAM_DUMP, &raw)
}

/// Parse a current program dump SysEx message.
///
/// # Errors
///
/// Returns an error if the message is malformed, has the wrong function ID,
/// or the payload cannot be parsed as program data.
pub fn parse_current_program_dump(bytes: &[u8]) -> Result<ProgramData> {
    let frame = parse_sysex(bytes)?;
    if frame.function_id != CURRENT_PROGRAM_DUMP {
        return Err(SysexError::WrongFunctionId {
            expected: CURRENT_PROGRAM_DUMP,
            found: frame.function_id,
        }
        .into());
    }
    ProgramData::from_bytes(&frame.data)
}

/// Build a SysEx message requesting a specific stored program.
///
/// The program number is split into LSB (bits 0--6) and MSB (bit 7+).
pub fn build_program_request(channel: U4, number: ProgramNumber) -> Vec<u8> {
    let n = number.value();
    let lsb = (n & 0x7F) as u8;
    let msb = ((n >> 7) & 0x7F) as u8;
    let mut msg = Vec::with_capacity(10);
    msg.push(0xF0);
    msg.push(crate::sysex::KORG_ID);
    msg.push(0x30 | channel.value());
    msg.extend_from_slice(&crate::sysex::DEVICE_ID);
    msg.push(PROGRAM_DATA_REQUEST);
    msg.push(lsb);
    msg.push(msb);
    msg.push(0xF7);
    msg
}

/// Build a SysEx message containing a stored program dump.
///
/// The program number is encoded in 2 bytes before the 7-bit-encoded data.
pub fn build_program_dump(channel: U4, number: ProgramNumber, data: &ProgramData) -> Vec<u8> {
    let raw = data.to_bytes();
    let encoded = crate::codec::encode_7bit(&raw);
    let n = number.value();
    let lsb = (n & 0x7F) as u8;
    let msb = ((n >> 7) & 0x7F) as u8;

    let mut msg = Vec::with_capacity(7 + 2 + encoded.len() + 1);
    msg.push(0xF0);
    msg.push(crate::sysex::KORG_ID);
    msg.push(0x30 | channel.value());
    msg.extend_from_slice(&crate::sysex::DEVICE_ID);
    msg.push(PROGRAM_DATA_DUMP);
    msg.push(lsb);
    msg.push(msb);
    msg.extend_from_slice(&encoded);
    msg.push(0xF7);
    msg
}

/// Parse a stored program dump SysEx message.
///
/// Returns the program number and the program data.
///
/// # Errors
///
/// Returns an error if the message is malformed, has the wrong function ID,
/// or the payload cannot be parsed.
pub fn parse_program_dump(bytes: &[u8]) -> Result<(ProgramNumber, ProgramData)> {
    // Manually parse the header to extract the program number before the
    // 7-bit-encoded payload.
    if bytes.len() < 10 {
        return Err(SysexError::InvalidHeader("message too short".to_string()).into());
    }
    if bytes[0] != 0xF0 || bytes[bytes.len() - 1] != 0xF7 {
        return Err(SysexError::InvalidHeader("missing F0/F7 markers".to_string()).into());
    }
    if bytes[1] != crate::sysex::KORG_ID {
        return Err(SysexError::InvalidHeader("wrong manufacturer ID".to_string()).into());
    }
    if bytes[2] & 0xF0 != 0x30 {
        return Err(SysexError::InvalidHeader("wrong channel byte".to_string()).into());
    }
    if bytes[3..6] != crate::sysex::DEVICE_ID {
        return Err(SysexError::InvalidHeader("wrong device ID".to_string()).into());
    }
    if bytes[6] != PROGRAM_DATA_DUMP {
        return Err(SysexError::WrongFunctionId {
            expected: PROGRAM_DATA_DUMP,
            found: bytes[6],
        }
        .into());
    }

    let lsb = bytes[7];
    let msb = bytes[8];
    let number = ProgramNumber::new(u16::from(lsb) | (u16::from(msb) << 7))?;

    // Decode the 7-bit payload (everything after the 2 program-number bytes,
    // before the trailing F7).
    let payload_wire = &bytes[9..bytes.len() - 1];
    let decoded = crate::codec::decode_7bit(payload_wire)?;
    let data = ProgramData::from_bytes(&decoded)?;

    Ok((number, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u8) -> U4 {
        U4::new(n).unwrap()
    }

    // ---------------------------------------------------------------
    // ProgramNumber
    // ---------------------------------------------------------------

    #[test]
    fn program_number_valid() {
        let pn = ProgramNumber::new(0).unwrap();
        assert_eq!(pn.value(), 0);
        assert_eq!(pn.bank(), 0);
        assert_eq!(pn.slot_in_bank(), 0);

        let pn = ProgramNumber::new(499).unwrap();
        assert_eq!(pn.value(), 499);
        assert_eq!(pn.bank(), 4);
        assert_eq!(pn.slot_in_bank(), 99);
    }

    #[test]
    fn program_number_invalid() {
        assert!(ProgramNumber::new(500).is_err());
        assert!(ProgramNumber::new(1000).is_err());
        assert!(ProgramNumber::new(u16::MAX).is_err());
    }

    #[test]
    fn program_number_bank_decomposition() {
        for n in 0..500u16 {
            let pn = ProgramNumber::new(n).unwrap();
            assert_eq!(u16::from(pn.bank()) * 100 + u16::from(pn.slot_in_bank()), n);
        }
    }

    #[test]
    fn program_number_display() {
        let pn = ProgramNumber::new(42).unwrap();
        assert_eq!(format!("{pn}"), "42");
    }

    #[test]
    fn program_number_ordering() {
        let a = ProgramNumber::new(10).unwrap();
        let b = ProgramNumber::new(20).unwrap();
        assert!(a < b);
    }

    #[test]
    fn program_number_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ProgramNumber::new(0).unwrap());
        set.insert(ProgramNumber::new(0).unwrap());
        assert_eq!(set.len(), 1);
    }

    // ---------------------------------------------------------------
    // ProgramData
    // ---------------------------------------------------------------

    #[test]
    fn program_data_from_bytes_valid() {
        let data = ProgramData::default();
        let bytes = data.to_bytes();
        assert_eq!(bytes.len(), 1024);
        let recovered = ProgramData::from_bytes(&bytes).unwrap();
        assert_eq!(data, recovered);
    }

    #[test]
    fn program_data_too_short() {
        let short = vec![0u8; 500];
        assert!(ProgramData::from_bytes(&short).is_err());
    }

    #[test]
    fn program_data_round_trip() {
        let mut data = ProgramData::default();
        data.synth.name = ProgramName::from_string("RoundTrip").unwrap();
        data.synth.cutoff = 800;
        data.synth.delay_on = true;
        data.sequencer.bpm = 1400;
        data.sequencer.steps[0].notes[0] = 60;
        data.sequencer.steps[0].velocities[0] = 127;

        let bytes = data.to_bytes();
        let recovered = ProgramData::from_bytes(&bytes).unwrap();
        assert_eq!(data, recovered);
    }

    #[test]
    fn program_data_size() {
        assert_eq!(SynthParams::SIZE + SequencerParams::SIZE, ProgramData::SIZE);
    }

    // ---------------------------------------------------------------
    // SysEx current program
    // ---------------------------------------------------------------

    #[test]
    fn current_program_request() {
        let msg = build_current_program_request(ch(0));
        assert_eq!(msg.len(), 8);
        assert_eq!(msg[6], CURRENT_PROGRAM_REQUEST);
    }

    #[test]
    fn current_program_dump_round_trip() {
        let data = ProgramData::default();
        let msg = build_current_program_dump(ch(3), &data);
        let recovered = parse_current_program_dump(&msg).unwrap();
        assert_eq!(data, recovered);
    }

    #[test]
    fn current_program_dump_wrong_function_id() {
        let msg = build_sysex_request(ch(0), 0x51);
        assert!(parse_current_program_dump(&msg).is_err());
    }

    #[test]
    fn current_program_dump_with_modifications() {
        let mut data = ProgramData::default();
        data.synth.name = ProgramName::from_string("Test Patch").unwrap();
        data.synth.vco1_wave = crate::param::enums::VcoWave::Sqr;
        data.synth.cutoff = 512;
        data.synth.resonance = 768;

        let msg = build_current_program_dump(ch(0), &data);
        let recovered = parse_current_program_dump(&msg).unwrap();
        assert_eq!(recovered.synth.name.as_str(), "Test Patch");
        assert_eq!(recovered.synth.vco1_wave, crate::param::enums::VcoWave::Sqr);
        assert_eq!(recovered.synth.cutoff, 512);
        assert_eq!(recovered.synth.resonance, 768);
    }

    // ---------------------------------------------------------------
    // SysEx stored program
    // ---------------------------------------------------------------

    #[test]
    fn program_request() {
        let pn = ProgramNumber::new(0).unwrap();
        let msg = build_program_request(ch(0), pn);
        assert_eq!(msg.len(), 10);
        assert_eq!(msg[6], PROGRAM_DATA_REQUEST);
        assert_eq!(msg[7], 0); // LSB
        assert_eq!(msg[8], 0); // MSB
    }

    #[test]
    fn program_request_large_number() {
        let pn = ProgramNumber::new(499).unwrap();
        let msg = build_program_request(ch(0), pn);
        // 499 = 0x1F3 => LSB = 0x73, MSB = 0x03
        assert_eq!(msg[7], (499u16 & 0x7F) as u8); // 0x73 = 115
        assert_eq!(msg[8], (499u16 >> 7) as u8); // 3
    }

    #[test]
    fn program_dump_round_trip() {
        let data = ProgramData::default();
        let pn = ProgramNumber::new(42).unwrap();
        let msg = build_program_dump(ch(5), pn, &data);
        let (recovered_pn, recovered_data) = parse_program_dump(&msg).unwrap();
        assert_eq!(recovered_pn, pn);
        assert_eq!(recovered_data, data);
    }

    #[test]
    fn program_dump_round_trip_large_number() {
        let data = ProgramData::default();
        let pn = ProgramNumber::new(499).unwrap();
        let msg = build_program_dump(ch(0), pn, &data);
        let (recovered_pn, recovered_data) = parse_program_dump(&msg).unwrap();
        assert_eq!(recovered_pn.value(), 499);
        assert_eq!(recovered_data, data);
    }

    #[test]
    fn program_dump_wrong_function_id() {
        // Build a current program dump and try to parse as stored program dump.
        let data = ProgramData::default();
        let msg = build_current_program_dump(ch(0), &data);
        assert!(parse_program_dump(&msg).is_err());
    }

    #[test]
    fn program_dump_too_short() {
        let msg = vec![0xF0, 0x42, 0x30, 0x00, 0x01, 0x51, 0x4C, 0xF7];
        assert!(parse_program_dump(&msg).is_err());
    }

    #[test]
    fn program_dump_with_data() {
        let mut data = ProgramData::default();
        data.synth.name = ProgramName::from_string("Stored").unwrap();
        data.synth.reverb_on = true;
        data.sequencer.bpm = 2000;

        let pn = ProgramNumber::new(150).unwrap();
        let msg = build_program_dump(ch(0), pn, &data);
        let (recovered_pn, recovered_data) = parse_program_dump(&msg).unwrap();
        assert_eq!(recovered_pn.value(), 150);
        assert_eq!(recovered_data.synth.name.as_str(), "Stored");
        assert!(recovered_data.synth.reverb_on);
        assert_eq!(recovered_data.sequencer.bpm, 2000);
    }

    #[test]
    fn program_dump_all_channels() {
        let data = ProgramData::default();
        let pn = ProgramNumber::new(0).unwrap();
        for c in 0..=15 {
            let msg = build_program_dump(ch(c), pn, &data);
            let (recovered_pn, _) = parse_program_dump(&msg).unwrap();
            assert_eq!(recovered_pn, pn);
        }
    }
}
