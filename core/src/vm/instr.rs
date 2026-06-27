use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return,
    Load(Reg, Imm),
    Print(Reg),
    Add { r0: Reg, r1: Reg, rd: Reg },
    Sub { r0: Reg, r1: Reg, rd: Reg },
    Mul { r0: Reg, r1: Reg, rd: Reg },
    UDiv { r0: Reg, r1: Reg, rd: Reg },
    SDiv { r0: Reg, r1: Reg, rd: Reg },
    Eq { r0: Reg, r1: Reg, rd: Reg },
    ULt { r0: Reg, r1: Reg, rd: Reg },
    SLt { r0: Reg, r1: Reg, rd: Reg },
    Not { r0: Reg, rd: Reg },
    Move { r0: Reg, rd: Reg },
    MakeArray { r0: Reg, rn: Reg, rd: Reg },
    Index { r0: Reg, r1: Reg, rd: Reg },
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
            Self::Return => EncodedInstr::opcode(0),
            Self::Load(dst, imm) => EncodedInstr::opcode(1).reg0(dst).imm1(imm),
            Self::Print(src) => EncodedInstr::opcode(2).reg0(src),
            Self::Add { r0, r1, rd } => EncodedInstr::opcode(10).reg0(r0).reg1(r1).reg2(rd),
            Self::Sub { r0, r1, rd } => EncodedInstr::opcode(11).reg0(r0).reg1(r1).reg2(rd),
            Self::Mul { r0, r1, rd } => EncodedInstr::opcode(12).reg0(r0).reg1(r1).reg2(rd),
            Self::UDiv { r0, r1, rd } => EncodedInstr::opcode(13).reg0(r0).reg1(r1).reg2(rd),
            Self::SDiv { r0, r1, rd } => EncodedInstr::opcode(14).reg0(r0).reg1(r1).reg2(rd),
            Self::Eq { r0, r1, rd } => EncodedInstr::opcode(15).reg0(r0).reg1(r1).reg2(rd),
            Self::ULt { r0, r1, rd } => EncodedInstr::opcode(16).reg0(r0).reg1(r1).reg2(rd),
            Self::SLt { r0, r1, rd } => EncodedInstr::opcode(17).reg0(r0).reg1(r1).reg2(rd),
            Self::Not { r0, rd } => EncodedInstr::opcode(18).reg0(r0).reg2(rd),
            Self::Move { r0, rd } => EncodedInstr::opcode(20).reg0(r0).reg1(rd),
            Self::MakeArray { r0, rn, rd } => EncodedInstr::opcode(21).reg0(r0).reg1(rn).reg2(rd),
            Self::Index { r0, r1, rd } => EncodedInstr::opcode(22).reg0(r0).reg1(r1).reg2(rd),
        }
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match enc.get_opcode() {
            0 => Self::Return,
            1 => Self::Load(enc.get_reg0(), enc.get_imm1()),
            2 => Self::Print(enc.get_reg0()),
            10 => Self::Add { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            11 => Self::Sub { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            12 => Self::Mul { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            13 => Self::UDiv { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            14 => Self::SDiv { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            15 => Self::Eq { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            16 => Self::ULt { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            17 => Self::SLt { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            18 => Self::Not { r0: enc.get_reg0(), rd: enc.get_reg2() },
            20 => Self::Move { r0: enc.get_reg0(), rd: enc.get_reg1() },
            21 => Self::MakeArray { r0: enc.get_reg0(), rn: enc.get_reg1(), rd: enc.get_reg2() },
            22 => Self::Index { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            i => panic!("illegal instruction: {i}"),
        }
    }
}

impl EncodedInstr {
    fn opcode(opcode: u8) -> Self {
        Self(opcode as u32)
    }

    fn reg0(self, reg: Reg) -> Self {
        Self(self.0 | ((reg.0 as u32) << 8))
    }

    fn reg1(self, reg: Reg) -> Self {
        Self(self.0 | ((reg.0 as u32) << 16))
    }

    fn reg2(self, reg: Reg) -> Self {
        Self(self.0 | ((reg.0 as u32) << 24))
    }

    fn imm1(self, imm: Imm) -> Self {
        Self(self.0 | ((imm.0 as u32) << 16))
    }

    fn get_opcode(self) -> u8 {
        (self.0 & 0x3f) as u8
    }

    fn get_reg0(self) -> Reg {
        Reg(((self.0 >> 8) & 0xff) as _)
    }

    fn get_reg1(self) -> Reg {
        Reg(((self.0 >> 16) & 0xff) as _)
    }

    fn get_reg2(self) -> Reg {
        Reg(((self.0 >> 24) & 0xff) as _)
    }

    fn get_imm1(self) -> Imm {
        Imm(((self.0 >> 16) & 0xff) as _)
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct InstrWriter<'a, 'b> {
            f: &'a mut std::fmt::Formatter<'b>,
            instr_len: usize,
            first_arg: bool,
        }

        impl<'a, 'b> InstrWriter<'a, 'b> {
            fn new(f: &'a mut std::fmt::Formatter<'b>) -> Self {
                Self { f, instr_len: 0, first_arg: true }
            }

            fn mnem(mut self, mnem: &str) -> Self {
                self.instr_len += mnem.len();
                write!(self.f, "{}", mnem);
                self
            }

            fn reg(mut self, reg: Reg) -> Self {
                self.start_arg();
                write!(self.f, "{}", reg);
                self
            }

            fn imm(mut self, imm: Imm) -> Self {
                self.start_arg();
                write!(self.f, "{}", imm);
                self
            }

            fn start_arg(&mut self) {
                if self.first_arg {
                    const WIDTH: &str = "        ";
                    write!(self.f, "{}", &WIDTH[self.instr_len..]);
                    self.first_arg = false;
                } else {
                    write!(self.f, ", ");
                }
            }
        }

        let w = InstrWriter::new(f);
        match *self {
            Self::Return => w.mnem("ret"),
            Self::Load(dst, imm) => w.mnem("load").reg(dst).imm(imm),
            Self::Print(src) => w.mnem("print").reg(src),
            Self::Add { r0, r1, rd } => w.mnem("add").reg(rd).reg(r0).reg(r1),
            Self::Sub { r0, r1, rd } => w.mnem("sub").reg(rd).reg(r0).reg(r1),
            Self::Mul { r0, r1, rd } => w.mnem("mul").reg(rd).reg(r0).reg(r1),
            Self::UDiv { r0, r1, rd } => w.mnem("udiv").reg(rd).reg(r0).reg(r1),
            Self::SDiv { r0, r1, rd } => w.mnem("sdiv").reg(rd).reg(r0).reg(r1),
            Self::Eq { r0, r1, rd } => w.mnem("eq").reg(rd).reg(r0).reg(r1),
            Self::ULt { r0, r1, rd } => w.mnem("ult").reg(rd).reg(r0).reg(r1),
            Self::SLt { r0, r1, rd } => w.mnem("slt").reg(rd).reg(r0).reg(r1),
            Self::Not { r0, rd } => w.mnem("not").reg(rd).reg(r0),
            Self::Move { r0, rd } => w.mnem("move").reg(rd).reg(r0),
            Self::MakeArray { r0, rn, rd } => w.mnem("mk.arr").reg(rd).reg(r0).reg(rn),
            Self::Index { r0, r1, rd } => w.mnem("index").reg(rd).reg(r0).reg(r1),
        };
        Ok(())
    }
}

impl Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}", self.0)
    }
}

impl Display for Imm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.0)
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
        test(Instr::Load(Reg(0), Imm(0)));
        test(Instr::Load(Reg(78), Imm(89)));
        test(Instr::Print(Reg(0)));
        test(Instr::Print(Reg(86)));
        test(Instr::Print(Reg(234)));
        test(Instr::Add { r0: Reg(1), r1: Reg(2), rd: Reg(3) });
        test(Instr::Add { r0: Reg(10), r1: Reg(20), rd: Reg(30) });
        test(Instr::Add { r0: Reg(100), r1: Reg(200), rd: Reg(250) });
        test(Instr::Add { r0: Reg(255), r1: Reg(255), rd: Reg(255) });
    }
}
