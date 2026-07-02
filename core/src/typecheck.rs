use std::marker::PhantomData;

use bumpalo::Bump;
use fnv::FnvHashMap;
use itertools::Itertools;
use thiserror::Error;

use crate::ast::span::Span;
use crate::ast::{self, BinOp, Block, Expr, ExprKind, Func, Lit, NodeId, Program, Stmt, StmtKind, Symbol, UnaryOp};
use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::interner::IndexTable;
use crate::name_resolution::{DefId, NameTables};
use crate::types::infer::TypeError;
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::{CommonTypes, TyInterners};
use crate::types::{Ty, TyKind};
use crate::util::Args;

pub struct TypeChecker<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    name_tables: &'a NameTables,
    type_map: FnvHashMap<NodeId, Ty<'ty>>,
    def_map: FnvHashMap<DefId, Ty<'ty>>,
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
            def_map: FnvHashMap::default(),
            sym_table,
            errors,
        }
    }

    pub fn finish(self) -> (FnvHashMap<NodeId, Ty<'ty>>, FnvHashMap<DefId, Ty<'ty>>) {
        (self.type_map, self.def_map)
    }

    pub fn check_program(&mut self, ast: &Program) {
        if let Some(def_id) = self.name_tables.print_def_id {
            self.def_map
                .insert(def_id, self.ty_ctx.mk_func(&[self.common().any], self.common().unit()));
        }

        for func in &ast.funcs {
            self.check_func_signature(func);
        }
        for func in &ast.funcs {
            self.check_func_body(func);
        }
    }

    pub fn check_func_signature(&mut self, ast: &Func) {
        // FIXME: dedupe logic with `check_func_body`
        let def_id = self.name_tables.uses[&ast.name.id].unwrap();
        let param_tys = ast.params.iter().map(|(_, ty)| self.lower_ty(ty)).collect_vec();
        let ret_ty = ast
            .ret
            .as_ref()
            .map(|ty| self.lower_ty(ty))
            .unwrap_or(self.common().unit());
        let ty = self.ty_ctx.mk_func(&param_tys, ret_ty);
        self.def_map.insert(def_id, ty);
    }

    pub fn check_func_body(&mut self, ast: &Func) {
        for (name, ty) in &ast.params {
            let def_id = *self.name_tables.uses.get(&name.id).unwrap(); // FIXME
            let ty = self.lower_ty(ty);
            if let Ok(def_id) = def_id {
                self.def_map.insert(def_id, ty);
            }
        }
        let ret_ty = ast
            .ret
            .as_ref()
            .map(|ty| self.lower_ty(ty))
            .unwrap_or(self.common().unit());
        self.check_block(&ast.body, ret_ty);
    }

    pub fn check_block(&mut self, ast: &Block, expected: Ty<'ty>) -> Ty<'ty> {
        for stmt in &ast.stmts {
            self.check_stmt(stmt);
        }
        if let Some(tail) = &ast.tail {
            self.check_expr(tail, expected)
        } else {
            self.common().unit()
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
                    self.def_map.insert(def_id, ty); // FIXME?
                }
            }
            StmtKind::Assign(lhs, rhs) => {
                let expected = *self
                    .name_tables
                    .uses
                    .get(&lhs.id)
                    .and_then(|def_id| def_id.ok())
                    .and_then(|def_id| self.def_map.get(&def_id))
                    .unwrap_or(&self.common().infer);
                self.check_expr(rhs, expected);
            }
        }
    }

    fn infer_expr(&mut self, ast: &Expr) -> Ty<'ty> {
        self.check_expr(ast, self.common().infer)
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
                    .map(|def_id| *self.def_map.get(&def_id).unwrap()) // FIXME: unwrap
                    .unwrap_or_else(|err| self.ty_ctx.mk_error(err)) // FIXME: unwrap
            }
            ExprKind::Unary(op, rhs, span) => {
                let rhs = self.check_expr(rhs, self.common().infer);
                match (op, rhs.kind()) {
                    (UnaryOp::Not, TyKind::Bool) => rhs,
                    (UnaryOp::Negate, TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_)) => rhs,
                    (_, _) => {
                        let msg = format!("cannot apply unary operator '{op}' to '{rhs}'");
                        let err = self.errors.report_err(Diagnostic::error(msg, *span));
                        self.ty_ctx.mk_error(err)
                    }
                }
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
            ExprKind::Call(func, args, span) => {
                let func_ty = self.infer_expr(func);
                match func_ty.kind() {
                    TyKind::Func(func) => {
                        if func.params.len() != args.len() {
                            let msg = format!("expected {}, found {}", Args(func.params.len()), args.len());
                            let diagnostic = Diagnostic::error(msg, *span);
                            let err = self.errors.report_err(diagnostic);
                        }

                        let infer = self.common().infer;
                        for (idx, arg) in args.iter().enumerate() {
                            let expected = func.params.get(idx).copied().unwrap_or(infer);
                            self.check_expr(arg, expected);
                        }
                        func.ret
                    }
                    TyKind::Error(err) => {
                        for arg in args {
                            self.infer_expr(arg);
                        }
                        self.ty_ctx.mk_error(err)
                    }
                    _ => {
                        let msg = format!("expected a function, found '{func_ty}'");
                        let diagnostic = Diagnostic::error(msg, func.span);
                        let err = self.errors.report_err(diagnostic);
                        for arg in args {
                            self.infer_expr(arg);
                        }
                        self.ty_ctx.mk_error(err)
                    }
                }
            }
            ExprKind::Grouped(expr) => self.check_expr(expr, expected),
            ExprKind::Array(exprs) => {
                let expected = expected.element_ty().unwrap_or(self.common().infer);
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
                    None => self.common().unit(),
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
                "never" => self.common().never,
                "null" => self.common().opt_never,
                "bool" => self.common().bool,
                "uint8" => self.common().uint8,
                "uint16" => self.common().uint16,
                "uint32" => self.common().uint32,
                "uint64" => self.common().uint64,
                "uint" => self.common().uint64,
                "int8" => self.common().int8,
                "int16" => self.common().int16,
                "int32" => self.common().int32,
                "int64" => self.common().int64,
                "int" => self.common().int64,
                "float32" => self.common().float32,
                "float64" => self.common().float64,
                "float" => self.common().float64,
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
            ast::TyKind::Tuple(tys) => {
                let tys = tys.into_iter().map(|ty| self.lower_ty(ty)).collect_vec();
                self.ty_ctx.mk_tuple(&tys)
            }
        }
    }

    fn common(&self) -> &CommonTypes<'ty> {
        self.ty_ctx.common()
    }
}
