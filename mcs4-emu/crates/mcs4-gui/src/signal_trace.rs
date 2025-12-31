// Signal trace buffer scaffolding
use std::sync::{Arc,RwLock};

#[derive(Clone, Copy, Debug)]
pub enum BusCycle { A1, A2, A3, M1, M2, X1, X2, X3 }

pub struct SignalTrace {
    pub timestamps: Vec<u64>,
    pub phi1: Vec<bool>,
    pub phi2: Vec<bool>,
    pub sync: Vec<bool>,
    pub data_bus: Vec<u8>,
    pub cm_rom: Vec<u8>,
    pub cm_ram: Vec<u8>,
    pub phase: Vec<BusCycle>,
}

impl SignalTrace {
    pub fn new() -> Self { Self { timestamps: vec![], phi1: vec![], phi2: vec![], sync: vec![], data_bus: vec![], cm_rom: vec![], cm_ram: vec![], phase: vec![] } }
    pub fn capture(&mut self, tick: u64, phi1: bool, phi2: bool, sync: bool, data: u8, cm_rom: u8, cm_ram: u8, phase: BusCycle) {
        self.timestamps.push(tick);
        self.phi1.push(phi1);
        self.phi2.push(phi2);
        self.sync.push(sync);
        self.data_bus.push(data & 0x0F);
        self.cm_rom.push(cm_rom & 0x0F);
        self.cm_ram.push(cm_ram & 0x03);
        self.phase.push(phase);
    }
}
