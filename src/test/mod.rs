#![cfg(test)]

use crate::interpreter::{Value, interpret_program};
use crate::lower::lower_program;
use crate::parser::Parser;
use crate::typecheck::with_ty_ctx;

fn eval_expr(src: &str) -> Value {
    let mut parser = Parser::new(src);
    let ast = parser.parse_program().unwrap();
    with_ty_ctx(|tcx| {
        let ir = lower_program(&ast, tcx).unwrap();
        interpret_program(ir)
    })
}

#[test]
fn single_binary_op() {
    let src = "2 + 3";
    let result = eval_expr(src);
    assert_eq!(result, Value::Scalar(5));
}

#[test]
fn nested_parens() {
    let src = "3 + (7 * (10 / 5) + 4)";
    let result = eval_expr(src);
    assert_eq!(result, Value::Scalar(21));
}
