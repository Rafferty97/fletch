use std::collections::HashMap;
use std::hash::Hash;

use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::types::ty_ctx::Variance;

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

pub struct InferCtx<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    nodes: HashMap<u32, Ty<'ty>>,
    diagnostics: &'a dyn DiagnosticReporter,
}

impl<'a, 'ty> InferCtx<'a, 'ty> {
    pub fn infer<N: ExprNode<'ty>>(&mut self, node: &N, expected: Ty<'ty>) -> Ty<'ty> {
        if let Some(ty) = self.nodes.get(&node.id()) {
            return *ty;
        }

        let ty = match node.infer(self, expected) {
            Ok(ty) => ty,
            Err(err) => {
                let err = self.diagnostics.report(err.into());
                self.ty_ctx.mk_err(err)
            }
        };

        if self.ty_ctx.is_final(ty) {
            self.nodes.insert(node.id(), ty);
        }

        ty
    }
}

pub trait ExprNode<'ty> {
    type Error: Into<Diagnostic>;

    fn id(&self) -> u32;

    fn infer<'a>(&self, ctx: &mut InferCtx<'a, 'ty>, expected: Ty<'ty>) -> Result<Ty<'ty>, Self::Error>;
}

/// Represents the infered bounds of a type parameter
#[derive(Clone, Copy, Debug)]
pub struct TyBounds<'ty> {
    lower: Ty<'ty>,
    upper: Ty<'ty>,
}

impl<'a, 'ty> TyCtx<'a, 'ty> {
    /// Finds the greatest lower bound of two types
    pub fn meet(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        use TyKind::*;

        match (lhs.kind(), rhs.kind()) {
            // Equality
            _ if lhs == rhs => lhs,

            // Sentinal values
            (_, Error(err)) | (Error(err), _) => self.mk_err(err),
            (_, Infer) | (Infer, _) => self.common().infer,
            (_, Pending) | (Pending, _) => self.common().pending,

            // Numeric types
            (Int(lhs), Int(rhs)) => self.mk_int(lhs.min(rhs)),
            (UInt(lhs), UInt(rhs)) => self.mk_uint(lhs.min(rhs)),
            (Float(lhs), Float(rhs)) => self.mk_float(lhs.min(rhs)),

            // Nullable types
            (Nullable(lhs), Nullable(rhs)) => self.mk_nullable(self.meet(lhs, rhs)),
            (Nullable(lhs), _) => self.meet(lhs, rhs),
            (_, Nullable(rhs)) => self.meet(lhs, rhs),

            // Arrays
            (Array(lhs), Array(rhs)) => self.mk_array(self.meet(lhs, rhs)),

            // Tuples
            (Tuple(lhs), Tuple(rhs)) if lhs.len() == rhs.len() => {
                let tys: Vec<_> = lhs
                    .iter()
                    .zip(rhs.iter())
                    .map(|(&lhs, &rhs)| self.meet(lhs, rhs))
                    .collect();
                self.mk_tuple(&tys)
            }

            // Functions
            (Func(lhs), Func(rhs)) if lhs.params.len() == rhs.params.len() => {
                let params: Vec<_> = lhs
                    .params
                    .iter()
                    .zip(rhs.params.iter())
                    .map(|(&lhs, &rhs)| self.meet(lhs, rhs))
                    .collect();
                self.mk_func(&params, self.meet(lhs.ret, rhs.ret))
            }

            // No common type
            _ => self.common().never,
        }
    }

