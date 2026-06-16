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
}

#[derive(Clone, Debug)]
pub enum Lit {
    Int(Symbol),
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
