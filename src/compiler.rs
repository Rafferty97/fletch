use thiserror::Error;

use crate::ast::{Expr, ExprKind, Func, Lit, Stmt, StmtKind, Symbol};
use crate::interner::IndexedInterner;
use crate::vm::chunk::{Chunk, ChunkBuilder};
use crate::vm::instr::Width;
use crate::vm::instr::{Instr, Reg};
use crate::vm::value::Value;

pub fn compile_func(ast: &Func, sym_interner: &IndexedInterner<'_, Symbol, str>) -> Result<Chunk> {
    let builder = ChunkBuilder::new();
    let mut compiler = Compiler { builder, sym_interner };
    compiler.compile_func(ast)?;
    Ok(compiler.builder.build())
}

struct Compiler<'a, 'sym> {
    builder: ChunkBuilder,
    sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
}

impl<'a, 'sym> Compiler<'a, 'sym> {
    fn compile_func(&mut self, ast: &Func) -> Result<()> {
        for stmt in &ast.body.stmts {
            self.compile_stmt(stmt);
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.node {
            StmtKind::Print(expr) => {
                let value = self.compile_expr(expr)?;
                self.builder.ins(Instr::PrintInt(value));
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<Reg> {
        match &expr.node {
            ExprKind::Lit(lit) => self.compile_lit(lit),
            ExprKind::Binary(op, lhs, rhs) => todo!(),
        }
    }

    fn compile_lit(&mut self, lit: &Lit) -> Result<Reg> {
        match lit {
            &Lit::Int(sym) => {
                let reg = Reg(0);
                let value = self
                    .sym_interner
                    .get_str(sym)
                    .parse()
                    .map_err(|_| CompilerError::InvalidLiteral)?;
                let imm = self.builder.constant(Value::new_int(value));
                self.builder.ins(Instr::Const(reg, imm));
                Ok(reg)
            }
            _ => todo!(),
        }
    }
}

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("invalid literal")]
    InvalidLiteral,
}

pub type Result<T, E = CompilerError> = std::result::Result<T, E>;
