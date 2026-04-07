//! Read a patch from the Minilogue XD and display its settings.
//!
//! Usage: cargo run -p minilogue-xd --example read_patch [program_number]
//!
//! Program numbers are 0-based internally (0 = "001" on the XD display).
//! Default: 0 (the first patch, typically "Replicant xd").

use std::time::Duration;

use minilogue_xd::device;
use minilogue_xd::message::U4;
use minilogue_xd::sysex::program::ProgramNumber;
use minilogue_xd::sysex::transaction::SysexTransaction;
use minilogue_xd::transport::{MidirInput, MidirOutput};

fn main() -> minilogue_xd::Result<()> {
    let prog_num: u16 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let program = ProgramNumber::new(prog_num)?;

    // Connect to the XD.
    // SysEx: send on SOUND (direct to synth), listen on KBD/KNOB (synth responses).
    let out_ports = device::list_output_ports()?;
    let in_ports = device::list_input_ports()?;

    let out_port = out_ports
        .iter()
        .find(|p| p.contains(device::OUTPUT_PORT_SOUND))
        .expect("Minilogue XD SOUND port not found");

    let in_port = in_ports
        .iter()
        .find(|p| p.contains("KBD/KNOB"))
        .expect("Minilogue XD KBD/KNOB port not found");

    println!("Connecting to Minilogue XD...");
    let mut output = MidirOutput::connect(out_port)?;
    let mut input = MidirInput::connect(in_port)?;

    println!("Requesting program {prog_num:03}...\n");
    let mut tx = SysexTransaction::new(&mut output, &mut input, U4::new(0)?)
        .with_timeout(Duration::from_secs(5));

    let data = tx.request_program(program)?;
    let s = &data.synth;

    println!("=== Program {:03}: {} ===\n", prog_num, s.name);

    println!("--- Oscillators ---");
    println!(
        "VCO 1:           {} {} (pitch={}, shape={})",
        s.vco1_wave, s.vco1_octave, s.vco1_pitch, s.vco1_shape
    );
    println!(
        "VCO 2:           {} {} (pitch={}, shape={})",
        s.vco2_wave, s.vco2_octave, s.vco2_pitch, s.vco2_shape
    );
    println!(
        "Sync:            {}   Ring: {}",
        if s.sync { "On" } else { "Off" },
        if s.ring { "On" } else { "Off" }
    );
    println!("Cross Mod Depth: {}", s.cross_mod_depth);
    println!();

    println!("--- Multi Engine ---");
    println!("Type:            {}", s.multi_type);
    println!(
        "Noise/VPM/User:  {} / {} / {}",
        s.select_noise, s.select_vpm, s.select_user
    );
    println!();

    println!("--- Mixer ---");
    println!(
        "VCO1={} VCO2={} Multi={}",
        s.vco1_level, s.vco2_level, s.multi_level
    );
    println!();

    println!("--- Filter ---");
    println!(
        "Cutoff:          {} ({:.0}%)",
        s.cutoff,
        s.cutoff as f32 / 10.23
    );
    println!(
        "Resonance:       {} ({:.0}%)",
        s.resonance,
        s.resonance as f32 / 10.23
    );
    println!(
        "Drive:           {}   Keytrack: {}",
        s.cutoff_drive, s.cutoff_keytrack
    );
    println!();

    println!("--- Amp EG ---");
    println!(
        "A={} D={} S={} R={}",
        s.amp_eg_attack, s.amp_eg_decay, s.amp_eg_sustain, s.amp_eg_release
    );
    println!();

    println!("--- EG ---");
    println!(
        "A={} D={} Int={} Target={}",
        s.eg_attack, s.eg_decay, s.eg_int, s.eg_target
    );
    println!();

    println!("--- LFO ---");
    println!(
        "{} {} rate={} int={} → {}",
        s.lfo_wave, s.lfo_mode, s.lfo_rate, s.lfo_int, s.lfo_target
    );
    println!();

    println!("--- Effects ---");
    println!(
        "Mod FX:          {} (type={})",
        if s.mod_fx_on { "On" } else { "Off" },
        s.mod_fx_type
    );
    println!(
        "Delay:           {} {} (time={} depth={} dw={})",
        if s.delay_on { "On" } else { "Off" },
        s.delay_sub_type,
        s.delay_time,
        s.delay_depth,
        s.delay_dry_wet
    );
    println!(
        "Reverb:          {} {} (time={} depth={} dw={})",
        if s.reverb_on { "On" } else { "Off" },
        s.reverb_sub_type,
        s.reverb_time,
        s.reverb_depth,
        s.reverb_dry_wet
    );
    println!();

    println!("--- Sequencer ---");
    println!("BPM:             {:.1}", data.sequencer.bpm as f32 / 10.0);
    println!(
        "Steps:           {} @ {}",
        data.sequencer.step_length, data.sequencer.step_resolution
    );

    Ok(())
}
