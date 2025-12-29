//! Intel 4308 ROM (stub)
use mcs4_bus::BusCycle;

#[derive(Clone, Debug)]
pub struct I4308 { rom: Vec<u8> }
impl I4308 { pub fn new() -> Self { Self { rom: vec![0; 1024] } } }
impl Default for I4308 { fn default() -> Self { Self::new() } }
impl super::Chip for I4308 {
    fn name(&self) -> &'static str { "4308" }
    fn reset(&mut self) {}
    fn tick(&mut self, _phase: BusCycle) {}
}
