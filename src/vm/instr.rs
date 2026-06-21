use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return,
    Const(Reg, Imm),
    PrintInt(Reg),
    Add { r0: Reg, r1: Reg, rd: Reg },
    Sub { r0: Reg, r1: Reg, rd: Reg },
    Move { r0: Reg, rd: Reg },
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
            Self::Const(dst, imm) => EncodedInstr::opcode(1).reg0(dst).imm1(imm),
            Self::PrintInt(src) => EncodedInstr::opcode(2).reg0(src),
            Self::Add { r0, r1, rd } => EncodedInstr::opcode(3).reg0(r0).reg1(r1).reg2(rd),
            Self::Sub { r0, r1, rd } => EncodedInstr::opcode(4).reg0(r0).reg1(r1).reg2(rd),
            Self::Move { r0, rd } => EncodedInstr::opcode(5).reg0(r0).reg1(rd),
        }
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match enc.get_opcode() {
            0 => Self::Return,
            1 => Self::Const(enc.get_reg0(), enc.get_imm1()),
            2 => Self::PrintInt(enc.get_reg0()),
            3 => Self::Add { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            4 => Self::Sub { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            5 => Self::Move { r0: enc.get_reg0(), rd: enc.get_reg1() },
            _ => panic!("illegal instruction"),
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
            Self::Const(dst, imm) => w.mnem("const").reg(dst).imm(imm),
            Self::PrintInt(src) => w.mnem("printi").reg(src),
            Self::Add { r0, r1, rd } => w.mnem("add").reg(rd).reg(r0).reg(r1),
            Self::Sub { r0, r1, rd } => w.mnem("sub").reg(rd).reg(r0).reg(r1),
            Self::Move { r0, rd } => w.mnem("move").reg(rd).reg(r0),
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
        test(Instr::Const(Reg(0), Imm(0)));
        test(Instr::Const(Reg(78), Imm(89)));
        test(Instr::PrintInt(Reg(0)));
        test(Instr::PrintInt(Reg(86)));
        test(Instr::PrintInt(Reg(234)));
    }
}
