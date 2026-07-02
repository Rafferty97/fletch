use std::fmt::Display;

use crate::types::ty::{FloatTy, IntTy, UIntTy};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return { r0: Reg },
    LoadUnit { rd: Reg },
    LoadNull { rd: Reg },
    LoadFalse { rd: Reg },
    LoadTrue { rd: Reg },
    LoadZero { w: Width, rd: Reg },
    LoadFZero { w: Width, rd: Reg },
    Load { rd: Reg, imm: Imm },
    Print { r0: Reg },
    Add { w: Width, r0: Reg, r1: Reg, rd: Reg },
    Sub { w: Width, r0: Reg, r1: Reg, rd: Reg },
    Mul { w: Width, r0: Reg, r1: Reg, rd: Reg },
    UDiv { w: Width, r0: Reg, r1: Reg, rd: Reg },
    SDiv { w: Width, r0: Reg, r1: Reg, rd: Reg },
    Eq { r0: Reg, r1: Reg, rd: Reg },
    ULt { r0: Reg, r1: Reg, rd: Reg },
    SLt { r0: Reg, r1: Reg, rd: Reg },
    Not { r0: Reg, rd: Reg },
    Neg { w: Width, r0: Reg, rd: Reg },
    FNeg { w: Width, r0: Reg, rd: Reg },
    Move { r0: Reg, rd: Reg },
    MakeArray { r0: Reg, rn: Reg, rd: Reg },
    Index { r0: Reg, r1: Reg, rd: Reg },
    Jump { addr: Addr },
    JumpIfTrue { r0: Reg, addr: Addr },
    JumpIfFalse { r0: Reg, addr: Addr },
    FAdd { w: Width, r0: Reg, r1: Reg, rd: Reg },
    FSub { w: Width, r0: Reg, r1: Reg, rd: Reg },
    FMul { w: Width, r0: Reg, r1: Reg, rd: Reg },
    FDiv { w: Width, r0: Reg, r1: Reg, rd: Reg },
    FLt { w: Width, r0: Reg, r1: Reg, rd: Reg },
    Call { func: Reg, rd: Reg },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EncodedInstr(u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Width {
    _8 = 0,
    _16 = 1,
    _32 = 2,
    _64 = 3,
}

impl Width {
    pub fn as_uint(self) -> UIntTy {
        match self {
            Self::_8 => UIntTy::UInt8,
            Self::_16 => UIntTy::UInt16,
            Self::_32 => UIntTy::UInt32,
            Self::_64 => UIntTy::UInt64,
        }
    }

    pub fn as_int(self) -> IntTy {
        match self {
            Self::_8 => IntTy::Int8,
            Self::_16 => IntTy::Int16,
            Self::_32 => IntTy::Int32,
            Self::_64 => IntTy::Int64,
        }
    }

    pub fn as_float(self) -> FloatTy {
        match self {
            Self::_32 => FloatTy::Float32,
            Self::_64 => FloatTy::Float64,
            _ => panic!("invalid width for float"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reg(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Imm(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Addr(pub u16);

impl Instr {
    pub fn encode(self) -> EncodedInstr {
        match self {
            Self::Return { r0 } => EncodedInstr::opcode(0).reg0(r0),
            Self::LoadUnit { rd } => EncodedInstr::opcode(1).reg0(rd),
            Self::LoadNull { rd } => EncodedInstr::opcode(2).reg0(rd),
            Self::LoadFalse { rd } => EncodedInstr::opcode(3).reg0(rd),
            Self::LoadTrue { rd } => EncodedInstr::opcode(4).reg0(rd),
            Self::LoadZero { w, rd } => EncodedInstr::opcode(5).width(w).reg0(rd),
            Self::LoadFZero { w, rd } => EncodedInstr::opcode(6).width(w).reg0(rd),
            Self::Load { rd, imm } => EncodedInstr::opcode(7).reg0(rd).imm1(imm),
            Self::Print { r0 } => EncodedInstr::opcode(8).reg0(r0),
            Self::Add { w, r0, r1, rd } => EncodedInstr::opcode(10).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::Sub { w, r0, r1, rd } => EncodedInstr::opcode(11).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::Mul { w, r0, r1, rd } => EncodedInstr::opcode(12).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::UDiv { w, r0, r1, rd } => EncodedInstr::opcode(13).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::SDiv { w, r0, r1, rd } => EncodedInstr::opcode(14).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::Eq { r0, r1, rd } => EncodedInstr::opcode(15).reg0(r0).reg1(r1).reg2(rd),
            Self::ULt { r0, r1, rd } => EncodedInstr::opcode(16).reg0(r0).reg1(r1).reg2(rd),
            Self::SLt { r0, r1, rd } => EncodedInstr::opcode(17).reg0(r0).reg1(r1).reg2(rd),
            Self::Not { r0, rd } => EncodedInstr::opcode(18).reg0(r0).reg2(rd),
            Self::Neg { w, r0, rd } => EncodedInstr::opcode(19).width(w).reg0(r0).reg2(rd),
            Self::FNeg { w, r0, rd } => EncodedInstr::opcode(20).width(w).reg0(r0).reg2(rd),
            Self::MakeArray { r0, rn, rd } => EncodedInstr::opcode(21).reg0(r0).reg1(rn).reg2(rd),
            Self::Index { r0, r1, rd } => EncodedInstr::opcode(22).reg0(r0).reg1(r1).reg2(rd),
            Self::Move { r0, rd } => EncodedInstr::opcode(23).reg0(r0).reg1(rd),
            Self::Jump { addr } => EncodedInstr::opcode(30).addr(addr),
            Self::JumpIfTrue { r0, addr } => EncodedInstr::opcode(31).reg0(r0).addr(addr),
            Self::JumpIfFalse { r0, addr } => EncodedInstr::opcode(32).reg0(r0).addr(addr),
            Self::FAdd { w, r0, r1, rd } => EncodedInstr::opcode(40).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::FSub { w, r0, r1, rd } => EncodedInstr::opcode(41).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::FMul { w, r0, r1, rd } => EncodedInstr::opcode(42).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::FDiv { w, r0, r1, rd } => EncodedInstr::opcode(43).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::FLt { w, r0, r1, rd } => EncodedInstr::opcode(44).width(w).reg0(r0).reg1(r1).reg2(rd),
            Self::Call { func, rd } => EncodedInstr::opcode(50).reg0(func).reg2(rd),
        }
    }

    pub fn decode(enc: EncodedInstr) -> Self {
        match enc.get_opcode() {
            0 => Self::Return { r0: enc.get_reg0() },
            1 => Self::LoadUnit { rd: enc.get_reg0() },
            2 => Self::LoadNull { rd: enc.get_reg0() },
            3 => Self::LoadFalse { rd: enc.get_reg0() },
            4 => Self::LoadTrue { rd: enc.get_reg0() },
            5 => Self::LoadZero { w: enc.get_width(), rd: enc.get_reg0() },
            6 => Self::LoadFZero { w: enc.get_width(), rd: enc.get_reg0() },
            7 => Self::Load { rd: enc.get_reg0(), imm: enc.get_imm1() },
            8 => Self::Print { r0: enc.get_reg0() },
            10 => Self::Add { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            11 => Self::Sub { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            12 => Self::Mul { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            13 => Self::UDiv { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            14 => Self::SDiv { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            15 => Self::Eq { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            16 => Self::ULt { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            17 => Self::SLt { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            18 => Self::Not { r0: enc.get_reg0(), rd: enc.get_reg2() },
            19 => Self::Neg { w: enc.get_width(), r0: enc.get_reg0(), rd: enc.get_reg2() },
            20 => Self::FNeg { w: enc.get_width(), r0: enc.get_reg0(), rd: enc.get_reg2() },
            21 => Self::MakeArray { r0: enc.get_reg0(), rn: enc.get_reg1(), rd: enc.get_reg2() },
            22 => Self::Index { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            23 => Self::Move { r0: enc.get_reg0(), rd: enc.get_reg1() },
            30 => Self::Jump { addr: enc.get_addr() },
            31 => Self::JumpIfTrue { r0: enc.get_reg0(), addr: enc.get_addr() },
            32 => Self::JumpIfFalse { r0: enc.get_reg0(), addr: enc.get_addr() },
            40 => Self::FAdd { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            41 => Self::FSub { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            42 => Self::FMul { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            43 => Self::FDiv { w: enc.get_width(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            50 => Self::Call { func: enc.get_reg0(), rd: enc.get_reg2() },
            i => panic!("illegal instruction: {i}"),
        }
    }

    pub fn patch_addr(self, addr: Addr) -> Self {
        match self {
            Self::Jump { .. } => Self::Jump { addr },
            Self::JumpIfTrue { r0, .. } => Self::JumpIfTrue { r0, addr },
            Self::JumpIfFalse { r0, .. } => Self::JumpIfFalse { r0, addr },
            _ => panic!("cannot backpatch this instruction"),
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

    fn reg1(self, reg: Reg) -> Self {
        Self(self.0 | ((reg.0 as u32) << 16))
    }

    fn reg2(self, reg: Reg) -> Self {
        Self(self.0 | ((reg.0 as u32) << 24))
    }

    fn imm1(self, imm: Imm) -> Self {
        Self(self.0 | ((imm.0 as u32) << 16))
    }

    fn addr(self, addr: Addr) -> Self {
        Self(self.0 | ((addr.0 as u32) << 16))
    }

    fn get_opcode(self) -> u8 {
        (self.0 & 0x3f) as u8
    }

    fn get_width(self) -> Width {
        match (self.0 >> 6) & 0x03 {
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

    fn get_reg1(self) -> Reg {
        Reg(((self.0 >> 16) & 0xff) as _)
    }

    fn get_reg2(self) -> Reg {
        Reg(((self.0 >> 24) & 0xff) as _)
    }

    fn get_imm1(self) -> Imm {
        Imm(((self.0 >> 16) & 0xffff) as _)
    }

    fn get_addr(self) -> Addr {
        Addr(((self.0 >> 16) & 0xffff) as _)
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
                let width = match width {
                    Width::_8 => ".8",
                    Width::_16 => ".16",
                    Width::_32 => ".32",
                    Width::_64 => ".64",
                };
                self.instr_len += width.len();
                write!(self.f, "{}", width);
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

            fn addr(mut self, addr: Addr) -> Self {
                self.start_arg();
                write!(self.f, "{}", addr);
                self
            }

            fn start_arg(&mut self) {
                if self.first_arg {
                    const WIDTH: &str = "          ";
                    write!(self.f, "{}", &WIDTH[self.instr_len..]);
                    self.first_arg = false;
                } else {
                    write!(self.f, ", ");
                }
            }
        }

        let iw = InstrWriter::new(f);
        match *self {
            Self::Return { r0 } => iw.mnem("ret").reg(r0),
            Self::LoadUnit { rd } => iw.mnem("ld.unit").reg(rd),
            Self::LoadNull { rd } => iw.mnem("ld.null").reg(rd),
            Self::LoadFalse { rd } => iw.mnem("ld.false").reg(rd),
            Self::LoadTrue { rd } => iw.mnem("ld.true").reg(rd),
            Self::LoadZero { w, rd } => iw.mnem("ld.zero").reg(rd),
            Self::LoadFZero { w, rd } => iw.mnem("ld.fzero").reg(rd),
            Self::Load { rd, imm } => iw.mnem("ld").reg(rd).imm(imm),
            Self::Print { r0 } => iw.mnem("print").reg(r0),
            Self::Add { w, r0, r1, rd } => iw.mnem("add").width(w).reg(rd).reg(r0).reg(r1),
            Self::Sub { w, r0, r1, rd } => iw.mnem("sub").width(w).reg(rd).reg(r0).reg(r1),
            Self::Mul { w, r0, r1, rd } => iw.mnem("mul").width(w).reg(rd).reg(r0).reg(r1),
            Self::UDiv { w, r0, r1, rd } => iw.mnem("udiv").width(w).reg(rd).reg(r0).reg(r1),
            Self::SDiv { w, r0, r1, rd } => iw.mnem("sdiv").width(w).reg(rd).reg(r0).reg(r1),
            Self::Eq { r0, r1, rd } => iw.mnem("eq").reg(rd).reg(r0).reg(r1),
            Self::ULt { r0, r1, rd } => iw.mnem("ult").reg(rd).reg(r0).reg(r1),
            Self::SLt { r0, r1, rd } => iw.mnem("slt").reg(rd).reg(r0).reg(r1),
            Self::Not { r0, rd } => iw.mnem("not").reg(rd).reg(r0),
            Self::Neg { w, r0, rd } => iw.mnem("neg").width(w).reg(rd).reg(r0),
            Self::FNeg { w, r0, rd } => iw.mnem("fneg").width(w).reg(rd).reg(r0),
            Self::Move { r0, rd } => iw.mnem("move").reg(rd).reg(r0),
            Self::MakeArray { r0, rn, rd } => iw.mnem("mk.arr").reg(rd).reg(r0).reg(rn),
            Self::Index { r0, r1, rd } => iw.mnem("index").reg(rd).reg(r0).reg(r1),
            Self::Jump { addr } => iw.mnem("jmp").addr(addr),
            Self::JumpIfTrue { r0, addr } => iw.mnem("brt").reg(r0).addr(addr),
            Self::JumpIfFalse { r0, addr } => iw.mnem("brf").reg(r0).addr(addr),
            Self::FAdd { w, r0, r1, rd } => iw.mnem("fadd").width(w).reg(rd).reg(r0).reg(r1),
            Self::FSub { w, r0, r1, rd } => iw.mnem("fsub").width(w).reg(rd).reg(r0).reg(r1),
            Self::FMul { w, r0, r1, rd } => iw.mnem("fmul").width(w).reg(rd).reg(r0).reg(r1),
            Self::FDiv { w, r0, r1, rd } => iw.mnem("fdiv").width(w).reg(rd).reg(r0).reg(r1),
            Self::FLt { w, r0, r1, rd } => iw.mnem("flt").width(w).reg(rd).reg(r0).reg(r1),
            Self::Call { func, rd } => iw.mnem("call").reg(rd).reg(func),
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

impl Display for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

        test(Instr::Return { r0: Reg(0) });
        test(Instr::Return { r0: Reg(144) });
        test(Instr::Load { rd: Reg(0), imm: Imm(0) });
        test(Instr::Load { rd: Reg(78), imm: Imm(89) });
        test(Instr::Print { r0: Reg(0) });
        test(Instr::Print { r0: Reg(86) });
        test(Instr::Print { r0: Reg(234) });
        test(Instr::Add { w: Width::_8, r0: Reg(1), r1: Reg(2), rd: Reg(3) });
        test(Instr::Add { w: Width::_16, r0: Reg(10), r1: Reg(20), rd: Reg(30) });
        test(Instr::Add { w: Width::_32, r0: Reg(100), r1: Reg(200), rd: Reg(250) });
        test(Instr::Add { w: Width::_64, r0: Reg(255), r1: Reg(255), rd: Reg(255) });
        test(Instr::Sub { w: Width::_8, r0: Reg(1), r1: Reg(2), rd: Reg(3) });
        test(Instr::Sub { w: Width::_16, r0: Reg(10), r1: Reg(20), rd: Reg(30) });
        test(Instr::Sub { w: Width::_32, r0: Reg(100), r1: Reg(200), rd: Reg(250) });
        test(Instr::Sub { w: Width::_64, r0: Reg(255), r1: Reg(255), rd: Reg(255) });
        test(Instr::Move { r0: Reg(12), rd: Reg(24) });
    }
}
