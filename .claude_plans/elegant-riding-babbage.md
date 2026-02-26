# Intel MCS-4/MCS-40 Transistor-Level Emulator: Comprehensive Rescoping and Modernization Plan

**Project**: 4-bit Intel MCS-4/MCS-40 cycle-accurate emulator with transistor-level extraction
**Current State**: Phase 0.5 completion (50-60% overall), Phase 1 ready to begin
**Date**: 2026-01-29
**Agent IDs**: ae2491e (emulator), a1170b6 (OCR), aec953a (extraction)

---

## EXECUTIVE SUMMARY

This project aims to build a **transistor-level cycle-accurate emulator** of the Intel MCS-4 (4004 CPU) and MCS-40 (4040 CPU) chipsets, with **full netlist extraction** from die photos and schematics, **modernized OCR pipelines** for documentation processing, and **clustering infrastructure** for parallel simulation.

**Current Achievement**:
- **Emulator Core**: 50-60% complete (4004 fully functional, 4040 partial, GUI stub)
- **Extraction Pipeline**: netlist_v0/v1 for all 4 chips (4001/4002/4003/4004)
- **OCR Infrastructure**: multi-tier fallback (Tesseract, ONNX, templates) with benchmarking
- **Test Coverage**: 115 tests passing, all clippy checks clean

**Critical Path Forward**:
1. Complete 4040 CPU (14 new opcodes, interrupt handling)
2. Modernize OCR for higher accuracy (ensemble methods, ONNX training, caching)
3. Build transistor-level simulation from extracted netlists
4. Complete GUI debugger with waveform viewer
5. Implement clustering for parallel simulation

---

## PART 1: CURRENT STATE AUDIT

### 1.1 Emulator Implementation Status

**COMPLETED COMPONENTS**:

| Component | Status | Coverage | Notes |
|-----------|--------|----------|-------|
| 4004 CPU | ✅ COMPLETE | 100% | All 46 instructions, ALU, registers, 3-level stack |
| 4001 ROM | ✅ COMPLETE | 100% | 256×8 ROM, 4-bit I/O, chip select, bus protocol |
| 4002 RAM | ✅ COMPLETE | 100% | 320-bit RAM, 4-bit output, SRC addressing |
| 4003 Shift Register | ⚠️ STUB | 30% | Basic logic exists, needs bus integration |
| mcs4-system | ✅ COMPLETE | 100% | Full system integration, fixture runner |
| Test Infrastructure | ✅ ROBUST | 100% | 115 tests passing, 66 unit + 41 integration |

**PARTIAL/STUB COMPONENTS**:

| Component | Status | Coverage | Blockers |
|-----------|--------|----------|----------|
| 4040 CPU | ⚠️ PARTIAL | 65% | 14 new opcodes missing, interrupt handling incomplete |
| Disassembler | ⚠️ PARTIAL | 60% | Core exists, needs 4040 support + symbol tables |
| GUI Waveform | ⚠️ FUNCTIONAL | 80% | Rendering works, needs integration with debugger |
| GUI Debugger | ⚠️ STUB | 20% | Register/memory/stack panels stubbed |

**NOT STARTED**:
- MCS-40 support chips: 4101 RAM, 4201 Clock, 4289 Interface, 4308 ROM
- TTL/Bus chips: 3216/3226 drivers, 3205 decoder, 74-series
- Era-appropriate peripherals: 7-seg displays, Nixie tubes, serial UART

### 1.2 Chip Extraction Status

**NETLIST EXTRACTION COMPLETE**:

| Chip | Nodes | Transistors (v0) | Transistors (v1) | Anchored Signals | Subcircuits |
|------|-------|------------------|------------------|------------------|-------------|
| 4001 | 5744 | 2000 | 1999 | 14 | 11 (max 117T) |
| 4002 | 3280 | 640 | 639 | 14 | 6 (max 42T) |
| 4003 | 490 | 38 | 37 | 14 | 5 (max 9T) |
| 4004 | 3448 | 1031 | 1030 | 19 | 11 (max 101T) |

**KEY ACHIEVEMENTS**:
- ✅ All chips have netlist_v0 (layout-only) and netlist_v1 (schematic+layout)
- ✅ All anchors have nonzero transistor incidence (0 failures)
- ✅ Power rails (VSS/VDD) medium-confidence for 4001/4002/4003 (geometry-corroborated)
- ✅ Subcircuits extracted for all anchor nodes (BFS radius=3)
- ✅ CI schematic pipeline passing (audit, incidence, uniqueness checks)

**OPEN LACUNAE**:
- ❌ No schematic↔layout coordinate transforms (homography/affine mappings missing)
- ❌ Pad label OCR ambiguity (short tokens like "S", "T", "R3" require manual validation)
- ❌ Via connectivity not modeled (assumes transistor A-B nodes are electrically connected)
- ❌ No parasitic extraction (no RC delay models)
- ❌ Gate-level timing models missing (transistors are "perfect switches")

### 1.3 OCR Pipeline Status

