use fnv::FnvHashMap;

use crate::ast::Symbol;
use crate::interner::IndexTable;
use crate::vm::chunk::Chunk;
use crate::vm::value::FuncObjRef;

pub struct Module {
    pub main: FuncId,
    pub funcs: FnvHashMap<FuncId, FuncObjRef>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FuncId(pub u32);

impl Module {
    pub fn main(&self) -> &Chunk {
        &self.funcs[&self.main].chunk
    }

    pub fn disassemble(&self, sym_table: &IndexTable<'_, Symbol, str>) -> String {
        use std::fmt::Write;

        let mut out = String::new();
        for (_, func) in &self.funcs {
            write!(&mut out, "<{}>\n", func.name);
            write!(&mut out, "{}\n", func.chunk.disassemble(sym_table));
        }
        out
    }
}
