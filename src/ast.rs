use std::fmt::Display;
use std::hash::Hash;

use crate::ast::span::Spanned;
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

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Expr(Box<Expr>),
    Let(Ident, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Var(Ident),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
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
