#![allow(unused)]

use std::io::Write;

use bumpalo::Bump;

use crate::{
    ast::print::print_expr,
    types::{FunctionCtx, check_expr},
};

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
    let expr = match parser.parse_toplevel_expr() {
        Ok(expr) => expr,
        Err(err) => {
            write!(out, "Parse error: {}", err.message).unwrap();
            return;
        }
    };

    let mut func_ctx = FunctionCtx::new(ctx);
    let ty = match func_ctx.check_expr(&expr).and_then(|ty| func_ctx.resolve(ty)) {
        Ok(expr) => expr,
        Err(err) => {
            write!(out, "Type error: {err}").unwrap();
            return;
        }
    };

    write!(out, "{} :: {}", print_expr(ctx, &expr), ty).unwrap();
}

pub fn run_repl<I, W>(inner: I)
where
    I: FnOnce(&mut dyn FnMut(String, &mut W)),
    W: Write,
{
    let arena = Bump::new();
    let mut handler = diagnostics::Diagnostics::new();
    let ctx = arena::Ctx::new(&arena, &mut handler);

    inner(&mut |line, mut out| {
        let mut parser = parser::Parser::new(ctx, &line);
        let expr = match parser.parse_toplevel_expr() {
            Ok(expr) => expr,
            Err(err) => {
                write!(out, "Parse error: {}", err.message).unwrap();
                return;
            }
        };

        let mut func_ctx = FunctionCtx::new(ctx);
        let ty = match func_ctx.check_expr(&expr).and_then(|ty| func_ctx.resolve(ty)) {
            Ok(expr) => expr,
            Err(err) => {
                write!(out, "Type error: {err}").unwrap();
                return;
            }
        };

        write!(out, "{} :: {}", print_expr(ctx, &expr), ty).unwrap();
    });
}
