# Transistor-Level Solver Architecture - Phase 4 Research

**Date**: 2026-01-29  
**Status**: Architecture Research Phase  
**Component**: Physics-Based Circuit Simulation  
**Priority**: Phase 4 Foundational  

## 1. OBJECTIVE

Implement a **transistor-level simulator** that can execute extracted MCS-4 netlists and produce results matching:
- Gate-level truth tables (perfect match)
- Timing behavior (±10% accuracy target)
- Power consumption (order-of-magnitude estimate)
- Electrostatic effects (hot-electron injection, substrate effects)

## 2. SIMULATION ALGORITHMS

### 2.1 Event-Driven Simulation (Preferred for Extracted Netlists)

**Algorithm Overview**:
```
Initialize:
  - Set all nodes to initial state (0 or floating)
  - Evaluate all gates using simple delay model
  - Queue pending events

Simulation loop:
  For each time step:
    1. Dequeue events in time order
    2. Update node voltages
    3. Evaluate affected gates
    4. Queue new events if node state changed
    5. Advance time by minimum event delay
```

**Advantages**:
- Only processes changing signals (sparse updates)
- Natural handling of delays
- Efficient for highly gated circuits
- Good fit for MCS-4 extracted netlists

**Disadvantages**:
- Complex event queue management
- Difficult to parallelize
- Limited analog behavior

### 2.2 Nodal Analysis (SPICE-Style)

**Algorithm Overview**:
```
For each time step:
  1. Build conductance matrix (G) from transistor on/off states
  2. Build current vector (I) from voltage sources
  3. Solve G*V = I (Kirchhoff's current law)
  4. Update node voltages (V)
  5. Update transistor states
  6. Advance time by fixed dt (Euler integration)
```

**Advantages**:
- Accurate analog behavior
- Handles capacitive coupling
- Precedent from SPICE

**Disadvantages**:
- Dense matrix solve required
- Slow for large circuits
- Requires transistor compact models

### 2.3 Hybrid Approach (Recommended)

**Strategy**:
- Use **event-driven** for logic transitions (fast)
- Use **nodal analysis** for critical timing paths (accurate)
- Adaptive switching between modes

**Implementation**:
```
For each event:
  If critical path (ROM access, RAM timing):
    Use nodal analysis (40 time steps, 1ns each)
  Else:
    Use event-driven with fixed delay (±10% model)
```

## 3. TRANSISTOR MODELS

### 3.1 Minimum Model (Switch-Level)

**Switch Parameters**:
- Vth: Threshold voltage (~1V for 10µm NMOS)
- Ron: On-state resistance (~10kΩ for 10µm NMOS)
- Tfall: Fall time (20-50ns)
- Trise: Rise time (20-50ns)

**Gate Drive Logic**:
```
If Vgs > Vth:
  Conduct with resistance Ron
  Propagate voltage drop
Else:
  High impedance (open circuit)
```

**RC Delay Calculation**:
```
τ = R * C
  R = Ron (transistor channel resistance)
  C = Cload (wire + gate capacitance)
  
Delay ≈ 0.69 * τ (63% settling for RC rise time)
Or 2.2 * τ for 90% settling
```

### 3.2 Intermediate Model (Switch + Substrate)

**Additional Parameters**:
- Csub: Substrate coupling capacitance
- Vfb: Flat-band voltage
- Cgs: Gate-source capacitance
- Cgd: Gate-drain capacitance (Miller effect)

**Equation**:
```
Ids = (W/L) * μ * Cox * ((Vgs - Vth) * Vds - Vds²/2)  [Triode region]
Ids = (W/L) * μ * Cox * (Vgs - Vth)² / 2              [Saturation]
```

### 3.3 Full BSIM4 Model (Not Recommended Initially)

**Parameters**: 300+ (industry standard)
- Process corners
- Temperature effects
- Geometry-dependent parameters
- Short-channel effects
- Statistical variation

**Overhead**: 100× slower than switch model, 10× slower than intermediate

## 4. ARCHITECTURE DESIGN

### 4.1 Core Data Structures

