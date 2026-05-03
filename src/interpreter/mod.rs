use crate::ir::{Call, Expr, ExprKind, Lit, Program};

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
        op @ 0..=4 => {
            let [lhs, rhs] = &ir.args[..] else {
                panic!("expected 2 arguments, got {}", ir.args.len());
            };
            let lhs = interpret_expr(lhs);
            let rhs = interpret_expr(rhs);
            match (op, lhs, rhs) {
                (0, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs + rhs),
                (1, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs - rhs),
                (2, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs * rhs),
                (3, Value::UInt64(lhs), Value::UInt64(rhs)) => Value::UInt64(lhs / rhs),
                (4.., _, _) => unreachable!(),
            }
        }
        _ => panic!("unresolved function call"),
    }
}

fn interpret_lit(ir: &Lit) -> Value {
    match ir {
        Lit::UInt64(value) => Value::UInt64(*value),
        _ => unimplemented!(),
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    UInt64(u64),
}
