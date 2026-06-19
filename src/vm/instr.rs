use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return,
    Const(Reg, Imm),
    Print(Reg),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EncodedInstr(u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reg(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Imm(pub u16);

impl Instr {
    pub fn encode(self) -> EncodedInstr {
        match self {
            Self::Return => Self::encode_bare(0),
            Self::Const(dst, imm) => Self::encode_reg_imm(1, dst, imm),
            Self::Print(src) => Self::encode_reg(2, src),
        }
    }

    fn encode_bare(opcode: u8) -> EncodedInstr {
        EncodedInstr(opcode as u32)
    }

    fn encode_reg(opcode: u8, reg: Reg) -> EncodedInstr {
        let mut i = opcode as u32;
        i |= (reg.0 as u32) << 6;
        EncodedInstr(i)
    }

    fn encode_reg_imm(opcode: u8, reg: Reg, imm: Imm) -> EncodedInstr {
        let mut i = opcode as u32;
        i |= (reg.0 as u32) << 6;
        i |= (imm.0 as u32) << 15;
        EncodedInstr(i)
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match Self::get_opcode(enc) {
            0 => Self::Return,
            1 => Self::Const(Self::get_reg0(enc), Self::get_imm1(enc)),
            2 => Self::Print(Self::get_reg0(enc)),
            _ => panic!("illegal instruction"),
        }
    }

    fn get_opcode(enc: EncodedInstr) -> u8 {
        (enc.0 & 0x3f) as u8
    }

    fn get_reg0(enc: EncodedInstr) -> Reg {
        Reg(((enc.0 >> 6) & 0x1ff) as _)
    }

    fn get_imm1(enc: EncodedInstr) -> Imm {
        Imm(((enc.0 >> 15) & 0x1ff) as _)
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Formatter;
        let write_bare = |f: &mut Formatter<'_>, op| write!(f, "{op}");
        let write_reg = |f: &mut Formatter<'_>, op, arg0| write!(f, "{op:<6 } {arg0}");
        let write_reg_imm = |f: &mut Formatter<'_>, op, arg0, imm1| write!(f, "{op:<6 } {arg0}, {imm1}");

        match self {
            Self::Return => write_bare(f, "ret"),
            Self::Const(dst, imm) => write_reg_imm(f, "const", dst, imm),
            Self::Print(src) => write_reg(f, "print", src),
        }
    }
}

impl Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
    }
}

impl Display for Imm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}", self.0)
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
        test(Instr::Print(Reg(0)));
        test(Instr::Print(Reg(248)));
    }
}