**ARCHITECTURE**:
- Multi-tier fallback: ONNX CTC (GPU) → Tesseract CLI (fast) → Tesseract sweep (multi-PSM) → Template matching → Hu moments
- Preprocessing: CLAHE, morphology, adaptive thresholding, 3-7× upscaling
- Backends: 6 configurations (edge_label, digits_tiny, glyph_single, outline_fill, edge_label_light, custom)

**ACCURACY METRICS**:
- ✅ Benchmarking framework exists (`ocr_benchmark_v0.py`)
- ✅ Micro-benchmarks for 4004 edge labels (10-30 fixtures)
- ⚠️ Limited regression suite (only 4004 covered, not 4001/4002/4003)
- ❌ No comprehensive cross-validation against external sources

**IDENTIFIED GAPS**:
1. **Accuracy**: Single-glyph errors (O↔0, I↔1, l↔1), multi-char degradation
2. **Coverage**: 4001/4002/4003 pad labels require manual verification
3. **Performance**: No persistent cache (redundant Tesseract invocations)
4. **ONNX**: No production-ready model (optional external model only)
5. **Validation**: No systematic cross-check against datasheet pinouts

---

## PART 2: MODERNIZATION PLAN

### 2.1 OCR Pipeline Modernization

**TIER 1: HIGH-IMPACT (1-2 sprints)**

**M1.1: Persistent OCR Cache**
- **What**: Disk-backed cache for OCR results keyed by (image_hash, config_hash)
- **Why**: Eliminate redundant Tesseract invocations (3-10× speedup)
- **How**:
  - Implement `scripts/ocr_cache_v0.py` with SQLite backend
  - Hash inputs: (image_sha256, preset, psm, threshold_mode)
  - Store: (token, confidence, backend_used, timestamp)
  - Integrate into `ocr_backend_v0.py` before backend invocation
- **Verification**: Re-run full extraction, verify bit-identical outputs, measure speedup

**M1.2: Comprehensive Regression Benchmark**
- **What**: Expand benchmark coverage to all chips, all pad labels
- **Why**: Current suite only covers 4004 edge labels (10-30 samples)
- **How**:
  - Add benchmark sets: `4001_pad_labels.json`, `4002_pad_labels.json`, `4003_pad_labels.json`
  - Include edge labels for 4001/4002/4003 (currently missing)
  - Target: 100-200 labeled crops per chip
  - Validate against PRIMARY_SOURCE_PINOUTS.md
- **Verification**: `python3 scripts/ocr_benchmark_v0.py --all` with accuracy >95%

**M1.3: CI Environment Pinning**
- **What**: Lock exact Tesseract, ONNX, OpenCV versions
- **Why**: Prevent toolchain drift that degrades accuracy
- **How**:
  - Document exact versions in `docs/TOOLING_AUDIT.md`
  - Add CI gate: `scripts/check_ocr_versions.sh` (fail if mismatch)
  - Snapshot current tooling: `scripts/tooling_audit.sh > docs/TOOLING_AUDIT_$(date +%Y%m%d).md`
  - Re-run extraction after upgrades, compare outputs
- **Verification**: CI fails on version mismatch, extraction outputs reproducible

**TIER 2: MEDIUM-IMPACT (2-4 sprints)**

**M2.1: Multi-Modal OCR Fusion**
- **What**: Ensemble voting with learned weights (XGBoost or logistic regression)
- **Why**: Single backend errors can be corrected by ensemble consensus
- **How**:
  - Collect 50-100 diverse labeled crops with ground truth
  - Extract features: (tesseract_conf, template_score, hu_distance, glyph_area, aspect_ratio)
  - Train binary classifier: is_correct(token, features)
  - Weight backends by learned confidence
- **Verification**: A/B test on regression suite, target +5% accuracy

**M2.2: Adaptive Preprocessing Per-Crop**
- **What**: Use image statistics to select preset (contrast, edge density, ink %)
- **Why**: Current heuristic is coarse (max_len + whitelist only)
- **How**:
  - Implement `scripts/select_preset_adaptive_v0.py`
  - Features: (mean_intensity, contrast, edge_density, fill_ratio)
  - Decision tree: if contrast < 50 → CLAHE; if edge_dense → minimal_morph; etc.
- **Verification**: A/B test on regression suite, measure precision improvement

**M2.3: Schematic↔Layout Cross-Validation**
- **What**: Automatically verify OCR'd pin labels match schematic connectivity
- **Why**: Catch OCR errors by checking electrical consistency
- **How**:
  - Implement `scripts/validate_pin_connectivity_v0.py`
  - For each pin label: trace layout node → schematic net → expected pin number
  - Flag disconnected signals (likely OCR errors)
  - Generate report: (signal, ocr_token, expected_pin, actual_connectivity)
- **Verification**: Run on all chips, identify and fix disconnects

**TIER 3: RESEARCH/LONG-TERM (4+ sprints)**

