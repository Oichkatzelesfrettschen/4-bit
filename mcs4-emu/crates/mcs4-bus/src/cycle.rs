//! Bus cycle and machine state definitions
//!
//! The MCS-4 uses an 8-phase machine cycle:
//! - A1, A2, A3: Address output phases (CPU sends 12-bit ROM address)
//! - M1, M2: Memory read phases (ROM outputs 8-bit instruction)
//! - X1, X2, X3: Execution phases (varies by instruction)

/// Bus cycle phase within a machine cycle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BusCycle {
    /// Address phase 1 - CPU outputs address bits 0-3
    A1 = 0,
    /// Address phase 2 - CPU outputs address bits 4-7
    A2 = 1,
    /// Address phase 3 - CPU outputs address bits 8-11
    A3 = 2,
    /// Memory read phase 1 - ROM outputs instruction bits 0-3 (OPA)
    M1 = 3,
    /// Memory read phase 2 - ROM outputs instruction bits 4-7 (OPR)
    M2 = 4,
    /// Execution phase 1
    X1 = 5,
    /// Execution phase 2
    X2 = 6,
    /// Execution phase 3
    X3 = 7,
}

impl BusCycle {
    /// Get the next phase in sequence
    pub fn next(self) -> Self {
        match self {
            BusCycle::A1 => BusCycle::A2,
            BusCycle::A2 => BusCycle::A3,
            BusCycle::A3 => BusCycle::M1,
            BusCycle::M1 => BusCycle::M2,
            BusCycle::M2 => BusCycle::X1,
            BusCycle::X1 => BusCycle::X2,
            BusCycle::X2 => BusCycle::X3,
            BusCycle::X3 => BusCycle::A1, // Wrap around
        }
    }

    /// Is this an address phase?
    pub fn is_address_phase(self) -> bool {
        matches!(self, BusCycle::A1 | BusCycle::A2 | BusCycle::A3)
    }

    /// Is this a memory read phase?
    pub fn is_memory_phase(self) -> bool {
        matches!(self, BusCycle::M1 | BusCycle::M2)
    }

    /// Is this an execution phase?
    pub fn is_execution_phase(self) -> bool {
        matches!(self, BusCycle::X1 | BusCycle::X2 | BusCycle::X3)
    }

    /// Phase number (0-7)
    pub fn phase_number(self) -> u8 {
        self as u8
    }
}

/// Higher-level machine state for multi-cycle instructions
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineState {
    /// Fetching first instruction byte (all instructions)
    Fetch1,
    /// Fetching second instruction byte (2-byte instructions only)
    Fetch2,
    /// Executing instruction
    Execute,
    /// Halted (4040 only)
    Halted,
    /// Interrupt acknowledge (4040 only)
    InterruptAck,
}

impl MachineState {
    /// Is the CPU fetching an instruction?
    pub fn is_fetching(self) -> bool {
        matches!(self, MachineState::Fetch1 | MachineState::Fetch2)
    }
}

/// Complete cycle state tracking
#[derive(Clone, Debug)]
pub struct CycleState {
    /// Current bus phase
    pub phase: BusCycle,

    /// Current machine state
    pub state: MachineState,

    /// Machine cycle count (8 phases = 1 cycle)
    pub cycle_count: u64,

    /// Instruction cycle count (1-2 machine cycles per instruction)
    pub instruction_count: u64,

    /// Is this a two-cycle instruction?
    pub two_cycle: bool,

    /// Second cycle of two-cycle instruction?
    pub second_cycle: bool,
}

impl CycleState {
    /// Create initial cycle state
    pub fn new() -> Self {
        Self {
            phase: BusCycle::A1,
            state: MachineState::Fetch1,
            cycle_count: 0,
            instruction_count: 0,
            two_cycle: false,
            second_cycle: false,
        }
    }

