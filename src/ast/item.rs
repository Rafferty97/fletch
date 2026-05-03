use super::expr::Expr;
use super::ident::Ident;
use super::lit::Lit;
use super::stmt::Stmt;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ItemKind {
    Func(Func),
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: Ident,
    pub args: Vec<Ident>,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
    pub span: Span,
}
