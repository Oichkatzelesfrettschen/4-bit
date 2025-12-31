// 4040 interrupt controller scaffolding
pub struct InterruptCtrl {
    pub enabled: bool,
    pub pending: bool,
    pub src_save: u8,
}

impl InterruptCtrl {
    pub fn new() -> Self { Self { enabled: false, pending: false, src_save: 0 } }
}
