use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EncodedInstr(u32);

impl Instr {
    pub fn encode(self) -> EncodedInstr {
        match self {
            Self::Return => Self::encode_bare(0),
        }
    }

    fn encode_bare(opcode: u8) -> EncodedInstr {
        EncodedInstr(opcode as u32)
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match Self::get_opcode(enc) {
            0 => Self::Return,
            _ => panic!("illegal instruction"),
        }
    }

    fn get_opcode(enc: EncodedInstr) -> u8 {
        (enc.0 & 0x3f) as u8
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return => write!(f, "ret"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instr_roundtrip() {
        let test = |instr: Instr| {
            assert_eq!(instr, Instr::decode(instr.encode()));
        };

        test(Instr::Return);
    }
}
