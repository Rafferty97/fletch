use crate::util::escape::escape;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Represents literal values in expressions
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Lit {
    pub kind: LitKind,
    pub raw: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LitKind {
    /// Null value
    Null,
    /// Boolean value
    Bool,
    /// Integer
    Integer,
    /// Foating point number
    Float,
    /// UTF-8 string
    Str,
}
