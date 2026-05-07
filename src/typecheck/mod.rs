use std::{fmt::Display, ops::Deref};

use bumpalo::Bump;

use crate::ast::BinOp;
use crate::error::{Error, Result};
use crate::util::intern::{Interned, Interner};
use crate::util::span::Span;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'tcx>(Interned<'tcx, TyKind<'tcx>>);

pub type Tys<'tcx> = &'tcx [Ty<'tcx>];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'tcx> {
    Null,
    Bool,
    Char,
    Int(IntTy),
    UInt(UIntTy),
    Float(FloatTy),
    Array(Ty<'tcx>),
    Tuple(Tys<'tcx>),
    Infer(InferVar),
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum FloatTy {
    F32,
    F64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InferVar(u32);

impl<'ctx> Ty<'ctx> {
    pub fn kind(self) -> TyKind<'ctx> {
        *self.0
    }
}

impl Display for Ty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind().fmt(f)
    }
}

impl Display for TyKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool => write!(f, "bool"),
            Self::Char => write!(f, "char"),
            Self::Int(IntTy::I8) => write!(f, "i8"),
            Self::Int(IntTy::I16) => write!(f, "i16"),
            Self::Int(IntTy::I32) => write!(f, "i32"),
            Self::Int(IntTy::I64) => write!(f, "i64"),
            Self::UInt(UIntTy::U8) => write!(f, "u8"),
            Self::UInt(UIntTy::U16) => write!(f, "u16"),
            Self::UInt(UIntTy::U32) => write!(f, "u32"),
            Self::UInt(UIntTy::U64) => write!(f, "u64"),
            Self::Float(FloatTy::F32) => write!(f, "f32"),
            Self::Float(FloatTy::F64) => write!(f, "f64"),
            Self::Array(ty) => write!(f, "{ty}[]"),
            Self::Tuple([]) => write!(f, "()"),
            Self::Tuple([first, rest @ ..]) => {
                write!(f, "({first}")?;
                for ty in rest {
                    write!(f, ", {ty}")?;
                }
                write!(f, ")")
            }
            Self::Infer(_) => write!(f, "?"),
            Self::Never => write!(f, "!"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TyCtx<'tcx>(&'tcx TyCtxInner<'tcx>);

impl<'tcx> Deref for TyCtx<'tcx> {
    type Target = TyCtxInner<'tcx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct TyCtxInner<'tcx> {
    arena: &'tcx Bump,
    ty_interner: Interner<'tcx, TyKind<'tcx>>,
    pub tys: CommonTypes<'tcx>,
}

impl<'tcx> TyCtx<'tcx> {
    pub fn make_ty(&self, kind: TyKind<'tcx>) -> Ty<'tcx> {
        Ty(self.ty_interner.intern(kind, |kind| self.arena.alloc(kind)))
    }
}

pub fn with_ty_ctx<T>(f: impl FnOnce(TyCtx) -> T) -> T {
    let arena = &Bump::new();
    let ty_interner = Interner::new();
    let tys = CommonTypes::new(arena, &ty_interner);
    let inner = arena.alloc(TyCtxInner { arena, ty_interner, tys });
    f(TyCtx(&inner))
}

pub struct CommonTypes<'tcx> {
    pub unit: Ty<'tcx>,
    pub null: Ty<'tcx>,
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
            null: make_ty(TyKind::Null),
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
