//! Hybrid event-driven + transient simulation scheduler.
//!
//! For efficiency, the solver does not re-evaluate the entire circuit at
//! every time step. Instead, it uses an event queue to track which nodes
//! have pending changes, and only re-solves the affected subcircuit.
//!
//! This is similar to the event-driven pattern used in the gate-level
//! simulator (see `simulator.rs`), extended to work with the analog solver.

use std::{cmp::Reverse, collections::BinaryHeap};

/// A scheduled simulation event.
#[derive(Clone, Debug, PartialEq)]
pub struct SimEvent {
    /// Event time (seconds).
    pub time: f64,
    /// Node index that changed.
    pub node: usize,
    /// New voltage value at this node.
    pub voltage: f64,
}

// Ordering for the min-heap: earlier events first.
impl Eq for SimEvent {}

impl PartialOrd for SimEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SimEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by time, then node index for stability
        self.time
            .total_cmp(&other.time)
            .then_with(|| self.node.cmp(&other.node))
    }
}

/// Event-driven simulation scheduler.
pub struct EventScheduler {
    /// Priority queue of pending events (min-heap by time).
    queue: BinaryHeap<Reverse<SimEvent>>,
    /// Current simulation time (seconds).
    pub current_time: f64,
    /// Total events processed.
    pub events_processed: u64,
}

impl EventScheduler {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            current_time: 0.0,
            events_processed: 0,
        }
    }

    /// Schedule a new event.
    pub fn schedule(&mut self, event: SimEvent) {
        assert!(event.time >= self.current_time, "Cannot schedule event in the past");
        self.queue.push(Reverse(event));
    }

    /// Schedule a voltage change at a given time.
    pub fn schedule_change(&mut self, time: f64, node: usize, voltage: f64) {
        self.schedule(SimEvent { time, node, voltage });
    }

    /// Pop the next event from the queue.
    ///
    /// Returns None if the queue is empty.
    pub fn next_event(&mut self) -> Option<SimEvent> {
        self.queue.pop().map(|Reverse(event)| {
            self.current_time = event.time;
            self.events_processed += 1;
            event
        })
    }

    /// Peek at the next event time without consuming it.
    pub fn peek_time(&self) -> Option<f64> {
        self.queue.peek().map(|Reverse(e)| e.time)
    }

    /// Number of pending events.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Clear all pending events.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl Default for EventScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_processed_in_time_order() {
        let mut sched = EventScheduler::new();

        sched.schedule_change(3.0, 0, 5.0);
        sched.schedule_change(1.0, 1, 0.0);
        sched.schedule_change(2.0, 2, -15.0);

        let e1 = sched.next_event().expect("event 1");
        assert!((e1.time - 1.0).abs() < 1e-10);
        assert_eq!(e1.node, 1);

        let e2 = sched.next_event().expect("event 2");
        assert!((e2.time - 2.0).abs() < 1e-10);

        let e3 = sched.next_event().expect("event 3");
        assert!((e3.time - 3.0).abs() < 1e-10);

        assert!(sched.is_empty());
    }

    #[test]
    fn empty_scheduler() {
        let mut sched = EventScheduler::new();
        assert!(sched.is_empty());
        assert!(sched.next_event().is_none());
        assert_eq!(sched.pending_count(), 0);
    }

    #[test]
    fn peek_does_not_consume() {
        let mut sched = EventScheduler::new();
        sched.schedule_change(1.0, 0, 5.0);

        assert_eq!(sched.peek_time(), Some(1.0));
        assert_eq!(sched.pending_count(), 1);
        assert_eq!(sched.peek_time(), Some(1.0)); // still there
    }

    #[test]
    fn clear_removes_all() {
        let mut sched = EventScheduler::new();
        sched.schedule_change(1.0, 0, 5.0);
        sched.schedule_change(2.0, 1, 0.0);

        sched.clear();
        assert!(sched.is_empty());
    }

    #[test]
    fn current_time_advances() {
        let mut sched = EventScheduler::new();
        sched.schedule_change(1.5, 0, 5.0);

        assert!((sched.current_time - 0.0).abs() < 1e-10);
        sched.next_event();
        assert!((sched.current_time - 1.5).abs() < 1e-10);
    }

    #[test]
    fn events_processed_counter() {
        let mut sched = EventScheduler::new();
        sched.schedule_change(1.0, 0, 5.0);
        sched.schedule_change(2.0, 1, 0.0);

        assert_eq!(sched.events_processed, 0);
        sched.next_event();
        assert_eq!(sched.events_processed, 1);
        sched.next_event();
        assert_eq!(sched.events_processed, 2);
    }

    #[test]
    fn same_time_events_stable() {
        let mut sched = EventScheduler::new();
        sched.schedule_change(1.0, 5, 0.0);
        sched.schedule_change(1.0, 2, 5.0);
        sched.schedule_change(1.0, 8, -15.0);

        // Should all have time=1.0, ordered by node index
        let e1 = sched.next_event().expect("e1");
        let e2 = sched.next_event().expect("e2");
        let e3 = sched.next_event().expect("e3");

        assert!(
            e1.node <= e2.node && e2.node <= e3.node,
            "Same-time events should be ordered by node: {}, {}, {}",
            e1.node,
            e2.node,
            e3.node
        );
    }
}
