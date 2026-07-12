use std::hint::unreachable_unchecked;
use std::os::raw::c_void;
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

pub struct Value {
    #[cfg(target_pointer_width = "32")]
    half: u32,
    ptr: *const c_void,
}

enum Variant {
    Unit,
    Null,
    Bool(bool),
    Int48(i64),
    Float32(OrderedFloat<f32>),
    Float64(OrderedFloat<f64>),
    Func(FuncId),
    Boxed(*const c_void),
}

pub type Int = BigInt;

const NAN_MASK: u64 = 0x7FFF_0000_0000_0000;

impl Value {
    pub const fn new_null() -> Self {
        Self::from_variant(Variant::Null)
    }

    pub const fn new_unit() -> Self {
        Self::from_variant(Variant::Unit)
    }

    pub const fn new_bool(value: bool) -> Self {
        Self::from_variant(Variant::Bool(value))
    }

    pub fn new_int(value: Int) -> Self {
        const I48_MIN: i64 = -(1 << 47);
        const I48_MAX: i64 = (1 << 47) - 1;
        let variant = match i64::try_from(value) {
            Ok(v) if (I48_MIN..=I48_MAX).contains(&v) => Variant::Int48(v),
            _ => todo!(),
        };
        Self::from_variant(variant)
    }

    pub const fn new_f32(value: f32) -> Self {
        Self::from_variant(Variant::Float32(OrderedFloat(value)))
    }

    pub const fn new_f64(value: f64) -> Self {
        Self::from_variant(Variant::Float64(OrderedFloat(value)))
    }

    pub fn new_str(value: &str) -> Self {
        todo!()
    }

    pub fn new_array<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        // Self(ValueInner::Array(alloc))
        todo!()
    }

    pub fn new_tuple<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        // Self(ValueInner::Tuple(alloc))
        todo!()
    }

    pub fn new_func(func_id: FuncId) -> Self {
        Self::from_variant(Variant::Func(func_id))
    }

    pub fn as_bool(&self) -> bool {
        match self.variant() {
            Variant::Bool(v) => v,
            _ => panic!("expected a boolean value"),
        }
    }

    pub fn as_int(&self) -> &Int {
        match self.variant() {
            Variant::Int48(_) => todo!(),
            Variant::Boxed(_) => todo!(),
            _ => panic!("expected an integer value"),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self.variant() {
            Variant::Float32(v) => v.0,
            _ => panic!("expected an f32 value"),
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self.variant() {
            Variant::Float64(v) => v.0,
            _ => panic!("expected an f64 value"),
        }
    }

    pub fn as_func(&self) -> FuncId {
        match self.variant() {
            Variant::Func(v) => v,
            _ => panic!("expected a function value"),
        }
    }

    pub fn as_array(&self) -> &[Value] {
        match self.variant() {
            Variant::Boxed(_) => todo!(),
            // ValueInner::Array(alloc) => &alloc.slice,
            _ => panic!("expected an array value"),
        }
    }

    pub fn as_tuple(&self) -> &[Value] {
        match self.variant() {
            Variant::Boxed(_) => todo!(),
            // ValueInner::Tuple(alloc) => &alloc.slice,
            _ => panic!("expected a tuple value"),
        }
    }

    #[cfg(target_pointer_width = "64")]
    const fn from_variant(variant: Variant) -> Self {
        let data = match variant {
            Variant::Unit => (1 << 48) | 0,
            Variant::Null => (1 << 48) | 1,
            Variant::Bool(false) => (1 << 48) | 2,
            Variant::Bool(true) => (1 << 48) | 3,
            Variant::Int48(value) => (2 << 48) | (((value << 16) as u64) >> 16),
            Variant::Float32(value) => (3 << 48) | (value.0.to_bits() as u64),
            Variant::Float64(value) if value.0.is_nan() => (1 << 48) | 4,
            Variant::Float64(value) => value.0.to_bits() ^ NAN_MASK,
            Variant::Func(func_id) => (4 << 48) | (func_id.0 as u64),
            Variant::Boxed(ptr) => return Self { ptr },
        };

        Self { ptr: ptr::without_provenance(data as usize) }
    }

    #[cfg(target_pointer_width = "64")]
    fn variant(&self) -> Variant {
        let value = self.ptr.addr() as u64;
        match value >> 48 {
            0 => Variant::Boxed(self.ptr),
            1 => match value & 0xF {
                0 => Variant::Unit,
                1 => Variant::Null,
                2 => Variant::Bool(false),
                3 => Variant::Bool(true),
                4 => Variant::Float64(f64::NAN.into()),
                _ => {
                    debug_assert!(false, "invalid subtag {}", value & 0xF);
                    unsafe { unreachable_unchecked() }
                }
            },
            2 => Variant::Int48(((value << 16) as i64) >> 16),
            3 => Variant::Float32(f32::from_bits(value as u32).into()),
            4 => Variant::Func(FuncId(value as u32)),
            _ => Variant::Float64(f64::from_bits(value ^ NAN_MASK).into()),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.variant() {
            // ValueInner::Unit => write!(f, "()"),
            // ValueInner::Null => write!(f, "null"),
            // ValueInner::Bool(false) => write!(f, "false"),
            // ValueInner::Bool(true) => write!(f, "true"),
            // ValueInner::Int(value) => write!(f, "{}", value),
            // ValueInner::Float32(value) => write!(f, "{}", value),
            // ValueInner::Float64(value) => write!(f, "{}", value),
            // ValueInner::Str(value) => write!(f, "\"{}\"", escape::escape(value)),
            // ValueInner::Array(value) => match &value.slice {
            //     [] => write!(f, "[]"),
            //     [first, rest @ ..] => {
            //         write!(f, "[{first}")?;
            //         rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
            //         write!(f, "]")
            //     }
            // },
            // ValueInner::Tuple(value) => match &value.slice {
            //     [] => write!(f, "()"),
            //     [first] => write!(f, "({first},)"),
            //     [first, rest @ ..] => {
            //         write!(f, "({first}")?;
            //         rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
            //         write!(f, ")")
            //     }
            // },
            // ValueInner::Struct(value) => todo!(),
            // ValueInner::Func(value) => write!(f, "<func:{}>", value.0), // FIXME: function name
            _ => todo!(),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value({self})")
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self.variant() {
            Variant::Boxed(ptr) => todo!(),
            other => Self { ptr: self.ptr },
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.ptr == other.ptr {
            return true;
        }
        return false; // TODO: fix
    }
}

impl Eq for Value {}

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
