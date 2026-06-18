#![cfg(test)]

use std::fmt::Display;

use bumpalo::Bump;

use crate::ast::SExpr;
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
    SExpr<T>: Display,
    P: FnOnce(&mut Parser<'a, '_>) -> Result<'a, T>,
{
    let mut parser = Parser::new(ctx, src);
    parser.consume().unwrap();
    let result = parse(&mut parser).unwrap();
    let actual = SExpr::new(&result).to_string();
    assert_eq!(actual, expected);
}

#[test]
fn parse_literals() {
    with_parse_ctx(|ctx| {
        test_parse(ctx, |p| p.parse_expr(), "null", "null");
        test_parse(ctx, |p| p.parse_expr(), "false", "false");
        test_parse(ctx, |p| p.parse_expr(), "true", "true");
    });
}

#[test]
fn parse_simple_arithmetic() {
    with_parse_ctx(|ctx| {
        test_parse(ctx, |p| p.parse_expr(), "2 + 2", "(+ (int $0) (int $0))");
    });
}

#[test]
fn parse_simple_arithmetic_program() {
    with_parse_ctx(|ctx| {
        let src = r#"
            fn main() {
                print(4);
            }"#;
        let expected = r#"(func $0 (block (print (int $1)) none))"#;
        test_parse(ctx, |p| p.parse_program(), src, expected);
    });
}
