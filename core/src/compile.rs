use bumpalo::Bump;
use fnv::FnvHashMap;
use itertools::Itertools;
use thiserror::Error;

use crate::ast::{BinOp, Block, Expr, ExprKind, Func, Ident, Lit, NodeId, Stmt, StmtKind, Symbol};
use crate::interner::IndexTable;
use crate::types::infer::TypeError;
use crate::types::ty::IntTy;
use crate::types::{Ty, TyKind};
use crate::util::IdGen;
use crate::vm::chunk::{Chunk, ChunkBuilder};
use crate::vm::instr::{Addr, Instr, Reg};
use crate::vm::value::{ScalarTy, Value};

pub fn compile_func(
    ast: &Func,
    sym_table: &IndexTable<'_, Symbol, str>,
    type_map: FnvHashMap<NodeId, Ty<'_>>,
) -> Result<Chunk> {
    let builder = ChunkBuilder::new();
    let mut compiler = Compiler {
        builder,
        sym_table,
        type_map,
        locals: vec![],
        stack_pos: StackPos(0),
        stack_size: StackPos(0),
        labels: IdGen::new(|id| format!("L{id}")),
    };
    compiler.compile_func(ast)?;
    Ok(compiler.builder.build(compiler.stack_size.0 as usize))
}

struct Compiler<'a> {
    builder: ChunkBuilder,
    sym_table: &'a IndexTable<'a, Symbol, str>,
    type_map: FnvHashMap<NodeId, Ty<'a>>,
    locals: Vec<Symbol>,
    stack_pos: StackPos,
    stack_size: StackPos,
    labels: IdGen<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct StackPos(u16);

impl<'a> Compiler<'a> {
    fn compile_func(&mut self, ast: &Func) -> Result<()> {
        self.builder.ins_label("start");
        let r0 = self.compile_block(&ast.body, None)?;
        self.builder.ins(Instr::Return { r0 });
        Ok(())
    }

    fn compile_block(&mut self, ast: &Block, rd: Option<Reg>) -> Result<Reg> {
        for stmt in &ast.stmts {
            self.compile_stmt(stmt)?;
        }
        if let Some(expr) = &ast.tail {
            self.compile_expr(expr, rd)
        } else {
            let rd = rd.unwrap_or_else(|| self.push());
            let imm = self.builder.constant(Value::new_null());
            self.builder.ins(Instr::Load { rd, imm });
            Ok(rd)
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.node {
            StmtKind::Expr(expr) => {
                let sp = self.stack_pos;
                let value = self.compile_expr(expr, None)?;
                self.stack_pos = sp;
                Ok(())
            }
            StmtKind::Let(name, _, value, _) => {
                let (rd, sp) = self.reserve();
                self.compile_expr(value, Some(rd))?;
                self.stack_pos = sp;
                self.locals.push(name.sym);
                Ok(())
            }
            StmtKind::Assign(lhs, rhs) => {
                let sp = self.stack_pos;
                let rd = self.lookup_var(lhs)?;
                self.compile_expr(rhs, Some(rd))?;
                self.stack_pos = sp;
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, rd: Option<Reg>) -> Result<Reg> {
        match &expr.node {
            ExprKind::Lit(lit) => {
                let imm = self.builder.constant(Value::new_null());
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
            ExprKind::Binary(op, lhs, rhs, _) => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(lhs, None)?;
                let r1 = self.compile_expr(rhs, None)?;
                self.stack_pos = sp;

                let rd = rd.unwrap_or_else(|| self.push());
                match op {
                    BinOp::Add => self.builder.ins(Instr::Add { r0, r1, rd }),
                    BinOp::Sub => self.builder.ins(Instr::Sub { r0, r1, rd }),
                    BinOp::Mul => self.builder.ins(Instr::Mul { r0, r1, rd }),
                    BinOp::Div => self.builder.ins(Instr::UDiv { r0, r1, rd }), // FIXME
                    BinOp::Eq => self.builder.ins(Instr::Eq { r0, r1, rd }),
                    BinOp::NotEq => {
                        self.builder.ins(Instr::Eq { r0, r1, rd });
                        self.builder.ins(Instr::Not { r0: rd, rd });
                    }
                    BinOp::Lt => self.builder.ins(Instr::SLt { r0, r1, rd }), // FIXME
                    BinOp::LtEq => {
                        self.builder.ins(Instr::SLt { r0: r1, r1: r0, rd }); // FIXME
                        self.builder.ins(Instr::Not { r0: rd, rd });
                    }
                    BinOp::Gt => self.builder.ins(Instr::SLt { r0: r1, r1: r0, rd }), // FIXME
                    BinOp::GtEq => {
                        self.builder.ins(Instr::SLt { r0, r1, rd }); // FIXME
                        self.builder.ins(Instr::Not { r0: rd, rd });
                    }
                }
                Ok(rd)
            }
            ExprKind::Call(func, args) => {
                let ExprKind::Var(func) = func.node else { todo!() };
                match self.sym_table.get_str(func.sym) {
                    "print" => {
                        let [arg] = &args[..] else {
                            Err(CompilerError::Arity { exp: 1, act: args.len() })?
                        };
                        let r0 = self.compile_expr(arg, rd)?;
                        self.builder.ins(Instr::Print { r0 });
                        Ok(r0) // FIXME
                    }
                    name => Err(CompilerError::UndefinedName(name.into()))?,
                }
            }
            ExprKind::Grouped(expr) => self.compile_expr(expr, rd),
            ExprKind::Array(exprs) => {
                let sp = self.stack_pos;
                for (idx, expr) in exprs.iter().enumerate() {
                    let (rd, sp) = self.reserve();
                    self.compile_expr(expr, Some(rd))?;
                    self.stack_pos = sp;
                }
                self.stack_pos = sp;
                let (r0, rn) = self.top_n(exprs.len());

                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::MakeArray { r0, rn, rd });
                Ok(rd)
            }
            ExprKind::Index(expr, index) => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(expr, None)?;
                let r1 = self.compile_expr(index, None)?;
                self.stack_pos = sp;

                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::Index { r0, r1, rd });
                Ok(rd)
            }
            ExprKind::If { cond, then, r#else } => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(cond, rd)?;
                self.stack_pos = sp;

                let rd = rd.unwrap_or_else(|| self.push());

                let label = self.labels.next();
                self.builder.ins_jump_if_false(r0, &label);
                self.compile_expr(then, Some(rd))?;
                self.builder.ins_label(label);

                Ok(rd)
            }
            ExprKind::Block(block) => self.compile_block(block, rd),
        }
    }

