pub use ty::*;

use crate::arena::Ctx;
use crate::ast::{self, BinOp, BinOpKind, Expr, ExprKind, Ident, LetStmt, Lit};
use crate::lexer::LitKind;
use crate::types::typechecker::TypecheckCtx;

mod fold;
mod ty;
mod typechecker;

pub fn check_expr<'cx>(ctx: Ctx<'cx>, expr: &Expr) -> Result<Ty<'cx>, String> {
    let mut func_ctx = FunctionCtx::new(ctx);
    let ty = func_ctx.check_expr(expr)?;
    func_ctx.resolve(ty)
}

pub struct FunctionCtx<'cx> {
    ctx: Ctx<'cx>,
    pub tc: TypecheckCtx<'cx>,
}

impl<'cx> FunctionCtx<'cx> {
    pub fn new(ctx: Ctx<'cx>) -> Self {
        let tc = TypecheckCtx::new(ctx);
        Self { ctx, tc }
    }

    pub fn resolve(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, String> {
        self.tc.resolve(ty)
    }

    pub fn resolve_partial(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, String> {
        self.tc.resolve_partial(ty)
    }

    pub fn check_let(&mut self, stmt: &LetStmt) -> Result<Ty<'cx>, String> {
        let expected = stmt.ty.as_ref().map(|ty| self.check_ty(ty)).transpose()?;
        let actual = self.check_expr(&stmt.expr)?;

        if let Some(expected) = expected {
            self.tc.coerce(actual, expected)?;
        }

        Ok(expected.unwrap_or(actual))
    }

    pub fn check_ty(&mut self, ty: &ast::Ty) -> Result<Ty<'cx>, String> {
        Ok(match &ty.kind {
            ast::TyKind::Var(ident) => match self.ctx.get_str(ident.sym) {
                "bool" => Ty(self.ctx.intern_ty_kind(TyKind::Bool)),
                "i8" => Ty(self.ctx.intern_ty_kind(TyKind::Int(IntTy::Int8))),
                "i16" => Ty(self.ctx.intern_ty_kind(TyKind::Int(IntTy::Int16))),
                "i32" => Ty(self.ctx.intern_ty_kind(TyKind::Int(IntTy::Int32))),
                "i64" => Ty(self.ctx.intern_ty_kind(TyKind::Int(IntTy::Int64))),
                "u8" => Ty(self.ctx.intern_ty_kind(TyKind::UInt(UIntTy::UInt8))),
                "u16" => Ty(self.ctx.intern_ty_kind(TyKind::UInt(UIntTy::UInt16))),
                "u32" => Ty(self.ctx.intern_ty_kind(TyKind::UInt(UIntTy::UInt32))),
                "u64" => Ty(self.ctx.intern_ty_kind(TyKind::UInt(UIntTy::UInt64))),
                "f32" => Ty(self.ctx.intern_ty_kind(TyKind::Float(FloatTy::Float32))),
                "f64" => Ty(self.ctx.intern_ty_kind(TyKind::Float(FloatTy::Float64))),
                "str" => Ty(self.ctx.intern_ty_kind(TyKind::Str)),
                str => Err(format!("unknown type: {str}"))?,
            },
        })
    }

    pub fn check_expr(&mut self, expr: &Expr) -> Result<Ty<'cx>, String> {
        let ty = match &expr.kind {
            ExprKind::Lit(lit) => self.check_lit(*lit),
            ExprKind::Var(ident) => self.check_var(*ident),
            ExprKind::Binary(op, lhs, rhs) => self.check_binop(op, lhs, rhs),
            ExprKind::Paren(inner) => self.check_expr(inner),
        }?;
        self.ctx.set_node_ty(expr.id, ty);
        Ok(ty)
    }

    fn check_lit(&mut self, lit: Lit) -> Result<Ty<'cx>, String> {
        match lit.kind {
            LitKind::Bool => Ok(Ty(self.ctx.intern_ty_kind(TyKind::Bool))),
            LitKind::Integer => Ok(self.tc.new_int_var()),
            LitKind::Float => Ok(self.tc.new_float_var()),
            LitKind::Str => Ok(Ty(self.ctx.intern_ty_kind(TyKind::Str))),
        }
    }

    fn check_var(&mut self, ident: Ident) -> Result<Ty<'cx>, String> {
        self.tc.get_variable(ident.sym)
    }

    fn check_binop(&mut self, op: &BinOp, lhs: &Expr, rhs: &Expr) -> Result<Ty<'cx>, String> {
        use BinOpKind::*;

        let lhs_ty = self.check_expr(lhs)?;
        let rhs_ty = self.check_expr(rhs)?;

        match op.node {
            Add | Sub | Mul | Div => {
                self.tc.unify(lhs_ty, rhs_ty).map_err(|_| {
                    let lhs = self.tc.resolve_partial(lhs_ty).unwrap();
                    let rhs = self.tc.resolve_partial(rhs_ty).unwrap();
                    format!("no implementation for {lhs} {} {rhs}", op.node)
                })?;
                let ty = self.tc.resolve_partial(lhs_ty)?;
                match ty.kind() {
                    TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_) => Ok(ty),
                    TyKind::NumVar(_) => Ok(ty),
                    TyKind::TyVar(_) => {
                        let ret = self.tc.new_num_var();
                        self.tc.unify(ty, ret)?;
                        Ok(ret)
                    }
                    _ => {
                        let lhs = self.tc.resolve_partial(lhs_ty).unwrap();
                        let rhs = self.tc.resolve_partial(rhs_ty).unwrap();
                        Err(format!("no implementation for {lhs} {} {rhs}", op.node))
                    }
                }
            }
        }
    }
}
