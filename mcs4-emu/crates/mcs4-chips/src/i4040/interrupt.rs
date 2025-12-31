//! 4040 interrupt controller: EIN/DIN, INT vector 0x003, SRC save/restore.

#[derive(Default, Debug, Clone)]
pub struct InterruptCtrl {
    pub enabled: bool,
    pub pending: bool,
    pub src_save: u8,
}

impl InterruptCtrl {
    #[inline]
    pub fn ein(&mut self) { self.enabled = true; }
    #[inline]
    pub fn din(&mut self) { self.enabled = false; }

    /// Request an interrupt; CPU should check and service at instruction boundary.
    pub fn request(&mut self) { if self.enabled { self.pending = true; } }

    /// Service the interrupt: returns vector address; disables further interrupts.
    pub fn service(&mut self, current_src: u8) -> Option<u16> {
        if self.pending && self.enabled {
            self.src_save = current_src;
            self.enabled = false; // auto-disable
            self.pending = false;
            Some(0x003)
        } else { None }
    }

    /// Branch Back from Service (BBS): restore SRC and re-enable optionally.
    pub fn bbs_restore(&mut self) -> u8 { self.src_save }
}
