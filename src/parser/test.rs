#![cfg(test)]

use std::fmt::Display;

use bumpalo::Bump;

use crate::ast::sexpr::{SExpr, SExprCtx};
use crate::interner::IndexedInterner;
use crate::parser::ParseCtx;

use super::*;

fn with_parse_ctx(f: impl FnOnce(ParseCtx)) {
    let arena = &Bump::new();
    let sym_interner = &IndexedInterner::new();
    let ctx = ParseCtx { arena, sym_interner };
    f(ctx);
}

fn test_parse<'a, T, P>(ctx: ParseCtx<'a, '_>, parse: P, src: &'a str, expected: &str)
where
    T: SExpr,
    P: FnOnce(&mut Parser<'a, '_>) -> Result<'a, T>,
{
    let mut parser = Parser::new(ctx, src);
    parser.consume().unwrap();
    let result = parse(&mut parser).unwrap();
    let mut actual = String::new();
    let mut sexpr_ctx = SExprCtx { str: &mut actual, sym_interner: ctx.sym_interner };
    result.write(&mut sexpr_ctx);
    assert_eq!(actual, expected);
}

#[test]
fn parse_literals() {
    with_parse_ctx(|ctx| {
        test_parse(ctx, |p| p.parse_expr(), "null", "null");
        test_parse(ctx, |p| p.parse_expr(), "false", "false");
        test_parse(ctx, |p| p.parse_expr(), "true", "true");
        test_parse(ctx, |p| p.parse_expr(), "42", "(int 42)");
        test_parse(ctx, |p| p.parse_expr(), "4.2", "(float 4.2)");
        test_parse(ctx, |p| p.parse_expr(), "\"hello world\"", "(str \"hello world\")");
    });
}

#[test]
fn parse_simple_arithmetic() {
    with_parse_ctx(|ctx| {
        test_parse(ctx, |p| p.parse_expr(), "2 + 2", "(+ (int 2) (int 2))");
    });
}

#[test]
fn parse_simple_arithmetic_program() {
    with_parse_ctx(|ctx| {
        let src = r#"
            fn main() {
                print(2 + 2);
            }"#;
        let expected = r#"(func main (block (print (+ (int 2) (int 2))) none))"#;
        test_parse(ctx, |p| p.parse_program(), src, expected);
    });
}
