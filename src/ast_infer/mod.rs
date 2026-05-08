use crate::ast::{Expr, ExprKind, Lit, LitKind, Program};
use crate::error::Result;
use crate::typecheck::{InferConstraint, Ty, TyCtx};

pub fn infer_program<'tcx>(ast: Program<()>, tcx: TyCtx<'tcx>) -> Result<Program<Ty<'tcx>>> {
    todo!()
}

pub fn infer_expr<'tcx>(ast: Expr<()>, tcx: TyCtx<'tcx>) -> Result<Expr<Ty<'tcx>>> {
    Ok(match ast.kind {
        ExprKind::Lit(lit) => {
            let ann = infer_lit(&lit, tcx)?;
            let kind = ExprKind::Lit(lit);
            Expr { kind, span: ast.span, ann }
        }
        ExprKind::Binary(op, lhs, rhs, span) => {
            let lhs = infer_expr(*lhs, tcx)?;
            let rhs = infer_expr(*rhs, tcx)?;
            let ann = tcx.tys.bool; // fixme
            let kind = ExprKind::Binary(op, lhs.into(), rhs.into(), span);
            Expr { kind, span: ast.span, ann }
        }
        _ => unimplemented!(),
    })
}

pub fn infer_lit<'tcx>(ast: &Lit, tcx: TyCtx<'tcx>) -> Result<Ty<'tcx>> {
    Ok(match ast.kind {
        LitKind::Null => tcx.tys.null,
        LitKind::Bool => tcx.tys.bool,
        LitKind::Integer => tcx.fresh_infer_var(InferConstraint::Integer),
        LitKind::Float => tcx.fresh_infer_var(InferConstraint::Float),
        LitKind::Str => tcx.tys.str,
    })
}
