//! Intel 4201 Clock Generator (stub)
use mcs4_bus::BusCycle;

#[derive(Clone, Debug, Default)]
pub struct I4201;
impl I4201 { pub fn new() -> Self { Self } }
impl super::Chip for I4201 {
    fn name(&self) -> &'static str { "4201" }
    fn reset(&mut self) {}
    fn tick(&mut self, _phase: BusCycle) {}
}
