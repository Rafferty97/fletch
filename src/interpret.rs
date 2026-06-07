use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};

use thiserror::Error;

use crate::ast::{BinOp, Block, Expr, ExprKind, StmtKind};

#[derive(Clone, Debug)]
pub enum Value {
    Int32(i32),
    Float64(f64),
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("invalid integer literal: {0}")]
    InvalidIntLiteral(ParseIntError),
    #[error("invalid float literal: {0}")]
    InvalidFloatLiteral(ParseFloatError),
    #[error("internal error: {0}")]
    Internal(Box<str>),
}

pub fn eval(block: &Block) -> Result<(), RuntimeError> {
    for stmt in &block.stmts {
        match &stmt.kind {
            StmtKind::Let(_, _, expr) => {
                eval_expr(&*expr)?;
            }
            StmtKind::Expr(expr) => {
                eval_expr(&*expr)?;
            }
        }
    }
    Ok(())
}

pub fn eval_expr(expr: &Expr) -> Result<Value, RuntimeError> {
    match &expr.kind {
        ExprKind::Var(_) => todo!(),
        ExprKind::IntLiteral(lit) => Ok(Value::Int32(lit.parse()?)),
        ExprKind::FloatLiteral(lit) => Ok(Value::Float64(lit.parse()?)),
        ExprKind::Binary(op, lhs, rhs) => {
            let lhs = eval_expr(lhs)?;
            let rhs = eval_expr(rhs)?;
            Ok(match (op, lhs, rhs) {
                (BinOp::Add, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs + rhs),
                (BinOp::Add, Value::Float64(lhs), Value::Float64(rhs)) => Value::Float64(lhs + rhs),
                (BinOp::Sub, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs - rhs),
                (BinOp::Sub, Value::Float64(lhs), Value::Float64(rhs)) => Value::Float64(lhs - rhs),
                (BinOp::Mul, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs * rhs),
                (BinOp::Mul, Value::Float64(lhs), Value::Float64(rhs)) => Value::Float64(lhs * rhs),
                (BinOp::Div, Value::Int32(lhs), Value::Int32(rhs)) => Value::Int32(lhs / rhs),
                (BinOp::Div, Value::Float64(lhs), Value::Float64(rhs)) => Value::Float64(lhs / rhs),
                _ => Err(RuntimeError::Internal("type mismatch".into()))?,
            })
        }
    }
}

impl From<ParseIntError> for RuntimeError {
    fn from(err: ParseIntError) -> Self {
        Self::InvalidIntLiteral(err)
    }
}

impl From<ParseFloatError> for RuntimeError {
    fn from(err: ParseFloatError) -> Self {
        Self::InvalidFloatLiteral(err)
    }
}
