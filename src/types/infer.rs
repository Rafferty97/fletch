use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use itertools::Itertools;

use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::types::ty::FuncTy;
use crate::types::ty_ctx::Variance;

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

pub type Result<T, E = TypeError> = std::result::Result<T, E>;

pub struct InferCtx<'a, 'ty, Env> {
    ty_ctx: TyCtx<'a, 'ty>,
    env: &'a Env,
    nodes: HashMap<u32, Ty<'ty>>,
    diagnostics: &'a dyn DiagnosticReporter,
}

impl<'a, 'ty, Env> InferCtx<'a, 'ty, Env> {
    pub fn derive<'b>(&self, env: &'b Env) -> InferCtx<'b, 'ty, Env>
    where
        'a: 'b,
    {
        InferCtx { env, nodes: HashMap::new(), ..*self }
    }

    pub fn env(&self) -> &Env {
        self.env
    }

    pub fn infer<N>(&mut self, node: &N, expect: Ty<'ty>) -> Ty<'ty>
    where
        N: ExprNode<'ty, Env = Env>,
    {
        if let Some(ty) = self.lookup_ty(node) {
            return ty;
        }

        let ty = match node.infer(self, expect) {
            Ok(ty) => ty,
            Err(err) => {
                let err = self.diagnostics.report(err.into());
                self.ty_ctx.mk_error(err)
            }
        };

        if ty.is_final() {
            self.nodes.insert(node.id(), ty);
        }

        ty
    }

    pub fn infer_call<N>(&mut self, params: u32, def: &FuncTy<'ty>, args: &[N], expect: Ty<'ty>) -> Result<Ty<'ty>>
    where
        N: ExprNode<'ty, Env = Env>,
    {
        check_arity(def.params.len(), args.len())?;

        let pending = self.ty_ctx.common().pending;
        let mut arg_tys = args
            .iter()
            .zip(def.params.iter())
            .map(|(arg, param)| match self.lookup_ty(arg) {
                Some(ty) => (arg, *param, ty, true),
                None => (arg, *param, pending, false),
            })
            .collect_vec();

        let empty_bounds = self.ty_ctx.empty_bounds();
        let mut bounds = vec![empty_bounds; params as usize];

        for i in 1.. {
            println!("Iteration {i}");

            // Compute type bounds
            bounds.fill(empty_bounds);
            self.ty_ctx.update_bounds(&mut bounds, def.ret, expect);
            for &(_, param, ty, _) in &arg_tys {
                self.ty_ctx.update_bounds(&mut bounds, ty, param);
            }

            for (i, TyBounds { lower, upper }) in bounds.iter().enumerate() {
                println!("    ${i}  \t{lower}  \t{upper}");
            }

            // compute arguments from T, U, etc.
            let mut changed = false;
            for (arg, param, ty, done) in arg_tys.iter_mut().filter(|(_, _, _, done)| !done) {
                let expect = self.ty_ctx.substitute_upper(*param, &bounds);
                let new_ty = self.infer(*arg, expect);
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

        println!("subst: {}, {:?}", def.ret, &bounds);
        Ok(self.ty_ctx.substitute_lower(def.ret, &bounds))
    }

    pub fn infer_closure<F>(&mut self, def: &FuncTy<'ty>, expect: Ty<'ty>, body: F) -> Result<Ty<'ty>>
    where
        F: FnOnce(&Self, &[Ty<'ty>]) -> Result<Ty<'ty>>,
    {
        // Ensure expected type is a function of the correct arity
        let TyKind::Func(expect) = expect.kind() else { Err(TypeError::Mismatch)? };
        check_arity(expect.params.len(), def.params.len());

        // Ensure arguments are resolved before checking the body
        let ret = if expect.params.iter().all(|t| t.is_final()) {
            body(self, &expect.params)?
        } else {
            self.ty_ctx.common().pending
        };

        Ok(self.ty_ctx.mk_func(&def.params, ret))
    }

    /// Finds the greatest lower bound of two types
    pub fn meet(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        self.ty_ctx.meet(lhs, rhs)
    }

    /// Finds the least upper bound of two types, if one exists
    pub fn join(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        self.ty_ctx.join(lhs, rhs)
    }

    fn lookup_ty(&self, node: &impl ExprNode<'ty>) -> Option<Ty<'ty>> {
        self.nodes.get(&node.id()).copied()
    }
}

fn check_arity(expected: usize, actual: usize) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        let (expected, actual) = (expected as u32, actual as u32);
        Err(TypeError::Arity { expected, actual })
    }
}

pub trait ExprNode<'ty>: Debug {
    type Env;

    fn id(&self) -> u32;

    fn infer<'a>(&self, ctx: &mut InferCtx<'a, 'ty, Self::Env>, expected: Ty<'ty>) -> Result<Ty<'ty>, TypeError>;
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
            (Param(id), _) => {
                let bound = &mut bounds[id as usize].upper;
                *bound = self.meet(*bound, upper);
            }
            (_, Param(id)) => {
                let bound = &mut bounds[id as usize].lower;
                *bound = self.join(*bound, lower);
            }
            (Infer, _) => upper.params(|id| bounds[id as usize].lower = lower),
            (_, Infer) => lower.params(|id| bounds[id as usize].upper = upper),
            (Pending, _) => upper.params(|id| bounds[id as usize].lower = lower),
            (_, Pending) => lower.params(|id| bounds[id as usize].upper = upper),
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
            TyKind::Param(idx) => params[idx as usize],
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types, using the upper bound
    pub fn substitute_upper(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(idx) => match var {
                Variance::Co => bounds[idx as usize].upper,
                Variance::Contra => bounds[idx as usize].lower,
            },
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types, using the lower bound
    pub fn substitute_lower(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(idx) => match var {
                Variance::Co => bounds[idx as usize].lower,
                Variance::Contra => bounds[idx as usize].upper,
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
    pub fn params(self, mut visit: impl FnMut(u32)) {
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

impl From<TypeError> for Diagnostic {
    fn from(err: TypeError) -> Self {
        Self { message: format!("{err:?}") }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicU32;
    use std::sync::{LazyLock, Mutex};

    use bumpalo::Bump;

    use crate::diagnostics::{VecReporter, dummy_reporter};
    use crate::types::infer;
    use crate::types::infer::*;
    use crate::types::ty_interners::TyInterners;

    #[derive(Default, Debug)]
    struct Env<'ty> {
        vars: HashMap<VarId, Ty<'ty>>,
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
        Call { func: FuncDecl<'ty>, args: Vec<Expr<'ty>> },
    }

    /// Unique variable ID (post name resolution)
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    struct VarId(u32);

    /// A function declaration
    #[derive(Clone, Debug)]
    struct FuncDecl<'ty> {
        generic_tys: u32,
        def: FuncTy<'ty>,
    }

    impl<'ty> ExprNode<'ty> for Expr<'ty> {
        type Env = Env<'ty>;

        fn id(&self) -> u32 {
            self.id
        }

        fn infer<'a>(&self, ctx: &mut InferCtx<'a, 'ty, Self::Env>, expect: Ty<'ty>) -> Result<Ty<'ty>, TypeError> {
            match &self.kind {
                ExprKind::Lit(ty) => Ok(*ty),
                ExprKind::Var(id) => ctx.env().vars.get(id).copied().ok_or(TypeError::Undefined),
                ExprKind::NumBinOp(lhs, rhs) => {
                    let lhs = ctx.infer(&**lhs, expect);
                    let rhs = ctx.infer(&**rhs, expect);
                    if lhs != rhs {
                        println!("binop 1: {lhs} vs {rhs}");
                        Err(TypeError::BinOp)?;
                    }
                    if !matches!(lhs.kind(), TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_)) {
                        println!("binop 2");
                        Err(TypeError::BinOp)?;
                    }
                    Ok(lhs)
                }
                ExprKind::Call { func, args } => ctx.infer_call(func.generic_tys, &func.def, args, expect),
                ExprKind::Closure { def, params, body } => ctx.infer_closure(def, expect, |ctx, args| {
                    let vars = params.iter().copied().zip(args.iter().copied()).collect();
                    Ok(ctx.derive(&Env { vars }).infer(&**body, expect))
                }),
                _ => panic!("implement {:?}", self.kind),
            }
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

    fn call<'ty>(func: FuncDecl<'ty>, args: impl IntoIterator<Item = Expr<'ty>>) -> Expr<'ty> {
        expr(ExprKind::Call { func, args: args.into_iter().collect() })
    }

    fn mint_vars<const N: usize>() -> [VarId; N] {
        std::array::from_fn(|i| VarId(i as _))
    }

    fn map_fn<'a, 'ty>(ctx: TyCtx<'a, 'ty>) -> FuncDecl<'ty> {
        let (t, u) = (ctx.mk_param(0), ctx.mk_param(1));
        FuncDecl {
            generic_tys: 2,
            def: FuncTy { params: ctx.mk_tys(&[ctx.mk_array(t), ctx.mk_func(&[t], u)]), ret: ctx.mk_array(u) },
        }
    }

    fn infer_ctx<'a, 'ty>(
        ctx: TyCtx<'a, 'ty>,
        diagnostics: &'a impl DiagnosticReporter,
    ) -> InferCtx<'a, 'ty, Env<'ty>> {
        static EMPTY_MAP: LazyLock<Env<'static>> = LazyLock::new(|| Default::default());
        InferCtx { ty_ctx: ctx, env: &EMPTY_MAP, nodes: Default::default(), diagnostics }
    }

    fn inference_test(f: impl for<'ty> FnOnce(TyCtx<'_, 'ty>, &mut InferCtx<'_, 'ty, Env<'ty>>)) {
        let arena = Bump::new();
        let interners = TyInterners::new(&arena);
        let ctx = TyCtx::new(&arena, &interners);

        let mut errors = VecReporter::new();
        let mut infer_ctx = infer_ctx(ctx, &errors);

        f(ctx, &mut infer_ctx);

        errors.assert_ok();
    }

    /// Tests the expression `map(xs, x => x + 1)`
    #[test]
    fn test_array_map() {
        inference_test(|ctx, infer| {
            let i32 = ctx.common().int32;
            let i32_array = ctx.mk_array(ctx.common().int32);
            let [x] = mint_vars();

            let expr = call(map_fn(ctx), [lit(i32_array), bare_closure(ctx, [x], num_binop(var(x), lit(i32)))]);

            let result = infer.infer(&expr, ctx.common().infer);
            assert_eq!(result, i32_array);
        });
    }
}
