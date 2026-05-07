pub use binop::*;
pub use expr::*;
pub use lit::*;

mod binop;
mod expr;
mod lit;

#[derive(Clone, Debug)]
pub struct Program<'tcx> {
    pub expr: Expr<'tcx>,
}
