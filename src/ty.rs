use bumpalo::Bump;

use crate::interner::{Interned, Interner};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'ty>(Interned<'ty, TyKind<'ty>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'ty> {
    Bool,
    Array(Ty<'ty>),
    Tuple(&'ty [Ty<'ty>]),
}

pub struct TyCtx<'ty> {
    arena: &'ty Bump,
    ty_interner: Interner<'ty, TyKind<'ty>>,
    ty_slice_interner: Interner<'ty, [Ty<'ty>]>,
    common_types: CommonTypes<'ty>,
}

pub struct CommonTypes<'ty> {
    unit: Ty<'ty>,
    bool: Ty<'ty>,
}

impl<'ty> TyCtx<'ty> {
    pub fn new(arena: &'ty Bump) -> Self {
        let ty_interner = Interner::new();
        let ty_slice_interner = Interner::new();

        let common_types = CommonTypes {
            unit: Ty(ty_interner.intern(arena, TyKind::Tuple(&[]))),
            bool: Ty(ty_interner.intern(arena, TyKind::Bool)),
        };

        Self {
            arena,
            ty_interner,
            ty_slice_interner,
            common_types,
        }
    }

    pub fn new_ty(&self, kind: TyKind<'ty>) -> Ty<'ty> {
        Ty(self.ty_interner.intern(&self.arena, kind))
    }

    pub fn new_unit(&self) -> Ty<'ty> {
        self.common_types.unit
    }

    pub fn new_bool(&self) -> Ty<'ty> {
        self.common_types.bool
    }
}
