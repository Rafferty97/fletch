use bumpalo::Bump;

use crate::util::intern::{Interned, Interner};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'tcx>(Interned<'tcx, TyKind<'tcx>>);

pub type Tys<'tcx> = &'tcx [Ty<'tcx>];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'tcx> {
    Bool,
    Char,
    Int(IntTy),
    UInt(UIntTy),
    Array(Ty<'tcx>),
    Tuple(Tys<'tcx>),
    Never,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum UIntTy {
    U8,
    U16,
    U32,
    U64,
}

pub struct TyCtx<'tcx> {
    arena: &'tcx Bump,
    ty_interner: Interner<'tcx, TyKind<'tcx>>,
    tys: CommonTypes<'tcx>,
}

impl<'tcx> TyCtx<'tcx> {
    pub fn make_ty(&self, kind: TyKind<'tcx>) -> Ty<'tcx> {
        Ty(self.ty_interner.intern(kind, |kind| self.arena.alloc(kind)))
    }
}

pub struct CommonTypes<'tcx> {
    pub unit: Ty<'tcx>,
    pub bool: Ty<'tcx>,
    pub u8: Ty<'tcx>,
    pub u16: Ty<'tcx>,
    pub u32: Ty<'tcx>,
    pub u64: Ty<'tcx>,
    pub i8: Ty<'tcx>,
    pub i16: Ty<'tcx>,
    pub i32: Ty<'tcx>,
    pub i64: Ty<'tcx>,
}

impl<'tcx> CommonTypes<'tcx> {
    pub fn new(arena: &'tcx Bump, interner: &Interner<'tcx, TyKind<'tcx>>) -> Self {
        let make_ty = |kind| Ty(interner.intern(kind, |kind| arena.alloc(kind)));

        Self {
            unit: make_ty(TyKind::Tuple(&[])),
            bool: make_ty(TyKind::Bool),
            u8: make_ty(TyKind::UInt(UIntTy::U8)),
            u16: make_ty(TyKind::UInt(UIntTy::U16)),
            u32: make_ty(TyKind::UInt(UIntTy::U32)),
            u64: make_ty(TyKind::UInt(UIntTy::U64)),
            i8: make_ty(TyKind::Int(IntTy::I8)),
            i16: make_ty(TyKind::Int(IntTy::I16)),
            i32: make_ty(TyKind::Int(IntTy::I32)),
            i64: make_ty(TyKind::Int(IntTy::I64)),
        }
    }
}
