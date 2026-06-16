use bumpalo::Bump;
use thiserror::Error;

use crate::ast::{ExprKind, Lit, StmtKind};
use crate::interner::IndexedInterner;
use crate::parser::error::ParseError;
use crate::parser::{ParseCtx, Parser};

pub fn run(src: &str) -> Result<(), String> {
    // Create arena and interners
    let arena = Bump::new();
    let sym_interner = IndexedInterner::new();

    // Parse
    let ctx = ParseCtx::new(&arena, &sym_interner);
    let mut parser = Parser::new(ctx, src);
    let ast = parser.parse_program().map_err(|e| e.to_string())?;

    // Execute
    for stmt in ast.main.body.stmts {
        match stmt.node {
            StmtKind::Print(expr) => match expr.node {
                ExprKind::Lit(lit) => {
                    let value = match lit {
                        Lit::Int(sym) => {
                            let raw: i64 = sym_interner.get_str(sym).parse().map_err(|_| "invalid int literal")?;
                            raw
                        }
                    };
                    println!("{value}");
                }
            },
        }
    }

    Ok(())
}
