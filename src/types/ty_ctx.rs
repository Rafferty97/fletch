use bumpalo::Bump;

use crate::diagnostics::ErrGuaranteed;

use super::ty::{FloatTy, FuncTy, IntTy, Ty, TyKind, UIntTy};
use super::ty_interners::{CommonTypes, TyInterners};

#[derive(Clone, Copy)]
pub struct TyCtx<'a, 'ty> {
    arena: &'a Bump,
    interners: &'a TyInterners<'ty>,
}

impl<'a: 'ty, 'ty> TyCtx<'a, 'ty> {
    pub fn new(arena: &'a Bump, interners: &'a TyInterners<'ty>) -> Self {
        Self { arena, interners }
    }

    pub fn common(&self) -> &CommonTypes<'ty> {
        &self.interners.common_types
    }

    pub fn mk_ty_from_kind(&self, kind: TyKind<'ty>) -> Ty<'ty> {
        Ty(self.interners.ty_kind.intern(&self.arena, kind))
    }

    pub fn mk_int(&self, kind: IntTy) -> Ty<'ty> {
        match kind {
            IntTy::Int8 => self.common().int8,
            IntTy::Int16 => self.common().int16,
            IntTy::Int32 => self.common().int32,
            IntTy::Int64 => self.common().int64,
        }
    }

    pub fn mk_uint(&self, kind: UIntTy) -> Ty<'ty> {
        match kind {
            UIntTy::UInt8 => self.common().uint8,
            UIntTy::UInt16 => self.common().uint16,
            UIntTy::UInt32 => self.common().uint32,
            UIntTy::UInt64 => self.common().uint64,
        }
    }

    pub fn mk_float(&self, kind: FloatTy) -> Ty<'ty> {
        match kind {
            FloatTy::Float32 => self.common().float32,
            FloatTy::Float64 => self.common().float64,
        }
    }

    pub fn mk_nullable(&self, inner: Ty<'ty>) -> Ty<'ty> {
        self.mk_ty_from_kind(TyKind::Nullable(inner))
    }

    pub fn mk_array(&self, elem: Ty<'ty>) -> Ty<'ty> {
        self.mk_ty_from_kind(TyKind::Array(elem))
    }

    pub fn mk_tuple(&self, tys: &[Ty<'ty>]) -> Ty<'ty> {
        let tys = self.interners.ty_slice.intern_slice(self.arena, tys);
        self.mk_ty_from_kind(TyKind::Tuple(tys))
    }

    pub fn mk_func(&self, params: &[Ty<'ty>], ret: Ty<'ty>) -> Ty<'ty> {
        let params = self.interners.ty_slice.intern_slice(self.arena, params);
        self.mk_ty_from_kind(TyKind::Func(FuncTy { params, ret }))
    }

    pub fn mk_err(&self, err: ErrGuaranteed) -> Ty<'ty> {
        self.mk_ty_from_kind(TyKind::Err(err))
    }

    pub fn transform<F>(&self, ty: Ty<'ty>, visit: F) -> Ty<'ty>
    where
        F: Fn(Ty<'ty>) -> Ty<'ty>,
    {
        self.transform_with_state(ty, (), |ty, _, recurse| recurse(visit(ty), ()))
    }

    pub fn transform_with_variance<F>(&self, ty: Ty<'ty>, visit: F) -> Ty<'ty>
    where
        F: Fn(Ty<'ty>, Variance) -> Ty<'ty>,
    {
        self.transform_with_state(ty, Variance::Co, |ty, var, recurse| {
            let ty = visit(ty, var);
            match ty.kind() {
                TyKind::Func(FuncTy { params, ret }) => {
                    let new_params: Vec<_> = params.iter().map(|ty| visit(recurse(*ty, var), !var)).collect();
                    let new_ret = visit(recurse(ty, var), var);
                    let changed = *new_params != *params || new_ret != ret;
                    if changed {
                        self.mk_func(&new_params, new_ret)
                    } else {
                        ty
                    }
                }
                _ => recurse(ty, var),
            }
        })
    }

    fn transform_with_state<S, F>(&self, ty: Ty<'ty>, init: S, visit: F) -> Ty<'ty>
    where
        S: Copy,
        F: Fn(Ty<'ty>, S, &dyn Fn(Ty<'ty>, S) -> Ty<'ty>) -> Ty<'ty> + Copy,
    {
        let mut recurse = |ty: Ty<'ty>, init: S| {
            let new_ty = match ty.kind() {
                TyKind::Nullable(inner) => {
                    let new_inner = self.transform_with_state(inner, init, visit);
                    (new_inner != inner).then(|| self.mk_nullable(new_inner))
                }
                TyKind::Array(inner) => {
                    let new_inner = self.transform_with_state(inner, init, visit);
                    (new_inner != inner).then(|| self.mk_array(new_inner))
                }
                TyKind::Tuple(inner) => {
                    let new_inner: Vec<_> = inner
                        .iter()
                        .map(|ty| self.transform_with_state(*ty, init, visit))
                        .collect();
                    (*new_inner != *inner).then(|| self.mk_tuple(&new_inner))
                }
                TyKind::Func(FuncTy { params, ret }) => {
                    let new_params: Vec<_> = params
                        .iter()
                        .map(|ty| self.transform_with_state(*ty, init, visit))
                        .collect();
                    let new_ret = self.transform_with_state(ret, init, visit);
                    let changed = *new_params != *params || new_ret != ret;
                    changed.then(|| self.mk_func(&new_params, new_ret))
                }
                _ => None,
            };
            new_ty.unwrap_or(ty)
        };

        visit(ty, init, &recurse)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Variance {
    Co,
    Contra,
}

impl std::ops::Not for Variance {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Co => Self::Contra,
            Self::Contra => Self::Co,
        }
    }
}
