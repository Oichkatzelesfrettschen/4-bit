//! Main application state

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use eframe::egui;
use mcs4_system::mcs4::Mcs4System;

use crate::{
    panels::{disasm::DisasmPanel, waveform::WaveformPanel},
    signal_trace::SignalTrace,
};

pub struct Mcs4App {
    // Emulator
    system: Arc<RwLock<Mcs4System>>,
    _trace: Arc<RwLock<SignalTrace>>,

    // UI Panels
    waveform_panel: WaveformPanel,
    disasm_panel: DisasmPanel,

    // Control
    running: Arc<AtomicBool>,
    rom_data: Vec<u8>,
}

impl Mcs4App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let system = Arc::new(RwLock::new(Mcs4System::minimal()));
        let trace = Arc::new(RwLock::new(SignalTrace::new()));
        let running = Arc::new(AtomicBool::new(false));

        // Spawn emulator thread
        let sys_clone = system.clone();
        let trace_clone = trace.clone();
        let running_clone = running.clone();

        thread::spawn(move || {
            loop {
                if !running_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                if let (Ok(mut sys), Ok(mut trace)) = (sys_clone.write(), trace_clone.write()) {
                    // Actually, let's step the system multiple times per loop
                    for _ in 0..100 {
                        // Capture before step
                        trace.push(sys.cycles(), &sys.bus, &sys.control, sys.phase(), &sys.clock);
                        sys.step();
                    }
                }
                thread::sleep(Duration::from_millis(1));
            }
        });

        Self {
            system,
            _trace: trace.clone(),
            waveform_panel: WaveformPanel::new(trace),
            disasm_panel: DisasmPanel::new(),
            running,
            rom_data: vec![0; 256],
        }
    }
}

impl eframe::App for Mcs4App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut is_running = self.running.load(Ordering::Relaxed);
                if ui.button(if is_running { "Stop" } else { "Run" }).clicked() {
                    is_running = !is_running;
                    self.running.store(is_running, Ordering::Relaxed);
                }
                if ui.button("Step").clicked() {
                    if let Ok(mut sys) = self.system.write() {
                        sys.step();
                        // Update disasm on step
                        self.disasm_panel.update(&self.rom_data, sys.pc());
                    }
                }
                if ui.button("Reset").clicked() {
                    if let Ok(mut sys) = self.system.write() {
                        sys.reset();
                        // Reload ROM
                        sys.load_rom(&self.rom_data);
                        self.disasm_panel.update(&self.rom_data, sys.pc());
                    }
                }
            });
        });

        egui::SidePanel::right("disasm_panel").min_width(300.0).show(ctx, |ui| {
            self.disasm_panel.show(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.waveform_panel.show(ui);
        });

        // Request repaint for smooth waveform updates if running
        if self.running.load(Ordering::Relaxed) {
            ctx.request_repaint();
        }
    }
}
