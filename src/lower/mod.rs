use crate::ast;
use crate::error::{Error, Result};
use crate::ir::{self, ExprKind, intrinsics};
use crate::typecheck::{
    IntTy, Ty, TyCtx, TyKind, UIntTy, check_type, expected_lhs_for_binop, expected_rhs_for_binop,
    with_ty_ctx,
};
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
    let lhs_expect = expect.and_then(|ty| expected_lhs_for_binop(op, ty));
    let lhs = lower_expr(lhs, lhs_expect, ctx)?;

    let rhs_expect = expected_rhs_for_binop(op, lhs.ty);
    let rhs = lower_expr(rhs, rhs_expect, ctx)?;

    if lhs.ty != rhs.ty {
        Err(Error::new_binop(op, lhs.ty, rhs.ty, span))?;
    }

    let (func, ty) = match (op, lhs.ty.kind()) {
        (ast::BinOp::Add, TyKind::Int(IntTy::I32)) => (intrinsics::ADD_I32, ctx.tys.i32),
        (ast::BinOp::Add, TyKind::UInt(UIntTy::U64)) => (intrinsics::ADD_U64, ctx.tys.u64),
        (ast::BinOp::Sub, TyKind::Int(IntTy::I32)) => (intrinsics::SUB_I32, ctx.tys.i32),
        (ast::BinOp::Sub, TyKind::UInt(UIntTy::U64)) => (intrinsics::SUB_U64, ctx.tys.u64),
        (ast::BinOp::Mul, TyKind::Int(IntTy::I32)) => (intrinsics::MUL_I32, ctx.tys.i32),
        (ast::BinOp::Mul, TyKind::UInt(UIntTy::U64)) => (intrinsics::MUL_U64, ctx.tys.u64),
        (ast::BinOp::Div, TyKind::Int(IntTy::I32)) => (intrinsics::DIV_I32, ctx.tys.i32),
        (ast::BinOp::Div, TyKind::UInt(UIntTy::U64)) => (intrinsics::DIV_U64, ctx.tys.u64),
        _ => unimplemented!(),
    };

    if let Some(expected) = expect {
        check_type(expected, ty, span)?;
    }

    Ok(ir::Expr {
        kind: ir::ExprKind::Call(ir::Call { func, args: vec![lhs, rhs] }),
        ty,
    })
}
