use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use itertools::Itertools;

use crate::types::ty::{FuncTy, ParamId};
use crate::types::ty_ctx::Variance;

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

pub trait InferEnv<'ty> {
    type Expr;

    fn ty_ctx(&self) -> TyCtx<'_, 'ty>;

    fn get_expr_ty(&self, expr: &Self::Expr) -> Option<Ty<'ty>>;

    fn set_expr_ty(&mut self, expr: &Self::Expr, ty: Ty<'ty>);

    fn infer_expr(&mut self, expr: &Self::Expr, expected: Ty<'ty>) -> Ty<'ty>;

    fn get_param_ty(&self, param: ParamId) -> Option<Ty<'ty>>;

    fn set_param_ty(&mut self, param: ParamId, ty: Ty<'ty>);
}

pub fn infer<'ty, E>(env: &mut E, expr: &E::Expr, expected: Ty<'ty>) -> Ty<'ty>
where
    E: InferEnv<'ty>,
{
    if let Some(ty) = env.get_expr_ty(expr) {
        return ty;
    }

    let ty = env.infer_expr(expr, expected);

    if ty.is_final() {
        env.set_expr_ty(expr, ty);
    }

    ty
}

pub struct FuncDecl<'a, 'ty> {
    generic_tys: u32,
    params: &'a [Ty<'ty>],
    ret: Ty<'ty>,
}

pub fn infer_call<'ty, E>(
    env: &mut E,
    func: FuncDecl<'_, 'ty>,
    args: &[E::Expr],
    expected: Ty<'ty>,
) -> Result<impl Iterator<Item = Result<Ty<'ty>>>>
where
    E: InferEnv<'ty>,
{
    check_arity(func.params.len(), args.len())?;

    let pending = env.ty_ctx().common().pending;
    let mut arg_tys = args
        .iter()
        .zip(func.params.iter())
        .map(|(arg, param)| match env.get_expr_ty(arg) {
            Some(ty) => (arg, *param, ty, true),
            None => (arg, *param, pending, false),
        })
        .collect_vec();

    let empty_bounds = env.ty_ctx().empty_bounds();
    let mut bounds = vec![empty_bounds; func.generic_tys as usize];

    for i in 1.. {
        println!("Iteration {i}");

        // Compute type parameter bounds from argument and return types
        bounds.fill(empty_bounds);
        env.ty_ctx().update_bounds(&mut bounds, func.ret, expected);
        for &(_, param, ty, _) in &arg_tys {
            env.ty_ctx().update_bounds(&mut bounds, ty, param);
        }

        for (i, TyBounds { lower, upper }) in bounds.iter().enumerate() {
            println!("    ${i}  \t{lower}  \t{upper}");
        }

        // compute argument types from type parameters
        let mut changed = false;
        for (arg, param, ty, done) in arg_tys.iter_mut().filter(|(_, _, _, done)| !done) {
            let expect = env.ty_ctx().substitute_upper(*param, &bounds);
            let new_ty = infer(env, *arg, expect);
            changed |= new_ty != *ty;
            *ty = new_ty;
            *done = new_ty.is_final();
        }

        for (i, (_, param, ty, done)) in arg_tys.iter().enumerate() {
            println!("    {i}  \t{param}  \t{ty}  \t{done}");
        }
        println!();

        if !changed {
            break;
        }
    }

    Ok(bounds.into_iter().map(|b| env.ty_ctx().reconcile(b.lower, b.upper)))
}

