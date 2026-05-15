#![allow(unused)]

use std::io::Write;

use bumpalo::Bump;

use crate::{
    ast::{Stmt, StmtKind, print::print_expr},
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
    let mut func_ctx = FunctionCtx::new(ctx);

    inner(&mut |line, mut out| {
        if line.starts_with('.') {
            match &*line {
                ".env" => func_ctx.tc.debug_env(),
                line => write!(out, "Unknown command: {line}").unwrap(),
            }
            return;
        }

        let mut parser = parser::Parser::new(ctx, &line);

        match parser.parse_toplevel_stmt() {
            Ok(Stmt { kind: StmtKind::Let(r#let), .. }) => {
                match func_ctx.check_expr(&r#let.expr).and_then(|ty| func_ctx.resolve_partial(ty)) {
                    Ok(ty) => {
                        let name = r#let.name.sym;
                        func_ctx.tc.bind_variable(name, ty);
                        write!(out, "{} :: {}", ctx.get_str(r#let.name.sym), ty).unwrap();
                    }
                    Err(err) => write!(out, "Type error: {err}").unwrap(),
                };
            }
            Ok(Stmt { kind: StmtKind::Expr(expr), .. }) => {
                match func_ctx.check_expr(&expr).and_then(|ty| func_ctx.resolve_partial(ty)) {
                    Ok(ty) => write!(out, "{} :: {}", print_expr(ctx, &expr), ty).unwrap(),
                    Err(err) => write!(out, "Type error: {err}").unwrap(),
                };
            }
            Err(err) => write!(out, "Parse error: {}", err.message).unwrap(),
        };
    });
}
