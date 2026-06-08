// Type

use std::collections::HashMap;

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
        Ty::Array(ty) => variance_in_ty(ty, var),
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
            _ if sub == sup => {}
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
            (Ty::Array(sub), Ty::Array(sup)) => self.add_constraint(sub, sup),
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
            Ty::Array(inner) => Ty::Array(self.solve_ty_inner(inner, variances).into()),
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
                    self.errors.push(format!("no unique type"));
                    Ty::Err
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_map() {
        // map<T, U>(f: fn(T) -> U, xs: T[]) -> U[]
        // map(xs, x => x + 1) where xs: Int32[]

        let mut ctx = InferCtx::new();

        // Instantiate map's type parameters
        let t = ctx.fresh_var(); // T
        let u = ctx.fresh_var(); // U

        // map's instantiated signature
        let f_param = Ty::Func(FuncTy {
            params: vec![Ty::Var(t)],
            ret: Box::new(Ty::Var(u)),
        });
        let xs_param = Ty::Array(Box::new(Ty::Var(t)));
        let ret_ty = Ty::Array(Box::new(Ty::Var(u)));

        // Phase 1 - constrain from xs: Int32[]
        let xs_ty = Ty::Array(Box::new(Ty::Int(IntTy::Int32)));
        ctx.add_constraint(&xs_ty, &xs_param);

        // Resolve T for lambda parameter
        let t_ty = ctx.solve_var(t, &ret_ty);
        assert_eq!(t_ty, Ty::Int(IntTy::Int32));

        // Phase 2 - lambda body x + 1 where x: T produces Int32
        let body_ty = Ty::Int(IntTy::Int32);
        ctx.add_constraint(
            &Ty::Func(FuncTy {
                params: vec![t_ty],
                ret: Box::new(body_ty),
            }),
            &f_param,
        );

        // Solve return type
        let result = ctx.solve_ty(&ret_ty, &ret_ty);
        assert_eq!(result, Ty::Array(Box::new(Ty::Int(IntTy::Int32))));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn test_map_with_conflicting_annotation() {
        // map<T, U>(f: fn(T) -> U, xs: T[]) -> U[]
        // map(xs, x: Int64 => x + 1) where xs: Int32[]
        // T gets lower bound Int32 from xs, upper bound Int64 from annotation
        // Int32 <: Int64 so this is fine, T resolves to Int32

        let mut ctx = InferCtx::new();

        let t = ctx.fresh_var();
        let u = ctx.fresh_var();

        let f_param = Ty::Func(FuncTy {
            params: vec![Ty::Var(t)],
            ret: Box::new(Ty::Var(u)),
        });
        let xs_param = Ty::Array(Box::new(Ty::Var(t)));
        let ret_ty = Ty::Array(Box::new(Ty::Var(u)));

        // Phase 1 - xs: Int32[]
        let xs_ty = Ty::Array(Box::new(Ty::Int(IntTy::Int32)));
        ctx.add_constraint(&xs_ty, &xs_param);

        // Annotated lambda parameter x: Int64 contributes upper bound
        ctx.add_constraint(&Ty::Var(t), &Ty::Int(IntTy::Int64));

        // Resolve T - lower Int32, upper Int64, Int32 <: Int64, picks lower
        let t_ty = ctx.solve_var(t, &ret_ty);
        assert_eq!(t_ty, Ty::Int(IntTy::Int32));

        // Lambda body produces Int32
        ctx.add_constraint(
            &Ty::Func(FuncTy {
                params: vec![t_ty],
                ret: Box::new(Ty::Int(IntTy::Int32)),
            }),
            &f_param,
        );

        let result = ctx.solve_ty(&ret_ty, &ret_ty);
        assert_eq!(result, Ty::Array(Box::new(Ty::Int(IntTy::Int32))));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn test_map_with_incompatible_annotation() {
        // map(xs, x: Bool => ...) where xs: Int32[]
        // T gets lower bound Int32, upper bound Bool - incompatible, should error

        let mut ctx = InferCtx::new();

        let t = ctx.fresh_var();
        let u = ctx.fresh_var();

        let f_param = Ty::Func(FuncTy {
            params: vec![Ty::Var(t)],
            ret: Box::new(Ty::Var(u)),
        });
        let xs_param = Ty::Array(Box::new(Ty::Var(t)));
        let ret_ty = Ty::Array(Box::new(Ty::Var(u)));

        // Phase 1 - xs: Int32[]
        ctx.add_constraint(&Ty::Array(Box::new(Ty::Int(IntTy::Int32))), &xs_param);

        // Annotated lambda parameter x: Bool - incompatible upper bound
        ctx.add_constraint(&Ty::Var(t), &Ty::Bool);

        // T lower=Int32, upper=Bool, Int32 is not <: Bool, should return Err
        let t_ty = ctx.solve_var(t, &ret_ty);
        assert_eq!(t_ty, Ty::Err);
    }

    #[test]
    fn test_both_contravariant() {
        // both<T>(f: fn(T) -> Bool, g: fn(T) -> Bool, x: T) -> Bool
        // both(fn(x: Int32) => x > 0, fn(x: Int64) => x < 100, 42)
        // T gets upper bound Int32 from f (contravariant), upper bound Int64 from g
        // meet(Int32, Int64) = Int32, so T resolves to Int32

        let mut ctx = InferCtx::new();

        let t = ctx.fresh_var();

        let f_param = Ty::Func(FuncTy {
            params: vec![Ty::Var(t)],
            ret: Box::new(Ty::Bool),
        });
        let g_param = Ty::Func(FuncTy {
            params: vec![Ty::Var(t)],
            ret: Box::new(Ty::Bool),
        });
        let x_param = Ty::Var(t);
        let ret_ty = Ty::Bool;

        // Phase 1 - constrain from f: fn(Int32) -> Bool (contravariant - upper bound)
        ctx.add_constraint(
            &Ty::Func(FuncTy {
                params: vec![Ty::Int(IntTy::Int32)],
                ret: Box::new(Ty::Bool),
            }),
            &f_param,
        );

        // constrain from g: fn(Int64) -> Bool (contravariant - upper bound)
        ctx.add_constraint(
            &Ty::Func(FuncTy {
                params: vec![Ty::Int(IntTy::Int64)],
                ret: Box::new(Ty::Bool),
            }),
            &g_param,
        );

        // constrain from x: Int32 (lower bound)
        ctx.add_constraint(&Ty::Int(IntTy::Int32), &x_param);

        // T in ret_ty Bool is Bivariant (doesn't appear), so defaults to lower bound
        // But T's upper bound is meet(Int32, Int64) = Int32
        // lower = Int32, upper = Int32, resolves to Int32
        let t_ty = ctx.solve_var(t, &ret_ty);
        assert_eq!(t_ty, Ty::Int(IntTy::Int32));

        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn test_invariant_position() {
        // identity<T>(x: T) -> T
        // T appears in both covariant (return) and contravariant (parameter) positions
        // With a concrete argument, bounds should coincide

        let mut ctx = InferCtx::new();

        let t = ctx.fresh_var();

        let x_param = Ty::Var(t);
        let ret_ty = Ty::Var(t);

        // Constrain from x: Int32
        ctx.add_constraint(&Ty::Int(IntTy::Int32), &x_param);

        // T appears in both positions in ret_ty Var(t)
        // variance_in_ty(Var(t), t) = Covariant
        // But t also appears as a parameter so overall it's invariant
        // lower = Int32, upper = Top
        // With covariant solve picks lower = Int32
        let result = ctx.solve_ty(&ret_ty, &ret_ty);
        assert_eq!(result, Ty::Int(IntTy::Int32));

        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn test_empty_array() {
        // map<T, U>(f: fn(T) -> U, xs: T[]) -> U[]
        // map([], x => x + 1)
        // T unconstrained, should solve to Never

        let mut ctx = InferCtx::new();

        let t = ctx.fresh_var();
        let u = ctx.fresh_var();

        let f_param = Ty::Func(FuncTy {
            params: vec![Ty::Var(t)],
            ret: Box::new(Ty::Var(u)),
        });
        let xs_param = Ty::Array(Box::new(Ty::Var(t)));
        let ret_ty = Ty::Array(Box::new(Ty::Var(u)));

        // Phase 1 - xs: Never[] (empty array)
        ctx.add_constraint(&Ty::Array(Box::new(Ty::Never)), &xs_param);

        // T lower = Never, upper = Top, resolves to Never
        let t_ty = ctx.solve_var(t, &ret_ty);
        assert_eq!(t_ty, Ty::Never);

        // Lambda body with Never parameter produces Never
        ctx.add_constraint(
            &Ty::Func(FuncTy {
                params: vec![t_ty],
                ret: Box::new(Ty::Never),
            }),
            &f_param,
        );

        let result = ctx.solve_ty(&ret_ty, &ret_ty);
        assert_eq!(result, Ty::Array(Box::new(Ty::Never)));
        assert!(ctx.errors.is_empty());
    }
}
