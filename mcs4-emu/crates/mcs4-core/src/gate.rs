//! Gate-level primitives for digital simulation
//!
//! These implement the basic logic gates used in the Intel 4004/4040.
//! The 4004 was implemented in pMOS technology, primarily using NAND
//! and NOR gates with depletion-load inverters.

use crate::signal::{SignalId, SignalLevel};
use crate::timing::{Delay, gate_delay};

/// Gate type enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateType {
    Inv,
    Nand2,
    Nand3,
    Nand4,
    Nor2,
    Nor3,
    Nor4,
    And2,
    Or2,
    Xor2,
    Mux2,
    Latch,
    DFlipFlop,
}

impl GateType {
    /// Base propagation delay for this gate type
    pub fn base_delay(self) -> Delay {
        match self {
            GateType::Inv => gate_delay::INV_BASE,
            GateType::Nand2 | GateType::And2 => gate_delay::NAND2_BASE,
            GateType::Nand3 => gate_delay::NAND3_BASE,
            GateType::Nand4 => gate_delay::NAND3_BASE + 2000, // ~2ns more
            GateType::Nor2 | GateType::Or2 => gate_delay::NOR2_BASE,
            GateType::Nor3 => gate_delay::NOR3_BASE,
            GateType::Nor4 => gate_delay::NOR3_BASE + 2000,
            GateType::Xor2 => gate_delay::NAND2_BASE * 2, // XOR = 2 gate levels
            GateType::Mux2 => gate_delay::NAND2_BASE * 2,
            GateType::Latch => gate_delay::INV_BASE * 2,
            GateType::DFlipFlop => gate_delay::NAND2_BASE * 3,
        }
    }
}

/// Trait for all gate primitives
pub trait Gate: Send + Sync {
    /// Gate type
    fn gate_type(&self) -> GateType;

    /// Evaluate output given current input states
    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel;

    /// Propagation delay (including fanout effects)
    fn propagation_delay(&self) -> Delay;

    /// Output signal ID
    fn output(&self) -> SignalId;

    /// Input signal IDs
    fn inputs(&self) -> &[SignalId];
}

/// Inverter (NOT gate)
#[derive(Clone, Debug)]
pub struct Inverter {
    pub input: SignalId,
    pub output: SignalId,
    pub delay: Delay,
}

impl Inverter {
    pub fn new(input: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            input,
            output,
            delay: gate_delay::with_fanout(gate_delay::INV_BASE, fanout),
        }
    }
}

impl Gate for Inverter {
    fn gate_type(&self) -> GateType {
        GateType::Inv
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 1);
        inputs[0].invert()
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        std::slice::from_ref(&self.input)
    }
}

/// 2-input NAND gate
#[derive(Clone, Debug)]
pub struct Nand2 {
    pub inputs: [SignalId; 2],
    pub output: SignalId,
    pub delay: Delay,
}

impl Nand2 {
    pub fn new(a: SignalId, b: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            inputs: [a, b],
            output,
            delay: gate_delay::with_fanout(gate_delay::NAND2_BASE, fanout),
        }
    }
}

impl Gate for Nand2 {
    fn gate_type(&self) -> GateType {
        GateType::Nand2
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 2);
        inputs[0].and(inputs[1]).invert()
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        &self.inputs
    }
}

/// 3-input NAND gate
#[derive(Clone, Debug)]
pub struct Nand3 {
    pub inputs: [SignalId; 3],
    pub output: SignalId,
    pub delay: Delay,
}

impl Nand3 {
    pub fn new(a: SignalId, b: SignalId, c: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            inputs: [a, b, c],
            output,
            delay: gate_delay::with_fanout(gate_delay::NAND3_BASE, fanout),
        }
    }
}

impl Gate for Nand3 {
    fn gate_type(&self) -> GateType {
        GateType::Nand3
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 3);
        inputs[0].and(inputs[1]).and(inputs[2]).invert()
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        &self.inputs
    }
}

/// 2-input NOR gate
#[derive(Clone, Debug)]
pub struct Nor2 {
    pub inputs: [SignalId; 2],
    pub output: SignalId,
    pub delay: Delay,
}

impl Nor2 {
    pub fn new(a: SignalId, b: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            inputs: [a, b],
            output,
            delay: gate_delay::with_fanout(gate_delay::NOR2_BASE, fanout),
        }
    }
}

