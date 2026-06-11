// Type

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use tap::Pipe;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Never,
    Bool,
    Int(IntTy),
    Nullable(Box<Ty>),
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    Func(FuncTy),
    Param(ParamId),
    Unknown,
    Infer,
    Err,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum IntTy {
    Int8,
    Int16,
    Int32,
    Int64,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct FuncTy {
    pub params: Vec<Ty>,
    pub ret: Box<Ty>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ParamId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct VarId(pub u32);

// Variance

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Variance {
    Invariant,
    Covariant,
    Contravariant,
    Bivariant,
}

impl Variance {
    pub fn invert(self) -> Self {
        match self {
            Self::Invariant => Self::Invariant,
            Self::Covariant => Self::Contravariant,
            Self::Contravariant => Self::Covariant,
            Self::Bivariant => Self::Bivariant,
        }
    }

    pub fn merge(a: Self, b: Self) -> Self {
        let (a, b) = (a.into_parts(), b.into_parts());
        Self::from_parts(a.0 && b.0, a.1 && b.1)
    }

    fn from_parts(co: bool, contra: bool) -> Self {
        match (co, contra) {
            (false, false) => Self::Invariant,
            (true, false) => Self::Covariant,
            (false, true) => Self::Contravariant,
            (true, true) => Self::Bivariant,
        }
    }

    fn into_parts(self) -> (bool, bool) {
        match self {
            Self::Invariant => (false, false),
            Self::Covariant => (true, false),
            Self::Contravariant => (false, true),
            Self::Bivariant => (true, true),
        }
    }
}

pub fn variance_in_ty(ty: &Ty, param: ParamId) -> Variance {
    match ty {
        Ty::Array(ty) => variance_in_ty(ty, param),
        Ty::Tuple(tys) => variance_in_tys(tys, param),
        Ty::Func(func) => {
            let param_variance = variance_in_tys(&func.params, param).invert();
            let ret_variance = variance_in_ty(&func.ret, param);
            Variance::merge(param_variance, ret_variance)
        }
        Ty::Param(v) if *v == param => Variance::Covariant,
        _ => Variance::Bivariant,
    }
}

pub fn variance_in_tys<'a>(tys: impl IntoIterator<Item = &'a Ty>, param: ParamId) -> Variance {
    tys.into_iter()
        .map(|ty| variance_in_ty(ty, param))
        .fold(Variance::Bivariant, Variance::merge)
}

// Meet, join and check_subtype

pub fn meet(a: &Ty, b: &Ty) -> Ty {
    match (a, b) {
        _ if a == b => a.clone(),
        (Ty::Err, _) | (_, Ty::Err) => Ty::Err,
        (Ty::Infer, _) | (_, Ty::Infer) => Ty::Infer,
        (ty, Ty::Unknown) | (Ty::Unknown, ty) => ty.clone(),
        (Ty::Int(a), Ty::Int(b)) => Ty::Int(*a.min(b)),
        (Ty::Nullable(a), Ty::Nullable(b)) => Ty::Nullable(meet(a, b).into()),
        (a, Ty::Nullable(b)) => meet(a, b),
        (Ty::Nullable(a), b) => meet(a, b),
        (Ty::Array(a), Ty::Array(b)) => Ty::Array(meet(a, b).into()),
        // todo: tuple, func
        _ => Ty::Never,
    }
}

pub fn join(a: &Ty, b: &Ty) -> Ty {
    match (a, b) {
        _ if a == b => a.clone(),
        (Ty::Err, _) | (_, Ty::Err) => Ty::Err,
        (Ty::Infer, _) | (_, Ty::Infer) => Ty::Infer,
        (ty, Ty::Never) | (Ty::Never, ty) => ty.clone(),
        (Ty::Int(a), Ty::Int(b)) => Ty::Int(*a.max(b)),
        (Ty::Nullable(a), Ty::Nullable(b)) => Ty::Nullable(join(a, b).into()),
        (a, Ty::Nullable(b)) => Ty::Nullable(join(a, b).into()),
        (Ty::Nullable(a), b) => Ty::Nullable(join(a, b).into()),
        (Ty::Array(a), Ty::Array(b)) => Ty::Array(join(a, b).into()),
        // todo: tuple, func
        _ => Ty::Unknown,
    }
}

pub fn check_subtype(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        _ if a == b => true,
        (Ty::Never, _) | (_, Ty::Unknown) => true,
        (_, Ty::Err) | (Ty::Err, _) => true,
        (Ty::Int(a), Ty::Int(b)) => a <= b,
        (Ty::Nullable(a), Ty::Nullable(b)) => check_subtype(a, b),
        (_, Ty::Nullable(b)) => check_subtype(a, b),
        (Ty::Array(a), Ty::Array(b)) => check_subtype(a, b),
        (Ty::Tuple(a), Ty::Tuple(b)) if a.len() == b.len() => {
            a.iter().zip(b.iter()).all(|(a, b)| check_subtype(a, b))
        }
        (Ty::Func(a), Ty::Func(b)) => {
            a.params.len() == b.params.len()
                && a.params.iter().zip(b.params.iter()).all(|(a, b)| check_subtype(b, a)) // contravariant
                && check_subtype(&a.ret, &b.ret) // covariant
        }
        _ => false,
    }
}

