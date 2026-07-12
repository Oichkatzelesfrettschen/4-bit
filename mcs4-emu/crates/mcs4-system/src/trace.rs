//! Deterministic post-phase trace records for system-level regression fixtures.

use std::collections::BTreeSet;

use mcs4_bus::{control::IoOp, BusCycle, ControlSignals, DataBus};
use serde::{Deserialize, Serialize};

/// Schema version for cross-fidelity trace frames.
pub const TRACE_FRAME_SCHEMA_VERSION: u32 = 1;

/// Architecture that produced a phase trace sample.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SystemArchitecture {
    /// Intel MCS-4 system with a 4004 CPU.
    Mcs4,
    /// Intel MCS-40 system with a 4040 CPU.
    Mcs40,
}

/// Stable trace spelling for an MCS-4-family bus phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TracePhase {
    A1,
    A2,
    A3,
    M1,
    M2,
    X1,
    X2,
    X3,
}

impl From<BusCycle> for TracePhase {
    fn from(value: BusCycle) -> Self {
        match value {
            BusCycle::A1 => Self::A1,
            BusCycle::A2 => Self::A2,
            BusCycle::A3 => Self::A3,
            BusCycle::M1 => Self::M1,
            BusCycle::M2 => Self::M2,
            BusCycle::X1 => Self::X1,
            BusCycle::X2 => Self::X2,
            BusCycle::X3 => Self::X3,
        }
    }
}

impl TracePhase {
    /// Return the stable ordinal in one eight-phase machine cycle.
    pub const fn ordinal(self) -> u8 {
        match self {
            Self::A1 => 0,
            Self::A2 => 1,
            Self::A3 => 2,
            Self::M1 => 3,
            Self::M2 => 4,
            Self::X1 => 5,
            Self::X2 => 6,
            Self::X3 => 7,
        }
    }
}

/// Serializable post-phase state that remains independent of tracing subscribers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhaseTrace {
    /// Schema version for reviewed fixture compatibility.
    pub schema_version: u32,
    /// System family that emitted this sample.
    pub architecture: SystemArchitecture,
    /// Phase completed by this system step.
    pub completed_phase: TracePhase,
    /// Phase selected for the next system step.
    pub next_phase: TracePhase,
    /// Complete machine cycles after this step.
    pub machine_cycles: u64,
    /// Complete instructions after this step, owned by the CPU cycle state.
    pub instruction_count: u64,
    /// CPU program counter after this step.
    pub pc: u16,
    /// CPU accumulator after this step.
    pub accumulator: u8,
    /// CPU carry flag after this step.
    pub carry: bool,
    /// Four-bit bus value after this step.
    pub bus_value: u8,
    /// Whether every bus line has a defined logical level.
    pub bus_valid: bool,
    /// Whether the bus contains a detected contention state.
    pub bus_contention: bool,
    /// Selected ROM bank after this step, if any.
    pub selected_rom: Option<u8>,
    /// Selected RAM bank after this step, if any.
    pub selected_ram: Option<u8>,
    /// Normalized high-level I/O operation after this step, if any.
    pub io_op: Option<String>,
}

impl PhaseTrace {
    /// Construct one stable trace record from observable post-phase state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        architecture: SystemArchitecture,
        completed_phase: BusCycle,
        next_phase: BusCycle,
        machine_cycles: u64,
        instruction_count: u64,
        pc: u16,
        accumulator: u8,
        carry: bool,
        bus: &DataBus,
        control: &ControlSignals,
    ) -> Self {
        Self {
            schema_version: 1,
            architecture,
            completed_phase: completed_phase.into(),
            next_phase: next_phase.into(),
            machine_cycles,
            instruction_count,
            pc,
            accumulator,
            carry,
            bus_value: bus.read(),
            bus_valid: bus.is_valid(),
            bus_contention: bus.has_contention(),
            selected_rom: control.selected_rom(),
            selected_ram: control.selected_ram(),
            io_op: control.io_op.map(normalize_io_op),
        }
    }
}

/// Execution backend that emitted a trace frame.
///
/// The backend identifies the implementation surface only. It does not imply
/// that another fidelity surface has passed an equivalence check.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceBackend {
    /// Rust behavioral system model.
    Behavioral,
    /// Extracted switch-level cone.
    SwitchLevel,
    /// Nodal or transient circuit solver.
    Nodal,
    /// Verilator-generated C++ model.
    Verilator,
    /// Post-synthesis digital model.
    PostSynthesis,
    /// Attended physical-board observation.
    Hardware,
}

