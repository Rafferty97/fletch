use crate::ir::{BinOp, BinOpKind, Call, Expr, ExprKind, Lit, Program};

pub fn interpret_program(ir: Program) -> Value {
    interpret_expr(&ir.expr)
}

fn interpret_expr(ir: &Expr) -> Value {
    match &ir.kind {
        ExprKind::Lit(lit) => interpret_lit(lit),
        ExprKind::BinOp(binop) => interpret_binop(binop),
        ExprKind::Call(call) => interpret_call(call),
        _ => unimplemented!(),
    }
}

fn interpret_lit(ir: &Lit) -> Value {
    match ir {
        Lit::Int32(value) => Value::Int32(*value),
        Lit::UInt64(value) => Value::UInt64(*value),
        _ => unimplemented!(),
    }
}

fn interpret_binop(ir: &BinOp) -> Value {
    let lhs = interpret_expr(&ir.lhs);
    let rhs = interpret_expr(&ir.rhs);

    match ir.op {
        BinOpKind::Add => match (lhs, rhs) {
            (Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs + rhs),
            (Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs + rhs),
            _ => panic!("invalid operands"),
        },
        BinOpKind::Sub => match (lhs, rhs) {
            (Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs - rhs),
            (Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs - rhs),
            _ => panic!("invalid operands"),
        },
        BinOpKind::SMul | BinOpKind::UMul => match (lhs, rhs) {
            (Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs * rhs),
            (Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs * rhs),
            _ => panic!("invalid operands"),
        },
        BinOpKind::SDiv | BinOpKind::UDiv => match (lhs, rhs) {
            (Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs / rhs),
            (Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs / rhs),
            _ => panic!("invalid operands"),
        },
    }
}

fn interpret_call(ir: &Call) -> Value {
    unimplemented!()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Int32(i32),
    UInt64(u64),
}
