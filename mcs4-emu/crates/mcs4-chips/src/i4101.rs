//! Intel 4101 RAM (stub)
use mcs4_bus::BusCycle;

#[derive(Clone, Debug)]
pub struct I4101 { ram: [u8; 256] }
impl I4101 { pub fn new() -> Self { Self { ram: [0; 256] } } }
impl Default for I4101 { fn default() -> Self { Self::new() } }
impl super::Chip for I4101 {
    fn name(&self) -> &'static str { "4101" }
    fn reset(&mut self) { self.ram = [0; 256]; }
    fn tick(&mut self, _phase: BusCycle) {}
}
