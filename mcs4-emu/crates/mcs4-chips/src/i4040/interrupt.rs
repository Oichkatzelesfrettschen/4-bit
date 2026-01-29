//! 4040 Interrupt Controller

/// Interrupt controller state
#[derive(Clone, Debug)]
pub struct InterruptController {
    /// Interrupts enabled (EIN/DIN)
    enabled: bool,
    /// INT pin state
    int_pin: bool,
    /// Interrupt pending (recognized but not yet serviced)
    pending: bool,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            enabled: false,
            int_pin: false,
            pending: false,
        }
    }

    /// Get interrupt enabled state
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Get interrupt pending state
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// Enable interrupts (EIN instruction)
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable interrupts (DIN instruction)
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set INT pin state (from external hardware)
    pub fn set_int_pin(&mut self, state: bool) {
        self.int_pin = state;
    }

    /// Check if interrupt should be serviced
    /// Returns true if INT asserted and enabled
    pub fn should_service(&mut self) -> bool {
        if self.int_pin && self.enabled && !self.pending {
            self.pending = true;
            true
        } else {
            false
        }
    }

    /// Acknowledge interrupt (automatically disables)
    pub fn acknowledge(&mut self) {
        self.pending = false;
        self.enabled = false; // Auto-disable on interrupt
    }

    /// Clear pending interrupt
    pub fn clear_pending(&mut self) {
        self.pending = false;
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_enable_disable() {
        let mut int = InterruptController::new();
        assert!(!int.enabled());

        int.enable();
        assert!(int.enabled());

        int.disable();
        assert!(!int.enabled());
    }

    #[test]
    fn test_interrupt_service() {
        let mut int = InterruptController::new();
        
        // INT asserted but not enabled - no service
        int.set_int_pin(true);
        assert!(!int.should_service());

        // Enable interrupts - should service
        int.enable();
        assert!(int.should_service());

        // Acknowledge - disables interrupts
        int.acknowledge();
        assert!(!int.enabled());
        assert!(!int.pending);
    }

    #[test]
    fn test_int_pin_edge() {
        let mut int = InterruptController::new();
        int.enable();
        
        // INT goes high - service once
        int.set_int_pin(true);
        assert!(int.should_service());
        
        // INT still high - no re-service until acknowledged
        assert!(!int.should_service());
        
        // Acknowledge
        int.acknowledge();
        
        // INT goes low then high - service again
        int.set_int_pin(false);
        int.set_int_pin(true);
        int.enable();
        assert!(int.should_service());
    }
}
