//! Integration tests verifying builder output round-trips through serialization.
//!
//! Build a patch with [`PatchBuilder`], serialize to bytes via
//! [`ProgramData::to_bytes`], parse back via [`ProgramData::from_bytes`],
//! and compare key fields.

use minilogue_xd::builder::PatchBuilder;
use minilogue_xd::param::enums::*;
use minilogue_xd::sysex::program::ProgramData;

#[test]
fn default_patch_serialization_round_trip() {
    let original = PatchBuilder::new().build();
    let bytes = original.to_bytes();
    assert_eq!(bytes.len(), ProgramData::SIZE);

    let recovered = ProgramData::from_bytes(&bytes).unwrap();
    assert_eq!(recovered, original);
}

#[test]
fn named_patch_round_trip() {
    let original = PatchBuilder::new().name("BrassLead").unwrap().build();
    let bytes = original.to_bytes();
    let recovered = ProgramData::from_bytes(&bytes).unwrap();

    assert_eq!(recovered.synth.name.as_str(), "BrassLead");
    assert_eq!(recovered, original);
}

#[test]
fn complex_patch_round_trip() {
    let original = PatchBuilder::new()
        .name("TestPatch")
        .unwrap()
        .vco1(VcoWave::Saw, VcoOctave::Eight, 0.5, 0.3)
        .vco2(VcoWave::Sqr, VcoOctave::Four, 0.7, 0.0)
        .vco1_level(0.8)
        .vco2_level(0.6)
        .sync_ring(Sync::On, Ring::Off)
        .cross_mod_depth(0.25)
        .filter(0.75, 0.2, CutoffDrive::Half, CutoffKeytrack::Full)
        .amp_eg(0.1, 0.5, 0.7, 0.3)
        .eg(0.2, 0.4, 0.6, EgTarget::Cutoff)
        .lfo(LfoWave::Tri, LfoMode::Normal, 0.5, 0.3, LfoTarget::Pitch)
        .delay(true, DelaySubType::Stereo, 0.5, 0.5, 0.5)
        .reverb(true, ReverbSubType::Hall, 0.6, 0.4, 0.5)
        .portamento(64)
        .build();

    let bytes = original.to_bytes();
    assert_eq!(bytes.len(), ProgramData::SIZE);

    let recovered = ProgramData::from_bytes(&bytes).unwrap();

    // Verify key synth fields survived the round trip.
    assert_eq!(recovered.synth.name.as_str(), "TestPatch");
    assert_eq!(recovered.synth.vco1_wave, VcoWave::Saw);
    assert_eq!(recovered.synth.vco1_octave, VcoOctave::Eight);
    assert_eq!(recovered.synth.vco2_wave, VcoWave::Sqr);
    assert_eq!(recovered.synth.cutoff_drive, CutoffDrive::Half);
    assert_eq!(recovered.synth.cutoff_keytrack, CutoffKeytrack::Full);
    assert_eq!(recovered.synth.eg_target, EgTarget::Cutoff);
    assert_eq!(recovered.synth.portamento, 64);
    assert!(recovered.synth.sync);
    assert!(!recovered.synth.ring);
    assert!(recovered.synth.delay_on);
    assert!(recovered.synth.reverb_on);

    // Full equality check.
    assert_eq!(recovered, original);
}

#[test]
fn builder_with_custom_sequencer() {
    use minilogue_xd::sysex::program::SequencerParams;

    let seq = SequencerParams {
        bpm: 2400, // 240.0 BPM
        ..SequencerParams::default()
    };

    let original = PatchBuilder::new()
        .name("FastSeq")
        .unwrap()
        .build_with_sequencer(seq);

    let bytes = original.to_bytes();
    let recovered = ProgramData::from_bytes(&bytes).unwrap();

    assert_eq!(recovered.sequencer.bpm, 2400);
    assert_eq!(recovered.synth.name.as_str(), "FastSeq");
    assert_eq!(recovered, original);
}

#[test]
fn program_data_size_is_1024() {
    assert_eq!(ProgramData::SIZE, 1024);

    let data = ProgramData::default();
    let bytes = data.to_bytes();
    assert_eq!(bytes.len(), 1024);
}

#[test]
fn builder_clamps_out_of_range_floats() {
    // Values outside 0.0--1.0 should be clamped, not rejected.
    let patch = PatchBuilder::new()
        .vco1(VcoWave::Saw, VcoOctave::Eight, 1.5, -0.5)
        .filter(2.0, -1.0, CutoffDrive::Off, CutoffKeytrack::Off)
        .build();

    // Clamped to max/min.
    assert_eq!(patch.synth.vco1_pitch, 1023); // clamped from 1.5 to 1.0
    assert_eq!(patch.synth.vco1_shape, 0); // clamped from -0.5 to 0.0
    assert_eq!(patch.synth.cutoff, 1023); // clamped from 2.0 to 1.0
    assert_eq!(patch.synth.resonance, 0); // clamped from -1.0 to 0.0
}
