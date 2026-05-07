use super::expr::Expr;

#[derive(Clone, Debug)]
pub struct BinOp<'tcx> {
    pub op: BinOpKind,
    pub lhs: Box<Expr<'tcx>>,
    pub rhs: Box<Expr<'tcx>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
}
