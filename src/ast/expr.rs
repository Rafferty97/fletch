use super::ident::Ident;
use super::lit::Lit;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Ident(Ident),
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}
