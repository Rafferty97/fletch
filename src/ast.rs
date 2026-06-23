use std::fmt::Display;
use std::hash::Hash;

use crate::ast::span::{Span, Spanned};
use crate::diagnostics::ErrGuaranteed;
use crate::interner::{Index, IndexedInterner};

pub mod sexpr;
pub mod span;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct NodeId(pub u32);

pub type Stmt = Spanned<StmtKind>;
pub type Expr = Spanned<ExprKind>;

#[derive(Clone, Debug)]
pub struct Program {
    pub main: Func,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: Ident,
    pub body: Block,
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Expr(Box<Expr>),
    Let(Ident, Box<Expr>, Mutability),
    Assign(Ident, Box<Expr>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutability {
    Mut,
    Not,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Var(Ident),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Grouped(Box<Expr>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lit {
    Null,
    Bool(bool),
    Int(Symbol),
    Float(Symbol),
    Str(Symbol),
    Err(ErrGuaranteed),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
}

#[derive(Clone, Copy, Debug)]
pub struct Ident {
    pub id: NodeId,
    pub sym: Symbol,
    pub span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Symbol(u32);

impl Index for Symbol {
    fn from_usize(index: usize) -> Self {
        Self(index.try_into().expect("too many symbols"))
    }

    fn into_usize(self) -> usize {
        self.0 as usize
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "+"),
            Self::Sub => write!(f, "-"),
        }
    }
}
