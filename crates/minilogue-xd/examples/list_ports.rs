//! List available MIDI ports and detect the Minilogue XD.

use minilogue_xd::device;

fn main() {
    match device::list_output_ports() {
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

    match device::find_output_port() {
        Ok(Some(port)) => println!("\nMinilogue XD found: {port}"),
        Ok(None) => println!("\nMinilogue XD not found."),
        Err(e) => eprintln!("Error searching for XD: {e}"),
    }
}
