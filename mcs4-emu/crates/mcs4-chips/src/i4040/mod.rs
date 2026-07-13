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
pub mod solver_bridge;

pub use instruction_decode::{decode_4040_specific, I4040Instruction, Instruction};
pub use interrupt::InterruptController;
// Note: 4040 uses 4004's ALU (no extensions needed)
use mcs4_bus::prelude::*;
use mcs4_core::SimulationFidelity;
pub use registers::Registers;

use crate::{
    i4004,
    i4004::{decode_cm_ram_lines, CM_RAM0},
};

/// Intel 4040 behavioral CPU.
///
/// The CPU executes phase-resolved instruction behavior. Physical pin-map and
/// transistor-netlist fidelity remain separately evidence-gated.
pub struct I4040 {
    /// ALU (from 4004 base)
    pub alu: i4004::Alu,
    /// 4040 registers
    pub registers: Registers,
    /// Interrupt controller
    pub intr: InterruptController,
    /// Instruction decoder
    pub decoder: i4004::InstructionDecoder,
    /// Halted state
    halted: bool,
    /// CM-RAM line selection mask (set by DCL; reset selects CM-RAM0)
    ram_bank: u8,
    /// ROM bank selection (0 or 1 via DB0/DB1)
    rom_bank: u8,
    /// Simulation fidelity level
    fidelity: SimulationFidelity,
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
    /// Decoded I/O operation for current instruction
    decoded_io_op: Option<IoOp>,
    /// True if PC was explicitly modified by instruction
    pc_modified: bool,
    /// Register pair awaiting the FIN indirect ROM fetch during the second machine cycle.
    fin_pair: Option<u8>,
}

impl I4040 {
    pub fn new() -> Self {
        Self {
            alu: i4004::Alu::new(),
            registers: Registers::new(),
            intr: InterruptController::new(),
            decoder: i4004::InstructionDecoder::new(),
            halted: false,
            ram_bank: CM_RAM0,
            rom_bank: 0,
            fidelity: SimulationFidelity::Behavioral,
            cycle: CycleState::new(),
            instruction_byte: 0,
            operand: 0,
            ram_address: 0,
            ram_chip: 0,
            test_pin: false,
            decoded_io_op: None,
            pc_modified: false,
            fin_pair: None,
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
        if self.halted {
            if phase == BusCycle::A1 {
                self.service_interrupt_if_pending(ctrl);
            }
            if self.halted {
                self.cycle.advance();
                return;
            }
        }
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

    pub fn rom_bank(&self) -> u8 {
        self.rom_bank
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
            Some(op @ (IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_) | IoOp::Rpm)) => Some(op),
            _ => None,
        }
    }

    pub fn decoded_io_op(&self) -> Option<IoOp> {
        self.decoded_io_op
    }

    fn is_read_instruction(&self, instr: i4004::Instruction) -> bool {
        matches!(
            instr,
            i4004::Instruction::Rdm
                | i4004::Instruction::Rdr
                | i4004::Instruction::Rd0
                | i4004::Instruction::Rd1
                | i4004::Instruction::Rd2
                | i4004::Instruction::Rd3
                | i4004::Instruction::Adm
                | i4004::Instruction::Sbm
        ) || matches!(instr, i4004::Instruction::Invalid { opcode: 0x0E })
    }

    // Bus phase methods

    fn phase_a1(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        ctrl.clear_io_op();
        ctrl.deselect_ram(0);

        self.service_interrupt_if_pending(ctrl);

        // Output address bits 0-3 and assert SYNC
        let addr = self.fetch_address();
        bus.write((addr & 0x0F) as u8);
        ctrl.assert_sync(0);
    }

