use std::path::PathBuf;

use mcs4_system::Mcs4System;

fn usage() -> ! {
    eprintln!("Usage: fixture_runner <path/to/fixture.hex> [cycles]");
    eprintln!("  cycles defaults to 20");
    std::process::exit(2);
}

fn main() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage();
    }
    let fixture = PathBuf::from(args.remove(0));
    let cycles: usize = if args.is_empty() {
        20
    } else {
        args.remove(0).parse().unwrap_or_else(|_| usage())
    };
    if !args.is_empty() {
        usage();
    }

    let mut sys = Mcs4System::minimal();
    sys.load_rom_hex_file(&fixture).expect("load fixture");
    sys.run_cycles(cycles);

    println!("fixture={}", fixture.display());
    println!("cycles={}", cycles);
    println!("pc={:#06x}", sys.pc());
    println!("acc={:#x}", sys.accumulator());
    println!("carry={}", sys.carry());
}
