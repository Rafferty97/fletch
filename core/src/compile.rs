use bumpalo::Bump;
use fnv::FnvHashMap;
use itertools::Itertools;
use thiserror::Error;

use crate::ast::{BinOp, Block, Expr, ExprKind, Func, Ident, Lit, NodeId, Program, Stmt, StmtKind, Symbol, UnaryOp};
use crate::diagnostics::{DiagnosticReporter, ErrGuaranteed};
use crate::interner::IndexTable;
use crate::name_resolution::DefId;
use crate::types::infer::TypeError;
use crate::types::ty::IntTy;
use crate::types::{Ty, TyKind};
use crate::util::IdGen;
use crate::vm::chunk::{Chunk, ChunkBuilder};
use crate::vm::instr::{Addr, Instr, Reg, Width};
use crate::vm::module::{FuncId, Module};
use crate::vm::value3::{FuncObj, ScalarTy, Value};

pub fn compile_program(
    ast: &Program,
    sym_table: &IndexTable<'_, Symbol, str>,
    uses: &FnvHashMap<NodeId, Result<DefId, ErrGuaranteed>>,
    type_map: &FnvHashMap<NodeId, Ty<'_>>,
) -> Module {
    let mut main = Option::<FuncId>::None;
    let mut func_ids = IdGen::new(FuncId);
    let mut funcs = FnvHashMap::default();
    let mut func_defs = FnvHashMap::default();

    for func in &ast.funcs {
        let def_id = uses[&func.name.id].unwrap();
        let func_id = func_ids.next();
        func_defs.insert(def_id, func_id);
    }

    for func in &ast.funcs {
        let def_id = uses[&func.name.id].unwrap();
        let func_id = func_defs[&def_id];
        let chunk = compile_func(func, func_id, sym_table, uses, type_map, &func_defs);
        let name = sym_table.get_str(func.name.sym).into();
        if name == "main" {
            main = Some(func_id);
        }
        let func = FuncObj { name, chunk };
        funcs.insert(func_id, func.into());
    }

    let Some(main) = main else {
        panic!("no main function");
    };

    Module { funcs, main }
}

pub fn compile_func(
    ast: &Func,
    func_id: FuncId,
    sym_table: &IndexTable<'_, Symbol, str>,
    uses: &FnvHashMap<NodeId, Result<DefId, ErrGuaranteed>>,
    type_map: &FnvHashMap<NodeId, Ty<'_>>,
    funcs: &FnvHashMap<DefId, FuncId>,
) -> Chunk {
    let builder = ChunkBuilder::new();
    let mut compiler = Compiler {
        builder,
        sym_table,
        uses,
        type_map,
        locals: vec![],
        funcs,
        stack_pos: StackPos(0),
        stack_size: StackPos(0),
        labels: IdGen::new(|id| format!("L{id}")),
    };
    compiler.compile_func(ast);
    compiler.builder.build(func_id, compiler.stack_size.0 as usize)
}

struct Compiler<'a> {
    builder: ChunkBuilder,
    sym_table: &'a IndexTable<'a, Symbol, str>,
    uses: &'a FnvHashMap<NodeId, Result<DefId, ErrGuaranteed>>,
    type_map: &'a FnvHashMap<NodeId, Ty<'a>>,
    locals: Vec<DefId>,
    funcs: &'a FnvHashMap<DefId, FuncId>,
    stack_pos: StackPos,
    stack_size: StackPos,
    labels: IdGen<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct StackPos(u16);

impl<'a> Compiler<'a> {
    fn compile_func(&mut self, ast: &Func) -> () {
        self.builder.ins_label("start");

        for (name, _) in &ast.params {
            let def_id = self.uses[&name.id].unwrap();
            self.locals.push(def_id);
        }
        self.stack_pos = StackPos(ast.params.len() as u16);
        self.stack_size = self.stack_pos;

        let r0 = self.compile_block(&ast.body, None);
        self.builder.ins(Instr::Return { r0 });
    }

