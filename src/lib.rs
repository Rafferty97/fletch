// Type

use std::fmt::{Debug, Display};

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

#[derive(Clone, Debug)]
struct FuncDecl {
    params: Vec<Ty>,
    ret: Ty,
    type_params: usize,
}

fn check_func_call(
    func: &FuncDecl,
    args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>],
    expected: &Ty,
) -> Result<Ty, String> {
    fn print_bounds(bounds: &[(Ty, Ty)]) {
        print!("Bounds: ");
        for bound in bounds {
            print!("{bound:?}\t");
        }
        println!();
    }

    if args.len() != func.params.len() {
        Err(format!(
            "expected {} arguments, got {}",
            func.params.len(),
            args.len()
        ))?;
    };

    let mut bounds = vec![(Ty::Never, Ty::Unknown); func.type_params];

    update_bounds(&mut bounds, &func.ret, expected, true)?;
    print_bounds(&bounds);

    loop {
        let mut changed = false;

        for (arg, param) in args.iter().zip(&func.params) {
            let expected = substitute(param, &bounds, true);
            let actual = arg(&expected)?;
            println!("    {param}: {expected} => {actual}");
            changed |= update_bounds(&mut bounds, param, &actual, false)?;
        }
        print_bounds(&bounds);

        if !changed {
            break;
        }
    }

    Ok(substitute(&func.ret, &bounds, false))
}

fn substitute(ty: &Ty, bounds: &[(Ty, Ty)], is_upper: bool) -> Ty {
    match ty {
        Ty::Nullable(inner) => Ty::Nullable(substitute(inner, bounds, is_upper).into()),
        Ty::Array(inner) => Ty::Array(substitute(inner, bounds, is_upper).into()),
        Ty::Tuple(tys) => tys
            .iter()
            .map(|param| substitute(param, bounds, is_upper))
            .collect::<Vec<_>>()
            .pipe(Ty::Tuple),
        Ty::Func(func) => {
            let params = func
                .params
                .iter()
                .map(|param| substitute(param, bounds, !is_upper))
                .collect();
            let ret = substitute(&func.ret, bounds, is_upper).into();
            Ty::Func(FuncTy { params, ret })
        }
        Ty::Param(id) => {
            let bounds = &bounds[id.0 as usize];
            if is_upper {
                bounds.1.clone()
            } else {
                bounds.0.clone()
            }
        }
        _ => ty.clone(),
    }
}