// Check generic parameter call

#[derive(Default)]
struct InferCtx {
    bounds: HashMap<ParamId, (Ty, Ty)>,
    params: HashMap<ParamId, Ty>,
}

impl InferCtx {
    fn subst_params(&mut self) -> Result<(), String> {
        self.params = self
            .bounds
            .iter()
            .map(|(id, (lower, upper))| Ok((*id, reconcile(upper, lower)?)))
            .collect::<Result<_, String>>()?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct FuncDecl {
    params: Vec<Ty>,
    ret: Ty,
    type_params: Vec<ParamId>,
}

struct Mock(Ty);

struct FuncCall {
    func: FuncDecl,
    args: Vec<Box<dyn Expr>>,
}

trait Expr {
    fn solve(&self, ctx: &mut InferCtx, expected: &Ty) -> Result<Ty, String>;
    fn check(&self, ctx: &InferCtx) -> Result<Ty, String>;
}

impl Expr for Mock {
    fn solve(&self, _ctx: &mut InferCtx, _expected: &Ty) -> Result<Ty, String> {
        Ok(self.0.clone())
    }

    fn check(&self, _ctx: &InferCtx) -> Result<Ty, String> {
        Ok(self.0.clone())
    }
}

impl Expr for FuncCall {
    fn solve(&self, ctx: &mut InferCtx, expected: &Ty) -> Result<Ty, String> {
        fn print_bounds(bounds: &[(Ty, Ty)]) {
            print!("Bounds: ");
            if bounds.is_empty() {
                println!("<empty>");
                return;
            }
            for bound in bounds {
                print!("{bound:?}\t");
            }
            println!();
        }

        for id in &self.func.type_params {
            ctx.bounds.entry(*id).or_insert((Ty::Never, Ty::Unknown));
        }

        if self.args.len() != self.func.params.len() {
            Err(format!(
                "expected {} arguments, got {}",
                self.func.params.len(),
                self.args.len()
            ))?;
        };

        update_bounds(ctx, &self.func.ret, expected, true)?;
        // print_bounds(&bounds);

        loop {
            let mut changed = false;

            for (arg, param) in self.args.iter().zip(&self.func.params) {
                let expected = substitute_bounded(ctx, param, true);
                let actual = arg.solve(ctx, &expected)?;
                println!("    {param} => exp: {expected}\tact: {actual}");
                changed |= update_bounds(ctx, param, &actual, false)?;
            }
            // print_bounds(&bounds);

            if !changed {
                break;
            }
        }

        Ok(substitute_bounded(ctx, &self.func.ret, false))
    }

    fn check(&self, ctx: &InferCtx) -> Result<Ty, String> {
        // Check arguments
        for (arg, param) in self.args.iter().zip(&self.func.params) {
            let expected = substitute(&ctx, param);
            let actual = arg.check(ctx)?;
            reconcile(&expected, &actual)?;
        }

        // Return type
        Ok(substitute(&ctx, &self.func.ret))
    }
}

fn check_func_call_outer(
    func: FuncDecl,
    args: Vec<Box<dyn Expr>>,
    expected: &Ty,
) -> Result<Ty, String> {
    let func_call = FuncCall { func, args };
    let mut ctx = InferCtx::default();
    func_call.solve(&mut ctx, expected)?;
    ctx.subst_params()?;
    func_call.check(&ctx)
}

fn substitute(ctx: &InferCtx, ty: &Ty) -> Ty {
    match ty {
        Ty::Nullable(inner) => Ty::Nullable(substitute(ctx, inner).into()),
        Ty::Array(inner) => Ty::Array(substitute(ctx, inner).into()),
        Ty::Tuple(tys) => tys
            .iter()
            .map(|param| substitute(ctx, param))
            .collect::<Vec<_>>()
            .pipe(Ty::Tuple),
        Ty::Func(func) => {
            let params = func
                .params
                .iter()
                .map(|param| substitute(ctx, param))
                .collect();
            let ret = substitute(ctx, &func.ret).into();
            Ty::Func(FuncTy { params, ret })
        }
        Ty::Param(id) => ctx.params[id].clone(),
        _ => ty.clone(),
    }
}

fn substitute_bounded(ctx: &InferCtx, ty: &Ty, is_upper: bool) -> Ty {
    match ty {
        Ty::Nullable(inner) => Ty::Nullable(substitute_bounded(ctx, inner, is_upper).into()),
        Ty::Array(inner) => Ty::Array(substitute_bounded(ctx, inner, is_upper).into()),
        Ty::Tuple(tys) => tys
            .iter()
            .map(|param| substitute_bounded(ctx, param, is_upper))
            .collect::<Vec<_>>()
            .pipe(Ty::Tuple),
        Ty::Func(func) => {
            let params = func
                .params
                .iter()
                .map(|param| substitute_bounded(ctx, param, !is_upper))
                .collect();
            let ret = substitute_bounded(ctx, &func.ret, is_upper).into();
            Ty::Func(FuncTy { params, ret })
        }
        Ty::Param(id) => {
            let bounds = &ctx.bounds[id];
            if is_upper {
                bounds.1.clone()
            } else {
                bounds.0.clone()
            }
        }
        _ => ty.clone(),
    }
}

fn update_bounds(ctx: &mut InferCtx, param: &Ty, arg: &Ty, is_upper: bool) -> Result<bool, String> {
    match (param, arg) {
        // Error
        (Ty::Err, _) | (_, Ty::Err) => Ok(false),
        // Hit a type parameter - update the appropriate bound
        (Ty::Param(id), arg) => {
            let (lower, upper) = ctx.bounds.get_mut(id).unwrap();
            if is_upper {
                // Contravariant position - arg contributes to upper bound via meet
                let new_upper = meet(upper, arg);
                if new_upper != *upper {
                    *upper = new_upper;
                    return Ok(true);
                }
            } else {
                // Covariant position - arg contributes to lower bound via join
                let new_lower = join(lower, arg);
                if new_lower != *lower {
                    *lower = new_lower;
                    return Ok(true);
                }
            }
            Ok(false)
        }
        // Infer decomposes with everything
        (_, Ty::Infer) => Ok(update_bounds_infer(ctx, param, is_upper)),
        // Scalars
        (Ty::Int(p), Ty::Int(a)) if a <= p => Ok(false),
        // Structural decomposition
        (Ty::Nullable(p), Ty::Nullable(a)) => update_bounds(ctx, p, a, is_upper),
        (p, Ty::Nullable(_)) => Err(format!(
            "cannot pass nullable value to non-nullable parameter {:?}",
            p
        )),
        (Ty::Nullable(p), _) => update_bounds(ctx, p, arg, is_upper),
        (Ty::Array(p), Ty::Array(a)) => update_bounds(ctx, p, a, is_upper),
        (Ty::Tuple(ps), Ty::Tuple(as_)) if ps.len() == as_.len() => {
            let mut changed = false;
            for (p, a) in ps.iter().zip(as_) {
                changed |= update_bounds(ctx, p, a, is_upper)?;
            }
            Ok(changed)
        }
        (Ty::Func(p), Ty::Func(a)) if p.params.len() == a.params.len() => {
            let mut changed = false;
            // Contravariant in params - flip rev
            for (pp, ap) in p.params.iter().zip(&a.params) {
                changed |= update_bounds(ctx, pp, ap, !is_upper)?;
            }
            // Covariant in return
            changed |= update_bounds(ctx, &p.ret, &a.ret, is_upper)?;
            Ok(changed)
        }
        (_, Ty::Param(id)) => unreachable!("arg contained param {:?}", id),
        (p, a) if p == a => Ok(false),
        (p, a) => Err(format!(
            "type mismatch: {:?} is not a subtype of {:?}",
            a, p
        )),
    }
}

fn update_bounds_infer(ctx: &mut InferCtx, param: &Ty, is_upper: bool) -> bool {
    match param {
        Ty::Param(id) => {
            let (lower, upper) = ctx.bounds.get_mut(id).unwrap();
            if is_upper {
                // Contravariant position - arg contributes to upper bound via meet
                if *upper != Ty::Infer {
                    *upper = Ty::Infer;
                    return true;
                }
            } else {
                // Covariant position - arg contributes to lower bound via join
                if *lower != Ty::Infer {
                    *lower = Ty::Infer;
                    return true;
                }
            }
            false
        }
        Ty::Nullable(inner) => update_bounds_infer(ctx, inner, is_upper),
        Ty::Array(inner) => update_bounds_infer(ctx, inner, is_upper),
        Ty::Tuple(tys) => {
            let mut changed = false;
            for ty in tys {
                changed |= update_bounds_infer(ctx, ty, is_upper);
            }
            changed
        }
        Ty::Func(FuncTy { params, ret }) => {
            let mut changed = false;
            for ty in params {
                changed |= update_bounds_infer(ctx, ty, !is_upper);
            }
            changed |= update_bounds_infer(ctx, ret, is_upper);
            changed
        }
        _ => false,
    }
}

fn reconcile(expected: &Ty, actual: &Ty) -> Result<Ty, String> {
    match (expected, actual) {
        (Ty::Infer, t) => {
            if contains_sink(t) {
                Err("ambiguous type; annotation required".into())
            } else {
                Ok(t.clone())
            }
        }
        (a, _) if !contains_sink(a) => Ok(a.clone()), // annotation fully concrete here, take it
        // annotation has `_` somewhere inside: descend structurally, annotation as skeleton
        (Ty::Array(a), Ty::Array(t)) => Ok(Ty::Array(reconcile(a, t)?.into())),
        (Ty::Nullable(a), Ty::Nullable(t)) => Ok(Ty::Nullable(reconcile(a, t)?.into())),
        (Ty::Nullable(a), t) => Ok(Ty::Nullable(reconcile(a, t)?.into())),
        (Ty::Tuple(a), Ty::Tuple(t)) if a.len() == t.len() => a
            .iter()
            .zip(t)
            .map(|(a, t)| reconcile(a, t))
            .collect::<Result<_, _>>()
            .map(Ty::Tuple),
        (Ty::Func(a), Ty::Func(t)) if a.params.len() == t.params.len() => {
            let params = a
                .params
                .iter()
                .zip(&t.params)
                .map(|(a, t)| reconcile(a, t))
                .collect::<Result<_, _>>()?;
            let ret = reconcile(&a.ret, &t.ret)?.into();
            Ok(Ty::Func(FuncTy { params, ret }))
        }
        _ => unreachable!(),
    }
}

/// Recursively checks whether a type contains `sink` (`Ty::Infer`) anywhere
/// in its structure. A `sink` reaching the root of an expression is the
/// signal for an ambiguity error — inference couldn't determine the type
/// from either producers or consumers, and an annotation is required.
fn contains_sink(ty: &Ty) -> bool {
    match ty {
        Ty::Infer => true,
        Ty::Nullable(inner) | Ty::Array(inner) => contains_sink(inner),
        Ty::Tuple(tys) => tys.iter().any(contains_sink),
        Ty::Func(func) => func.params.iter().any(contains_sink) || contains_sink(&func.ret),
        _ => false,
    }
}

// Display

impl Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Never => write!(f, "!"),
            Ty::Bool => write!(f, "bool"),
            Ty::Int(IntTy::Int8) => write!(f, "i8"),
            Ty::Int(IntTy::Int16) => write!(f, "i16"),
            Ty::Int(IntTy::Int32) => write!(f, "i32"),
            Ty::Int(IntTy::Int64) => write!(f, "i64"),
            Ty::Nullable(inner) => write!(f, "{inner}?"),
            Ty::Array(inner) => write!(f, "[{inner}]"),
            Ty::Tuple(tys) => match &tys[..] {
                [] => write!(f, "()"),
                [first, rest @ ..] => {
                    write!(f, "({first}")?;
                    for ty in rest {
                        write!(f, ", {ty}")?;
                    }
                    write!(f, ")")
                }
            },
            Ty::Func(FuncTy { params, ret }) => match &params[..] {
                [] => write!(f, "() -> {ret}"),
                [arg] => write!(f, "{arg} -> {ret}"),
                [first, rest @ ..] => {
                    write!(f, "({first}")?;
                    for ty in rest {
                        write!(f, ", {ty}")?;
                    }
                    write!(f, ") -> {ret}")
                }
            },
            Ty::Param(id) => write!(f, "${}", id.0),
            Ty::Unknown => write!(f, "unknown"),
            Ty::Infer => write!(f, "_"),
            Ty::Err => write!(f, "err"),
        }
    }
}

