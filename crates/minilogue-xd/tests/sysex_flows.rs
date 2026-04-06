//! End-to-end SysEx transaction tests.
//!
//! These tests exercise the [`SysexTransaction`] public API using mock
//! transports, verifying that request/response flows produce correct results.

use std::time::Duration;

use minilogue_xd::message::types::U4;
use minilogue_xd::sysex::frame::{build_status, SysexStatus};
use minilogue_xd::sysex::global::{build_global_dump, GlobalParams};
use minilogue_xd::sysex::program::{build_current_program_dump, ProgramData, ProgramNumber};
use minilogue_xd::sysex::transaction::SysexTransaction;
use minilogue_xd::transport::{MockMidiInput, MockOutput};

fn ch() -> U4 {
    U4::new(0).unwrap()
}

#[test]
fn request_global_round_trip() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    // Build and queue a global data dump response.
    let params = GlobalParams::default();
    input.queue_response(build_global_dump(channel, &params));

    let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
    let result = txn.request_global().unwrap();
    assert_eq!(result, params);

    // Verify the request was actually sent.
    assert_eq!(output.messages().len(), 1);
}

#[test]
fn request_current_program_round_trip() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    let program = ProgramData::default();
    input.queue_response(build_current_program_dump(channel, &program));

    let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
    let result = txn.request_current_program().unwrap();
    assert_eq!(result, program);
}

#[test]
fn send_current_program_receives_ack() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    // Queue an ACK response.
    input.queue_response(build_status(channel, SysexStatus::DataLoadCompleted));

    let program = ProgramData::default();
    let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
    txn.send_current_program(&program).unwrap();

    // The transaction should have sent the program dump.
    assert_eq!(output.messages().len(), 1);
}

#[test]
fn send_current_program_nak_is_error() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    // Queue a NAK response.
    input.queue_response(build_status(channel, SysexStatus::DataLoadError));

    let program = ProgramData::default();
    let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
    let result = txn.send_current_program(&program);
    assert!(result.is_err(), "NAK should produce an error");
}

#[test]
fn transaction_timeout_when_no_response() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    // Do NOT queue any response — the mock will return None (simulating timeout).
    let mut txn = SysexTransaction::new(&mut output, &mut input, channel)
        .with_timeout(Duration::from_millis(10));
    let result = txn.request_global();
    assert!(
        result.is_err(),
        "No response should produce a timeout error"
    );
}

#[test]
fn transaction_custom_timeout() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    let txn = SysexTransaction::new(&mut output, &mut input, channel)
        .with_timeout(Duration::from_secs(10));
    assert_eq!(txn.timeout(), Duration::from_secs(10));
    assert_eq!(txn.channel(), channel);
}

#[test]
fn send_global_receives_ack() {
    let mut output = MockOutput::new();
    let mut input = MockMidiInput::new();
    let channel = ch();

    input.queue_response(build_status(channel, SysexStatus::DataLoadCompleted));

    let params = GlobalParams::default();
    let mut txn = SysexTransaction::new(&mut output, &mut input, channel);
    txn.send_global(&params).unwrap();

    assert_eq!(output.messages().len(), 1);
}

#[test]
fn program_number_addressing() {
    // Verify ProgramNumber bank/slot decomposition across boundaries.
    let p0 = ProgramNumber::new(0).unwrap();
    assert_eq!(p0.bank(), 0);
    assert_eq!(p0.slot_in_bank(), 0);

    let p99 = ProgramNumber::new(99).unwrap();
    assert_eq!(p99.bank(), 0);
    assert_eq!(p99.slot_in_bank(), 99);

    let p100 = ProgramNumber::new(100).unwrap();
    assert_eq!(p100.bank(), 1);
    assert_eq!(p100.slot_in_bank(), 0);

    let p499 = ProgramNumber::new(499).unwrap();
    assert_eq!(p499.bank(), 4);
    assert_eq!(p499.slot_in_bank(), 99);

    assert!(ProgramNumber::new(500).is_err());
}
