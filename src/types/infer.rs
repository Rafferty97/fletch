use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use itertools::Itertools;

use crate::diagnostics::ErrGuaranteed;
use crate::types::ty::{FuncTy, ParamId};
use crate::types::ty_ctx::Variance;

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

pub struct FuncDecl<'a, 'ty> {
    type_params: u32,
    params: &'a [Ty<'ty>],
    ret: Ty<'ty>,
}

pub struct InferCallResult<'ty> {
    pub params: Vec<Result<Ty<'ty>>>,
    pub ret: Ty<'ty>,
}

pub fn infer_call<'ty, A>(
    ctx: TyCtx<'_, 'ty>,
    func: FuncDecl<'_, 'ty>,
    type_args: &[Ty<'ty>],
    args: &[A],
    expected: Ty<'ty>,
    mut infer: impl FnMut(&A, Ty<'ty>) -> Ty<'ty>,
) -> Result<InferCallResult<'ty>> {
    check_arity(func.params.len(), args.len())?;

    let mut args = args
        .iter()
        .zip(func.params.iter())
        .map(|(arg, param)| {
            let ty = infer(arg, ctx.common().pending);
            (arg, *param, ty, ty.is_final())
        })
        .collect_vec();

    let mut bounds = vec![];

    loop {
        // Compute type parameter bounds from argument and return types
        bounds = type_args.iter().map(|ty| ctx.make_bounds(*ty)).collect();
        ctx.update_bounds(&mut bounds, func.ret, expected);
        for &(_, param, ty, _) in &args {
            ctx.update_bounds(&mut bounds, ty, param);
        }

        // compute argument types from type parameters
        let mut changed = false;
        for (arg, param, ty, done) in args.iter_mut().filter(|(_, _, _, done)| !done) {
            let expected = ctx.substitute_upper(*param, &bounds);
            let new_ty = infer(arg, expected);
            changed |= new_ty != *ty;
            *ty = new_ty;
            *done = new_ty.is_final();
        }

        if !changed {
            break;
        }
    }

    let ret = ctx.substitute_lower(func.ret, &bounds);

    let params = expected
        .is_final()
        .then(|| {
            bounds
                .into_iter()
                .map(|bounds| ctx.reconcile(bounds.lower, bounds.upper))
                .collect()
        })
        .unwrap_or_default();

    Ok(InferCallResult { params, ret })
}

pub struct ClosureDef<'a, 'ty> {
    params: &'a [Ty<'ty>],
    ret: Ty<'ty>,
}

pub fn infer_closure<'ty>(
    ctx: TyCtx<'_, 'ty>,
    def: ClosureDef<'_, 'ty>,
    body: impl FnOnce(&[Ty<'ty>]) -> Ty<'ty>,
    expected: Ty<'ty>,
) -> Result<Ty<'ty>> {
    let ret = match expected.kind() {
        TyKind::Pending => ctx.common().pending,
        TyKind::Func(func) => {
            check_arity(func.params.len(), def.params.len())?;
            if func.params.iter().all(|t| t.is_final()) {
                body(&func.params)
            } else {
                ctx.common().pending
            }
        }
        _ => Err(TypeError::Mismatch)?,
    };

    Ok(ctx.mk_func(&def.params, ret))
}

fn check_arity(expected: usize, actual: usize) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        let (expected, actual) = (expected as u32, actual as u32);
        Err(TypeError::Arity { expected, actual })
    }
}

