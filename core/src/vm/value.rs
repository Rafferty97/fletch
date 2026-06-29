use std::ops::Deref;

use arcstr::ArcStr;
use triomphe::{Arc, ThinArc};

use crate::ast::Symbol;
use crate::parser::escape;
use crate::types::ty::{FloatTy, IntTy, UIntTy};
use crate::vm::chunk::Chunk;
use crate::vm::instr::Width;
use crate::vm::module::FuncId;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Null,
    Scalar { ty: ScalarTy, value: u64 },
    Str(ArcStr),
    Array(ThinArc<(), Value>),
    Func(FuncId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScalarTy {
    Bool,
    Int(IntTy),
    UInt(UIntTy),
    Float(FloatTy),
}

#[derive(Clone, Debug)]
pub struct FuncObjRef(Arc<FuncObj>);

#[derive(Debug)]
pub struct FuncObj {
    pub name: String,
    pub chunk: Chunk,
}

impl PartialEq for FuncObjRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for FuncObjRef {}

impl From<FuncObj> for FuncObjRef {
    fn from(value: FuncObj) -> Self {
        Self(value.into())
    }
}

impl Deref for FuncObjRef {
    type Target = FuncObj;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Value {
    pub fn new_null() -> Self {
        Self::Null
    }

    pub fn new_bool(value: bool) -> Self {
        Self::new_scalar(ScalarTy::Bool, value as u64)
    }

    pub fn new_sint(value: i64, width: Width) -> Self {
        let ty = ScalarTy::Int(width.as_int());
        Self::new_scalar(ty, u64::from_ne_bytes(i64::to_ne_bytes(value)))
    }

    pub fn new_uint(value: u64, width: Width) -> Self {
        let ty = ScalarTy::UInt(width.as_uint());
        Self::new_scalar(ty, value)
    }

    pub fn new_f32(value: f32) -> Self {
        Self::new_scalar(
            ScalarTy::Float(FloatTy::Float32),
            u32::from_ne_bytes(f32::to_ne_bytes(value)) as u64,
        )
    }

    pub fn new_f64(value: f64) -> Self {
        Self::new_scalar(
            ScalarTy::Float(FloatTy::Float64),
            u64::from_ne_bytes(f64::to_ne_bytes(value)),
        )
    }

    pub fn new_str(value: impl Into<ArcStr>) -> Self {
        Self::Str(value.into())
    }

    pub fn new_array<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        Self::Array(ThinArc::from_header_and_iter((), values.into_iter()))
    }

    pub fn new_func(value: FuncId) -> Self {
        Self::Func(value)
    }

    fn new_scalar(ty: ScalarTy, value: u64) -> Self {
        Self::Scalar { ty, value }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_bool(&self) -> bool {
        self.as_scalar() != 0
    }

    pub fn as_sint(&self) -> i64 {
        let value = self.as_scalar();
        i64::from_ne_bytes(u64::to_ne_bytes(value))
    }

    pub fn as_uint(&self) -> u64 {
        self.as_scalar()
    }

    pub fn as_f32(&self) -> f32 {
        let value = self.as_scalar();
        f32::from_ne_bytes(u32::to_ne_bytes(value as _))
    }

    pub fn as_f64(&self) -> f64 {
        let value = self.as_scalar();
        f64::from_ne_bytes(u64::to_ne_bytes(value))
    }

    pub fn as_str(&self) -> &ArcStr {
        match self {
            Self::Str(value) => value,
            _ => panic!("expected string value"),
        }
    }

    pub fn as_array(&self) -> &[Value] {
        match self {
            Self::Array(value) => &value.slice,
            _ => panic!("expected array value"),
        }
    }

    pub fn as_func(&self) -> FuncId {
        match self {
            Self::Func(value) => *value,
            _ => panic!("expected function value"),
        }
    }

    fn as_scalar(&self) -> u64 {
        match self {
            Self::Scalar { value, .. } => *value,
            _ => panic!("expected scalar value"),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Scalar { ty: ScalarTy::Bool, value: 0 } => write!(f, "false"),
            Self::Scalar { ty: ScalarTy::Bool, value: 1.. } => write!(f, "true"),
            Self::Scalar { ty: ScalarTy::Int(ty), value } => {
                let value = match ty {
                    IntTy::Int8 => (*value as u8) as u64,
                    IntTy::Int16 => (*value as u16) as u64,
                    IntTy::Int32 => (*value as u32) as u64,
                    IntTy::Int64 => (*value as u64) as u64,
                };
                write!(f, "{}", value)
            }
            Self::Scalar { ty: ScalarTy::UInt(ty), value } => {
                let value = match ty {
                    UIntTy::UInt8 => (*value as i8) as i64,
                    UIntTy::UInt16 => (*value as i16) as i64,
                    UIntTy::UInt32 => (*value as i32) as i64,
                    UIntTy::UInt64 => (*value as i64) as i64,
                };
                write!(f, "{}", value)
            }
            Self::Scalar { ty: ScalarTy::Float(ty), value } => match ty {
                FloatTy::Float32 => write!(f, "{}", f32::from_ne_bytes(u32::to_ne_bytes(*value as u32))),
                FloatTy::Float64 => write!(f, "{}", f64::from_ne_bytes(u64::to_ne_bytes(*value))),
            },
            Self::Str(value) => write!(f, "\"{}\"", escape::escape(value)),
            Self::Array(value) => match &value.slice {
                [] => write!(f, "[]"),
                [first, rest @ ..] => {
                    write!(f, "[{first}")?;
                    rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
                    write!(f, "]")
                }
            },
            Self::Func(value) => write!(f, "<function>"), // FIXME: function name
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub fn test_display_array() {
        let value = Value::new_array([
            Value::new_null(),
            Value::new_bool(true),
            Value::new_sint(123, Width::_32),
        ]);
        assert_eq!(format!("{value}"), "[null, true, 123]");
    }
}
