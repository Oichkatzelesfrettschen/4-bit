//! 4004 phase-window timing and logical I/O observation.

use mcs4_bus::{cycle::cycle_timing, BusCycle, CycleState};
use mcs4_core::timing::{clock_spec, Time};

/// Source-bounded timing profile for one logical MCS-4 bus phase.
///
/// The profile describes the external two-phase clock waveform inside one
/// logical phase. It does not model transistor propagation delay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimingProfile {
    /// Duration of one A1 through X3 logical phase in picoseconds.
    pub phase_period_ps: Time,
    /// PHI1 high width in picoseconds.
    pub phi1_width_ps: Time,
    /// PHI1-to-PHI2 gap in picoseconds.
    pub phi1_to_phi2_gap_ps: Time,
    /// PHI2 high width in picoseconds.
    pub phi2_width_ps: Time,
    /// PHI2-to-next-PHI1 gap in picoseconds.
    pub phi2_to_phi1_gap_ps: Time,
}

impl TimingProfile {
    /// Build the source-bounded typical profile from `clock_spec` constants.
    pub const fn typical() -> Self {
        let covered = clock_spec::T0PW_MIN + clock_spec::T0D1_MIN + clock_spec::T0PW_MIN;
        Self {
            phase_period_ps: clock_spec::TCY_TYP,
            phi1_width_ps: clock_spec::T0PW_MIN,
            phi1_to_phi2_gap_ps: clock_spec::T0D1_MIN,
            phi2_width_ps: clock_spec::T0PW_MIN,
            // Preserve the documented typical logical-phase period. The
            // residual is greater than the retained minimum T0D2 bound.
            phi2_to_phi1_gap_ps: clock_spec::TCY_TYP - covered,
        }
    }

    /// Validate bounds and the exact phase-period decomposition.
    pub fn validate(&self) -> Result<(), TimingViolation> {
        if !(clock_spec::TCY_MIN..=clock_spec::TCY_MAX).contains(&self.phase_period_ps) {
            return Err(TimingViolation::InvalidProfile {
                reason: "phase period is outside the retained clock range",
            });
        }
        if !(clock_spec::T0PW_MIN..=clock_spec::T0PW_MAX).contains(&self.phi1_width_ps)
            || !(clock_spec::T0PW_MIN..=clock_spec::T0PW_MAX).contains(&self.phi2_width_ps)
        {
            return Err(TimingViolation::InvalidProfile {
                reason: "clock pulse width is outside the retained range",
            });
        }
        if !(clock_spec::T0D1_MIN..=clock_spec::T0D1_MAX).contains(&self.phi1_to_phi2_gap_ps) {
            return Err(TimingViolation::InvalidProfile {
                reason: "PHI1-to-PHI2 gap is outside the retained range",
            });
        }
        if self.phi2_to_phi1_gap_ps < clock_spec::T0D2_MIN {
            return Err(TimingViolation::InvalidProfile {
                reason: "PHI2-to-PHI1 gap is below the retained minimum",
            });
        }
        let parts = self
            .phi1_width_ps
            .checked_add(self.phi1_to_phi2_gap_ps)
            .and_then(|value| value.checked_add(self.phi2_width_ps))
            .and_then(|value| value.checked_add(self.phi2_to_phi1_gap_ps));
        if parts != Some(self.phase_period_ps) {
            return Err(TimingViolation::InvalidProfile {
                reason: "clock waveform components do not equal the phase period",
            });
        }
        Ok(())
    }
}

impl Default for TimingProfile {
    fn default() -> Self {
        Self::typical()
    }
}

/// A source-bounded logical signal window inside one machine cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimingWindow {
    /// Logical bus phase that owns this window.
    pub phase: BusCycle,
    /// Inclusive window start in picoseconds.
    pub start_ps: Time,
    /// Exclusive window end in picoseconds.
    pub end_ps: Time,
}

/// Logical MCS-4 signal windows for the machine cycle containing a sample.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalWindows {
    /// SYNC assertion window.
    pub sync: TimingWindow,
    /// CM-ROM valid window.
    pub cm_rom: TimingWindow,
    /// CM-RAM candidate transfer window. Actual selection remains operation-dependent.
    pub cm_ram_candidate: TimingWindow,
}

