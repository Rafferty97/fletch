use std::sync::Arc;

use indexmap::IndexMap;

use crate::arena::Symbol;

#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Str(Arc<str>),
    Array(Arc<[Value]>),
    Tuple(Arc<[Value]>),
    Struct(IndexMap<Symbol, Value>),
    Variant(Symbol, Arc<Value>),
    Func(FuncId),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FuncId(u32);