**M3.1: Custom ONNX CTC Training Pipeline**
- **What**: Train lightweight (<5MB) CTC model on chip-specific glyphs
- **Why**: Generic Tesseract underperforms on thin-stroke PMOS-era fonts
- **How**:
  - Collect 200+ diverse crops from all chips (4001/4002/4003/4004)
  - Annotate ground truth (use existing manual readings as seed)
  - Train CTC model: Conv layers + LSTM + CTC loss
  - Export to ONNX, integrate as preferred backend
- **Verification**: Measure accuracy on held-out test set, target >98%

**M3.2: Layout Feature Extraction (CNN)**
- **What**: Use pre-trained CNN to predict OCR success probability
- **Why**: Avoid wasting time on low-quality crops
- **How**:
  - Use ResNet18 backbone, fine-tune on crop quality dataset
  - Predict: (good_crop, blurry, low_contrast, occluded)
  - Skip OCR on predicted-bad crops, flag for manual review
- **Verification**: Reduce false positives by 20% on regression suite

### 2.2 Emulator Completion Plan

**TIER 1: CRITICAL PATH (Phase 1)**

**E1.1: Complete 4040 CPU**

**Files to Create/Modify**:
```
mcs4-emu/crates/mcs4-chips/src/i4040/
├── mod.rs                 # Main CPU struct, tick(), execute()
├── registers.rs           # 24-register file with bank switching (DONE)
├── stack.rs              # 7-level stack with overflow checks (STARTED)
├── alu.rs                # Extended ALU (OR4/5, AN6/7 operations)
├── instruction_decode.rs # Extended decoder (60 instructions)
├── interrupt.rs          # Interrupt controller state machine
└── tests.rs              # Unit tests for new instructions
```

**Implementation Steps**:
1. **Register Bank Switching** (✅ DONE):
   - `regs: [u8; 24]` (R0-R23)
   - `bank: u8` (0 or 1)
   - `get_r()` / `set_r()` apply bank offset for R0-R7 when bank=1 (→ R16-R23)
   - R8-R15 always accessible

2. **7-Level Stack** (⚠️ STARTED):
   - Extend from 3 to 7 levels
   - Add overflow/underflow checks
   - Unit tests for edge cases (7 nested calls, stack wrap)

3. **Interrupt Controller**:
   - Add fields: `int_enabled: bool`, `int_pending: bool`, `src_save: u8`, `halted: bool`
   - Implement INT pin handling in `tick()`:
     - Check INT pin && int_enabled
     - Complete current instruction
     - Push PC to stack, save SRC to src_save
     - Disable interrupts (auto), vector to 0x003

4. **New Instructions** (14 opcodes):
   - 0x01 HLT: `halted = true; stop execution`
   - 0x02 BBS: `pc = stack.pop(); src = src_save; int_enabled = true;`
   - 0x03 LCR: `ram[char] = rom[pc]; advance ROM address`
   - 0x04 OR4: `acc |= regs[4]`
   - 0x05 OR5: `acc |= regs[5]`
   - 0x06 AN6: `acc &= regs[6]`
   - 0x07 AN7: `acc &= regs[7]`
   - 0x08 DB0: `bank = 0`
   - 0x09 DB1: `bank = 1`
   - 0x0A SB0: `ram_bank = 0`
   - 0x0B SB1: `ram_bank = 1`
   - 0x0C EIN: `int_enabled = true`
   - 0x0D DIN: `int_enabled = false`
   - 0x0E RPM: `acc = rom[pc]; advance ROM address`

5. **Testing**:
   - Unit tests for each new instruction
   - Interrupt vector test (INT pin → 0x003)
   - BBS restore test (SRC saved/restored correctly)
   - Backward compatibility test (all 46 4004 instructions still work)
   - Stack depth test (7 nested JMS calls)

**Verification**:
- All 115 existing tests still pass
- 20+ new tests for 4040 features
- Clippy clean with `-D warnings`
- No unsafe Rust

**E1.2: Complete Disassembler**

**Files to Create**:
```
mcs4-emu/crates/mcs4-chips/src/
├── disasm.rs          # Core disassembler module
└── disasm/
    ├── mod.rs         # Module exports
    ├── format.rs      # Output formatting (assembly listing)
    ├── symbols.rs     # Symbol table management
    └── tests.rs       # Unit tests
```

**API Design**:
```rust
pub struct Disassembler {
    cpu_type: CpuType,  // I4004 or I4040
    symbols: HashMap<u16, String>,
    comments: HashMap<u16, String>,
}

pub struct DisasmLine {
    pub address: u16,
    pub bytes: Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
    pub comment: Option<String>,
    pub is_jump_target: bool,
}

impl Disassembler {
    pub fn disasm_one(&self, rom: &[u8], addr: u16) -> DisasmLine;
    pub fn disasm_range(&self, rom: &[u8], start: u16, end: u16) -> Vec<DisasmLine>;
    pub fn format_listing(&self, lines: &[DisasmLine]) -> String;
    pub fn auto_label(&mut self, rom: &[u8]);  // Find jump targets
}
```

