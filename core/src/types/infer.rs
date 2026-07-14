use std::fmt::Debug;

use itertools::Itertools;
use thiserror::Error;

use crate::types::ty::{FuncTy, ParamId, VarId};
use crate::types::ty_ctx::Variance;
use crate::util::{Args, Elements};

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

/// Represents the infered bounds of a type parameter
#[derive(Clone, Copy, Debug)]
pub struct TyBounds<'ty> {
    pub lower: Ty<'ty>,
    pub upper: Ty<'ty>,
}

/// The polarity of a type bound
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Lower,
    Upper,
}

#[derive(Error, Clone, PartialEq, Eq, Debug)]
pub struct TypeError<'ty> {
    kind: TypeErrorKind<'ty>,
    causes: Vec<TypeError<'ty>>,
}

pub type TyResult<'ty> = std::result::Result<Ty<'ty>, TypeError<'ty>>;

#[derive(Error, Clone, PartialEq, Eq, Debug)]
pub enum TypeErrorKind<'ty> {
    #[error("Cannot infer a type here")]
    Ambiguous,
    #[error("'{act}' is not assignable to '{exp}'")]
    Unassignable { act: Ty<'ty>, exp: Ty<'ty> },
    #[error("expected {exp}, but got {act}")]
    Arity { exp: Args, act: Args },
    #[error("expected {exp}, but got {act}")]
    TupleLength { exp: Elements, act: Elements },
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
            (Int(_) | UInt(_), Integer) => lhs,
            (Integer, Int(_) | UInt(_)) => rhs,
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
            (Int(_) | UInt(_), Integer) => self.common().int,
            (Integer, Int(_) | UInt(_)) => self.common().int,
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

    /// Instantiates type arguments with inference variables,
    /// and returns the number of inference variables produced
    pub fn instantiate(self, params: &mut [Ty<'ty>]) -> usize {
        let mut var_cnt = 0;
        for param in params {
            *param = self.transform(*param, &mut |ty| match ty.kind() {
                TyKind::Infer => {
                    let ty_var = self.mk_var(VarId(var_cnt));
                    var_cnt += 1;
                    ty_var
                }
                _ => ty,
            });
        }
        var_cnt as usize
    }

    /// Substitutes occurances of type parameters in the type with their instantiations
    pub fn substitute_params(self, ty: Ty<'ty>, params: &[Ty<'ty>]) -> Ty<'ty> {
        self.transform(ty, &mut |ty| match ty.kind() {
            TyKind::Param(idx) => params[idx.0 as usize],
            _ => ty,
        })
    }

    /// Substitutes occurances of type variables with their current upper or lower bound
    fn substitute_vars(self, ty: Ty<'ty>, bounds: &[TyBounds<'ty>], bound: Bound) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, variance| match ty.kind() {
            TyKind::Param(idx) => match (bound, variance) {
                (Bound::Upper, Variance::Co) => bounds[idx.0 as usize].upper,
                (Bound::Lower, Variance::Contra) => bounds[idx.0 as usize].upper,
                (Bound::Lower, Variance::Co) => bounds[idx.0 as usize].lower,
                (Bound::Upper, Variance::Contra) => bounds[idx.0 as usize].lower,
            },
            _ => ty,
        })
    }

    /// Creates an empty pair of type bounds
    pub fn new_bounds(self) -> TyBounds<'ty> {
        TyBounds { lower: self.common().never, upper: self.common().any }
    }

    /// Compares the expected type `exp` against the provided type `act`,
    /// extracts the resulting type parameter bounds, and applies them to `bounds`.
    /// This does not check that `act <: exp`; it is only used for accumulating bounds.
    pub fn update_bounds(&self, act: Ty<'ty>, exp: Ty<'ty>, bounds: &mut [TyBounds<'ty>]) {
        use TyKind::*;

        match (act.kind(), exp.kind()) {
            (Var(id), _) => {
                let bound = &mut bounds[id.0 as usize].upper;
                *bound = self.meet(*bound, exp);
            }
            (_, Var(id)) => {
                let bound = &mut bounds[id.0 as usize].lower;
                *bound = self.join(*bound, act);
            }

            (Error(_) | Pending | Infer, _) => match exp.kind() {
                Nullable(exp) => self.update_bounds(act, exp, bounds),
                Array(exp) => self.update_bounds(act, exp, bounds),
                Tuple(exp) => {
                    for &exp in exp.iter() {
                        self.update_bounds(act, exp, bounds);
                    }
                }
                Func(exp) => {
                    for &exp in exp.params.iter() {
                        self.update_bounds(exp, act, bounds); // Flip variance
                    }
                    self.update_bounds(act, exp.ret, bounds);
                }
                _ => {}
            },
            (_, Error(_) | Pending | Infer) => match act.kind() {
                Nullable(act) => self.update_bounds(act, exp, bounds),
                Array(act) => self.update_bounds(act, exp, bounds),
                Tuple(act) => {
                    for &act in act.iter() {
                        self.update_bounds(act, exp, bounds);
                    }
                }
                Func(act) => {
                    for &act in act.params.iter() {
                        self.update_bounds(exp, act, bounds); // Flip variance
                    }
                    self.update_bounds(act.ret, exp, bounds);
                }
                _ => {}
            },

            (Nullable(act), Nullable(exp)) => self.update_bounds(act, exp, bounds),
            (Array(act), Array(exp)) => self.update_bounds(act, exp, bounds),
            (Tuple(act), Tuple(exp)) if act.len() == exp.len() => {
                for (&act, &exp) in act.iter().zip(exp.iter()) {
                    self.update_bounds(act, exp, bounds);
                }
            }
            (Func(act), Func(exp)) if act.params.len() == exp.params.len() => {
                for (&act, &exp) in act.params.iter().zip(exp.params.iter()) {
                    self.update_bounds(exp, act, bounds); // Flip variance
                }
                self.update_bounds(act.ret, exp.ret, bounds);
            }

            _ => {}
        }
    }

    /// Reconciles the expected type `exp` against the provided type `act`,
    /// returning the resolved expected type, or an error if the types are incompatible or ambiguous
    pub fn reconcile(self, act: Ty<'ty>, exp: Ty<'ty>) -> TyResult<'ty> {
        // FIXME: is "expected" type always the best choice?
        use TyKind::*;

        match (act.kind(), exp.kind()) {
            // Sentinal values
            (Error(_) | Pending, _) => Ok(act),
            (_, Error(_) | Pending) => Ok(exp),

            // Equality
            (Infer, Infer) => Err(TypeError::ambiguous()),
            _ if act == exp => Ok(act),

            // Type inference
            (Infer, _) => {
                if exp.has_infer() {
                    Err(TypeError::ambiguous())
                } else {
                    Ok(exp)
                }
            }
            (_, Infer) => {
                if act.has_infer() {
                    Err(TypeError::ambiguous())
                } else {
                    Ok(act)
                }
            }

            // Structural decomposition
            (Nullable(act_in), Nullable(exp_in)) => {
                let result = self
                    .reconcile(act_in, exp_in)
                    .map_err(|err| TypeError::unassignable(act, exp).with_cause(err))?;
                Ok(self.mk_nullable(result))
            }
            (act_in, Nullable(exp_in)) => {
                let result = self
                    .reconcile(act, exp_in)
                    .map_err(|err| TypeError::unassignable(act, exp).with_cause(err))?;
                Ok(self.mk_nullable(result))
            }
            (Array(act_in), Array(exp_in)) => {
                let result = self
                    .reconcile(act_in, exp_in)
                    .map_err(|err| TypeError::unassignable(act, exp).with_cause(err))?;
                Ok(self.mk_array(result))
            }
            (Tuple(act_in), Tuple(exp_in)) => {
                check_tuple_len(act_in.len(), exp_in.len())?;
                let (tys, errors): (Vec<_>, Vec<_>) = act_in
                    .iter()
                    .zip(exp_in.iter())
                    .map(|(&act, &exp)| self.reconcile(act, exp))
                    .partition_result();
                if errors.is_empty() {
                    Ok(self.mk_tuple(&tys))
                } else {
                    Err(TypeError::unassignable(act, exp).with_causes(errors))
                }
            }
            (Func(act_in), Func(exp_in)) => {
                check_arity(act_in.params.len(), exp_in.params.len())?;
                let (params, mut errors): (Vec<_>, Vec<_>) = act_in
                    .params
                    .iter()
                    .zip(exp_in.params.iter())
                    .map(|(&act, &exp)| self.reconcile(exp, act)) // Flip variance
                    .partition_result();
                let ret = match self.reconcile(act_in.ret, exp_in.ret) {
                    Ok(ret) => ret,
                    Err(err) => {
                        errors.push(err);
                        self.common().never
                    }
                };
                if errors.is_empty() {
                    Ok(self.mk_func(&params, ret))
                } else {
                    Err(TypeError::unassignable(act, exp).with_causes(errors))
                }
            }

            // Scalar types
            (Int(act_in), Int(exp_in)) if act_in <= exp_in => Ok(exp),
            (UInt(act_in), UInt(exp_in)) if act_in <= exp_in => Ok(exp),
            (Float(act_in), Float(exp_in)) if act_in <= exp_in => Ok(exp),

            // Top and bottom types
            (Never, _) | (_, Any) => Ok(exp),

            // Type mismatch
            _ => Err(TypeError::unassignable(act, exp)),
        }
    }
}

