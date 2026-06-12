use bumpalo::Bump;

use super::ty::{Ty, TyKind};
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
