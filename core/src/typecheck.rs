use std::marker::PhantomData;

use bumpalo::Bump;
use codespan_reporting::diagnostic;
use fnv::FnvHashMap;
use itertools::Itertools;
use thiserror::Error;

use crate::ast::span::Span;
use crate::ast::{
    self, BinOp, Block, Expr, ExprKind, Func, Ident, Lit, NodeId, Program, Stmt, StmtKind, Symbol, UnaryOp,
};
use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::interner::IndexTable;
use crate::name_resolution::{DefId, NameTables};
use crate::types::infer::{Bound, TyBounds, TypeError};
use crate::types::ty::{ParamId, VarId};
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::{CommonTypes, TyInterners};
use crate::types::{Ty, TyKind};
use crate::util::{Args, IdGen};

pub struct TypeChecker<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    name_tables: &'a NameTables,
    type_map: FnvHashMap<NodeId, Ty<'ty>>,
    def_map: FnvHashMap<DefId, Def<'ty>>,
    sym_table: &'a IndexTable<'a, Symbol, str>,
    errors: &'a dyn DiagnosticReporter,
    ty_vars: Vec<Vec<TyBounds<'ty>>>,
}

#[derive(Clone, Debug)]
pub enum Def<'ty> {
    Func(FuncDef<'ty>),
    Var(Ty<'ty>),
}

#[derive(Clone, Debug)]
pub struct FuncDef<'ty> {
    name: Symbol,
    ty_params: Vec<Ident>,
    params: Vec<Ty<'ty>>,
    ret: Ty<'ty>,
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
            ty_vars: vec![],
        }
    }

    pub fn finish(self) -> FnvHashMap<NodeId, Ty<'ty>> {
        self.type_map
    }

    pub fn check_program(&mut self, ast: &Program) {
        if let Some(def_id) = self.name_tables.print_def_id {
            let func = FuncDef {
                name: self.sym_table.find_str("print").unwrap(),
                ty_params: vec![],
                params: vec![self.common().any],
                ret: self.common().unit(),
            };
            self.def_map.insert(def_id, Def::Func(func));
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
        let param_tys = ast
            .params
            .iter()
            .map(|(_, ty)| self.lower_ty_with_params(ty, &ast.ty_params))
            .collect_vec();
        let ret_ty = ast
            .ret
            .as_ref()
            .map(|ty| self.lower_ty_with_params(ty, &ast.ty_params))
            .unwrap_or(self.common().unit());
        let func = FuncDef {
            name: ast.name.sym,
            ty_params: ast.ty_params.clone(),
            params: param_tys,
            ret: ret_ty,
        };
        self.def_map.insert(def_id, Def::Func(func));
    }

    pub fn check_func_body(&mut self, ast: &Func) {
        for (name, ty) in &ast.params {
            let def_id = *self.name_tables.uses.get(&name.id).unwrap(); // FIXME
            let ty = self.lower_ty_with_params(ty, &ast.ty_params);
            if let Ok(def_id) = def_id {
                self.def_map.insert(def_id, Def::Var(ty));
            }
        }
        let ret_ty = ast
            .ret
            .as_ref()
            .map(|ty| self.lower_ty_with_params(ty, &ast.ty_params))
            .unwrap_or(self.common().unit());
        self.check_block(&ast.body, ret_ty);
    }

    pub fn check_block(&mut self, ast: &Block, expected: Ty<'ty>) -> Ty<'ty> {
        for stmt in &ast.stmts {
            self.check_stmt(stmt);
        }
        if let Some(tail) = &ast.tail {
            self.check_toplevel_expr(tail, expected)
        } else {
            self.common().unit()
        }
    }

    fn check_toplevel_expr(&mut self, ast: &Expr, expected: Ty<'ty>) -> Ty<'ty> {
        self.ty_vars.push(vec![]);
        let mut ret = self.ty_ctx.common().pending;

        let mut cnt = 0;
        while !ret.is_final() {
            if cnt == 10 {
                let diagnostic = Diagnostic::error("hit iteration limit", Span::dummy());
                self.errors.report_err(diagnostic);
                break;
            }

            ret = self.check_expr(ast, expected);
            cnt += 1;
        }

        // FIXME: check all inference vars are resolved
        self.ty_vars.pop();

        ret
    }

    fn check_stmt(&mut self, ast: &Stmt) {
        match &ast.node {
            StmtKind::Expr(expr) => {
                self.check_toplevel_expr(&*expr, self.common().infer);
            }
            StmtKind::Let(name, ty, value, _) => {
                let def_id = *self.name_tables.uses.get(&name.id).unwrap(); // FIXME
                let expected = ty.as_ref().map(|ty| self.lower_ty(ty)).unwrap_or(self.common().infer);
                let ty = self.check_toplevel_expr(&*value, expected);
                if let Ok(def_id) = def_id {
                    self.def_map.insert(def_id, Def::Var(ty)); // FIXME?
                }
            }
            StmtKind::Assign(lhs, rhs) => {
                let expected = self
                    .name_tables
                    .uses
                    .get(&lhs.id)
                    .and_then(|def_id| def_id.ok())
                    .and_then(|def_id| self.def_map.get(&def_id))
                    .and_then(|def| def.as_var())
                    .unwrap_or(self.common().infer);
                self.check_toplevel_expr(rhs, expected);
            }
        }
    }

    fn infer_expr(&mut self, ast: &Expr) -> Ty<'ty> {
        self.check_expr(ast, self.common().infer)
    }

    fn check_expr(&mut self, ast: &Expr, mut expected: Ty<'ty>) -> Ty<'ty> {
        if let Some(&ty) = self.type_map.get(&ast.id) {
            return ty;
        }

        let vars = self.ty_vars.last().expect("not in expression");
        if !vars.is_empty() {
            expected = self.ty_ctx.substitute_vars(expected, vars, Bound::Upper);
        };

        let mut actual = match &ast.node {
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
                    .map(|def_id| self.def_ty(def_id))
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
            ExprKind::Array(exprs) => {
                let expected = expected.array_elem().unwrap_or(self.common().infer);
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
            ExprKind::Tuple(exprs) => {
                let expected = expected.tuple_elems().unwrap_or(&[]);
                let infer = self.common().infer;
                let never = self.common().never;
                let expr_tys = exprs
                    .iter()
                    .enumerate()
                    .map(|(index, expr)| self.check_expr(expr, expected.get(index).copied().unwrap_or(infer)))
                    .collect_vec();
                self.ty_ctx.mk_tuple(&expr_tys)
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

        let vars = self.ty_vars.last().expect("not in expression");
        if !vars.is_empty() {
            actual = self.ty_ctx.substitute_vars(expected, vars, Bound::Lower);
        };

        if !expected.is_final() {
            return actual;
        }

        let ty = match self.ty_ctx.reconcile(actual, expected) {
            Ok(ty) => ty,
            Err(err) => {
                let diagnostic = Diagnostic::error(err.to_string(), ast.span);
                self.ty_ctx.mk_error(self.errors.report_err(diagnostic))
            }
        };

        self.type_map.insert(ast.id, ty);
        ty
    }

    fn lower_ty(&mut self, ast: &ast::Ty) -> Ty<'ty> {
        self.lower_ty_with_params(ast, &[])
    }

    fn lower_ty_with_params(&mut self, ast: &ast::Ty, ty_params: &[Ident]) -> Ty<'ty> {
        match &ast.node {
            ast::TyKind::Infer => self.common().infer,
            ast::TyKind::Var(ident) => {
                if let Some(index) = ty_params.iter().position(|p| p.sym == ident.sym) {
                    return self.ty_ctx.mk_param(ParamId(index as u32));
                }
                match self.sym_table.get_str(ident.sym) {
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
                }
            }
            ast::TyKind::Nullable(inner) => {
                let inner = self.lower_ty_with_params(inner, ty_params);
                self.ty_ctx.mk_nullable(inner)
            }
            ast::TyKind::Array(inner) => {
                let inner = self.lower_ty_with_params(inner, ty_params);
                self.ty_ctx.mk_array(inner)
            }
            ast::TyKind::Tuple(tys) => {
                let tys = tys
                    .into_iter()
                    .map(|ty| self.lower_ty_with_params(ty, ty_params))
                    .collect_vec();
                self.ty_ctx.mk_tuple(&tys)
            }
        }
    }

    fn common(&self) -> &CommonTypes<'ty> {
        self.ty_ctx.common()
    }

    fn def_ty(&mut self, def_id: DefId) -> Ty<'ty> {
        let def = self.def_map.get(&def_id).unwrap(); // FIXME: unwrap
        let mut vars = self.ty_vars.last_mut().expect("not in statement");
        match def {
            Def::Var(ty) => *ty,
            Def::Func(func) => {
                let vars = func
                    .ty_params
                    .iter()
                    .map(|_| Self::fresh_ty_var(self.ty_ctx, vars))
                    .collect_vec();
                let params = func
                    .params
                    .iter()
                    .map(|ty| self.ty_ctx.substitute_params(*ty, &vars))
                    .collect_vec();
                let ret = self.ty_ctx.substitute_params(func.ret, &vars);
                self.ty_ctx.mk_func(&params, ret)
            }
        }
    }

    fn fresh_ty_var(ctx: TyCtx<'_, 'ty>, vars: &mut Vec<TyBounds<'ty>>) -> Ty<'ty> {
        let id = VarId(vars.len() as u32);
        vars.push(ctx.new_bounds());
        ctx.mk_var(id)
    }
}

impl<'ty> Def<'ty> {
    pub fn as_var(&self) -> Option<Ty<'ty>> {
        match self {
            Self::Var(ty) => Some(*ty),
            _ => None,
        }
    }
}
