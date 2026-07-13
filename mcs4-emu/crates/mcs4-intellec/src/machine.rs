//! Phase-driven Intellec machine assembly.

use std::collections::BTreeMap;

use mcs4_periph::{Teletype33, TeletypeSignals, TeletypeTiming};
use mcs4_system::{Mcs40System, Mcs4System, PhaseTrace, System};

use crate::{
    console::{IntellecPanel, PanelDrive, PanelInput, PanelSnapshot, ResetScope},
    monitor_rom::{MonitorRom, MonitorRomError},
    profile::{IntellecProfile, SourceReference},
};

/// Minimal machine boundary required by an Intellec profile.
pub trait IntellecBusMachine: System {
    /// Advance exactly one observable MCS bus phase.
    fn step_intellec_phase(&mut self) -> PhaseTrace;

    /// Read one ROM I/O output latch.
    fn read_intellec_rom_port(&self, chip_id: u8) -> Option<u8>;

    /// Drive one ROM I/O input latch.
    fn write_intellec_rom_port_input(&mut self, chip_id: u8, value: u8);

    /// Read one RAM I/O output latch.
    fn read_intellec_ram_port(&self, bank_id: u8, chip_id: u8) -> Option<u8>;

    /// Drive the processor TEST input through the machine boundary.
    fn drive_intellec_test_pin(&mut self, high: bool);
}

impl IntellecBusMachine for Mcs4System {
    fn step_intellec_phase(&mut self) -> PhaseTrace {
        self.step_traced()
    }

    fn read_intellec_rom_port(&self, chip_id: u8) -> Option<u8> {
        self.read_rom_port(chip_id)
    }

    fn write_intellec_rom_port_input(&mut self, chip_id: u8, value: u8) {
        self.write_rom_port_input(chip_id, value);
    }

    fn read_intellec_ram_port(&self, bank_id: u8, chip_id: u8) -> Option<u8> {
        self.read_ram_port(bank_id, chip_id)
    }

    fn drive_intellec_test_pin(&mut self, high: bool) {
        self.set_test_pin(high);
    }
}

impl IntellecBusMachine for Mcs40System {
    fn step_intellec_phase(&mut self) -> PhaseTrace {
        self.step_traced()
    }

    fn read_intellec_rom_port(&self, chip_id: u8) -> Option<u8> {
        self.read_rom_port(chip_id)
    }

    fn write_intellec_rom_port_input(&mut self, chip_id: u8, value: u8) {
        self.write_rom_port_input(chip_id, value);
    }

    fn read_intellec_ram_port(&self, bank_id: u8, chip_id: u8) -> Option<u8> {
        self.read_ram_port(bank_id, chip_id)
    }

    fn drive_intellec_test_pin(&mut self, high: bool) {
        self.set_test_pin(high);
    }
}

/// A host-originated event accepted by the source-bound machine boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntellecEvent {
    /// Physical console action.
    Panel(PanelInput),
    /// One keyboard character entering the terminal serial transmitter.
    TerminalKey(u8),
    /// Replace terminal paper-reader input.
    ReaderTape(Vec<u8>),
    /// Enable or disable terminal-side punch capture.
    PunchEnabled(bool),
}

/// Immutable machine observation emitted after one completed phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntellecFrame {
    /// Immutable base-machine phase observation.
    pub phase: PhaseTrace,
    /// Immutable panel state.
    pub panel: PanelSnapshot,
    /// Terminal wire state after the completed phase.
    pub terminal: TeletypeSignals,
    /// Profile identity.
    pub profile_model: crate::profile::IntellecModel,
    /// Profile source references.
    pub sources: &'static [SourceReference],
}

