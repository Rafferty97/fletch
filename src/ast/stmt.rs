use super::expr::Expr;
use super::ident::Ident;
use super::lit::Lit;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    LetDecl(LetDecl),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct LetDecl {
    pub ident: Ident,
    pub value: Box<Expr>,
}
