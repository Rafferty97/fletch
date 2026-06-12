use std::collections::HashMap;
use std::hash::Hash;

use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::types::ty_ctx::Variance;

use super::ty::{Ty, TyKind};
use super::ty_ctx::TyCtx;

pub struct InferCtx<'a, 'ty> {
    ty_ctx: TyCtx<'a, 'ty>,
    nodes: HashMap<u32, Ty<'ty>>,
    diagnostics: &'a dyn DiagnosticReporter,
}

impl<'a: 'ty, 'ty> InferCtx<'a, 'ty> {
    pub fn infer<N: ExprNode>(&mut self, node: &N, expected: Ty<'ty>) -> Ty<'ty> {
        if let Some(ty) = self.nodes.get(&node.id()) {
            return *ty;
        }

        let ty = match node.infer(self, expected) {
            Ok(ty) => ty,
            Err(err) => {
                let err = self.diagnostics.report(err.into());
                self.ty_ctx.mk_err(err)
            }
        };

        if self.ty_ctx.is_final(ty) {
            self.nodes.insert(node.id(), ty);
        }

        ty
    }
}

pub trait ExprNode {
    type Error: Into<Diagnostic>;

    fn id(&self) -> u32;

    fn infer<'ty>(&self, ctx: &mut InferCtx<'_, 'ty>, expected: Ty<'ty>) -> Result<Ty<'ty>, Self::Error>;
}

/// Represents the infered bounds of a type parameter
#[derive(Clone, Copy, Debug)]
pub struct TyBounds<'ty> {
    lower: Ty<'ty>,
    upper: Ty<'ty>,
}

impl<'a: 'ty, 'ty> TyCtx<'a, 'ty> {
    /// Finds the greatest lower bound of two types
    pub fn meet(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        use TyKind::*;

        match (lhs.kind(), rhs.kind()) {
            // Equality
            _ if lhs == rhs => lhs,

            // Sentinal values
            (_, Err(err)) | (Err(err), _) => self.mk_err(err),
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

            // Sentinal values
            (_, Err(err)) | (Err(err), _) => self.mk_err(err),
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

    /// Substitutes type parameters with concrete types
    pub fn substitute(self, ty: Ty<'ty>, params: &[Ty<'ty>]) -> Ty<'ty> {
        self.transform(ty, |ty| match ty.kind() {
            TyKind::Param(id) => params[id.0 as usize],
            _ => ty,
        })
    }

    /// Substitutes type parameters with concrete types,
    /// using the upper bound in covariant positions and
    /// the lower bound in contravariant positions
    pub fn substitute_bounds(self, ty: Ty<'ty>, params: &[TyBounds<'ty>]) -> Ty<'ty> {
        self.transform_with_variance(ty, |ty, var| match ty.kind() {
            TyKind::Param(id) => match var {
                Variance::Co => params[id.0 as usize].upper,
                Variance::Contra => params[id.0 as usize].lower,
            },
            _ => ty,
        })
    }

    /// Returns `true` if no constituents of the type are pending
    pub fn is_final(self, ty: Ty<'ty>) -> bool {
        ty.fold(true, |acc, ty| acc && ty.kind() != TyKind::Pending)
    }

    /// Returns `true` if any constituents of the type are `Infer`
    pub fn has_infer(self, ty: Ty<'ty>) -> bool {
        ty.fold(false, |acc, ty| acc || ty.kind() == TyKind::Infer)
    }
}