/// Fidelity statement attached to one trace frame.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceFidelity {
    /// Functional state-machine behavior.
    Behavioral,
    /// Phase-resolved digital behavior.
    PhaseAccurate,
    /// Ideal-switch transistor behavior.
    SwitchLevel,
    /// Voltage-resolved circuit behavior.
    Nodal,
    /// Target FPGA implementation behavior.
    Fpga,
    /// Observed programmed-board behavior.
    Hardware,
}

/// Evidence state of the model that emitted a frame.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceEvidenceStatus {
    /// The local process has no immutable provenance manifest.
    LocalUnsealed,
    /// The model has a retained reproducible artifact.
    Reproduced,
    /// The model has focused automated tests.
    Tested,
    /// The model has a retained synthesis result.
    Synthesized,
    /// The model has an attended board capture.
    HardwareProbed,
    /// A named external evidence requirement blocks promotion.
    Blocked,
}

/// Canonical representation used to derive a declared stimulus digest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceStimulusKind {
    /// Ordered behavioral replay inputs with stable event identifiers.
    ReplayInputTranscript,
    /// Exact bytes of one external scenario JSON document.
    ScenarioJson,
}

/// Provenance that prevents a visual trace from becoming an anonymous claim.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TraceProvenance {
    /// Backend implementation that emitted the frame.
    pub backend: TraceBackend,
    /// Fidelity represented by the frame values.
    pub fidelity: TraceFidelity,
    /// Stable target identifier, such as `mcs4-behavioral` or `i4003_fpga`.
    pub model_id: String,
    /// SHA-256 of the source-model manifest when one exists.
    pub model_sha256: Option<String>,
    /// SHA-256 of the applied stimulus when one exists.
    pub stimulus_sha256: Option<String>,
    /// Canonical form used to derive `stimulus_sha256`.
    #[serde(default)]
    pub stimulus_kind: Option<TraceStimulusKind>,
    /// Current evidence status for this exact model surface.
    pub evidence_status: TraceEvidenceStatus,
}

impl TraceProvenance {
    /// Describe a local behavioral MCS-4 session without claiming a sealed artifact.
    pub fn behavioral_mcs4() -> Self {
        Self {
            backend: TraceBackend::Behavioral,
            fidelity: TraceFidelity::PhaseAccurate,
            model_id: "mcs4-behavioral".to_owned(),
            model_sha256: None,
            stimulus_sha256: None,
            stimulus_kind: None,
            evidence_status: TraceEvidenceStatus::LocalUnsealed,
        }
    }

    /// Describe a local behavioral MCS-40 session without claiming a sealed artifact.
    pub fn behavioral_mcs40() -> Self {
        Self {
            backend: TraceBackend::Behavioral,
            fidelity: TraceFidelity::PhaseAccurate,
            model_id: "mcs40-behavioral".to_owned(),
            model_sha256: None,
            stimulus_sha256: None,
            stimulus_kind: None,
            evidence_status: TraceEvidenceStatus::LocalUnsealed,
        }
    }
}

/// Four-state digital logic value used by cross-fidelity signal records.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceLogic {
    /// Logical zero.
    Zero,
    /// Logical one.
    One,
    /// Unknown or unresolved logic value.
    X,
    /// High-impedance logic value.
    Z,
}

impl From<bool> for TraceLogic {
    fn from(value: bool) -> Self {
        if value {
            Self::One
        } else {
            Self::Zero
        }
    }
}

/// Value carried by one named signal in a trace frame.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TraceValue {
    /// One four-state digital value.
    Logic { value: TraceLogic },
    /// An unsigned digital bus with an explicit width.
    Bits { width: u8, value: u64 },
    /// A voltage-resolved value from an electrical backend.
    Voltage { volts: f64 },
    /// A known unavailable value with an explicit reason.
    Unavailable { reason: String },
}

/// Named, hierarchy-qualified signal observation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TraceSignal {
    /// Stable hierarchy path, such as `mcs4.bus`.
    pub path: String,
    /// Observed value at the frame boundary.
    pub value: TraceValue,
    /// Evidence object or model path that supports this observation.
    pub source: Option<String>,
}

