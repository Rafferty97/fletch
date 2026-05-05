pub use expr::*;
pub use lit::*;

mod expr;
pub mod intrinsics;
mod lit;

#[derive(Clone, Debug)]
pub struct Program<'tcx> {
    pub expr: Expr<'tcx>,
}
