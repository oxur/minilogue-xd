//! Read the current edit buffer and generate Rust patch code.
//!
//! Reads whatever patch is currently loaded on the Minilogue XD and outputs
//! ready-to-paste Rust code in two formats:
//!
//!   1. `apply_patch_params(&mut SynthParams)` — raw 10-bit values for SysEx
//!   2. `setup_patch(&mut RealtimeController)` — float CCs for real-time control
//!
//! Workflow: tweak a patch on the hardware, run this, paste the output into
//! your example or library code.
//!
//! Usage: cargo run -p minilogue-xd --example read_edit_buffer

use std::time::Duration;

use minilogue_xd::device;
use minilogue_xd::message::U4;
use minilogue_xd::sysex::program::SynthParams;
use minilogue_xd::sysex::transaction::SysexTransaction;
use minilogue_xd::transport::{MidirInput, MidirOutput};

/// Convert a 10-bit raw value (0–1023) to a 3-decimal float string.
fn raw_to_float(raw: u16) -> String {
    format!("{:.3}", raw as f64 / 1023.0)
}

/// Generate `apply_patch_params` code (raw SynthParams values).
fn print_apply_patch_params(s: &SynthParams) {
    println!("/// Apply patch settings to an existing SynthParams (raw 10-bit values).");
    println!("fn apply_patch_params(s: &mut SynthParams) {{");
    println!("    use minilogue_xd::sysex::program::ProgramName;");
    println!(
        "    s.name = ProgramName::from_string({:?}).unwrap();",
        s.name.as_str().trim()
    );

    println!("    s.vco1_wave = {:?};", s.vco1_wave);
    println!("    s.vco1_octave = {:?};", s.vco1_octave);
    println!("    s.vco1_pitch = {};", s.vco1_pitch);
    println!("    s.vco1_shape = {};", s.vco1_shape);
    println!("    s.vco2_wave = {:?};", s.vco2_wave);
    println!("    s.vco2_octave = {:?};", s.vco2_octave);
    println!("    s.vco2_pitch = {};", s.vco2_pitch);
    println!("    s.vco2_shape = {};", s.vco2_shape);
    println!("    s.vco1_level = {};", s.vco1_level);
    println!("    s.vco2_level = {};", s.vco2_level);
    println!("    s.sync = {};", s.sync);
    println!("    s.ring = {};", s.ring);
    println!("    s.cross_mod_depth = {};", s.cross_mod_depth);
    println!("    s.cutoff = {};", s.cutoff);
    println!("    s.resonance = {};", s.resonance);
    println!("    s.cutoff_drive = {:?};", s.cutoff_drive);
    println!("    s.cutoff_keytrack = {:?};", s.cutoff_keytrack);
    println!("    s.amp_eg_attack = {};", s.amp_eg_attack);
    println!("    s.amp_eg_decay = {};", s.amp_eg_decay);
    println!("    s.amp_eg_sustain = {};", s.amp_eg_sustain);
    println!("    s.amp_eg_release = {};", s.amp_eg_release);
    println!("    s.eg_attack = {};", s.eg_attack);
    println!("    s.eg_decay = {};", s.eg_decay);
    println!("    s.eg_int = {};", s.eg_int);
    println!("    s.eg_target = {:?};", s.eg_target);
    println!("    s.lfo_wave = {:?};", s.lfo_wave);
    println!("    s.lfo_mode = {:?};", s.lfo_mode);
    println!("    s.lfo_rate = {};", s.lfo_rate);
    println!("    s.lfo_int = {};", s.lfo_int);
    println!("    s.lfo_target = {:?};", s.lfo_target);
    println!("    s.mod_fx_on = {};", s.mod_fx_on);
    println!(
        "    s.mod_fx_type = {};  // raw program value",
        s.mod_fx_type
    );
    println!("    s.delay_on = {};", s.delay_on);
    println!("    s.delay_sub_type = {:?};", s.delay_sub_type);
    println!("    s.delay_time = {};", s.delay_time);
    println!("    s.delay_depth = {};", s.delay_depth);
    println!("    s.delay_dry_wet = {};", s.delay_dry_wet);
    println!("    s.reverb_on = {};", s.reverb_on);
    println!("    s.reverb_sub_type = {:?};", s.reverb_sub_type);
    println!("    s.reverb_time = {};", s.reverb_time);
    println!("    s.reverb_depth = {};", s.reverb_depth);
    println!("    s.reverb_dry_wet = {};", s.reverb_dry_wet);
    println!("}}");
}

