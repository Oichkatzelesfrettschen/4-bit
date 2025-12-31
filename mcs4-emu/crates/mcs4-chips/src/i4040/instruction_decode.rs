// 4040 instruction decode placeholder for new opcodes
#[derive(Clone, Copy, Debug)]
pub enum Opcode4040 {
    Hlt, Bbs, Lcr, Or4, Or5, An6, An7, Db0, Db1, Sb0, Sb1, Ein, Din, Rpm,
}

pub fn decode_ext(op: u8) -> Option<Opcode4040> {
    match op {
        0x01 => Some(Opcode4040::Hlt),
        0x02 => Some(Opcode4040::Bbs),
        0x03 => Some(Opcode4040::Lcr),
        0x04 => Some(Opcode4040::Or4),
        0x05 => Some(Opcode4040::Or5),
        0x06 => Some(Opcode4040::An6),
        0x07 => Some(Opcode4040::An7),
        0x08 => Some(Opcode4040::Db0),
        0x09 => Some(Opcode4040::Db1),
        0x0A => Some(Opcode4040::Sb0),
        0x0B => Some(Opcode4040::Sb1),
        0x0C => Some(Opcode4040::Ein),
        0x0D => Some(Opcode4040::Din),
        0x0E => Some(Opcode4040::Rpm),
        _ => None,
    }
}
