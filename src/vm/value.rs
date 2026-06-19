use std::sync::Arc;

use crate::ast::Symbol;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Null,
    Scalar(u64),
    // Variant(Symbol, Arc<Value>),
}

impl Value {
    // pub fn
}