impl Debug for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // ---- helpers to cut down on noise ----

    fn int(i: IntTy) -> Ty {
        Ty::Int(i)
    }

    fn i8_() -> Ty {
        Ty::Int(IntTy::Int8)
    }
    fn i32_() -> Ty {
        Ty::Int(IntTy::Int32)
    }
    fn i64_() -> Ty {
        Ty::Int(IntTy::Int64)
    }

    fn arr(ty: Ty) -> Ty {
        Ty::Array(ty.into())
    }

    fn func(params: Vec<Ty>, ret: Ty) -> Ty {
        Ty::Func(FuncTy {
            params,
            ret: ret.into(),
        })
    }

    fn param(id: u32) -> Ty {
        Ty::Param(ParamId(id))
    }

    /// A lambda whose body computes its return type from its (single) parameter
    /// type via `body`, and which reports `sink` in its parameter position
    /// (because the parameter is unannotated — it is not the authority on its
    /// own type). This mirrors how an unannotated lambda behaves: checked
    /// against whatever expected type flows down, but reporting `sink` upward.
    fn unannotated_lambda(ret: Ty) -> Box<dyn Expr> {
        Box::new(Mock(func(vec![Ty::Infer], ret)))
    }

    /// A lambda with an *annotated* parameter of type `annot`. It checks its
    /// body against `annot` (ignoring the expected parameter type that flows
    /// down) and reports `annot` in its parameter position — because an
    /// annotation IS an authoritative statement about the parameter type.
    fn annotated_lambda(annot: Ty, ret: Ty) -> Box<dyn Expr> {
        Box::new(Mock(func(vec![annot], ret)))
    }

    // ============================================================
    // Case 1 — baseline map (your original, kept for completeness)
    // ============================================================
    #[test]
    fn test_case1_baseline_map() {
        let (t, u) = (0, 1);
        let func_decl = FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: vec![ParamId(t), ParamId(u)],
        };
        let args = vec![Box::new(Mock(arr(i32_()))), unannotated_lambda(i32_())];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, arr(i32_()));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 2 — empty producer: map(never[], y => y + 1) -> never[]
    // ============================================================
    #[test]
    fn test_case2_empty_producer() {
        let (t, u) = (0, 1);
        let func_decl = FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: vec![ParamId(t), ParamId(u)],
        };
        let args = vec![
            Box::new(Mock(arr(Ty::Never))),
            unannotated_lambda(Ty::Never),
        ];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        // T = never (genuine producer contribution), U = never, result never[]
        assert_eq!(result, arr(Ty::Never));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 3 — two producers join: zip(i32[], i64[]) -> (i64, i64)[]
    // ============================================================
    #[test]
    fn test_case3_two_producers_join() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![arr(param(t)), arr(param(t))],
            ret: arr(Ty::Tuple(vec![param(t), param(t)])),
            type_params: vec![ParamId(t)],
        };
        let args: Vec<Box<dyn Expr>> =
            vec![Box::new(Mock(arr(i32_()))), Box::new(Mock(arr(i64_())))];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        // join(i32, i64) = i64
        assert_eq!(result, arr(Ty::Tuple(vec![i64_(), i64_()])));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 4 — two consumers + producer:
    //   both(a => a > 0, b => b > 0, 5i32) -> bool
    // T produced by x:i32, consumed by both lambdas. Requires the loop to
    // recheck the lambdas after the producer raises lower(T) to i32.
    // ============================================================
    #[test]
    fn test_case4_two_consumers_with_producer() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
                param(t),
            ],
            ret: Ty::Bool,
            type_params: vec![ParamId(t)],
        };
        let args = vec![
            unannotated_lambda(Ty::Bool),
            unannotated_lambda(Ty::Bool),
            Box::new(Mock(i32_())), // x : i32, the producer
        ];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        // T resolves from the producer; return is Bool regardless.
        assert_eq!(result, Ty::Bool);
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 5a — foo(42i32, y => y > 0) with NO annotation -> AMBIGUOUS
    //   foo<T>(x: T, inner: T -> bool) -> (T -> int)
    // Producer gives lower(T)=i32; expected type is sink and T is contravariant
    // in the result, so sink flows into lower(T) too -> join(i32, sink) = sink.
    // Result is sink -> int, sink reaches root -> ambiguity.
    // ============================================================
    #[test]
    fn test_case5a_producer_vs_consumer_ambiguous() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![param(t), func(vec![param(t)], Ty::Bool)],
            ret: func(vec![param(t)], int(IntTy::Int32)),
            type_params: vec![ParamId(t)],
        };
        let args = vec![Box::new(Mock(i32_())), unannotated_lambda(Ty::Bool)];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap_err();
        assert!(result.contains("annotation required"))
    }

    // ============================================================
    // Case 5b — SAME call but annotated: let h: i32 -> int = foo(...)
    // The expected type i32 -> int seeds lower(T) = i32 with a REAL type.
    // Return T -> int, T contravariant, reads lower(T) = i32 -> i32 -> int.
    // ============================================================
    #[test]
    fn test_case5b_producer_with_annotation_resolves() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![param(t), func(vec![param(t)], Ty::Bool)],
            ret: func(vec![param(t)], int(IntTy::Int32)),
            type_params: vec![ParamId(t)],
        };
        let args = vec![Box::new(Mock(i32_())), unannotated_lambda(Ty::Bool)];
        let expected = func(vec![i32_()], int(IntTy::Int32));
        let result = check_func_call_outer(func_decl, args, &expected).unwrap();
        let result = reconcile(&expected, &result).unwrap();
        assert_eq!(result, func(vec![i32_()], int(IntTy::Int32)));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 6 — foo(y => y > 0) bare, no producer, no annotation -> AMBIGUOUS
    //   foo<T>(inner: T -> bool) -> (T -> int)
    // ============================================================
    #[test]
    fn test_case6_unconstrained_contravariant_ambiguous() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![func(vec![param(t)], Ty::Bool)],
            ret: func(vec![param(t)], int(IntTy::Int32)),
            type_params: vec![ParamId(t)],
        };
        let args = vec![unannotated_lambda(Ty::Bool)];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap_err();
        assert!(result.contains("annotation required"))
    }

    // ============================================================
    // Case 7 — both((a: i32) => a > 0, b => b > 0) -> bool
    //   both<T>(f: T -> bool, g: T -> bool) -> bool
    // Annotated f writes upper(T)=i32; unannotated g meets sink into upper ->
    // upper(T)=sink. No producer, no annotation -> lower(T)=sink. T is absent
    // from the Bool return, so the unresolved bounds don't matter: -> bool.
    // ============================================================
    #[test]
    fn test_case7_annotated_and_unannotated_consumers() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
            ],
            ret: Ty::Bool,
            type_params: vec![ParamId(t)],
        };
        let args = vec![
            annotated_lambda(i32_(), Ty::Bool),
            unannotated_lambda(Ty::Bool),
        ];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        // T is bivariant/absent in the Bool return — result is Bool regardless.
        assert_eq!(result, Ty::Bool);
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 8 — your simplified version, annotated:
    //   both<T>(a: T -> bool, b: T -> bool) -> (T -> bool)
    //   let h: i32 -> bool = both(a => a > 0, b => b < 10)
    // Expected i32 -> bool seeds lower(T)=i32 (contravariant flip). Lambdas
    // checked at i32, report sink into upper. Return T -> bool, T contravariant,
    // reads lower(T)=i32 -> i32 -> bool.
    // ============================================================
    #[test]
    fn test_case8_simplified_both_annotated() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
            ],
            ret: func(vec![param(t)], Ty::Bool),
            type_params: vec![ParamId(t)],
        };
        let args = vec![unannotated_lambda(Ty::Bool), unannotated_lambda(Ty::Bool)];
        let expected = func(vec![i32_()], Ty::Bool);
        let result = check_func_call_outer(func_decl, args, &expected).unwrap();
        let result = reconcile(&expected, &result).unwrap();
        assert_eq!(result, func(vec![i32_()], Ty::Bool));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 8-unannotated — same but no annotation -> AMBIGUOUS
    //   let h = both(a => a > 0, b => b < 10)
    // lower(T) seeded sink by the sink expected type (contravariant), upper(T)
    // sink from consumers -> (sink, sink) -> result sink -> bool -> ambiguity.
    // ============================================================
    #[test]
    fn test_case8_simplified_both_unannotated_ambiguous() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
            ],
            ret: func(vec![param(t)], Ty::Bool),
            type_params: vec![ParamId(t)],
        };
        let args = vec![unannotated_lambda(Ty::Bool), unannotated_lambda(Ty::Bool)];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap_err();
        assert!(result.contains("annotation required"));
    }

    // ============================================================
    // Case 9 — dependency chain:
    //   foo<T, U, V>(a: (T, U -> V), b: T -> U) -> V
    // Needs multiple passes: T from a.0, U from b (using T), V from a.1
    // (using U). I've made the arguments concrete enough to drive this:
    //   a = (i32, <lambda U -> V computing V from U>)
    //   b = <lambda T -> U computing U from T>
    // ----- SEE QUESTION BELOW: this one I'm least sure about. -----
    // ============================================================
    #[test]
    fn test_case9_dependency_chain() {
        let (t, u, v) = (0, 1, 2);
        let func_decl = FuncDecl {
            params: vec![
                Ty::Tuple(vec![param(t), func(vec![param(u)], param(v))]),
                func(vec![param(t)], param(u)),
            ],
            ret: param(v),
            type_params: vec![ParamId(t), ParamId(u), ParamId(v)],
        };
        let args: Vec<Box<dyn Expr>> = vec![
            // a : (i32, (U -> V))  where the inner lambda maps its param type
            // through unchanged (U -> U), so V = U.
            Box::new(Mock(Ty::Tuple(vec![i32_(), func(vec![Ty::Infer], i32_())]))),
            // b : T -> U, body maps T -> T (U = T)
            unannotated_lambda(i32_()),
        ];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        // T = i32 (from a.0); U = T = i32 (from b); V = U = i32 (from a.1).
        assert_eq!(result, i32_());
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 10 — bar(x => x, y => y) produces never type
    //   bar<T>(f: T -> T, g: T -> T) -> T
    // ============================================================
    #[test]
    fn test_case10_cycle_ambiguous() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![
                func(vec![param(t)], param(t)),
                func(vec![param(t)], param(t)),
            ],
            ret: param(t),
            type_params: vec![ParamId(t)],
        };
        // identity lambdas: body returns its parameter type unchanged.
        let ident = || unannotated_lambda(Ty::Never);
        let args = vec![ident(), ident()];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, Ty::Never);
    }

    // ============================================================
    // Case 11 — nested map:
    //   map(map(xs: i32[], a => a > 0), b => if b then 1 else 0)
    // Inner map resolves to bool[]; outer T=bool, U=i32 -> i32[].
    // Modelled by making the first argument of the OUTER map a closure that
    // itself runs check_func_call for the inner map.
    // ============================================================
    #[test]
    fn test_case11_nested_map() {
        let (t, u) = (0, 1);
        let map_decl = FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: vec![ParamId(t), ParamId(u)],
        };

        let (t, u) = (2, 3);
        let inner_decl = FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: vec![ParamId(t), ParamId(u)],
        };

        let outer_args = vec![
            Box::new(FuncCall {
                func: inner_decl,
                args: vec![Box::new(Mock(arr(i32_()))), unannotated_lambda(Ty::Bool)],
            }),
            // b => if b then 1 else 0 : produces i32 regardless of (bool) input
            unannotated_lambda(i32_()),
        ];
        let result = check_func_call_outer(map_decl, outer_args, &Ty::Infer).unwrap();
        assert_eq!(result, arr(i32_()));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 12 — buried lambda:
    //   map(xs: i32[], identity(y => y + 1))
    //   identity<A>(x: A) -> A
    // The lambda is wrapped in identity. The sink in identity's A must
    // propagate up so map fills it from xs.
    // ============================================================
    #[test]
    fn test_case12_buried_lambda() {
        let (t, u) = (0, 1);
        let map_decl = FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: vec![ParamId(t), ParamId(u)],
        };

        let a = 2;
        let identity_decl = FuncDecl {
            params: vec![param(a)],
            ret: param(a),
            type_params: vec![ParamId(a)],
        };

        let args: Vec<Box<dyn Expr>> = vec![
            Box::new(Mock(arr(i32_()))),
            Box::new(FuncCall {
                func: identity_decl,
                args: vec![unannotated_lambda(i32_())],
            }),
        ];
        let result = check_func_call_outer(map_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, arr(i32_()));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 13 — nullable: map(xs: i32?[], y => y) -> i32?[]
    // ============================================================
    #[test]
    fn test_case13_nullable() {
        let (t, u) = (0, 1);
        let func_decl = FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: vec![ParamId(t), ParamId(u)],
        };
        let nullable_i32 = Ty::Nullable(i32_().into());
        let args = vec![
            Box::new(Mock(arr(nullable_i32.clone()))),
            unannotated_lambda(nullable_i32.clone()),
        ];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, arr(nullable_i32));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 14 — subtype widening through producers:
    //   sink_fn<T>(x: T, y: T) -> T ; sink_fn(i8, i32) -> i32
    // ============================================================
    #[test]
    fn test_case14_producer_widening() {
        let t = 0;
        let func_decl = FuncDecl {
            params: vec![param(t), param(t)],
            ret: param(t),
            type_params: vec![ParamId(t)],
        };
        let args: Vec<Box<dyn Expr>> = vec![Box::new(Mock(i8_())), Box::new(Mock(i32_()))];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, i32_()); // join(i8, i32)
        assert!(!contains_sink(&result));
    }

    #[test]
    fn test_subtype() {
        let func_decl = FuncDecl {
            params: vec![i64_()],
            ret: Ty::Bool,
            type_params: vec![],
        };
        let args: Vec<Box<dyn Expr>> = vec![Box::new(Mock(i32_()))];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, Ty::Bool);
    }

    #[test]
    fn test_not_subtype() {
        let func_decl = FuncDecl {
            params: vec![i32_()],
            ret: Ty::Bool,
            type_params: vec![],
        };
        let args: Vec<Box<dyn Expr>> = vec![Box::new(Mock(i64_()))];
        let result = check_func_call_outer(func_decl, args, &Ty::Infer);
        assert!(result.is_err());
    }

    fn map_decl(t: ParamId, u: ParamId) -> FuncDecl {
        FuncDecl {
            params: vec![
                Ty::Array(Ty::Param(t).into()),
                Ty::Func(FuncTy {
                    params: vec![Ty::Param(t)],
                    ret: Ty::Param(u).into(),
                }),
            ],
            ret: Ty::Array(Ty::Param(u).into()),
            type_params: vec![t, u],
        }
    }

    // (A) Nested generic calls share one InferCtx; inner result propagates to outer lower bound.
    // Distinct params (inner 0,1 / outer 2,3) — correct under global-uniqueness. Green.
    #[test]
    fn test_nested_maps_distinct_params() {
        let inner = FuncCall {
            func: map_decl(ParamId(0), ParamId(1)),
            args: vec![
                Box::new(Mock(Ty::Array(Ty::Int(IntTy::Int32).into()))),
                Box::new(Mock(Ty::Func(FuncTy {
                    params: vec![Ty::Infer],
                    ret: Ty::Bool.into(),
                }))), // a => a > 0, mocked as _ -> bool
            ],
        };
        let outer_func = map_decl(ParamId(2), ParamId(3));
        let outer_args: Vec<Box<dyn Expr>> = vec![
            Box::new(inner), // outer's producer is the inner map call
            Box::new(Mock(Ty::Func(FuncTy {
                params: vec![Ty::Infer],
                ret: Ty::Int(IntTy::Int32).into(),
            }))), // b => ..., mocked as _ -> i32
        ];

        let result = check_func_call_outer(outer_func, outer_args, &Ty::Infer).unwrap();
        assert_eq!(result, Ty::Array(Ty::Int(IntTy::Int32).into()));
    }

    // (B) Aliasing tripwire: inner and outer both use params (0,1), shared ctx.
    // EXPECTED TO FAIL until generic params are freshly instantiated per call site.
    // Carries the global-uniqueness assumption across the upcoming rewrite as an
    // executable note. Asserts only that the CORRECT answer is produced; under a
    // flat ParamId->bounds map the shared slot corrupts and this won't hold.
    #[test]
    fn test_aliasing_tripwire() {
        let inner = FuncCall {
            func: map_decl(ParamId(0), ParamId(1)),
            args: vec![
                Box::new(Mock(Ty::Array(Ty::Int(IntTy::Int32).into()))),
                Box::new(Mock(Ty::Func(FuncTy {
                    params: vec![Ty::Infer],
                    ret: Ty::Bool.into(),
                }))),
            ],
        };
        let outer_func = map_decl(ParamId(2), ParamId(3)); // SAME ids as inner — the bug
        let outer_args: Vec<Box<dyn Expr>> = vec![
            Box::new(inner),
            Box::new(Mock(Ty::Func(FuncTy {
                params: vec![Ty::Infer],
                ret: Ty::Int(IntTy::Int32).into(),
            }))),
        ];

        let result = check_func_call_outer(outer_func, outer_args, &Ty::Infer);
        assert_eq!(result.unwrap(), Ty::Array(Ty::Int(IntTy::Int32).into()));
    }
}