**Implementation Steps**:
1. Reuse existing `InstructionDecoder` from i4004/i4040
2. Add operand formatting for each instruction type
3. Implement `auto_label()` to scan for JUN/JMS/JCN targets
4. Create listing formatter (address, bytes, mnemonic, operands, comment)
5. Add 4040 opcode support (14 new instructions)
6. Integration with GUI disasm panel

**Verification**:
- All 46 4004 instructions disassemble correctly
- All 14 4040 instructions disassemble correctly
- Round-trip test: disasm → reassemble → byte-identical
- Auto-labeling finds all jump targets

**E1.3: Complete GUI Debugger**

**Files to Create/Modify**:
```
mcs4-emu/crates/mcs4-gui/src/
├── app.rs            # Main app with debugger state
├── debugger.rs       # Debugger controller (NEW)
├── breakpoints.rs    # Breakpoint management (NEW)
├── panels/
│   ├── cpu_state.rs  # Enhanced register view
│   ├── memory.rs     # Enhanced with hex editor
│   ├── disasm.rs     # Disassembly panel (NEW)
│   ├── stack.rs      # Stack view (NEW)
│   ├── waveform.rs   # Waveform display (EXISTS, needs integration)
│   └── breakpoints.rs # Breakpoint list panel (NEW)
└── shortcuts.rs      # Keyboard handler (NEW)
```

**Features to Implement**:
1. **Register Panel**: Edit registers by clicking, show register pairs
2. **Memory Panel**: Hex viewer/editor for ROM (read-only) and RAM (editable)
3. **Disassembly Panel**: Current instruction highlighted, click to set breakpoint
4. **Stack Panel**: Show all stack levels (3 for 4004, 7 for 4040)
5. **Breakpoint Manager**: List breakpoints, enable/disable, set on PC/memory/register
6. **Execution Control**: Run/Stop, Step (phase/cycle/instruction), Run N cycles
7. **Keyboard Shortcuts**: F5=Run, F6=Stop, F7=Step, F9=Toggle BP, Ctrl+G=Goto

**Verification**:
- Step instruction works correctly
- Breakpoints halt at correct address
- Register/memory edits take effect
- Keyboard shortcuts functional
- Waveform synchronized with execution

**TIER 2: SUPPORT CHIPS (Phase 2)**

**E2.1: Implement 4101 RAM (256×4 static RAM)**
- Full bus protocol implementation
- Address decoding, read/write timing
- Chip enable logic
- Unit tests + integration with 4040 system

**E2.2: Implement 4201 Clock Generator**
- Crystal oscillator interface
- PHI1/PHI2 generation with proper timing
- SYNC signal generation
- Replace software clock with hardware-accurate model

**E2.3: Implement 4289 Standard Memory Interface**
- Address multiplexing
- Read/write control generation
- Support for 2101, 2102, 4316, 4702A
- Timing for standard memory chips

**E2.4: Implement 4308/4316 ROMs**
- Larger ROM variants (1K/2K × 8-bit)
- Different chip select scheme
- No I/O ports (unlike 4001)

**TIER 3: GUI AND PERIPHERALS**

**E3.1: Enhanced Waveform Viewer**
- Signal trace buffer (100K samples)
- Digital signal rendering + 4-bit bus hex
- Zoom/pan/cursor interactions
- Performance: 60fps with 10K+ samples

**E3.2: Era-Appropriate Peripherals**
- 7-segment LED drivers
- Nixie tube interface
- Matrix keyboard scanner
- Serial UART (RS-232)
- Paper tape / cassette interface

### 2.3 Extraction Pipeline Enhancements

**TIER 1: CRITICAL GAPS (Phase 0.5 → Phase 1)**

**X1.1: Schematic↔Layout Coordinate Transforms**
- **What**: Explicit homography/affine mappings for each chip
- **Why**: signals.txt coordinates are in schematic space, not layout pixel space
- **How**:
  - Implement `scripts/build_coordinate_transform_v0.py`
  - For each chip: find 4+ anchor points with known schematic and layout coordinates
  - Compute homography matrix (OpenCV `findHomography()`)
  - Validate: apply transform, check residuals < 5px
  - Output: `docs/evidence/coordinate_transforms_v0/XXXX_transform.json`
- **Verification**: Transform all signals.txt points, overlay on layout, visual inspection

**X1.2: Via Connectivity Modeling**
- **What**: Model actual via placement and multi-layer routing
- **Why**: Current netlists assume transistor A-B nodes are electrically connected
- **How**:
  - Extract vias layer from `i400X-vias.bmp`
  - For each transistor: check if A-B connection routed through metal or via
  - Update netlist with via graph (node → via → node)
  - Output: `netlist_v2` with explicit via connectivity
- **Verification**: Compare transistor count with/without via modeling