    /// Advance to next phase
    pub fn advance(&mut self) {
        let prev_phase = self.phase;
        self.phase = self.phase.next();

        // Count cycles
        if prev_phase == BusCycle::X3 {
            self.cycle_count += 1;

            // Update machine state
            match self.state {
                MachineState::Fetch1 => {
                    if self.two_cycle {
                        self.state = MachineState::Fetch2;
                        self.second_cycle = true;
                    } else {
                        self.instruction_count += 1;
                        self.two_cycle = false;
                    }
                }
                MachineState::Fetch2 => {
                    self.state = MachineState::Fetch1;
                    self.instruction_count += 1;
                    self.two_cycle = false;
                    self.second_cycle = false;
                }
                MachineState::Execute => {
                    self.state = MachineState::Fetch1;
                }
                MachineState::Halted | MachineState::InterruptAck => {
                    // Stay in this state until externally changed
                }
            }
        }
    }

    /// Mark current instruction as two-cycle
    pub fn set_two_cycle(&mut self) {
        self.two_cycle = true;
    }

    /// Enter halted state
    pub fn halt(&mut self) {
        self.state = MachineState::Halted;
    }

    /// Exit halted state
    pub fn resume(&mut self) {
        if self.state == MachineState::Halted {
            self.state = MachineState::Fetch1;
        }
    }

    /// Begin interrupt acknowledge sequence
    pub fn interrupt_ack(&mut self) {
        self.state = MachineState::InterruptAck;
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for CycleState {
    fn default() -> Self {
        Self::new()
    }
}

/// Cycle timing for signal assertions
pub mod cycle_timing {
    use super::BusCycle;

    /// When SYNC should be asserted (relative to cycle start)
    pub const SYNC_ASSERT: BusCycle = BusCycle::A1;

    /// When SYNC should be deasserted
    pub const SYNC_DEASSERT: BusCycle = BusCycle::A2;

    /// When CM-ROM should be valid
    pub const CM_ROM_VALID: BusCycle = BusCycle::A3;

    /// When CM-RAM should be valid
    pub const CM_RAM_VALID: BusCycle = BusCycle::X2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_sequence() {
        let mut phase = BusCycle::A1;

        // Walk through all 8 phases
        assert_eq!(phase, BusCycle::A1);
        phase = phase.next();
        assert_eq!(phase, BusCycle::A2);
        phase = phase.next();
        assert_eq!(phase, BusCycle::A3);
        phase = phase.next();
        assert_eq!(phase, BusCycle::M1);
        phase = phase.next();
        assert_eq!(phase, BusCycle::M2);
        phase = phase.next();
        assert_eq!(phase, BusCycle::X1);
        phase = phase.next();
        assert_eq!(phase, BusCycle::X2);
        phase = phase.next();
        assert_eq!(phase, BusCycle::X3);
        phase = phase.next();
        assert_eq!(phase, BusCycle::A1); // Wrap around
    }

    #[test]
    fn test_phase_classification() {
        assert!(BusCycle::A1.is_address_phase());
        assert!(BusCycle::A2.is_address_phase());
        assert!(BusCycle::A3.is_address_phase());

        assert!(BusCycle::M1.is_memory_phase());
        assert!(BusCycle::M2.is_memory_phase());

        assert!(BusCycle::X1.is_execution_phase());
        assert!(BusCycle::X2.is_execution_phase());
        assert!(BusCycle::X3.is_execution_phase());
    }

    #[test]
    fn test_cycle_state() {
        let mut state = CycleState::new();

        // Run through one complete cycle
        for _ in 0..8 {
            state.advance();
        }

        assert_eq!(state.cycle_count, 1);
        assert_eq!(state.instruction_count, 1);
    }

    #[test]
    fn test_two_cycle_instruction() {
        let mut state = CycleState::new();
        state.set_two_cycle();

        // First cycle
        for _ in 0..8 {
            state.advance();
        }
        assert_eq!(state.cycle_count, 1);
        assert_eq!(state.instruction_count, 0); // Not complete yet
        assert_eq!(state.state, MachineState::Fetch2);

        // Second cycle
        for _ in 0..8 {
            state.advance();
        }
        assert_eq!(state.cycle_count, 2);
        assert_eq!(state.instruction_count, 1); // Now complete
        assert_eq!(state.state, MachineState::Fetch1);
    }
}
