use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Args(pub u32);

impl std::fmt::Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => write!(f, "no arguments"),
            1 => write!(f, "1 argument"),
            n => write!(f, "{n} arguments"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Elements(pub u32);

impl std::fmt::Display for Elements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => write!(f, "no elements"),
            1 => write!(f, "1 elements"),
            n => write!(f, "{n} elements"),
        }
    }
}

pub struct IdGen<T> {
    next_id: AtomicU32,
    make: fn(u32) -> T,
}

impl<T> IdGen<T> {
    pub fn new(make: fn(u32) -> T) -> Self {
        Self { next_id: AtomicU32::new(0), make }
    }

    pub fn next(&self) -> T {
        (self.make)(self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}