    fn compile_block(&mut self, ast: &Block, rd: Option<Reg>) -> Reg {
        for stmt in &ast.stmts {
            self.compile_stmt(stmt);
        }
        if let Some(expr) = &ast.tail {
            self.compile_expr(expr, rd)
        } else {
            let rd = rd.unwrap_or_else(|| self.push());
            self.builder.ins(Instr::LoadUnit { rd });
            rd
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> () {
        match &stmt.node {
            StmtKind::Expr(expr) => {
                let sp = self.stack_pos;
                let value = self.compile_expr(expr, None);
                self.stack_pos = sp;
            }
            StmtKind::Let(name, _, value, _) => {
                let (rd, sp) = self.reserve();
                self.compile_expr(value, Some(rd));
                self.stack_pos = sp;
                self.locals.push(self.uses[&name.id].unwrap());
            }
            StmtKind::Assign(lhs, rhs) => {
                let sp = self.stack_pos;
                let def_id = self.uses[&lhs.id].unwrap();
                let rd = self.lookup_var(def_id);
                self.compile_expr(rhs, Some(rd));
                self.stack_pos = sp;
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, rd: Option<Reg>) -> Reg {
        match &expr.node {
            ExprKind::Lit(lit) => {
                let rd = rd.unwrap_or_else(|| self.push());
                self.compile_lit(lit, rd);
                rd
            }
            ExprKind::Var(ident) => {
                let def_id = self.uses[&ident.id].unwrap();

                // Top-level functions
                if let Some(&func_id) = self.funcs.get(&def_id) {
                    let rd = rd.unwrap_or_else(|| self.push());
                    let imm = self.builder.constant(Value::new_func(func_id));
                    self.builder.ins(Instr::Load { rd, imm });
                    return rd;
                }

                // Local vars
                let r0 = self.lookup_var(def_id);
                match rd {
                    Some(rd) if rd != r0 => {
                        self.builder.ins(Instr::Move { r0, rd });
                        rd
                    }
                    _ => r0,
                }
            }
            ExprKind::Unary(op, rhs, _) => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(rhs, None);
                self.stack_pos = sp;

                let ty = *self.type_map.get(&rhs.id).unwrap();

                let rd = rd.unwrap_or_else(|| self.push());
                match (op, ty.kind()) {
                    (UnaryOp::Not, TyKind::Bool) => self.builder.ins(Instr::Not { r0, rd }),
                    (UnaryOp::Not, _) => unreachable!(),

                    (UnaryOp::Negate, TyKind::Int(ty)) => self.builder.ins(Instr::Neg { w: ty.width(), r0, rd }),
                    (UnaryOp::Negate, TyKind::Float(ty)) => self.builder.ins(Instr::FNeg { w: ty.width(), r0, rd }),
                    (UnaryOp::Negate, _) => unreachable!(),
                }
                rd
            }
            ExprKind::Binary(op, lhs, rhs, _) => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(lhs, None);
                let r1 = self.compile_expr(rhs, None);
                self.stack_pos = sp;

                let ty = *self.type_map.get(&lhs.id).unwrap();

                let rd = rd.unwrap_or_else(|| self.push());
                match (op, ty.kind()) {
                    (BinOp::Add, TyKind::Int(ty)) => self.builder.ins(Instr::Add { w: ty.width(), r0, r1, rd }),
                    (BinOp::Add, TyKind::UInt(ty)) => self.builder.ins(Instr::Add { w: ty.width(), r0, r1, rd }),
                    (BinOp::Add, TyKind::Float(ty)) => self.builder.ins(Instr::FAdd { w: ty.width(), r0, r1, rd }),
                    (BinOp::Add, _) => unreachable!(),

                    (BinOp::Sub, TyKind::Int(ty)) => self.builder.ins(Instr::Sub { w: ty.width(), r0, r1, rd }),
                    (BinOp::Sub, TyKind::UInt(ty)) => self.builder.ins(Instr::Sub { w: ty.width(), r0, r1, rd }),
                    (BinOp::Sub, TyKind::Float(ty)) => self.builder.ins(Instr::FSub { w: ty.width(), r0, r1, rd }),
                    (BinOp::Sub, _) => unreachable!(),

                    (BinOp::Mul, TyKind::Int(ty)) => self.builder.ins(Instr::Mul { w: ty.width(), r0, r1, rd }),
                    (BinOp::Mul, TyKind::UInt(ty)) => self.builder.ins(Instr::Mul { w: ty.width(), r0, r1, rd }),
                    (BinOp::Mul, TyKind::Float(ty)) => self.builder.ins(Instr::FMul { w: ty.width(), r0, r1, rd }),
                    (BinOp::Mul, _) => unreachable!(),

                    (BinOp::Div, TyKind::Int(ty)) => self.builder.ins(Instr::SDiv { w: ty.width(), r0, r1, rd }),
                    (BinOp::Div, TyKind::UInt(ty)) => self.builder.ins(Instr::UDiv { w: ty.width(), r0, r1, rd }),
                    (BinOp::Div, TyKind::Float(ty)) => self.builder.ins(Instr::FDiv { w: ty.width(), r0, r1, rd }),
                    (BinOp::Div, _) => unreachable!(),

                    (BinOp::Eq, _) => self.builder.ins(Instr::Eq { r0, r1, rd }),
                    (BinOp::NotEq, _) => {
                        self.builder.ins(Instr::Eq { r0, r1, rd });
                        self.builder.ins(Instr::Not { r0: rd, rd });
                    }

                    (BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq, _) => {
                        let (r0, r1) = match op {
                            BinOp::Lt | BinOp::GtEq => (r0, r1),
                            BinOp::Gt | BinOp::LtEq => (r1, r0),
                            _ => unreachable!(),
                        };
                        match ty.kind() {
                            TyKind::Int(_) => self.builder.ins(Instr::SLt { r0, r1, rd }),
                            TyKind::UInt(_) => self.builder.ins(Instr::ULt { r0, r1, rd }),
                            TyKind::Float(ty) => self.builder.ins(Instr::FLt { w: ty.width(), r0, r1, rd }),
                            _ => unreachable!(),
                        }
                        match op {
                            BinOp::Lt | BinOp::Gt => {}
                            BinOp::LtEq | BinOp::GtEq => self.builder.ins(Instr::Not { r0: rd, rd }),
                            _ => unreachable!(),
                        }
                    }
                }
                rd
            }
            ExprKind::Call(func, args, _) => {
                // FIXME: remove
                if let ExprKind::Var(func) = func.node
                    && self.sym_table.get_str(func.sym) == "print"
                {
                    let [arg] = &args[..] else {
                        panic!("expected one argument");
                    };
                    let r0 = self.compile_expr(arg, rd);
                    self.builder.ins(Instr::Print { r0 });
                    let rd = rd.unwrap_or_else(|| self.push());
                    self.builder.ins(Instr::LoadUnit { rd });
                    return rd;
                }

                let sp = self.stack_pos;
                for expr in std::iter::once(&**func).chain(args.iter()) {
                    let (rd, sp) = self.reserve();
                    self.compile_expr(expr, Some(rd));
                    self.stack_pos = sp;
                }
                self.stack_pos = sp;
                let func = self.top();

                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::Call { func, rd });
                rd
            }
            ExprKind::Array(exprs) => {
                let sp = self.stack_pos;
                for expr in exprs.iter() {
                    let (rd, sp) = self.reserve();
                    self.compile_expr(expr, Some(rd));
                    self.stack_pos = sp;
                }
                self.stack_pos = sp;
                let (r0, rn) = self.top_n(exprs.len());

                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::MakeArray { r0, rn, rd });
                rd
            }
            ExprKind::Index(expr, index) => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(expr, None);
                let r1 = self.compile_expr(index, None);
                self.stack_pos = sp;

                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::Index { r0, r1, rd });
                rd
            }
            ExprKind::Tuple(exprs) if exprs.is_empty() => {
                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::LoadUnit { rd });
                rd
            }
            ExprKind::Tuple(exprs) => {
                let sp = self.stack_pos;
                for expr in exprs.iter() {
                    let (rd, sp) = self.reserve();
                    self.compile_expr(expr, Some(rd));
                    self.stack_pos = sp;
                }
                self.stack_pos = sp;
                let (r0, rn) = self.top_n(exprs.len());

                let rd = rd.unwrap_or_else(|| self.push());
                self.builder.ins(Instr::MakeTuple { r0, rn, rd });
                rd
            }
            ExprKind::If { cond, then, r#else } => {
                let sp = self.stack_pos;
                let r0 = self.compile_expr(cond, rd);
                self.stack_pos = sp;

                let (rd, sp) = match rd {
                    Some(rd) => (rd, self.stack_pos),
                    None => self.reserve(),
                };

                let else_label = self.labels.next();
                let end_label = self.labels.next();

                self.builder.ins_jump_if_false(r0, &else_label);
                self.compile_expr(then, Some(rd));
                self.builder.ins_jump(&end_label);
                self.builder.ins_label(else_label);
                match r#else {
                    Some(r#else) => {
                        self.compile_expr(r#else, Some(rd));
                    }
                    None => self.compile_lit(&Lit::Null, rd),
                }
                self.builder.ins_label(end_label);

                rd
            }
            ExprKind::Block(block) => self.compile_block(block, rd),
        }
    }

    fn compile_lit(&mut self, lit: &Lit, rd: Reg) -> () {
        match lit {
            &Lit::Null => self.builder.ins(Instr::LoadNull { rd }),
            &Lit::Bool(false) => self.builder.ins(Instr::LoadFalse { rd }),
            &Lit::Bool(true) => self.builder.ins(Instr::LoadTrue { rd }),
            &Lit::Int(sym) => {
                let value = self.sym_table.get_str(sym).parse().unwrap(); // FIXME: unwrap
                let width = Width::_32; // FIXME
                if value == 0 {
                    self.builder.ins(Instr::LoadZero { w: width, rd });
                    return;
                }
                let imm = self.builder.constant(Value::new_sint(value, width));
                self.builder.ins(Instr::Load { rd, imm });
            }
            &Lit::Float(sym) => {
                let value = self.sym_table.get_str(sym).parse().unwrap(); // FIXME: unwrap
                let width = Width::_64; // FIXME
                if value == 0.0 {
                    self.builder.ins(Instr::LoadFZero { w: width, rd });
                    return;
                }
                let imm = self.builder.constant(Value::new_f64(value)); // FIXME
                self.builder.ins(Instr::Load { rd, imm });
            }
            &Lit::Str(str) => {
                let str = self.sym_table.get_str(str);
                let imm = self.builder.constant(Value::new_str(str));
                self.builder.ins(Instr::Load { rd, imm });
            }
            &Lit::Err(_) => unreachable!(),
        }
    }

    fn lookup_var(&self, def_id: DefId) -> Reg {
        let index = self.locals.iter().position(|s| *s == def_id).unwrap(); // FIXME: unwrap
        Reg(index as u16)
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