    /// Finds the least upper bound of two types, if one exists
    pub fn join(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        use TyKind::*;

        match (lhs.kind(), rhs.kind()) {
            // Equality
            _ if lhs == rhs => lhs,

            // Sentinal values
            (_, Error(err)) | (Error(err), _) => self.mk_err(err),
            (_, Infer) | (Infer, _) => self.common().infer,
            (_, Pending) | (Pending, _) => self.common().pending,

            // Numeric types
            (Int(lhs), Int(rhs)) => self.mk_int(lhs.max(rhs)),
            (UInt(lhs), UInt(rhs)) => self.mk_uint(lhs.max(rhs)),
            (Float(lhs), Float(rhs)) => self.mk_float(lhs.max(rhs)),

            // Nullable types
            (Nullable(lhs), Nullable(rhs)) => self.mk_nullable(self.join(lhs, rhs)),
            (Nullable(lhs), _) => self.mk_nullable(self.join(lhs, rhs)),
            (_, Nullable(rhs)) => self.mk_nullable(self.join(lhs, rhs)),

            // Arrays
            (Array(lhs), Array(rhs)) => self.mk_array(self.join(lhs, rhs)),

            // Tuples
            (Tuple(lhs), Tuple(rhs)) if lhs.len() == rhs.len() => {
                let tys: Vec<_> = lhs
                    .iter()
                    .zip(rhs.iter())
                    .map(|(&lhs, &rhs)| self.join(lhs, rhs))
                    .collect();
                self.mk_tuple(&tys)
            }

            // Functions
            (Func(lhs), Func(rhs)) if lhs.params.len() == rhs.params.len() => {
                let params: Vec<_> = lhs
                    .params
                    .iter()
                    .zip(rhs.params.iter())
                    .map(|(&lhs, &rhs)| self.join(lhs, rhs))
                    .collect();
                self.mk_func(&params, self.join(lhs.ret, rhs.ret))
            }

            // No common type
            _ => self.common().any,
        }
    }

    /// Substitutes type parameters with concrete types
    pub fn substitute(self, ty: Ty<'ty>, params: &[Ty<'ty>]) -> Ty<'ty> {
        self.transform(ty, |ty| match ty.kind() {
            TyKind::Param(id) => params[id.0 as usize],
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types,
    /// using the upper bound in covariant positions and
    /// the lower bound in contravariant positions
    pub fn substitute_bounds(self, ty: Ty<'ty>, params: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(id) => match var {
                Variance::Co => params[id.0 as usize].upper,
                Variance::Contra => params[id.0 as usize].lower,
            },
            _ => ty,
        })
    }

    pub fn reconcile(self, lower: Ty<'ty>, upper: Ty<'ty>) -> Result<Ty<'ty>, TypeError> {
        use TyKind::*;

        // FIXME: prefer `lower` or `upper`?

        match (lower.kind(), upper.kind()) {
            // Equality
            _ if lower == upper => Ok(lower),

            // Sentinal values
            (Error(e), _) | (_, Error(e)) => Ok(self.mk_err(e)),
            (Pending, _) | (_, Pending) => unreachable!(),

            // Type inference
            (Infer, _) => {
                if self.has_infer(upper) {
                    Err(TypeError::Ambiguous)
                } else {
                    Ok(upper)
                }
            }
            (_, Infer) => {
                if self.has_infer(lower) {
                    Err(TypeError::Ambiguous)
                } else {
                    Ok(lower)
                }
            }

            // Numeric types
            // (Int(lhs), Int(rhs)) => self.mk_int(lhs.max(rhs)),
            // (UInt(lhs), UInt(rhs)) => self.mk_uint(lhs.max(rhs)),
            // (Float(lhs), Float(rhs)) => self.mk_float(lhs.max(rhs)),

            // Nullable types
            (Nullable(lower), Nullable(upper)) => Ok(self.mk_nullable(self.reconcile(lower, upper)?)),
            (_, Nullable(upper)) => Ok(self.mk_nullable(self.reconcile(lower, upper)?)),

            // Type mismatch
            _ => Err(TypeError::Mismatch),
        }
    }

    /// Returns `true` if no constituents of the type are pending
    pub fn is_final(self, ty: Ty<'ty>) -> bool {
        ty.fold(true, |acc, ty| acc && ty.kind() != TyKind::Pending)
    }

    /// Returns `true` if any constituents of the type are `Infer`
    pub fn has_infer(self, ty: Ty<'ty>) -> bool {
        ty.fold(false, |acc, ty| acc || ty.kind() == TyKind::Infer)
    }
}

pub enum TypeError {
    Ambiguous,
    Mismatch,
}

impl From<TypeError> for Diagnostic {
    fn from(value: TypeError) -> Self {
        Self {}
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicU32;

    use bumpalo::Bump;

    use crate::diagnostics::dummy_reporter;
    use crate::types::ParamId;
    use crate::types::infer;
    use crate::types::infer::*;
    use crate::types::ty_interners::TyInterners;

    /// A minimal AST for exercising the type inference algorithms
    #[derive(Clone, Debug)]
    struct Expr<'ty> {
        id: u32,
        kind: ExprKind<'ty>,
    }

