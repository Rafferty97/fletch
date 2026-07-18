use std::fmt::Display;

use crate::types::ty::{FloatTy, IntTy, UIntTy};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Instr {
    Return { r0: Reg },
    LoadUnit { rd: Reg },
    LoadNull { rd: Reg },
    LoadFalse { rd: Reg },
    LoadTrue { rd: Reg },
    LoadIntZero { rd: Reg },
    LoadF32Zero { rd: Reg },
    LoadF64Zero { rd: Reg },
    Load { rd: Reg, imm: Imm },
    Print { r0: Reg },
    Add { r0: Reg, r1: Reg, rd: Reg },
    Sub { r0: Reg, r1: Reg, rd: Reg },
    Mul { r0: Reg, r1: Reg, rd: Reg },
    Div { r0: Reg, r1: Reg, rd: Reg },
    Eq { r0: Reg, r1: Reg, rd: Reg },
    Lt { r0: Reg, r1: Reg, rd: Reg },
    Not { r0: Reg, rd: Reg },
    Neg { r0: Reg, rd: Reg },
    FNeg { w: FloatTy, r0: Reg, rd: Reg },
    Move { r0: Reg, rd: Reg },
    MakeArray { r0: Reg, rn: Reg, rd: Reg },
    Index { r0: Reg, r1: Reg, rd: Reg },
    MakeTuple { r0: Reg, rn: Reg, rd: Reg },
    MakeVariant { r0: Reg, imm: Imm, rd: Reg },
    Jump { addr: Addr },
    JumpIfTrue { r0: Reg, addr: Addr },
    JumpIfFalse { r0: Reg, addr: Addr },
    FAdd { w: FloatTy, r0: Reg, r1: Reg, rd: Reg },
    FSub { w: FloatTy, r0: Reg, r1: Reg, rd: Reg },
    FMul { w: FloatTy, r0: Reg, r1: Reg, rd: Reg },
    FDiv { w: FloatTy, r0: Reg, r1: Reg, rd: Reg },
    FLt { w: FloatTy, r0: Reg, r1: Reg, rd: Reg },
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
            Self::LoadIntZero { rd } => EncodedInstr::opcode(5).reg0(rd),
            Self::LoadF32Zero { rd } => EncodedInstr::opcode(6).reg0(rd),
            Self::LoadF64Zero { rd } => EncodedInstr::opcode(7).reg0(rd),
            Self::Load { rd, imm } => EncodedInstr::opcode(8).reg0(rd).imm1(imm),
            Self::Print { r0 } => EncodedInstr::opcode(9).reg0(r0),
            Self::Add { r0, r1, rd } => EncodedInstr::opcode(10).reg0(r0).reg1(r1).reg2(rd),
            Self::Sub { r0, r1, rd } => EncodedInstr::opcode(11).reg0(r0).reg1(r1).reg2(rd),
            Self::Mul { r0, r1, rd } => EncodedInstr::opcode(12).reg0(r0).reg1(r1).reg2(rd),
            Self::Div { r0, r1, rd } => EncodedInstr::opcode(13).reg0(r0).reg1(r1).reg2(rd),
            Self::Eq { r0, r1, rd } => EncodedInstr::opcode(15).reg0(r0).reg1(r1).reg2(rd),
            Self::Lt { r0, r1, rd } => EncodedInstr::opcode(16).reg0(r0).reg1(r1).reg2(rd),
            Self::Not { r0, rd } => EncodedInstr::opcode(18).reg0(r0).reg2(rd),
            Self::Neg { r0, rd } => EncodedInstr::opcode(19).reg0(r0).reg2(rd),
            Self::FNeg { w, r0, rd } => EncodedInstr::opcode(20).fwid(w).reg0(r0).reg2(rd),
            Self::MakeArray { r0, rn, rd } => EncodedInstr::opcode(21).reg0(r0).reg1(rn).reg2(rd),
            Self::Index { r0, r1, rd } => EncodedInstr::opcode(22).reg0(r0).reg1(r1).reg2(rd),
            Self::MakeTuple { r0, rn, rd } => EncodedInstr::opcode(23).reg0(r0).reg1(rn).reg2(rd),
            Self::MakeVariant { r0, imm, rd } => EncodedInstr::opcode(24).reg0(r0).reg1(rd).imm2(imm),
            Self::Move { r0, rd } => EncodedInstr::opcode(25).reg0(r0).reg1(rd),
            Self::Jump { addr } => EncodedInstr::opcode(30).addr(addr),
            Self::JumpIfTrue { r0, addr } => EncodedInstr::opcode(31).reg0(r0).addr(addr),
            Self::JumpIfFalse { r0, addr } => EncodedInstr::opcode(32).reg0(r0).addr(addr),
            Self::FAdd { w, r0, r1, rd } => EncodedInstr::opcode(40).reg0(r0).reg1(r1).reg2(rd),
            Self::FSub { w, r0, r1, rd } => EncodedInstr::opcode(41).reg0(r0).reg1(r1).reg2(rd),
            Self::FMul { w, r0, r1, rd } => EncodedInstr::opcode(42).reg0(r0).reg1(r1).reg2(rd),
            Self::FDiv { w, r0, r1, rd } => EncodedInstr::opcode(43).reg0(r0).reg1(r1).reg2(rd),
            Self::FLt { w, r0, r1, rd } => EncodedInstr::opcode(44).reg0(r0).reg1(r1).reg2(rd),
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
            5 => Self::LoadIntZero { rd: enc.get_reg0() },
            6 => Self::LoadF32Zero { rd: enc.get_reg0() },
            7 => Self::LoadF64Zero { rd: enc.get_reg0() },
            8 => Self::Load { rd: enc.get_reg0(), imm: enc.get_imm1() },
            9 => Self::Print { r0: enc.get_reg0() },
            10 => Self::Add { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            11 => Self::Sub { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            12 => Self::Mul { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            13 => Self::Div { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            15 => Self::Eq { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            16 => Self::Lt { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            18 => Self::Not { r0: enc.get_reg0(), rd: enc.get_reg2() },
            19 => Self::Neg { r0: enc.get_reg0(), rd: enc.get_reg2() },
            20 => Self::FNeg { w: enc.get_fwid(), r0: enc.get_reg0(), rd: enc.get_reg2() },
            21 => Self::MakeArray { r0: enc.get_reg0(), rn: enc.get_reg1(), rd: enc.get_reg2() },
            22 => Self::Index { r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            23 => Self::MakeTuple { r0: enc.get_reg0(), rn: enc.get_reg1(), rd: enc.get_reg2() },
            24 => Self::MakeVariant { r0: enc.get_reg0(), imm: enc.get_imm2(), rd: enc.get_reg1() },
            25 => Self::Move { r0: enc.get_reg0(), rd: enc.get_reg1() },
            30 => Self::Jump { addr: enc.get_addr() },
            31 => Self::JumpIfTrue { r0: enc.get_reg0(), addr: enc.get_addr() },
            32 => Self::JumpIfFalse { r0: enc.get_reg0(), addr: enc.get_addr() },
            40 => Self::FAdd { w: enc.get_fwid(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            41 => Self::FSub { w: enc.get_fwid(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            42 => Self::FMul { w: enc.get_fwid(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
            43 => Self::FDiv { w: enc.get_fwid(), r0: enc.get_reg0(), r1: enc.get_reg1(), rd: enc.get_reg2() },
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

    fn fwid(self, width: FloatTy) -> Self {
        let bit = match width {
            FloatTy::Float32 => 0,
            FloatTy::Float64 => 1,
        };
        Self(self.0 | (bit << 7))
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

    fn imm2(self, imm: Imm) -> Self {
        Self(self.0 | ((imm.0 as u32) << 24))
    }

    fn addr(self, addr: Addr) -> Self {
        Self(self.0 | ((addr.0 as u32) << 16))
    }

    fn get_opcode(self) -> u8 {
        (self.0 & 0x3f) as u8
    }

    fn get_fwid(self) -> FloatTy {
        match (self.0 >> 7) & 0x01 {
            0 => FloatTy::Float32,
            1 => FloatTy::Float64,
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

    fn get_imm2(self) -> Imm {
        Imm(((self.0 >> 24) & 0xff) as _)
    }

    fn get_addr(self) -> Addr {
        Addr(((self.0 >> 16) & 0xffff) as _)
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FloatTy::*;

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

            fn val(mut self, val: impl Display) -> Self {
                self.start_arg();
                write!(self.f, "{}", val);
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
            Self::LoadUnit { rd } => iw.mnem("load").reg(rd).val("unit"),
            Self::LoadNull { rd } => iw.mnem("load").reg(rd).val("null"),
            Self::LoadFalse { rd } => iw.mnem("load").reg(rd).val("false"),
            Self::LoadTrue { rd } => iw.mnem("load").reg(rd).val("true"),
            Self::LoadIntZero { rd } => iw.mnem("load").reg(rd).val("0i".to_string()),
            Self::LoadF32Zero { rd } => iw.mnem("load").reg(rd).val("0f32".to_string()),
            Self::LoadF64Zero { rd } => iw.mnem("load").reg(rd).val("0f64".to_string()),
            Self::Load { rd, imm } => iw.mnem("load").reg(rd).imm(imm),
            Self::Print { r0 } => iw.mnem("print").reg(r0),
            Self::Add { r0, r1, rd } => iw.mnem("add.i").reg(rd).reg(r0).reg(r1),
            Self::Sub { r0, r1, rd } => iw.mnem("sub.i").reg(rd).reg(r0).reg(r1),
            Self::Mul { r0, r1, rd } => iw.mnem("mul.i").reg(rd).reg(r0).reg(r1),
            Self::Div { r0, r1, rd } => iw.mnem("div.i").reg(rd).reg(r0).reg(r1),
            Self::Eq { r0, r1, rd } => iw.mnem("eq").reg(rd).reg(r0).reg(r1),
            Self::Lt { r0, r1, rd } => iw.mnem("lt").reg(rd).reg(r0).reg(r1),
            Self::Not { r0, rd } => iw.mnem("not").reg(rd).reg(r0),
            Self::Neg { r0, rd } => iw.mnem("neg.i").reg(rd).reg(r0),
            Self::FNeg { w: Float32, r0, rd } => iw.mnem("neg.f").reg(rd).reg(r0),
            Self::FNeg { w: Float64, r0, rd } => iw.mnem("neg.d").reg(rd).reg(r0),
            Self::Move { r0, rd } => iw.mnem("move").reg(rd).reg(r0),
            Self::MakeArray { r0, rn, rd } => iw.mnem("mk.arr").reg(rd).reg(r0).reg(rn),
            Self::Index { r0, r1, rd } => iw.mnem("index").reg(rd).reg(r0).reg(r1),
            Self::MakeTuple { r0, rn, rd } => iw.mnem("mk.tup").reg(rd).reg(r0).reg(rn),
            Self::MakeVariant { r0, imm, rd } => iw.mnem("mk.var").reg(rd).reg(r0).imm(imm),
            Self::Jump { addr } => iw.mnem("jmp").addr(addr),
            Self::JumpIfTrue { r0, addr } => iw.mnem("brt").reg(r0).addr(addr),
            Self::JumpIfFalse { r0, addr } => iw.mnem("brf").reg(r0).addr(addr),
            Self::FAdd { w: Float32, r0, r1, rd } => iw.mnem("add.f").reg(rd).reg(r0).reg(r1),
            Self::FSub { w: Float32, r0, r1, rd } => iw.mnem("sub.f").reg(rd).reg(r0).reg(r1),
            Self::FMul { w: Float32, r0, r1, rd } => iw.mnem("mul.f").reg(rd).reg(r0).reg(r1),
            Self::FDiv { w: Float32, r0, r1, rd } => iw.mnem("div.f").reg(rd).reg(r0).reg(r1),
            Self::FLt { w: Float32, r0, r1, rd } => iw.mnem("lt.f").reg(rd).reg(r0).reg(r1),
            Self::FAdd { w: Float64, r0, r1, rd } => iw.mnem("add.d").reg(rd).reg(r0).reg(r1),
            Self::FSub { w: Float64, r0, r1, rd } => iw.mnem("sub.d").reg(rd).reg(r0).reg(r1),
            Self::FMul { w: Float64, r0, r1, rd } => iw.mnem("mul.d").reg(rd).reg(r0).reg(r1),
            Self::FDiv { w: Float64, r0, r1, rd } => iw.mnem("div.d").reg(rd).reg(r0).reg(r1),
            Self::FLt { w: Float64, r0, r1, rd } => iw.mnem("lt.d").reg(rd).reg(r0).reg(r1),
            Self::Call { func, rd } => iw.mnem("call").reg(rd).reg(func),
        };
        Ok(())
    }
}

impl Display for Width {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width = match self {
            Width::_8 => "8",
            Width::_16 => "16",
            Width::_32 => "32",
            Width::_64 => "64",
        };
        write!(f, "{}", width)
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
        test(Instr::Add { r0: Reg(1), r1: Reg(2), rd: Reg(3) });
        test(Instr::Add { r0: Reg(10), r1: Reg(20), rd: Reg(30) });
        test(Instr::Add { r0: Reg(100), r1: Reg(200), rd: Reg(250) });
        test(Instr::Add { r0: Reg(255), r1: Reg(255), rd: Reg(255) });
        test(Instr::Sub { r0: Reg(1), r1: Reg(2), rd: Reg(3) });
        test(Instr::Sub { r0: Reg(10), r1: Reg(20), rd: Reg(30) });
        test(Instr::Sub { r0: Reg(100), r1: Reg(200), rd: Reg(250) });
        test(Instr::Sub { r0: Reg(255), r1: Reg(255), rd: Reg(255) });
        test(Instr::Move { r0: Reg(12), rd: Reg(24) });
        test(Instr::FNeg { w: FloatTy::Float32, r0: Reg(22), rd: Reg(23) });
        test(Instr::FNeg { w: FloatTy::Float64, r0: Reg(33), rd: Reg(34) });
        test(Instr::MakeVariant { r0: Reg(45), imm: Imm(10), rd: Reg(12) });
    }
}
