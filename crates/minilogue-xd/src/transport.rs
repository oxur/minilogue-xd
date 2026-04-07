//! MIDI transport abstractions (output and input).
//!
//! Provides the [`MidiOutput`] trait for sending raw MIDI bytes and the
//! [`MidiInput`] trait for receiving them, along with test doubles
//! ([`MockOutput`], [`MockMidiInput`]) and optional `midir`-backed
//! implementations (behind the `midi-io` feature flag).

use std::collections::VecDeque;
use std::time::Duration;

use crate::error::Result;

/// A MIDI input port capable of receiving raw byte messages.
///
/// Implementations must be `Send` so they can be shared across threads.
pub trait MidiInput: Send {
    /// Wait for the next complete MIDI message, up to the given timeout.
    ///
    /// Returns `Ok(Some(bytes))` when a message is received, `Ok(None)` on
    /// timeout, or `Err` on an I/O failure.
    fn receive(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>>;
}

/// A MIDI output port capable of sending raw byte messages.
///
/// Implementations must be `Send` so they can be shared across threads
/// (e.g., in async contexts or with thread-based schedulers).
pub trait MidiOutput: Send {
    /// Sends a raw MIDI message.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying I/O layer fails to deliver the message.
    fn send(&mut self, bytes: &[u8]) -> Result<()>;
}

/// A test double that records all sent MIDI messages in memory.
///
/// # Examples
///
/// ```
/// use minilogue_xd::transport::{MockOutput, MidiOutput};
///
/// let mut out = MockOutput::new();
/// out.send(&[0x90, 60, 100]).unwrap();
/// assert_eq!(out.last_message(), Some(&[0x90, 60, 100][..]));
/// ```
#[derive(Debug, Default)]
pub struct MockOutput {
    messages: Vec<Vec<u8>>,
}

impl MockOutput {
    /// Creates a new, empty `MockOutput`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a slice of all recorded messages.
    pub fn messages(&self) -> &[Vec<u8>] {
        &self.messages
    }

    /// Returns the most recently sent message, or `None` if no messages
    /// have been sent.
    pub fn last_message(&self) -> Option<&[u8]> {
        self.messages.last().map(Vec::as_slice)
    }

    /// Clears all recorded messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

impl MidiOutput for MockOutput {
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        self.messages.push(bytes.to_vec());
        Ok(())
    }
}

/// A test double that plays back pre-scripted MIDI responses.
///
/// Each call to [`receive`](MidiInput::receive) pops the next queued
/// response. When the queue is empty, `receive` returns `Ok(None)`,
/// simulating a timeout.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use minilogue_xd::transport::{MockMidiInput, MidiInput};
///
/// let mut inp = MockMidiInput::new();
/// inp.queue_response(vec![0xF0, 0x42, 0xF7]);
/// let msg = inp.receive(Duration::from_secs(1)).unwrap();
/// assert_eq!(msg, Some(vec![0xF0, 0x42, 0xF7]));
/// // Queue is now empty — simulates timeout.
/// assert_eq!(inp.receive(Duration::from_secs(1)).unwrap(), None);
/// ```
#[derive(Debug, Default)]
pub struct MockMidiInput {
    responses: VecDeque<Vec<u8>>,
}

impl MockMidiInput {
    /// Creates a new, empty `MockMidiInput`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a response that will be returned by the next `receive()` call.
    pub fn queue_response(&mut self, bytes: Vec<u8>) {
        self.responses.push_back(bytes);
    }

    /// Queue multiple responses.
    pub fn queue_responses(&mut self, responses: impl IntoIterator<Item = Vec<u8>>) {
        self.responses.extend(responses);
    }

