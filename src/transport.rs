//! MIDI output transport abstraction.
//!
//! Provides the [`MidiOutput`] trait for sending raw MIDI bytes, along with
//! a [`MockOutput`] for testing and an optional `MidirOutput` backend
//! (behind the `midi-io` feature flag).

use crate::error::Result;

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

        let midi_out =
            midir::MidiOutput::new("minilogue-xd").map_err(|e| Error::MidiIo(e.to_string()))?;

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
            .connect(port, "minilogue-xd-out")
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

        let midi_out = midir::MidiOutput::new("minilogue-xd-list")
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
}
