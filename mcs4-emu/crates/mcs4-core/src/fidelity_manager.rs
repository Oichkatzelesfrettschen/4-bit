//! Fidelity Manager for multi-resolution simulation
//!
//! Orchestrates the switching between behavioral, switch-level,
//! and analog nodal solvers for system components.

use crate::fidelity::SimulationFidelity;
use crate::nodal_solver::NodalSolver;
use crate::tcad::TCADBridge;
use crate::process::ProcessParams;
use crate::circuit::graph::CircuitGraph;
use std::collections::HashMap;

use crate::bridge::{PinDirection, PinMapping};

/// Manages the simulation fidelity of various system components.
pub struct FidelityManager {
    /// Desired fidelity for each named component.
    component_fidelity: HashMap<String, SimulationFidelity>,
    
    /// Active nodal solvers for components running at Nodal or TCAD level.
    nodal_backends: HashMap<String, NodalSolver>,
    
    /// Mapping of pins for each component
    pin_mappings: HashMap<String, Vec<PinMapping>>,
    
    /// Global process parameters for physics-driven simulation.
    process: ProcessParams,
}

impl FidelityManager {
    /// Create a new manager with default process parameters.
    pub fn new(process: ProcessParams) -> Self {
        Self {
            component_fidelity: HashMap::new(),
            nodal_backends: HashMap::new(),
            pin_mappings: HashMap::new(),
            process,
        }
    }

    /// Set the simulation fidelity for a named component.
    pub fn set_fidelity(&mut self, name: &str, level: SimulationFidelity) {
        self.component_fidelity.insert(name.to_string(), level);
    }

    /// Get the current fidelity level for a component.
    pub fn get_fidelity(&self, name: &str) -> SimulationFidelity {
        self.component_fidelity.get(name).cloned().unwrap_or_default()
    }

    /// Register pin mappings for a component.
    pub fn register_pins(&mut self, name: &str, pins: Vec<PinMapping>) {
        self.pin_mappings.insert(name.to_string(), pins);
    }

    /// Initialize an analog nodal backend for a component using its circuit graph.
    pub fn init_nodal_backend(&mut self, name: &str, graph: &CircuitGraph) {
        let mut solver = NodalSolver::new();
        
        let level = self.get_fidelity(name);
        if level == SimulationFidelity::TCADLevel {
            solver.set_bridge(TCADBridge::new(&self.process));
        }
        
        solver.load_circuit_graph(graph);
        
        // Pre-add voltage sources for all input pins to allow dynamic updating
        if let Some(pins) = self.pin_mappings.get(name) {
            for pin in pins {
                if pin.direction != PinDirection::Output {
                    solver.add_voltage_source(pin.node_id, solver.get_gnd(), 0.0);
                }
            }
        }

        self.nodal_backends.insert(name.to_string(), solver);
    }

    /// Synchronize digital states to analog voltages.
    pub fn sync_digital_to_analog(&mut self, name: &str, bus_value: u8, vdd: f64, vss: f64) {
        if let (Some(solver), Some(pins)) = (self.nodal_backends.get_mut(name), self.pin_mappings.get(name)) {
            for pin in pins {
                if pin.direction != PinDirection::Output {
                    let voltage = if pin.name.starts_with('D') {
                        let bit_idx: u8 = pin.name[1..].parse().unwrap_or(0);
                        if (bus_value & (1 << bit_idx)) != 0 { vss } else { vdd }
                    } else {
                        vss // Default to inactive/high
                    };
                    solver.update_voltage_source(pin.node_id, solver.get_gnd(), voltage);
                }
            }
        }
    }

    /// Synchronize analog voltages back to digital states.
    pub fn sync_analog_to_digital(&self, name: &str, vdd: f64, vss: f64) -> u8 {
        let mut bus_out = 0u8;
        if let (Some(solver), Some(pins)) = (self.nodal_backends.get(name), self.pin_mappings.get(name)) {
            for pin in pins {
                if pin.direction != PinDirection::Input {
                    if pin.name.starts_with('D') {
                        let bit_idx: u8 = pin.name[1..].parse().unwrap_or(0);
                        let v = solver.voltage(pin.node_id);
                        // pMOS logic: nearer to VSS (0V) is High (1), nearer to VDD (-15V) is Low (0)
                        let threshold = (vdd + vss) / 2.0;
                        if v > threshold {
                            bus_out |= 1 << bit_idx;
                        }
                    }
                }
            }
        }
        bus_out
    }

    /// Get the nodal solver for a component, if it exists.
    pub fn get_solver(&mut self, name: &str) -> Option<&mut NodalSolver> {
        self.nodal_backends.get_mut(name)
    }

    /// Get a read-only reference to the nodal solver for a component.
    pub fn get_solver_ref(&self, name: &str) -> Option<&NodalSolver> {
        self.nodal_backends.get(name)
    }

    /// Advance simulation for all components running at analog levels.
    pub fn step_analog(&mut self, dt: f64) {
        for (name, solver) in &mut self.nodal_backends {
            let level = self.component_fidelity.get(name).cloned().unwrap_or_default();
            if level >= SimulationFidelity::NodalLevel {
                solver.set_dt(dt);
                solver.solve_robust();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fidelity_registry() {
        let mut mgr = FidelityManager::new(ProcessParams::default());
        mgr.set_fidelity("CPU", SimulationFidelity::TCADLevel);
        mgr.set_fidelity("RAM", SimulationFidelity::Behavioral);

        assert_eq!(mgr.get_fidelity("CPU"), SimulationFidelity::TCADLevel);
        assert_eq!(mgr.get_fidelity("RAM"), SimulationFidelity::Behavioral);
        assert_eq!(mgr.get_fidelity("ROM"), SimulationFidelity::Behavioral); // Default
    }
}