pub fn check_tuple_len(expected: usize, actual: usize) -> Result<(), TypeError<'static>> {
    if expected == actual {
        Ok(())
    } else {
        Err(TypeError::tuple_length(expected, actual))
    }
}

pub fn check_arity(expected: usize, actual: usize) -> Result<(), TypeError<'static>> {
    if expected == actual {
        Ok(())
    } else {
        Err(TypeError::arity(expected, actual))
    }
}

impl<'ty> TypeError<'ty> {
    fn new(kind: TypeErrorKind<'ty>) -> Self {
        Self { kind, causes: vec![] }
    }

    fn ambiguous() -> Self {
        Self::new(TypeErrorKind::Ambiguous)
    }

    fn unassignable(act: Ty<'ty>, exp: Ty<'ty>) -> Self {
        Self::new(TypeErrorKind::Unassignable { act, exp })
    }

    fn tuple_length(act: usize, exp: usize) -> Self {
        let exp = Elements(exp.try_into().unwrap());
        let act = Elements(act.try_into().unwrap());
        Self::new(TypeErrorKind::TupleLength { act, exp })
    }

    fn arity(act: usize, exp: usize) -> Self {
        let exp = Args(exp.try_into().unwrap());
        let act = Args(act.try_into().unwrap());
        Self::new(TypeErrorKind::Arity { act, exp })
    }

