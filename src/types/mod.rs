use std::{fmt::Debug, marker::PhantomData};

use crate::arena::{Interned, Symbol};

mod typechecker;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ty<'tc>(Interned<'tc, TyData<'tc>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyList<'tc>(Interned<'tc, [Ty<'tc>]>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldList<'tc>(Interned<'tc, [(Symbol, Ty<'tc>)]>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TyData<'tc> {
    pub kind: TyKind<'tc>,
    pub nullable: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'tc> {
    Bool,
    Int(IntTy),
    UInt(UIntTy),
    Float(FloatTy),
    Str,
    Array(Ty<'tc>),
    Tuple(TyList<'tc>),
    Struct(FieldList<'tc>, Option<RowVar<'tc>>),
    Enum(FieldList<'tc>, Option<RowVar<'tc>>),
    TyVar(TyVar<'tc>),
    IntVar(IntVar<'tc>),
    FloatVar(FloatVar<'tc>),
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyVar<'cx>(u32, PhantomData<&'cx ()>);

impl Debug for TyVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TyVar({})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntVar<'cx>(u32, PhantomData<&'cx ()>);

impl Debug for IntVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IntVar({})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloatVar<'cx>(u32, PhantomData<&'cx ()>);

impl Debug for FloatVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FloatVar({})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowVar<'cx>(u32, PhantomData<&'cx ()>);

impl Debug for RowVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RowVar({})", self.0)
    }
}

impl<'cx> Ty<'cx> {
    pub fn new(data: Interned<'cx, TyData<'cx>>) -> Self {
        Self(data)
    }

    pub fn kind(&self) -> &TyKind<'cx> {
        &self.0.kind
    }

    pub fn nullable(&self) -> bool {
        self.0.nullable
    }
}

impl<'cx> Debug for Ty<'cx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl<'cx> TyList<'cx> {
    pub fn tys(&self) -> &[Ty<'cx>] {
        &self.0.0
    }
}

impl<'cx> Debug for TyList<'cx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<'cx> FieldList<'cx> {
    pub fn fields(&self) -> &[(Symbol, Ty<'cx>)] {
        &self.0.0
    }
}

impl<'cx> Debug for FieldList<'cx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
