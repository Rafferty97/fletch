use std::hash::{BuildHasher, RandomState};

use bumpalo::Bump;
use hashbrown::hash_table::HashTable;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Symbol(u32);

// #[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
// pub struct ByteSymbol(u32);

#[derive(Debug)]
pub struct SymbolInterner<'a> {
    values: Vec<&'a str>,
    indices: HashTable<(Symbol, u64)>,
    state: RandomState,
}

impl<'a> SymbolInterner<'a> {
    pub fn new() -> Self {
        Self {
            values: vec![],
            indices: HashTable::new(),
            state: RandomState::new(),
        }
    }

    pub fn intern_str(&mut self, arena: &'a Bump, str: &str) -> Symbol {
        let hash = self.state.hash_one(str);

        match self.indices.find(hash, |&(sym, _)| self.get_str(sym) == str) {
            Some(&(sym, _)) => sym,
            None => {
                let idx = self.values.len().try_into().expect("exceeded capacity");
                let sym = Symbol(idx);
                self.values.push(arena.alloc_str(str));
                self.indices.insert_unique(hash, (sym, hash), |&(_, hash)| hash);
                sym
            }
        }
    }

    pub fn get_str(&self, symbol: Symbol) -> &'a str {
        self.values[symbol.0 as usize]
    }

    pub fn size(&self) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn intern_symbols() {
        let arena = &Bump::new();
        let mut interner = SymbolInterner::new();

        let foo = interner.intern_str(arena, "foo");
        let bar = interner.intern_str(arena, "bar");
        let hello = interner.intern_str(arena, "hello world");

        // Check that symbols can be converted back to strings
        assert_eq!(interner.get_str(foo), "foo");
        assert_eq!(interner.get_str(bar), "bar");
        assert_eq!(interner.get_str(hello), "hello world");

        // Check that symbols are duplicated
        assert_eq!(interner.intern_str(arena, "bar"), bar);
        assert_eq!(interner.intern_str(arena, "hello world"), hello);
        assert_eq!(interner.intern_str(arena, "foo"), foo);
        assert_eq!(interner.size(), 3);
    }
}