impl Gate for Nor2 {
    fn gate_type(&self) -> GateType {
        GateType::Nor2
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 2);
        inputs[0].or(inputs[1]).invert()
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        &self.inputs
    }
}

/// 3-input NOR gate
#[derive(Clone, Debug)]
pub struct Nor3 {
    pub inputs: [SignalId; 3],
    pub output: SignalId,
    pub delay: Delay,
}

impl Nor3 {
    pub fn new(a: SignalId, b: SignalId, c: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            inputs: [a, b, c],
            output,
            delay: gate_delay::with_fanout(gate_delay::NOR3_BASE, fanout),
        }
    }
}

impl Gate for Nor3 {
    fn gate_type(&self) -> GateType {
        GateType::Nor3
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 3);
        inputs[0].or(inputs[1]).or(inputs[2]).invert()
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        &self.inputs
    }
}

/// 2-input AND gate (NAND + INV, but modeled as single gate)
#[derive(Clone, Debug)]
pub struct And2 {
    pub inputs: [SignalId; 2],
    pub output: SignalId,
    pub delay: Delay,
}

impl And2 {
    pub fn new(a: SignalId, b: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            inputs: [a, b],
            output,
            // AND = NAND + INV
            delay: gate_delay::with_fanout(gate_delay::NAND2_BASE + gate_delay::INV_BASE, fanout),
        }
    }
}

impl Gate for And2 {
    fn gate_type(&self) -> GateType {
        GateType::And2
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 2);
        inputs[0].and(inputs[1])
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        &self.inputs
    }
}

/// 2-input OR gate (NOR + INV, but modeled as single gate)
#[derive(Clone, Debug)]
pub struct Or2 {
    pub inputs: [SignalId; 2],
    pub output: SignalId,
    pub delay: Delay,
}

impl Or2 {
    pub fn new(a: SignalId, b: SignalId, output: SignalId, fanout: usize) -> Self {
        Self {
            inputs: [a, b],
            output,
            delay: gate_delay::with_fanout(gate_delay::NOR2_BASE + gate_delay::INV_BASE, fanout),
        }
    }
}

impl Gate for Or2 {
    fn gate_type(&self) -> GateType {
        GateType::Or2
    }

    fn evaluate(&self, inputs: &[SignalLevel]) -> SignalLevel {
        debug_assert_eq!(inputs.len(), 2);
        inputs[0].or(inputs[1])
    }

    fn propagation_delay(&self) -> Delay {
        self.delay
    }

    fn output(&self) -> SignalId {
        self.output
    }

    fn inputs(&self) -> &[SignalId] {
        &self.inputs
    }
}

/// SR Latch (built from cross-coupled NOR gates)
#[derive(Clone, Debug)]
pub struct SRLatch {
    pub s: SignalId,
    pub r: SignalId,
    pub q: SignalId,
    pub q_bar: SignalId,
    pub delay: Delay,
    state: SignalLevel,
}

impl SRLatch {
    pub fn new(s: SignalId, r: SignalId, q: SignalId, q_bar: SignalId, fanout: usize) -> Self {
        Self {
            s,
            r,
            q,
            q_bar,
            delay: gate_delay::with_fanout(gate_delay::NOR2_BASE * 2, fanout),
            state: SignalLevel::Low,
        }
    }

    /// Update latch state
    pub fn update(&mut self, s: SignalLevel, r: SignalLevel) -> (SignalLevel, SignalLevel) {
        match (s, r) {
            (SignalLevel::High, SignalLevel::Low) => self.state = SignalLevel::High,
            (SignalLevel::Low, SignalLevel::High) => self.state = SignalLevel::Low,
            (SignalLevel::High, SignalLevel::High) => {} // Invalid - keep current
            (SignalLevel::Low, SignalLevel::Low) => {}   // Hold
            _ => {}
        }
        (self.state, self.state.invert())
    }

    pub fn state(&self) -> SignalLevel {
        self.state
    }
}

/// D Flip-Flop (edge-triggered)
#[derive(Clone, Debug)]
pub struct DFlipFlop {
    pub d: SignalId,
    pub clk: SignalId,
    pub q: SignalId,
    pub q_bar: SignalId,
    pub delay: Delay,
    state: SignalLevel,
    prev_clk: SignalLevel,
}