/// Versioned cross-fidelity frame for GUI, replay, and comparison consumers.
///
/// `sequence` is unique within `run_id`. It remains phase-unique even when
/// eight consecutive frames share one `machine_cycles` value. A missing
/// `physical_time_ps` states that the backend has no calibrated physical time
/// value; callers must not infer one from the logical sequence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TraceFrame {
    /// Schema version for JSON compatibility.
    pub schema_version: u32,
    /// Monotonic run identity. Reset starts a new run.
    pub run_id: u64,
    /// Monotonic phase or event sequence within this run.
    pub sequence: u64,
    /// Most recent input event that influences this frame.
    pub input_event_id: u64,
    /// Logical event index. It matches `sequence` for phase-driven backends.
    pub logical_tick: u64,
    /// Calibrated physical time when the backend can establish it.
    pub physical_time_ps: Option<u64>,
    /// MCS bus-phase record when the backend exposes one.
    pub phase: Option<PhaseTrace>,
    /// Model and evidence identity for every displayed value.
    pub provenance: TraceProvenance,
    /// Stable named signal observations.
    pub signals: Vec<TraceSignal>,
}

impl TraceFrame {
    /// Wrap one behavioral post-phase record in the shared frame schema.
    pub fn from_behavioral_phase(
        run_id: u64,
        sequence: u64,
        input_event_id: u64,
        provenance: TraceProvenance,
        phase: PhaseTrace,
    ) -> Self {
        let signal_source = Some("mcs4-system::PhaseTrace".to_owned());
        let system_path = match phase.architecture {
            SystemArchitecture::Mcs4 => "mcs4",
            SystemArchitecture::Mcs40 => "mcs40",
        };
        let signals = vec![
            TraceSignal {
                path: format!("{system_path}.phase"),
                value: TraceValue::Bits {
                    width: 3,
                    value: u64::from(phase.completed_phase.ordinal()),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.bus"),
                value: TraceValue::Bits {
                    width: 4,
                    value: u64::from(phase.bus_value),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.bus.valid"),
                value: TraceValue::Logic {
                    value: phase.bus_valid.into(),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.bus.contention"),
                value: TraceValue::Logic {
                    value: phase.bus_contention.into(),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.cpu.pc"),
                value: TraceValue::Bits {
                    width: 12,
                    value: u64::from(phase.pc),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.cpu.accumulator"),
                value: TraceValue::Bits {
                    width: 4,
                    value: u64::from(phase.accumulator),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.cpu.carry"),
                value: TraceValue::Logic {
                    value: phase.carry.into(),
                },
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.control.rom"),
                value: optional_bits(phase.selected_rom, 4, "ROM selection is inactive"),
                source: signal_source.clone(),
            },
            TraceSignal {
                path: format!("{system_path}.control.ram"),
                value: optional_bits(phase.selected_ram, 4, "RAM selection is inactive"),
                source: signal_source,
            },
        ];

        Self {
            schema_version: TRACE_FRAME_SCHEMA_VERSION,
            run_id,
            sequence,
            input_event_id,
            logical_tick: sequence,
            physical_time_ps: None,
            phase: Some(phase),
            provenance,
            signals,
        }
    }

    /// Return the observation for a stable hierarchy path.
    pub fn signal(&self, path: &str) -> Option<&TraceSignal> {
        self.signals.iter().find(|signal| signal.path == path)
    }

    /// Validate structural invariants before a frame enters a comparison or GUI trace.
    pub fn validate(&self) -> Result<(), TraceFrameError> {
        if self.schema_version != TRACE_FRAME_SCHEMA_VERSION {
            return Err(TraceFrameError::UnsupportedSchema(self.schema_version));
        }
        if self.provenance.model_id.trim().is_empty() {
            return Err(TraceFrameError::EmptyModelId);
        }
        if let Some(stimulus_sha256) = self.provenance.stimulus_sha256.as_deref() {
            if !is_sha256(stimulus_sha256) {
                return Err(TraceFrameError::InvalidStimulusSha256(stimulus_sha256.to_owned()));
            }
        }
        if self.provenance.stimulus_sha256.is_some() != self.provenance.stimulus_kind.is_some() {
            return Err(TraceFrameError::IncompleteStimulusIdentity);
        }
        let mut signal_paths = BTreeSet::new();
        for signal in &self.signals {
            if signal.path.trim().is_empty() {
                return Err(TraceFrameError::EmptySignalPath);
            }
            if !signal_paths.insert(&signal.path) {
                return Err(TraceFrameError::DuplicateSignalPath(signal.path.clone()));
            }
            if let TraceValue::Bits { width, value } = signal.value {
                if width == 0 || width > 64 || (width < 64 && value >= (1_u64 << width)) {
                    return Err(TraceFrameError::InvalidBits {
                        path: signal.path.clone(),
                        width,
                        value,
                    });
                }
            }
        }
        Ok(())
    }
}

fn optional_bits(value: Option<u8>, width: u8, unavailable_reason: &str) -> TraceValue {
    value.map_or_else(
        || TraceValue::Unavailable {
            reason: unavailable_reason.to_owned(),
        },
        |value| TraceValue::Bits {
            width,
            value: u64::from(value),
        },
    )
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Structural trace-frame validation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceFrameError {
    /// The consumer does not support this schema version.
    UnsupportedSchema(u32),
    /// Model identity is absent.
    EmptyModelId,
    /// A declared stimulus digest is not a SHA-256 hexadecimal value.
    InvalidStimulusSha256(String),
    /// A frame declares only half of its stimulus identity.
    IncompleteStimulusIdentity,
    /// A signal has no hierarchy path.
    EmptySignalPath,
    /// A bus value does not fit its declared width.
    InvalidBits {
        /// Signal that violates the invariant.
        path: String,
        /// Declared bus width.
        width: u8,
        /// Observed unsigned value.
        value: u64,
    },
    /// A frame declares the same hierarchy path more than once.
    DuplicateSignalPath(String),
}

impl std::fmt::Display for TraceFrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(formatter, "unsupported trace-frame schema version {version}"),
            Self::EmptyModelId => formatter.write_str("trace-frame model_id is empty"),
            Self::InvalidStimulusSha256(value) => {
                write!(formatter, "trace-frame stimulus SHA-256 is invalid: {value}")
            }
            Self::IncompleteStimulusIdentity => {
                formatter.write_str("trace-frame stimulus hash and kind must be declared together")
            }
            Self::EmptySignalPath => formatter.write_str("trace-frame signal path is empty"),
            Self::InvalidBits { path, width, value } => {
                write!(
                    formatter,
                    "trace-frame signal {path} has value {value} outside {width}-bit range"
                )
            }
            Self::DuplicateSignalPath(path) => {
                write!(formatter, "trace-frame signal path {path} is duplicated")
            }
        }
    }
}

impl std::error::Error for TraceFrameError {}

/// Outcome of one explicit shared-signal trace comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceComparison {
    /// SHA-256 that identifies the common stimulus.
    pub stimulus_sha256: String,
    /// Shared signal paths that have identical values.
    pub matching_paths: Vec<String>,
    /// Shared signal paths that differ at the selected frame boundaries.
    pub mismatching_paths: Vec<String>,
}

/// Reason two trace frames cannot enter a value comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceComparisonError {
    /// The left input violates the trace-frame contract.
    InvalidLeftFrame(TraceFrameError),
    /// The right input violates the trace-frame contract.
    InvalidRightFrame(TraceFrameError),
    /// One frame uses evidence marked as blocked.
    BlockedEvidence,
    /// The left frame does not declare a stimulus hash.
    MissingLeftStimulus,
    /// The right frame does not declare a stimulus hash.
    MissingRightStimulus,
    /// The left frame does not declare a stimulus representation kind.
    MissingLeftStimulusKind,
    /// The right frame does not declare a stimulus representation kind.
    MissingRightStimulusKind,
    /// The frames use distinct canonical stimulus representations.
    StimulusKindMismatch {
        /// Stimulus representation on the left frame.
        left: TraceStimulusKind,
        /// Stimulus representation on the right frame.
        right: TraceStimulusKind,
    },
    /// The frames declare different stimuli.
    StimulusMismatch {
        /// Stimulus hash on the left frame.
        left: String,
        /// Stimulus hash on the right frame.
        right: String,
    },
    /// The frames expose no common hierarchy-qualified signal path.
    NoSharedSignals,
}

impl std::fmt::Display for TraceComparisonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLeftFrame(error) => write!(formatter, "left trace frame is invalid: {error}"),
            Self::InvalidRightFrame(error) => write!(formatter, "right trace frame is invalid: {error}"),
            Self::BlockedEvidence => formatter.write_str("trace comparison includes blocked evidence"),
            Self::MissingLeftStimulus => formatter.write_str("left trace frame does not declare a stimulus hash"),
            Self::MissingRightStimulus => formatter.write_str("right trace frame does not declare a stimulus hash"),
            Self::MissingLeftStimulusKind => {
                formatter.write_str("left trace frame does not declare a stimulus representation")
            }
            Self::MissingRightStimulusKind => {
                formatter.write_str("right trace frame does not declare a stimulus representation")
            }
            Self::StimulusKindMismatch { left, right } => {
                write!(
                    formatter,
                    "trace comparison stimulus representation mismatch: {left:?} versus {right:?}"
                )
            }
            Self::StimulusMismatch { left, right } => {
                write!(formatter, "trace comparison stimulus mismatch: {left} versus {right}")
            }
            Self::NoSharedSignals => formatter.write_str("trace frames have no shared signal paths"),
        }
    }
}