**X1.3: Gate-Level Extraction (Intermediate Fidelity)**
- **What**: Extract NMOS/PMOS cell boundaries, map to schematic gates
- **Why**: Enable medium-fidelity simulation before full transistor-level
- **How**:
  - Implement `scripts/extract_gates_v0.py`
  - Detect inverters (1 PMOS + 1 NMOS with common gate/drain)
  - Detect NAND/NOR gates (series/parallel transistor groups)
  - Detect transmission gates (NMOS+PMOS pairs with complementary gates)
  - Output: `docs/evidence/gates_v0/XXXX_gates_v0.json`
- **Verification**: Compare gate count with schematic (if available)

**TIER 2: CLUSTERING INFRASTRUCTURE (Phase 4)**

**X2.1: Hierarchical Clustering**
- **What**: Group subcircuits into functional blocks (input buffers, latches, drivers)
- **Why**: Enable multi-scale simulation (transistor ↔ gate ↔ RTL)
- **How**:
  - Implement `scripts/cluster_subcircuits_v0.py`
  - Define clustering strategies: spatial (BFS radius), functional (anchor type), electrical (connectivity)
  - Merge adjacent subcircuits with >50% shared nodes
  - Output: `docs/evidence/clusters_v0/XXXX_clusters_v0.json`
- **Verification**: Visualize clusters, check for overlap/gaps

**X2.2: SIMD Cluster Execution**
- **What**: Vectorize 4004/4040 instances using `std::simd` (nightly)
- **Why**: Parallel fuzz testing, ROM validation
- **How**:
  - Implement `mcs4-emu/crates/mcs4-system/src/simd_cluster.rs`
  - Run N copies of CPU in SIMD lanes (N=4, 8, 16)
  - Synchronize bus phases across lanes
  - Aggregate outputs for differential testing
- **Verification**: Run same ROM on 16 instances, verify bit-identical outputs

**X2.3: Transistor-Level Simulation**
- **What**: Event-driven switch-level solver consuming extracted netlists
- **Why**: Ultimate accuracy goal - electron-level simulation
- **How**:
  - Implement `mcs4-emu/crates/mcs4-core/src/transistor_solver.rs`
  - Load netlist_v1 (transistors + nodes + anchors)
  - Implement nodal analysis or event-driven simulation
  - Add BSIM4 or simple switch model parameters (Vth, Ron, Cgs)
  - Simulate subcircuits, compare with RTL emulator outputs
- **Verification**: Small subcircuits (inverters, NAND gates) match expected truth tables

---

## PART 3: IMPLEMENTATION ROADMAP

### Phase 0.5 Completion (Current → +1 sprint)

**Objective**: Finalize evidence extraction and close remaining gaps

**Deliverables**:
- [x] Power rail anchors upgraded to medium confidence (DONE 2026-01-14)
- [x] Subcircuits extracted for all chips (DONE 2026-01-14)
- [x] CI schematic pipeline passing (DONE 2026-01-14)
- [ ] Coordinate transforms for all 4 chips (X1.1)
- [ ] OCR persistent cache (M1.1)
- [ ] Comprehensive OCR regression suite (M1.2)
- [ ] CI environment pinning (M1.3)

**Success Criteria**:
- All extraction scripts run reproducibly (bit-identical outputs)
- OCR accuracy >95% on regression suite
- Coordinate transforms validated (residuals <5px)

### Phase 1: 4040 CPU and Disassembler (+2 sprints)

**Objective**: Complete core emulator functionality for MCS-40

**Deliverables**:
- [ ] 4040 CPU complete (E1.1): 14 new opcodes, interrupt handling, 7-level stack
- [ ] Disassembler core (E1.2): Full 4004+4040 support, symbol tables, auto-labeling
- [ ] Unit tests: 20+ new tests for 4040 features
- [ ] Documentation: Update ARCHITECTURE.md, STATUS.md, ROADMAP.md

**Success Criteria**:
- All 60 4040 instructions execute correctly
- Interrupt handling verified end-to-end
- Backward compatibility: all 46 4004 instructions still work
- All tests pass (135+ total)

### Phase 2: Support Chips and GUI (+3 sprints)

**Objective**: Expand chip support and complete debugger UI

**Deliverables**:
- [ ] Support chips (E2.1-E2.4): 4101 RAM, 4201 Clock, 4289 Interface, 4308 ROM
- [ ] GUI Debugger (E1.3): Register/memory/stack panels, breakpoints, shortcuts
- [ ] Waveform integration (E3.1): Synchronized with execution, cursor follows PC
- [ ] Documentation: Update emulator user guide

**Success Criteria**:
- MCS-40 system fully functional with 4040 + 4101 + 4289
- GUI debugger usable for interactive debugging
- Waveform viewer renders 10K+ samples at 60fps

### Phase 3: Extraction Enhancements (+2 sprints)

**Objective**: Close extraction gaps and enable transistor-level simulation

**Deliverables**:
- [ ] Via connectivity modeling (X1.2)
- [ ] Gate-level extraction (X1.3)
- [ ] Multi-modal OCR fusion (M2.1)
- [ ] Schematic↔layout cross-validation (M2.3)