    fn service_interrupt_if_pending(&mut self, ctrl: &ControlSignals) {
        // Sample INT pin and check for interrupt service.
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
                self.registers.save_src((self.ram_chip << 4) | self.ram_address);
                self.registers.push_return(current_pc);
                // Acknowledge interrupt (auto-disables)
                self.intr.acknowledge();
                // Vector to interrupt handler at 0x003
                self.registers.set_pc(0x003);
                self.pc_modified = true;
                self.halted = false;
            }
        }
    }

    /// Address emitted during A1-A3.
    ///
    /// The FIN second cycle sends register pair 0 as the low 8 bits with the
    /// page taken from the program counter. The PC has already advanced past
    /// the FIN byte, so a FIN in the last location of a page fetches from the
    /// next page, matching the documented boundary behavior.
    fn fetch_address(&self) -> u16 {
        let pc = self.registers.pc();
        if self.fin_pair.is_some() && self.cycle.second_cycle {
            (pc & 0xF00) | u16::from(self.registers.get_pair(0))
        } else {
            pc
        }
    }

    fn phase_a2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 4-7, deassert SYNC
        let addr = self.fetch_address();
        bus.write(((addr >> 4) & 0x0F) as u8);
        ctrl.deassert_sync(0);
    }

    fn phase_a3(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Output address bits 8-11, select ROM bank
        let addr = self.fetch_address();
        bus.write(((addr >> 8) & 0x0F) as u8);

        // 4040 selects one of two 4KB banks via DB0/DB1.
        // We use the first two CM-ROM lines for these banks.
        ctrl.deselect_rom(0);
        ctrl.select_rom(self.rom_bank, 0);
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
        // FIN second cycle: M1/M2 latched the byte at the indirect address;
        // load it into the target register pair. The PC already points at the
        // next instruction, so suppress decode and the X3 PC advance.
        if self.cycle.second_cycle {
            if let Some(pair) = self.fin_pair.take() {
                self.registers.set_pair(pair, self.instruction_byte);
                self.decoder.instruction = None;
                self.decoded_io_op = None;
                self.pc_modified = true;
                ctrl.clear_io_op();
                return;
            }
        }

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
            Some(i4004::Instruction::Invalid { opcode: 0x0E }) => Some(IoOp::Rpm),
            _ => None,
        };
        ctrl.clear_io_op();
    }

    fn phase_x2(&mut self, bus: &mut DataBus, ctrl: &mut ControlSignals) {
        // Assert write-oriented I/O op only during X2
        ctrl.clear_io_op();
        ctrl.deassert_pm(0);

        if let Some(op) = self.decoded_io_op {
            if op == IoOp::Rpm {
                ctrl.assert_pm(0);
            }
            if matches!(
                op,
                IoOp::Src | IoOp::RamMainWrite | IoOp::RamPortWrite | IoOp::RomPortWrite | IoOp::RamStatusWrite(_)
            ) {
                ctrl.set_io_op(op);
            }
        }

        // Execute non-read instructions
        if let Some(instr) = self.decoder.get_instruction() {
            if !self.is_read_instruction(instr) {
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
            if op == IoOp::Rpm {
                ctrl.assert_pm(0);
            }
            if matches!(
                op,
                IoOp::Src | IoOp::RamMainRead | IoOp::RomPortRead | IoOp::RamStatusRead(_) | IoOp::Rpm
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
            if self.is_read_instruction(instr) {
                self.execute_4004(instr, bus);
            }
        }

        // FIN: schedule the indirect ROM fetch as a second machine cycle.
        // The PC still advances past the FIN byte below, which gives the
        // second cycle's A phases the page of the byte after the FIN (the
        // documented page-boundary behavior).
        if self.fin_pair.is_some() && !self.cycle.second_cycle {
            self.cycle.two_cycle = true;
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
                // Fetch indirect: arm the second machine cycle, which sends
                // register pair 0 as the ROM address and loads the fetched
                // byte into the target pair.
                self.fin_pair = Some(pair);
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
                // Designate command line: decode accumulator bits 2:0 into the
                // CM-RAM line mask that selects the DATA RAM bank.
                self.ram_bank = decode_cm_ram_lines(self.alu.accumulator());
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
                    0x0E => self.execute_rpm(bus),
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
        // The TEST condition is satisfied when the TEST pin is at 0.
        if test_pin && !self.test_pin {
            result = true;
        }

        if invert {
            !result
        } else {
            result
        }
    }

    /// Execute a 4040-specific instruction
    pub fn execute(&mut self, instruction: I4040Instruction, bus: &mut DataBus) {
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
            I4040Instruction::Rpm => self.execute_rpm(bus),
        }
    }

    // Instruction implementations

    /// HLT (0x01) - Halt execution (low-power mode)
    fn execute_hlt(&mut self) {
        self.halted = true;
    }

    /// BBS (0x02) - Branch Back from interrupt
    fn execute_bbs(&mut self) {
        // Restore PC and the saved SRC address. The X2/X3 phases then expose
        // the recovered address through the existing SRC bus path.
        let saved_src = self.registers.ret_from_interrupt();
        self.ram_chip = (saved_src >> 4) & 0x0f;
        self.ram_address = saved_src & 0x0f;
        self.decoded_io_op = Some(IoOp::Src);
        // Interrupts remain disabled after acknowledge. The ISR executes EIN
        // when it intentionally permits the next interrupt.
        self.pc_modified = true;
    }

    /// LCR (0x03) - Load Control Register
    ///
    /// Copies internal CPU state into the accumulator:
    /// Bit 3: ROM Bank (0 or 1)
    /// Bit 2: Index Register Bank (0 or 1)
    /// Bit 1: Interrupt Enable (0=disabled, 1=enabled)
    /// Bit 0: 0
    fn execute_lcr(&mut self) {
        let mut val = 0u8;
        if self.rom_bank == 1 {
            val |= 0x08;
        }
        if self.registers.bank() == 1 {
            val |= 0x04;
        }
        if self.intr.enabled() {
            val |= 0x02;
        }
        self.alu.set_accumulator(val);
    }

    /// RPM (0x0E) - Read Program Memory
    fn execute_rpm(&mut self, bus: &mut DataBus) {
        let val = bus.read() & 0x0F;
        self.alu.set_accumulator(val);
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

    /// DB0 (0x08) - Designate Bank 0 (ROM)
    fn execute_db0(&mut self) {
        self.rom_bank = 0;
    }

    /// DB1 (0x09) - Designate Bank 1 (ROM)
    fn execute_db1(&mut self) {
        self.rom_bank = 1;
    }

    /// SB0 (0x0A) - Select Bank 0 (Index Registers)
    fn execute_sb0(&mut self) {
        self.registers.set_bank(0);
    }

    /// SB1 (0x0B) - Select Bank 1 (Index Registers)
    fn execute_sb1(&mut self) {
        self.registers.set_bank(1);
    }

    /// EIN (0x0C) - Enable Interrupts
    fn execute_ein(&mut self) {
        self.intr.enable();
    }

    /// DIN (0x0D) - Disable Interrupts
    fn execute_din(&mut self) {
        self.intr.disable();
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
        self.intr = InterruptController::new();
        self.decoder = i4004::InstructionDecoder::new();
        self.halted = false;
        self.ram_bank = CM_RAM0;
        self.rom_bank = 0;
        self.cycle = CycleState::new();
        self.instruction_byte = 0;
        self.operand = 0;
        self.ram_address = 0;
        self.ram_chip = 0;
        self.test_pin = false;
        self.decoded_io_op = None;
        self.pc_modified = false;
        self.fin_pair = None;
    }

    fn tick(&mut self, phase: BusCycle) {
        // Simplified tick without bus access
        let _ = phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcr_execution() {
        let mut cpu = I4040::new();
        // Initial state: ROM bank 0, Reg bank 0, Int disabled -> 0x0
        cpu.execute_lcr();
        assert_eq!(cpu.accumulator(), 0x0);

        // ROM bank 1
        cpu.execute_db1();
        cpu.execute_lcr();
        assert_eq!(cpu.accumulator(), 0x08);

        // Reg bank 1
        cpu.execute_sb1();
        cpu.execute_lcr();
        assert_eq!(cpu.accumulator(), 0x0C); // 0x08 | 0x04

        // Int enabled
        cpu.execute_ein();
        cpu.execute_lcr();
        assert_eq!(cpu.accumulator(), 0x0E); // 0x08 | 0x04 | 0x02
    }

    #[test]
    fn sb0_sb1_switch_reg_bank() {
        let mut cpu = I4040::new();
        assert_eq!(cpu.registers.bank(), 0);

        cpu.execute_sb1();
        assert_eq!(cpu.registers.bank(), 1);

        cpu.execute_sb0();
        assert_eq!(cpu.registers.bank(), 0);
    }

    #[test]
    fn db0_db1_switch_rom_bank() {
        let mut cpu = I4040::new();
        assert_eq!(cpu.rom_bank, 0);

        cpu.execute_db1();
        assert_eq!(cpu.rom_bank, 1);

        cpu.execute_db0();
        assert_eq!(cpu.rom_bank, 0);
    }

    #[test]
    fn halt_and_resume() {
        let mut cpu = I4040::new();
        assert!(!cpu.halted());

        cpu.execute_hlt();
        assert!(cpu.halted());

        cpu.registers.set_pc(0x123);
        let mut bus = DataBus::new();
        let mut control = ControlSignals::mcs40();
        cpu.tick(BusCycle::A1, &mut bus, &mut control);
        assert_eq!(cpu.pc(), 0x123);
    }

    #[test]
    fn bbs_restores_saved_src_address() {
        let mut cpu = I4040::new();
        cpu.registers.save_src(0xab);
        cpu.registers.push_return(0x456);

        cpu.execute_bbs();

        assert_eq!(cpu.pc(), 0x456);
        assert_eq!(cpu.ram_chip(), 0x0a);
        assert_eq!(cpu.ram_address(), 0x0b);
        assert_eq!(cpu.decoded_io_op(), Some(IoOp::Src));
    }

    #[test]
    fn or_instructions() {
        let mut cpu = I4040::new();
        // Set R4 = 0xA, accumulator = 0x5
        cpu.registers.set_r(4, 0xA);
        cpu.alu.set_accumulator(0x5);

        cpu.execute_or4();
        assert_eq!(cpu.accumulator(), 0xF); // 0x5 | 0xA = 0xF
    }

    #[test]
    fn reset_clears_fields() {
        use crate::Chip;
        let mut cpu = I4040::new();
        cpu.execute_db1();
        cpu.execute_sb1();
        cpu.execute_ein();
        assert_eq!(cpu.rom_bank, 1);

        cpu.reset();
        assert_eq!(cpu.rom_bank, 0);
        assert_eq!(cpu.registers.bank(), 0);
        assert!(!cpu.intr.enabled());
    }
}