/// A stable observation after one completed CPU bus phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimingSnapshot {
    /// Most recently completed phase, or none before the first CPU tick.
    pub completed_phase: Option<BusCycle>,
    /// Phase that the CPU executes next.
    pub next_phase: BusCycle,
    /// Start time of the completed phase in picoseconds.
    pub phase_start_ps: Time,
    /// Elapsed simulation time after the completed phase in picoseconds.
    pub elapsed_ps: Time,
    /// Completed machine-cycle count copied from the CPU cycle state.
    pub machine_cycles: u64,
    /// Completed instruction count copied from the CPU cycle state.
    pub instruction_count: u64,
    /// Logical windows for the completed phase's machine cycle.
    pub windows: LogicalWindows,
    /// Any timing-contract violation observed while recording this sample.
    pub violation: Option<TimingViolation>,
}

/// A violation that prevents timing data from being treated as a valid observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimingViolation {
    /// The profile conflicts with the retained clock bounds.
    InvalidProfile { reason: &'static str },
    /// The caller attempted to record a phase different from the CPU state.
    PhaseMismatch { expected: BusCycle, observed: BusCycle },
    /// A CPU cycle counter moved backward between observations.
    CycleRegression { before: u64, after: u64 },
    /// The picosecond timeline exceeded its representable range.
    TimeOverflow,
}

/// Timing and I/O controller for the 4004.
#[derive(Clone, Debug)]
pub struct TimingIo {
    profile: TimingProfile,
    snapshot: TimingSnapshot,
}

impl TimingIo {
    /// Construct the retained typical timing profile.
    pub fn new() -> Self {
        Self::with_profile(TimingProfile::typical()).expect("the retained typical profile is valid")
    }

    /// Construct a timing controller only when its waveform profile is valid.
    pub fn with_profile(profile: TimingProfile) -> Result<Self, TimingViolation> {
        profile.validate()?;
        let windows = windows_for_cycle(&profile, 0).expect("initial timing window fits in Time");
        Ok(Self {
            profile,
            snapshot: TimingSnapshot {
                completed_phase: None,
                next_phase: BusCycle::A1,
                phase_start_ps: 0,
                elapsed_ps: 0,
                machine_cycles: 0,
                instruction_count: 0,
                windows,
                violation: None,
            },
        })
    }

    /// Return the immutable profile used for all recorded timing samples.
    pub fn profile(&self) -> &TimingProfile {
        &self.profile
    }

    /// Return the most recent timing observation.
    pub fn snapshot(&self) -> &TimingSnapshot {
        &self.snapshot
    }

    /// Reset observation state while retaining the validated timing profile.
    pub fn reset(&mut self) {
        let windows = windows_for_cycle(&self.profile, 0).expect("initial timing window fits in Time");
        self.snapshot = TimingSnapshot {
            completed_phase: None,
            next_phase: BusCycle::A1,
            phase_start_ps: 0,
            elapsed_ps: 0,
            machine_cycles: 0,
            instruction_count: 0,
            windows,
            violation: None,
        };
    }

    /// Record one completed phase from the CPU-owned cycle state.
    pub(crate) fn record_completed_phase(&mut self, executed: BusCycle, before: &CycleState, after: &CycleState) {
        let violation = if before.phase != executed {
            Some(TimingViolation::PhaseMismatch {
                expected: before.phase,
                observed: executed,
            })
        } else if after.cycle_count < before.cycle_count {
            Some(TimingViolation::CycleRegression {
                before: before.cycle_count,
                after: after.cycle_count,
            })
        } else if after.phase != executed.next() {
            Some(TimingViolation::PhaseMismatch {
                expected: executed.next(),
                observed: after.phase,
            })
        } else {
            None
        };

        let (phase_start_ps, elapsed_ps, windows, overflow) = match timing_for_phase(&self.profile, before) {
            Ok((phase_start_ps, elapsed_ps, windows)) => (phase_start_ps, elapsed_ps, windows, false),
            Err(()) => (0, Time::MAX, self.snapshot.windows, true),
        };
        self.snapshot = TimingSnapshot {
            completed_phase: Some(executed),
            next_phase: after.phase,
            phase_start_ps,
            elapsed_ps,
            machine_cycles: after.cycle_count,
            instruction_count: after.instruction_count,
            windows,
            violation: if overflow {
                Some(TimingViolation::TimeOverflow)
            } else {
                violation
            },
        };
    }
}

