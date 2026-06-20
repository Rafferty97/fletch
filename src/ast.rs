use std::fmt::Display;

use crate::diagnostics::ErrGuaranteed;
use crate::interner::{Index, IndexedInterner};
use crate::span::Spanned;

pub mod sexpr;

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

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Print(Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Var(Ident),
    Binary(BinOp, Box<Expr>, Box<Expr>),
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
}

#[derive(Clone, Copy, Debug)]
pub struct Ident {
    pub sym: Symbol,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Symbol(u32);

impl Index for Symbol {
    fn from_usize(index: usize) -> Self {
        Self(index.try_into().expect("too many symbols"))
    }

    fn into_usize(self) -> usize {
        self.0 as usize
    }
}
