//! Frontend-neutral machine-state data transfer objects.
//!
//! The runtime owns the canonical shape of every state view the worker
//! publishes. A frontend renders these structs; it does not define them. Each
//! type is plain data with pure accessors, so egui, Bevy, or a headless test
//! consumes one schema.

/// A memory region a snapshot describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegion {
    /// Program ROM.
    Rom,
    /// Data RAM main memory.
    Ram,
}

/// Snapshot of a memory region for display.
#[derive(Clone, Debug)]
pub struct MemorySnapshot {
    /// Region this snapshot covers.
    pub region: MemoryRegion,
    /// First address the `data` slice represents.
    pub base_addr: u16,
    /// Contiguous bytes starting at `base_addr`.
    pub data: Vec<u8>,
}

impl MemorySnapshot {
    /// Build a ROM view rooted at address zero.
    pub fn from_rom(data: &[u8]) -> Self {
        Self {
            region: MemoryRegion::Rom,
            base_addr: 0,
            data: data.to_vec(),
        }
    }

    /// Build a RAM view for one bank/chip, encoding the base address.
    pub fn from_ram(bank: u8, chip: u8, data: &[u8]) -> Self {
        let base = ((bank as u16) << 8) | ((chip as u16) << 6);
        Self {
            region: MemoryRegion::Ram,
            base_addr: base,
            data: data.to_vec(),
        }
    }

    /// Number of bytes in the view.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// True when the view carries no bytes.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Byte at `offset` from `base_addr`, if present.
    pub fn byte_at(&self, offset: usize) -> Option<u8> {
        self.data.get(offset).copied()
    }
}

/// Snapshot of CPU state for display.
#[derive(Clone, Debug, Default)]
pub struct CpuSnapshot {
    /// Index register file (16 for 4004, 24 for 4040).
    pub registers: Vec<u8>,
    /// 4-bit accumulator.
    pub accumulator: u8,
    /// Carry flag.
    pub carry: bool,
    /// 12-bit program counter.
    pub pc: u16,
    /// Call-stack entries.
    pub stack: Vec<u16>,
    /// Stack pointer.
    pub sp: u8,
    /// Halt state (4040).
    pub halted: bool,
    /// Interrupt-enable state (4040).
    pub interrupt_enabled: bool,
}

impl CpuSnapshot {
    /// Create a 4004-style snapshot, masking accumulator and PC to width.
    pub fn from_4004(regs: &[u8; 16], acc: u8, carry: bool, pc: u16) -> Self {
        Self {
            registers: regs.to_vec(),
            accumulator: acc & 0x0F,
            carry,
            pc: pc & 0x0FFF,
            stack: Vec::new(),
            sp: 0,
            halted: false,
            interrupt_enabled: false,
        }
    }

    /// Register count.
    pub fn register_count(&self) -> usize {
        self.registers.len()
    }
}

/// Snapshot of the call stack for display.
#[derive(Clone, Debug, Default)]
pub struct StackSnapshot {
    /// Stack entries (address values).
    pub entries: Vec<u16>,
    /// Current stack pointer.
    pub sp: u8,
}

impl StackSnapshot {
    /// Create a stack snapshot from a raw stack array and pointer.
    pub fn new(entries: &[u16], sp: u8) -> Self {
        Self {
            entries: entries.to_vec(),
            sp,
        }
    }

    /// Number of stack slots.
    pub fn depth(&self) -> usize {
        self.entries.len()
    }

    /// True if the stack has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the entry at a given level, if it exists.
    pub fn entry_at(&self, level: usize) -> Option<u16> {
        self.entries.get(level).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::{CpuSnapshot, MemoryRegion, MemorySnapshot, StackSnapshot};

    #[test]
    fn cpu_from_4004_masks_values() {
        let regs = [0xFFu8; 16];
        let snap = CpuSnapshot::from_4004(&regs, 0xFF, true, 0xFFFF);
        assert_eq!(snap.accumulator, 0x0F);
        assert_eq!(snap.pc, 0x0FFF);
    }

    #[test]
    fn memory_from_ram_base_address() {
        let snap = MemorySnapshot::from_ram(1, 2, &[0xAA; 4]);
        assert_eq!(snap.region, MemoryRegion::Ram);
        assert_eq!(snap.base_addr, (1 << 8) | (2 << 6));
        assert_eq!(snap.len(), 4);
    }

    #[test]
    fn stack_entry_at_out_of_bounds() {
        let snap = StackSnapshot::new(&[0x100], 1);
        assert_eq!(snap.entry_at(0), Some(0x100));
        assert_eq!(snap.entry_at(1), None);
        assert_eq!(snap.entry_at(99), None);
    }
}
