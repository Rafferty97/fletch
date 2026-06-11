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
        Ty::Array(p) => update_bounds_infer(bounds, p, is_upper),
        _ => todo!(),
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

    #[test]
    fn test_case1() {
        let (t, u) = (ParamId(0), ParamId(1));

        let func = &FuncDecl {
            params: vec![
                Ty::Array(Ty::Param(t).into()),
                Ty::Func(FuncTy {
                    params: vec![Ty::Param(t)],
                    ret: Ty::Param(u).into(),
                }),
            ],
            ret: Ty::Array(Ty::Param(u).into()),
            type_params: 2,
        };

        let args: &[Box<dyn Fn(&Ty) -> Result<Ty, String>>] = &[
            Box::new(|_| Ok(Ty::Array(Ty::Int(IntTy::Int32).into()))),
            Box::new(|ty| {
                let Ty::Func(FuncTy { params, .. }) = ty else {
                    Err(format!("{ty:?} is not a function!"))?
                };
                let [ty] = &params[..] else {
                    Err(format!("expected 1 argument, got {}", params.len()))?
                };
                let ret = match ty {
                    Ty::Never => Ty::Never,
                    Ty::Int(i) => Ty::Int(*i),
                    _ => Err(format!("{ty:?} is not a number!"))?,
                };
                Ok(Ty::Func(FuncTy {
                    params: vec![Ty::Infer],
                    ret: ret.into(),
                }))
            }),
        ];

        let expected = &Ty::Infer;

        let result = check_func_call(func, args, expected).unwrap();
        assert_eq!(result, Ty::Array(Ty::Int(IntTy::Int32).into()));
    }
}
