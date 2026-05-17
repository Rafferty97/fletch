use std::hash::{BuildHasher, Hash, RandomState};
use std::ops::Deref;
use std::sync::Mutex;

use bumpalo::Bump;
use hashbrown::HashTable;

#[derive(Hash, Debug)]
pub struct Interned<'a, T: ?Sized>(&'a T);

impl<'a, T: ?Sized> Clone for Interned<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<'a, T: ?Sized> Copy for Interned<'a, T> {}

impl<'a, T: ?Sized> PartialEq for Interned<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<'a, T: ?Sized> Eq for Interned<'a, T> {}

impl<'a, T> Deref for Interned<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub struct Interner<'a, T: ?Sized> {
    values: Mutex<HashTable<(&'a T, u64)>>,
    state: RandomState,
}

impl<'a, T: ?Sized> Interner<'a, T> {
    pub fn new() -> Self {
        Self {
            values: Mutex::new(HashTable::new()),
            state: RandomState::new(),
        }
    }
}

impl<'a, T: Hash + Eq> Interner<'a, T> {
    pub fn intern(&self, arena: &'a Bump, value: T) -> Interned<'a, T> {
        let hash = self.state.hash_one(&value);
        let &(value, _) = self
            .values
            .lock()
            .unwrap()
            .entry(hash, |&(v, _)| v == &value, |&(_, hash)| hash)
            .or_insert_with(|| (arena.alloc(value), hash))
            .get();
        Interned(value)
    }
}

impl<'a, T: Hash + Eq + Copy> Interner<'a, [T]> {
    pub fn intern_slice(&self, arena: &'a Bump, values: &[T]) -> Interned<'a, [T]> {
        let hash = self.state.hash_one(&values);
        let &(values, _) = self
            .values
            .lock()
            .unwrap()
            .entry(hash, |&(v, _)| v == values, |&(_, hash)| hash)
            .or_insert_with(|| (arena.alloc_slice_copy(values), hash))
            .get();
        Interned(values)
    }
}

pub trait Index: Copy + Eq {
    fn from_usize(index: usize) -> Self;
    fn into_usize(self) -> usize;
}

#[derive(Debug)]
pub struct IndexedInterner<'a, S, T: ?Sized> {
    values: Vec<&'a T>,
    indices: HashTable<(S, u64)>,
    state: RandomState,
}

impl<'a, S, T: ?Sized> IndexedInterner<'a, S, T> {
    pub fn new() -> Self {
        Self {
            values: vec![],
            indices: HashTable::new(),
            state: RandomState::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.values.len()
    }
}

impl<'a, S: Index> IndexedInterner<'a, S, str> {
    pub fn intern_str(&mut self, arena: &'a Bump, str: &str) -> S {
        let hash = self.state.hash_one(str);

        match self
            .indices
            .find(hash, |&(sym, _)| self.get_str(sym) == str)
        {
            Some(&(sym, _)) => sym,
            None => {
                let idx = self.values.len();
                let sym = S::from_usize(idx);
                self.values.push(arena.alloc_str(str));
                self.indices
                    .insert_unique(hash, (sym, hash), |&(_, hash)| hash);
                sym
            }
        }
    }

    pub fn get_str(&self, symbol: S) -> &'a str {
        self.values[symbol.into_usize()]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct Symbol(usize);

    impl Index for Symbol {
        fn from_usize(index: usize) -> Self {
            Self(index)
        }

        fn into_usize(self) -> usize {
            self.0
        }
    }

    #[test]
    fn intern_symbols() {
        let arena = &Bump::new();
        let mut interner = IndexedInterner::new();

        let foo: Symbol = interner.intern_str(arena, "foo");
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
