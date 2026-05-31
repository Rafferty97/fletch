use bumpalo::Bump;

use super::ty_interners::TyInterners;
use super::{Ty, TyKind};

#[derive(Clone, Copy)]
pub struct TyCtx<'a, 'ty> {
    arena: &'a Bump,
    interners: &'a TyInterners<'ty>,
}

impl<'a: 'ty, 'ty> TyCtx<'a, 'ty> {
    pub fn new(arena: &'a Bump, interners: &'a TyInterners<'ty>) -> Self {
        Self { arena, interners }
    }

    pub fn mk_ty_from_kind(&self, kind: TyKind<'ty>) -> Ty<'ty> {
        Ty(self.interners.ty_kind.intern(&self.arena, kind))
    }

    pub fn mk_never(&self) -> Ty<'ty> {
        self.interners.common_types.never
    }

    pub fn mk_bool(&self) -> Ty<'ty> {
        self.interners.common_types.bool
    }

    pub fn mk_null(&self) -> Ty<'ty> {
        self.interners.common_types.opt_never
    }

    pub fn mk_nullable(&self, inner: Ty<'ty>) -> Ty<'ty> {
        self.mk_ty_from_kind(TyKind::Nullable(inner))
    }

    pub fn mk_array(&self, elem: Ty<'ty>) -> Ty<'ty> {
        self.mk_ty_from_kind(TyKind::Array(elem))
    }
}
