#![allow(unused)]

use std::io::Write;

use bumpalo::Bump;

use crate::ast::{Stmt, StmtKind, print::print_expr};
use crate::types::{FunctionCtx, check_expr};

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

pub trait ReplIo {
    fn read_line(&mut self, cont: bool, out: &mut String);
    fn write(&mut self) -> &mut impl Write;
}

pub fn run_repl(mut io: impl ReplIo) {
    let arena = Bump::new();
    let mut handler = diagnostics::Diagnostics::new();
    let ctx = arena::Ctx::new(&arena, &mut handler);
    let mut func_ctx = FunctionCtx::new(ctx);

    let mut line = String::new();

    'outer: loop {
        line.clear();
        io.read_line(false, &mut line);

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('.') {
            match trimmed {
                ".exit" => return,
                ".env" => func_ctx.tc.debug_env(),
                line => write!(io.write(), "Unknown command: {line}\n").unwrap(),
            }
            continue;
        }

        let stmt = loop {
            let mut parser = parser::Parser::new(ctx, &line);
            match parser.parse_toplevel_stmt() {
                Ok(stmt) => break stmt,
                Err(_) if parser.is_eof() => io.read_line(true, &mut line),
                Err(err) => {
                    write!(io.write(), "Parse error: {}\n", err.message).unwrap();
                    continue 'outer;
                }
            }
        };

        match stmt.kind {
            StmtKind::Let(r#let) => {
                match func_ctx.check_let(&r#let).and_then(|ty| func_ctx.resolve_partial(ty)) {
                    Ok(ty) => {
                        let name = r#let.name.sym;
                        func_ctx.tc.bind_variable(name, ty);
                        write!(io.write(), "{} :: {}\n", ctx.get_str(r#let.name.sym), ty).unwrap();
                    }
                    Err(err) => write!(io.write(), "Type error: {err}\n").unwrap(),
                };
            }
            StmtKind::Expr(expr) => {
                match func_ctx.check_expr(&expr).and_then(|ty| func_ctx.resolve_partial(ty)) {
                    Ok(ty) => write!(io.write(), "{} :: {}\n", print_expr(ctx, &expr), ty).unwrap(),
                    Err(err) => write!(io.write(), "Type error: {err}\n").unwrap(),
                };
            }
        };
    }
}
