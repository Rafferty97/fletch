use crate::{diagnostics::ErrGuaranteed, interner::Interned};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'ty>(pub(super) Interned<'ty, TyKind<'ty>>);

pub type Tys<'ty> = Interned<'ty, [Ty<'ty>]>;

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
    Tuple(Tys<'ty>),
    Func(FuncTy<'ty>),
    Any,
    Param(ParamId),
    Infer,
    Pending,
    Err(ErrGuaranteed),
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FuncTy<'ty> {
    pub params: Tys<'ty>,
    pub ret: Ty<'ty>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamId(pub u32);

impl<'ty> Ty<'ty> {
    pub fn kind(self) -> TyKind<'ty> {
        *self.0
    }

    pub fn fold<T>(self, init: T, mut visit: impl FnMut(T, Self) -> T) -> T {
        let accum = visit(init, self);
        match self.kind() {
            TyKind::Nullable(ty) => ty.fold(accum, visit),
            TyKind::Array(ty) => ty.fold(accum, visit),
            TyKind::Tuple(tys) => tys.iter().copied().fold(accum, visit),
            TyKind::Func(FuncTy { params, ret }) => {
                let accum = params.iter().copied().fold(accum, &mut visit);
                ret.fold(accum, visit)
            }
            _ => accum,
        }
    }
}
