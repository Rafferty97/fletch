use bumpalo::Bump;

use super::ty::{FloatTy, IntTy, Ty, TyKind, UIntTy};
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
}
