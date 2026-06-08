// Type

use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum Ty {
    Never,
    Bool,
    Int(IntTy),
    Nullable(Box<Ty>),
    Tuple(Vec<Ty>),
    Func(FuncTy),
    Param(ParamId),
    Var(VarId),
    Top,
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

pub fn variance_in_ty(ty: &Ty, var: VarId) -> Variance {
    match ty {
        Ty::Tuple(tys) => variance_in_tys(tys, var),
        Ty::Func(func) => {
            let param_variance = variance_in_tys(&func.params, var).invert();
            let ret_variance = variance_in_ty(&func.ret, var);
            Variance::merge(param_variance, ret_variance)
        }
        Ty::Var(v) if *v == var => Variance::Covariant,
        _ => Variance::Bivariant,
    }
}

pub fn variance_in_tys<'a>(tys: impl IntoIterator<Item = &'a Ty>, var: VarId) -> Variance {
    tys.into_iter()
        .map(|ty| variance_in_ty(ty, var))
        .fold(Variance::Bivariant, Variance::merge)
}

// Meet, join and check_subtype

pub fn meet(a: &Ty, b: &Ty) -> Ty {
    match (a, b) {
        _ if a == b => a.clone(),
        (Ty::Err, _) | (_, Ty::Err) => Ty::Err,
        (ty, Ty::Top) | (Ty::Top, ty) => ty.clone(),
        (Ty::Int(a), Ty::Int(b)) => Ty::Int(*a.min(b)),
        (Ty::Nullable(a), Ty::Nullable(b)) => Ty::Nullable(meet(a, b).into()),
        (a, Ty::Nullable(b)) => meet(a, b),
        (Ty::Nullable(a), b) => meet(a, b),
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
        // todo: tuple, func
        _ => Ty::Top,
    }
}

pub fn check_subtype(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        _ if a == b => true,
        (Ty::Never, _) | (_, Ty::Top) => true,
        (_, Ty::Err) | (Ty::Err, _) => true,
        (Ty::Int(a), Ty::Int(b)) => a <= b,
        (Ty::Nullable(a), Ty::Nullable(b)) => check_subtype(a, b),
        (_, Ty::Nullable(b)) => check_subtype(a, b),
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

// Inference context

#[derive(Default)]
pub struct InferCtx {
    vars: Vec<InferVar>,
    errors: Vec<String>,
}

struct InferVar {
    lower: Ty,
    upper: Ty,
}

impl InferCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fresh_var(&mut self) -> VarId {
        let id = VarId(self.vars.len() as u32);
        self.vars.push(InferVar {
            lower: Ty::Never,
            upper: Ty::Top,
        });
        id
    }

    pub fn add_constraint(&mut self, sub: &Ty, sup: &Ty) {
        // This method assumes that either `sub` or `sup` is fully concrete
        match (sub, sup) {
            (Ty::Var(sub), Ty::Var(sup)) => unreachable!(),
            (Ty::Err, _) | (_, Ty::Err) => {}
            (Ty::Var(sub), sup) => {
                let mut var = &mut self.vars[sub.0 as usize];
                var.upper = meet(&var.upper, sup);
            }
            (sub, Ty::Var(sup)) => {
                let mut var = &mut self.vars[sup.0 as usize];
                var.lower = join(&var.lower, sub);
            }
            (Ty::Never, _) | (_, Ty::Top) => {}
            (Ty::Nullable(sub), Ty::Nullable(sup)) => self.add_constraint(sub, sup),
            (sub, Ty::Nullable(sup)) => self.add_constraint(sub, sup),
            (Ty::Tuple(sub), Ty::Tuple(sup)) => {
                if sub.len() != sup.len() {
                    self.errors.push(format!(
                        "type mismatch, tuples have different lengths ({} vs {})",
                        sub.len(),
                        sup.len()
                    ));
                    return;
                }
                for (sub, sup) in sub.iter().zip(sup) {
                    self.add_constraint(sub, sup);
                }
            }
            (Ty::Func(sub), Ty::Func(sup)) => {
                if sub.params.len() != sup.params.len() {
                    self.errors.push(format!(
                        "type mismatch, functions have different arg counts ({} vs {})",
                        sub.params.len(),
                        sup.params.len()
                    ));
                    return;
                }
                for (sub, sup) in sub.params.iter().zip(&sup.params) {
                    self.add_constraint(sup, sub); // flip variance
                }
                self.add_constraint(&sub.ret, &sup.ret);
            }
            _ => {
                self.errors
                    .push(format!("type mismatch, {:?} vs {:?}", sub, sup));
            }
        }
    }

    pub fn solve_var(&mut self, var: VarId, ret_ty: &Ty) -> Ty {
        self.solve_var_with_variance(var, variance_in_ty(ret_ty, var))
    }

    pub fn solve_ty(&mut self, ty: &Ty, ret_ty: &Ty) -> Ty {
        let variances: HashMap<VarId, Variance> = (0..self.vars.len())
            .map(|i| {
                let id = VarId(i as u32);
                (id, variance_in_ty(ret_ty, id))
            })
            .collect();
        self.solve_ty_inner(ty, &variances)
    }

    fn solve_ty_inner(&mut self, ty: &Ty, variances: &HashMap<VarId, Variance>) -> Ty {
        match ty {
            Ty::Var(id) => self.solve_var_with_variance(*id, variances[id]),
            Ty::Nullable(inner) => Ty::Nullable(self.solve_ty_inner(inner, variances).into()),
            Ty::Tuple(tys) => Ty::Tuple(
                tys.iter()
                    .map(|t| self.solve_ty_inner(t, variances))
                    .collect(),
            ),
            Ty::Func(f) => Ty::Func(FuncTy {
                params: f
                    .params
                    .iter()
                    .map(|p| self.solve_ty_inner(p, variances))
                    .collect(),
                ret: self.solve_ty_inner(&f.ret, variances).into(),
            }),
            _ => ty.clone(),
        }
    }

    fn solve_var_with_variance(&mut self, var: VarId, variance: Variance) -> Ty {
        let v = &self.vars[var.0 as usize];
        if !check_subtype(&v.lower, &v.upper) {
            self.errors
                .push(format!("cannot assign {:?} to {:?}", v.lower, v.upper));
            return Ty::Err;
        }
        match variance {
            Variance::Covariant | Variance::Bivariant => v.lower.clone(),
            Variance::Contravariant => v.upper.clone(),
            Variance::Invariant => {
                if v.lower == v.upper {
                    v.lower.clone()
                } else {
                    Ty::Err
                }
            }
        }
    }
}
