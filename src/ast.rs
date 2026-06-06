use logos::Span;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(pub u32);

#[derive(Clone, Debug)]
pub struct Expr {
    pub id: NodeId,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Var(Box<str>),
    IntLiteral(Box<str>),
    FloatLiteral(Box<str>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}
