use bumpalo::Bump;
use fnv::FnvHashMap;

use crate::ast::{Expr, ExprKind, Func, Lit, Stmt, StmtKind, Symbol};
use crate::compiler::{Compiler, CompilerError, Result};
use crate::types::Ty;
use crate::types::infer::TypeError;
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::{CommonTypes, TyInterners};

struct TyCheckCtx<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    locals: FnvHashMap<Symbol, Ty<'ty>>,
}

impl<'a, 'sym, 'ty> Compiler<'a, 'sym, 'ty> {
    pub(super) fn typech_func(&mut self, ast: &Func) -> Result<()> {
        let ty_interners = TyInterners::new(&self.arena);
        let ty_ctx = TyCtx::new(&self.arena, &ty_interners);
        let locals = FnvHashMap::default();
        let mut ctx = TyCheckCtx { ty_ctx, locals };

        for stmt in &ast.body.stmts {
            self.typech_stmt(&mut ctx, stmt)?;
        }

        Ok(())
    }

    fn typech_stmt(&mut self, ctx: &mut TyCheckCtx<'_, 'ty>, ast: &Stmt) -> Result<()> {
        match &ast.node {
            StmtKind::Expr(expr) => {
                self.typech_expr(ctx, &*expr, ctx.common().infer)?;
            }
            StmtKind::Let(name, value) => {
                let ty = self.typech_expr(ctx, &*value, ctx.common().infer)?;
                ctx.locals.insert(name.sym, ty);
            }
            StmtKind::Assign(lhs, rhs) => {
                let ExprKind::Var(lhs) = &lhs.node else {
                    Err(CompilerError::InvalidAssignment)?
                };
                let Some(&expected) = ctx.locals.get(&lhs.sym) else {
                    let name = self.sym_interner.get_str(lhs.sym).into();
                    Err(CompilerError::UndefinedName(name))?
                };
                self.typech_expr(ctx, rhs, expected)?;
            }
        }
        Ok(())
    }

    fn typech_expr(&mut self, ctx: &mut TyCheckCtx<'_, 'ty>, ast: &Expr, expected: Ty<'ty>) -> Result<Ty<'ty>> {
        let actual = match &ast.node {
            ExprKind::Lit(lit) => match lit {
                Lit::Null => ctx.common().opt_never,
                Lit::Bool(_) => ctx.common().bool,
                Lit::Int(_) => ctx.common().int32,     // FIXME
                Lit::Float(_) => ctx.common().float64, // FIXME
                Lit::Str(_) => ctx.common().str,
                Lit::Err(err) => ctx.ty_ctx.mk_error(*err),
            },
            ExprKind::Var(name) => match ctx.locals.get(&name.sym) {
                Some(ty) => *ty,
                None => {
                    let name = self.sym_interner.get_str(name.sym).into();
                    Err(CompilerError::UndefinedName(name))?
                }
            },
            ExprKind::Binary(op, lhs, rhs) => {
                let lhs = self.typech_expr(ctx, lhs, ctx.common().infer)?;
                let rhs = self.typech_expr(ctx, rhs, ctx.common().infer)?;
                if lhs == rhs {
                    lhs
                } else {
                    Err(CompilerError::TypeError(format!(
                        "no implementation of '{lhs}' {op} '{rhs}'"
                    )))?
                }
            }
            ExprKind::Call(func, args) => ctx.common().opt_never, // FIXME
            ExprKind::Grouped(expr) => self.typech_expr(ctx, expr, expected)?,
        };

        let ty = if expected.is_final() {
            ctx.ty_ctx
                .reconcile(actual, expected)
                .map_err(|err| CompilerError::TypeError(err.to_string()))?
        } else {
            actual
        };

        self.type_map.insert(ast.id, ty);

        Ok(ty)
    }
}

impl<'a, 'ty> TyCheckCtx<'a, 'ty> {
    fn common(&self) -> &CommonTypes<'ty> {
        self.ty_ctx.common()
    }
}
