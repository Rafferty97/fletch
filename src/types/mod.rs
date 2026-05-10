use crate::arena::Interned;

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
