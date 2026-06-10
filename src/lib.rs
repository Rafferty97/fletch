// Type

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
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