    /// Returns the number of queued responses remaining.
    pub fn remaining(&self) -> usize {
        self.responses.len()
    }
}

impl MidiInput for MockMidiInput {
    fn receive(&mut self, _timeout: Duration) -> Result<Option<Vec<u8>>> {
        Ok(self.responses.pop_front())
    }
}

// ---------------------------------------------------------------------------
// midir backend (feature-gated)
// ---------------------------------------------------------------------------

/// A real MIDI output backed by [`midir`].
///
/// Only available when the `midi-io` feature is enabled.
#[cfg(feature = "midi-io")]
pub struct MidirOutput {
    connection: midir::MidiOutputConnection,
}

#[cfg(feature = "midi-io")]
impl MidirOutput {
    /// Opens a connection to the MIDI output port whose name contains
    /// `port_name`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MidiIo`](crate::error::Error::MidiIo) if no matching
    /// port is found or the connection cannot be established.
    pub fn connect(port_name: &str) -> Result<Self> {
        use crate::error::Error;

        let midi_out = midir::MidiOutput::new(crate::device::MIDI_CLIENT_NAME)
            .map_err(|e| Error::MidiIo(e.to_string()))?;

        let ports = midi_out.ports();
        let port = ports
            .iter()
            .find(|p| {
                midi_out
                    .port_name(p)
                    .map(|n| n.contains(port_name))
                    .unwrap_or(false)
            })
            .ok_or_else(|| Error::MidiIo(format!("no MIDI output port matching '{port_name}'")))?;

        let connection = midi_out
            .connect(port, crate::device::MIDI_OUT_PORT_NAME)
            .map_err(|e| Error::MidiIo(e.to_string()))?;

        Ok(Self { connection })
    }

    /// Lists the names of all available MIDI output ports.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MidiIo`](crate::error::Error::MidiIo) if the MIDI
    /// subsystem cannot be initialized.
    pub fn available_ports() -> Result<Vec<String>> {
        use crate::error::Error;

        let midi_out = midir::MidiOutput::new(crate::device::MIDI_CLIENT_NAME)
            .map_err(|e| Error::MidiIo(e.to_string()))?;

        let ports = midi_out.ports();
        let names: Vec<String> = ports
            .iter()
            .filter_map(|p| midi_out.port_name(p).ok())
            .collect();

        Ok(names)
    }
}

#[cfg(feature = "midi-io")]
impl MidiOutput for MidirOutput {
    fn send(&mut self, bytes: &[u8]) -> Result<()> {
        use crate::error::Error;
        self.connection
            .send(bytes)
            .map_err(|e| Error::MidiIo(e.to_string()))
    }
}

#[cfg(feature = "midi-io")]
impl std::fmt::Debug for MidirOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidirOutput")
            .field("connection", &"<MidiOutputConnection>")
            .finish()
    }
}

// ---------------------------------------------------------------------------
// MidirInput (feature-gated)
// ---------------------------------------------------------------------------

/// A real MIDI input backed by [`midir`].
///
/// Incoming messages are forwarded from the `midir` callback thread to the
/// calling thread via an `mpsc` channel. [`receive`](MidiInput::receive) blocks
/// until a message arrives or the timeout expires.
///
/// Only available when the `midi-io` feature is enabled.
#[cfg(feature = "midi-io")]
pub struct MidirInput {
    /// Held to keep the connection alive; dropped when `MidirInput` is dropped.
    _connection: midir::MidiInputConnection<()>,
    receiver: std::sync::mpsc::Receiver<Vec<u8>>,
}

#[cfg(feature = "midi-io")]
impl MidirInput {
    /// Opens a connection to the MIDI input port whose name contains
    /// `port_name`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MidiIo`](crate::error::Error::MidiIo) if no matching
    /// port is found or the connection cannot be established.
    pub fn connect(port_name: &str) -> Result<Self> {
        use crate::error::Error;

        let midi_in = midir::MidiInput::new(crate::device::MIDI_CLIENT_NAME)
            .map_err(|e| Error::MidiIo(e.to_string()))?;

        let ports = midi_in.ports();
        let port = ports
            .iter()
            .find(|p| {
                midi_in
                    .port_name(p)
                    .map(|n| n.contains(port_name))
                    .unwrap_or(false)
            })
            .ok_or_else(|| Error::MidiIo(format!("no MIDI input port matching '{port_name}'")))?
            .clone();

        let (tx, rx) = std::sync::mpsc::channel();

        let connection = midi_in
            .connect(
                &port,
                crate::device::MIDI_IN_PORT_NAME,
                move |_timestamp, message, _| {
                    // Ignore send errors — the receiver may have been dropped.
                    let _ = tx.send(message.to_vec());
                },
                (),
            )
            .map_err(|e| Error::MidiIo(e.to_string()))?;

        Ok(Self {
            _connection: connection,
            receiver: rx,
        })
    }

