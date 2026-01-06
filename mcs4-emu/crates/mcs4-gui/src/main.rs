#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
mod app;
use app::Mcs4App;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Re-export modules from lib.rs
use mcs4_gui::{panels, signal_trace};

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("MCS-4 Emulator"),
        ..Default::default()
    };

    eframe::run_native("mcs4-emu", options, Box::new(|cc| Ok(Box::new(Mcs4App::new(cc)))))
}
