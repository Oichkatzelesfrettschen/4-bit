//! Multi-System Cluster Support
//!
//! Enables connecting multiple MCS-4/MCS-40 systems together to form a cluster.
//! Supports signal propagation between systems with configurable latencies.

use rayon::prelude::*;

use crate::mcs4::Mcs4System;

/// ID for a system in the cluster
pub type SystemId = usize;

/// Port/Pin types that can be connected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortType {
    /// ROM I/O Port (0-15)
    RomPort(u8),
    /// RAM Output Port (0-3)
    RamPort(u8),
    /// CPU Test Pin
    TestPin,
    /// CPU Reset Pin
    ResetPin,
}

/// A connection between two systems
#[derive(Debug, Clone)]
pub struct Connection {
    pub src_sys: SystemId,
    pub src_port: PortType,
    pub dst_sys: SystemId,
    pub dst_port: PortType,
    /// Delay in machine cycles
    pub latency_cycles: u64,
}

/// A cluster of MCS-4 systems
pub struct Cluster {
    systems: Vec<Mcs4System>,
    connections: Vec<Connection>,
    /// Pending signals with their arrival time: (arrival_cycle, dst_sys, dst_port, value)
    pending_signals: Vec<(u64, SystemId, PortType, u8)>,
    cycle_count: u64,
}

impl Cluster {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            connections: Vec::new(),
            pending_signals: Vec::new(),
            cycle_count: 0,
        }
    }

    /// Add a system to the cluster, returning its ID
    pub fn add_system(&mut self, system: Mcs4System) -> SystemId {
        let id = self.systems.len();
        self.systems.push(system);
        id
    }

    /// Connect a port on one system to a port on another (immediate)
    pub fn connect(&mut self, src_sys: SystemId, src_port: PortType, dst_sys: SystemId, dst_port: PortType) {
        self.connect_with_latency(src_sys, src_port, dst_sys, dst_port, 0);
    }

    /// Connect a port on one system to a port on another with specified latency
    pub fn connect_with_latency(
        &mut self,
        src_sys: SystemId,
        src_port: PortType,
        dst_sys: SystemId,
        dst_port: PortType,
        latency: u64,
    ) {
        self.connections.push(Connection {
            src_sys,
            src_port,
            dst_sys,
            dst_port,
            latency_cycles: latency,
        });
    }

    /// Advance the entire cluster by one bus phase
    pub fn step(&mut self) {
        // 1. Collect new signals from sources
        for conn in &self.connections {
            let val = self.read_port(conn.src_sys, conn.src_port);
            // Schedule for future arrival
            self.pending_signals
                .push((self.cycle_count + conn.latency_cycles, conn.dst_sys, conn.dst_port, val));
        }

        // 2. Apply signals that have arrived
        let current_cycle = self.cycle_count;
        let mut i = 0;
        while i < self.pending_signals.len() {
            if self.pending_signals[i].0 <= current_cycle {
                let (_, sys_id, port, val) = self.pending_signals.remove(i);
                self.write_port(sys_id, port, val);
            } else {
                i += 1;
            }
        }

        // 3. Step all systems
        self.systems.par_iter_mut().for_each(|sys| {
            sys.step();
        });

        self.cycle_count += 1;
    }

    fn read_port(&self, sys_id: SystemId, port: PortType) -> u8 {
        if let Some(_sys) = self.systems.get(sys_id) {
            match port {
                PortType::RomPort(_id) => {
                    // TODO: implement read_rom_port in Mcs4System
                    0
                }
                PortType::RamPort(_chip) => {
                    // TODO: implement read_ram_port in Mcs4System
                    0
                }
                PortType::TestPin => 0,
                PortType::ResetPin => 0,
            }
        } else {
            0
        }
    }

    fn write_port(&mut self, sys_id: SystemId, port: PortType, val: u8) {
        if let Some(sys) = self.systems.get_mut(sys_id) {
            match port {
                PortType::TestPin => {
                    sys.set_test_pin(val != 0);
                }
                PortType::ResetPin => {
                    if val != 0 {
                        sys.reset();
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_step() {
        let mut cluster = Cluster::new();
        let s1 = Mcs4System::minimal();
        let s2 = Mcs4System::minimal();

        cluster.add_system(s1);
        cluster.add_system(s2);

        cluster.step();
    }
}
