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
    todo!()
}
