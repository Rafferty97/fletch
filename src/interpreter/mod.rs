use crate::ir::{
    Call, Expr, ExprKind, Lit, Program,
    intrinsics::{ADD_I32, ADD_U64, DIV_I32, DIV_U64, MUL_I32, MUL_U64, SUB_I32, SUB_U64},
};

pub fn interpret_program(ir: Program) -> Value {
    interpret_expr(&ir.expr)
}

fn interpret_expr(ir: &Expr) -> Value {
    match &ir.kind {
        ExprKind::Call(call) => interpret_call(call),
        ExprKind::Lit(lit) => interpret_lit(lit),
        _ => unimplemented!(),
    }
}

fn interpret_call(ir: &Call) -> Value {
    match ir.func.0 {
        op @ 0..40 => {
            let [lhs, rhs] = &ir.args[..] else {
                panic!("expected 2 arguments, got {}", ir.args.len());
            };
            let lhs = interpret_expr(lhs);
            let rhs = interpret_expr(rhs);
            match (ir.func, lhs, rhs) {
                (ADD_I32, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs + rhs),
                (ADD_U64, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs + rhs),
                (SUB_I32, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs - rhs),
                (SUB_U64, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs - rhs),
                (MUL_I32, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs * rhs),
                (MUL_U64, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs * rhs),
                (DIV_I32, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs / rhs),
                (DIV_U64, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs / rhs),
                _ => unreachable!(),
            }
        }
        _ => panic!("unresolved function call"),
    }
}

fn interpret_lit(ir: &Lit) -> Value {
    match ir {
        Lit::Int32(value) => Value::Int32(*value),
        Lit::UInt64(value) => Value::UInt64(*value),
        _ => unimplemented!(),
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Int32(i32),
    UInt64(u64),
}
