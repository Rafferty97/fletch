use logos::Span;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(pub u32);

#[derive(Clone, Debug)]
pub struct Node<T> {
    pub id: NodeId,
    pub span: Span,
    pub kind: T,
}

pub type Stmt = Node<StmtKind>;
pub type Expr = Node<ExprKind>;
pub type Ty = Node<TyKind>;

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let(Box<str>, Option<Ty>, Box<Expr>),
    Expr(Box<Expr>),
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

#[derive(Clone, Debug)]
pub enum TyKind {
    // todo
}
