/// List all available MIDI output ports.
use minilogue_xd::transport::MidirOutput;

fn main() {
    match MidirOutput::available_ports() {
        Ok(ports) => {
            println!("Available MIDI output ports:");
            for (i, name) in ports.iter().enumerate() {
                println!("  {i}: {name}");
            }
            if ports.is_empty() {
                println!("  (none found)");
            }
        }
        Err(e) => eprintln!("Error listing ports: {e}"),
    }
}
