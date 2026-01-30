//! Signal trace buffer for waveform capture and debugging
//!
//! Captures signal values at each bus cycle phase with configurable depth.
//! Provides efficient storage and retrieval of signal history for debugging
//! and waveform visualization.

use std::collections::VecDeque;

/// A single captured signal value at a specific time
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalSample {
    /// Simulation cycle number
    pub cycle: u64,

    /// Bus phase in current cycle (0-7 for A1-A3, M1-M2, X1-X3)
    pub phase: u8,

    /// Signal name (e.g., "ACC", "PC", "DATA[3:0]")
    pub name: String,

    /// Signal value (can be 4-bit, 8-bit, 12-bit, 16-bit depending on signal)
    pub value: u16,

    /// Signal width in bits
    pub width: u8,

    /// Optional event description (e.g., "INT sampled", "WRITE", "PC increment")
    pub event: Option<String>,
}

/// Signal trace buffer for capturing execution history
///
/// Stores signal samples in a circular buffer with configurable capacity.
/// Provides windowing, querying, and event filtering.
#[derive(Clone, Debug)]
pub struct TraceBuffer {
    /// Circular buffer of signal samples
    samples: VecDeque<SignalSample>,

    /// Maximum number of samples before oldest are discarded
    capacity: usize,

    /// Total samples ever recorded (for cycle numbering)
    sample_count: u64,

    /// Current simulation cycle
    current_cycle: u64,

    /// Sampling enabled flag
    enabled: bool,

    /// Sample every N cycles (1 = capture every cycle, 2 = every other, etc.)
    sample_rate: u64,

    /// Capture detailed events (may reduce performance)
    capture_events: bool,
}