**Success Criteria**:
- netlist_v2 with explicit via connectivity
- Gate-level netlists match schematic (if available)
- OCR accuracy >97% on regression suite

### Phase 4: Clustering and Performance (+2 sprints)

**Objective**: Enable parallel simulation and advanced clustering

**Deliverables**:
- [ ] Hierarchical clustering (X2.1)
- [ ] SIMD cluster execution (X2.2)
- [ ] Transistor-level simulation proof-of-concept (X2.3)
- [ ] Benchmark suite with CI thresholds

**Success Criteria**:
- SIMD cluster runs 16 instances in parallel
- Transistor-level simulation validates small subcircuits
- Benchmark suite runs in CI with performance thresholds

### Phase 5: FPGA and Advanced Features (+3+ sprints)

**Objective**: Enable FPGA synthesis and advanced peripherals

**Deliverables**:
- [ ] Verilog export with gate-level netlist
- [ ] FPGA synthesis targeting Lattice iCE40 or Xilinx Spartan
- [ ] Era-appropriate peripherals (E3.2): 7-seg, Nixie, UART, keyboard
- [ ] Custom ONNX CTC training (M3.1)

**Success Criteria**:
- Synthesized FPGA bitstream runs on hardware
- Peripherals enable building complete vintage systems
- ONNX model achieves >98% OCR accuracy

---

## PART 4: QUALITY GATES AND VERIFICATION

### 4.1 Build and Test Standards

**MANDATORY FOR ALL COMMITS**:
- `cargo clippy --all-targets --all-features -- -D warnings` (PASS)
- `cargo test --workspace` (ALL PASS)
- `cargo fmt --check` (PASS)
- `cargo deny check` (PASS)
- `scripts/todo_scan.sh` (update docs/TODO.md)

**CODE REVIEW CHECKLIST**:
- [ ] No `unsafe` Rust without justification and safety comments
- [ ] No secrets or credentials in code/config
- [ ] Input validation on all external data (file loads, network, user input)
- [ ] Error handling via `Result<>`, no unwrap() in critical paths
- [ ] Public APIs documented with rustdoc
- [ ] Complex algorithms explained with WHY comments
- [ ] Unit tests for new functionality (target >=90% coverage)
- [ ] Integration tests for cross-module interactions

### 4.2 OCR Quality Gates

**ACCURACY THRESHOLDS**:
- Regression suite: >=95% accuracy (current: variable)
- Per-chip benchmarks: >=90% for each of 4001/4002/4003/4004
- False positive rate: <5%
- Confidence score calibration: within 10% of actual accuracy

**REPRODUCIBILITY**:
- Bit-identical outputs across runs (same inputs, same config)
- Version-locked toolchain (Tesseract, ONNX, OpenCV)
- CI gate fails on version mismatch
- Extraction snapshots committed with SHA256 hashes

### 4.3 Extraction Validation

**NETLIST CONSISTENCY**:
- All anchors have >=1 incident transistor (0 failures)
- Required signals have unique incident nodes (uniqueness check)
- Power rail confidence: high (OCR-backed) or medium (geometry-backed)
- Transistor counts match analyzer reports (±5 tolerance)

**CROSS-VALIDATION**:
- Schematic connectivity matches layout connectivity
- Pin labels match PRIMARY_SOURCE_PINOUTS.md
- Coordinate transforms have residuals <5px
- Via connectivity consistent with mask layers

### 4.4 Emulator Correctness

**FUNCTIONAL TESTS**:
- All 46 4004 instructions: unit tests + integration tests
- All 14 4040 new instructions: unit tests
- Interrupt handling: vector, save/restore, enable/disable
- Stack overflow/underflow: boundary tests
- Bus protocol: phase-accurate I/O assertions

**INTEGRATION TESTS**:
- Fixture execution: 41 end-to-end tests (SRC/WRM/RDM/WRR/RDR)
- Multi-chip systems: ROM + RAM + CPU interactions
- Breakpoint system: halt at correct addresses
- Memory inspection: read/write verification

**PERFORMANCE BENCHMARKS**:
- Fixture execution: <10ms for 41 tests
- Waveform rendering: 60fps with 10K+ samples
- SIMD cluster: 16 instances with <2× overhead

---

## PART 5: CRITICAL FILES TO CREATE/MODIFY

### 5.1 New Files (Priority Order)

**Phase 1 (4040 CPU)**:
1. `mcs4-emu/crates/mcs4-chips/src/i4040/alu.rs` (extended ALU)
2. `mcs4-emu/crates/mcs4-chips/src/i4040/instruction_decode.rs` (60 instructions)
3. `mcs4-emu/crates/mcs4-chips/src/i4040/interrupt.rs` (interrupt controller)
4. `mcs4-emu/crates/mcs4-chips/src/i4040/tests.rs` (unit tests)

**Phase 1 (Disassembler)**:
5. `mcs4-emu/crates/mcs4-chips/src/disasm.rs` (core module)
6. `mcs4-emu/crates/mcs4-chips/src/disasm/format.rs` (listing formatter)
7. `mcs4-emu/crates/mcs4-chips/src/disasm/symbols.rs` (symbol table)