/// Represents the infered bounds of a type parameter
#[derive(Clone, Copy, Debug)]
struct TyBounds<'ty> {
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

            // Top and bottom types
            (Never, _) | (_, Any) => lhs,
            (_, Never) | (Any, _) => rhs,

            // Sentinal values
            (_, Error(err)) | (Error(err), _) => self.mk_error(err),
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

            // Top and bottom types
            (_, Never) | (Any, _) => lhs,
            (Never, _) | (_, Any) => rhs,

            // Sentinal values
            (_, Error(err)) | (Error(err), _) => self.mk_error(err),
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

    /// Produces an empty `TyBounds` with the identity elements
    fn empty_bounds(&self) -> TyBounds<'ty> {
        TyBounds { lower: self.common().never, upper: self.common().any }
    }

    /// Produces an empty `TyBounds` with the identity elements
    fn make_bounds(&self, ty: Ty<'ty>) -> TyBounds<'ty> {
        match ty.kind() {
            TyKind::Infer => self.empty_bounds(),
            // Compound types
            TyKind::Nullable(_) => todo!(),
            TyKind::Array(inner) => {
                let TyBounds { lower, upper } = self.make_bounds(inner);
                TyBounds { lower: self.mk_array(lower), upper: self.mk_array(upper) }
            }
            TyKind::Tuple(_) => todo!(),
            TyKind::Func(_) => todo!(),
            // Remaining scalar types
            _ => TyBounds { lower: ty, upper: ty },
        }
    }

    /// Compares the expected type `upper` against the provided type `lower`,
    /// extracts the resulting type parameter bounds, and applies them to `bounds`
    fn update_bounds(&self, bounds: &mut [TyBounds<'ty>], lower: Ty<'ty>, upper: Ty<'ty>) {
        use TyKind::*;

        match (lower.kind(), upper.kind()) {
            (Error(_), _) => upper.params(|id| bounds[id.0 as usize].lower = lower),
            (_, Error(_)) => lower.params(|id| bounds[id.0 as usize].upper = upper),
            (Param(id), _) => {
                let bound = &mut bounds[id.0 as usize].upper;
                *bound = self.meet(*bound, upper);
            }
            (_, Param(id)) => {
                let bound = &mut bounds[id.0 as usize].lower;
                *bound = self.join(*bound, lower);
            }
            (Infer, _) => upper.params(|id| bounds[id.0 as usize].lower = lower),
            (_, Infer) => lower.params(|id| bounds[id.0 as usize].upper = upper),
            (Pending, _) => upper.params(|id| bounds[id.0 as usize].lower = lower),
            (_, Pending) => lower.params(|id| bounds[id.0 as usize].upper = upper),
            (Array(lower), Array(upper)) => self.update_bounds(bounds, lower, upper),
            (Func(lower), Func(upper)) => {
                // FIXME: arity check
                for (lower, upper) in lower.params.iter().zip(upper.params.iter()) {
                    // Variance reverses in function arguments
                    self.update_bounds(bounds, *upper, *lower);
                }
                self.update_bounds(bounds, lower.ret, upper.ret);
            }
            (l, u) => panic!("todo: {l:?}, {u:?}"),
        }
    }

    /// Substitutes type parameters with concrete types
    pub fn substitute(self, ty: Ty<'ty>, params: &[Ty<'ty>]) -> Ty<'ty> {
        self.transform(ty, |ty| match ty.kind() {
            TyKind::Param(idx) => params[idx.0 as usize],
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types, using the upper bound
    fn substitute_upper(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(idx) => match var {
                Variance::Co => bounds[idx.0 as usize].upper,
                Variance::Contra => bounds[idx.0 as usize].lower,
            },
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types, using the lower bound
    fn substitute_lower(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(idx) => match var {
                Variance::Co => bounds[idx.0 as usize].lower,
                Variance::Contra => bounds[idx.0 as usize].upper,
            },
            _ => ty,
        })
    }

    /// Reconciles the expected type `upper` against the provided type `lower`,
    /// returning a canonical resulting type or an error if the types are incompatible or ambiguous
    pub fn reconcile(self, lower: Ty<'ty>, upper: Ty<'ty>) -> Result<Ty<'ty>, TypeError> {
        use TyKind::*;

        match (lower.kind(), upper.kind()) {
            // Equality
            _ if lower == upper => Ok(lower),

            // Sentinal values
            (Error(e), _) | (_, Error(e)) => Ok(self.mk_error(e)),
            (Pending, _) | (_, Pending) => unreachable!(),

            // Type inference
            (Infer, _) => {
                if !upper.has_infer() {
                    Ok(upper)
                } else {
                    Err(TypeError::Ambiguous)
                }
            }
            (_, Infer) => {
                if !lower.has_infer() {
                    Ok(lower)
                } else {
                    Err(TypeError::Ambiguous)
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
}

impl<'ty> Ty<'ty> {
    /// Returns `true` if no constituents of the type are `Pending`
    pub fn is_final(self) -> bool {
        self.fold(true, |acc, ty| acc && ty.kind() != TyKind::Pending)
    }

    /// Returns `true` if any constituents of the type are `Infer`
    pub fn has_infer(self) -> bool {
        self.fold(false, |acc, ty| acc || ty.kind() == TyKind::Infer)
    }

    /// Calls `visit` for each type parameter contained within the type
    pub fn params(self, mut visit: impl FnMut(ParamId)) {
        self.visit(|ty| match ty.kind() {
            TyKind::Param(id) => visit(id),
            _ => {}
        });
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TypeError {
    Ambiguous,
    Mismatch,
    Arity { expected: u32, actual: u32 },
    Unimplemented,
}

pub type Result<T, E = TypeError> = std::result::Result<T, E>;

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicU32;
    use std::sync::{LazyLock, Mutex};

    use bumpalo::Bump;

    use crate::diagnostics::{VecReporter, dummy_reporter};
    use crate::types::ty_interners::TyInterners;

    use super::*;

    // #[derive(Debug)]
    // struct TestEnv<'a, 'ty> {
    //     ty_ctx: TyCtx<'a, 'ty>,
    //     vars: HashMap<VarId, Ty<'ty>>,
    //     exprs: HashMap<u32, Ty<'ty>>,
    //     params: HashMap<ParamId, Ty<'ty>>,
    //     diagnostics: VecReporter,
    // }

    // /// A minimal AST for exercising the type inference algorithms
    // #[derive(Clone, Debug)]
    // struct Expr<'ty> {
    //     id: u32,
    //     kind: ExprKind<'ty>,
    // }

    // #[derive(Clone, Debug)]
    // enum ExprKind<'ty> {
    //     /// A literal with an exact type
    //     Lit(Ty<'ty>),
    //     /// A variable reference
    //     Var(VarId),
    //     /// A numeric binary op of the form `(T, T) -> T`
    //     NumBinOp(Box<Expr<'ty>>, Box<Expr<'ty>>),
    //     /// A closure
    //     Closure { def: FuncTy<'ty>, params: Vec<VarId>, body: Box<Expr<'ty>> },
    //     /// A function call
    //     Call { generic_tys: u32, def: FuncTy<'ty>, args: Vec<Expr<'ty>> },
    // }

    // /// Unique variable ID (post name resolution)
    // #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    // struct VarId(u32);

    // impl<'a, 'ty> InferEnv<'ty> for TestEnv<'a, 'ty> {
    //     type Expr = Expr<'ty>;

    //     fn ty_ctx(&self) -> TyCtx<'_, 'ty> {
    //         self.ty_ctx
    //     }

    //     fn set_expr_ty(&mut self, expr: &Self::Expr, ty: Ty<'ty>) {
    //         self.exprs.insert(expr.id, ty);
    //     }

    //     fn infer_expr(&mut self, expr: &Expr<'ty>, expected: Ty<'ty>) -> Ty<'ty> {
    //         match &expr.kind {
    //             &ExprKind::Lit(ty) => ty,
    //             &ExprKind::Var(id) => match self.vars.get(&id) {
    //                 Some(ty) => *ty,
    //                 None => todo!(),
    //             },
    //             ExprKind::NumBinOp(lhs, rhs) => {
    //                 let lhs = infer(self, &**lhs, expected);
    //                 let rhs = infer(self, &**rhs, expected);
    //                 if lhs.is_never() || rhs.is_never() {
    //                     return self.ty_ctx.common().never;
    //                 }
    //                 if lhs != rhs {
    //                     todo!()
    //                 }
    //                 if !matches!(lhs.kind(), TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_)) {
    //                     todo!()
    //                 }
    //                 lhs
    //             }
    //             ExprKind::Call { generic_tys, def, args } => {
    //                 let func = FuncDecl { generic_tys: *generic_tys, params: &def.params, ret: def.ret };
    //                 let Ok(ret) = infer_call(self, func, &args, expected) else { todo!() };
    //                 ret
    //             }
    //             ExprKind::Closure { def, params, body } => {
    //                 let body = |env: &mut TestEnv<'_, 'ty>, args: &[Ty<'ty>]| {
    //                     let vars = params.iter().copied().zip(args.iter().copied()).collect();
    //                     let mut env = TestEnv {
    //                         ty_ctx: env.ty_ctx,
    //                         vars,
    //                         exprs: HashMap::new(),
    //                         params: HashMap::new(),
    //                         diagnostics: env.diagnostics.clone(),
    //                     };
    //                     Ok(infer(&mut env, &*body, expected))
    //                 };
    //                 let Ok(ret) = infer_closure(self, def, body, expected) else { todo!() };
    //                 ret
    //             }
    //             _ => panic!("implement {:?}", expr.kind),
    //         }
    //     }

    //     fn set_param_ty(&mut self, id: ParamId, ty: Ty<'ty>) {
    //         self.params.insert(id, ty);
    //     }
    // }

    // static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    // fn expr<'ty>(kind: ExprKind<'ty>) -> Expr<'ty> {
    //     use std::sync::atomic::Ordering;
    //     Expr { id: NEXT_ID.fetch_add(1, Ordering::Relaxed), kind }
    // }

    // fn lit<'ty>(ty: Ty<'ty>) -> Expr<'ty> {
    //     expr(ExprKind::Lit(ty))
    // }

    // fn var<'ty>(id: VarId) -> Expr<'ty> {
    //     expr(ExprKind::Var(id))
    // }

    // fn num_binop<'ty>(lhs: Expr<'ty>, rhs: Expr<'ty>) -> Expr<'ty> {
    //     expr(ExprKind::NumBinOp(lhs.into(), rhs.into()))
    // }

    // fn bare_closure<'ty>(ctx: TyCtx<'_, 'ty>, params: impl IntoIterator<Item = VarId>, body: Expr<'ty>) -> Expr<'ty> {
    //     let infer = ctx.common().infer;
    //     let params = params.into_iter().collect_vec();
    //     expr(ExprKind::Closure {
    //         def: FuncTy { params: ctx.mk_tys(&vec![infer; params.len()]), ret: infer },
    //         params,
    //         body: body.into(),
    //     })
    // }

    // fn mint_vars<const N: usize>() -> [VarId; N] {
    //     std::array::from_fn(|i| VarId(i as _))
    // }

    // fn call_map_fn<'ty>(ctx: TyCtx<'_, 'ty>, xs: Expr<'ty>, map: Expr<'ty>) -> Expr<'ty> {
    //     let (t, u) = (ctx.mk_param(ParamId(0)), ctx.mk_param(ParamId(1)));
    //     let def = FuncTy {
    //         params: ctx.mk_tys(&[ctx.mk_array(t), ctx.mk_func(&[t], u)]),
    //         ret: ctx.mk_array(u),
    //     };
    //     expr(ExprKind::Call { generic_tys: 2, def, args: vec![xs, map] })
    // }

    // fn call_zip<'ty>(ctx: TyCtx<'_, 'ty>, xs: Expr<'ty>, ys: Expr<'ty>) -> Expr<'ty> {
    //     let t = ctx.mk_param(ParamId(0));
    //     let def = FuncTy { params: ctx.mk_tys(&[ctx.mk_array(t), ctx.mk_array(t)]), ret: ctx.mk_array(t) };
    //     expr(ExprKind::Call { generic_tys: 1, def, args: vec![xs, ys] })
    // }

    fn with_ctx(f: impl for<'ty> FnOnce(TyCtx<'_, 'ty>)) {
        let arena = Bump::new();
        let interners = TyInterners::new(&arena);
        let ctx = TyCtx::new(&arena, &interners);
        f(ctx);
    }

    fn mint_param_ids<'ty, const N: usize>(ctx: TyCtx<'_, 'ty>) -> [Ty<'ty>; N] {
        std::array::from_fn(|i| ctx.mk_param(ParamId(i as u32)))
    }

    enum MockExpr<'ty> {
        Lit(Ty<'ty>),
        BareClosure { args: usize, ret: Ty<'ty> },
    }

    fn mock_infer<'ty>(ctx: TyCtx<'_, 'ty>, expr: &MockExpr<'ty>, expected: Ty<'ty>) -> Ty<'ty> {
        match expr {
            MockExpr::Lit(ty) => *ty,
            MockExpr::BareClosure { args, ret } => {
                let infer = ctx.common().infer;
                let params = vec![infer; *args];
                let def = ClosureDef { params: &params, ret: infer };
                infer_closure(ctx, def, |_| *ret, expected).unwrap()
            }
        }
    }

    /// Tests the expression `map(xs, x => x + 1)`
    #[test]
    fn test_array_map() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let i32 = ctx.common().int32;
            let i32_array = ctx.mk_array(ctx.common().int32);

            let [t, u] = mint_param_ids(ctx);
            let func = FuncDecl {
                type_params: 2,
                params: &[ctx.mk_array(t), ctx.mk_func(&[t], u)],
                ret: ctx.mk_array(u),
            };

            let type_args = &[infer, infer];

            let xs = MockExpr::Lit(i32_array);
            let map = MockExpr::BareClosure { args: 1, ret: i32 };
            let args = &[xs, map];

            let result = infer_call(ctx, func, type_args, args, infer, |arg, expected| {
                mock_infer(ctx, arg, expected)
            });

            let result = result.unwrap();
            assert_eq!(result.ret, i32_array);
            assert_eq!(result.params, vec![Ok(i32), Ok(i32)]);
        });
    }

    /// Tests the expression `map([], x => x + 1)`
    #[test]
    fn test_empty_array_map() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let never = ctx.common().never;
            let i32 = ctx.common().int32;
            let empty_array = ctx.mk_array(ctx.common().never);

            let [t, u] = mint_param_ids(ctx);
            let func = FuncDecl {
                type_params: 2,
                params: &[ctx.mk_array(t), ctx.mk_func(&[t], u)],
                ret: ctx.mk_array(u),
            };

            let type_args = &[infer, infer];

            let xs = MockExpr::Lit(empty_array);
            let map = MockExpr::BareClosure { args: 1, ret: never };
            let args = &[xs, map];

            let result = infer_call(ctx, func, type_args, args, infer, |arg, expected| {
                mock_infer(ctx, arg, expected)
            });

            let result = result.unwrap();
            assert_eq!(result.ret, empty_array);
            assert_eq!(result.params, vec![Ok(never), Ok(never)]);
        });
    }

    // /// Tests the expression `concat(i32[], i64[]) -> i64[]`
    // #[test]
    // fn test_array_concat() {
    //     with_ctx(|env| {
    //         let ctx = env.ty_ctx;
    //         let i32_array = ctx.mk_array(ctx.common().int32);
    //         let i64_array = ctx.mk_array(ctx.common().int64);

    //         let expr = call_zip(ctx, lit(i32_array), lit(i64_array));

    //         let result = infer(env, &expr, ctx.common().infer);
    //         assert_eq!(result, i64_array);

    //         env.diagnostics.assert_ok();
    //     });
    // }
}
