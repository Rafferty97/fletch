use super::expr::Expr;
use super::ident::Ident;
use super::lit::Lit;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Stmt<A> {
    pub kind: StmtKind<A>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind<A> {
    LetDecl(LetDecl<A>),
    Expr(Box<Expr<A>>),
}

#[derive(Clone, Debug)]
pub struct LetDecl<A> {
    pub ident: Ident,
    pub value: Box<Expr<A>>,
}
