//! MCS-4/MCS-40 GUI Emulator

use clap::Parser;
use eframe::egui;

#[derive(Parser)]
#[command(name = "mcs4-emu")]
#[command(about = "Intel MCS-4/MCS-40 Emulator")]
struct Args {
    /// ROM file to load
    #[arg(short, long)]
    rom: Option<String>,

    /// System type (mcs4 or mcs40)
    #[arg(short, long, default_value = "mcs4")]
    system: String,
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("MCS-4/MCS-40 Emulator"),
        ..Default::default()
    };

    eframe::run_native(
        "MCS-4 Emulator",
        options,
        Box::new(|cc| Ok(Box::new(EmulatorApp::new(cc, args)))),
    )
}

struct EmulatorApp {
    system: mcs4_system::Mcs4System,
    running: bool,
}

impl EmulatorApp {
    fn new(_cc: &eframe::CreationContext<'_>, args: Args) -> Self {
        let mut system = mcs4_system::Mcs4System::minimal();

        if let Some(rom_path) = args.rom {
            if let Ok(data) = std::fs::read(&rom_path) {
                system.load_rom(&data);
                tracing::info!("Loaded ROM: {}", rom_path);
            }
        }

        Self {
            system,
            running: false,
        }
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.running { "Stop" } else { "Run" }).clicked() {
                    self.running = !self.running;
                }
                if ui.button("Step").clicked() {
                    self.system.step();
                }
                if ui.button("Reset").clicked() {
                    self.system = mcs4_system::Mcs4System::minimal();
                }
            });
        });

        egui::SidePanel::left("registers").show(ctx, |ui| {
            ui.heading("CPU State");
            ui.label(format!("PC: {:03X}", self.system.cpu.pc()));
            ui.label(format!("ACC: {:X}", self.system.cpu.accumulator()));
            ui.label(format!("CY: {}", self.system.cpu.carry() as u8));

            ui.separator();
            ui.heading("Index Registers");
            // TODO: Display all 16 registers
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Memory View");
            // TODO: Memory display
            ui.label("ROM/RAM viewer (TODO)");

            ui.separator();
            ui.heading("Waveform");
            // TODO: Waveform display
            ui.label("Signal waveforms (TODO)");
        });

        if self.running {
            self.system.step();
            ctx.request_repaint();
        }
    }
}
