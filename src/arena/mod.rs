use std::sync::{Mutex, RwLock};

use bumpalo::Bump;
use hashbrown::HashMap;
pub use interned::*;
pub use symbol::*;

use crate::ast::NodeId;
use crate::diagnostics::{DiagCtx, DiagnosticHandler};
use crate::types::{Ty, TyKind};

mod interned;
mod symbol;

#[derive(Clone, Copy)]
pub struct Ctx<'cx> {
    pub inner: &'cx CtxInner<'cx>,
}

pub struct CtxInner<'cx> {
    diag: DiagCtx<'cx>,
    arena: &'cx Bump,
    symbol_interner: RwLock<SymbolInterner<'cx>>,
    ty_interner: Mutex<Interner<'cx, TyKind<'cx>>>,
    ty_list_interner: Mutex<Interner<'cx, [Ty<'cx>]>>,
    field_list_interner: Mutex<Interner<'cx, [(Symbol, Ty<'cx>)]>>,
    node_tys: Mutex<HashMap<NodeId, Ty<'cx>>>,
}

impl<'cx> Ctx<'cx> {
    pub fn new(arena: &'cx Bump, handler: &'cx mut dyn DiagnosticHandler) -> Self {
        let inner = arena.alloc(CtxInner {
            diag: DiagCtx::new(handler),
            arena,
            symbol_interner: RwLock::new(SymbolInterner::new()),
            ty_interner: Mutex::new(Interner::new()),
            ty_list_interner: Mutex::new(Interner::new()),
            field_list_interner: Mutex::new(Interner::new()),
            node_tys: Mutex::new(HashMap::new()),
        });
        Self { inner }
    }

    pub fn intern_str(&mut self, str: &str) -> Symbol {
        self.inner.symbol_interner.write().unwrap().intern_str(self.inner.arena, str)
    }

    pub fn intern_ty_kind(&mut self, kind: TyKind<'cx>) -> Interned<'cx, TyKind<'cx>> {
        self.inner.ty_interner.lock().unwrap().intern(self.inner.arena, kind)
    }

    pub fn intern_tys(&mut self, tys: &[Ty<'cx>]) -> Interned<'cx, [Ty<'cx>]> {
        self.inner.ty_list_interner.lock().unwrap().intern_slice(self.inner.arena, tys)
    }

    pub fn intern_fields(
        &mut self,
        fields: &[(Symbol, Ty<'cx>)],
    ) -> Interned<'cx, [(Symbol, Ty<'cx>)]> {
        self.inner.field_list_interner.lock().unwrap().intern_slice(self.inner.arena, fields)
    }

    // pub fn intern_fields_iter(
    //     &mut self,
    //     fields: impl Iterator<Item = (Symbol, Ty<'cx>)>,
    // ) -> Interned<'cx, [(Symbol, Ty<'cx>)]> {
    //     self.inner.field_list_interner.lock().unwrap().intern_slice(self.inner.arena, fields)
    // }

    pub fn get_str(&self, symbol: Symbol) -> &'cx str {
        self.inner.symbol_interner.read().unwrap().get_str(symbol)
    }

    pub fn set_node_ty(&self, node_id: NodeId, ty: Ty<'cx>) {
        self.inner.node_tys.lock().unwrap().insert(node_id, ty);
    }
}
