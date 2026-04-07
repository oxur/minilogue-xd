//! SysEx transaction manager for request/response conversations.
//!
//! A [`SysexTransaction`] encapsulates the send-request / wait-for-response
//! pattern used by most Korg Minilogue XD SysEx operations. It handles:
//!
//! - Sending a SysEx request or data dump.
//! - Waiting for the device's response (with a configurable timeout).
//! - Detecting ACK ([`SysexStatus::DataLoadCompleted`]) and NAK status codes.
//! - Parsing the response into the appropriate domain type.
//!
//! # Example (with test doubles)
//!
//! ```
//! use std::time::Duration;
//! use minilogue_xd::message::types::U4;
//! use minilogue_xd::sysex::global::{build_global_dump, GlobalParams};
//! use minilogue_xd::sysex::transaction::SysexTransaction;
//! use minilogue_xd::transport::{MockMidiInput, MockOutput};
//!
//! let mut output = MockOutput::new();
//! let mut input = MockMidiInput::new();
//! let channel = U4::new(0).unwrap();
//!
//! // Queue a simulated device response.
//! let params = GlobalParams::default();
//! input.queue_response(build_global_dump(channel, &params));
//!
//! let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
//! let result = txn.request_global().unwrap();
//! assert_eq!(result, params);
//! ```

use std::time::Duration;

use crate::error::{Result, SysexError};
use crate::message::types::U4;
use crate::sysex::frame::{
    parse_sysex, SysexFrame, SysexStatus, CURRENT_PROGRAM_DUMP, GLOBAL_DATA_DUMP, USER_OCTAVE_DUMP,
    USER_SCALE_DUMP,
};
use crate::sysex::global::{self, GlobalParams};
use crate::sysex::identity::{self, IdentityReply};
use crate::sysex::program::{self, ProgramData, ProgramNumber};
use crate::sysex::tuning::{self, UserOctave, UserScale};
use crate::transport::{MidiInput, MidiOutput};

/// Default transaction timeout.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// A SysEx transaction manager for request/response conversations.
///
/// Borrows a [`MidiOutput`] and a [`MidiInput`] for the duration of one or
/// more request/response exchanges. Each high-level method sends a request,
/// waits for the response, checks for errors, and parses the result.
pub struct SysexTransaction<'a, O: MidiOutput, I: MidiInput> {
    output: &'a mut O,
    input: &'a mut I,
    channel: U4,
    timeout: Duration,
}

impl<'a, O: MidiOutput, I: MidiInput> SysexTransaction<'a, O, I> {
    /// Create a new transaction with the default timeout (5 seconds).
    pub fn new(output: &'a mut O, input: &'a mut I, channel: U4) -> Self {
        Self {
            output,
            input,
            channel,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Set a custom timeout for all operations on this transaction.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Returns the configured timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns the MIDI channel used by this transaction.
    pub fn channel(&self) -> U4 {
        self.channel
    }

    // -----------------------------------------------------------------------
    // Core helpers
    // -----------------------------------------------------------------------

    /// Send a request and wait for a response, parsing the frame and checking
    /// for NAK status codes and the expected function ID.
    ///
    /// This is used for messages whose payloads are entirely 7-bit encoded
    /// (current program, global, tuning). It is *not* suitable for stored
    /// program dumps, which have the program number outside the encoded
    /// payload.
    /// Receive the next SysEx message, skipping over realtime messages
    /// (clock F8, active sensing FE, etc.) that the synth may interleave.
    fn receive_sysex(&mut self) -> Result<Vec<u8>> {
        let deadline = std::time::Instant::now() + self.timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(SysexError::Timeout(self.timeout).into());
            }
            match self.input.receive(remaining)? {
                Some(bytes) if bytes.first() == Some(&0xF0) => return Ok(bytes),
                Some(_) => continue, // skip clock, active sensing, etc.
                None => return Err(SysexError::Timeout(self.timeout).into()),
            }
        }
    }