fn update_bounds(
    bounds: &mut [(Ty, Ty)],
    param: &Ty,
    arg: &Ty,
    is_upper: bool,
) -> Result<bool, String> {
    match (param, arg) {
        // Error
        (Ty::Err, _) | (_, Ty::Err) => Ok(false),
        // Hit a type parameter - update the appropriate bound
        (Ty::Param(id), arg) => {
            let (lower, upper) = &mut bounds[id.0 as usize];
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
        (_, Ty::Infer) => Ok(update_bounds_infer(bounds, param, is_upper)),
        // Structural decomposition
        (Ty::Nullable(p), Ty::Nullable(a)) => update_bounds(bounds, p, a, is_upper),
        (p, Ty::Nullable(_)) => Err(format!(
            "cannot pass nullable value to non-nullable parameter {:?}",
            p
        )),
        (Ty::Nullable(p), _) => update_bounds(bounds, p, arg, is_upper),
        (Ty::Array(p), Ty::Array(a)) => update_bounds(bounds, p, a, is_upper),
        (Ty::Tuple(ps), Ty::Tuple(as_)) if ps.len() == as_.len() => {
            let mut changed = false;
            for (p, a) in ps.iter().zip(as_) {
                changed |= update_bounds(bounds, p, a, is_upper)?;
            }
            Ok(changed)
        }
        (Ty::Func(p), Ty::Func(a)) if p.params.len() == a.params.len() => {
            let mut changed = false;
            // Contravariant in params - flip rev
            for (pp, ap) in p.params.iter().zip(&a.params) {
                changed |= update_bounds(bounds, pp, ap, !is_upper)?;
            }
            // Covariant in return
            changed |= update_bounds(bounds, &p.ret, &a.ret, is_upper)?;
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

fn update_bounds_infer(bounds: &mut [(Ty, Ty)], param: &Ty, is_upper: bool) -> bool {
    match param {
        Ty::Param(id) => {
            let (lower, upper) = &mut bounds[id.0 as usize];
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
        Ty::Nullable(inner) => update_bounds_infer(bounds, inner, is_upper),
        Ty::Array(inner) => update_bounds_infer(bounds, inner, is_upper),
        Ty::Tuple(tys) => {
            let mut changed = false;
            for ty in tys {
                changed |= update_bounds_infer(bounds, ty, is_upper);
            }
            changed
        }
        Ty::Func(FuncTy { params, ret }) => {
            let mut changed = false;
            for ty in params {
                changed |= update_bounds_infer(bounds, ty, !is_upper);
            }
            changed |= update_bounds_infer(bounds, ret, is_upper);
            changed
        }
        _ => false,
    }
}

fn reconcile(annotation: &Ty, actual: &Ty) -> Result<Ty, String> {
    match (annotation, actual) {
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
    fn unannotated_lambda(
        body: impl Fn(&Ty) -> Result<Ty, String> + 'static,
    ) -> Box<dyn Fn(&Ty) -> Result<Ty, String>> {
        Box::new(move |ty| {
            let Ty::Func(FuncTy { params, .. }) = ty else {
                return Err(format!("{ty:?} is not a function!"));
            };
            let [pty] = &params[..] else {
                return Err(format!("expected 1 argument, got {}", params.len()));
            };
            let ret = body(pty)?;
            Ok(func(vec![Ty::Infer], ret))
        })
    }

    /// A lambda with an *annotated* parameter of type `annot`. It checks its
    /// body against `annot` (ignoring the expected parameter type that flows
    /// down) and reports `annot` in its parameter position — because an
    /// annotation IS an authoritative statement about the parameter type.
    fn annotated_lambda(
        annot: Ty,
        body: impl Fn(&Ty) -> Result<Ty, String> + 'static,
    ) -> Box<dyn Fn(&Ty) -> Result<Ty, String>> {
        Box::new(move |_expected| {
            let ret = body(&annot)?;
            Ok(func(vec![annot.clone()], ret))
        })
    }

    /// Numeric body: `x => x <op> <literal>`. Given the parameter type, returns
    /// the same numeric type (the result of e.g. `x + 1`). `Never` parameter
    /// yields `Never`; a concrete int yields that int; anything else is a type
    /// error.
    fn numeric_body(pty: &Ty) -> Result<Ty, String> {
        match pty {
            Ty::Never => Ok(Ty::Never),
            Ty::Int(i) => Ok(Ty::Int(*i)),
            // sink flowing into the body: the parameter type isn't known yet.
            // The body can't be checked, so it reports sink onward.
            Ty::Infer => Ok(Ty::Infer),
            _ => Err(format!("{pty:?} is not a number!")),
        }
    }

    /// Boolean-predicate body: `x => x <cmp> <literal>` — same parameter
    /// constraints as `numeric_body` but always returns `Bool`.
    fn predicate_body(pty: &Ty) -> Result<Ty, String> {
        match pty {
            Ty::Never => Ok(Ty::Bool),
            Ty::Int(_) => Ok(Ty::Bool),
            Ty::Infer => Ok(Ty::Bool),
            _ => Err(format!("{pty:?} is not a number!")),
        }
    }

    // ============================================================
    // Case 1 — baseline map (your original, kept for completeness)
    // ============================================================
    #[test]
    fn test_case1_baseline_map() {
        let (t, u) = (0, 1);
        let func_decl = &FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: 2,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            Box::new(|_| Ok(arr(i32_()))),
            unannotated_lambda(numeric_body),
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, arr(i32_()));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 2 — empty producer: map(never[], y => y + 1) -> never[]
    // ============================================================
    #[test]
    fn test_case2_empty_producer() {
        let (t, u) = (0, 1);
        let func_decl = &FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: 2,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            Box::new(|_| Ok(arr(Ty::Never))),
            unannotated_lambda(numeric_body),
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![arr(param(t)), arr(param(t))],
            ret: arr(Ty::Tuple(vec![param(t), param(t)])),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] =
            &[Box::new(|_| Ok(arr(i32_()))), Box::new(|_| Ok(arr(i64_())))];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
                param(t),
            ],
            ret: Ty::Bool,
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            unannotated_lambda(predicate_body),
            unannotated_lambda(predicate_body),
            Box::new(|_| Ok(i32_())), // x : i32, the producer
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![param(t), func(vec![param(t)], Ty::Bool)],
            ret: func(vec![param(t)], int(IntTy::Int32)),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] =
            &[Box::new(|_| Ok(i32_())), unannotated_lambda(predicate_body)];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        // Ambiguous: no canonical most-specific T. sink must reach the root.
        assert!(
            contains_sink(&result),
            "expected ambiguity (sink at root), got {result:?}"
        );
    }

    // ============================================================
    // Case 5b — SAME call but annotated: let h: i32 -> int = foo(...)
    // The expected type i32 -> int seeds lower(T) = i32 with a REAL type.
    // Return T -> int, T contravariant, reads lower(T) = i32 -> i32 -> int.
    // ============================================================
    #[test]
    fn test_case5b_producer_with_annotation_resolves() {
        let t = 0;
        let func_decl = &FuncDecl {
            params: vec![param(t), func(vec![param(t)], Ty::Bool)],
            ret: func(vec![param(t)], int(IntTy::Int32)),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] =
            &[Box::new(|_| Ok(i32_())), unannotated_lambda(predicate_body)];
        let expected = func(vec![i32_()], int(IntTy::Int32));
        let result = check_func_call(func_decl, args, &expected).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![func(vec![param(t)], Ty::Bool)],
            ret: func(vec![param(t)], int(IntTy::Int32)),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[unannotated_lambda(predicate_body)];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        assert!(
            contains_sink(&result),
            "expected ambiguity (sink at root), got {result:?}"
        );
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
        let func_decl = &FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
            ],
            ret: Ty::Bool,
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            annotated_lambda(i32_(), predicate_body),
            unannotated_lambda(predicate_body),
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
            ],
            ret: func(vec![param(t)], Ty::Bool),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            unannotated_lambda(predicate_body),
            unannotated_lambda(predicate_body),
        ];
        let expected = func(vec![i32_()], Ty::Bool);
        let result = check_func_call(func_decl, args, &expected).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![
                func(vec![param(t)], Ty::Bool),
                func(vec![param(t)], Ty::Bool),
            ],
            ret: func(vec![param(t)], Ty::Bool),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            unannotated_lambda(predicate_body),
            unannotated_lambda(predicate_body),
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        assert!(
            contains_sink(&result),
            "expected ambiguity (sink at root), got {result:?}"
        );
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
        let func_decl = &FuncDecl {
            params: vec![
                Ty::Tuple(vec![param(t), func(vec![param(u)], param(v))]),
                func(vec![param(t)], param(u)),
            ],
            ret: param(v),
            type_params: 3,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            // a : (i32, (U -> V))  where the inner lambda maps its param type
            // through unchanged (U -> U), so V = U.
            Box::new(|ty| {
                // expected type is a tuple (T_bound, (U_bound -> V_bound));
                // we produce (i32, (<param> -> <param>)) reflecting the inner
                // lambda's body U -> U.
                let Ty::Tuple(elems) = ty else {
                    return Err(format!("{ty:?} is not a tuple!"));
                };
                let [_t_slot, f_slot] = &elems[..] else {
                    return Err(format!("expected 2-tuple, got {}", elems.len()));
                };
                let Ty::Func(FuncTy { params, .. }) = f_slot else {
                    return Err(format!("{f_slot:?} is not a function!"));
                };
                let [u_in] = &params[..] else {
                    return Err("expected 1 param".into());
                };
                // inner lambda is U -> U: its return mirrors its (unannotated)
                // parameter, so it reports sink in the param and the body type
                // (= whatever U currently is) in the return.
                let ret = match u_in {
                    Ty::Never => Ty::Never,
                    other => other.clone(),
                };
                Ok(Ty::Tuple(vec![i32_(), func(vec![Ty::Infer], ret)]))
            }),
            // b : T -> U, body maps T -> T (U = T)
            unannotated_lambda(|pty| {
                Ok(match pty {
                    Ty::Never => Ty::Never,
                    Ty::Infer => Ty::Infer,
                    other => other.clone(),
                })
            }),
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        // T = i32 (from a.0); U = T = i32 (from b); V = U = i32 (from a.1).
        assert_eq!(result, i32_());
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 10 — cycle: bar(x => x, y => y) -> AMBIGUOUS
    //   bar<T>(f: T -> T, g: T -> T) -> T
    // ============================================================
    #[test]
    fn test_case10_cycle_ambiguous() {
        let t = 0;
        let func_decl = &FuncDecl {
            params: vec![
                func(vec![param(t)], param(t)),
                func(vec![param(t)], param(t)),
            ],
            ret: param(t),
            type_params: 1,
        };
        // identity lambdas: body returns its parameter type unchanged.
        let ident = || {
            unannotated_lambda(|pty| {
                Ok(match pty {
                    Ty::Never => Ty::Never,
                    Ty::Infer => Ty::Infer,
                    other => other.clone(),
                })
            })
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[ident(), ident()];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        assert!(
            contains_sink(&result),
            "expected ambiguity (sink at root), got {result:?}"
        );
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
            type_params: 2,
        };

        // Outer map. First arg = result of inner map(i32[], a => a>0) = bool[].
        let inner_decl = map_decl.clone(); // assumes FuncDecl: Clone — SEE Q
        let outer_args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            Box::new(move |_| {
                let inner_args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
                    Box::new(|_| Ok(arr(i32_()))),
                    unannotated_lambda(predicate_body),
                ];
                check_func_call(&inner_decl, inner_args, &Ty::Infer)
            }),
            // b => if b then 1 else 0 : produces i32 regardless of (bool) input
            unannotated_lambda(|pty| match pty {
                Ty::Never => Ok(Ty::Never),
                Ty::Bool => Ok(i32_()),
                Ty::Infer => Ok(Ty::Infer),
                other => Err(format!("{other:?} is not bool!")),
            }),
        ];
        let result = check_func_call(&map_decl, outer_args, &Ty::Infer).unwrap();
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
        let map_decl = &FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: 2,
        };

        let a = 0u32;
        let identity_decl = FuncDecl {
            params: vec![param(a)],
            ret: param(a),
            type_params: 1,
        };

        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            Box::new(|_| Ok(arr(i32_()))),
            // second arg = identity(y => y + 1): run identity's own call,
            // passing the lambda as identity's argument. The expected type
            // flowing into this position (map's f-slot, substituted) is passed
            // through to identity, and onward to the lambda.
            Box::new(move |expected| {
                let inner_args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] =
                    &[unannotated_lambda(numeric_body)];
                check_func_call(&identity_decl, inner_args, expected)
            }),
        ];
        let result = check_func_call(map_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, arr(i32_()));
        assert!(!contains_sink(&result));
    }

    // ============================================================
    // Case 13 — nullable: map(xs: i32?[], y => y) -> i32?[]
    // ============================================================
    #[test]
    fn test_case13_nullable() {
        let (t, u) = (0, 1);
        let func_decl = &FuncDecl {
            params: vec![arr(param(t)), func(vec![param(t)], param(u))],
            ret: arr(param(u)),
            type_params: 2,
        };
        let nullable_i32 = Ty::Nullable(i32_().into());
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            {
                let n = nullable_i32.clone();
                Box::new(move |_| Ok(arr(n.clone())))
            },
            // identity body: returns its parameter type unchanged
            unannotated_lambda(|pty| Ok(pty.clone())),
        ];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
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
        let func_decl = &FuncDecl {
            params: vec![param(t), param(t)],
            ret: param(t),
            type_params: 1,
        };
        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] =
            &[Box::new(|_| Ok(i8_())), Box::new(|_| Ok(i32_()))];
        let result = check_func_call(func_decl, args, &Ty::Infer).unwrap();
        assert_eq!(result, i32_()); // join(i8, i32)
        assert!(!contains_sink(&result));
    }
}
