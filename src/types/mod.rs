use bumpalo::Bump;

use crate::arena::{Interned, Interner};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'tc>(Interned<'tc, TyKind<'tc>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TyList<'tc>(Interned<'tc, [Ty<'tc>]>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'tc> {
    Bool,
    Int(IntTy),
    UInt(UIntTy),
    Str,
    Array(Ty<'tc>),
    Tuple(TyList<'tc>),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum IntTy {
    Int8,
    Int16,
    Int32,
    Int64,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum UIntTy {
    UInt8,
    UInt16,
    UInt32,
    UInt64,
}

struct TyCtx<'tc> {
    arena: &'tc Bump,
    ty_interner: Interner<'tc, TyKind<'tc>>,
    ty_list_interner: Interner<'tc, [Ty<'tc>]>,
}

impl<'tc> TyCtx<'tc> {
    fn new_ty(&mut self, kind: TyKind<'tc>) -> Ty<'tc> {
        Ty(self.ty_interner.intern(kind))
    }

    fn new_ty_list(&mut self, tys: &[Ty<'tc>]) -> TyList<'tc> {
        TyList(self.ty_list_interner.intern_slice(tys))
    }

    fn new_tuple(&mut self, tys: &[Ty<'tc>]) -> Ty<'tc> {
        let tys = self.new_ty_list(tys);
        self.new_ty(TyKind::Tuple(tys))
    }
}
