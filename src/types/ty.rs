use std::fmt::{Debug, Display};

use crate::{diagnostics::ErrGuaranteed, interner::Interned};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
    Param(u32),
    Infer,
    Pending,
    Error(ErrGuaranteed),
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

impl<'ty> Ty<'ty> {
    pub fn kind(self) -> TyKind<'ty> {
        *self.0
    }

    pub fn is_never(self) -> bool {
        self.kind() == TyKind::Never
    }

    pub fn visit(self, mut visit: impl FnMut(Self)) {
        self.fold((), |_, ty| visit(ty))
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

impl<'ty> Display for Ty<'ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind() {
            TyKind::Never => write!(f, "!"),
            TyKind::Bool => write!(f, "bool"),
            TyKind::Int(IntTy::Int8) => write!(f, "int8"),
            TyKind::Int(IntTy::Int16) => write!(f, "int16"),
            TyKind::Int(IntTy::Int32) => write!(f, "int32"),
            TyKind::Int(IntTy::Int64) => write!(f, "int64"),
            TyKind::Nullable(inner) => write!(f, "{inner}?"),
            TyKind::Array(inner) => write!(f, "{inner}[]"),
            TyKind::Tuple(tys) => match &tys[..] {
                [] => write!(f, "()"),
                [first, rest @ ..] => {
                    write!(f, "({first}")?;
                    for ty in rest {
                        write!(f, ", {ty}")?;
                    }
                    write!(f, ")")
                }
            },
            TyKind::Func(FuncTy { params, ret }) => match &params[..] {
                [] => write!(f, "() -> {ret}"),
                [arg] => write!(f, "{arg} -> {ret}"),
                [first, rest @ ..] => {
                    write!(f, "({first}")?;
                    for ty in rest {
                        write!(f, ", {ty}")?;
                    }
                    write!(f, ") -> {ret}")
                }
            },
            TyKind::Param(id) => write!(f, "${}", id),
            TyKind::Infer => write!(f, "_"),
            TyKind::Pending => write!(f, "?"),
            TyKind::Error(_) => write!(f, "{{err}}"),
            k => write!(f, "{k:?}"),
        }
    }
}

impl<'ty> Debug for Ty<'ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ty({self})")
    }
}
