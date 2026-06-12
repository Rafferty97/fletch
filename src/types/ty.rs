use crate::interner::Interned;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'ty>(pub(super) Interned<'ty, TyKind<'ty>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'ty> {
    Never,
    Bool,
    Int(IntTy),
    UInt(UIntTy),
    Float(FloatTy),
    Str,
    Nullable(Ty<'ty>),
    Array(Ty<'ty>),
    Tuple(Interned<'ty, [Ty<'ty>]>),
    Infer,
    Pending,
    Err,
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum FloatTy {
    Float32,
    Float64,
}

impl<'ty> Ty<'ty> {
    pub fn kind(self) -> TyKind<'ty> {
        *self.0
    }
}
