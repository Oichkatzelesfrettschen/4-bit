//! Main GUI application state.

use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use eframe::egui;
use mcs4_system::{parse_hex_bytes, TraceFrame};

use crate::{
    panels::{
        die_viewer::DieViewerPanel,
        disasm::DisasmPanel,
        intellec::{IntellecWorkspace, Mod40EvidenceWorkspace},
        memory::{MemoryPanel, MemoryRegion},
        provenance::ProvenancePanel,
        registers::{CpuMode, RegisterPanel},
        seven_seg::SevenSegPanel,
        stack::StackPanel,
        waveform::WaveformPanel,
    },
    session::{MachineSnapshot, SimulationCommand, SimulationEvent, SimulationSession},
    signal_trace::{SignalTrace, MAX_FRAMES},
};

/// Number of phases executed by one bounded Run request.
const RUN_BATCH_PHASES: usize = 256;
const MAX_IMPORTED_TRACE_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMPORTED_TRACE_LINE_BYTES: usize = 64 * 1024;
/// 4001 ROM chip span the disassembler and memory panel display.
const ROM_IMAGE_BYTES: usize = 256;

/// A selectable bundled program.
struct Scenario {
    /// Menu label.
    name: &'static str,
    /// Whitespace-separated hex bytes of the ROM program.
    hex: &'static str,
}