impl TraceBuffer {
    /// Create new trace buffer with default capacity (10K samples)
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    /// Create trace buffer with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
            sample_count: 0,
            current_cycle: 0,
            enabled: true,
            sample_rate: 1,
            capture_events: true,
        }
    }

    /// Record a signal sample
    pub fn record(&mut self, name: String, value: u16, width: u8) {
        self.record_with_event(name, value, width, None);
    }

    /// Record a signal sample with event description
    pub fn record_with_event(
        &mut self,
        name: String,
        value: u16,
        width: u8,
        event: Option<String>,
    ) {
        if !self.enabled || self.sample_count % self.sample_rate != 0 {
            self.sample_count += 1;
            return;
        }

        let sample = SignalSample {
            cycle: self.current_cycle,
            phase: (self.sample_count % 8) as u8,
            name,
            value,
            width,
            event: if self.capture_events { event } else { None },
        };

        // Add to buffer, removing oldest if at capacity
        if self.samples.len() >= self.capacity {
            self.samples.pop_front();
        }

        self.samples.push_back(sample);
        self.sample_count += 1;
    }

    /// Advance to next bus cycle phase (auto-called from system tick)
    pub fn tick(&mut self) {
        // Phase tracking is implicit in sample_count % 8
        // This method exists for explicit cycle advancement if needed
    }

    /// Advance to next complete cycle
    pub fn next_cycle(&mut self) {
        self.current_cycle += 1;
        self.sample_count = self.current_cycle * 8; // Reset phase counter
    }

    /// Get all recorded samples
    pub fn samples(&self) -> Vec<SignalSample> {
        self.samples.iter().cloned().collect()
    }

    /// Get samples within a cycle range
    pub fn samples_in_range(&self, start_cycle: u64, end_cycle: u64) -> Vec<SignalSample> {
        self.samples
            .iter()
            .filter(|s| s.cycle >= start_cycle && s.cycle <= end_cycle)
            .cloned()
            .collect()
    }

    /// Get samples for a specific signal
    pub fn samples_for_signal(&self, name: &str) -> Vec<SignalSample> {
        self.samples
            .iter()
            .filter(|s| s.name == name)
            .cloned()
            .collect()
    }

    /// Get samples matching a signal pattern (supports wildcards)
    pub fn samples_matching(&self, pattern: &str) -> Vec<SignalSample> {
        self.samples
            .iter()
            .filter(|s| {
                // Simple pattern matching (not full regex)
                if pattern.contains('*') {
                    s.name.contains(&pattern.replace("*", ""))
                } else {
                    s.name == pattern
                }
            })
            .cloned()
            .collect()
    }

    /// Get samples with events
    pub fn samples_with_events(&self) -> Vec<SignalSample> {
        self.samples
            .iter()
            .filter(|s| s.event.is_some())
            .cloned()
            .collect()
    }

    /// Clear all samples
    pub fn clear(&mut self) {
        self.samples.clear();
        self.sample_count = 0;
        self.current_cycle = 0;
    }

    /// Get current buffer size (number of samples stored)
    pub fn size(&self) -> usize {
        self.samples.len()
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current cycle number
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get total samples recorded (including discarded)
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Enable/disable recording
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set sample rate (1 = every sample, N = every Nth sample)
    pub fn set_sample_rate(&mut self, rate: u64) {
        self.sample_rate = rate.max(1);
    }

    /// Set event capture flag
    pub fn set_capture_events(&mut self, enabled: bool) {
        self.capture_events = enabled;
    }

    /// Get buffer utilization percentage (0-100)
    pub fn utilization(&self) -> usize {
        (self.samples.len() * 100) / self.capacity
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.samples.len() >= self.capacity
    }

    /// Export samples as CSV format for external analysis
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("cycle,phase,name,value,width,event\n");
        for sample in &self.samples {
            csv.push_str(&format!(
                "{},{},{},{:04X},{},{}\n",
                sample.cycle,
                sample.phase,
                sample.name,
                sample.value,
                sample.width,
                sample.event.as_deref().unwrap_or("")
            ));
        }
        csv
    }

    /// Get latest sample for a signal (most recent captured)
    pub fn latest(&self, name: &str) -> Option<SignalSample> {
        self.samples
            .iter()
            .rev()
            .find(|s| s.name == name)
            .cloned()
    }

    /// Get value transitions for a signal (changes over time)
    pub fn transitions(&self, name: &str) -> Vec<(u64, u16, u16)> {
        let mut transitions = Vec::new();
        let samples = self.samples_for_signal(name);

        if samples.len() < 2 {
            return transitions;
        }

        for window in samples.windows(2) {
            if window[0].value != window[1].value {
                transitions.push((window[0].cycle, window[0].value, window[1].value));
            }
        }

        transitions
    }

    /// Find cycles where a specific event occurred
    pub fn events_matching(&self, event: &str) -> Vec<u64> {
        self.samples
            .iter()
            .filter(|s| {
                s.event
                    .as_ref()
                    .map(|e| e.contains(event))
                    .unwrap_or(false)
            })
            .map(|s| s.cycle)
            .collect()
    }
}

