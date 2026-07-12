//! Bounded shared trace storage for versioned cross-fidelity frames.

use std::collections::VecDeque;

use mcs4_system::{TraceFrame, TraceFrameError};

/// Maximum number of frames retained by the interactive UI.
pub const MAX_FRAMES: usize = 100_000;

/// Stable identity of a retained frame.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FrameId {
    /// Run identity. A reset creates a new run.
    pub run_id: u64,
    /// Phase-unique sequence within the run.
    pub sequence: u64,
}

impl From<&TraceFrame> for FrameId {
    fn from(frame: &TraceFrame) -> Self {
        Self {
            run_id: frame.run_id,
            sequence: frame.sequence,
        }
    }
}

/// Retained window and explicit loss accounting for a bounded trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRetention {
    /// First retained frame, if the trace holds data.
    pub first: Option<FrameId>,
    /// Last retained frame, if the trace holds data.
    pub last: Option<FrameId>,
    /// Number of frames evicted because the UI retention bound is full.
    pub dropped_frame_count: u64,
}

/// Frame store consumed by waveform, provenance, and replay panels.
pub struct SignalTrace {
    frames: VecDeque<TraceFrame>,
    dropped_frame_count: u64,
}

impl SignalTrace {
    /// Create an empty bounded trace store.
    pub fn new() -> Self {
        Self {
            frames: VecDeque::with_capacity(MAX_FRAMES),
            dropped_frame_count: 0,
        }
    }

    /// Validate and retain one canonical frame.
    pub fn push_frame(&mut self, frame: TraceFrame) -> Result<(), TraceFrameError> {
        frame.validate()?;
        if self.frames.len() >= MAX_FRAMES {
            let _ = self.frames.pop_front();
            self.dropped_frame_count += 1;
        }
        self.frames.push_back(frame);
        Ok(())
    }

    /// Iterate retained frames in insertion order.
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, TraceFrame> {
        self.frames.iter()
    }

    /// Return one retained frame by stable identity.
    pub fn frame(&self, id: FrameId) -> Option<&TraceFrame> {
        self.frames.iter().find(|frame| FrameId::from(*frame) == id)
    }

    /// Return one retained frame by its current display offset.
    pub fn frame_at(&self, offset: usize) -> Option<&TraceFrame> {
        self.frames.get(offset)
    }

    /// Return the current retention window and any evicted-frame count.
    pub fn retention(&self) -> TraceRetention {
        TraceRetention {
            first: self.frames.front().map(FrameId::from),
            last: self.frames.back().map(FrameId::from),
            dropped_frame_count: self.dropped_frame_count,
        }
    }

    /// Return the number of retained frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Return whether no frame is retained.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Clear retained UI history and its loss accounting.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.dropped_frame_count = 0;
    }
}

impl Default for SignalTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use mcs4_system::{Mcs4System, ReplaySession};

    use super::*;

    fn frame(sequence: u64) -> TraceFrame {
        let mut session = ReplaySession::<Mcs4System>::new();
        for _ in 0..sequence {
            let _ = session.step_phase().expect("step phase");
        }
        session.last_frame().expect("frame").clone()
    }

    #[test]
    fn new_trace_is_empty() {
        let trace = SignalTrace::new();
        assert!(trace.is_empty());
        assert_eq!(trace.len(), 0);
    }

    #[test]
    fn frame_identity_selects_the_phase_unique_record() {
        let mut trace = SignalTrace::new();
        let first = frame(1);
        let second = frame(2);
        let first_id = FrameId::from(&first);
        trace.push_frame(first).expect("first frame");
        trace.push_frame(second).expect("second frame");

        let retained = trace.frame(first_id).expect("first retained frame");
        assert_eq!(retained.sequence, 1);
        assert_eq!(retained.provenance.model_id, "mcs4-behavioral");
    }

    #[test]
    fn clear_resets_visible_retention_accounting() {
        let mut trace = SignalTrace::new();
        trace.push_frame(frame(1)).expect("frame");
        assert!(trace.retention().first.is_some());
        trace.clear();
        assert_eq!(
            trace.retention(),
            TraceRetention {
                first: None,
                last: None,
                dropped_frame_count: 0,
            }
        );
    }

    #[test]
    fn frames_retain_their_declared_target_identity() {
        let mut trace = SignalTrace::new();
        let sample = frame(1);
        trace.push_frame(sample).expect("frame");
        let retained = trace.frame_at(0).expect("retained frame");
        assert_eq!(
            retained.phase.as_ref().expect("phase").architecture,
            mcs4_system::SystemArchitecture::Mcs4
        );
    }
}
