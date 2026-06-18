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
            Self::Return => EncodedInstr(0),
        }
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match enc.0 {
            0 => Self::Return,
            _ => panic!("illegal instruction"),
        }
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return => write!(f, "ret"),
        }
    }
}
