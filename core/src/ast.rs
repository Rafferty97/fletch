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
pub type Ty = Spanned<TyKind>;

#[derive(Clone, Debug)]
pub struct Program {
    pub funcs: Vec<Func>,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: Ident,
    pub params: Vec<(Ident, Ty)>,
    pub ret: Option<Ty>,
    pub body: Block,
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Expr(Box<Expr>),
    Let(Ident, Option<Box<Ty>>, Box<Expr>, Mutability),
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
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>),
    Grouped(Box<Expr>),
    Array(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        r#else: Option<Box<Expr>>,
    },
    Block(Block),
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
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Clone, Debug)]
pub enum TyKind {
    Infer,
    Var(Ident),
    Nullable(Box<Ty>),
    Array(Box<Ty>),
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
            Self::Mul => write!(f, "*"),
            Self::Div => write!(f, "/"),
            Self::Eq => write!(f, "=="),
            Self::NotEq => write!(f, "!="),
            Self::Lt => write!(f, "<"),
            Self::LtEq => write!(f, "<="),
            Self::Gt => write!(f, ">"),
            Self::GtEq => write!(f, ">="),
        }
    }
}
