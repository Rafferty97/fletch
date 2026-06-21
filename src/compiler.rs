use itertools::Itertools;
use thiserror::Error;

use crate::ast::{BinOp, Expr, ExprKind, Func, Ident, Lit, Stmt, StmtKind, Symbol};
use crate::interner::IndexedInterner;
use crate::vm::chunk::{Chunk, ChunkBuilder};
use crate::vm::instr::{Instr, Reg};
use crate::vm::value::Value;

pub fn compile_func(ast: &Func, sym_interner: &IndexedInterner<'_, Symbol, str>) -> Result<Chunk> {
    let builder = ChunkBuilder::new();
    let mut compiler = Compiler { builder, sym_interner, locals: vec![], stack_pos: 0, stack_size: 0 };
    compiler.compile_func(ast)?;
    Ok(compiler.builder.build(compiler.stack_size as usize))
}

struct Compiler<'a, 'sym> {
    builder: ChunkBuilder,
    sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
    locals: Vec<Symbol>,
    stack_pos: u16,
    stack_size: u16,
}

impl<'a, 'sym> Compiler<'a, 'sym> {
    fn compile_func(&mut self, ast: &Func) -> Result<()> {
        for stmt in &ast.body.stmts {
            self.compile_stmt(stmt);
        }
        self.builder.ins(Instr::Return);
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.node {
            StmtKind::Let(name, value) => {
                let sp = self.stack_pos;
                let rd = self.reserve();
                let value = self.compile_expr(value, Some(rd))?;
                self.stack_pos = sp + 1;
                self.locals.push(name.sym);
                Ok(())
            }
            StmtKind::Assign(lhs, rhs) => {
                let ExprKind::Var(lhs) = &lhs.node else {
                    Err(CompilerError::InvalidAssignment)?
                };
                let sp = self.stack_pos;
                let rd = self.lookup_var(lhs)?;
                let value = self.compile_expr(rhs, Some(rd))?;
                self.stack_pos = sp;
                Ok(())
            }
            StmtKind::Print(expr) => {
                let sp = self.stack_pos;
                let value = self.compile_expr(expr, None)?;
                self.builder.ins(Instr::PrintInt(value));
                self.stack_pos = sp;
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, rd: Option<Reg>) -> Result<Reg> {
        match &expr.node {
            ExprKind::Lit(lit) => {
                let rd = rd.unwrap_or_else(|| self.push());
                self.compile_lit(lit, rd)?;
                Ok(rd)
            }
            ExprKind::Var(ident) => {
                let r0 = self.lookup_var(ident)?;
                Ok(match rd {
                    Some(rd) if rd != r0 => {
                        self.builder.ins(Instr::Move { r0, rd });
                        rd
                    }
                    _ => r0,
                })
            }
            ExprKind::Binary(op, lhs, rhs) => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(lhs, None)?;
                let r1 = self.compile_expr(rhs, None)?;
                self.stack_pos = sp;

                let rd = rd.unwrap_or_else(|| self.push());
                match op {
                    BinOp::Add => self.builder.ins(Instr::Add { r0, r1, rd }),
                }
                Ok(rd)
            }
        }
    }

    fn compile_lit(&mut self, lit: &Lit, rd: Reg) -> Result<()> {
        match lit {
            &Lit::Int(sym) => {
                let value = self
                    .sym_interner
                    .get_str(sym)
                    .parse()
                    .map_err(|_| CompilerError::InvalidLiteral)?;
                let imm = self.builder.constant(Value::new_int(value));
                self.builder.ins(Instr::Const(rd, imm));
                Ok(())
            }
            _ => todo!(),
        }
    }

    fn lookup_var(&self, ident: &Ident) -> Result<Reg> {
        let index = self.locals.iter().position(|s| *s == ident.sym).ok_or_else(|| {
            let name = self.sym_interner.get_str(ident.sym).into();
            CompilerError::UndefinedName(name)
        })?;
        Ok(Reg(index as u16))
    }

    fn reserve(&mut self) -> Reg {
        let reg = Reg(self.stack_pos);
        self.stack_size = self.stack_size.max(self.stack_pos + 1);
        reg
    }

    fn push(&mut self) -> Reg {
        let reg = Reg(self.stack_pos);
        self.stack_pos += 1;
        self.stack_size = self.stack_size.max(self.stack_pos);
        reg
    }

    fn pop(&mut self) {
        self.stack_pos -= 1;
    }
}

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("invalid literal")]
    InvalidLiteral,
    #[error("cannot find name `{0}`")]
    UndefinedName(Box<str>),
    #[error("cannot assign to this expression")]
    InvalidAssignment,
}

pub type Result<T, E = CompilerError> = std::result::Result<T, E>;
