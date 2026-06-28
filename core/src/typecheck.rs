use std::marker::PhantomData;

use bumpalo::Bump;
use fnv::FnvHashMap;
use itertools::Itertools;
use thiserror::Error;

use crate::ast::span::Span;
use crate::ast::{self, BinOp, Block, Expr, ExprKind, Func, Lit, NodeId, Program, Stmt, StmtKind, Symbol};
use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::interner::IndexTable;
use crate::name_resolution::{DefId, NameTables};
use crate::types::infer::TypeError;
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::{CommonTypes, TyInterners};
use crate::types::{Ty, TyKind};

pub struct TypeChecker<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    name_tables: &'a NameTables,
    type_map: FnvHashMap<NodeId, Ty<'ty>>,
    locals: FnvHashMap<DefId, Ty<'ty>>,
    sym_table: &'a IndexTable<'a, Symbol, str>,
    errors: &'a dyn DiagnosticReporter,
}

pub type Result<'ty, T> = std::result::Result<T, TypeError<'ty>>;

impl<'a, 'ty> TypeChecker<'a, 'ty> {
    pub fn new(
        ty_ctx: TyCtx<'a, 'ty>,
        name_tables: &'a NameTables,
        sym_table: &'a IndexTable<'a, Symbol, str>,
        errors: &'a dyn DiagnosticReporter,
    ) -> Self {
        Self {
            ty_ctx,
            name_tables,
            type_map: FnvHashMap::default(),
            locals: FnvHashMap::default(),
            sym_table,
            errors,
        }
    }