pub fn infer_closure<'ty, E, F>(env: &mut E, def: &FuncTy<'ty>, body: F, expected: Ty<'ty>) -> Result<Ty<'ty>>
where
    E: InferEnv<'ty>,
    F: FnOnce(&mut E, &[Ty<'ty>]) -> Result<Ty<'ty>>,
{
    // Ensure expected type is a function of the correct arity
    let TyKind::Func(expect) = expected.kind() else { Err(TypeError::Mismatch)? };
    check_arity(expect.params.len(), def.params.len());

    // Ensure arguments are resolved before checking the body
    let ret = if expect.params.iter().all(|t| t.is_final()) {
        body(env, &expect.params)?
    } else {
        env.ty_ctx().common().pending
    };

    Ok(env.ty_ctx().mk_func(&def.params, ret))
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
                let tys: Vec<_> = lhs.iter().zip(rhs.iter()).map(|(&lhs, &rhs)| self.meet(lhs, rhs)).collect();
                self.mk_tuple(&tys)
            }

            // Functions
            (Func(lhs), Func(rhs)) if lhs.params.len() == rhs.params.len() => {
                let params: Vec<_> =
                    lhs.params.iter().zip(rhs.params.iter()).map(|(&lhs, &rhs)| self.meet(lhs, rhs)).collect();
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
                let tys: Vec<_> = lhs.iter().zip(rhs.iter()).map(|(&lhs, &rhs)| self.join(lhs, rhs)).collect();
                self.mk_tuple(&tys)
            }

            // Functions
            (Func(lhs), Func(rhs)) if lhs.params.len() == rhs.params.len() => {
                let params: Vec<_> =
                    lhs.params.iter().zip(rhs.params.iter()).map(|(&lhs, &rhs)| self.join(lhs, rhs)).collect();
                self.mk_func(&params, self.join(lhs.ret, rhs.ret))
            }

            // No common type
            _ => self.common().any,
        }
    }

    fn empty_bounds(&self) -> TyBounds<'ty> {
        TyBounds { lower: self.common().never, upper: self.common().any }
    }

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
    pub fn substitute_upper(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(idx) => match var {
                Variance::Co => bounds[idx.0 as usize].upper,
                Variance::Contra => bounds[idx.0 as usize].lower,
            },
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types, using the lower bound
    pub fn substitute_lower(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(idx) => match var {
                Variance::Co => bounds[idx.0 as usize].lower,
                Variance::Contra => bounds[idx.0 as usize].upper,
            },
            _ => ty,
        })
    }

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
    /// Returns `true` if no constituents of the type are pending
    pub fn is_final(self) -> bool {
        self.fold(true, |acc, ty| acc && ty.kind() != TyKind::Pending)
    }

    /// Returns `true` if any constituents of the type are `Infer`
    pub fn has_infer(self) -> bool {
        self.fold(false, |acc, ty| acc || ty.kind() == TyKind::Infer)
    }

    /// Iterates over the type parameters contains in this type
    pub fn params(self, mut visit: impl FnMut(ParamId)) {
        self.visit(|ty| match ty.kind() {
            TyKind::Param(id) => visit(id),
            _ => {}
        });
    }
}

#[derive(Debug)]
pub enum TypeError {
    Undefined,
    Ambiguous,
    Mismatch,
    BinOp,
    Arity { expected: u32, actual: u32 },
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

