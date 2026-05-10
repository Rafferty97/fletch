use std::hash::{BuildHasher, Hash, RandomState};
use std::ops::Deref;

use bumpalo::Bump;
use hashbrown::HashTable;

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Interned<'a, T: ?Sized>(&'a T);

impl<'a, T: ?Sized> Clone for Interned<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<'a, T: ?Sized> Copy for Interned<'a, T> {}

impl<'a, T> Deref for Interned<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub struct Interner<'a, T: ?Sized> {
    arena: &'a Bump,
    values: HashTable<(&'a T, u64)>,
    state: RandomState,
}

impl<'a, T: Hash + Eq> Interner<'a, T> {
    pub fn intern(&mut self, value: T) -> Interned<'a, T> {
        let hash = self.state.hash_one(&value);
        let &(value, _) = self
            .values
            .entry(hash, |&(v, _)| v == &value, |&(_, hash)| hash)
            .or_insert_with(|| (self.arena.alloc(value), hash))
            .get();
        Interned(value)
    }
}

impl<'a, T: Hash + Eq + Copy> Interner<'a, [T]> {
    pub fn intern_slice(&mut self, values: &[T]) -> Interned<'a, [T]> {
        let hash = self.state.hash_one(&values);
        let &(values, _) = self
            .values
            .entry(hash, |&(v, _)| v == values, |&(_, hash)| hash)
            .or_insert_with(|| (self.arena.alloc_slice_copy(values), hash))
            .get();
        Interned(values)
    }
}