    #[derive(Clone, Debug)]
    enum ExprKind<'ty> {
        /// A literal with an exact type
        Lit(Ty<'ty>),
        /// A variable reference
        Var(VarId),
        /// A numeric binary op of the form `(T, T) -> T`
        NumBinOp(Box<Expr<'ty>>, Box<Expr<'ty>>),
        /// A closure
        Closure {
            params: Vec<(VarId, Ty<'ty>)>,
            ret: Ty<'ty>,
            body: Box<Expr<'ty>>,
        },
        /// A function call
        Call { func: FuncDecl<'ty>, args: Vec<Expr<'ty>> },
    }

    /// Unique variable ID (post name resolution)
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct VarId(u32);

    /// A function declaration
    #[derive(Clone, Debug)]
    struct FuncDecl<'ty> {
        generic_tys: Vec<ParamId>,
        params: Vec<Ty<'ty>>,
        ret: Ty<'ty>,
    }

    impl<'ty> ExprNode<'ty> for Expr<'ty> {
        type Error = TypeError;

        fn id(&self) -> u32 {
            self.id
        }

        fn infer<'a>(&self, ctx: &mut InferCtx<'a, 'ty>, expected: Ty<'ty>) -> Result<Ty<'ty>, Self::Error> {
            match &self.kind {
                ExprKind::Lit(ty) => Ok(*ty),
                ExprKind::Call { func, args: _ } => Ok(func.ret), // FIXME
                _ => panic!("implement {:?}", self.kind),
            }
        }
    }

    static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    fn expr<'ty>(kind: ExprKind<'ty>) -> Expr<'ty> {
        use std::sync::atomic::Ordering;
        Expr {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            kind,
        }
    }

    fn lit<'ty>(ty: Ty<'ty>) -> Expr<'ty> {
        expr(ExprKind::Lit(ty))
    }

    fn var<'ty>(id: VarId) -> Expr<'ty> {
        expr(ExprKind::Var(id))
    }

    fn num_binop<'ty>(lhs: Expr<'ty>, rhs: Expr<'ty>) -> Expr<'ty> {
        expr(ExprKind::NumBinOp(lhs.into(), rhs.into()))
    }

    fn bare_closure<'ty>(ctx: TyCtx<'_, 'ty>, params: impl IntoIterator<Item = VarId>, body: Expr<'ty>) -> Expr<'ty> {
        let infer = ctx.common().infer;
        expr(ExprKind::Closure {
            params: params.into_iter().map(|var| (var, infer)).collect(),
            ret: infer,
            body: body.into(),
        })
    }

    fn call<'ty>(func: FuncDecl<'ty>, args: impl IntoIterator<Item = Expr<'ty>>) -> Expr<'ty> {
        expr(ExprKind::Call {
            func,
            args: args.into_iter().collect(),
        })
    }

    fn mint_vars<const N: usize>() -> [VarId; N] {
        std::array::from_fn(|i| VarId(i as _))
    }

    fn map_fn<'a, 'ty>(ctx: TyCtx<'a, 'ty>) -> FuncDecl<'ty> {
        let (t_param, u_param) = (ParamId(0), ParamId(1));
        let (t, u) = (ctx.mk_param(t_param), ctx.mk_param(u_param));
        FuncDecl {
            generic_tys: vec![t_param, u_param],
            params: vec![ctx.mk_array(t), ctx.mk_func(&[t], u)],
            ret: ctx.mk_array(u),
        }
    }

    fn infer_ctx<'a, 'ty>(ctx: TyCtx<'a, 'ty>) -> InferCtx<'a, 'ty> {
        InferCtx {
            ty_ctx: ctx,
            nodes: Default::default(),
            diagnostics: dummy_reporter(),
        }
    }

    /// Tests the expression `map(xs, x => x + 1)`
    #[test]
    fn test_array_map() {
        let arena = Bump::new();
        let interners = TyInterners::new(&arena);
        let ctx = TyCtx::new(&arena, &interners);

        let i32 = ctx.common().int32;
        let i32_array = ctx.mk_array(ctx.common().int32);
        let [x] = mint_vars();

        let expr = call(
            map_fn(ctx),
            [lit(i32_array), bare_closure(ctx, [x], num_binop(var(x), lit(i32)))],
        );

        let mut infer_ctx = infer_ctx(ctx);
        let result = infer_ctx.infer(&expr, ctx.common().infer);
        assert_eq!(result, i32_array);
    }
}