    #[derive(Debug)]
    struct TestEnv<'a, 'ty> {
        ty_ctx: TyCtx<'a, 'ty>,
        vars: HashMap<VarId, Ty<'ty>>,
        exprs: HashMap<u32, Ty<'ty>>,
        params: HashMap<ParamId, Ty<'ty>>,
        diagnostics: VecReporter,
    }

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
        Closure { def: FuncTy<'ty>, params: Vec<VarId>, body: Box<Expr<'ty>> },
        /// A function call
        Call { generic_tys: u32, def: FuncTy<'ty>, args: Vec<Expr<'ty>> },
    }

    /// Unique variable ID (post name resolution)
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    struct VarId(u32);

    impl<'a, 'ty> InferEnv<'ty> for TestEnv<'a, 'ty> {
        type Expr = Expr<'ty>;

        fn ty_ctx(&self) -> TyCtx<'_, 'ty> {
            self.ty_ctx
        }

        fn get_expr_ty(&self, expr: &Self::Expr) -> Option<Ty<'ty>> {
            self.exprs.get(&expr.id).copied()
        }

        fn set_expr_ty(&mut self, expr: &Self::Expr, ty: Ty<'ty>) {
            self.exprs.insert(expr.id, ty);
        }

        fn infer_expr(&mut self, expr: &Expr<'ty>, expected: Ty<'ty>) -> Ty<'ty> {
            match &expr.kind {
                &ExprKind::Lit(ty) => ty,
                &ExprKind::Var(id) => match self.vars.get(&id) {
                    Some(ty) => *ty,
                    None => todo!(),
                },
                ExprKind::NumBinOp(lhs, rhs) => {
                    let lhs = infer(self, &**lhs, expected);
                    let rhs = infer(self, &**rhs, expected);
                    if lhs.is_never() || rhs.is_never() {
                        return self.ty_ctx.common().never;
                    }
                    if lhs != rhs {
                        todo!()
                    }
                    if !matches!(lhs.kind(), TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_)) {
                        todo!()
                    }
                    lhs
                }
                ExprKind::Call { generic_tys, def, args } => {
                    let func = FuncDecl { generic_tys: *generic_tys, params: &def.params, ret: def.ret };
                    let Ok(params) = infer_call(self, func, &args, expected) else { todo!() };
                    let Ok(params) = params.collect::<Result<Vec<_>, _>>() else { todo!() };
                    self.ty_ctx.substitute(def.ret, &params)
                }
                ExprKind::Closure { def, params, body } => {
                    let body = |env: &mut TestEnv<'_, 'ty>, args: &[Ty<'ty>]| {
                        let vars = params.iter().copied().zip(args.iter().copied()).collect();
                        let mut env = TestEnv {
                            ty_ctx: env.ty_ctx,
                            vars,
                            exprs: HashMap::new(),
                            params: HashMap::new(),
                            diagnostics: env.diagnostics.clone(),
                        };
                        Ok(infer(&mut env, &*body, expected))
                    };
                    let Ok(ret) = infer_closure(self, def, body, expected) else { todo!() };
                    ret
                }
                _ => panic!("implement {:?}", expr.kind),
            }
        }

        fn get_param_ty(&self, id: ParamId) -> Option<Ty<'ty>> {
            self.params.get(&id).copied()
        }

        fn set_param_ty(&mut self, id: ParamId, ty: Ty<'ty>) {
            self.params.insert(id, ty);
        }
    }

    static NEXT_ID: AtomicU32 = AtomicU32::new(0);

    fn expr<'ty>(kind: ExprKind<'ty>) -> Expr<'ty> {
        use std::sync::atomic::Ordering;
        Expr { id: NEXT_ID.fetch_add(1, Ordering::Relaxed), kind }
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
        let params = params.into_iter().collect_vec();
        expr(ExprKind::Closure {
            def: FuncTy { params: ctx.mk_tys(&vec![infer; params.len()]), ret: infer },
            params,
            body: body.into(),
        })
    }

    fn mint_vars<const N: usize>() -> [VarId; N] {
        std::array::from_fn(|i| VarId(i as _))
    }

    fn call_map_fn<'ty>(ctx: TyCtx<'_, 'ty>, xs: Expr<'ty>, map: Expr<'ty>) -> Expr<'ty> {
        let (t, u) = (ctx.mk_param(ParamId(0)), ctx.mk_param(ParamId(1)));
        let def = FuncTy {
            params: ctx.mk_tys(&[ctx.mk_array(t), ctx.mk_func(&[t], u)]),
            ret: ctx.mk_array(u),
        };
        expr(ExprKind::Call { generic_tys: 2, def, args: vec![xs, map] })
    }

    fn call_zip<'ty>(ctx: TyCtx<'_, 'ty>, xs: Expr<'ty>, ys: Expr<'ty>) -> Expr<'ty> {
        let t = ctx.mk_param(ParamId(0));
        let def = FuncTy { params: ctx.mk_tys(&[ctx.mk_array(t), ctx.mk_array(t)]), ret: ctx.mk_array(t) };
        expr(ExprKind::Call { generic_tys: 1, def, args: vec![xs, ys] })
    }

    fn inference_test(f: impl for<'ty> FnOnce(&mut TestEnv<'_, 'ty>)) {
        let arena = Bump::new();
        let interners = TyInterners::new(&arena);
        let ty_ctx = TyCtx::new(&arena, &interners);

        let mut env = TestEnv {
            ty_ctx,
            vars: HashMap::new(),
            exprs: HashMap::new(),
            params: HashMap::new(),
            diagnostics: VecReporter::new(),
        };

        f(&mut env);
    }

    /// Tests the expression `map(xs, x => x + 1)`
    #[test]
    fn test_array_map() {
        inference_test(|env| {
            let ctx = env.ty_ctx;
            let i32 = ctx.common().int32;
            let i32_array = ctx.mk_array(ctx.common().int32);
            let [x] = mint_vars();

            let expr = call_map_fn(ctx, lit(i32_array), bare_closure(ctx, [x], num_binop(var(x), lit(i32))));

            let result = infer(env, &expr, ctx.common().infer);
            assert_eq!(result, i32_array);

            env.diagnostics.assert_ok();
        });
    }

    /// Tests the expression `map([], x => x + 1)`
    #[test]
    fn test_empty_array_map() {
        inference_test(|env| {
            let ctx = env.ty_ctx;
            let i32 = ctx.common().int32;
            let empty_array = ctx.mk_array(ctx.common().never);
            let [x] = mint_vars();

            let expr = call_map_fn(ctx, lit(empty_array), bare_closure(ctx, [x], num_binop(var(x), lit(i32))));

            let result = infer(env, &expr, ctx.common().infer);
            assert_eq!(result, empty_array);

            env.diagnostics.assert_ok();
        });
    }

    /// Tests the expression `concat(i32[], i64[]) -> i64[]`
    #[test]
    fn test_array_concat() {
        inference_test(|env| {
            let ctx = env.ty_ctx;
            let i32_array = ctx.mk_array(ctx.common().int32);
            let i64_array = ctx.mk_array(ctx.common().int64);

            let expr = call_zip(ctx, lit(i32_array), lit(i64_array));

            let result = infer(env, &expr, ctx.common().infer);
            assert_eq!(result, i64_array);

            env.diagnostics.assert_ok();
        });
    }
}