/// A failure that preserves the machine state and its evidence boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntellecMachineError {
    /// The stopped panel does not authorize phase advancement.
    PanelStopped,
    /// Console-memory access requires a modeled memory-control card.
    ConsoleMemoryAccessUnavailable,
    /// The profile has not met its historical source gate.
    ProfileEvidence(crate::profile::ProfileEvidenceError),
    /// Monitor media does not match the selected profile.
    MonitorRom(MonitorRomError),
    /// The current machine does not expose a source-bound monitor slot map.
    MonitorSlotMapUnavailable,
    /// The selected terminal input ROM port is absent from the assembled machine.
    TerminalRomPortUnavailable {
        /// ROM chip identity.
        chip_id: u8,
    },
    /// The selected terminal output RAM port is absent from the assembled machine.
    TerminalRamPortUnavailable {
        /// Terminal wire driven by the absent endpoint.
        wire: &'static str,
        /// 4002 bank identity.
        bank_id: u8,
        /// 4002 chip identity inside the bank.
        chip_id: u8,
    },
    /// The selected terminal endpoint names a non-existent 4-bit port line.
    TerminalPortBitInvalid {
        /// Terminal wire that names the invalid bit.
        wire: &'static str,
        /// Requested 4-bit port line.
        bit: u8,
    },
}

/// Source-bound composition of one MCS machine, console, and ASR-33 terminal.
#[derive(Clone, Debug)]
pub struct IntellecMachine<M: IntellecBusMachine> {
    machine: M,
    profile: IntellecProfile,
    panel: IntellecPanel,
    terminal: Teletype33,
    terminal_signals: TeletypeSignals,
    terminal_input_latches: BTreeMap<u8, u8>,
}

impl<M: IntellecBusMachine> IntellecMachine<M> {
    /// Assemble one machine from an immutable profile.
    pub fn new(machine: M, profile: IntellecProfile) -> Self {
        let terminal = Teletype33::new(TeletypeTiming::asr33(profile.phase_ticks_per_second()));
        Self {
            machine,
            profile,
            panel: IntellecPanel::new(),
            terminal,
            terminal_signals: TeletypeSignals {
                terminal_to_machine: true,
                machine_to_terminal: true,
                reader_enabled: false,
            },
            terminal_input_latches: BTreeMap::new(),
        }
    }

    /// Return the immutable profile.
    pub const fn profile(&self) -> &IntellecProfile {
        &self.profile
    }

    /// Return the underlying MCS machine for read-only inspection.
    pub const fn machine(&self) -> &M {
        &self.machine
    }

    /// Return the rendered panel state.
    pub const fn panel_snapshot(&self) -> PanelSnapshot {
        self.panel.snapshot()
    }

    /// Return the terminal for rendered paper output inspection.
    pub const fn terminal(&self) -> &Teletype33 {
        &self.terminal
    }

    /// Drain rendered terminal printer characters.
    pub fn drain_printed_terminal_bytes(&mut self) -> Vec<u8> {
        self.terminal.drain_printed()
    }

    /// Drain rendered terminal paper-punch characters.
    pub fn drain_punched_terminal_bytes(&mut self) -> Vec<u8> {
        self.terminal.drain_punched()
    }

    /// Apply an external event without allowing host code to mutate CPU state.
    pub fn apply_event(&mut self, event: IntellecEvent) {
        match event {
            IntellecEvent::Panel(input) => self.panel.apply(input),
            IntellecEvent::TerminalKey(byte) => self.terminal.enqueue_keyboard(byte),
            IntellecEvent::ReaderTape(bytes) => self.terminal.load_reader_tape(bytes),
            IntellecEvent::PunchEnabled(enabled) => self.terminal.set_punch_enabled(enabled),
        }
    }

    /// Install monitor media only after the profile evidence gate passes.
    pub fn install_monitor_rom(&mut self, rom: &MonitorRom) -> Result<(), IntellecMachineError> {
        self.profile
            .validate_boot_evidence()
            .map_err(IntellecMachineError::ProfileEvidence)?;
        rom.validate_model(self.profile.model())
            .map_err(IntellecMachineError::MonitorRom)?;
        if rom.load_address() != 0 {
            return Err(IntellecMachineError::MonitorSlotMapUnavailable);
        }
        self.machine.load_rom(rom.bytes());
        Ok(())
    }

    /// Advance one phase through the panel, terminal, and MCS-machine boundary.
    pub fn step_phase(&mut self) -> Result<IntellecFrame, IntellecMachineError> {
        self.profile
            .validate_boot_evidence()
            .map_err(IntellecMachineError::ProfileEvidence)?;
        if !self.panel.runs_machine() {
            return Err(IntellecMachineError::PanelStopped);
        }
        self.validate_terminal_endpoints()?;

        let drive = self.panel.take_drive();
        self.apply_panel_drive(drive)?;
        self.apply_terminal_input();
        let phase = self.machine.step_intellec_phase();
        self.panel.observe_phase(&phase);
        self.advance_terminal()?;

        Ok(IntellecFrame {
            phase,
            panel: self.panel.snapshot(),
            terminal: self.terminal_signals,
            profile_model: self.profile.model(),
            sources: self.profile.sources(),
        })
    }