    fn request_response(&mut self, request: &[u8], expected_fn: u8) -> Result<SysexFrame> {
        self.output.send(request)?;
        let response_bytes = self.receive_sysex()?;

        let frame = parse_sysex(&response_bytes)?;

        // Check for device error status.
        if let Some(status) = SysexStatus::from_byte(frame.function_id) {
            if status.is_error() {
                return Err(SysexError::NakReceived(status).into());
            }
        }

        if frame.function_id != expected_fn {
            return Err(SysexError::UnexpectedResponse(frame.function_id).into());
        }

        Ok(frame)
    }

    /// Send a request and wait for the raw response bytes, checking for
    /// NAK status codes.
    ///
    /// Used for messages that need custom parsing (e.g., stored program dumps
    /// where the program number precedes the 7-bit encoded data).
    fn raw_request_response(&mut self, request: &[u8]) -> Result<Vec<u8>> {
        self.output.send(request)?;
        let response_bytes = self.receive_sysex()?;

        // Try to detect NAK status in the response. A NAK/ACK frame is a
        // minimal SysEx frame with just the status byte as the function ID.
        // We only check if the response is short enough to be a status frame
        // (8 bytes = min Korg SysEx frame with no payload).
        if response_bytes.len() == 8 {
            if let Ok(frame) = parse_sysex(&response_bytes) {
                if let Some(status) = SysexStatus::from_byte(frame.function_id) {
                    if status.is_error() {
                        return Err(SysexError::NakReceived(status).into());
                    }
                }
            }
        }

        Ok(response_bytes)
    }

    /// Send a data dump and wait for an ACK/NAK status response.
    fn send_and_wait_ack(&mut self, data: &[u8]) -> Result<()> {
        self.output.send(data)?;
        let response_bytes = self.receive_sysex()?;

        let frame = parse_sysex(&response_bytes)?;

        if let Some(status) = SysexStatus::from_byte(frame.function_id) {
            if status.is_error() {
                return Err(SysexError::NakReceived(status).into());
            }
            // ACK (DataLoadCompleted) — success.
            return Ok(());
        }

        Err(SysexError::UnexpectedResponse(frame.function_id).into())
    }

    // -----------------------------------------------------------------------
    // Program operations
    // -----------------------------------------------------------------------

    /// Request the current program from the device.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, unexpected response, or if the
    /// program data cannot be parsed.
    pub fn request_current_program(&mut self) -> Result<ProgramData> {
        let request = program::build_current_program_request(self.channel);
        let frame = self.request_response(&request, CURRENT_PROGRAM_DUMP)?;
        ProgramData::from_bytes(&frame.data)
    }

    /// Send a program to the device's current edit buffer.
    ///
    /// Waits for an ACK/NAK response.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, or if the send fails.
    pub fn send_current_program(&mut self, data: &ProgramData) -> Result<()> {
        let msg = program::build_current_program_dump(self.channel, data);
        self.send_and_wait_ack(&msg)
    }

    /// Request a specific stored program by number.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, unexpected response, or if the
    /// program data cannot be parsed.
    pub fn request_program(&mut self, number: ProgramNumber) -> Result<ProgramData> {
        let request = program::build_program_request(self.channel, number);
        let raw = self.raw_request_response(&request)?;
        let (_num, data) = program::parse_program_dump(&raw)?;
        Ok(data)
    }

    /// Send a program to a specific stored slot.
    ///
    /// Waits for an ACK/NAK response.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, or if the send fails.
    pub fn send_program(&mut self, number: ProgramNumber, data: &ProgramData) -> Result<()> {
        let msg = program::build_program_dump(self.channel, number, data);
        self.send_and_wait_ack(&msg)
    }

    // -----------------------------------------------------------------------
    // Global operations
    // -----------------------------------------------------------------------