    pub fn finish(self) -> FnvHashMap<NodeId, Ty<'ty>> {
        self.type_map
    }

    pub fn check_program(&mut self, ast: &Program) {
        for func in &ast.funcs {
            self.check_func(func);
        }
    }

    pub fn check_func(&mut self, ast: &Func) {
        self.locals.clear();
        for (name, ty) in &ast.params {
            let def_id = *self.name_tables.uses.get(&name.id).unwrap(); // FIXME
            let ty = self.lower_ty(ty);
            if let Ok(def_id) = def_id {
                self.locals.insert(def_id, ty);
            }
        }
        let ret_ty = ast
            .ret
            .as_ref()
            .map(|ty| self.lower_ty(ty))
            .unwrap_or(self.common().opt_never);
        self.check_block(&ast.body, ret_ty);
    }

    pub fn check_block(&mut self, ast: &Block, expected: Ty<'ty>) -> Ty<'ty> {
        for stmt in &ast.stmts {
            self.check_stmt(stmt);
        }
        if let Some(tail) = &ast.tail {
            self.check_expr(tail, expected)
        } else {
            self.common().opt_never
        }
    }

    fn check_stmt(&mut self, ast: &Stmt) {
        match &ast.node {
            StmtKind::Expr(expr) => {
                self.check_expr(&*expr, self.common().infer);
            }
            StmtKind::Let(name, ty, value, _) => {
                let def_id = *self.name_tables.uses.get(&name.id).unwrap(); // FIXME
                let expected = ty.as_ref().map(|ty| self.lower_ty(ty)).unwrap_or(self.common().infer);
                let ty = self.check_expr(&*value, expected);
                if let Ok(def_id) = def_id {
                    self.locals.insert(def_id, ty); // FIXME?
                }
            }
            StmtKind::Assign(lhs, rhs) => {
                let expected = *self
                    .name_tables
                    .uses
                    .get(&lhs.id)
                    .and_then(|def_id| def_id.ok())
                    .and_then(|def_id| self.locals.get(&def_id))
                    .unwrap_or(&self.common().infer);
                self.check_expr(rhs, expected);
            }
        }
    }

    fn check_expr(&mut self, ast: &Expr, expected: Ty<'ty>) -> Ty<'ty> {
        let actual = match &ast.node {
            ExprKind::Lit(lit) => match lit {
                Lit::Null => self.common().opt_never,
                Lit::Bool(_) => self.common().bool,
                Lit::Int(_) => self.common().int32,     // FIXME
                Lit::Float(_) => self.common().float64, // FIXME
                Lit::Str(_) => self.common().str,
                Lit::Err(err) => self.ty_ctx.mk_error(*err),
            },
            ExprKind::Var(name) => {
                self.name_tables
                    .uses
                    .get(&name.id)
                    .unwrap() // FIXME: unwrap
                    .map(|def_id| *self.locals.get(&def_id).unwrap()) // FIXME: unwrap
                    .unwrap_or_else(|err| self.ty_ctx.mk_error(err)) // FIXME: unwrap
            }
            ExprKind::Binary(op, lhs, rhs, span) => {
                let lhs = self.check_expr(lhs, self.common().infer);
                let rhs = self.check_expr(rhs, self.common().infer);
                let result = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => (lhs == rhs).then_some(lhs),
                    BinOp::Eq | BinOp::NotEq => {
                        if self.ty_ctx.meet(lhs, rhs).is_never() {
                            let msg =
                                format!("this comparison can never be true as '{lhs}' and '{rhs}' have no overlap");
                            let diagnostic = Diagnostic::warning(msg, *span);
                            self.errors.report(diagnostic);
                        }
                        Some(self.common().bool)
                    }
                    BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => Some(self.common().bool),
                };
                match result {
                    Some(ty) => ty,
                    None => {
                        let msg = format!("no implementation for '{lhs}' {op} '{rhs}'");
                        let err = self.errors.report_err(Diagnostic::error(msg, *span));
                        self.ty_ctx.mk_error(err)
                    }
                }
            }
            ExprKind::Call(func, args) => {
                for arg in args {
                    self.check_expr(arg, self.common().infer);
                }
                self.common().opt_never // FIXME
            }
            ExprKind::Grouped(expr) => self.check_expr(expr, expected),
            ExprKind::Array(exprs) => {
                let expected = match expected.kind() {
                    TyKind::Array(ty) => ty,
                    _ => self.common().infer,
                };
                let never = self.common().never;
                let expr_tys = exprs.iter().map(|expr| self.check_expr(expr, expected)).collect_vec();
                let element_ty = expr_tys.into_iter().fold(never, |a, b| self.ty_ctx.join(a, b));
                self.ty_ctx.mk_array(element_ty)
            }
            ExprKind::Index(expr, index) => {
                let expr_ty = self.check_expr(expr, self.common().infer);
                self.check_expr(index, self.common().int64);
                match expr_ty.kind() {
                    TyKind::Array(el) => el,
                    TyKind::Error(_) => expr_ty,
                    _ => {
                        let msg = format!("cannot index into type '{}'", expr_ty);
                        let diagnostic = Diagnostic::error(msg, expr.span);
                        self.ty_ctx.mk_error(self.errors.report_err(diagnostic))
                    }
                }
            }
            ExprKind::If { cond, then, r#else } => {
                self.check_expr(cond, self.common().bool);
                let then_ty = self.check_expr(then, expected);
                let else_ty = match r#else {
                    Some(r#else) => self.check_expr(r#else, expected),
                    None => self.common().opt_never,
                };
                self.ty_ctx.join(then_ty, else_ty)
            }
            ExprKind::Block(block) => self.check_block(block, expected),
        };

        let ty = if expected.is_final() {
            match self.ty_ctx.reconcile(actual, expected) {
                Ok(ty) => ty,
                Err(err) => {
                    let diagnostic = Diagnostic::error(err.to_string(), ast.span);
                    self.ty_ctx.mk_error(self.errors.report_err(diagnostic))
                }
            }
        } else {
            actual
        };

        self.type_map.insert(ast.id, ty);

        ty
    }

    fn lower_ty(&mut self, ast: &ast::Ty) -> Ty<'ty> {
        match &ast.node {
            ast::TyKind::Infer => self.common().infer,
            ast::TyKind::Var(ident) => match self.sym_table.get_str(ident.sym) {
                "bool" => self.common().bool,
                "u8" => self.common().uint8,
                "u16" => self.common().uint16,
                "u32" => self.common().uint32,
                "u64" => self.common().uint64,
                "i8" => self.common().int8,
                "i16" => self.common().int16,
                "i32" => self.common().int32,
                "i64" => self.common().int64,
                "str" => self.common().str,
                name => {
                    let msg = format!("cannot find type '{name}' in scope");
                    let diagnostic = Diagnostic::error(msg, ident.span);
                    self.ty_ctx.mk_error(self.errors.report_err(diagnostic))
                }
            },
            ast::TyKind::Nullable(inner) => {
                let inner = self.lower_ty(inner);
                self.ty_ctx.mk_nullable(inner)
            }
            ast::TyKind::Array(inner) => {
                let inner = self.lower_ty(inner);
                self.ty_ctx.mk_array(inner)
            }
        }
    }

    fn common(&self) -> &CommonTypes<'ty> {
        self.ty_ctx.common()
    }
}
