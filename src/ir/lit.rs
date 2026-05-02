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
    /// Unsigned 8-bit integer
    UInt8(u8),
    /// Unsigned 16-bit integer
    UInt16(u16),
    /// Unsigned 32-bit integer
    UInt32(u32),
    /// Unsigned 64-bit integer
    UInt64(u64),
    /// Unsigned integer
    UInt(u64),
    /// Signed 8-bit integer
    Int8(i8),
    /// Signed 16-bit integer
    Int16(i16),
    /// Signed 32-bit integer
    Int32(i32),
    /// Signed 64-bit integer
    Int64(i64),
    /// Signed integer
    Int(i64),
    /// 32-bit floating point
    Float32(NotNan<f32>),
    /// 64-bit floating point
    Float64(NotNan<f64>),
    /// UTF-8 string
    Str(String),
}

impl Lit {
    pub fn as_i64(&self) -> Result<i64, ()> {
        match self {
            &Self::UInt8(value) => value.try_into().map_err(|_| ()),
            &Self::UInt16(value) => value.try_into().map_err(|_| ()),
            &Self::UInt32(value) => value.try_into().map_err(|_| ()),
            &Self::UInt64(value) => value.try_into().map_err(|_| ()),
            &Self::UInt(value) => value.try_into().map_err(|_| ()),
            &Self::Int8(value) => value.try_into().map_err(|_| ()),
            &Self::Int16(value) => value.try_into().map_err(|_| ()),
            &Self::Int32(value) => value.try_into().map_err(|_| ()),
            &Self::Int64(value) => value.try_into().map_err(|_| ()),
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
            Self::UInt8(value) => write!(f, "{value}"),
            Self::UInt16(value) => write!(f, "{value}"),
            Self::UInt32(value) => write!(f, "{value}"),
            Self::UInt64(value) => write!(f, "{value}"),
            Self::UInt(value) => write!(f, "{value}"),
            Self::Int8(value) => write!(f, "{value}"),
            Self::Int16(value) => write!(f, "{value}"),
            Self::Int32(value) => write!(f, "{value}"),
            Self::Int64(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float32(value) => write!(f, "{value}f"),
            Self::Float64(value) => write!(f, "{value}f"),
            Self::Str(value) => write!(f, "\"{}\"", escape(value)),
        }
    }
}