impl Default for TraceBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = TraceBuffer::new();
        assert_eq!(buf.size(), 0);
        assert_eq!(buf.sample_count(), 0);
    }

    #[test]
    fn test_record_single_sample() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        assert_eq!(buf.size(), 1);
        assert_eq!(buf.sample_count(), 1);
    }

    #[test]
    fn test_record_multiple_samples() {
        let mut buf = TraceBuffer::new();
        for i in 0..10 {
            buf.record(format!("SIG{}", i % 2), i, 4);
        }
        assert_eq!(buf.size(), 10);
    }

    #[test]
    fn test_samples_for_signal() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record("PC".to_string(), 0x100, 12);
        buf.record("ACC".to_string(), 0x6, 4);

        let acc_samples = buf.samples_for_signal("ACC");
        assert_eq!(acc_samples.len(), 2);
        assert_eq!(acc_samples[0].value, 0x5);
        assert_eq!(acc_samples[1].value, 0x6);
    }

    #[test]
    fn test_record_with_event() {
        let mut buf = TraceBuffer::new();
        buf.record_with_event(
            "INT".to_string(),
            1,
            1,
            Some("INT sampled".to_string()),
        );

        let samples = buf.samples();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].event, Some("INT sampled".to_string()));
    }

    #[test]
    fn test_samples_with_events() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record_with_event("INT".to_string(), 1, 1, Some("Interrupt".to_string()));
        buf.record("PC".to_string(), 0x100, 12);

        let event_samples = buf.samples_with_events();
        assert_eq!(event_samples.len(), 1);
        assert_eq!(event_samples[0].name, "INT");
    }

    #[test]
    fn test_clear_buffer() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record("PC".to_string(), 0x100, 12);

        buf.clear();
        assert_eq!(buf.size(), 0);
        assert_eq!(buf.sample_count(), 0);
    }

    #[test]
    fn test_capacity_limit() {
        let mut buf = TraceBuffer::with_capacity(10);
        for i in 0..20 {
            buf.record(format!("SIG{}", i), i as u16, 4);
        }

        // Should not exceed capacity
        assert_eq!(buf.size(), 10);
        // Should have recorded all samples
        assert_eq!(buf.sample_count(), 20);
    }

    #[test]
    fn test_enabled_flag() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        assert_eq!(buf.size(), 1);

        buf.set_enabled(false);
        buf.record("PC".to_string(), 0x100, 12);
        assert_eq!(buf.size(), 1); // No new sample recorded
    }

    #[test]
    fn test_sample_rate() {
        let mut buf = TraceBuffer::new();
        buf.set_sample_rate(2); // Every other sample

        for i in 0..10 {
            buf.record(format!("SIG"), i as u16, 4);
        }

        // Should have captured samples 0, 2, 4, 6, 8 (approximately)
        assert!(buf.size() <= 5);
    }

    #[test]
    fn test_latest_sample() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record("PC".to_string(), 0x100, 12);
        buf.record("ACC".to_string(), 0x6, 4);

        let latest = buf.latest("ACC").unwrap();
        assert_eq!(latest.value, 0x6);
    }

    #[test]
    fn test_transitions() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record("ACC".to_string(), 0x6, 4);
        buf.record("ACC".to_string(), 0x6, 4);
        buf.record("ACC".to_string(), 0x7, 4);

        let trans = buf.transitions("ACC");
        assert_eq!(trans.len(), 2); // Two transitions: 5→6, 6→7
    }

    #[test]
    fn test_events_matching() {
        let mut buf = TraceBuffer::new();
        buf.record_with_event(
            "INT".to_string(),
            1,
            1,
            Some("Interrupt at 0x003".to_string()),
        );
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record_with_event(
            "INT".to_string(),
            0,
            1,
            Some("Interrupt cleared".to_string()),
        );

        let events = buf.events_matching("Interrupt");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_samples_in_range() {
        let mut buf = TraceBuffer::new();
        for cycle in 0..10 {
            buf.record(format!("SIG"), cycle as u16, 4);
            buf.next_cycle();
        }

        let range_samples = buf.samples_in_range(2, 5);
        assert!(range_samples.iter().all(|s| s.cycle >= 2 && s.cycle <= 5));
    }

    #[test]
    fn test_utilization() {
        let mut buf = TraceBuffer::with_capacity(100);
        for i in 0..50 {
            buf.record(format!("SIG"), i as u16, 4);
        }

        let util = buf.utilization();
        assert_eq!(util, 50); // 50%
    }

    #[test]
    fn test_is_full() {
        let mut buf = TraceBuffer::with_capacity(10);
        assert!(!buf.is_full());

        for i in 0..10 {
            buf.record(format!("SIG"), i as u16, 4);
        }

        assert!(buf.is_full());
    }

    #[test]
    fn test_csv_export() {
        let mut buf = TraceBuffer::new();
        buf.record("ACC".to_string(), 0x5, 4);
        buf.record("PC".to_string(), 0x100, 12);

        let csv = buf.to_csv();
        assert!(csv.contains("ACC"));
        assert!(csv.contains("PC"));
        assert!(csv.contains("0005")); // Hex value
        assert!(csv.contains("0100")); // Hex value
    }
}
