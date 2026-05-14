pub use ty::*;

use crate::arena::Ctx;
use crate::ast::{BinOp, BinOpKind, Expr, ExprKind, Ident, Lit};
use crate::lexer::LitKind;
use crate::types::typechecker::TypecheckCtx;

mod fold;
mod ty;
mod typechecker;

pub fn check_expr<'cx>(ctx: Ctx<'cx>, expr: &Expr) -> Result<Ty<'cx>, String> {
    let mut func_ctx = FunctionCtx::new(ctx);
    let ty = func_ctx.check_expr(expr)?;
    func_ctx.tc.resolve(ty)
}

struct FunctionCtx<'cx> {
    ctx: Ctx<'cx>,
    tc: TypecheckCtx<'cx>,
}

impl<'cx> FunctionCtx<'cx> {
    fn new(ctx: Ctx<'cx>) -> Self {
        let tc = TypecheckCtx::new(ctx);
        Self { ctx, tc }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Ty<'cx>, String> {
        match &expr.kind {
            ExprKind::Lit(lit) => self.check_lit(*lit),
            ExprKind::Var(ident) => self.check_var(*ident),
            ExprKind::Binary(op, lhs, rhs) => self.check_binop(op, lhs, rhs),
            ExprKind::Paren(inner) => {
                let ty = self.check_expr(inner)?;
                self.ctx.set_node_ty(expr.id, ty);
                Ok(ty)
            }
        }
    }

    fn check_lit(&mut self, lit: Lit) -> Result<Ty<'cx>, String> {
        match lit.kind {
            LitKind::Bool => Ok(Ty(self.ctx.intern_ty_kind(TyKind::Bool))),
            LitKind::Integer => Ok(self.tc.new_int_var()),
            LitKind::Float => Ok(self.tc.new_float_var()),
            LitKind::Str => Ok(Ty(self.ctx.intern_ty_kind(TyKind::Str))),
        }
    }

    fn check_var(&mut self, ident: Ident) -> Result<Ty<'cx>, String> {
        self.tc.get_variable(ident.sym)
    }

    fn check_binop(&mut self, op: &BinOp, lhs: &Expr, rhs: &Expr) -> Result<Ty<'cx>, String> {
        use BinOpKind::*;

        let lhs_ty = self.check_expr(lhs)?;
        let rhs_ty = self.check_expr(rhs)?;

        match op.node {
            Add | Sub | Mul | Div => {
                self.tc.unify(lhs_ty, rhs_ty).map_err(|_| {
                    let lhs = lhs_ty; //self.tc.resolve(lhs_ty).unwrap_or(lhs_ty);
                    let rhs = rhs_ty; //self.tc.resolve(rhs_ty).unwrap_or(rhs_ty);
                    format!("no implementation for {lhs} {} {rhs}", op.node)
                })?;
                let ty = self.tc.resolve_partial(lhs_ty)?;
                match ty.kind() {
                    TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_) => Ok(ty),
                    TyKind::IntVar(_) | TyKind::FloatVar(_) => Ok(ty),
                    TyKind::TyVar(_) => Err(format!("types must be known at this point")),
                    _ => {
                        let lhs = lhs_ty; //self.tc.resolve(lhs_ty).unwrap_or(lhs_ty);
                        let rhs = rhs_ty; //self.tc.resolve(rhs_ty).unwrap_or(rhs_ty);
                        Err(format!("no implementation for {lhs} {} {rhs}", op.node))
                    }
                }
            }
        }
    }
}
