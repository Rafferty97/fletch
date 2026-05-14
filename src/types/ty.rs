use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use crate::arena::{Interned, Symbol};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ty<'tc>(pub Interned<'tc, TyKind<'tc>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyList<'tc>(pub Interned<'tc, [Ty<'tc>]>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldList<'tc>(pub Interned<'tc, [(Symbol, Ty<'tc>)]>);

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
    Nullable(Ty<'tc>),
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

impl<'cx> TyVar<'cx> {
    pub fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }
}

impl Into<u32> for TyVar<'_> {
    fn into(self) -> u32 {
        self.0
    }
}

impl Debug for TyVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TyVar({})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntVar<'cx>(u32, PhantomData<&'cx ()>);

impl<'cx> IntVar<'cx> {
    pub fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }
}

impl Into<u32> for IntVar<'_> {
    fn into(self) -> u32 {
        self.0
    }
}

impl Debug for IntVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IntVar({})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloatVar<'cx>(u32, PhantomData<&'cx ()>);

impl<'cx> FloatVar<'cx> {
    pub fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }
}

impl Into<u32> for FloatVar<'_> {
    fn into(self) -> u32 {
        self.0
    }
}

impl Debug for FloatVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FloatVar({})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowVar<'cx>(u32, PhantomData<&'cx ()>);

impl<'cx> RowVar<'cx> {
    pub fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }
}

impl Into<u32> for RowVar<'_> {
    fn into(self) -> u32 {
        self.0
    }
}

impl Debug for RowVar<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RowVar({})", self.0)
    }
}

impl<'cx> Ty<'cx> {
    pub fn kind(&self) -> &TyKind<'cx> {
        &self.0.0
    }
}

impl<'cx> Debug for Ty<'cx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl<'cx> Deref for TyList<'cx> {
    type Target = [Ty<'cx>];

    fn deref(&self) -> &Self::Target {
        &self.0.0
    }
}

impl<'cx> IntoIterator for TyList<'cx> {
    type Item = Ty<'cx>;
    type IntoIter = std::iter::Copied<std::slice::Iter<'cx, Ty<'cx>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.0.into_iter().copied()
    }
}

impl<'a, 'cx> IntoIterator for &'a TyList<'cx> {
    type Item = Ty<'cx>;
    type IntoIter = std::iter::Copied<std::slice::Iter<'cx, Ty<'cx>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.0.into_iter().copied()
    }
}

impl<'cx> Debug for TyList<'cx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<'cx> Deref for FieldList<'cx> {
    type Target = [(Symbol, Ty<'cx>)];

    fn deref(&self) -> &Self::Target {
        &self.0.0
    }
}

impl<'cx> IntoIterator for FieldList<'cx> {
    type Item = (Symbol, Ty<'cx>);
    type IntoIter = std::iter::Copied<std::slice::Iter<'cx, (Symbol, Ty<'cx>)>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.0.into_iter().copied()
    }
}

impl<'a, 'cx> IntoIterator for &'a FieldList<'cx> {
    type Item = (Symbol, Ty<'cx>);
    type IntoIter = std::iter::Copied<std::slice::Iter<'cx, (Symbol, Ty<'cx>)>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.0.into_iter().copied()
    }
}

impl<'cx> Debug for FieldList<'cx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
