//! MCS-4 System Integration

pub mod cluster;
pub mod mcs4;
pub mod mcs40;

pub use cluster::Cluster;
pub use mcs4::Mcs4System;

#[cfg(test)]
mod wiring_tests {
    use std::io::Write;

    use bumpalo::Bump;
    use memmap2::MmapOptions;

    #[test]
    fn mmap_and_bumpalo_work() {
        let mut f = tempfile::tempfile().expect("tempfile");
        f.write_all(&[0u8; 64]).expect("write temp data");
        let mmap = unsafe { MmapOptions::new().len(64).map(&f).expect("mmap temp file") };
        assert_eq!(mmap.len(), 64);
        let bump = Bump::new();
        let s = bump.alloc_slice_fill_copy(10, 7u8);
        assert_eq!(s.len(), 10);
    }
}