impl Default for TimingIo {
    fn default() -> Self {
        Self::new()
    }
}

fn timing_for_phase(profile: &TimingProfile, cycle: &CycleState) -> Result<(Time, Time, LogicalWindows), ()> {
    let cycle_width = profile.phase_period_ps.checked_mul(8).ok_or(())?;
    let cycle_start = cycle_width.checked_mul(cycle.cycle_count).ok_or(())?;
    let phase_offset = profile
        .phase_period_ps
        .checked_mul(u64::from(cycle.phase.phase_number()))
        .ok_or(())?;
    let phase_start = cycle_start.checked_add(phase_offset).ok_or(())?;
    let elapsed = phase_start.checked_add(profile.phase_period_ps).ok_or(())?;
    let windows = windows_for_cycle(profile, cycle.cycle_count).map_err(|_| ())?;
    Ok((phase_start, elapsed, windows))
}

fn windows_for_cycle(profile: &TimingProfile, machine_cycle: u64) -> Result<LogicalWindows, ()> {
    let cycle_width = profile.phase_period_ps.checked_mul(8).ok_or(())?;
    let cycle_start = cycle_width.checked_mul(machine_cycle).ok_or(())?;
    Ok(LogicalWindows {
        sync: window_for_phase(profile, cycle_start, cycle_timing::SYNC_ASSERT)?,
        cm_rom: window_for_phase(profile, cycle_start, cycle_timing::CM_ROM_VALID)?,
        cm_ram_candidate: window_for_phase(profile, cycle_start, cycle_timing::CM_RAM_VALID)?,
    })
}

fn window_for_phase(profile: &TimingProfile, cycle_start: Time, phase: BusCycle) -> Result<TimingWindow, ()> {
    let offset = profile
        .phase_period_ps
        .checked_mul(u64::from(phase.phase_number()))
        .ok_or(())?;
    let start_ps = cycle_start.checked_add(offset).ok_or(())?;
    let end_ps = start_ps.checked_add(profile.phase_period_ps).ok_or(())?;
    Ok(TimingWindow {
        phase,
        start_ps,
        end_ps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typical_profile_preserves_the_logical_phase_period() {
        let profile = TimingProfile::typical();
        assert_eq!(profile.phase_period_ps, clock_spec::TCY_TYP);
        assert_eq!(
            profile.phi1_width_ps + profile.phi1_to_phi2_gap_ps + profile.phi2_width_ps + profile.phi2_to_phi1_gap_ps,
            profile.phase_period_ps
        );
        assert!(profile.phi2_to_phi1_gap_ps >= clock_spec::T0D2_MIN);
    }

    #[test]
    fn invalid_profile_rejects_a_period_mismatch() {
        let mut profile = TimingProfile::typical();
        profile.phase_period_ps += 1;
        assert!(matches!(
            TimingIo::with_profile(profile),
            Err(TimingViolation::InvalidProfile { .. })
        ));
    }

    #[test]
    fn records_phase_time_and_logical_windows() {
        let mut timing = TimingIo::new();
        let before = CycleState::new();
        let mut after = before.clone();
        after.advance();
        timing.record_completed_phase(BusCycle::A1, &before, &after);

        let snapshot = timing.snapshot();
        assert_eq!(snapshot.completed_phase, Some(BusCycle::A1));
        assert_eq!(snapshot.next_phase, BusCycle::A2);
        assert_eq!(snapshot.phase_start_ps, 0);
        assert_eq!(snapshot.elapsed_ps, clock_spec::TCY_TYP);
        assert_eq!(snapshot.windows.sync.phase, BusCycle::A1);
        assert_eq!(snapshot.windows.cm_rom.phase, BusCycle::A3);
        assert_eq!(snapshot.windows.cm_ram_candidate.phase, BusCycle::X2);
        assert_eq!(snapshot.violation, None);
    }

    #[test]
    fn records_phase_mismatch_without_mutating_cpu_state() {
        let mut timing = TimingIo::new();
        let before = CycleState::new();
        let mut after = before.clone();
        after.advance();
        timing.record_completed_phase(BusCycle::A2, &before, &after);
        assert!(matches!(
            timing.snapshot().violation,
            Some(TimingViolation::PhaseMismatch {
                expected: BusCycle::A1,
                observed: BusCycle::A2,
            })
        ));
    }
}
