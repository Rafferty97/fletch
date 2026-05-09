use std::hash::{BuildHasher, RandomState};

use bumpalo::Bump;
use hashbrown::hash_table::HashTable;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Symbol(u32);

// #[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
// pub struct ByteSymbol(u32);

#[derive(Debug)]
pub struct SymbolInterner {
    arena: Bump,
    values: Vec<(*const u8, usize)>,
    indices: HashTable<(u32, u64)>,
    state: RandomState,
}

impl SymbolInterner {
    pub fn new() -> Self {
        Self {
            arena: Bump::new(),
            values: vec![],
            indices: HashTable::new(),
            state: RandomState::new(),
        }
    }

    pub fn intern_str(&mut self, str: &str) -> Symbol {
        let bytes = str.as_bytes();
        let hash = self.state.hash_one(bytes);

        dbg!(str, hash);
        let idx = match self.indices.find(hash, |&(idx, _)| self.get_inner(idx) == bytes) {
            Some(&(idx, _)) => idx,
            None => {
                let idx = self.values.len().try_into().expect("exceeded capacity");
                let slice = self.arena.alloc_slice_copy(bytes);
                self.values.push((slice.as_ptr(), slice.len()));
                self.indices.insert_unique(hash, (idx, hash), |&(_, hash)| hash);
                idx
            }
        };

        Symbol(idx)
    }

    pub fn get_str(&self, symbol: Symbol) -> &str {
        let bytes = self.get_inner(symbol.0);

        // SAFETY: The index came from a `Symbol`, which can
        // only be created from valid UTF-8 strings
        unsafe { str::from_utf8_unchecked(bytes) }
    }

    pub fn size(&self) -> usize {
        self.values.len()
    }

    fn get_inner(&self, index: u32) -> &[u8] {
        let (ptr, len) = self.values[index as usize];

        // SAFETY: the string is allocated in `self.arena`,
        // and so must live at least as long as `&self`
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn intern_symbols() {
        let mut interner = SymbolInterner::new();

        let foo = interner.intern_str("foo");
        let bar = interner.intern_str("bar");
        let hello = interner.intern_str("hello world");

        // Check that symbols can be converted back to strings
        assert_eq!(interner.get_str(foo), "foo");
        assert_eq!(interner.get_str(bar), "bar");
        assert_eq!(interner.get_str(hello), "hello world");

        // Check that symbols are duplicated
        assert_eq!(interner.intern_str("bar"), bar);
        assert_eq!(interner.intern_str("hello world"), hello);
        assert_eq!(interner.intern_str("foo"), foo);
        assert_eq!(interner.size(), 3);
    }
}