/// Bundled MCS-4 programs the operator can load at runtime. Each is a validated
/// fixture under `mcs4-system/fixtures`; the first boots by default so the
/// register, RAM, and waveform panels show real activity on first launch instead
/// of an inert zero-ROM machine.
const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "RAM roundtrip (SRC/WRM/RDM)",
        hex: include_str!("../../mcs4-system/fixtures/src_wrm_rdm.hex"),
    },
    Scenario {
        name: "RAM status write/read",
        hex: include_str!("../../mcs4-system/fixtures/ram_status_wr1_rd1.hex"),
    },
    Scenario {
        name: "ROM port write/read",
        hex: include_str!("../../mcs4-system/fixtures/rom_port_wrr_rdr.hex"),
    },
    Scenario {
        name: "7-segment counter",
        hex: include_str!("../../mcs4-system/fixtures/seven_seg_count.hex"),
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
enum TraceMode {
    Behavioral,
    Imported { source: String },
}

/// Interactive application with one simulation owner and one UI-owned trace.
pub struct Mcs4App {
    simulation: SimulationSession,
    trace: SignalTrace,
    waveform_panel: WaveformPanel,
    disasm_panel: DisasmPanel,
    provenance_panel: ProvenancePanel,
    die_panel: DieViewerPanel,
    register_panel: RegisterPanel,
    stack_panel: StackPanel,
    memory_panel: MemoryPanel,
    seven_seg_panel: SevenSegPanel,
    intellec_workspace: IntellecWorkspace,
    mod40_evidence_workspace: Mod40EvidenceWorkspace,
    running: bool,
    run_request_pending: bool,
    latest_frame: Option<TraceFrame>,
    last_snapshot: Option<MachineSnapshot>,
    shown_memory_region: Option<MemoryRegion>,
    last_fault: Option<String>,
    rom_data: Vec<u8>,
    selected_scenario: usize,
    trace_mode: TraceMode,
}

impl Mcs4App {
    /// Construct the UI and initialize its behavioral worker with a zeroed ROM image.
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_trace_frames(creation_context, None)
    }

    /// Construct the UI and optionally load a read-only shared-trace JSONL file.
    pub fn new_with_trace_frames(_creation_context: &eframe::CreationContext<'_>, trace_frames: Option<&Path>) -> Self {
        let simulation = SimulationSession::spawn();
        let rom_data = rom_image_from_hex(SCENARIOS[0].hex);
        let mut app = Self {
            simulation,
            trace: SignalTrace::new(),
            waveform_panel: WaveformPanel::new(),
            disasm_panel: DisasmPanel::new(),
            provenance_panel: ProvenancePanel,
            die_panel: DieViewerPanel::new(),
            register_panel: RegisterPanel::new(CpuMode::I4004),
            stack_panel: StackPanel::new(CpuMode::I4004),
            memory_panel: MemoryPanel::new(),
            seven_seg_panel: SevenSegPanel,
            intellec_workspace: IntellecWorkspace::new(),
            mod40_evidence_workspace: Mod40EvidenceWorkspace::new(),
            running: false,
            run_request_pending: false,
            latest_frame: None,
            last_snapshot: None,
            shown_memory_region: None,
            last_fault: None,
            rom_data,
            selected_scenario: 0,
            trace_mode: TraceMode::Behavioral,
        };
        app.send_command(SimulationCommand::LoadRom {
            bytes: app.rom_data.clone(),
        });
        app.disasm_panel.update(&app.rom_data, 0);
        if let Some(path) = trace_frames {
            if let Err(error) = app.import_trace_frames(path) {
                app.last_fault = Some(error);
            }
        }
        app
    }

    fn send_command(&mut self, command: SimulationCommand) {
        if let Err(error) = self.simulation.send(command) {
            self.running = false;
            self.run_request_pending = false;
            self.last_fault = Some(error.to_string());
        }
    }

    fn drain_simulation_events(&mut self) {
        for event in self.simulation.drain_events() {
            match event {
                SimulationEvent::Frame(frame) if self.trace_mode == TraceMode::Behavioral => {
                    match self.trace.push_frame(frame.clone()) {
                        Ok(()) => {
                            if let Some(phase) = frame.phase.as_ref() {
                                self.disasm_panel.update(&self.rom_data, phase.pc);
                            }
                            self.latest_frame = Some(frame);
                        }
                        Err(error) => self.last_fault = Some(format!("trace frame rejected: {error}")),
                    }
                }
                SimulationEvent::Frame(_) => {}
                SimulationEvent::Snapshot(snapshot) => {
                    self.register_panel.update(snapshot.cpu.clone());
                    self.stack_panel.update(snapshot.stack.clone());
                    self.last_snapshot = Some(snapshot);
                    self.feed_memory_panel();
                }
                SimulationEvent::IntellecConsole(snapshot) => {
                    self.intellec_workspace.set_console(snapshot);
                }
                SimulationEvent::RunBoundary { reason, .. } => {
                    self.trace.clear();
                    self.latest_frame = None;
                    self.waveform_panel.cursors.clear_time_cursor();
                    self.waveform_panel.cursors.clear_markers();
                    self.last_fault = None;
                    if reason != "reset" {
                        self.last_fault = Some(format!("unexpected run boundary: {reason}"));
                    }
                }
                SimulationEvent::BatchComplete => self.run_request_pending = false,
                SimulationEvent::Fault { message } => {
                    self.running = false;
                    self.run_request_pending = false;
                    self.last_fault = Some(message);
                }
            }
        }
    }

    /// Feed the memory panel the region it currently shows from the latest
    /// worker snapshot, so ROM/RAM toggling reads the one machine on demand.
    fn feed_memory_panel(&mut self) {
        let Some(snapshot) = self.last_snapshot.as_ref() else {
            return;
        };
        let region = self.memory_panel.selected_region();
        let view = match region {
            MemoryRegion::Rom => snapshot.rom.clone(),
            MemoryRegion::Ram => snapshot.ram.clone(),
        };
        self.memory_panel.update(view);
        self.shown_memory_region = Some(region);
    }

    /// Load a bundled scenario into the shared machine and restart it at address
    /// zero, returning the live view to the behavioral worker.
    fn load_scenario(&mut self, index: usize) {
        let Some(scenario) = SCENARIOS.get(index) else {
            return;
        };
        self.selected_scenario = index;
        self.rom_data = rom_image_from_hex(scenario.hex);
        self.running = false;
        self.run_request_pending = false;
        self.trace_mode = TraceMode::Behavioral;
        self.send_command(SimulationCommand::LoadRom {
            bytes: self.rom_data.clone(),
        });
        self.send_command(SimulationCommand::Reset);
        self.disasm_panel.update(&self.rom_data, 0);
    }

    fn queue_run_batch_if_needed(&mut self) {
        if self.trace_mode == TraceMode::Behavioral && self.running && !self.run_request_pending {
            self.run_request_pending = true;
            self.send_command(SimulationCommand::RunPhases {
                phases: RUN_BATCH_PHASES,
            });
        }
    }

    fn show_controls(&mut self, ui: &mut egui::Ui) {
        let imported = matches!(self.trace_mode, TraceMode::Imported { .. });
        let mut chosen_scenario = self.selected_scenario;
        ui.horizontal(|ui| {
            ui.label("Scenario");
            egui::ComboBox::from_id_salt("scenario_selector")
                .selected_text(SCENARIOS[self.selected_scenario].name)
                .show_ui(ui, |ui| {
                    for (index, scenario) in SCENARIOS.iter().enumerate() {
                        ui.selectable_value(&mut chosen_scenario, index, scenario.name);
                    }
                });
        });
        if chosen_scenario != self.selected_scenario {
            self.load_scenario(chosen_scenario);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!imported, egui::Button::new(if self.running { "Stop" } else { "Run" }))
                .clicked()
            {
                self.running = !self.running;
            }
            if ui
                .add_enabled(
                    !imported && !self.running && !self.run_request_pending,
                    egui::Button::new("Step"),
                )
                .clicked()
            {
                self.send_command(SimulationCommand::StepPhase);
            }
            if ui.button("Reset").clicked() {
                self.running = false;
                self.run_request_pending = false;
                self.trace_mode = TraceMode::Behavioral;
                self.send_command(SimulationCommand::Reset);
            }
            if self.run_request_pending {
                ui.label("running bounded phase batch");
            }
            if let Some(frame) = self.latest_frame.as_ref() {
                ui.monospace(format!("run {} frame {}", frame.run_id, frame.sequence));
            }
        });
        if let TraceMode::Imported { source } = &self.trace_mode {
            ui.label(format!(
                "Imported trace: {source}. Reset returns to the behavioral worker."
            ));
        }
        if let Some(fault) = self.last_fault.as_deref() {
            ui.colored_label(egui::Color32::RED, fault);
        }
    }

    fn import_trace_frames(&mut self, path: &Path) -> Result<(), String> {
        let (trace, latest_frame) = load_trace_frames_jsonl(path)?;
        self.trace = trace;
        self.waveform_panel.cursors.clear_time_cursor();
        self.waveform_panel.cursors.clear_markers();
        self.latest_frame = Some(latest_frame);
        self.running = false;
        self.run_request_pending = false;
        self.trace_mode = TraceMode::Imported {
            source: path.display().to_string(),
        };
        Ok(())
    }
}

