//! Intel 4040 CPU Implementation
//!
//! The 4040 is a backward-compatible enhancement of the 4004 with:
//! - 24 index registers (vs 16) with bank switching
//! - 7-level stack (vs 3)
//! - 14 new instructions (60 total)
//! - Single-level interrupt support
//! - Halt mode

pub mod instruction_decode;
pub mod interrupt;
pub mod registers;

pub use instruction_decode::{decode_4040_specific, I4040Instruction, Instruction};
pub use interrupt::InterruptController;
// Note: 4040 uses 4004's ALU (no extensions needed)
use mcs4_bus::prelude::*;
pub use registers::Registers;

use crate::i4004;

/// Intel 4040 CPU (stub implementation)
///
/// Full implementation deferred - this provides type compatibility
pub struct I4040 {
    /// ALU (from 4004 base)
    pub alu: i4004::Alu,
    /// 4040 registers
    pub registers: Registers,
    /// 4004 registers (temporary for base compatibility)
    pub registers_4004: i4004::Registers,
    /// Interrupt controller
    pub intr: InterruptController,
    /// Instruction decoder
    pub decoder: i4004::InstructionDecoder,
    /// Halted state
    halted: bool,
    /// RAM bank selection (0 or 1)
    ram_bank: u8,
    /// Current cycle state
    cycle: CycleState,
    /// Fetched instruction byte (OPR:OPA)
    instruction_byte: u8,
    /// Second byte of two-byte instruction
    operand: u8,
    /// Currently selected RAM address (from SRC)
    ram_address: u8,
    /// Currently selected RAM chip
    ram_chip: u8,
    /// Test pin input
    test_pin: bool,
    /// Pending I/O data
    io_data: u8,
    /// Decoded I/O operation for current instruction
    decoded_io_op: Option<IoOp>,
    /// True if PC was explicitly modified by instruction
    pc_modified: bool,
}

impl I4040 {
    pub fn new() -> Self {
        Self {
            alu: i4004::Alu::new(),
            registers: Registers::new(),
            registers_4004: i4004::Registers::new(),
            intr: InterruptController::new(),
            decoder: i4004::InstructionDecoder::new(),
            halted: false,
            ram_bank: 0,
            cycle: CycleState::new(),
            instruction_byte: 0,
            operand: 0,
            ram_address: 0,
            ram_chip: 0,
            test_pin: false,
            io_data: 0,
            decoded_io_op: None,
            pc_modified: false,
        }
    }

    pub fn pc(&self) -> u16 {
        self.registers.pc()
    }

    pub fn accumulator(&self) -> u8 {
        self.alu.accumulator()
    }

    pub fn carry(&self) -> bool {
        self.alu.carry()
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn cycle_state(&self) -> &CycleState {
        &self.cycle
    }

    pub fn instruction_byte(&self) -> u8 {
        self.instruction_byte
    }

    pub fn tick(&mut self, phase: BusCycle, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        match phase {
            BusCycle::A1 => self.phase_a1(bus, ctrl),
            BusCycle::A2 => self.phase_a2(bus, ctrl),
            BusCycle::A3 => self.phase_a3(bus, ctrl),
            BusCycle::M1 => self.phase_m1(bus),
            BusCycle::M2 => self.phase_m2(bus),
            BusCycle::X1 => self.phase_x1(bus, ctrl),
            BusCycle::X2 => self.phase_x2(bus, ctrl),
            BusCycle::X3 => self.phase_x3(bus, ctrl),
        }
        self.cycle.advance();
    }

    // Compatibility methods from 4004
    pub fn set_test_pin(&mut self, state: bool) {
        self.test_pin = state;
    }

    pub fn test_pin(&self) -> bool {
        self.test_pin
    }

    pub fn ram_address(&self) -> u8 {
        self.ram_address
    }

    pub fn ram_chip(&self) -> u8 {
        self.ram_chip
    }

    pub fn ram_bank(&self) -> u8 {
        self.ram_bank
    }

    pub fn x3_cpu_drives_first(&self) -> bool {
        // SRC uses the bus in X3 for the second nibble (address), so CPU must drive first.
        // For SRC, the CPU drives the bus with the character address and the RAM latches it.
        matches!(self.decoded_io_op, Some(IoOp::Src))
    }

    pub fn x2_ram_bank_select(&self) -> bool {
        matches!(
            self.decoded_io_op,
            Some(IoOp::Src | IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RamStatusWrite(_))
        )
    }

    pub fn x3_ram_bank_select(&self) -> bool {
        matches!(
            self.decoded_io_op,
            Some(IoOp::Src | IoOp::RamMainRead | IoOp::RamStatusRead(_))
        )
    }

    pub fn x3_peripheral_io_op(&self) -> Option<IoOp> {
        match self.decoded_io_op {
            Some(op @ (IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_))) => Some(op),
            _ => None,
        }
    }

