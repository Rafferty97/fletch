use std::fmt::Debug;
use std::hash::{BuildHasher, Hash, Hasher, RandomState};
use std::ops::Deref;

use bumpalo::Bump;
use hashbrown::HashTable;

#[derive(PartialEq, Eq, Hash)]
pub struct Interned<'a, T: ?Sized>(pub &'a T);

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

impl<T: Debug + ?Sized> Debug for Interned<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Interner<'a, T: ?Sized> {
    values: HashTable<(&'a T, u64)>,
    state: RandomState,
}

impl<'a, T: ?Sized> Interner<'a, T> {
    pub fn new() -> Self {
        Self { values: HashTable::new(), state: RandomState::new() }
    }
}

impl<'a, T: Hash + Eq> Interner<'a, T> {
    pub fn intern(&mut self, arena: &'a Bump, value: T) -> Interned<'a, T> {
        let hash = self.state.hash_one(&value);
        let &(value, _) = self
            .values
            .entry(hash, |&(v, _)| v == &value, |&(_, hash)| hash)
            .or_insert_with(|| (arena.alloc(value), hash))
            .get();
        Interned(value)
    }
}

impl<'a, T: Hash + Eq + Copy> Interner<'a, [T]> {
    pub fn intern_slice(&mut self, arena: &'a Bump, values: &[T]) -> Interned<'a, [T]> {
        let mut hasher = self.state.build_hasher();
        for value in values {
            value.hash(&mut hasher);
        }
        let hash = hasher.finish();

        let &(values, _) = self
            .values
            .entry(hash, |&(v, _)| v == values, |&(_, hash)| hash)
            .or_insert_with(|| (arena.alloc_slice_copy(values), hash))
            .get();
        Interned(values)
    }

    // pub fn intern_iter(
    //     &mut self,
    //     arena: &'a Bump,
    //     values: impl Iterator<Item = T> + Clone,
    // ) -> Interned<'a, [T]> {
    //     let mut hasher = self.state.build_hasher();
    //     for value in values.clone() {
    //         value.hash(&mut hasher);
    //     }
    //     let hash = hasher.finish();

    //     let &(values, _) = self
    //         .values
    //         .entry(hash, |&(v, _)| v == values, |&(_, hash)| hash)
    //         .or_insert_with(|| (arena.alloc_it(values), hash))
    //         .get();
    //     Interned(values)
    // }
}
