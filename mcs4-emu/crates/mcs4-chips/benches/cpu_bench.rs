#![allow(missing_docs)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mcs4_bus::{ControlSignals, DataBus};
use mcs4_chips::i4004::I4004;

fn bench_4004_tick(c: &mut Criterion) {
    let mut cpu = I4004::new();
    let mut bus = DataBus::new();
    let mut ctrl = ControlSignals::mcs4();

    c.bench_function("4004_tick_1k_phases", |b| {
        b.iter(|| {
            for _ in 0..black_box(1000u32) {
                cpu.tick(mcs4_bus::BusCycle::A1, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::A2, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::A3, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::M1, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::M2, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::X1, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::X2, &mut bus, &mut ctrl);
                cpu.tick(mcs4_bus::BusCycle::X3, &mut bus, &mut ctrl);
            }
        })
    });
}

criterion_group!(benches, bench_4004_tick);
criterion_main!(benches);