    /// Request the global parameter data from the device.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, unexpected response, or if the
    /// global data cannot be parsed.
    pub fn request_global(&mut self) -> Result<GlobalParams> {
        let request = global::build_global_request(self.channel);
        let frame = self.request_response(&request, GLOBAL_DATA_DUMP)?;
        GlobalParams::from_bytes(&frame.data)
    }

    /// Send global parameter data to the device.
    ///
    /// Waits for an ACK/NAK response.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, or if the send fails.
    pub fn send_global(&mut self, params: &GlobalParams) -> Result<()> {
        let msg = global::build_global_dump(self.channel, params);
        self.send_and_wait_ack(&msg)
    }

    // -----------------------------------------------------------------------
    // Tuning operations
    // -----------------------------------------------------------------------

    /// Request the user scale tuning table from the device.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, unexpected response, or if the
    /// scale data cannot be parsed.
    pub fn request_user_scale(&mut self) -> Result<UserScale> {
        let request = tuning::build_user_scale_request(self.channel);
        let frame = self.request_response(&request, USER_SCALE_DUMP)?;
        UserScale::from_bytes(&frame.data)
    }

    /// Send a user scale tuning table to the device.
    ///
    /// Waits for an ACK/NAK response.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, or if the send fails.
    pub fn send_user_scale(&mut self, scale: &UserScale) -> Result<()> {
        let msg = tuning::build_user_scale_dump(self.channel, scale);
        self.send_and_wait_ack(&msg)
    }

    /// Request the user octave tuning table from the device.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, unexpected response, or if the
    /// octave data cannot be parsed.
    pub fn request_user_octave(&mut self) -> Result<UserOctave> {
        let request = tuning::build_user_octave_request(self.channel);
        let frame = self.request_response(&request, USER_OCTAVE_DUMP)?;
        UserOctave::from_bytes(&frame.data)
    }

    /// Send a user octave tuning table to the device.
    ///
    /// Waits for an ACK/NAK response.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout, NAK, or if the send fails.
    pub fn send_user_octave(&mut self, octave: &UserOctave) -> Result<()> {
        let msg = tuning::build_user_octave_dump(self.channel, octave);
        self.send_and_wait_ack(&msg)
    }

    // -----------------------------------------------------------------------
    // Identity
    // -----------------------------------------------------------------------

    /// Query the device identity via Universal Non-Realtime Identity Request.
    ///
    /// The identity reply uses a different SysEx format (Universal
    /// Non-Realtime) so it bypasses the Korg-specific frame parser.
    ///
    /// # Errors
    ///
    /// Returns an error on timeout or if the identity reply cannot be parsed.
    pub fn query_identity(&mut self) -> Result<IdentityReply> {
        let request = identity::build_identity_request(self.channel);
        self.output.send(&request)?;
        let response = self.receive_sysex()?;
        identity::parse_identity_reply(&response)
    }
}

// ---------------------------------------------------------------------------
// FailingOutput — test helper for simulating send failures
// ---------------------------------------------------------------------------

/// A mock output that always returns an error on `send`.
///
/// Used in tests to verify error propagation from the transport layer.
#[cfg(test)]
struct FailingOutput;

