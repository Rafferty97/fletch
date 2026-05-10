use bumpalo::Bump;
pub use interned::*;
pub use symbol::*;

use crate::types::{Ty, TyKind};

mod interned;
mod symbol;

pub struct Ctx<'cx> {
    arena: &'cx Bump,
    symbol_interner: SymbolInterner<'cx>,
    ty_interner: Interner<'cx, TyKind<'cx>>,
    ty_list_interner: Interner<'cx, [Ty<'cx>]>,
}

impl<'cx> Ctx<'cx> {
    fn intern_symbol(&mut self, str: &str) -> Symbol {
        self.symbol_interner.intern_str(self.arena, str)
    }

    fn intern_ty_kind(&mut self, kind: TyKind<'cx>) -> Interned<'cx, TyKind<'cx>> {
        self.ty_interner.intern(self.arena, kind)
    }

    fn intern_ty_slice(&mut self, tys: &[Ty<'cx>]) -> Interned<'cx, [Ty<'cx>]> {
        self.ty_list_interner.intern_slice(self.arena, tys)
    }
}