    fn apply_panel_drive(&mut self, drive: PanelDrive) -> Result<(), IntellecMachineError> {
        if let Some(scope) = drive.reset {
            self.machine.reset();
            if scope == ResetScope::System {
                self.terminal.reset();
            }
        }
        if let Some(test_hold) = drive.test_hold {
            self.machine.drive_intellec_test_pin(test_hold);
        }
        if drive.test_one_shot {
            self.machine.drive_intellec_test_pin(true);
        }
        if drive.console_memory_access.is_some() {
            return Err(IntellecMachineError::ConsoleMemoryAccessUnavailable);
        }
        Ok(())
    }

    fn apply_terminal_input(&mut self) {
        let Some(wiring) = self.profile.terminal_wiring() else {
            return;
        };
        let latch = self
            .terminal_input_latches
            .entry(wiring.terminal_input_rom_chip_id)
            .or_insert(0x0f);
        *latch = update_bit(
            *latch,
            wiring.terminal_to_machine_bit,
            self.terminal_signals.terminal_to_machine,
        );
        self.machine
            .write_intellec_rom_port_input(wiring.terminal_input_rom_chip_id, *latch);
    }

    fn validate_terminal_endpoints(&self) -> Result<(), IntellecMachineError> {
        let Some(wiring) = self.profile.terminal_wiring() else {
            return Ok(());
        };
        if wiring.terminal_to_machine_bit >= 4 {
            return Err(IntellecMachineError::TerminalPortBitInvalid {
                wire: "terminal-to-machine",
                bit: wiring.terminal_to_machine_bit,
            });
        }
        if wiring.printer_output.bit >= 4 {
            return Err(IntellecMachineError::TerminalPortBitInvalid {
                wire: "machine-to-terminal",
                bit: wiring.printer_output.bit,
            });
        }
        if wiring.reader_control.bit >= 4 {
            return Err(IntellecMachineError::TerminalPortBitInvalid {
                wire: "reader-enable",
                bit: wiring.reader_control.bit,
            });
        }
        if self
            .machine
            .read_intellec_rom_port(wiring.terminal_input_rom_chip_id)
            .is_none()
        {
            return Err(IntellecMachineError::TerminalRomPortUnavailable {
                chip_id: wiring.terminal_input_rom_chip_id,
            });
        }
        for (wire, endpoint) in [
            ("machine-to-terminal", wiring.printer_output),
            ("reader-enable", wiring.reader_control),
        ] {
            if self
                .machine
                .read_intellec_ram_port(endpoint.bank_id, endpoint.chip_id)
                .is_none()
            {
                return Err(IntellecMachineError::TerminalRamPortUnavailable {
                    wire,
                    bank_id: endpoint.bank_id,
                    chip_id: endpoint.chip_id,
                });
            }
        }
        Ok(())
    }

    fn advance_terminal(&mut self) -> Result<(), IntellecMachineError> {
        let Some(wiring) = self.profile.terminal_wiring() else {
            self.terminal_signals = self.terminal.advance_phase_ticks(1, true);
            return Ok(());
        };
        let printer_output = self
            .machine
            .read_intellec_ram_port(wiring.printer_output.bank_id, wiring.printer_output.chip_id)
            .ok_or(IntellecMachineError::TerminalRamPortUnavailable {
                wire: "machine-to-terminal",
                bank_id: wiring.printer_output.bank_id,
                chip_id: wiring.printer_output.chip_id,
            })?;
        let reader_output = self
            .machine
            .read_intellec_ram_port(wiring.reader_control.bank_id, wiring.reader_control.chip_id)
            .ok_or(IntellecMachineError::TerminalRamPortUnavailable {
                wire: "reader-enable",
                bank_id: wiring.reader_control.bank_id,
                chip_id: wiring.reader_control.chip_id,
            })?;
        let machine_to_terminal = bit_is_high(printer_output, wiring.printer_output.bit);
        self.terminal
            .set_reader_enabled(bit_is_high(reader_output, wiring.reader_control.bit));
        self.terminal_signals = self.terminal.advance_phase_ticks(1, machine_to_terminal);
        Ok(())
    }
}