#[cfg(test)]
impl MidiOutput for FailingOutput {
    fn send(&mut self, _bytes: &[u8]) -> Result<()> {
        Err(crate::error::Error::InvalidMessage(
            "simulated send failure".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sysex::frame::{self, build_status};
    use crate::sysex::tuning::CentOffset;
    use crate::transport::{MockMidiInput, MockOutput};

    fn ch(n: u8) -> U4 {
        U4::new(n).expect("test channel")
    }

    // ===================================================================
    // Happy path — request operations
    // ===================================================================

    #[test]
    fn request_current_program_happy_path() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let data = ProgramData::default();
        let response = program::build_current_program_dump(channel, &data);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_current_program().unwrap();
        assert_eq!(result, data);

        // Verify the request was sent.
        assert_eq!(output.messages().len(), 1);
        assert_eq!(output.messages()[0][6], frame::CURRENT_PROGRAM_REQUEST);
    }

    #[test]
    fn request_program_happy_path() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(3);

        let data = ProgramData::default();
        let pn = ProgramNumber::new(42).unwrap();
        let response = program::build_program_dump(channel, pn, &data);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_program(pn).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn request_program_large_number() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let data = ProgramData::default();
        let pn = ProgramNumber::new(499).unwrap();
        let response = program::build_program_dump(channel, pn, &data);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_program(pn).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn request_global_happy_path() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(5);

        let params = GlobalParams::default();
        let response = global::build_global_dump(channel, &params);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_global().unwrap();
        assert_eq!(result, params);
    }

    #[test]
    fn request_user_scale_happy_path() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let scale = UserScale::equal_temperament();
        let response = tuning::build_user_scale_dump(channel, &scale);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_user_scale().unwrap();
        assert_eq!(result, scale);
    }

