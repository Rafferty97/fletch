use std::fmt::{Debug, Display};

use crate::{
    ast::{Ident, Symbol},
    diagnostics::ErrGuaranteed,
    interner::{IndexTable, Interned},
    parser::SymTable,
    types::ty_ctx::Variance,
    vm::instr::Width,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ty<'ty>(pub(super) Interned<'ty, TyKind<'ty>>);

pub type Tys<'ty> = Interned<'ty, [Ty<'ty>]>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'ty> {
    Never,
    Bool,
    Int(IntTy),
    UInt(UIntTy),
    Integer,
    Float(FloatTy),
    Str,
    Nullable(Ty<'ty>),
    Array(Ty<'ty>),
    Tuple(Tys<'ty>),
    Func(FuncTy<'ty>),
    Any,
    Param(ParamId),
    Var(VarId),
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

impl IntTy {
    pub fn width(self) -> Width {
        match self {
            Self::Int8 => Width::_8,
            Self::Int16 => Width::_16,
            Self::Int32 => Width::_32,
            Self::Int64 => Width::_64,
        }
    }
}

impl UIntTy {
    pub fn width(self) -> Width {
        match self {
            Self::UInt8 => Width::_8,
            Self::UInt16 => Width::_16,
            Self::UInt32 => Width::_32,
            Self::UInt64 => Width::_64,
        }
    }
}

impl FloatTy {
    pub fn width(self) -> Width {
        match self {
            Self::Float32 => Width::_32,
            Self::Float64 => Width::_64,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ParamId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VarId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FuncTy<'ty> {
    pub params: Tys<'ty>,
    pub ret: Ty<'ty>,
}

impl<'ty> Ty<'ty> {
    pub fn kind(self) -> TyKind<'ty> {
        *self.0
    }

    pub fn is_unit(self) -> bool {
        match self.kind() {
            TyKind::Tuple(elems) => elems.is_empty(),
            _ => false,
        }
    }

    pub fn is_never(self) -> bool {
        self.kind() == TyKind::Never
    }

    pub fn is_err(self) -> bool {
        matches!(self.kind(), TyKind::Error(_))
    }

    /// Returns `true` if no constituents of the type are `Pending`
    pub fn is_final(self) -> bool {
        self.fold(true, &mut |acc, ty| acc && ty.kind() != TyKind::Pending)
    }

    /// Returns `true` if any constituents of the type are `Infer`
    pub fn has_infer(self) -> bool {
        self.fold(false, &mut |acc, ty| acc || ty.kind() == TyKind::Infer)
    }

    /// If the type is an array, returns its element type, otherwise `None`
    pub fn array_elem(self) -> Option<Self> {
        match self.kind() {
            TyKind::Array(inner) => Some(inner),
            TyKind::Nullable(inner) => inner.array_elem(),
            _ => None,
        }
    }

    /// If the type is a tuple, returns its element types, otherwise `None`
    pub fn tuple_elems(self) -> Option<&'ty [Ty<'ty>]> {
        match self.kind() {
            TyKind::Tuple(inner) => Some(inner.as_ref()),
            TyKind::Nullable(inner) => inner.tuple_elems(),
            _ => None,
        }
    }

    pub fn visit(self, mut visit: impl FnMut(Self)) {
        self.fold((), &mut |_, ty| visit(ty))
    }

    pub fn fold<T>(self, init: T, visit: &mut impl FnMut(T, Self) -> T) -> T {
        let accum = visit(init, self);
        match self.kind() {
            TyKind::Nullable(ty) => ty.fold(accum, visit),
            TyKind::Array(ty) => ty.fold(accum, visit),
            TyKind::Tuple(tys) => tys.iter().fold(accum, |accum, ty| ty.fold(accum, visit)),
            TyKind::Func(FuncTy { params, ret }) => {
                let accum = params.iter().fold(accum, |accum, ty| ty.fold(accum, visit));
                ret.fold(accum, visit)
            }
            _ => accum,
        }
    }

    pub fn display_ctx<'a>(self, ty_params: &'a [Ident], sym_table: &'a SymTable<'a>) -> TyWithCtx<'a, 'ty> {
        TyWithCtx { ty: self, ty_params, sym_table }
    }

    pub fn with_ctx<'a>(self, other: TyWithCtx<'a, 'ty>) -> TyWithCtx<'a, 'ty> {
        TyWithCtx { ty: self, ..other }
    }
}

impl<'ty> Display for Ty<'ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_ctx(&[], IndexTable::empty()))
    }
}

#[derive(Clone, Copy)]
pub struct TyWithCtx<'a, 'ty> {
    pub ty: Ty<'ty>,
    pub ty_params: &'a [Ident],
    pub sym_table: &'a SymTable<'a>,
}

impl<'ty> Display for TyWithCtx<'_, 'ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ty.kind() {
            TyKind::Never => write!(f, "never"),
            TyKind::Bool => write!(f, "bool"),
            TyKind::Int(IntTy::Int8) => write!(f, "int8"),
            TyKind::Int(IntTy::Int16) => write!(f, "int16"),
            TyKind::Int(IntTy::Int32) => write!(f, "int32"),
            TyKind::Int(IntTy::Int64) => write!(f, "int64"),
            TyKind::UInt(UIntTy::UInt8) => write!(f, "uint8"),
            TyKind::UInt(UIntTy::UInt16) => write!(f, "uint16"),
            TyKind::UInt(UIntTy::UInt32) => write!(f, "uint32"),
            TyKind::UInt(UIntTy::UInt64) => write!(f, "uint64"),
            TyKind::Integer => write!(f, "int"),
            TyKind::Float(FloatTy::Float32) => write!(f, "float32"),
            TyKind::Float(FloatTy::Float64) => write!(f, "float64"),
            TyKind::Str => write!(f, "str"),
            TyKind::Nullable(inner) => {
                if inner.kind() == TyKind::Never {
                    write!(f, "null")
                } else {
                    write!(f, "{}?", inner.with_ctx(*self))
                }
            }
            TyKind::Array(inner) => write!(f, "[{}]", inner.with_ctx(*self)),
            TyKind::Tuple(tys) => match &tys[..] {
                [] => write!(f, "()"),
                [first, rest @ ..] => {
                    write!(f, "({}", first.with_ctx(*self))?;
                    for ty in rest {
                        write!(f, ", {}", ty.with_ctx(*self))?;
                    }
                    write!(f, ")")
                }
            },
            TyKind::Func(FuncTy { params, ret }) if ret.is_unit() => match &params[..] {
                [] => write!(f, "fn()"),
                [first, rest @ ..] => {
                    write!(f, "fn({}", first.with_ctx(*self))?;
                    for ty in rest {
                        write!(f, ", {}", ty.with_ctx(*self))?;
                    }
                    write!(f, ")")
                }
            },
            TyKind::Func(FuncTy { params, ret }) => match &params[..] {
                [] => write!(f, "fn() -> {}", ret.with_ctx(*self)),
                [first, rest @ ..] => {
                    write!(f, "fn({}", first.with_ctx(*self))?;
                    for ty in rest {
                        write!(f, ", {}", ty.with_ctx(*self))?;
                    }
                    write!(f, ") -> {}", ret.with_ctx(*self))
                }
            },
            TyKind::Any => write!(f, "any"),
            TyKind::Param(id) => match self
                .ty_params
                .get(id.0 as usize)
                .map(|ident| self.sym_table.get_str(ident.sym))
            {
                Some(name) => write!(f, "{}", name),
                None => write!(f, "${}", id.0),
            },
            TyKind::Var(id) => write!(f, "?{}", id.0),
            TyKind::Infer => write!(f, "_"),
            TyKind::Pending => write!(f, "?"),
            TyKind::Error(_) => write!(f, "{{err}}"),
        }
    }
}

impl<'ty> Debug for Ty<'ty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ty({})", self)
    }
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;

    use crate::types::{ty_ctx::TyCtx, ty_interners::TyInterners};

    use super::*;

    fn with_ctx(f: impl for<'ty> FnOnce(TyCtx<'_, 'ty>)) {
        let arena = Bump::new();
        let interners = TyInterners::new(&arena);
        let ctx = TyCtx::new(&arena, &interners);
        f(ctx);
    }

    #[test]
    fn test_opt_never() {
        with_ctx(|ctx| {
            let actual = ctx.common().opt_never;
            let expected = ctx.mk_ty_from_kind(TyKind::Nullable(ctx.mk_ty_from_kind(TyKind::Never)));
            assert_eq!(actual, expected);
        });
    }

    #[test]
    fn test_is_final() {
        with_ctx(|ctx| {
            let infer = ctx.common().infer;
            let pending = ctx.common().pending;
            let i32 = ctx.common().int32;

            assert_eq!(ctx.mk_tuple(&[i32, ctx.mk_func(&[infer], pending)]).is_final(), false);
            assert_eq!(ctx.mk_tuple(&[i32, ctx.mk_func(&[infer], i32)]).is_final(), true);
        });
    }
}