    fn with_cause(mut self, cause: Self) -> Self {
        self.causes.push(cause);
        self
    }

    fn with_causes(mut self, mut causes: Vec<Self>) -> Self {
        self.causes.append(&mut causes);
        self
    }
}

impl<'ty> std::fmt::Display for TypeError<'ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        if f.alternate() {
            for cause in &self.causes {
                let indented = format!("{:#}", cause)
                    .lines()
                    .map(|line| format!("    {line}"))
                    .join("\n");
                write!(f, "\n{indented}")?;
            }
        }
        Ok(())
    }
}

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

    #[test]
    fn test_cause_chain() {
        with_ctx(|ctx| {
            let common = ctx.common();
            let actual = ctx.mk_nullable(ctx.mk_array(common.int32));
            let expected = ctx.mk_nullable(ctx.mk_array(common.str));
            let result = ctx.reconcile(actual, expected);

            // Output should resemble:
            // 'int32[]?' is not assignable to 'str[]?'
            //     'int32[]' is not assignable to 'str[]'
            //         'int32' is not assignable to 'str'
            let err_message = format!("{:#}", result.unwrap_err());
            assert_eq!(err_message.lines().count(), 3);
        });
    }

    #[test]
    fn test_cause_chain_tuple() {
        with_ctx(|ctx| {
            let common = ctx.common();
            let actual = ctx.mk_tuple(&[common.int32, common.bool]);
            let expected = ctx.mk_tuple(&[common.int32, common.str]);
            let result = ctx.reconcile(actual, expected);

            // Output should resemble:
            // '(int32, bool)' is not assignable to '(int32, str)'
            //     'bool' is not assignable to 'str'
            let err_message = format!("{:#}", result.unwrap_err());
            assert_eq!(err_message.lines().count(), 2);
        });
    }

    #[test]
    fn test_cause_chain_func() {
        with_ctx(|ctx| {
            let common = ctx.common();
            let actual = ctx.mk_func(&[common.int32, common.bool], common.str);
            let expected = ctx.mk_func(&[common.int32, common.str], common.float32);
            let result = ctx.reconcile(actual, expected);

            // Output should resemble:
            // '(int32, bool) -> str' is not assignable to '(int32, str) -> float32'
            //     'str' is not assignable to 'bool'
            //     'str' is not assignable to 'float32'
            let err_message = format!("{:#}", result.unwrap_err());
            assert_eq!(err_message.lines().count(), 3);
        });
    }

    #[test]
    fn test_subtyping() {
        with_ctx(|ctx| {
            let common = ctx.common();
            let actual = ctx.mk_tuple(&[common.int32, common.bool, common.never]);
            let expected = ctx.mk_tuple(&[common.int64, ctx.mk_nullable(common.bool), common.str]);
            ctx.reconcile(actual, expected).unwrap();
        });
    }
}
