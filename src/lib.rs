#![allow(unused)]

use crate::typecheck::with_ty_ctx;

mod ast;
mod ast_infer;
mod error;
mod interpreter;
mod ir;
mod lower;
mod parser;
mod test;
mod typecheck;
mod util;

pub fn run(src: &str) {
    let mut parser = parser::Parser::new(src);
    let ast = parser.parse_program().unwrap();
    let result = with_ty_ctx(|tcx| {
        let ir = lower::lower_program(&ast, tcx).unwrap();
        interpreter::interpret_program(ir)
    });
    println!("{result:?}");
}