**Phase 0.5 (OCR)**:
8. `scripts/ocr_cache_v0.py` (persistent cache)
9. `scripts/ocr_benchmark_4001_v0.py` (4001 regression)
10. `scripts/ocr_benchmark_4002_v0.py` (4002 regression)
11. `scripts/check_ocr_versions.sh` (CI gate)

**Phase 0.5 (Extraction)**:
12. `scripts/build_coordinate_transform_v0.py` (schematic↔layout)
13. `scripts/validate_pin_connectivity_v0.py` (cross-validation)

**Phase 2 (GUI)**:
14. `mcs4-emu/crates/mcs4-gui/src/debugger.rs` (controller)
15. `mcs4-emu/crates/mcs4-gui/src/panels/disasm.rs` (disassembly panel)
16. `mcs4-emu/crates/mcs4-gui/src/panels/stack.rs` (stack view)
17. `mcs4-emu/crates/mcs4-gui/src/shortcuts.rs` (keyboard handler)

### 5.2 Files to Update

**Documentation**:
- `docs/ROADMAP.md` (update Phase 1 status)
- `docs/CHIP_EXTRACTION_STATUS.md` (update netlist versions)
- `mcs4-emu/STATUS.md` (update emulator status)
- `docs/evidence/LACUNAE_STATUS.md` (close resolved gaps)
- `mcs4-emu/INSTALLATION.md` (add new dependencies)
- `docs/TOOLING_AUDIT.md` (lock OCR versions)

**Build Configuration**:
- `mcs4-emu/Cargo.toml` (add disasm module exports)
- `.github/workflows/ci.yml` (add OCR version check)

**Emulator Code**:
- `mcs4-emu/crates/mcs4-chips/src/lib.rs` (export disasm module)
- `mcs4-emu/crates/mcs4-system/src/mcs40.rs` (integrate 4040)
- `mcs4-emu/crates/mcs4-gui/src/app.rs` (integrate debugger)

---

## PART 6: RESOURCE REQUIREMENTS

### 6.1 System Dependencies

**Rust Toolchain**:
- Nightly: 2026-01-06 (locked in rust-toolchain.toml)
- Components: rustfmt, clippy, miri, llvm-tools-preview
- MSRV: 1.92.0 (stable baseline)

**System Packages (Linux)**:
- Build tools: gcc, pkg-config, libxcb-dev, libxkbcommon-dev
- OCR: tesseract-ocr, ocrmypdf, jbig2enc, python-pymupdf
- Optional: python-onnxruntime-gpu (CUDA), python-opencv-cuda

**Python Dependencies** (for extraction scripts):
- Core: numpy, opencv-python, pytesseract
- Optional: onnxruntime-gpu, torch (for ONNX training)
- Analysis: scikit-learn, xgboost (for ensemble fusion)

**Documentation Tools**:
- yq (doc registry validation)
- cargo-deny, cargo-audit (security checks)
- cargo-llvm-cov (coverage reporting)

### 6.2 Performance Targets

**Emulator**:
- Fixture execution: <10ms for 41 end-to-end tests
- GUI frame rate: 60fps with waveform rendering
- SIMD cluster: 16 parallel instances with <2× overhead
- Memory footprint: <100MB for full system

**OCR Pipeline**:
- Throughput: >100 crops/second (with caching)
- Latency: <50ms per crop (without cache)
- Cache hit rate: >80% on re-runs
- Accuracy: >95% on regression suite

**Extraction Pipeline**:
- Full extraction: <5 minutes for all 4 chips (cold cache)
- Incremental update: <30 seconds (warm cache)
- Netlist generation: <10 seconds per chip
- Subcircuit extraction: <5 seconds per chip

---

## PART 7: NEXT IMMEDIATE STEPS (SPRINT 1)

### Week 1: Phase 0.5 Completion + Phase 1 Start

**Day 1-2: OCR Modernization**
1. Implement persistent OCR cache (M1.1)
2. Create comprehensive regression benchmarks (M1.2)
3. Pin OCR toolchain versions (M1.3)

**Day 3-4: Extraction Enhancements**
4. Build coordinate transforms for all 4 chips (X1.1)
5. Run cross-validation against PRIMARY_SOURCE_PINOUTS.md

**Day 5-7: 4040 CPU Development**
6. Complete 7-level stack implementation
7. Implement 14 new 4040 opcodes
8. Add interrupt controller state machine
9. Write unit tests for new instructions

### Week 2: Phase 1 Completion

**Day 8-10: 4040 Testing and Integration**
10. Complete interrupt handling end-to-end
11. Run backward compatibility tests (all 4004 instructions)
12. Integration tests with 4040 system
13. Document new instructions and features

**Day 11-12: Disassembler Core**
14. Implement disasm_one() and disasm_range()
15. Add operand formatting for all 60 instructions
16. Implement auto_label() for jump targets
17. Unit tests for disassembler

