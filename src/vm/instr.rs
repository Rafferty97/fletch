use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return,
    Const(Reg, Imm),
    PrintInt(Reg),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EncodedInstr(u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Width {
    _8 = 0,
    _16 = 1,
    _32 = 2,
    _64 = 3,
}

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
        }
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match enc.get_opcode() {
            0 => Self::Return,
            1 => Self::Const(enc.get_reg0(), enc.get_imm1()),
            2 => Self::PrintInt(enc.get_reg0()),
            _ => panic!("illegal instruction"),
        }
    }
}

impl EncodedInstr {
    fn opcode(opcode: u8) -> Self {
        Self(opcode as u32)
    }

    fn width(self, width: Width) -> Self {
        Self(self.0 | ((width as u32) << 6))
    }

    fn reg0(self, reg: Reg) -> Self {
        Self(self.0 | ((reg.0 as u32) << 8))
    }

    fn imm1(self, imm: Imm) -> Self {
        Self(self.0 | ((imm.0 as u32) << 16))
    }

    fn get_opcode(self) -> u8 {
        (self.0 & 0x3f) as u8
    }

    fn get_width(self) -> Width {
        match ((self.0 >> 6) & 0x03) {
            0 => Width::_8,
            1 => Width::_16,
            2 => Width::_32,
            3 => Width::_64,
            _ => unreachable!(),
        }
    }

    fn get_reg0(self) -> Reg {
        Reg(((self.0 >> 8) & 0xff) as _)
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

            fn width(mut self, width: Width) -> Self {
                self.instr_len += 1;
                match width {
                    Width::_8 => write!(self.f, "b"),
                    Width::_16 => write!(self.f, "w"),
                    Width::_32 => write!(self.f, "l"),
                    Width::_64 => write!(self.f, "q"),
                };
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
        };
        Ok(())
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
        test(Instr::Const(Reg(0), Imm(0)));
        test(Instr::Const(Reg(78), Imm(89)));
        test(Instr::PrintInt(Reg(0)));
        test(Instr::PrintInt(Reg(86)));
        test(Instr::PrintInt(Reg(234)));
    }
}
