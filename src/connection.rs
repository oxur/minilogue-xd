//! High-level connection to a Korg Minilogue XD.
//!
//! [`MinilogueXd`] wraps any [`MidiOutput`] implementation, providing a
//! typed interface for sending MIDI messages on a specific channel.

use crate::error::Result;
use crate::message::{MidiMessage, ToMidiBytes, U4};
use crate::transport::MidiOutput;

/// A connection to a Minilogue XD on a specific MIDI channel.
///
/// The generic parameter `O` allows using any [`MidiOutput`] backend,
/// including [`MockOutput`](crate::transport::MockOutput) for testing and
/// `MidirOutput` for real hardware
/// (when the `midi-io` feature is enabled).
pub struct MinilogueXd<O: MidiOutput> {
    output: O,
    channel: U4,
}

impl<O: MidiOutput> MinilogueXd<O> {
    /// Creates a new connection using the given output and MIDI channel.
    pub fn new(output: O, channel: U4) -> Self {
        Self { output, channel }
    }

    /// Sends a [`MidiMessage`] through the underlying output.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying [`MidiOutput::send`] fails.
    pub fn send_message(&mut self, msg: &MidiMessage) -> Result<()> {
        let bytes = msg.to_midi_bytes();
        self.output.send(&bytes)
    }

    /// Returns the MIDI channel this connection is configured for.
    pub fn channel(&self) -> U4 {
        self.channel
    }

    /// Returns a reference to the underlying output.
    pub fn output(&self) -> &O {
        &self.output
    }

    /// Returns a mutable reference to the underlying output.
    pub fn output_mut(&mut self) -> &mut O {
        &mut self.output
    }
}

/// Convenience constructor for connecting to real hardware.
#[cfg(feature = "midi-io")]
impl MinilogueXd<crate::transport::MidirOutput> {
    /// Opens a connection to the Minilogue XD on the given MIDI channel,
    /// searching for a port whose name contains `port_name`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MidiIo`](crate::error::Error::MidiIo) if the port
    /// cannot be found or the connection fails.
    pub fn connect(port_name: &str, channel: U4) -> Result<Self> {
        let output = crate::transport::MidirOutput::connect(port_name)?;
        Ok(Self::new(output, channel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{NoteOn, U7};
    use crate::transport::MockOutput;

    #[test]
    fn send_message_via_mock() {
        let mock = MockOutput::new();
        let channel = U4::new(0).unwrap();
        let mut xd = MinilogueXd::new(mock, channel);

        let msg = MidiMessage::NoteOn(NoteOn {
            channel,
            key: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
        });

        xd.send_message(&msg).unwrap();

        let sent = xd.output().last_message().unwrap();
        assert_eq!(sent, &[0x90, 60, 100]);
    }

    #[test]
    fn channel_getter() {
        let mock = MockOutput::new();
        let channel = U4::new(5).unwrap();
        let xd = MinilogueXd::new(mock, channel);
        assert_eq!(xd.channel().value(), 5);
    }

    #[test]
    fn send_multiple_messages() {
        let mock = MockOutput::new();
        let channel = U4::new(0).unwrap();
        let mut xd = MinilogueXd::new(mock, channel);

        let note_on = MidiMessage::NoteOn(NoteOn {
            channel,
            key: U7::new(60).unwrap(),
            velocity: U7::new(100).unwrap(),
        });
        let clock = MidiMessage::TimingClock(crate::message::TimingClock);

        xd.send_message(&note_on).unwrap();
        xd.send_message(&clock).unwrap();

        assert_eq!(xd.output().messages().len(), 2);
        assert_eq!(xd.output().messages()[0], vec![0x90, 60, 100]);
        assert_eq!(xd.output().messages()[1], vec![0xF8]);
    }
}
