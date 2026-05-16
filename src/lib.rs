#![allow(unused)]

use std::io::Write;
use std::marker::PhantomData;

use bumpalo::Bump;
use colored::Colorize;
use reedline::Validator;

use crate::arena::Ctx;
use crate::ast::ItemKind;
use crate::ast::{Stmt, StmtKind, print::print_expr};
use crate::diagnostics::NullHandler;
use crate::types::{FunctionCtx, check_expr};

mod arena;
mod ast;
mod diagnostics;
mod lexer;
mod parser;
mod span;
mod types;

pub fn run_repl(inner: impl FnOnce(ReplCtx)) {
    let arena = Bump::new();
    let mut handler = diagnostics::Diagnostics::new();
    let ctx = arena::Ctx::new(&arena, &mut handler);
    let mut func_ctx = FunctionCtx::new(ctx);

    inner(ReplCtx { ctx, func_ctx });
}

pub struct ReplCtx<'a> {
    ctx: arena::Ctx<'a>,
    func_ctx: FunctionCtx<'a>,
}

impl ReplCtx<'_> {
    pub fn validator(&self) -> Box<dyn Validator> {
        Box::new(ReplValidator)
    }

    pub fn eval(&mut self, input: &str) {
        // Parse
        let mut parser = parser::Parser::new(self.ctx, input);

        let item = match parser.parse_toplevel_item() {
            Ok(item) => item,
            Err(err) => {
                let span = err.labels.get(0).map(|l| l.span);
                if let Some(span) = span {
                    println!("Parse error: {} at {span:?}", err.message);
                    print!("{}", &input[..span.start().into()].bright_black());
                    print!("{}", &input[span.start().into()..span.end().into()].bright_red());
                    println!("{}", &input[span.end().into()..].bright_black());
                } else {
                    println!("Parse error: {}", err.message);
                }
                return;
            }
        };

        // Eval
        match item.kind {
            ItemKind::Func(func) => {
                match self.func_ctx.check_func(&func).and_then(|ty| self.func_ctx.resolve(ty)) {
                    Ok(ty) => {
                        let name = func.name.sym;
                        self.func_ctx.tc.bind_variable(name, ty);
                        println!("{} :: {}", self.ctx.get_str(name), ty);
                    }
                    Err(err) => println!("Type error: {err}"),
                };
            }
            ItemKind::Stmt(Stmt { kind: StmtKind::Let(r#let), .. }) => {
                match self
                    .func_ctx
                    .check_let(&r#let)
                    .and_then(|ty| self.func_ctx.resolve_partial(ty))
                {
                    Ok(ty) => {
                        let name = r#let.name.sym;
                        self.func_ctx.tc.bind_variable(name, ty);
                        println!("{} :: {}", self.ctx.get_str(name), ty);
                    }
                    Err(err) => println!("Type error: {err}"),
                };
            }
            ItemKind::Stmt(Stmt { kind: StmtKind::Expr(expr), .. }) => {
                match self
                    .func_ctx
                    .check_expr(&expr)
                    .and_then(|ty| self.func_ctx.resolve_partial(ty))
                {
                    Ok(ty) => println!("{} :: {}", print_expr(self.ctx, &expr), ty),
                    Err(err) => println!("Type error: {err}"),
                };
            }
        };
    }

    pub fn print_env(&mut self) {
        self.func_ctx.tc.debug_env()
    }
}

struct ReplValidator;

impl Validator for ReplValidator {
    fn validate(&self, line: &str) -> reedline::ValidationResult {
        let arena = &bumpalo::Bump::new();
        let handler = &mut NullHandler;
        let mut ctx = Ctx::new(arena, handler);
        let mut parser = parser::Parser::new(ctx, &line);
        if parser.parse_toplevel_item().is_err() && parser.is_eof() {
            reedline::ValidationResult::Incomplete
        } else {
            reedline::ValidationResult::Complete
        }
    }
}
