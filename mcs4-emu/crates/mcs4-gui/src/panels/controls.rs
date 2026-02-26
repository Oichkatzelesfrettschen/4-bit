//! Execution controls panel: Run/Stop/Step/Reset, speed, CPU mode selection

use eframe::egui::{self, Color32};

use super::registers::CpuMode;

/// Execution state of the emulator
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Stopped,
    Running,
    /// Halted by breakpoint (stores the breakpoint ID that triggered)
    Halted(u32),
}

impl std::fmt::Display for RunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunState::Stopped => write!(f, "STOPPED"),
            RunState::Running => write!(f, "RUNNING"),
            RunState::Halted(id) => write!(f, "HALTED (BP #{})", id),
        }
    }
}

/// Speed presets (instructions per frame)
const SPEED_PRESETS: &[(f32, &str)] = &[
    (1.0, "1 Hz"),
    (10.0, "10 Hz"),
    (100.0, "100 Hz"),
    (1_000.0, "1 kHz"),
    (10_000.0, "10 kHz"),
    (100_000.0, "100 kHz"),
    (740_000.0, "740 kHz (real)"),
];

/// Execution controls panel
pub struct ControlsPanel {
    run_state: RunState,
    cpu_mode: CpuMode,
    /// Execution speed in instructions per second
    speed: f32,
    /// Cycle counter
    cycles: u64,
    /// Instructions executed
    instructions: u64,
}

impl ControlsPanel {
    pub fn new() -> Self {
        Self {
            run_state: RunState::Stopped,
            cpu_mode: CpuMode::I4004,
            speed: 1_000.0,
            cycles: 0,
            instructions: 0,
        }
    }

    pub fn run_state(&self) -> RunState {
        self.run_state
    }

    pub fn set_run_state(&mut self, state: RunState) {
        self.run_state = state;
    }

    pub fn cpu_mode(&self) -> CpuMode {
        self.cpu_mode
    }

    pub fn set_cpu_mode(&mut self, mode: CpuMode) {
        self.cpu_mode = mode;
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(1.0, 1_000_000.0);
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    pub fn instructions(&self) -> u64 {
        self.instructions
    }

    pub fn update_counters(&mut self, cycles: u64, instructions: u64) {
        self.cycles = cycles;
        self.instructions = instructions;
    }

    pub fn reset_counters(&mut self) {
        self.cycles = 0;
        self.instructions = 0;
    }

    /// Returns true if a Run/Stop/Step/Reset button was clicked.
    /// The caller should query run_state() after show() to determine what action to take.
    pub fn show(&mut self, ui: &mut egui::Ui) -> ControlAction {
        let mut action = ControlAction::None;

        ui.heading("Controls");
        ui.separator();

        // Run state indicator
        let state_color = match self.run_state {
            RunState::Stopped => Color32::GRAY,
            RunState::Running => Color32::GREEN,
            RunState::Halted(_) => Color32::RED,
        };
        ui.label(
            egui::RichText::new(format!("{}", self.run_state))
                .color(state_color)
                .strong(),
        );

        ui.separator();

        // Execution buttons
        ui.horizontal(|ui| {
            let is_running = self.run_state == RunState::Running;
            if ui.button(if is_running { "Stop" } else { "Run" }).clicked() {
                if is_running {
                    self.run_state = RunState::Stopped;
                    action = ControlAction::Stop;
                } else {
                    self.run_state = RunState::Running;
                    action = ControlAction::Run;
                }
            }
            if ui.button("Step").clicked() {
                self.run_state = RunState::Stopped;
                action = ControlAction::Step;
            }
            if ui.button("Reset").clicked() {
                self.run_state = RunState::Stopped;
                action = ControlAction::Reset;
            }
        });

        ui.separator();

        // Speed slider
        ui.horizontal(|ui| {
            ui.label("Speed:");
            let speed_log = self.speed.log10();
            let mut log_val = speed_log;
            ui.add(egui::Slider::new(&mut log_val, 0.0..=6.0).text("Hz (log)"));
            if (log_val - speed_log).abs() > f32::EPSILON {
                self.speed = 10.0f32.powf(log_val);
            }
        });

        // Speed presets
        ui.horizontal(|ui| {
            for &(spd, label) in SPEED_PRESETS {
                if ui.selectable_label((self.speed - spd).abs() < 0.5, label).clicked() {
                    self.speed = spd;
                }
            }
        });

        ui.separator();

        // CPU mode selector
        ui.horizontal(|ui| {
            ui.label("CPU:");
            if ui.selectable_label(self.cpu_mode == CpuMode::I4004, "4004").clicked() {
                self.cpu_mode = CpuMode::I4004;
            }
            if ui.selectable_label(self.cpu_mode == CpuMode::I4040, "4040").clicked() {
                self.cpu_mode = CpuMode::I4040;
            }
        });

        ui.separator();

        // Counters
        ui.label(egui::RichText::new(format!("Cycles: {}", self.cycles)).color(Color32::GRAY));
        ui.label(egui::RichText::new(format!("Instructions: {}", self.instructions)).color(Color32::GRAY));

        action
    }
}

/// Actions emitted by the controls panel
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    None,
    Run,
    Stop,
    Step,
    Reset,
}

impl Default for ControlsPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_panel_defaults() {
        let panel = ControlsPanel::new();
        assert_eq!(panel.run_state(), RunState::Stopped);
        assert_eq!(panel.cpu_mode(), CpuMode::I4004);
        assert_eq!(panel.cycles(), 0);
        assert_eq!(panel.instructions(), 0);
    }

    #[test]
    fn set_run_state() {
        let mut panel = ControlsPanel::new();
        panel.set_run_state(RunState::Running);
        assert_eq!(panel.run_state(), RunState::Running);

        panel.set_run_state(RunState::Halted(5));
        assert_eq!(panel.run_state(), RunState::Halted(5));
    }

    #[test]
    fn set_cpu_mode() {
        let mut panel = ControlsPanel::new();
        panel.set_cpu_mode(CpuMode::I4040);
        assert_eq!(panel.cpu_mode(), CpuMode::I4040);
    }

    #[test]
    fn speed_clamped() {
        let mut panel = ControlsPanel::new();
        panel.set_speed(0.001);
        assert!((panel.speed() - 1.0).abs() < f32::EPSILON);

        panel.set_speed(999_999_999.0);
        assert!((panel.speed() - 1_000_000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn update_counters() {
        let mut panel = ControlsPanel::new();
        panel.update_counters(1000, 250);
        assert_eq!(panel.cycles(), 1000);
        assert_eq!(panel.instructions(), 250);
    }

    #[test]
    fn reset_counters() {
        let mut panel = ControlsPanel::new();
        panel.update_counters(500, 100);
        panel.reset_counters();
        assert_eq!(panel.cycles(), 0);
        assert_eq!(panel.instructions(), 0);
    }

    #[test]
    fn run_state_display() {
        assert_eq!(format!("{}", RunState::Stopped), "STOPPED");
        assert_eq!(format!("{}", RunState::Running), "RUNNING");
        assert_eq!(format!("{}", RunState::Halted(3)), "HALTED (BP #3)");
    }

    #[test]
    fn default_is_new() {
        let panel = ControlsPanel::default();
        assert_eq!(panel.run_state(), RunState::Stopped);
        assert_eq!(panel.cpu_mode(), CpuMode::I4004);
    }
}
