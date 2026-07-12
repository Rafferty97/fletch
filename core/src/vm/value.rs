use std::hint::unreachable_unchecked;
use std::ptr;
use std::rc::Rc;

use arcstr::ArcStr;
use num_bigint::BigInt;
use ordered_float::OrderedFloat;
use triomphe::{Arc, ThinArc};

use crate::ast::Symbol;
use crate::parser::escape;
use crate::thin_rc::{Head, ThinRc};
use crate::vm::chunk::Chunk;
use crate::vm::module::FuncId;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Value(ValueInner);

#[derive(Clone, PartialEq, Eq, Debug)]
enum ValueInner {
    Unit,
    Null,
    Bool(bool),
    Int(Int),
    Float32(OrderedFloat<f32>),
    Float64(OrderedFloat<f64>),
    Str(ArcStr),
    Array(ThinArc<(), Value>),
    Tuple(ThinArc<(), Value>),
    Struct(ThinArc<(), (Symbol, Value)>),
    Func(FuncId),
}

pub type Int = BigInt;

impl Value {
    pub const fn new_null() -> Self {
        Self(ValueInner::Null)
    }

    pub const fn new_unit() -> Self {
        Self(ValueInner::Unit)
    }

    pub const fn new_bool(value: bool) -> Self {
        Self(ValueInner::Bool(value))
    }

    pub const fn new_int(value: Int) -> Self {
        Self(ValueInner::Int(value))
    }

    pub const fn new_f32(value: f32) -> Self {
        Self(ValueInner::Float32(OrderedFloat(value)))
    }

    pub const fn new_f64(value: f64) -> Self {
        Self(ValueInner::Float64(OrderedFloat(value)))
    }

    pub fn new_str(value: &str) -> Self {
        Self(ValueInner::Str(value.into()))
    }

    pub fn new_array<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        Self(ValueInner::Array(alloc))
    }

    pub fn new_tuple<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        Self(ValueInner::Tuple(alloc))
    }

    pub fn new_func(func_id: FuncId) -> Self {
        Self(ValueInner::Func(func_id))
    }

    pub fn as_bool(&self) -> bool {
        match &self.0 {
            ValueInner::Bool(v) => *v,
            _ => panic!("expected a boolean value"),
        }
    }

    pub fn as_int(&self) -> &Int {
        match &self.0 {
            ValueInner::Int(v) => v,
            _ => panic!("expected an integer value"),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match &self.0 {
            ValueInner::Float32(v) => v.0,
            _ => panic!("expected an f32 value"),
        }
    }

    pub fn as_f64(&self) -> f64 {
        match &self.0 {
            ValueInner::Float64(v) => v.0,
            _ => panic!("expected an f64 value"),
        }
    }

    pub fn as_func(&self) -> FuncId {
        match &self.0 {
            ValueInner::Func(v) => *v,
            _ => panic!("expected a function value"),
        }
    }

    pub fn as_array(&self) -> &[Value] {
        match &self.0 {
            ValueInner::Array(alloc) => &alloc.slice,
            _ => panic!("expected an array value"),
        }
    }

    pub fn as_tuple(&self) -> &[Value] {
        match &self.0 {
            ValueInner::Tuple(alloc) => &alloc.slice,
            _ => panic!("expected a tuple value"),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ValueInner::Unit => write!(f, "()"),
            ValueInner::Null => write!(f, "null"),
            ValueInner::Bool(false) => write!(f, "false"),
            ValueInner::Bool(true) => write!(f, "true"),
            ValueInner::Int(value) => write!(f, "{}", value),
            ValueInner::Float32(value) => write!(f, "{}", value),
            ValueInner::Float64(value) => write!(f, "{}", value),
            ValueInner::Str(value) => write!(f, "\"{}\"", escape::escape(value)),
            ValueInner::Array(value) => match &value.slice {
                [] => write!(f, "[]"),
                [first, rest @ ..] => {
                    write!(f, "[{first}")?;
                    rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
                    write!(f, "]")
                }
            },
            ValueInner::Tuple(value) => match &value.slice {
                [] => write!(f, "()"),
                [first] => write!(f, "({first},)"),
                [first, rest @ ..] => {
                    write!(f, "({first}")?;
                    rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
                    write!(f, ")")
                }
            },
            ValueInner::Struct(value) => todo!(),
            ValueInner::Func(value) => write!(f, "<func:{}>", value.0), // FIXME: function name
        }
    }
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

impl std::ops::Deref for FuncObjRef {
    type Target = FuncObj;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

#[cfg(test)]
mod test {
    use crate::vm::value::Value;

    #[test]
    fn roundtrip_primitive_values() {
        assert_eq!(Value::new_unit().to_string(), "()");
        assert_eq!(Value::new_null().to_string(), "null");
        assert_eq!(Value::new_bool(false).to_string(), "false");
        assert_eq!(Value::new_bool(true).to_string(), "true");
        assert_eq!(Value::new_int(0.into()).to_string(), "0");
        assert_eq!(Value::new_int(1.into()).to_string(), "1");
        assert_eq!(Value::new_int(2.into()).to_string(), "2");
        assert_eq!(Value::new_int(i32::MAX.into()).to_string(), i32::MAX.to_string());
        assert_eq!(Value::new_int(i32::MIN.into()).to_string(), i32::MIN.to_string());
        assert_eq!(Value::new_int(i64::MAX.into()).to_string(), i64::MAX.to_string());
        assert_eq!(Value::new_int(i64::MIN.into()).to_string(), i64::MIN.to_string());
        assert_eq!(Value::new_int(i128::MAX.into()).to_string(), i128::MAX.to_string());
        assert_eq!(Value::new_int(i128::MIN.into()).to_string(), i128::MIN.to_string());
        assert_eq!(Value::new_str("").to_string(), r#""""#);
        assert_eq!(Value::new_str("hello world").to_string(), r#""hello world""#);
        assert_eq!(Value::new_str("hello\nworld").to_string(), r#""hello\nworld""#);
        assert_eq!(Value::new_f32(0.0).to_string(), "0");
        assert_eq!(Value::new_f32(1.0).to_string(), "1");
        assert_eq!(Value::new_f32(10.0).to_string(), "10");
        assert_eq!(Value::new_f32(123.25).to_string(), "123.25");
        assert_eq!(Value::new_f64(0.0).to_string(), "0");
        assert_eq!(Value::new_f64(1.0).to_string(), "1");
        assert_eq!(Value::new_f64(10.0).to_string(), "10");
        assert_eq!(Value::new_f64(123.25).to_string(), "123.25");
    }
}
