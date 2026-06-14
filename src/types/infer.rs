use std::fmt::Debug;

use itertools::Itertools;

use crate::types::ty::{FuncTy, ParamId};
use crate::types::ty_ctx::Variance;

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

#[derive(Debug)]
pub struct FuncDecl<'a, 'ty> {
    type_params: u32,
    params: &'a [Ty<'ty>],
    ret: Ty<'ty>,
}

#[derive(Debug)]
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
    check_arity(func.type_params as usize, type_args.len())?;
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
    fn make_bounds(&self, ty: Ty<'ty>) -> TyBounds<'ty> {
        TyBounds { lower: self.make_bound(ty, false), upper: self.make_bound(ty, true) }
    }

    fn make_bound(&self, ty: Ty<'ty>, upper: bool) -> Ty<'ty> {
        match ty.kind() {
            TyKind::Infer => {
                if upper {
                    self.common().any
                } else {
                    self.common().never
                }
            }
            TyKind::Nullable(ty) => self.mk_array(self.make_bound(ty, upper)),
            TyKind::Array(ty) => self.mk_array(self.make_bound(ty, upper)),
            TyKind::Tuple(_) => todo!(),
            TyKind::Func(FuncTy { params, ret }) => {
                let params = &params.iter().map(|ty| self.make_bound(*ty, !upper)).collect_vec();
                let ret = self.make_bound(ret, upper);
                self.mk_func(&params, ret)
            }
            _ => ty,
        }
    }

    /// Compares the expected type `upper` against the provided type `lower`,
    /// extracts the resulting type parameter bounds, and applies them to `bounds`
    fn update_bounds(&self, bounds: &mut [TyBounds<'ty>], lower: Ty<'ty>, upper: Ty<'ty>) {
        use TyKind::*;

        if lower.is_scalar() && upper.is_scalar() {
            return;
        }

        match (lower.kind(), upper.kind()) {
            (Error(_), _) => Self::replace_bounds(bounds, upper, lower, false),
            (_, Error(_)) => Self::replace_bounds(bounds, lower, upper, true),
            (Param(id), _) => {
                let bound = &mut bounds[id.0 as usize].upper;
                *bound = self.meet(*bound, upper);
            }
            (_, Param(id)) => {
                let bound = &mut bounds[id.0 as usize].lower;
                *bound = self.join(*bound, lower);
            }
            (Infer, _) => Self::replace_bounds(bounds, upper, lower, false),
            (_, Infer) => Self::replace_bounds(bounds, lower, upper, true),
            (Pending, _) => Self::replace_bounds(bounds, upper, lower, false),
            (_, Pending) => Self::replace_bounds(bounds, lower, upper, true),
            (Array(lower), Array(upper)) => self.update_bounds(bounds, lower, upper),
            (Func(lower), Func(upper)) => {
                // FIXME: arity check
                for (lower, upper) in lower.params.iter().zip(upper.params.iter()) {
                    // Variance reverses in function arguments
                    self.update_bounds(bounds, *upper, *lower);
                }
                self.update_bounds(bounds, lower.ret, upper.ret);
            }
            _ => {}
        }
    }

    fn replace_bounds(bounds: &mut [TyBounds<'ty>], target: Ty<'ty>, value: Ty<'ty>, upper: bool) {
        match target.kind() {
            TyKind::Nullable(ty) => Self::replace_bounds(bounds, ty, value, upper),
            TyKind::Array(ty) => Self::replace_bounds(bounds, ty, value, upper),
            TyKind::Tuple(tys) => {
                for ty in tys.iter().copied() {
                    Self::replace_bounds(bounds, ty, value, upper);
                }
            }
            TyKind::Func(FuncTy { params, ret }) => {
                for ty in params.iter().copied() {
                    Self::replace_bounds(bounds, ty, value, !upper);
                }
                Self::replace_bounds(bounds, ret, value, upper);
            }
            TyKind::Param(id) => {
                if upper {
                    bounds[id.0 as usize].upper = value;
                } else {
                    bounds[id.0 as usize].lower = value;
                }
            }
            _ => {}
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
            (Infer, Infer) => Err(TypeError::Ambiguous),
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
    use bumpalo::Bump;

    use crate::types::ty_interners::TyInterners;

    use super::*;

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

    /// Tests the expression `concat(i32[], i64[]) -> i64[]`
    #[test]
    fn test_array_concat() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let i32_array = ctx.mk_array(ctx.common().int32);
            let i64_array = ctx.mk_array(ctx.common().int64);

            let [t] = mint_param_ids(ctx);
            let func = FuncDecl {
                type_params: 1,
                params: &[ctx.mk_array(t), ctx.mk_array(t)],
                ret: ctx.mk_array(t),
            };

            let type_args = &[infer];

            let xs = MockExpr::Lit(i32_array);
            let ys = MockExpr::Lit(i64_array);
            let args = &[xs, ys];

            let result = infer_call(ctx, func, type_args, args, infer, |arg, expected| {
                mock_infer(ctx, arg, expected)
            });

            let result = result.unwrap();
            assert_eq!(result.ret, i64_array);
            assert_eq!(result.params, vec![Ok(ctx.common().int64)]);
        });
    }

    /// Tests the expression `both(a => a > 0, b => b > 0, 5) -> bool`
    #[test]
    fn test_two_lambas() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let bool = ctx.common().bool;
            let i32 = ctx.common().int32;

            let [t] = mint_param_ids(ctx);
            let func = FuncDecl {
                type_params: 1,
                params: &[ctx.mk_func(&[t], bool), ctx.mk_func(&[t], bool), t],
                ret: bool,
            };

            let type_args = &[infer];

            let func_a = MockExpr::BareClosure { args: 1, ret: bool };
            let func_b = MockExpr::BareClosure { args: 1, ret: bool };
            let var = MockExpr::Lit(i32);
            let args = &[func_a, func_b, var];

            let result = infer_call(ctx, func, type_args, args, infer, |arg, expected| {
                mock_infer(ctx, arg, expected)
            });

            let result = result.unwrap();
            assert_eq!(result.ret, bool);
            assert_eq!(result.params, vec![Ok(i32)]);
        });
    }

    /// Tests the expression `foo(42, y => y > 0)`, where `foo: (T, T -> bool) -> (T -> i32)`
    #[test]
    fn test_infer_both_directions() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let bool = ctx.common().bool;
            let i32 = ctx.common().int32;

            let [t] = mint_param_ids(ctx);
            let func = FuncDecl {
                type_params: 1,
                params: &[t, ctx.mk_func(&[t], bool)],
                ret: ctx.mk_func(&[t], i32),
            };

            let type_args = &[infer];
            let args = &[MockExpr::Lit(i32), MockExpr::BareClosure { args: 1, ret: bool }];

            let result = infer_call(ctx, func, type_args, args, infer, |arg, expected| {
                mock_infer(ctx, arg, expected)
            });

            let result = result.unwrap();
            assert_eq!(result.ret, ctx.mk_func(&[infer], i32));
            assert_eq!(result.params, vec![Err(TypeError::Ambiguous)]);
        });
    }

    /// Tests the expression `foo(42, y => y > 0)`, where `foo: (T, T -> bool) -> (T -> i32)`
    /// Same as above, except the expression has an expected type of `i32 -> i32`
    #[test]
    fn test_infer_both_directions_annotated() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let bool = ctx.common().bool;
            let i32 = ctx.common().int32;

            let [t] = mint_param_ids(ctx);
            let func = FuncDecl {
                type_params: 1,
                params: &[t, ctx.mk_func(&[t], bool)],
                ret: ctx.mk_func(&[t], i32),
            };

            let type_args = &[infer];
            let args = &[MockExpr::Lit(i32), MockExpr::BareClosure { args: 1, ret: bool }];

            let expect = ctx.mk_func(&[i32], i32);

            let result = infer_call(ctx, func, type_args, args, expect, |arg, expected| {
                mock_infer(ctx, arg, expected)
            });

            let result = result.unwrap();
            assert_eq!(result.ret, ctx.mk_func(&[infer], i32));
            assert_eq!(result.params, vec![Ok(i32)]);
        });
    }
}
