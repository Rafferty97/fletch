use crate::util::escape::escape;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Represents literal values in expressions
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Lit {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Unsigned integer
    UInt(u64),
    /// Signed integer
    Int(i64),
    /// floating point
    Float(NotNan<f64>),
    /// UTF-8 string
    Str(String),
}

impl Lit {
    pub fn as_i64(&self) -> Result<i64, ()> {
        match self {
            &Self::UInt(value) => value.try_into().map_err(|_| ()),
            &Self::Int(value) => value.try_into().map_err(|_| ()),
            _ => Err(())?,
        }
    }
}

impl Display for Lit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(false) => write!(f, "false"),
            Self::Bool(true) => write!(f, "true"),
            Self::UInt(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value}f"),
            Self::Str(value) => write!(f, "\"{}\"", escape(value)),
        }
    }
}
