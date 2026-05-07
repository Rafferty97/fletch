use super::binop::BinOp;
use super::lit::Lit;
use crate::parser::Span;
use crate::typecheck::Ty;

#[derive(Clone, Debug)]
pub struct Expr<'tcx> {
    pub kind: ExprKind<'tcx>,
    pub ty: Ty<'tcx>,
}

#[derive(Clone, Debug)]
pub enum ExprKind<'tcx> {
    Lit(Lit),
    Var(VarId),
    BinOp(BinOp<'tcx>),
    Call(Call<'tcx>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VarId(pub u32);

#[derive(Clone, Debug)]
pub struct Call<'tcx> {
    pub func: VarId, // ??
    pub args: Vec<Expr<'tcx>>,
}
