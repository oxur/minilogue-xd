//! Diagnostic: test SysEx program round-trip fidelity against real hardware.
//!
//! Reads the current program from the XD, gets the raw decoded bytes,
//! parses them into ProgramData, re-serializes, and diffs byte-by-byte.
//! This identifies exactly which bytes change during the round-trip.
//!
//! Usage: cargo run -p minilogue-xd --example sysex_roundtrip_diag

use std::time::Duration;

use minilogue_xd::device;
use minilogue_xd::message::U4;
use minilogue_xd::sysex::frame::{self, parse_sysex};
use minilogue_xd::sysex::program::{self, ProgramData};
use minilogue_xd::transport::{MidiInput, MidiOutput, MidirInput, MidirOutput};

/// Compare two stored program slots byte-by-byte.
fn compare_programs(
    output: &mut MidirOutput,
    input: &mut MidirInput,
    channel: U4,
    slot_a: u16,
    slot_b: u16,
) -> minilogue_xd::Result<()> {
    use minilogue_xd::sysex::program::ProgramNumber;
    use minilogue_xd::sysex::transaction::SysexTransaction;

    let mut tx = SysexTransaction::new(output, input, channel).with_timeout(Duration::from_secs(5));

    println!("Reading program {}...", slot_a);
    let data_a = tx.request_program(ProgramNumber::new(slot_a)?)?;
    let bytes_a = data_a.to_bytes();

    println!("Reading program {}...", slot_b);
    let data_b = tx.request_program(ProgramNumber::new(slot_b)?)?;
    let bytes_b = data_b.to_bytes();

    println!(
        "Comparing: \"{}\" (slot {}) vs \"{}\" (slot {})\n",
        data_a.synth.name, slot_a, data_b.synth.name, slot_b
    );

    let mut diffs = 0;
    for (i, (a, b)) in bytes_a.iter().zip(bytes_b.iter()).enumerate() {
        if a != b {
            let region = if i < 156 { "Synth" } else { "Seq" };
            let offset = if i < 156 { i } else { i - 156 };
            println!(
                "  byte {:4} ({} {:3}): {} = 0x{:02X} ({:3})  |  {} = 0x{:02X} ({:3})",
                i, region, offset, slot_a, a, a, slot_b, b, b
            );
            diffs += 1;
        }
    }

    if diffs == 0 {
        println!("=== Programs are identical ===");
    } else {
        println!("\n=== {} bytes differ ===", diffs);
    }

    Ok(())
}

fn main() -> minilogue_xd::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let Some(out_port) = device::find_output(device::OutputPort::Sound)? else {
        eprintln!("Minilogue XD not found");
        std::process::exit(1);
    };
    let Some(in_port) = device::find_input(device::InputPort::KbdKnob)? else {
        eprintln!("Minilogue XD input port not found");
        std::process::exit(1);
    };

    let mut output = MidirOutput::connect(&out_port)?;
    let mut input = MidirInput::connect(&in_port)?;
    let channel = U4::new(0)?;

    // --compare A B: compare two stored program slots
    if args.iter().any(|a| a == "--compare") {
        let nums: Vec<u16> = args.iter().filter_map(|a| a.parse().ok()).collect();
        if nums.len() != 2 {
            eprintln!("Usage: --compare SLOT_A SLOT_B");
            std::process::exit(1);
        }
        return compare_programs(&mut output, &mut input, channel, nums[0], nums[1]);
    }

    let timeout = Duration::from_secs(5);

    // Step 1: Send current program request
    println!("Requesting current program...");
    let request = program::build_current_program_request(channel);
    output.send(&request)?;

    // Step 2: Receive raw SysEx response (reassemble fragments, skip realtime)
    let deadline = std::time::Instant::now() + timeout;
    let raw_sysex = {
        let mut buf: Vec<u8> = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                eprintln!("Timeout waiting for response");
                std::process::exit(1);
            }
            if let Some(bytes) = input.receive(remaining)? {
                let bytes: Vec<u8> = bytes;
                // Skip single-byte realtime messages (F8=clock, FE=active sensing)
                if bytes.len() == 1 && bytes[0] >= 0xF8 {
                    continue;
                }
                buf.extend_from_slice(&bytes);
                // Complete when we see F7 (End of SysEx)
                if buf.last() == Some(&0xF7) {
                    break buf;
                }
            }
        }
    };

    println!("Received {} raw SysEx bytes", raw_sysex.len());

    // Step 3: Parse the SysEx frame to get decoded 8-bit payload
    let frame = parse_sysex(&raw_sysex)?;
    assert_eq!(frame.function_id, frame::CURRENT_PROGRAM_DUMP);
    let original_bytes = &frame.data;
    println!("Decoded payload: {} bytes", original_bytes.len());

    // Step 4: Parse into ProgramData
    let program = ProgramData::from_bytes(original_bytes)?;
    println!("Parsed program: \"{}\"", program.synth.name);

    // Step 5: Re-serialize
    let reserialized = program.to_bytes();
    println!("Re-serialized: {} bytes", reserialized.len());

    // Step 6: Byte-by-byte diff
    assert_eq!(original_bytes.len(), reserialized.len());

    let mut diffs = 0;
    for (i, (orig, reser)) in original_bytes.iter().zip(reserialized.iter()).enumerate() {
        if orig != reser {
            let region = if i < 156 {
                "SynthParams"
            } else {
                "SequencerParams"
            };
            let offset = if i < 156 { i } else { i - 156 };
            println!(
                "  DIFF at byte {:4} ({} offset {:3}): original=0x{:02X} ({:3}), reserialized=0x{:02X} ({:3})",
                i, region, offset, orig, orig, reser, reser
            );
            diffs += 1;
        }
    }

    if diffs == 0 {
        println!(
            "\n=== PERFECT ROUND-TRIP: all {} bytes match ===",
            original_bytes.len()
        );
    } else {
        println!(
            "\n=== {} bytes differ out of {} ===",
            diffs,
            original_bytes.len()
        );
    }

    // Also try re-encoding as SysEx and compare the full message
    let rebuilt_sysex = program::build_current_program_dump(channel, &program);
    if raw_sysex == rebuilt_sysex {
        println!(
            "Full SysEx message also matches ({} bytes)",
            raw_sysex.len()
        );
    } else {
        println!(
            "Full SysEx message differs: original {} bytes, rebuilt {} bytes",
            raw_sysex.len(),
            rebuilt_sysex.len()
        );
    }

    Ok(())
}
