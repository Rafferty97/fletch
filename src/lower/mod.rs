use crate::ast::{self, BinOp};
use crate::error::{Error, Result};
use crate::ir::{self, BinOpKind, ExprKind};
use crate::typecheck::{IntTy, Ty, TyCtx, TyKind, UIntTy, with_ty_ctx};
use crate::util::span::{Span, TextSize};

pub fn lower_program<'tcx>(ast: &ast::Program, ctx: TyCtx<'tcx>) -> Result<ir::Program<'tcx>> {
    let main = ast
        .items
        .iter()
        .find_map(|item| match &item.kind {
            ast::ItemKind::Func(func) if func.name.0 == "main" => Some(func),
            _ => None,
        })
        .ok_or_else(|| {
            let span = Span::at(TextSize::from(0), TextSize::from(0));
            Error::new_other("no main function", span)
        })?;

    let expr = main.body.tail.as_ref().ok_or_else(|| {
        let span = Span::at(TextSize::from(0), TextSize::from(0));
        Error::new_other("no tail expression", span)
    })?;

    let expr = lower_expr(expr, None, ctx)?;

    Ok(ir::Program { expr })
}

pub fn lower_expr<'tcx>(
    ast: &ast::Expr,
    expect: Option<Ty<'tcx>>,
    ctx: TyCtx<'tcx>,
) -> Result<ir::Expr<'tcx>> {
    Ok(match &ast.kind {
        ast::ExprKind::Lit(lit) => lower_lit(lit, ast.span, expect, ctx)?,
        ast::ExprKind::Binary(op, lhs, rhs, span) => {
            lower_binary(*op, &*lhs, &*rhs, *span, expect, ctx)?
        }
        _ => unimplemented!(),
    })
}

pub fn lower_lit<'tcx>(
    ast: &ast::Lit,
    span: Span,
    expect: Option<Ty<'tcx>>,
    ctx: TyCtx<'tcx>,
) -> Result<ir::Expr<'tcx>> {
    let (kind, ty) = match ast.kind {
        ast::LitKind::Null => (ir::Lit::Null, ctx.tys.null),
        ast::LitKind::Integer => match expect.map(|ty| ty.kind()) {
            Some(TyKind::Int(IntTy::I8)) => (ir::Lit::Int8(ast.raw.parse().unwrap()), ctx.tys.i8), // FIXME: unwrap
            _ => (ir::Lit::Int32(ast.raw.parse().unwrap()), ctx.tys.i32), // FIXME: unwrap
        },
        _ => unimplemented!(),
    };

    if let Some(expected) = expect {
        check_type(expected, ty, span)?;
    }

    Ok(ir::Expr { kind: ExprKind::Lit(kind), ty })
}

pub fn lower_binary<'tcx>(
    op: ast::BinOp,
    lhs: &ast::Expr,
    rhs: &ast::Expr,
    span: Span,
    expect: Option<Ty<'tcx>>,
    ctx: TyCtx<'tcx>,
) -> Result<ir::Expr<'tcx>> {
    let lhs_expect = expect.and_then(|ty| match BinOpCategory::from(op) {
        BinOpCategory::Math => Some(ty),
        BinOpCategory::Comparison => None,
    });
    let lhs = lower_expr(lhs, lhs_expect, ctx)?;

    let rhs = lower_expr(rhs, Some(lhs.ty), ctx)?;

    if lhs.ty != rhs.ty {
        Err(Error::new_binop(op, lhs.ty, rhs.ty, span))?;
    }

    let (op, ty) = match (op, lhs.ty.kind()) {
        (ast::BinOp::Add, TyKind::Int(IntTy::I32)) => (BinOpKind::Add, ctx.tys.i32),
        (ast::BinOp::Add, TyKind::UInt(UIntTy::U64)) => (BinOpKind::Add, ctx.tys.u64),
        (ast::BinOp::Sub, TyKind::Int(IntTy::I32)) => (BinOpKind::Sub, ctx.tys.i32),
        (ast::BinOp::Sub, TyKind::UInt(UIntTy::U64)) => (BinOpKind::Sub, ctx.tys.u64),
        (ast::BinOp::Mul, TyKind::Int(IntTy::I32)) => (BinOpKind::SMul, ctx.tys.i32),
        (ast::BinOp::Mul, TyKind::UInt(UIntTy::U64)) => (BinOpKind::UMul, ctx.tys.u64),
        (ast::BinOp::Div, TyKind::Int(IntTy::I32)) => (BinOpKind::SDiv, ctx.tys.i32),
        (ast::BinOp::Div, TyKind::UInt(UIntTy::U64)) => (BinOpKind::UDiv, ctx.tys.u64),
        _ => unimplemented!(),
    };

    if let Some(expected) = expect {
        check_type(expected, ty, span)?;
    }

    let [lhs, rhs] = [lhs, rhs].map(Box::new);
    Ok(ir::Expr { kind: ir::ExprKind::BinOp(ir::BinOp { op, lhs, rhs }), ty })
}

#[derive(Clone, Copy)]
enum BinOpCategory {
    /// Operations that take equal types and produce the same type
    Math,
    /// Operations that take equal types and produce a boolean
    Comparison,
}

impl From<BinOp> for BinOpCategory {
    fn from(op: BinOp) -> Self {
        match op {
            BinOp::Add => Self::Math,
            BinOp::Sub => Self::Math,
            BinOp::Mul => Self::Math,
            BinOp::Div => Self::Math,
        }
    }
}

pub fn check_type<'tcx>(expected: Ty<'tcx>, got: Ty<'tcx>, span: Span) -> Result<()> {
    if got == expected {
        Ok(())
    } else {
        Err(Error::new_type_mismatch(expected, got, span))
    }
}
