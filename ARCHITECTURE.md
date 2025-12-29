# MCS-4/MCS-40 Emulator Architecture

## Project Goals

Build a gate-level accurate emulator for the Intel MCS-4 (4004) and MCS-40 (4040) microcomputer systems with:
- **Gate-level simulation** with propagation delays
- **Transistor-level stubs** for future SPICE-style accuracy
- **Full system emulation** including all support chips
- **GUI debugger/programmer** with waveform visualization
- **FPGA synthesis path** via Rust HDL crates

## Accuracy Levels

```
Level 3: Transistor-level (TODO)
  - SPICE-like switch model per transistor
  - Parasitic R/C from layout
  - ~4-10 Hz effective clock

Level 2: Gate-level (PRIMARY TARGET)
  - NAND/NOR/INV with propagation delays
  - Wire delays from fanout estimation
  - ~1-100 kHz effective clock

Level 1: Cycle-accurate (BASELINE)
  - Phase-accurate (phi1/phi2) state machine
  - Instruction-correct behavior
  - Real-time or faster execution
```

## System Architecture

### MCS-4 Family (4004-based)
```
+--------+     4-bit data bus      +--------+
|  4004  |<----------------------->|  4001  | ROM (256x8) + I/O
|  CPU   |<----------------------->|  4002  | RAM (320-bit) + output
|        |<----------------------->|  4003  | Shift register (10-bit)
+--------+                         +--------+
    |
    +-- SYNC, CM-ROM, CM-RAM control signals
    +-- phi1, phi2 two-phase clock
```

### MCS-40 Family (4040-based)
```
+--------+     4-bit data bus      +--------+
|  4040  |<----------------------->|  4001  | ROM (256x8) + I/O
|  CPU   |<----------------------->|  4002  | RAM (320-bit) + output
|        |<----------------------->|  4308  | ROM (1Kx8)
|        |<----------------------->|  4101  | RAM (256x4)
+--------+                         +--------+
    |
    +-- 4289 Standard Memory Interface
    +-- 4201/4207/4209/4211 Clock generators
    +-- Interrupt/halt signals
```

## Rust Workspace Structure

```
mcs4-emu/
  Cargo.toml                    # Workspace root

  crates/
    mcs4-core/                  # Simulation kernel
      src/
        lib.rs
        timing.rs               # Propagation delay models
        gate.rs                 # Gate-level primitives
        transistor.rs           # Transistor-level stubs (TODO)
        wire.rs                 # Wire/net modeling

    mcs4-bus/                   # Bus infrastructure
      src/
        lib.rs
        data_bus.rs             # 4-bit bidirectional bus
        control.rs              # SYNC, CM-ROM, CM-RAM
        clock.rs                # Two-phase clock generation

    mcs4-chips/                 # Chip implementations
      src/
        lib.rs
        i4004/                  # 4004 CPU
          mod.rs
          alu.rs
          registers.rs
          instruction_decode.rs
          timing_io.rs
        i4040/                  # 4040 CPU (extends 4004)
          mod.rs
          interrupts.rs
        i4001/                  # ROM + I/O
        i4002/                  # RAM + output
        i4003/                  # Shift register
        i4101/                  # Static RAM
        i4201/                  # Clock generator
        i4289/                  # Memory interface
        i4308/                  # Larger ROM

    mcs4-system/                # Complete system assembly
      src/
        lib.rs
        mcs4.rs                 # MCS-4 system builder
        mcs40.rs                # MCS-40 system builder

    mcs4-gui/                   # GUI debugger
      src/
        main.rs
        waveform.rs             # Logic analyzer view
        registers.rs            # Register inspector
        memory.rs               # Memory view
        disasm.rs               # Disassembly view
        programmer.rs           # ROM programming interface

    mcs4-fpga/                  # FPGA synthesis support
      src/
        lib.rs
        verilog.rs              # Verilog export
        yosys.rs                # Yosys integration

  tests/
    integration/
      busicom.rs                # Busicom 141-PF calculator test
      instruction_set.rs        # Full ISA verification
```

## Core Abstractions

### Signal Types

```rust
/// Voltage level at a point in time
#[derive(Clone, Copy, Debug)]
pub enum SignalLevel {
    /// Logic low (Vdd = -15V for pMOS)
    Low,
    /// Logic high (Vss = 0V for pMOS)
    High,
    /// Undefined/floating
    Z,
    /// Contention (bus fight)
    X,
}

/// Time in picoseconds (1e-12 seconds)
pub type Time = u64;

/// A signal with history for waveform display
pub struct Signal {
    name: String,
    history: Vec<(Time, SignalLevel)>,
    current: SignalLevel,
}
```

### Gate-Level Primitives

```rust
pub trait Gate {
    /// Evaluate output given current inputs
    fn evaluate(&self) -> SignalLevel;

    /// Propagation delay from input change to output change
    fn propagation_delay(&self) -> Time;

    /// Input capacitance (for fanout calculations)
    fn input_capacitance(&self) -> f64;
}

pub struct Nand2 {
    a: SignalLevel,
    b: SignalLevel,
    tpd: Time,  // propagation delay
}

pub struct Nor2 {
    a: SignalLevel,
    b: SignalLevel,
    tpd: Time,
}

pub struct Inverter {
    input: SignalLevel,
    tpd: Time,
}
```

### Transistor-Level Stubs (TODO)

