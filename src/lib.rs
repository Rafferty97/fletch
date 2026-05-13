#![allow(unused)]

use bumpalo::Bump;

use crate::ast::print::print_expr;

mod arena;
mod ast;
mod diagnostics;
mod lexer;
mod parser;
mod span;
mod types;

pub fn run(src: &str) {
    let arena = Bump::new();
    let mut handler = diagnostics::Diagnostics::new();
    let ctx = arena::Ctx::new(&arena, &mut handler);

    let mut parser = parser::Parser::new(ctx, src);
    let expr = parser.parse_expr().unwrap();

    println!("{}", print_expr(ctx, &expr));
}
