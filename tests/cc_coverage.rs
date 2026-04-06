//! Integration tests verifying CC parameter map completeness.
//!
//! These tests exercise the public API to ensure every CC number from the
//! Minilogue XD MIDI Implementation chart is correctly recognized by the
//! [`CcParamReceiver`], and that unrecognized CC numbers return `None`.

use minilogue_xd::message::{ControlChange, U4, U7};
use minilogue_xd::param::cc::CcParamReceiver;

fn make_cc(channel: U4, controller: u8, value: u8) -> ControlChange {
    ControlChange {
        channel,
        controller: U7::new(controller).unwrap(),
        value: U7::new(value).unwrap(),
    }
}

/// All CC numbers from the spec that should be recognized by the receiver.
const KNOWN_CCS: &[u8] = &[
    0, 1, 2, 5, 6, 16, 17, 18, 19, 20, 21, 22, 24, 26, 27, 28, 29, 32, 33, 34, 35, 36, 37, 39, 40,
    41, 43, 44, 48, 49, 50, 51, 53, 54, 56, 57, 58, 59, 63, 64, 80, 81, 83, 84, 88, 89, 90, 92, 93,
    94, 96, 98, 99, 103, 104, 105, 106, 107, 108, 109, 110, 118, 119,
];

/// CC numbers that should NOT be recognized (gaps in the implementation chart).
const UNKNOWN_CCS: &[u8] = &[
    3, 4, 7, 8, 9, 10, 11, 12, 13, 14, 15, 25, 30, 31, 38, 42, 45, 46, 47, 52, 55, 60, 61, 62, 65,
    66, 100, 101, 120, 127,
];

#[test]
fn all_known_cc_numbers_produce_params() {
    let mut receiver = CcParamReceiver::default();
    let ch = U4::new(0).unwrap();

    for &cc_num in KNOWN_CCS {
        // For 10-bit params, the receiver needs a CC63 preamble. We send
        // CC63 first for every CC to ensure the receiver can handle it
        // regardless of whether the param is 10-bit or not.
        let cc63 = make_cc(ch, 63, 0);
        receiver.feed(&cc63);

        let cc = make_cc(ch, cc_num, 64);
        let result = receiver.feed(&cc);
        assert!(result.is_some(), "CC {} should produce a CcParam", cc_num);
    }
}

#[test]
fn unknown_cc_numbers_return_none() {
    let mut receiver = CcParamReceiver::default();
    let ch = U4::new(0).unwrap();

    for &cc_num in UNKNOWN_CCS {
        let cc = make_cc(ch, cc_num, 64);
        let result = receiver.feed(&cc);
        assert!(result.is_none(), "CC {} should return None", cc_num);
    }
}

#[test]
fn cc_param_cc_number_round_trip() {
    // Verify that every CcParam produced by the receiver reports the
    // correct cc_number() back.
    let mut receiver = CcParamReceiver::default();
    let ch = U4::new(0).unwrap();

    for &cc_num in KNOWN_CCS {
        let cc63 = make_cc(ch, 63, 0);
        receiver.feed(&cc63);

        let cc = make_cc(ch, cc_num, 64);
        if let Some(param) = receiver.feed(&cc) {
            assert_eq!(
                param.cc_number(),
                cc_num,
                "CcParam produced by CC {} reports cc_number() = {}",
                cc_num,
                param.cc_number()
            );
        }
    }
}

#[test]
fn ten_bit_cc_requires_cc63_preamble() {
    // Sending a 10-bit CC without a CC63 preamble should still produce a
    // result (the MSB-only path), but with only 7 bits of resolution.
    let mut receiver = CcParamReceiver::default();
    let ch = U4::new(0).unwrap();

    // CC 43 (Cutoff) is a 10-bit param. Without CC63, it should still resolve.
    let cc = make_cc(ch, 43, 100);
    let result = receiver.feed(&cc);
    assert!(
        result.is_some(),
        "10-bit CC without CC63 preamble should still produce a param"
    );
}

#[test]
fn ten_bit_cc_with_cc63_produces_full_resolution() {
    // Send CC63 then a 10-bit CC, verify the value encodes both parts.
    let mut receiver = CcParamReceiver::default();
    let ch = U4::new(0).unwrap();

    // CC63 with LSB = 5, then CC 43 (Cutoff) with MSB = 100
    // Expected 10-bit value: (100 << 3) | 5 = 805
    let cc63 = make_cc(ch, 63, 5);
    receiver.feed(&cc63);

    let cc = make_cc(ch, 43, 100);
    let result = receiver.feed(&cc);
    assert!(result.is_some());

    if let minilogue_xd::param::cc::CcParam::Cutoff(ten_bit) = result.unwrap() {
        assert_eq!(ten_bit.value(), 805);
    } else {
        panic!("Expected CcParam::Cutoff");
    }
}