    /// Lists the names of all available MIDI input ports.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MidiIo`](crate::error::Error::MidiIo) if the MIDI
    /// subsystem cannot be initialized.
    pub fn available_ports() -> Result<Vec<String>> {
        use crate::error::Error;

        let midi_in = midir::MidiInput::new(crate::device::MIDI_CLIENT_NAME)
            .map_err(|e| Error::MidiIo(e.to_string()))?;

        let ports = midi_in.ports();
        let names: Vec<String> = ports
            .iter()
            .filter_map(|p| midi_in.port_name(p).ok())
            .collect();

        Ok(names)
    }
}

#[cfg(feature = "midi-io")]
impl MidiInput for MidirInput {
    fn receive(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        use crate::error::Error;

        match self.receiver.recv_timeout(timeout) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(Error::MidiIo("MIDI input disconnected".to_string()))
            }
        }
    }
}

#[cfg(feature = "midi-io")]
impl std::fmt::Debug for MidirInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidirInput")
            .field("_connection", &"<MidiInputConnection>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_output_captures_messages() {
        let mut out = MockOutput::new();
        out.send(&[0x90, 60, 100]).unwrap();
        out.send(&[0x80, 60, 0]).unwrap();
        assert_eq!(out.messages().len(), 2);
        assert_eq!(out.messages()[0], vec![0x90, 60, 100]);
        assert_eq!(out.messages()[1], vec![0x80, 60, 0]);
    }

    #[test]
    fn mock_output_last_message() {
        let mut out = MockOutput::new();
        assert_eq!(out.last_message(), None);
        out.send(&[0xF8]).unwrap();
        assert_eq!(out.last_message(), Some(&[0xF8][..]));
    }

    #[test]
    fn mock_output_clear() {
        let mut out = MockOutput::new();
        out.send(&[0x90, 60, 100]).unwrap();
        assert_eq!(out.messages().len(), 1);
        out.clear();
        assert!(out.messages().is_empty());
        assert_eq!(out.last_message(), None);
    }

    #[test]
    fn mock_output_default() {
        let out = MockOutput::default();
        assert!(out.messages().is_empty());
    }

    // ---------------------------------------------------------------
    // MockMidiInput
    // ---------------------------------------------------------------

    #[test]
    fn mock_input_returns_queued_responses() {
        let mut inp = MockMidiInput::new();
        inp.queue_response(vec![0xF0, 0x01, 0xF7]);
        inp.queue_response(vec![0xF0, 0x02, 0xF7]);

        let r1 = inp.receive(Duration::from_secs(1)).unwrap();
        assert_eq!(r1, Some(vec![0xF0, 0x01, 0xF7]));

        let r2 = inp.receive(Duration::from_secs(1)).unwrap();
        assert_eq!(r2, Some(vec![0xF0, 0x02, 0xF7]));
    }

    #[test]
    fn mock_input_returns_none_when_empty() {
        let mut inp = MockMidiInput::new();
        let r = inp.receive(Duration::from_secs(1)).unwrap();
        assert_eq!(r, None);
    }

    #[test]
    fn mock_input_queue_responses_batch() {
        let mut inp = MockMidiInput::new();
        inp.queue_responses(vec![vec![0x01], vec![0x02], vec![0x03]]);
        assert_eq!(inp.remaining(), 3);
        let _ = inp.receive(Duration::from_secs(1)).unwrap();
        assert_eq!(inp.remaining(), 2);
    }

    #[test]
    fn mock_input_default() {
        let inp = MockMidiInput::default();
        assert_eq!(inp.remaining(), 0);
    }

    #[test]
    fn mock_input_debug() {
        let inp = MockMidiInput::new();
        let _dbg = format!("{inp:?}");
    }
}