    pub fn decoded_io_op(&self) -> Option<IoOp> {
        self.decoded_io_op
    }

    // Bus phase methods

    fn phase_a1(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        ctrl.clear_io_op();
        ctrl.deselect_ram(0);

        // Sample INT pin and check for interrupt service
        if let Some(int_signal) = &ctrl.int {
            // Get the latest signal value from history
            let int_state = int_signal
                .history()
                .last()
                .map(|(_, level)| level.is_high())
                .unwrap_or(false);
            self.intr.set_int_pin(int_state);

            // Check if interrupt should be serviced (at instruction boundary)
            if self.intr.should_service() {
                // Save current PC to stack and jump to interrupt handler
                let current_pc = self.registers.pc();
                self.registers.push_return(current_pc);
                // Acknowledge interrupt (auto-disables)
                self.intr.acknowledge();
                // Vector to interrupt handler at 0x003
                self.registers.set_pc(0x003);
                self.pc_modified = true;
            }
        }

        // Output address bits 0-3 and assert SYNC
        let addr = self.registers.pc();
        bus.write((addr & 0x0F) as u8);
        ctrl.assert_sync(0);
    }

    fn phase_a2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 4-7, deassert SYNC
        let addr = self.registers.pc();
        bus.write(((addr >> 4) & 0x0F) as u8);
        ctrl.deassert_sync(0);
    }

    fn phase_a3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 8-11, select ROM bank
        let addr = self.registers.pc();
        bus.write(((addr >> 8) & 0x0F) as u8);
        ctrl.select_rom((addr >> 8) as u8 & 0x0F, 0);
    }

    fn phase_m1(&mut self, bus: &mut DataBus) {
        // Read instruction OPA (bits 0-3)
        let opa = bus.read();
        self.instruction_byte = (self.instruction_byte & 0xF0) | (opa & 0x0F);
    }

    fn phase_m2(&mut self, bus: &mut DataBus) {
        // Read instruction OPR (bits 4-7)
        let opr = bus.read();
        self.instruction_byte = (self.instruction_byte & 0x0F) | ((opr & 0x0F) << 4);
    }

    fn phase_x1(&mut self, _bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Decode the instruction
        if self.cycle.second_cycle {
            self.decoder.decode_second(self.instruction_byte);
        } else {
            self.decoder.decode_first(self.instruction_byte);
            // Track whether this is a two-byte instruction for cycle state
            self.cycle.two_cycle = self.decoder.two_byte;
        }
        self.pc_modified = false;

        // Decode I/O operations for phase-accurate control lines
        self.decoded_io_op = match self.decoder.get_instruction() {
            Some(i4004::Instruction::Src { .. }) => Some(IoOp::Src),
            Some(i4004::Instruction::Wrm) => Some(IoOp::RamMainWrite),
            Some(i4004::Instruction::Rdm | i4004::Instruction::Adm | i4004::Instruction::Sbm) => {
                Some(IoOp::RamMainRead)
            }
            Some(i4004::Instruction::Wmp) => Some(IoOp::RamPortWrite),
            Some(i4004::Instruction::Wrr) => Some(IoOp::RomPortWrite),
            Some(i4004::Instruction::Rdr) => Some(IoOp::RomPortRead),
            Some(i4004::Instruction::Wr0) => Some(IoOp::RamStatusWrite(0)),
            Some(i4004::Instruction::Wr1) => Some(IoOp::RamStatusWrite(1)),
            Some(i4004::Instruction::Wr2) => Some(IoOp::RamStatusWrite(2)),
            Some(i4004::Instruction::Wr3) => Some(IoOp::RamStatusWrite(3)),
            Some(i4004::Instruction::Rd0) => Some(IoOp::RamStatusRead(0)),
            Some(i4004::Instruction::Rd1) => Some(IoOp::RamStatusRead(1)),
            Some(i4004::Instruction::Rd2) => Some(IoOp::RamStatusRead(2)),
            Some(i4004::Instruction::Rd3) => Some(IoOp::RamStatusRead(3)),
            _ => None,
        };
        ctrl.clear_io_op();
    }

    fn phase_x2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Assert write-oriented I/O op only during X2
        ctrl.clear_io_op();
        if let Some(op) = self.decoded_io_op {
            if matches!(
                op,
                IoOp::Src | IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RomPortWrite | IoOp::RamStatusWrite(_)
            ) {
                ctrl.set_io_op(op);
            }
        }

        // Execute non-read instructions
        if let Some(instr) = self.decoder.get_instruction() {
            let is_read = matches!(
                instr,
                i4004::Instruction::Rdm
                    | i4004::Instruction::Rdr
                    | i4004::Instruction::Rd0
                    | i4004::Instruction::Rd1
                    | i4004::Instruction::Rd2
                    | i4004::Instruction::Rd3
                    | i4004::Instruction::Adm
                    | i4004::Instruction::Sbm
            );
            if !is_read {
                self.execute_4004(instr, bus);
            }
        }

        // SRC bus behavior
        if ctrl.io_op == Some(IoOp::Src) {
            bus.write(self.ram_chip & 0x0F);
        }
    }

    fn phase_x3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Assert read-oriented I/O op only during X3
        ctrl.clear_io_op();
        if let Some(op) = self.decoded_io_op {
            if matches!(
                op,
                IoOp::Src | IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_)
            ) {
                ctrl.set_io_op(op);
            }
        }

        // SRC bus behavior
        if ctrl.io_op == Some(IoOp::Src) {
            bus.write(self.ram_address & 0x0F);
        }

        // Execute read-oriented instructions after peripherals drive the bus
        if let Some(instr) = self.decoder.get_instruction() {
            let is_read = matches!(
                instr,
                i4004::Instruction::Rdm
                    | i4004::Instruction::Rdr
                    | i4004::Instruction::Rd0
                    | i4004::Instruction::Rd1
                    | i4004::Instruction::Rd2
                    | i4004::Instruction::Rd3
                    | i4004::Instruction::Adm
                    | i4004::Instruction::Sbm
            );
            if is_read {
                self.execute_4004(instr, bus);
            }
        }

        // Advance PC if not modified by instruction
        if !self.pc_modified {
            let next_pc = (self.registers.pc() + 1) & 0x0FFF;
            self.registers.set_pc(next_pc);
        }
    }

    /// Execute a 4004 instruction (backward compatibility)
    fn execute_4004(&mut self, instr: i4004::Instruction, bus: &mut DataBus) {
        use i4004::Instruction::*;
        match instr {
            Nop => {}

            Jcn { condition, addr_low } => {
                let jump = self.evaluate_condition(condition);
                if jump {
                    let pc = self.registers.pc();
                    let new_pc = (pc & 0xF00) | (addr_low as u16);
                    self.registers.set_pc(new_pc);
                    self.pc_modified = true;
                }
            }

            Fim { pair, data } => {
                self.registers.set_pair(pair, data);
            }
            Src { pair } => {
                let addr = self.registers.get_pair(pair);
                self.ram_address = addr & 0x0F;
                self.ram_chip = (addr >> 4) & 0x0F;
            }
            Fin { pair } => {
                let addr = self.registers.get_pair(0);
                self.io_data = addr;
                let _ = pair;
            }
            Jin { pair } => {
                let addr = self.registers.get_pair(pair);
                let pc = self.registers.pc();
                let new_pc = (pc & 0xF00) | (addr as u16);
                self.registers.set_pc(new_pc);
                self.pc_modified = true;
            }

            Jun { addr_high, addr_low } => {
                let new_pc = ((addr_high as u16) << 8) | (addr_low as u16);
                self.registers.set_pc(new_pc);
                self.pc_modified = true;
            }
            Jms { addr_high, addr_low } => {
                let new_pc = ((addr_high as u16) << 8) | (addr_low as u16);
                let return_addr = (self.registers.pc() + 1) & 0x0FFF;
                self.registers.call(return_addr, new_pc);
                self.pc_modified = true;
            }
            Isz { reg, addr_low } => {
                let wrapped = self.registers.inc_r(reg);
                if !wrapped {
                    let pc = self.registers.pc();
                    let new_pc = (pc & 0xF00) | (addr_low as u16);
                    self.registers.set_pc(new_pc);
                    self.pc_modified = true;
                }
            }

            Inc { reg } => {
                self.registers.inc_r(reg);
            }
            Add { reg } => {
                let value = self.registers.get_r(reg);
                self.alu.add(value);
            }
            Sub { reg } => {
                let value = self.registers.get_r(reg);
                self.alu.sub(value);
            }
            Ld { reg } => {
                let value = self.registers.get_r(reg);
                self.alu.load(value);
            }
            Xch { reg } => {
                let reg_val = self.registers.get_r(reg);
                let old_acc = self.alu.xch(reg_val);
                self.registers.set_r(reg, old_acc);
            }
            Bbl { data } => {
                self.registers.ret();
                self.alu.load(data);
                self.pc_modified = true;
            }

            Ldm { data } => {
                self.alu.load(data);
            }

            Wrm | Wmp | Wrr | Wpm => {
                bus.write(self.alu.accumulator());
            }
            Wr0 | Wr1 | Wr2 | Wr3 => {
                bus.write(self.alu.accumulator());
            }
            Sbm => {
                let value = bus.read();
                self.alu.sub(value);
            }
            Rdm => {
                let value = bus.read();
                self.alu.load(value);
            }
            Rdr => {
                let value = bus.read();
                self.alu.load(value);
            }
            Adm => {
                let value = bus.read();
                self.alu.add(value);
            }
            Rd0 | Rd1 | Rd2 | Rd3 => {
                let value = bus.read();
                self.alu.load(value);
            }

            Clb => self.alu.clb(),
            Clc => self.alu.set_carry(false),
            Iac => self.alu.iac(),
            Cmc => self.alu.cmc(),
            Cma => self.alu.cma(),
            Ral => self.alu.ral(),
            Rar => self.alu.rar(),
            Tcc => self.alu.tcc(),
            Dac => self.alu.dac(),
            Tcs => {
                let value = if self.alu.carry() { 10 } else { 9 };
                self.alu.set_accumulator(value);
                self.alu.set_carry(false);
            }
            Stc => self.alu.stc(),
            Daa => self.alu.daa(),
            Kbp => self.alu.kbp(),
            Dcl => {
                self.ram_bank = self.alu.accumulator() & 0x0F;
            }

            Invalid { opcode } => {
                // 4040-specific opcodes are not recognized by the 4004 decoder,
                // so they come through as Invalid. Map them to I4040Instruction here.
                match opcode {
                    0x01 => self.execute_hlt(),
                    0x02 => self.execute_bbs(),
                    0x03 => self.execute_lcr(),
                    0x04 => self.execute_or4(),
                    0x05 => self.execute_or5(),
                    0x06 => self.execute_an6(),
                    0x07 => self.execute_an7(),
                    0x08 => self.execute_db0(),
                    0x09 => self.execute_db1(),
                    0x0A => self.execute_sb0(),
                    0x0B => self.execute_sb1(),
                    0x0C => self.execute_ein(),
                    0x0D => self.execute_din(),
                    0x0E => self.execute_rpm(),
                    _ => {}
                }
            }
        }
    }

    /// Evaluate JCN condition
    fn evaluate_condition(&self, condition: u8) -> bool {
        let test_acc_zero = (condition & 0x04) != 0;
        let test_carry = (condition & 0x02) != 0;
        let test_pin = (condition & 0x01) != 0;
        let invert = (condition & 0x08) != 0;

        let mut result = false;
        if test_acc_zero && self.alu.accumulator() == 0 {
            result = true;
        }
        if test_carry && self.alu.carry() {
            result = true;
        }
        if test_pin && self.test_pin {
            result = true;
        }

        if invert {
            !result
        } else {
            result
        }
    }

    /// Execute a 4040-specific instruction
    pub fn execute(&mut self, instruction: I4040Instruction) {
        match instruction {
            I4040Instruction::Hlt => self.execute_hlt(),
            I4040Instruction::Bbs => self.execute_bbs(),
            I4040Instruction::Lcr => self.execute_lcr(),
            I4040Instruction::Or4 => self.execute_or4(),
            I4040Instruction::Or5 => self.execute_or5(),
            I4040Instruction::An6 => self.execute_an6(),
            I4040Instruction::An7 => self.execute_an7(),
            I4040Instruction::Db0 => self.execute_db0(),
            I4040Instruction::Db1 => self.execute_db1(),
            I4040Instruction::Sb0 => self.execute_sb0(),
            I4040Instruction::Sb1 => self.execute_sb1(),
            I4040Instruction::Ein => self.execute_ein(),
            I4040Instruction::Din => self.execute_din(),
            I4040Instruction::Rpm => self.execute_rpm(),
        }
    }

    // Instruction implementations

    /// HLT (0x01) - Halt execution (low-power mode)
    fn execute_hlt(&mut self) {
        self.halted = true;
    }

    /// BBS (0x02) - Branch Back from interrupt
    fn execute_bbs(&mut self) {
        // Restore PC from stack (ret_from_interrupt pops SP and sets PC directly)
        let _ = self.registers.ret_from_interrupt(); // Restores PC, returns saved SRC (unused)
                                                     // Note: Interrupts remain disabled (disabled by acknowledge() on INT entry)
                                                     // The ISR must call EIN explicitly to re-enable interrupts
        self.pc_modified = true;
    }

    /// LCR (0x03) - Load Command RAM (ROM -> RAM)
    fn execute_lcr(&mut self) {
        // TODO: Implement ROM to RAM copy
        // ram[char] = rom[pc]; advance ROM address
    }

    /// OR4 (0x04) - OR accumulator with R4
    fn execute_or4(&mut self) {
        let r4 = self.registers.get_r(4);
        let acc = self.alu.accumulator();
        self.alu.set_accumulator(acc | r4);
    }

    /// OR5 (0x05) - OR accumulator with R5
    fn execute_or5(&mut self) {
        let r5 = self.registers.get_r(5);
        let acc = self.alu.accumulator();
        self.alu.set_accumulator(acc | r5);
    }

    /// AN6 (0x06) - AND accumulator with R6
    fn execute_an6(&mut self) {
        let r6 = self.registers.get_r(6);
        let acc = self.alu.accumulator();
        self.alu.set_accumulator(acc & r6);
    }

    /// AN7 (0x07) - AND accumulator with R7
    fn execute_an7(&mut self) {
        let r7 = self.registers.get_r(7);
        let acc = self.alu.accumulator();
        self.alu.set_accumulator(acc & r7);
    }

    /// DB0 (0x08) - Designate Bank 0
    fn execute_db0(&mut self) {
        self.registers.set_bank(0);
    }

    /// DB1 (0x09) - Designate Bank 1
    fn execute_db1(&mut self) {
        self.registers.set_bank(1);
    }

    /// SB0 (0x0A) - Select RAM Bank 0
    fn execute_sb0(&mut self) {
        self.ram_bank = 0;
    }

    /// SB1 (0x0B) - Select RAM Bank 1
    fn execute_sb1(&mut self) {
        self.ram_bank = 1;
    }

    /// EIN (0x0C) - Enable Interrupts
    fn execute_ein(&mut self) {
        self.intr.enable();
    }

    /// DIN (0x0D) - Disable Interrupts
    fn execute_din(&mut self) {
        self.intr.disable();
    }

    /// RPM (0x0E) - Read Program Memory
    fn execute_rpm(&mut self) {
        // TODO: Implement ROM read into accumulator
        // acc = rom[pc]; advance ROM address
    }
}

impl Default for I4040 {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::Chip for I4040 {
    fn name(&self) -> &'static str {
        "4040"
    }

    fn reset(&mut self) {
        self.alu = i4004::Alu::new();
        self.registers = Registers::new();
        self.registers_4004 = i4004::Registers::new();
        self.intr = InterruptController::new();
        self.decoder = i4004::InstructionDecoder::new();
        self.halted = false;
        self.ram_bank = 0;
        self.cycle = CycleState::new();
        self.instruction_byte = 0;
        self.operand = 0;
        self.ram_address = 0;
        self.ram_chip = 0;
        self.test_pin = false;
        self.io_data = 0;
        self.decoded_io_op = None;
        self.pc_modified = false;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus access
        let _ = phase;
    }
}
