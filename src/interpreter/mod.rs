use std::fmt::Debug;
use std::ops::{Add, Div, Mul, Sub};

use crate::ir::{BinOp, BinOpKind, Call, Expr, ExprKind, Lit, Program};
use crate::typecheck::{FloatTy, IntTy, Ty, TyKind, UIntTy};

pub fn interpret_program(ir: Program) -> Value {
    interpret_expr(&ir.expr)
}

fn interpret_expr(ir: &Expr) -> Value {
    match &ir.kind {
        ExprKind::Lit(lit) => interpret_lit(lit),
        ExprKind::BinOp(binop) => interpret_binop(binop, ir.ty),
        ExprKind::Call(call) => interpret_call(call),
        _ => unimplemented!(),
    }
}

fn interpret_lit(ir: &Lit) -> Value {
    match ir {
        Lit::Int32(value) => Value::Scalar(*value as u64),
        Lit::UInt64(value) => Value::Scalar(*value as u64),
        _ => unimplemented!(),
    }
}

fn interpret_binop(ir: &BinOp, ty: Ty) -> Value {
    let lhs = interpret_expr(&ir.lhs);
    let rhs = interpret_expr(&ir.rhs);

    macro_rules! scalar_op {
        ($ty:expr, $lhs:expr, $rhs:expr, $op:expr) => {
            match $ty.kind() {
                TyKind::Int(IntTy::I8) => lhs.binop::<i8>(rhs, $op),
                TyKind::Int(IntTy::I16) => lhs.binop::<i16>(rhs, $op),
                TyKind::Int(IntTy::I32) => lhs.binop::<i32>(rhs, $op),
                TyKind::Int(IntTy::I64) => lhs.binop::<i64>(rhs, $op),
                TyKind::UInt(UIntTy::U8) => lhs.binop::<u8>(rhs, $op),
                TyKind::UInt(UIntTy::U16) => lhs.binop::<u16>(rhs, $op),
                TyKind::UInt(UIntTy::U32) => lhs.binop::<u32>(rhs, $op),
                TyKind::UInt(UIntTy::U64) => lhs.binop::<u64>(rhs, $op),
                TyKind::Float(FloatTy::F32) => lhs.binop::<f32>(rhs, $op),
                TyKind::Float(FloatTy::F64) => lhs.binop::<f64>(rhs, $op),
                other => panic!("invalid type: {other:?}"),
            }
        };
    }

    match ir.op {
        BinOpKind::Add => scalar_op!(ty, lhs, rhs, |a, b| a + b),
        BinOpKind::Sub => scalar_op!(ty, lhs, rhs, |a, b| a - b),
        BinOpKind::Mul => scalar_op!(ty, lhs, rhs, |a, b| a * b),
        BinOpKind::Div => scalar_op!(ty, lhs, rhs, |a, b| a / b),
    }
}

fn interpret_call(ir: &Call) -> Value {
    unimplemented!()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Scalar(u64),
}

impl Value {
    pub fn binop<T: Scalar>(self, rhs: Self, op: impl FnOnce(T, T) -> T) -> Self {
        let (Self::Scalar(lhs), Self::Scalar(rhs)) = (self, rhs);
        Self::Scalar(op(T::from_u64(lhs), T::from_u64(rhs)).into_u64())
    }
}

pub trait Scalar: Copy + Debug {
    fn from_u64(v: u64) -> Self;
    fn into_u64(self) -> u64;
}

macro_rules! scalar_impl {
    ($($tt:ty),*) => {
        $(impl Scalar for $tt {
            fn from_u64(v: u64) -> Self {
                v as _
            }

            fn into_u64(self) -> u64 {
                self as _
            }
        })*
    };
}

scalar_impl!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);
