use std::fmt::Display;

use crate::arena::Symbol;
use crate::lexer::LitKind;
use crate::span::{Span, Spanned};

pub mod print;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub u32);

#[derive(Clone, Debug)]
pub struct Stmt {
    pub id: NodeId,
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let(LetStmt),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<Ty>,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Var(Ident),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Paren(Box<Expr>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Lit {
    pub kind: LitKind,
    pub sym: Symbol,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ident {
    pub sym: Symbol,
    pub span: Span,
}

pub type BinOp = Spanned<BinOpKind>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
}

impl Display for BinOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TyKind {
    Var(Ident),
}