fn bit_is_high(value: u8, bit: u8) -> bool {
    bit < 4 && (value & (1 << bit)) != 0
}

fn update_bit(value: u8, bit: u8, high: bool) -> u8 {
    if bit >= 4 {
        return value & 0x0f;
    }
    if high {
        value | (1 << bit)
    } else {
        value & !(1 << bit)
    }
}

#[cfg(test)]
mod tests {
    use mcs4_system::Mcs4System;

    use super::{IntellecEvent, IntellecMachine, IntellecMachineError};
    use crate::{
        console::{PanelControl, PanelInput},
        profile::{IntellecProfile, RamPortEndpoint, TerminalWiring},
    };

    fn bench_machine() -> IntellecMachine<Mcs4System> {
        IntellecMachine::new(
            Mcs4System::standard(),
            IntellecProfile::bench(
                750_000,
                TerminalWiring {
                    terminal_input_rom_chip_id: 0,
                    terminal_to_machine_bit: 0,
                    printer_output: RamPortEndpoint {
                        bank_id: 0,
                        chip_id: 0,
                        bit: 1,
                    },
                    reader_control: RamPortEndpoint {
                        bank_id: 0,
                        chip_id: 0,
                        bit: 2,
                    },
                },
            ),
        )
    }

    #[test]
    fn stopped_panel_blocks_phase_execution() {
        let mut machine = bench_machine();
        assert_eq!(machine.step_phase(), Err(IntellecMachineError::PanelStopped));
    }

    #[test]
    fn panel_run_emits_profile_bound_frames() {
        let mut machine = bench_machine();
        machine.apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));
        let frame = machine.step_phase().expect("bench machine phase");
        assert_eq!(frame.profile_model, crate::profile::IntellecModel::Bench);
        assert!(frame.sources.is_empty());
    }

    #[test]
    fn terminal_input_stays_inside_terminal_event_boundary() {
        let mut machine = bench_machine();
        machine.apply_event(IntellecEvent::TerminalKey(b'A'));
        machine.apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));
        for _ in 0..20 {
            machine.step_phase().expect("bench terminal phase");
        }
        assert!(machine.terminal().timing().ticks_per_second > 0);
    }

    #[test]
    fn terminal_outputs_follow_ram_ports_not_a_rom_output_port() {
        let mut system = Mcs4System::standard();
        system.ram[0].wmp(0x0);
        system.ram[1].wmp(0x0);

        let mut machine = IntellecMachine::new(
            system,
            IntellecProfile::bench(
                750_000,
                TerminalWiring {
                    terminal_input_rom_chip_id: 0,
                    terminal_to_machine_bit: 0,
                    printer_output: RamPortEndpoint {
                        bank_id: 0,
                        chip_id: 0,
                        bit: 0,
                    },
                    reader_control: RamPortEndpoint {
                        bank_id: 0,
                        chip_id: 1,
                        bit: 0,
                    },
                },
            ),
        );
        machine.apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));

        let frame = machine.step_phase().expect("bench terminal phase");
        assert!(!frame.terminal.machine_to_terminal);
        assert!(!frame.terminal.reader_enabled);
    }

    #[test]
    fn absent_terminal_ram_endpoint_stops_execution_without_idle_fallback() {
        let mut machine = IntellecMachine::new(
            Mcs4System::minimal(),
            IntellecProfile::bench(
                750_000,
                TerminalWiring {
                    terminal_input_rom_chip_id: 0,
                    terminal_to_machine_bit: 0,
                    printer_output: RamPortEndpoint {
                        bank_id: 0,
                        chip_id: 0,
                        bit: 0,
                    },
                    reader_control: RamPortEndpoint {
                        bank_id: 0,
                        chip_id: 1,
                        bit: 0,
                    },
                },
            ),
        );
        machine.apply_event(IntellecEvent::Panel(PanelInput::Control(PanelControl::Run)));

        assert_eq!(
            machine.step_phase(),
            Err(IntellecMachineError::TerminalRamPortUnavailable {
                wire: "reader-enable",
                bank_id: 0,
                chip_id: 1,
            })
        );
    }
}