**Day 13-14: Phase 1 Validation**
18. Run full test suite (target: 135+ tests passing)
19. Update documentation (ROADMAP, STATUS, ARCHITECTURE)
20. Code review and cleanup
21. Prepare Phase 2 planning

---

## PART 8: SUCCESS METRICS

### Overall Project Metrics

**Emulator Completeness**:
- Current: 50-60% → Target: 90% (Phase 5)
- Phase 1 Target: 75% (4040 complete, disassembler working)
- Phase 2 Target: 85% (GUI debugger, support chips)

**Extraction Completeness**:
- Current: 80% (netlists extracted, anchors mapped) → Target: 95% (Phase 5)
- Phase 0.5 Target: 85% (coordinate transforms, OCR improvements)
- Phase 3 Target: 90% (via modeling, gate extraction)

**OCR Accuracy**:
- Current: ~85% (variable) → Target: >98% (Phase 5)
- Phase 0.5 Target: >95% (persistent cache, regression suite)
- Phase 3 Target: >97% (ensemble fusion, adaptive preprocessing)

**Test Coverage**:
- Current: 115 tests → Target: 200+ tests (Phase 5)
- Phase 1 Target: 135+ tests (4040 + disassembler)
- Phase 2 Target: 160+ tests (support chips + GUI)

### Quality Indicators

**Code Quality**:
- Zero clippy warnings (maintained throughout)
- Zero unsafe Rust blocks (except where justified)
- rustdoc coverage >90% for public APIs
- Cyclomatic complexity <10 for all functions

**Documentation Quality**:
- All public APIs documented with examples
- Architecture decisions documented with WHY
- Primary sources cited for all claims
- Installation instructions reproducible

**Reproducibility**:
- Bit-identical extraction outputs across runs
- Deterministic test results (no flakiness)
- Version-locked dependencies
- Docker/Nix build support (future)

---

## PART 9: RISK MITIGATION

### Technical Risks

**R1: 4040 Backward Compatibility**
- Risk: New instructions break existing 4004 functionality
- Mitigation: Run full 4004 regression suite before/after changes
- Test: 46 instruction tests + 41 fixture tests must pass

**R2: OCR Accuracy Degradation**
- Risk: Toolchain updates reduce accuracy
- Mitigation: Version pinning + CI gates + regression benchmarks
- Test: Accuracy must be >=95% on all benchmarks

**R3: Extraction Pipeline Fragility**
- Risk: Manual steps break reproducibility
- Mitigation: Automate all steps, document dependencies
- Test: Cold-cache extraction produces bit-identical outputs

**R4: GUI Performance**
- Risk: Waveform rendering too slow for large traces
- Mitigation: Incremental rendering, LOD optimization
- Test: Maintain 60fps with 10K+ samples

### Schedule Risks

**S1: Scope Creep**
- Risk: Adding features delays core milestones
- Mitigation: Strict phase gates, focus on critical path
- Control: No new features until Phase 1 complete

**S2: Testing Debt**
- Risk: Skipping tests to meet deadlines
- Mitigation: TDD approach, test-first development
- Control: No commit without tests for new code

**S3: Documentation Lag**
- Risk: Code ahead of docs, hard to onboard
- Mitigation: Update docs in same commit as code
- Control: PR checklist includes doc updates

---

## SUMMARY

This plan provides a **comprehensive roadmap** to complete the Intel MCS-4/MCS-40 transistor-level emulator project, covering:

1. **Current State**: 50-60% complete, Phase 0.5 finishing, Phase 1 ready
2. **OCR Modernization**: Persistent caching, ensemble fusion, ONNX training
3. **Emulator Completion**: 4040 CPU, disassembler, GUI debugger, support chips
4. **Extraction Enhancements**: Coordinate transforms, via modeling, gate-level extraction
5. **Clustering Infrastructure**: Hierarchical clustering, SIMD execution, transistor-level simulation
6. **Quality Gates**: Testing, validation, reproducibility, documentation
7. **Implementation Roadmap**: 5 phases, ~12 sprints, clear deliverables

**Critical Path**: Phase 0.5 completion (1 sprint) → Phase 1 (2 sprints) → Phase 2 (3 sprints) → Phase 3 (2 sprints) → Phase 4 (2 sprints) → Phase 5 (3+ sprints)

**Key Success Factors**:
- Maintain test-first development (>90% coverage)
- Treat warnings as errors (clippy, tests, linting)
- Update documentation with each milestone
- Validate against primary sources (datasheets, manuals)
- Ensure reproducibility (version locking, deterministic builds)

The project is **well-structured** with solid foundations (netlist extraction, emulator core, OCR infrastructure). The next 12 sprints will complete the critical path and deliver a fully functional, transistor-accurate emulator system.

---

**Plan Author**: Claude Sonnet 4.5 (claude-sonnet-4-5-20250929)
**Plan Date**: 2026-01-29
**Estimated Duration**: 12 sprints (~24-30 weeks)
**Total Effort**: ~2000-3000 engineering hours
