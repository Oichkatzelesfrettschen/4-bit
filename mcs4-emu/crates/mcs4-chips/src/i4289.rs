//! Intel 4289 Standard Memory Interface (stub)
use mcs4_bus::BusCycle;

#[derive(Clone, Debug, Default)]
pub struct I4289;
impl I4289 { pub fn new() -> Self { Self } }
impl super::Chip for I4289 {
    fn name(&self) -> &'static str { "4289" }
    fn reset(&mut self) {}
    fn tick(&mut self, _phase: BusCycle) {}
}
