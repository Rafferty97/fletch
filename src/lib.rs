#![allow(unused)]

use std::io::Write;

use bumpalo::Bump;

use crate::{ast::print::print_expr, types::check_expr};

mod arena;
mod ast;
mod diagnostics;
mod lexer;
mod parser;
mod span;
mod types;

pub fn run(src: &str, mut out: impl Write) {
    let arena = Bump::new();
    let mut handler = diagnostics::Diagnostics::new();
    let ctx = arena::Ctx::new(&arena, &mut handler);

    let mut parser = parser::Parser::new(ctx, src);
    let expr = match parser.parse_expr() {
        Ok(expr) => expr,
        Err(err) => {
            write!(out, "Parse error: {}", err.message).unwrap();
            return;
        }
    };

    let ty = match check_expr(ctx, &expr) {
        Ok(expr) => expr,
        Err(err) => {
            write!(out, "Type error: {err}").unwrap();
            return;
        }
    };

    write!(out, "{} :: {:?}", print_expr(ctx, &expr), ty).unwrap();
}
