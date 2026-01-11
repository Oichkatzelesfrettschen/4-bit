#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use mcs4_bus::prelude::{BusCycle, IoOp};
use mcs4_gui::Mcs4App;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Mode {
    Gui,
    Fixture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum SystemKind {
    Mcs4,
    Mcs40,
}

#[derive(Parser, Debug)]
#[command(name = "mcs4-emu")]
struct Args {
    #[arg(long, value_enum, default_value_t = Mode::Gui)]
    mode: Mode,

    #[arg(long, value_enum, default_value_t = SystemKind::Mcs4)]
    system: SystemKind,

    /// Fixture name (e.g. `src_wrm_rdm`) or `.hex` filename.
    #[arg(long, default_value = "")]
    fixture: String,

    /// Machine cycles to execute when running a fixture.
    #[arg(long, default_value_t = 20)]
    cycles: usize,

    /// Optional ROM I/O input nibble (0-15) to set before running a fixture.
    /// Defaults to leaving inputs at their reset value.
    #[arg(long)]
    rom_io_input: Option<u8>,

    /// ROM chip ID (0-15) to apply `--rom-io-input` to.
    #[arg(long, default_value_t = 0)]
    rom_io_chip: u8,

    /// Fail the fixture run if any I/O control op appears in the wrong bus phase.
    #[arg(long, default_value_t = false)]
    strict_io_phases: bool,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mcs4-system/fixtures")
        .to_path_buf()
}

fn validate_io_phase(phase: BusCycle, io_op: Option<IoOp>) {
    if phase == BusCycle::X1 {
        assert_eq!(io_op, None, "io_op must be deasserted in X1");
        return;
    }

    let Some(op) = io_op else {
        return;
    };

    match op {
        IoOp::Src => assert!(
            matches!(phase, BusCycle::X2 | BusCycle::X3),
            "SRC must assert io_op only in X2/X3 (got {phase:?})"
        ),
        IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RomPortWrite | IoOp::RamStatusWrite(_) => {
            assert_eq!(phase, BusCycle::X2, "write io_op must occur in X2");
        }
        IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_) => {
            assert_eq!(phase, BusCycle::X3, "read io_op must occur in X3");
        }
    }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    match args.mode {
        Mode::Gui => run_gui(),
        Mode::Fixture => run_fixture(args),
    }
}

fn run_gui() -> eframe::Result<()> {
    use eframe::egui;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("MCS-4 Emulator"),
        ..Default::default()
    };

    eframe::run_native("mcs4-emu", options, Box::new(|cc| Ok(Box::new(Mcs4App::new(cc)))))
}

fn run_fixture(args: Args) -> eframe::Result<()> {
    use mcs4_system::{mcs4::Mcs4System, mcs40::Mcs40System};

    if args.fixture.trim().is_empty() {
        eprintln!("--fixture is required for --mode fixture");
        return Ok(());
    }

    let mut name = args.fixture.trim().to_string();
    if !name.ends_with(".hex") {
        name.push_str(".hex");
    }
    let path = fixtures_dir().join(name);

    match args.system {
        SystemKind::Mcs4 => {
            let mut sys = Mcs4System::minimal();
            if let Some(val) = args.rom_io_input {
                let chip = args.rom_io_chip & 0x0F;
                if let Some(rom) = sys.rom.iter_mut().find(|r| r.chip_id() == chip) {
                    rom.set_io_input(val & 0x0F);
                }
            }
            if let Err(e) = sys.load_rom_hex_file(&path) {
                eprintln!("failed to load fixture {}: {e}", path.display());
                return Ok(());
            }
            for _ in 0..(args.cycles * 8) {
                let phase = sys.phase();
                sys.step();
                if args.strict_io_phases {
                    validate_io_phase(phase, sys.control.io_op);
                }
            }
            println!(
                "system=mcs4 cycles={} pc=0x{:03X} acc=0x{:X}",
                args.cycles,
                sys.pc(),
                sys.accumulator()
            );
        }
        SystemKind::Mcs40 => {
            let mut sys = Mcs40System::new();
            if let Some(val) = args.rom_io_input {
                let chip = args.rom_io_chip & 0x0F;
                if let Some(rom) = sys.rom.iter_mut().find(|r| r.chip_id() == chip) {
                    rom.set_io_input(val & 0x0F);
                }
            }
            if let Err(e) = sys.load_rom_hex_file(&path) {
                eprintln!("failed to load fixture {}: {e}", path.display());
                return Ok(());
            }
            for _ in 0..(args.cycles * 8) {
                let phase = sys.phase();
                sys.step();
                if args.strict_io_phases {
                    validate_io_phase(phase, sys.control.io_op);
                }
            }
            println!(
                "system=mcs40 cycles={} pc=0x{:03X} acc=0x{:X}",
                args.cycles,
                sys.pc(),
                sys.cpu.alu.accumulator()
            );
        }
    }

    Ok(())
}