```rust
/// Transistor instance
pub struct Transistor {
    id: TransistorId,
    type_: TransistorType,  // NMOS or PMOS
    gate_node: NodeId,
    drain_node: NodeId,
    source_node: NodeId,
    bulk_node: NodeId,
    w: f64,        // Width
    l: f64,        // Length
    vth: f64,      // Threshold voltage
    ron: f64,      // On-state resistance
    delay: f64,    // Propagation delay
}

/// Network node (connection point)
pub struct Node {
    id: NodeId,
    name: String,
    voltage: f64,           // Current voltage
    voltage_prev: f64,      // Previous voltage (for edge detection)
    capacitance: f64,       // Total node capacitance
    is_driven: bool,        // Connected to current source?
    connected_transistors: Vec<TransistorId>,  // Gate/drain/source connections
}

/// Event: node state change
pub struct Event {
    time: f64,
    node_id: NodeId,
    new_voltage: f64,
    priority: EventPriority,  // Critical path vs normal
}

/// Circuit netlist
pub struct Circuit {
    transistors: HashMap<TransistorId, Transistor>,
    nodes: HashMap<NodeId, Node>,
    power_rails: (NodeId, NodeId),  // VDD, VSS node IDs
}
```

### 4.2 Simulation Engine

```rust
pub struct TransistorSimulator {
    circuit: Circuit,
    event_queue: BinaryHeap<Event>,  // Min-heap by time
    current_time: f64,
    dt: f64,                         // Time step
    max_events: usize,               // Safety limit
    convergence_epsilon: f64,        // Voltage change threshold
}

impl TransistorSimulator {
    pub fn new(circuit: Circuit) -> Self { ... }
    
    // Main simulation loop
    pub fn step(&mut self) -> Result<bool> {
        // Returns true if events remain, false if settled
    }
    
    // Evaluate transistor state
    fn evaluate_transistor(&mut self, t: &Transistor) -> bool {
        // Returns true if transistor state changed
    }
    
    // Update node voltage from transistor network
    fn update_node_voltage(&mut self, node_id: NodeId) -> bool {
        // Returns true if voltage changed significantly
    }
    
    // Queue events for next evaluation
    fn queue_events(&mut self, node_id: NodeId, new_voltage: f64) {
        // Add to event queue with appropriate delay
    }
}
```

### 4.3 Integration with Netlist Extractor

```
Extracted Netlist (netlist_v1.json)
    ↓
Parse transistor and node definitions
    ↓
Build Circuit struct with all transistors
    ↓
Identify power rails (VDD, VSS)
    ↓
Load initial conditions (reset state)
    ↓
Create TransistorSimulator instance
    ↓
Execute simulation steps
    ↓
Compare outputs with expected results
```

## 5. IMPLEMENTATION PHASES

### Phase 4A: Switch-Level Simulator (2-3 sprints)

**Goals**:
- Implement minimum model transistors
- Event-driven simulation kernel
- Basic RC delay model
- Validation against gate-level truth tables

**Deliverables**:
- transistor_solver.rs module (500-700 lines)
- 10+ unit tests (inverter, NAND, NOR, latch)
- Comparison against gate-level baseline

**Success Criteria**:
- All gate truth tables match perfectly
- Timing accurate within ±20%
- Execution time <1 second per gate

### Phase 4B: Nodal Analysis (1-2 sprints)

**Goals**:
- Implement sparse matrix solver (Gaussian elimination)
- RC-coupled node voltage updates
- Capacitive coupling between nodes

**Deliverables**:
- Nodal analysis module
- Matrix equation solver
- 5+ analog tests (RC charging, cross-coupling)

**Success Criteria**:
- Timing accurate within ±5%
- Handles capacitive effects correctly
- Execution time <10 seconds per critical path

### Phase 4C: Validation & Optimization (1 sprint)

**Goals**:
- Compare against SPICE simulations (LTspice reference)
- Profile and optimize hot paths
- Develop timing corner models

**Deliverables**:
- Comparison report (vs LTspice)
- Optimization recommendations
- Corner models (fast, slow, typical)

## 6. TIMING SPECIFICATIONS

### 6.1 MCS-4 Critical Path Delays

From extracted netlists (4002 RAM):
- Word line decoder: 50-100ns
- Bit line precharge: 10-20ns
- Bit line develop: 30-60ns (write case slower than read)
- Sense amplifier: 20-40ns
- Output driver: 20-50ns
- **Total**: ~150-250ns for read path

### 6.2 Model Accuracy Targets

| Metric | Target | Tolerance |
|--------|--------|-----------|
| Logic delay | 100ns | ±20% (80-120ns) |
| Rise time | 20ns | ±15% (17-23ns) |
| Fall time | 25ns | ±15% (21-28ns) |
| Setup time | 50ns | ±10% |
| Hold time | 10ns | ±20% |

