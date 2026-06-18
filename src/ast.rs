use std::fmt::Display;

use crate::diagnostics::ErrGuaranteed;
use crate::interner::Index;
use crate::span::Spanned;

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

#[repr(transparent)]
pub struct SExpr<T>(pub T);

impl<T> SExpr<T> {
    pub fn new(val: &T) -> &SExpr<T> {
        /// SAFETY: `SExpr<T>` is guaranteed to have the same layout as `T`
        /// due to the `#[repr(transparent)]` attribute
        unsafe {
            &*(val as *const T as *const SExpr<T>)
        }
    }
}

impl Display for SExpr<Program> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", SExpr::new(&self.0.main))
    }
}

impl Display for SExpr<Func> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(func {} {})", SExpr(self.0.name.sym), SExpr::new(&self.0.body))
    }
}

impl Display for SExpr<Block> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(block")?;
        for stmt in &self.0.stmts {
            write!(f, " {}", SExpr::new(stmt))?;
        }
        match &self.0.tail {
            Some(tail) => write!(f, " {})", SExpr::new(tail)),
            None => write!(f, " none)"),
        }
    }
}

impl Display for SExpr<Stmt> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.node {
            StmtKind::Print(expr) => write!(f, "(print {})", SExpr::new(&**expr)),
        }
    }
}

impl Display for SExpr<Expr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.node {
            ExprKind::Lit(lit) => write!(f, "{}", SExpr::new(lit)),
            ExprKind::Binary(op, lhs, rhs) => {
                write!(f, "({} {} {})", SExpr(*op), SExpr::new(&**lhs), SExpr::new(&**rhs))
            }
        }
    }
}

impl Display for SExpr<BinOp> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self.0 {
            BinOp::Add => "+",
        };
        write!(f, "{}", op)
    }
}

impl Display for SExpr<Lit> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Lit::Null => write!(f, "null"),
            Lit::Bool(value) => write!(f, "{value}"),
            Lit::Int(sym) => write!(f, "(int {})", SExpr(sym)),
            Lit::Float(sym) => write!(f, "(float {})", SExpr(sym)),
            Lit::Str(sym) => write!(f, "(str {})", SExpr(sym)),
            Lit::Err(_) => write!(f, "err)"),
        }
    }
}

impl Display for SExpr<Symbol> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}", self.0.0)
    }
}