    fn compile_lit(&mut self, lit: &Lit, rd: Reg) -> Result<()> {
        match lit {
            &Lit::Null => {
                let imm = self.builder.constant(Value::new_null());
                self.builder.ins(Instr::Load { rd, imm });
                Ok(())
            }
            &Lit::Bool(value) => {
                let imm = self.builder.constant(Value::new_bool(value));
                self.builder.ins(Instr::Load { rd, imm });
                Ok(())
            }
            &Lit::Int(sym) => {
                let value = self
                    .sym_table
                    .get_str(sym)
                    .parse()
                    .map_err(|_| CompilerError::InvalidLiteral)?;
                let ty = ScalarTy::Int(IntTy::Int32);
                let imm = self.builder.constant(Value::new_sint(value, ty));
                self.builder.ins(Instr::Load { rd, imm });
                Ok(())
            }
            &Lit::Float(sym) => {
                let value = self
                    .sym_table
                    .get_str(sym)
                    .parse()
                    .map_err(|_| CompilerError::InvalidLiteral)?;
                let imm = self.builder.constant(Value::new_f64(value));
                self.builder.ins(Instr::Load { rd, imm });
                Ok(())
            }
            &Lit::Str(str) => {
                let str = self.sym_table.get_str(str);
                let imm = self.builder.constant(Value::new_str(str));
                self.builder.ins(Instr::Load { rd, imm });
                Ok(())
            }
            _ => todo!(),
        }
    }

    fn lookup_var(&self, ident: &Ident) -> Result<Reg> {
        let index = self.locals.iter().position(|s| *s == ident.sym).ok_or_else(|| {
            let name = self.sym_table.get_str(ident.sym).into();
            CompilerError::UndefinedName(name)
        })?;
        Ok(Reg(index as u16))
    }

    fn top(&self) -> Reg {
        Reg(self.stack_pos.0)
    }

    fn top_n(&self, n: usize) -> (Reg, Reg) {
        (Reg(self.stack_pos.0), Reg(self.stack_pos.0 + n as u16))
    }

    fn reserve(&mut self) -> (Reg, StackPos) {
        let reg = Reg(self.stack_pos.0);
        let next = StackPos(self.stack_pos.0 + 1);
        self.stack_size = self.stack_size.max(next);
        (reg, next)
    }

    fn push(&mut self) -> Reg {
        let reg = Reg(self.stack_pos.0);
        self.stack_pos.0 += 1;
        self.stack_size = self.stack_size.max(self.stack_pos);
        reg
    }

    fn pop(&mut self) {
        self.stack_pos.0 -= 1;
    }
}

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("invalid literal")]
    InvalidLiteral,
    #[error("cannot find name `{0}`")]
    UndefinedName(Box<str>),
    #[error("expected {exp} arguments, got {act}")]
    Arity { exp: usize, act: usize },
    #[error("{0}")]
    TypeError(String),
}

pub type Result<T, E = CompilerError> = std::result::Result<T, E>;