impl std::error::Error for TraceComparisonError {}

/// Compare values only after matching declared stimulus representation and digest.
///
/// This compares observations at their supplied boundaries. It does not infer
/// phase alignment, physical-time alignment, or whole-system equivalence.
pub fn compare_trace_frames(left: &TraceFrame, right: &TraceFrame) -> Result<TraceComparison, TraceComparisonError> {
    left.validate().map_err(TraceComparisonError::InvalidLeftFrame)?;
    right.validate().map_err(TraceComparisonError::InvalidRightFrame)?;
    if matches!(left.provenance.evidence_status, TraceEvidenceStatus::Blocked)
        || matches!(right.provenance.evidence_status, TraceEvidenceStatus::Blocked)
    {
        return Err(TraceComparisonError::BlockedEvidence);
    }

    let left_stimulus = left
        .provenance
        .stimulus_sha256
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or(TraceComparisonError::MissingLeftStimulus)?;
    let right_stimulus = right
        .provenance
        .stimulus_sha256
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or(TraceComparisonError::MissingRightStimulus)?;
    let left_stimulus_kind = left
        .provenance
        .stimulus_kind
        .ok_or(TraceComparisonError::MissingLeftStimulusKind)?;
    let right_stimulus_kind = right
        .provenance
        .stimulus_kind
        .ok_or(TraceComparisonError::MissingRightStimulusKind)?;
    if left_stimulus_kind != right_stimulus_kind {
        return Err(TraceComparisonError::StimulusKindMismatch {
            left: left_stimulus_kind,
            right: right_stimulus_kind,
        });
    }
    if left_stimulus != right_stimulus {
        return Err(TraceComparisonError::StimulusMismatch {
            left: left_stimulus.to_owned(),
            right: right_stimulus.to_owned(),
        });
    }

    let mut matching_paths = Vec::new();
    let mut mismatching_paths = Vec::new();
    for left_signal in &left.signals {
        let Some(right_signal) = right.signal(&left_signal.path) else {
            continue;
        };
        if left_signal.value == right_signal.value {
            matching_paths.push(left_signal.path.clone());
        } else {
            mismatching_paths.push(left_signal.path.clone());
        }
    }
    if matching_paths.is_empty() && mismatching_paths.is_empty() {
        return Err(TraceComparisonError::NoSharedSignals);
    }
    Ok(TraceComparison {
        stimulus_sha256: left_stimulus.to_owned(),
        matching_paths,
        mismatching_paths,
    })
}

/// Return whether two frames meet the minimum comparison contract.
pub fn frames_are_comparable(left: &TraceFrame, right: &TraceFrame) -> bool {
    compare_trace_frames(left, right).is_ok()
}

fn normalize_io_op(operation: IoOp) -> String {
    match operation {
        IoOp::Src => "src".to_owned(),
        IoOp::RomPortWrite => "rom_port_write".to_owned(),
        IoOp::RomPortRead => "rom_port_read".to_owned(),
        IoOp::RamMainWrite => "ram_main_write".to_owned(),
        IoOp::RamMainRead => "ram_main_read".to_owned(),
        IoOp::RamPortWrite => "ram_port_write".to_owned(),
        IoOp::RamStatusWrite(index) => format!("ram_status_write_{index}"),
        IoOp::RamStatusRead(index) => format!("ram_status_read_{index}"),
        IoOp::Rpm => "rpm".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_phase_matches_bus_phase() {
        assert_eq!(TracePhase::from(BusCycle::X2), TracePhase::X2);
    }

    #[test]
    fn normalized_status_operation_carries_its_index() {
        assert_eq!(normalize_io_op(IoOp::RamStatusWrite(3)), "ram_status_write_3");
    }
}
