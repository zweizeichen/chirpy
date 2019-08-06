mod bin;
mod periphery;
mod system;

use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

fn main() {
    // Initialize new system
    let mut system = system::System::default();

    // Parse arguments
    let mut args = env::args_os();

    if args.len() != 2 {
        panic!("Please supply the path to a valid ROM as first argument.")
    }

    // Load ROM from disk and put it into memory
    let path = args.nth(1).unwrap();
    let file = File::open(path).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut reader = BufReader::new(file);
    let mut buffer: Vec<u8> = vec![];
    reader.read_to_end(&mut buffer).unwrap();
    system.copy_buffer_to_memory(buffer, 0x200);

    // Run system
    system.run();
}