```rust
/// pMOS transistor model (future implementation)
pub trait Transistor {
    /// Update transistor state given terminal voltages
    fn update(&mut self, vg: f64, vs: f64, vd: f64);

    /// Drain-source current
    fn ids(&self) -> f64;

    /// On-resistance
    fn ron(&self) -> f64;

    /// Gate capacitance
    fn cg(&self) -> f64;
}

/// Placeholder for SPICE-like transistor model
pub struct PmosFet {
    // TODO: BSIM4 or simple switch model parameters
    w: f64,  // width
    l: f64,  // length
    vth: f64, // threshold voltage
}
```

### Bus Protocol

```rust
/// MCS-4 bus cycle states
#[derive(Clone, Copy, Debug)]
pub enum BusCycle {
    A1,  // Address phase 1 (bits 0-3)
    A2,  // Address phase 2 (bits 4-7)
    A3,  // Address phase 3 (bits 8-11)
    M1,  // Memory read phase 1 (bits 0-3)
    M2,  // Memory read phase 2 (bits 4-7)
    X1,  // Execute phase 1
    X2,  // Execute phase 2
    X3,  // Execute phase 3
}

/// Two-phase clock
pub struct Clock {
    phi1: Signal,
    phi2: Signal,
    period: Time,  // 1.35-2.0 us (740 kHz nominal)
}
```

## Timing Model

### Clock Specifications (from datasheet)
| Parameter | Min | Typ | Max | Unit |
|-----------|-----|-----|-----|------|
| tCY (period) | 1.35 | - | 2.0 | us |
| t0R (rise) | - | 50 | - | ns |
| t0F (fall) | - | 50 | - | ns |
| t0PW (width) | 380 | 480 | - | ns |
| t0D1 (phi1->phi2) | 400 | 550 | - | ns |
| t0D2 (phi2->phi1) | 150 | - | - | ns |

### Gate Delay Model

```rust
/// Simple linear delay model
pub fn gate_delay(gate_type: GateType, fanout: usize) -> Time {
    let base_delay = match gate_type {
        GateType::Nand2 => 5_000,   // 5 ns base
        GateType::Nor2 => 6_000,    // 6 ns base
        GateType::Inv => 3_000,     // 3 ns base
        GateType::Nand3 => 7_000,   // 7 ns base
        GateType::Nor3 => 8_000,    // 8 ns base
    };

    // Add delay per fanout (capacitive loading)
    let fanout_factor = 500;  // 0.5 ns per fanout
    base_delay + (fanout as Time * fanout_factor)
}
```

## Event-Driven Simulation

```rust
/// Simulation event
pub struct Event {
    time: Time,
    target: SignalId,
    value: SignalLevel,
}

/// Event queue for simulation
pub struct Simulator {
    current_time: Time,
    events: BinaryHeap<Reverse<Event>>,
    signals: HashMap<SignalId, Signal>,
    gates: Vec<Box<dyn Gate>>,
}

impl Simulator {
    pub fn step(&mut self) -> Option<Time> {
        if let Some(Reverse(event)) = self.events.pop() {
            self.current_time = event.time;
            self.apply_event(event);
            Some(self.current_time)
        } else {
            None
        }
    }

    fn apply_event(&mut self, event: Event) {
        // Update signal
        let signal = self.signals.get_mut(&event.target).unwrap();
        signal.update(event.time, event.value);

        // Propagate to dependent gates
        for gate_id in self.get_dependents(event.target) {
            let gate = &self.gates[gate_id];
            let new_value = gate.evaluate();
            let delay = gate.propagation_delay();

            self.events.push(Reverse(Event {
                time: self.current_time + delay,
                target: gate.output(),
                value: new_value,
            }));
        }
    }
}
```

## GUI Requirements

### Main Window Layout
```
+------------------+-------------------+
|   Registers      |    Memory View    |
|   (4004/4040)    |    (ROM/RAM)      |
+------------------+-------------------+
|           Waveform Display           |
|   (Logic Analyzer Style)             |
+--------------------------------------+
|           Disassembly View           |
+--------------------------------------+
|  Controls: Run | Step | Reset | Load |
+--------------------------------------+
```

### Key Features
1. **Waveform display**: Show phi1, phi2, SYNC, data bus, address, CM signals
2. **Register view**: All 16 index registers, accumulator, carry, PC stack
3. **Memory view**: ROM contents, RAM contents, I/O ports
4. **Disassembly**: Current instruction with highlighting
5. **Breakpoints**: By address, by signal condition
6. **ROM programmer**: Load binary/hex files, assemble source

## FPGA Synthesis Path

The gate-level design can be exported to Verilog for FPGA synthesis:

```rust
pub fn export_verilog(system: &System, path: &Path) -> io::Result<()> {
    let mut f = File::create(path)?;

    // Module header
    writeln!(f, "module mcs4_system(")?;
    writeln!(f, "  input wire clk,")?;
    writeln!(f, "  input wire rst,")?;
    writeln!(f, "  // ... ports")?;
    writeln!(f, ");")?;

    // Gate instantiations from design
    for gate in &system.gates {
        gate.emit_verilog(&mut f)?;
    }

    writeln!(f, "endmodule")?;
    Ok(())
}
```

## Validation Strategy

1. **Instruction tests**: Every 4004/4040 instruction in isolation
2. **Timing tests**: Verify bus cycle timing matches datasheet
3. **Integration test**: Run Busicom 141-PF calculator ROM
4. **Cross-validation**: Compare against 4004.com simulator output
5. **FPGA validation**: Synthesize and run on physical FPGA

## References

- Intel MCS-4 User Manual (Feb 1973)
- Intel MCS-40 User Manual (Nov 1974)
- Intel 4004/4040 Datasheets
- 4004.com transistor-level masks and schematics
- OpenCores MCS-4 Verilog implementation (reference only)