impl DFlipFlop {
    pub fn new(d: SignalId, clk: SignalId, q: SignalId, q_bar: SignalId, fanout: usize) -> Self {
        Self {
            d,
            clk,
            q,
            q_bar,
            delay: gate_delay::with_fanout(gate_delay::NAND2_BASE * 3, fanout),
            state: SignalLevel::Low,
            prev_clk: SignalLevel::Low,
        }
    }

    /// Update on clock edge
    pub fn update(&mut self, d: SignalLevel, clk: SignalLevel) -> (SignalLevel, SignalLevel) {
        // Rising edge detection
        if self.prev_clk == SignalLevel::Low && clk == SignalLevel::High {
            self.state = d;
        }
        self.prev_clk = clk;
        (self.state, self.state.invert())
    }

    pub fn state(&self) -> SignalLevel {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverter() {
        let inv = Inverter::new(SignalId(0), SignalId(1), 1);
        assert_eq!(inv.evaluate(&[SignalLevel::High]), SignalLevel::Low);
        assert_eq!(inv.evaluate(&[SignalLevel::Low]), SignalLevel::High);
    }

    #[test]
    fn test_nand2() {
        let nand = Nand2::new(SignalId(0), SignalId(1), SignalId(2), 1);
        assert_eq!(nand.evaluate(&[SignalLevel::High, SignalLevel::High]), SignalLevel::Low);
        assert_eq!(nand.evaluate(&[SignalLevel::High, SignalLevel::Low]), SignalLevel::High);
        assert_eq!(nand.evaluate(&[SignalLevel::Low, SignalLevel::High]), SignalLevel::High);
        assert_eq!(nand.evaluate(&[SignalLevel::Low, SignalLevel::Low]), SignalLevel::High);
    }

    #[test]
    fn test_nor2() {
        let nor = Nor2::new(SignalId(0), SignalId(1), SignalId(2), 1);
        assert_eq!(nor.evaluate(&[SignalLevel::High, SignalLevel::High]), SignalLevel::Low);
        assert_eq!(nor.evaluate(&[SignalLevel::High, SignalLevel::Low]), SignalLevel::Low);
        assert_eq!(nor.evaluate(&[SignalLevel::Low, SignalLevel::High]), SignalLevel::Low);
        assert_eq!(nor.evaluate(&[SignalLevel::Low, SignalLevel::Low]), SignalLevel::High);
    }

    #[test]
    fn test_sr_latch() {
        let mut latch = SRLatch::new(SignalId(0), SignalId(1), SignalId(2), SignalId(3), 1);

        // Set
        let (q, qb) = latch.update(SignalLevel::High, SignalLevel::Low);
        assert_eq!(q, SignalLevel::High);
        assert_eq!(qb, SignalLevel::Low);

        // Hold
        let (q, qb) = latch.update(SignalLevel::Low, SignalLevel::Low);
        assert_eq!(q, SignalLevel::High);
        assert_eq!(qb, SignalLevel::Low);

        // Reset
        let (q, qb) = latch.update(SignalLevel::Low, SignalLevel::High);
        assert_eq!(q, SignalLevel::Low);
        assert_eq!(qb, SignalLevel::High);
    }

    #[test]
    fn test_dff() {
        let mut dff = DFlipFlop::new(SignalId(0), SignalId(1), SignalId(2), SignalId(3), 1);

        // D=1, no clock edge - should not change
        let (q, _) = dff.update(SignalLevel::High, SignalLevel::Low);
        assert_eq!(q, SignalLevel::Low);

        // Rising clock edge with D=1
        let (q, _) = dff.update(SignalLevel::High, SignalLevel::High);
        assert_eq!(q, SignalLevel::High);

        // D changes while clock high - no effect
        let (q, _) = dff.update(SignalLevel::Low, SignalLevel::High);
        assert_eq!(q, SignalLevel::High);

        // Clock falls
        let (q, _) = dff.update(SignalLevel::Low, SignalLevel::Low);
        assert_eq!(q, SignalLevel::High);

        // Rising edge with D=0
        let (q, _) = dff.update(SignalLevel::Low, SignalLevel::High);
        assert_eq!(q, SignalLevel::Low);
    }
}
