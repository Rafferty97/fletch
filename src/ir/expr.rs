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
pub struct VarId(u32);

#[derive(Clone, Debug)]
pub struct Call {
    func: VarId, // ??
    args: Vec<Expr>,
}
