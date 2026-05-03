use crate::ast;
use crate::error::{Error, Result};
use crate::ir;
use crate::util::span::{Span, TextSize};

pub fn lower_program(ast: &ast::Program) -> Result<ir::Program> {
    let main = ast
        .items
        .iter()
        .find_map(|item| match &item.kind {
            ast::ItemKind::Func(func) if func.name.0 == "main" => Some(func),
            _ => None,
        })
        .ok_or_else(|| {
            let span = Span::at(TextSize::from(0), TextSize::from(0));
            Error::new("no main function", span)
        })?;

    let expr = main.body.tail.as_ref().ok_or_else(|| {
        let span = Span::at(TextSize::from(0), TextSize::from(0));
        Error::new("no tail expression", span)
    })?;

    let expr = lower_expr(expr)?;

    Ok(ir::Program { expr })
}

pub fn lower_expr(ast: &ast::Expr) -> Result<ir::Expr> {
    let kind = match &ast.kind {
        ast::ExprKind::Lit(lit) => ir::ExprKind::Lit(lower_lit(lit)?),
        ast::ExprKind::Binary(op, lhs, rhs, _) => lower_binary(*op, &*lhs, &*rhs)?,
        _ => unimplemented!(),
    };

    Ok(ir::Expr { kind })
}

pub fn lower_lit(ast: &ast::Lit) -> Result<ir::Lit> {
    Ok(match ast {
        ast::Lit::Null => ir::Lit::Null,
        ast::Lit::Bool(value) => ir::Lit::Bool(*value),
        ast::Lit::UInt(value) => ir::Lit::UInt64(*value),
        ast::Lit::Int(value) => ir::Lit::Int64(*value),
        ast::Lit::Float(value) => ir::Lit::Float64(*value),
        ast::Lit::Str(value) => ir::Lit::Str(value.clone()),
    })
}

pub fn lower_binary(op: ast::BinOp, lhs: &ast::Expr, rhs: &ast::Expr) -> Result<ir::ExprKind> {
    let lhs = lower_expr(lhs)?;
    let rhs = lower_expr(rhs)?;

    Ok(match op {
        ast::BinOp::Add => {
            ir::ExprKind::Call(ir::Call { func: ir::VarId(0), args: vec![lhs, rhs] })
        }
        _ => unimplemented!(),
    })
}
