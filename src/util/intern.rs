use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use dashmap::DashSet;

#[derive(Clone, Copy)]
pub struct Interned<'a, T>(&'a T);

impl<'a, T> Interned<'a, T> {
    pub fn new_unchecked(t: &'a T) -> Self {
        Self(t)
    }
}

impl<'a, T> Deref for Interned<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}

impl<'a, T> PartialEq for Interned<'a, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl<'a, T> Eq for Interned<'a, T> {}

impl<'a, T> Hash for Interned<'a, T> {
    #[inline]
    fn hash<H: Hasher>(&self, s: &mut H) {
        std::ptr::hash(self.0, s)
    }
}

impl<'a, T: Debug> Debug for Interned<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Interner<'a, T> {
    map: DashSet<&'a T>,
}

impl<'a, T: Hash + Eq> Interner<'a, T> {
    pub fn intern(&self, value: T, make: impl FnOnce(T) -> &'a T) -> Interned<'a, T> {
        if let Some(existing) = self.map.get(&value) {
            Interned::new_unchecked(&*existing)
        } else {
            let alloced = make(value);
            self.map.insert(alloced);
            Interned::new_unchecked(alloced)
        }
    }
}
