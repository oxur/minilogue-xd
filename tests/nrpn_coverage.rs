//! Integration tests verifying NRPN round-trip coverage.
//!
//! For representative NRPN parameter variants, encode via `to_midi_sequence`,
//! feed the CC sequence into an `NrpnReceiver`, and verify the reconstructed
//! parameter matches the original.

use minilogue_xd::message::{ControlChange, U4, U7};
use minilogue_xd::param::encoding::{EightBitHighRes, FourteenBitParam, TenBitSysex};
use minilogue_xd::param::enums::VoiceModeType;
use minilogue_xd::param::nrpn::{NrpnParam, NrpnReceiver};

fn ch() -> U4 {
    U4::new(0).unwrap()
}

/// Feed a CC sequence into a receiver and return the last non-None result.
fn round_trip(param: &NrpnParam) -> Option<NrpnParam> {
    let msgs = param.to_midi_sequence(ch()).unwrap();
    let mut receiver = NrpnReceiver::new();
    let mut result = None;
    for msg in &msgs {
        if let Some(p) = receiver.feed(msg) {
            result = Some(p);
        }
    }
    result
}

#[test]
fn round_trip_program_name() {
    let param = NrpnParam::ProgramName(3, U7::new(b'X').unwrap());
    let recovered = round_trip(&param).expect("should recover ProgramName");
    assert_eq!(recovered, param);
}

#[test]
fn round_trip_voice_mode_type() {
    let param = NrpnParam::VoiceModeType(VoiceModeType::Unison);
    let recovered = round_trip(&param).expect("should recover VoiceModeType");
    assert_eq!(recovered, param);
}

#[test]
fn round_trip_ten_bit_multi_shape() {
    let val = TenBitSysex::new(512).unwrap();
    let param = NrpnParam::MultiShapeVpm(val);
    let recovered = round_trip(&param).expect("should recover MultiShapeVpm");
    assert_eq!(recovered, param);
}

#[test]
fn round_trip_eight_bit_joystick_range() {
    let val = EightBitHighRes::new(150).unwrap();
    let param = NrpnParam::JoystickRangePlus(val);
    let recovered = round_trip(&param).expect("should recover JoystickRangePlus");
    assert_eq!(recovered, param);
}

#[test]
fn round_trip_master_volume() {
    let val = FourteenBitParam::new(10000).unwrap();
    let param = NrpnParam::MasterVolume(val);
    let recovered = round_trip(&param).expect("should recover MasterVolume");
    assert_eq!(recovered, param);
}

#[test]
fn round_trip_bool_param() {
    let param = NrpnParam::LfoKeySync(true);
    let recovered = round_trip(&param).expect("should recover LfoKeySync");
    assert_eq!(recovered, param);

    let param_off = NrpnParam::LfoKeySync(false);
    let recovered_off = round_trip(&param_off).expect("should recover LfoKeySync(false)");
    assert_eq!(recovered_off, param_off);
}

#[test]
fn round_trip_raw_value_param() {
    let param = NrpnParam::BendRangePlus(7);
    let recovered = round_trip(&param).expect("should recover BendRangePlus");
    assert_eq!(recovered, param);
}

#[test]
fn nrpn_receiver_resets_on_unexpected_cc() {
    // Start an NRPN sequence but interleave a non-NRPN CC, which should
    // reset the FSM.
    let mut receiver = NrpnReceiver::new();
    let channel = ch();

    // CC99 (NRPN MSB)
    let cc99 = ControlChange {
        channel,
        controller: U7::new(99).unwrap(),
        value: U7::new(0).unwrap(),
    };
    assert!(receiver.feed(&cc99).is_none());

    // Inject an unrelated CC (e.g., CC 43 = Cutoff)
    let unrelated = ControlChange {
        channel,
        controller: U7::new(43).unwrap(),
        value: U7::new(64).unwrap(),
    };
    assert!(receiver.feed(&unrelated).is_none());

    // CC98 (NRPN LSB) — should not transition because FSM was reset
    let cc98 = ControlChange {
        channel,
        controller: U7::new(98).unwrap(),
        value: U7::new(0x0C).unwrap(),
    };
    assert!(receiver.feed(&cc98).is_none());

    // CC6 (Data MSB) — should not produce a result
    let cc6 = ControlChange {
        channel,
        controller: U7::new(6).unwrap(),
        value: U7::new(1).unwrap(),
    };
    assert!(
        receiver.feed(&cc6).is_none(),
        "FSM should have been reset by the unrelated CC"
    );
}