/// Generate `setup_patch` code (float CCs for RealtimeController).
fn print_setup_patch(s: &SynthParams) {
    println!(
        "fn setup_patch(xd: &mut RealtimeController<MidirOutput>) -> minilogue_xd::Result<()> {{"
    );

    println!("    xd.set_vco1_wave({:?})?;", s.vco1_wave);
    println!("    xd.set_vco1_octave({:?})?;", s.vco1_octave);
    println!("    xd.set_vco1_pitch({})?;", raw_to_float(s.vco1_pitch));
    println!("    xd.set_vco1_shape({})?;", raw_to_float(s.vco1_shape));
    println!("    xd.set_vco2_wave({:?})?;", s.vco2_wave);
    println!("    xd.set_vco2_octave({:?})?;", s.vco2_octave);
    println!("    xd.set_vco2_pitch({})?;", raw_to_float(s.vco2_pitch));
    println!("    xd.set_vco2_shape({})?;", raw_to_float(s.vco2_shape));
    println!("    xd.set_vco1_level({})?;", raw_to_float(s.vco1_level));
    println!("    xd.set_vco2_level({})?;", raw_to_float(s.vco2_level));
    println!(
        "    xd.set_sync(Sync::{})?;",
        if s.sync { "On" } else { "Off" }
    );
    println!(
        "    xd.set_ring(Ring::{})?;",
        if s.ring { "On" } else { "Off" }
    );
    println!("    xd.set_cutoff({})?;", raw_to_float(s.cutoff));
    println!("    xd.set_resonance({})?;", raw_to_float(s.resonance));
    println!("    xd.set_cutoff_drive({:?})?;", s.cutoff_drive);
    println!("    xd.set_cutoff_keytrack({:?})?;", s.cutoff_keytrack);
    println!(
        "    xd.set_amp_eg_attack({})?;",
        raw_to_float(s.amp_eg_attack)
    );
    println!(
        "    xd.set_amp_eg_decay({})?;",
        raw_to_float(s.amp_eg_decay)
    );
    println!(
        "    xd.set_amp_eg_sustain({})?;",
        raw_to_float(s.amp_eg_sustain)
    );
    println!(
        "    xd.set_amp_eg_release({})?;",
        raw_to_float(s.amp_eg_release)
    );
    println!("    xd.set_eg_attack({})?;", raw_to_float(s.eg_attack));
    println!("    xd.set_eg_decay({})?;", raw_to_float(s.eg_decay));
    println!("    xd.set_eg_int({})?;", raw_to_float(s.eg_int));
    println!("    xd.set_eg_target({:?})?;", s.eg_target);
    println!("    xd.set_lfo_wave({:?})?;", s.lfo_wave);
    println!("    xd.set_lfo_mode({:?})?;", s.lfo_mode);
    println!("    xd.set_lfo_rate({})?;", raw_to_float(s.lfo_rate));
    println!("    xd.set_lfo_int({})?;", raw_to_float(s.lfo_int));
    println!("    xd.set_lfo_target({:?})?;", s.lfo_target);
    println!("    xd.set_mod_fx_on({})?;", s.mod_fx_on);
    println!(
        "    // mod_fx_type raw={} — set manually if needed",
        s.mod_fx_type
    );
    println!("    xd.set_delay_on({})?;", s.delay_on);
    println!("    xd.set_delay_sub_type({:?})?;", s.delay_sub_type);
    println!("    xd.set_delay_time({})?;", raw_to_float(s.delay_time));
    println!("    xd.set_delay_depth({})?;", raw_to_float(s.delay_depth));
    println!(
        "    xd.set_delay_dry_wet({})?;",
        raw_to_float(s.delay_dry_wet)
    );
    println!("    xd.set_reverb_on({})?;", s.reverb_on);
    println!("    xd.set_reverb_sub_type({:?})?;", s.reverb_sub_type);
    println!("    xd.set_reverb_time({})?;", raw_to_float(s.reverb_time));
    println!(
        "    xd.set_reverb_depth({})?;",
        raw_to_float(s.reverb_depth)
    );
    println!(
        "    xd.set_reverb_dry_wet({})?;",
        raw_to_float(s.reverb_dry_wet)
    );
    println!("    Ok(())");
    println!("}}");
}

fn main() -> minilogue_xd::Result<()> {
    let Some(out_port) = device::find_output(device::OutputPort::Sound)? else {
        eprintln!("Minilogue XD not found — is it connected via USB?");
        std::process::exit(1);
    };
    let Some(in_port) = device::find_input(device::InputPort::KbdKnob)? else {
        eprintln!("Minilogue XD input port not found.");
        std::process::exit(1);
    };

    let mut output = MidirOutput::connect(&out_port)?;
    let mut input = MidirInput::connect(&in_port)?;

    let mut tx = SysexTransaction::new(&mut output, &mut input, U4::new(0)?)
        .with_timeout(Duration::from_secs(5));

    eprintln!("Reading current edit buffer...");
    let data = tx.request_current_program()?;
    let s = &data.synth;

    eprintln!("Patch: \"{}\"", s.name);
    eprintln!();

    println!("// Generated from XD edit buffer: \"{}\"", s.name);
    println!("// Paste into your example code.\n");

    println!("// === Raw SynthParams (for SysEx / MIDI export) ===\n");
    print_apply_patch_params(s);

    println!("\n// === Float CCs (for RealtimeController / --save) ===\n");
    print_setup_patch(s);

    Ok(())
}
