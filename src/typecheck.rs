use std::marker::PhantomData;

use bumpalo::Bump;
use fnv::FnvHashMap;
use thiserror::Error;

use crate::ast::span::Span;
use crate::ast::{Expr, ExprKind, Func, Lit, NodeId, Stmt, StmtKind, Symbol};
use crate::interner::IndexTable;
use crate::types::Ty;
use crate::types::infer::TypeError;
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::{CommonTypes, TyInterners};

pub struct TypeChecker<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    type_map: FnvHashMap<NodeId, Ty<'ty>>,
    locals: FnvHashMap<Symbol, Ty<'ty>>,
    sym_table: &'a IndexTable<'a, Symbol, str>,
}

pub type Result<'ty, T> = std::result::Result<T, TypeError<'ty>>;

impl<'a, 'ty> TypeChecker<'a, 'ty> {
    pub fn new(ty_ctx: TyCtx<'a, 'ty>, sym_table: &'a IndexTable<'a, Symbol, str>) -> Self {
        Self {
            ty_ctx,
            type_map: FnvHashMap::default(),
            locals: FnvHashMap::default(),
            sym_table,
        }
    }

    pub fn check_func(&mut self, ast: &Func) -> Result<'ty, ()> {
        self.locals.clear();
        for stmt in &ast.body.stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, ast: &Stmt) -> Result<'ty, ()> {
        match &ast.node {
            StmtKind::Expr(expr) => {
                self.check_expr(&*expr, self.common().infer)?;
            }
            StmtKind::Let(name, value) => {
                let ty = self.check_expr(&*value, self.common().infer)?;
                self.locals.insert(name.sym, ty);
            }
            StmtKind::Assign(lhs, rhs) => {
                let Some(&expected) = self.locals.get(&lhs.sym) else {
                    // let name = self.sym_interner.get_str(lhs.sym).into();
                    // Err(CompilerError::UndefinedName(name))?
                    todo!()
                };
                self.check_expr(rhs, expected)?;
            }
        }
        Ok(())
    }

    fn check_expr(&mut self, ast: &Expr, expected: Ty<'ty>) -> Result<'ty, Ty<'ty>> {
        let actual = match &ast.node {
            ExprKind::Lit(lit) => match lit {
                Lit::Null => self.common().opt_never,
                Lit::Bool(_) => self.common().bool,
                Lit::Int(_) => self.common().int32,     // FIXME
                Lit::Float(_) => self.common().float64, // FIXME
                Lit::Str(_) => self.common().str,
                Lit::Err(err) => self.ty_ctx.mk_error(*err),
            },
            ExprKind::Var(name) => match self.locals.get(&name.sym) {
                Some(ty) => *ty,
                None => {
                    // let name = self.sym_table.get_str(name.sym).into();
                    // Err(CompilerError::UndefinedName(name))?
                    todo!()
                }
            },
            ExprKind::Binary(op, lhs, rhs) => {
                println!("{op:?}, {lhs:?}, {rhs:?}");
                let lhs = self.check_expr(lhs, self.common().infer)?;
                let rhs = self.check_expr(rhs, self.common().infer)?;
                if lhs == rhs {
                    lhs
                } else {
                    // Err(CompilerError::TypeError(format!(
                    //     "no implementation of '{lhs}' {op} '{rhs}'"
                    // )))?
                    todo!()
                }
            }
            ExprKind::Call(func, args) => {
                for arg in args {
                    self.check_expr(arg, self.common().infer)?;
                }
                self.common().opt_never // FIXME
            }
            ExprKind::Grouped(expr) => self.check_expr(expr, expected)?,
        };

        let ty = if expected.is_final() {
            self.ty_ctx.reconcile(actual, expected)?
        } else {
            actual
        };

        self.type_map.insert(ast.id, ty);

        Ok(ty)
    }

    fn common(&self) -> &CommonTypes<'ty> {
        self.ty_ctx.common()
    }
}
