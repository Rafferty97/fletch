use super::lit::Lit;
use crate::parser::Span;

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Var(VarId),
    Call(Call),
}

#[derive(Clone, Copy, Debug)]
pub struct VarId(pub u32);

#[derive(Clone, Debug)]
pub struct Call {
    pub func: VarId, // ??
    pub args: Vec<Expr>,
}
