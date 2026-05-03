pub use expr::*;
pub use lit::*;

mod expr;
mod lit;

#[derive(Clone, Debug)]
pub struct Program {
    pub expr: Expr,
}
