use crate::interner::Interned;

mod ty_ctx;
mod ty_interners;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'ty>(Interned<'ty, TyKind<'ty>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'ty> {
    Never,
    Bool,
    Str,
    Nullable(Ty<'ty>),
    Array(Ty<'ty>),
    Tuple(&'ty [Ty<'ty>]),
}