    #[test]
    fn request_user_octave_happy_path() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let octave = UserOctave::equal_temperament();
        let response = tuning::build_user_octave_dump(channel, &octave);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_user_octave().unwrap();
        assert_eq!(result, octave);
    }

    #[test]
    fn query_identity_happy_path() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        // Construct a valid identity reply.
        let reply_bytes = vec![
            0xF0, 0x7E, 0x00, 0x06, 0x02, // header
            0x42, // Korg
            0x51, 0x01, // family
            0x00, 0x00, // member
            0x02, 0x01, 0x00, 0x00, // version
            0xF7,
        ];
        input.queue_response(reply_bytes);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let reply = txn.query_identity().unwrap();
        assert_eq!(reply.manufacturer_id, 0x42);
        assert_eq!(reply.family_id, 0x0151);
    }

    // ===================================================================
    // Happy path — send operations (ACK)
    // ===================================================================

    #[test]
    fn send_current_program_ack() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let ack = build_status(channel, SysexStatus::DataLoadCompleted);
        input.queue_response(ack);

        let data = ProgramData::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        txn.send_current_program(&data).unwrap();

        // Verify the dump was sent.
        assert_eq!(output.messages().len(), 1);
        assert_eq!(output.messages()[0][6], frame::CURRENT_PROGRAM_DUMP);
    }

    #[test]
    fn send_program_ack() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let ack = build_status(channel, SysexStatus::DataLoadCompleted);
        input.queue_response(ack);

        let data = ProgramData::default();
        let pn = ProgramNumber::new(100).unwrap();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        txn.send_program(pn, &data).unwrap();

        assert_eq!(output.messages().len(), 1);
        assert_eq!(output.messages()[0][6], frame::PROGRAM_DATA_DUMP);
    }

    #[test]
    fn send_global_ack() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let ack = build_status(channel, SysexStatus::DataLoadCompleted);
        input.queue_response(ack);

        let params = GlobalParams::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        txn.send_global(&params).unwrap();
    }

    #[test]
    fn send_user_scale_ack() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let ack = build_status(channel, SysexStatus::DataLoadCompleted);
        input.queue_response(ack);

        let scale = UserScale::equal_temperament();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        txn.send_user_scale(&scale).unwrap();
    }

    #[test]
    fn send_user_octave_ack() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let ack = build_status(channel, SysexStatus::DataLoadCompleted);
        input.queue_response(ack);

        let octave = UserOctave::equal_temperament();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        txn.send_user_octave(&octave).unwrap();
    }

    // ===================================================================
    // Error paths — timeout
    // ===================================================================

    #[test]
    fn request_current_program_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        // No response queued.

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        let err = txn.request_current_program().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("timed out"), "got: {msg}");
    }

    #[test]
    fn send_current_program_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        // No ACK queued.

        let data = ProgramData::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        let err = txn.send_current_program(&data).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("timed out"), "got: {msg}");
    }

    #[test]
    fn request_global_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.request_global().is_err());
    }

    #[test]
    fn request_user_scale_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.request_user_scale().is_err());
    }

    #[test]
    fn request_user_octave_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.request_user_octave().is_err());
    }

    #[test]
    fn query_identity_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.query_identity().is_err());
    }

    #[test]
    fn request_program_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let pn = ProgramNumber::new(0).unwrap();
        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.request_program(pn).is_err());
    }

    // ===================================================================
    // Error paths — NAK
    // ===================================================================

    #[test]
    fn request_current_program_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::DataLoadError);
        input.queue_response(nak);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.request_current_program().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("error status"), "got: {msg}");
    }

    #[test]
    fn send_current_program_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::DataFormatError);
        input.queue_response(nak);

        let data = ProgramData::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.send_current_program(&data).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Data Format Error"), "got: {msg}");
    }

    #[test]
    fn request_global_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::DataLoadError);
        input.queue_response(nak);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        assert!(txn.request_global().is_err());
    }

    #[test]
    fn send_global_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::UserModuleError);
        input.queue_response(nak);

        let params = GlobalParams::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        assert!(txn.send_global(&params).is_err());
    }

    #[test]
    fn request_program_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        // An 8-byte NAK response will be detected by raw_request_response.
        let nak = build_status(channel, SysexStatus::DataLoadError);
        input.queue_response(nak);

        let pn = ProgramNumber::new(0).unwrap();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        assert!(txn.request_program(pn).is_err());
    }

    #[test]
    fn send_user_scale_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::DataLoadError);
        input.queue_response(nak);

        let scale = UserScale::equal_temperament();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        assert!(txn.send_user_scale(&scale).is_err());
    }

    #[test]
    fn send_user_octave_nak() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::DataLoadError);
        input.queue_response(nak);

        let octave = UserOctave::equal_temperament();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        assert!(txn.send_user_octave(&octave).is_err());
    }

    // ===================================================================
    // Error paths — unexpected response
    // ===================================================================

    #[test]
    fn request_current_program_wrong_function_id() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        // Send a global dump instead of a current program dump.
        let params = GlobalParams::default();
        let wrong_response = global::build_global_dump(channel, &params);
        input.queue_response(wrong_response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.request_current_program().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unexpected response"), "got: {msg}");
    }

    #[test]
    fn request_global_wrong_function_id() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        // Return a current program dump instead of global.
        let data = ProgramData::default();
        let wrong_response = program::build_current_program_dump(channel, &data);
        input.queue_response(wrong_response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.request_global().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unexpected response"), "got: {msg}");
    }

    #[test]
    fn send_and_wait_ack_unexpected_response() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        // Return a request frame (not an ACK/NAK status).
        let unexpected = frame::build_sysex_request(channel, frame::GLOBAL_DATA_REQUEST);
        input.queue_response(unexpected);

        let data = ProgramData::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.send_current_program(&data).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unexpected response"), "got: {msg}");
    }

    // ===================================================================
    // Error paths — send failure
    // ===================================================================

    #[test]
    fn request_current_program_send_failure() {
        let mut output = FailingOutput;
        let mut input = MockMidiInput::new();

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        let err = txn.request_current_program().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("simulated send failure"), "got: {msg}");
    }

    #[test]
    fn send_current_program_send_failure() {
        let mut output = FailingOutput;
        let mut input = MockMidiInput::new();

        let data = ProgramData::default();
        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.send_current_program(&data).is_err());
    }

    #[test]
    fn query_identity_send_failure() {
        let mut output = FailingOutput;
        let mut input = MockMidiInput::new();

        let mut txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert!(txn.query_identity().is_err());
    }

    // ===================================================================
    // Configuration and accessors
    // ===================================================================

    #[test]
    fn default_timeout_is_5_seconds() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let txn = SysexTransaction::new(&mut output, &mut input, ch(0));
        assert_eq!(txn.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn custom_timeout() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let txn = SysexTransaction::new(&mut output, &mut input, ch(0))
            .with_timeout(Duration::from_millis(500));
        assert_eq!(txn.timeout(), Duration::from_millis(500));
    }

    #[test]
    fn channel_accessor() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();

        let txn = SysexTransaction::new(&mut output, &mut input, ch(7));
        assert_eq!(txn.channel(), ch(7));
    }

    #[test]
    fn channel_propagation_in_request() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(12);

        // Queue a valid response so the transaction completes.
        let params = GlobalParams::default();
        input.queue_response(global::build_global_dump(channel, &params));

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        txn.request_global().unwrap();

        // Verify channel encoding in the sent message: byte[2] = 0x30 | channel.
        let sent = &output.messages()[0];
        assert_eq!(sent[2], 0x30 | 12);
    }

    // ===================================================================
    // NAK with various error statuses
    // ===================================================================

    #[test]
    fn nak_user_data_crc_error() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::UserDataCrcError);
        input.queue_response(nak);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.request_current_program().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("User Data CRC Error"), "got: {msg}");
    }

    #[test]
    fn nak_user_target_error() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let nak = build_status(channel, SysexStatus::UserTargetError);
        input.queue_response(nak);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let err = txn.request_user_scale().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("User Target Error"), "got: {msg}");
    }

    // ===================================================================
    // Multiple operations on one transaction
    // ===================================================================

    #[test]
    fn multiple_operations_sequential() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        // Queue responses for two operations.
        let params = GlobalParams::default();
        input.queue_response(global::build_global_dump(channel, &params));

        let data = ProgramData::default();
        input.queue_response(program::build_current_program_dump(channel, &data));

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);

        let g = txn.request_global().unwrap();
        assert_eq!(g, params);

        let p = txn.request_current_program().unwrap();
        assert_eq!(p, data);

        assert_eq!(output.messages().len(), 2);
    }

    // ===================================================================
    // Tuning with custom data
    // ===================================================================

    #[test]
    fn request_user_scale_custom_data() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let mut scale = UserScale::equal_temperament();
        // Modify one note.
        scale.0[60] = CentOffset::from_cents(60, 50.0);

        let response = tuning::build_user_scale_dump(channel, &scale);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_user_scale().unwrap();
        assert_eq!(result.0[60].semitone, 60);
    }

    #[test]
    fn request_user_octave_custom_data() {
        let mut output = MockOutput::new();
        let mut input = MockMidiInput::new();
        let channel = ch(0);

        let mut octave = UserOctave::equal_temperament();
        octave.0[6] = CentOffset::from_cents(6, 25.0);

        let response = tuning::build_user_octave_dump(channel, &octave);
        input.queue_response(response);

        let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
        let result = txn.request_user_octave().unwrap();
        assert_eq!(result.0[6].semitone, 6);
    }

    // ===================================================================
    // Error display tests for new SysexError variants
    // ===================================================================

    #[test]
    fn timeout_error_display() {
        let e: crate::error::Error = SysexError::Timeout(Duration::from_millis(500)).into();
        let msg = e.to_string();
        assert!(msg.contains("timed out"), "got: {msg}");
        assert!(msg.contains("500ms"), "got: {msg}");
    }

    #[test]
    fn nak_received_error_display() {
        let e: crate::error::Error = SysexError::NakReceived(SysexStatus::DataLoadError).into();
        let msg = e.to_string();
        assert!(msg.contains("Data Load Error"), "got: {msg}");
    }

    #[test]
    fn unexpected_response_error_display() {
        let e: crate::error::Error = SysexError::UnexpectedResponse(0x51).into();
        let msg = e.to_string();
        assert!(msg.contains("0x51"), "got: {msg}");
    }
}
