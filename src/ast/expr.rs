use std::fmt::Display;

use super::ident::Ident;
use super::lit::Lit;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Expr<A> {
    pub kind: ExprKind<A>,
    pub span: Span,
    pub ann: A,
}

#[derive(Clone, Debug)]
pub enum ExprKind<A> {
    Lit(Lit),
    Ident(Ident),
    Binary(BinOp, Box<Expr<A>>, Box<Expr<A>>, Span),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
        }
    }
}
