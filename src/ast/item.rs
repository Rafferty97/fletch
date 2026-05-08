use super::expr::Expr;
use super::ident::Ident;
use super::lit::Lit;
use super::stmt::Stmt;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Item<A> {
    pub kind: ItemKind<A>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ItemKind<A> {
    Func(Func<A>),
}

#[derive(Clone, Debug)]
pub struct Func<A> {
    pub name: Ident,
    pub args: Vec<Ident>,
    pub body: Block<A>,
}

#[derive(Clone, Debug)]
pub struct Block<A> {
    pub stmts: Vec<Stmt<A>>,
    pub tail: Option<Expr<A>>,
    pub span: Span,
}