/// Build a 256-byte ROM image from a scenario's hex program at address zero,
/// leaving the remaining bytes as NOP.
fn rom_image_from_hex(hex: &str) -> Vec<u8> {
    let program = parse_hex_bytes(hex).expect("embedded scenario fixture parses");
    let mut image = vec![0u8; ROM_IMAGE_BYTES];
    let length = program.len().min(image.len());
    image[..length].copy_from_slice(&program[..length]);
    image
}

fn load_trace_frames_jsonl(path: &Path) -> Result<(SignalTrace, TraceFrame), String> {
    let metadata = fs::metadata(path).map_err(|error| format!("stat trace frames {}: {error}", path.display()))?;
    if metadata.len() > MAX_IMPORTED_TRACE_BYTES as u64 {
        return Err(format!(
            "trace frame input {} exceeds the {} byte import limit",
            path.display(),
            MAX_IMPORTED_TRACE_BYTES
        ));
    }
    let file = File::open(path).map_err(|error| format!("read trace frames {}: {error}", path.display()))?;
    parse_trace_frames_jsonl(BufReader::new(file))
}

fn parse_trace_frames_jsonl<R: BufRead>(mut reader: R) -> Result<(SignalTrace, TraceFrame), String> {
    let mut trace = SignalTrace::new();
    let mut latest_frame = None;
    let mut previous = None;
    let mut line = Vec::new();
    let mut total_bytes = 0usize;
    let mut line_number = 0usize;
    while let Some(bytes_read) = read_bounded_line(&mut reader, &mut line)
        .map_err(|error| format!("read trace frame line {}: {error}", line_number + 1))?
    {
        line_number += 1;
        total_bytes = total_bytes.saturating_add(bytes_read);
        if total_bytes > MAX_IMPORTED_TRACE_BYTES {
            return Err(format!(
                "trace frame input exceeds the {} byte import limit",
                MAX_IMPORTED_TRACE_BYTES
            ));
        }
        let line = std::str::from_utf8(&line)
            .map_err(|error| format!("trace frame line {line_number} is not UTF-8: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        if trace.len() >= MAX_FRAMES {
            return Err(format!("trace frame input exceeds the {MAX_FRAMES} frame import limit"));
        }
        let frame: TraceFrame =
            serde_json::from_str(line).map_err(|error| format!("trace frame line {line_number} JSON: {error}"))?;
        frame
            .validate()
            .map_err(|error| format!("trace frame line {line_number} contract: {error}"))?;
        if let Some((run_id, sequence)) = previous {
            if frame.run_id < run_id {
                return Err(format!(
                    "trace frame line {} regresses run identity from {} to {}",
                    line_number, run_id, frame.run_id
                ));
            }
            if frame.run_id == run_id && frame.sequence <= sequence {
                return Err(format!(
                    "trace frame line {} does not increase sequence within run {}",
                    line_number, frame.run_id
                ));
            }
        }
        previous = Some((frame.run_id, frame.sequence));
        trace
            .push_frame(frame.clone())
            .map_err(|error| format!("trace frame line {line_number} retention: {error}"))?;
        latest_frame = Some(frame);
    }
    let Some(latest_frame) = latest_frame else {
        return Err("trace frame input contains no JSONL frames".to_owned());
    };
    Ok((trace, latest_frame))
}

fn read_bounded_line<R: BufRead>(reader: &mut R, line: &mut Vec<u8>) -> std::io::Result<Option<usize>> {
    line.clear();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            return Ok(Some(line.len()));
        }
        let chunk_length = buffer
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(buffer.len(), |index| index + 1);
        if line.len().saturating_add(chunk_length) > MAX_IMPORTED_TRACE_LINE_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("line exceeds the {MAX_IMPORTED_TRACE_LINE_BYTES} byte import limit"),
            ));
        }
        line.extend_from_slice(&buffer[..chunk_length]);
        reader.consume(chunk_length);
        if line.last() == Some(&b'\n') {
            return Ok(Some(line.len()));
        }
    }
}