## 7. VALIDATION STRATEGY

### 7.1 Unit Tests

1. **Inverter**:
   - Input 0 → Output 1 (within 50ns)
   - Input 1 → Output 0 (within 50ns)

2. **NAND Gate**:
   - 2-input NAND truth table
   - Timing for each transition

3. **SR Latch**:
   - Set operation (S=1, R=0)
   - Reset operation (S=0, R=1)
   - Hold operation (S=0, R=0)
   - Invalid state (S=1, R=1)

4. **Differential Pair** (for sense amp):
   - Input offset voltage < 100mV
   - Gain > 100V/V
   - Settling time < 30ns

### 7.2 Integration Tests

1. **6T RAM Cell**:
   - Write data → verify storage
   - Read data → verify output
   - Timing compliance

2. **Word Line Decoder**:
   - All 256 outputs unique
   - No race conditions

3. **4002 RAM Subcircuit**:
   - Full read cycle
   - Full write cycle
   - Timing vs spec

### 7.3 Regression Tests

- Compare against previous versions
- Check for non-deterministic behavior
- Profile execution time

## 8. PERFORMANCE TARGETS

| Benchmark | Target | Notes |
|-----------|--------|-------|
| Inverter | <1ms | 10 cycles |
| 4T gate | <10ms | 100 cycles |
| 6T cell | <100ms | 50 read + write cycles |
| 4002 subcircuit | <10s | 1000 access cycles |
| Full 4001/4002 | <100s | Complete 4-chip simulation |

## 9. EXTERNAL DEPENDENCIES

### 9.1 Linear Algebra (Matrix Solve)

**Options**:
- **nalgebra**: 5KB sparse matrix, full featured
- **ndarray**: 10KB dense array, good for small matrices
- **scipy equivalent**: Roll-our-own Gaussian elimination
- **Recommended**: nalgebra for sparse support

### 9.2 Visualization

- **plotly**: Interactive waveform plots
- **gnuplot**: Legacy but widely supported
- **Recommended**: Built-in trace buffer export as CSV → matplotlib

### 9.3 Reference Implementation

- **ngspice**: For comparison validation
- **LTspice**: Commercial but free for Linux (Wine)
- **Cider**: Research simulator (academic)

## 10. DESIGN DECISIONS

### Decision 1: Simulation Granularity

**Option A (Gate-level)**: Model gates as atomic operations
- Pros: Fast, sufficient for logic correctness
- Cons: Misses timing interactions

**Option B (Transistor-level)**: Model every transistor
- Pros: Physical accuracy, timing details
- Cons: Slow, complex

**Chosen**: Transistor-level (Option B)
**Rationale**: Phase 4 objective is to validate physical extraction

### Decision 2: Delay Model

**Option A (Zero-delay)**: Instantaneous propagation
- Pros: Fast
- Cons: No timing validation

**Option B (Fixed delays)**: 30ns per gate transition
- Pros: Simple, adequate for logic
- Cons: Inaccurate for analysis

**Option C (RC delays)**: Calculated from circuit parameters
- Pros: Accurate, physically based
- Cons: Requires transistor parameter extraction

**Chosen**: Option C (RC delays)
**Rationale**: Enables timing analysis of extracted circuits

### Decision 3: Convergence

**Option A (Fixed iterations)**: 100 time steps max
- Pros: Bounded execution time
- Cons: May not settle

**Option B (Voltage threshold)**: Iterate until ΔV < 1mV
- Pros: Accurate convergence
- Cons: Variable execution time

**Option C (Hybrid)**: Max 100 steps OR ΔV < 1mV
- Pros: Balanced accuracy + speed
- Cons: Complex logic

**Chosen**: Option C (Hybrid)
**Rationale**: Ensures both convergence and bounded time

## 11. REFERENCES

- Rabaey, Pedram: "Digital Integrated Circuits" (Ch. 5-7: Transistor models)
- Shichman & Hodges: "Modeling and Simulation of INSULATED GATE FETs" (Classic SPICE model)
- Berkeley SPICE3 Documentation (Nodal analysis algorithm)
- NGSPICE Manual (Reference implementation)
- Chandrakasan et al.: "Digital Integrated Circuits" (Low-power design, timing)

---

**Created**: 2026-01-29  
**Author**: Claude Haiku 4.5  
**Status**: Research Architecture Complete - Ready for Implementation
