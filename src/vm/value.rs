use std::sync::Arc;

use crate::ast::Symbol;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Null,
    Scalar(u64),
    Str(Arc<str>),
    // Variant(Symbol, Arc<Value>),
}

impl Value {
    pub fn new_null() -> Self {
        Self::Null
    }

    pub fn new_bool(value: bool) -> Self {
        Self::Scalar(value as u64)
    }

    pub fn new_int(value: i64) -> Self {
        Self::Scalar(u64::from_ne_bytes(i64::to_ne_bytes(value)))
    }

    pub fn new_uint(value: u64) -> Self {
        Self::Scalar(value)
    }

    pub fn new_f32(value: f32) -> Self {
        Self::Scalar(u32::from_ne_bytes(f32::to_ne_bytes(value)) as _)
    }

    pub fn new_f64(value: f64) -> Self {
        Self::Scalar(u64::from_ne_bytes(f64::to_ne_bytes(value)))
    }

    pub fn new_str(value: impl Into<Arc<str>>) -> Self {
        Self::Str(value.into())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_int(&self) -> i64 {
        i64::from_ne_bytes(u64::to_ne_bytes(self.as_scalar()))
    }

    pub fn as_uint(&self) -> u64 {
        self.as_scalar()
    }

    pub fn as_f32(&self) -> f32 {
        f32::from_ne_bytes(u32::to_ne_bytes(self.as_scalar() as _))
    }

    pub fn as_f64(&self) -> f64 {
        f64::from_ne_bytes(u64::to_ne_bytes(self.as_scalar()))
    }

    fn as_scalar(&self) -> u64 {
        match self {
            Self::Scalar(value) => *value,
            _ => panic!("expected scalar value"),
        }
    }
}