impl eframe::App for Mcs4App {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_simulation_events();
        self.queue_run_batch_if_needed();

        egui::TopBottomPanel::top("top_panel").show(context, |ui| self.show_controls(ui));

        egui::SidePanel::right("state_panel")
            .min_width(320.0)
            .show(context, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.register_panel.show(ui);
                    ui.separator();
                    self.stack_panel.show(ui);
                    ui.separator();
                    self.disasm_panel.show(ui);
                });
            });

        egui::SidePanel::left("intellec_panel")
            .min_width(340.0)
            .show(context, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.intellec_workspace.show(ui, &self.simulation);
                    ui.separator();
                    self.mod40_evidence_workspace.show(ui);
                });
            });

        egui::CentralPanel::default().show(context, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.waveform_panel.show(ui, &self.trace);
                ui.add_space(16.0);
                if Some(self.memory_panel.selected_region()) != self.shown_memory_region {
                    self.feed_memory_panel();
                }
                self.memory_panel.show(ui);
                ui.add_space(16.0);
                if let Some(snapshot) = self.last_snapshot.as_ref() {
                    self.seven_seg_panel.show(ui, &snapshot.seven_seg);
                    ui.add_space(16.0);
                }
                self.provenance_panel.show(ui, self.latest_frame.as_ref());
                ui.add_space(16.0);
                self.die_panel.ui(ui, self.latest_frame.as_ref());
            });
        });

        if self.running || self.run_request_pending {
            context.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{parse_trace_frames_jsonl, MAX_IMPORTED_TRACE_LINE_BYTES};

    const SYSTEM_VERILATOR_FRAME: &str =
        include_str!("../../mcs4-system/fixtures/traces/mcs4-system-verilator-frame-v1.jsonl");

    #[test]
    fn bundled_scenarios_embed_runnable_programs() {
        for scenario in super::SCENARIOS {
            let image = super::rom_image_from_hex(scenario.hex);
            assert_eq!(image.len(), super::ROM_IMAGE_BYTES);
            assert!(
                image.iter().any(|&byte| byte != 0),
                "scenario '{}' embeds a non-empty program",
                scenario.name
            );
        }
        // The default scenario opens with LDM 0xA (0xDA) then FIM P0, 0x01 (0x20 0x01).
        assert_eq!(
            &super::rom_image_from_hex(super::SCENARIOS[0].hex)[..3],
            &[0xDA, 0x20, 0x01]
        );
    }

    #[test]
    fn imported_system_adapter_frame_preserves_provenance() {
        let (trace, latest_frame) =
            parse_trace_frames_jsonl(Cursor::new(SYSTEM_VERILATOR_FRAME)).expect("parse system adapter frame");
        assert_eq!(trace.len(), 1);
        assert_eq!(latest_frame.provenance.model_id, "mcs4-system-fpga-verilator");
    }

    #[test]
    fn importer_rejects_nonincreasing_sequence_within_one_run() {
        let duplicate = format!("{0}\n{0}\n", SYSTEM_VERILATOR_FRAME.trim());
        let error = match parse_trace_frames_jsonl(Cursor::new(duplicate)) {
            Ok(_) => panic!("duplicate sequence must fail"),
            Err(error) => error,
        };
        assert!(error.contains("does not increase sequence"));
    }

    #[test]
    fn importer_rejects_regressing_run_identity() {
        let previous_run = SYSTEM_VERILATOR_FRAME.trim();
        let regressing_run = previous_run.replacen("\"run_id\":2", "\"run_id\":1", 1);
        let input = format!("{previous_run}\n{regressing_run}\n");
        let error = match parse_trace_frames_jsonl(Cursor::new(input)) {
            Ok(_) => panic!("run identity must be monotonic"),
            Err(error) => error,
        };
        assert!(error.contains("regresses run identity"));
    }

    #[test]
    fn importer_rejects_a_line_larger_than_the_bounded_reader_limit() {
        let oversized = format!("{}\n", "x".repeat(MAX_IMPORTED_TRACE_LINE_BYTES));
        let error = match parse_trace_frames_jsonl(Cursor::new(oversized)) {
            Ok(_) => panic!("oversized line must fail"),
            Err(error) => error,
        };
        assert!(error.contains("line exceeds"));
    }
}
