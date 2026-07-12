use std::hint::unreachable_unchecked;
use std::mem::{ManuallyDrop, offset_of};
use std::ops::Deref;
use std::os::raw::c_void;
use std::ptr;
use std::rc::Rc;

use num_bigint::{BigInt, Sign};
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
    Str(ThinArc<(), u8>),
    BigInt(ThinArc<Sign, u32>),
    Array(ThinArc<(), Value>),
    Tuple(ThinArc<(), Value>),
    Struct(ThinArc<(), (Symbol, Value)>),
}

enum VariantRef<'a> {
    Unit,
    Null,
    Bool(bool),
    Int48(i64),
    Float32(OrderedFloat<f32>),
    Float64(OrderedFloat<f64>),
    Func(FuncId),
    Str(&'a str),
    BigInt(&'a <ThinArc<Sign, u32> as Deref>::Target),
    Array(&'a <ThinArc<(), Value> as Deref>::Target),
    Tuple(&'a <ThinArc<(), Value> as Deref>::Target),
    Struct(&'a <ThinArc<(), (Symbol, Value)> as Deref>::Target),
}

pub type Int = BigInt;

const NAN_MASK: u64 = 0x7FFF_0000_0000_0000;
const CANONICAL_NAN: f64 = f64::from_bits(0x7FF0_0000_0000_0001);

impl Value {
    pub fn new_null() -> Self {
        Self::from_variant(Variant::Null)
    }

    pub fn new_unit() -> Self {
        Self::from_variant(Variant::Unit)
    }

    pub fn new_bool(value: bool) -> Self {
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

    pub fn new_f32(value: f32) -> Self {
        Self::from_variant(Variant::Float32(OrderedFloat(value)))
    }

    pub fn new_f64(value: f64) -> Self {
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
        // Self(VariantRef::Array(alloc))
        todo!()
    }

    pub fn new_tuple<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        // Self(VariantRef::Tuple(alloc))
        todo!()
    }

    pub fn new_func(func_id: FuncId) -> Self {
        Self::from_variant(Variant::Func(func_id))
    }

    pub fn as_bool(&self) -> bool {
        match self.variant() {
            VariantRef::Bool(v) => v,
            _ => panic!("expected a boolean value"),
        }
    }

    pub fn as_int(&self) -> Int {
        match self.variant() {
            VariantRef::Int48(value) => Int::from(value),
            VariantRef::BigInt(boxed) => Int::from_slice(boxed.header.header, &boxed.slice),
            _ => panic!("expected an integer value"),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self.variant() {
            VariantRef::Float32(v) => v.0,
            _ => panic!("expected an f32 value"),
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self.variant() {
            VariantRef::Float64(v) => v.0,
            _ => panic!("expected an f64 value"),
        }
    }

    pub fn as_func(&self) -> FuncId {
        match self.variant() {
            VariantRef::Func(v) => v,
            _ => panic!("expected a function value"),
        }
    }

    pub fn as_array(&self) -> &[Value] {
        match self.variant() {
            VariantRef::Array(boxed) => &boxed.slice,
            _ => panic!("expected an array value"),
        }
    }

    pub fn as_tuple(&self) -> &[Value] {
        match self.variant() {
            VariantRef::Tuple(boxed) => &boxed.slice,
            _ => panic!("expected a tuple value"),
        }
    }

    #[cfg(target_pointer_width = "64")]
    fn from_variant(variant: Variant) -> Self {
        fn inline(tag: u64, payload: u64) -> *const c_void {
            ptr::without_provenance(((tag << 48) | payload) as usize)
        }

        fn boxed<T>(tag: usize, ptr: *const T) -> *const c_void {
            ptr.map_addr(|a| a | (tag << 48)).cast()
        }

        let ptr = match variant {
            Variant::Unit => inline(0, 0),
            Variant::Null => inline(1, 0),
            Variant::Bool(value) => inline(2, value as u64),
            Variant::Int48(value) => inline(3, ((value << 16) as u64) >> 16),
            Variant::Float32(value) => inline(4, value.0.to_bits() as u64),
            Variant::Float64(value) => {
                let value = match value.is_nan() {
                    true => CANONICAL_NAN,
                    false => value.0,
                };
                ptr::without_provenance((value.to_bits() ^ NAN_MASK) as _)
            }
            Variant::Func(func_id) => inline(5, func_id.0 as u64),
            Variant::Str(value) => boxed(6, value.into_raw()),
            Variant::BigInt(value) => boxed(7, value.into_raw()),
            Variant::Array(value) => boxed(8, value.into_raw()),
            Variant::Tuple(value) => boxed(9, value.into_raw()),
            Variant::Struct(value) => boxed(10, value.into_raw()),
        };

        Self { ptr }
    }

    #[cfg(target_pointer_width = "64")]
    fn variant<'a>(&'a self) -> VariantRef<'a> {
        let value = self.ptr.addr() as u64;
        let ptr = self.ptr.map_addr(|a| a & 0xffff_ffff_ffff);
        match value >> 48 {
            0 => VariantRef::Unit,
            1 => VariantRef::Null,
            2 => VariantRef::Bool(value & 1 != 0),
            3 => VariantRef::Int48(((value << 16) as i64) >> 16),
            4 => VariantRef::Float32(f32::from_bits(value as u32).into()),
            5 => VariantRef::Func(FuncId(value as u32)),
            6 => unsafe {
                let arc = ManuallyDrop::new(ThinArc::<(), _>::from_raw(ptr));
                let slice = &*(&arc.slice as *const _);
                VariantRef::Str(str::from_utf8_unchecked(slice))
            },
            7 => unsafe {
                let arc = ManuallyDrop::new(ThinArc::from_raw(ptr));
                let slice = &*(&**arc as *const _);
                VariantRef::BigInt(slice)
            },
            8 => unsafe {
                let arc = ManuallyDrop::new(ThinArc::from_raw(ptr));
                let slice = &*(&**arc as *const _);
                VariantRef::Array(slice)
            },
            9 => unsafe {
                let arc = ManuallyDrop::new(ThinArc::from_raw(ptr));
                let slice = &*(&**arc as *const _);
                VariantRef::Tuple(slice)
            },
            10 => unsafe {
                let arc = ManuallyDrop::new(ThinArc::from_raw(ptr));
                let slice = &*(&**arc as *const _);
                VariantRef::Struct(slice)
            },
            _ => VariantRef::Float64(f64::from_bits(value ^ NAN_MASK).into()),
        }
    }

    // fn foo(&self) {
    //     let offset = offset_of!(<ThinArc<(), u8> as Deref>::Target, slice[0]);
    //     let head = unsafe { &*(self.ptr.byte_add(offset) as *const Head) };
    //     match head {
    //         Head::Scalar => ValueKind::BoxedScalar(unsafe { std::mem::transmute(&self.ptr) }),
    //         Head::Str => ValueKind::Str(unsafe { std::mem::transmute(&self.ptr) }),
    //         Head::Values => ValueKind::Values(unsafe { std::mem::transmute(&self.ptr) }),
    //     }
    // }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.variant() {
            VariantRef::Unit => write!(f, "()"),
            VariantRef::Null => write!(f, "null"),
            VariantRef::Bool(false) => write!(f, "false"),
            VariantRef::Bool(true) => write!(f, "true"),
            VariantRef::Int48(value) => write!(f, "{}", value),
            VariantRef::Float32(value) => write!(f, "{}", value),
            VariantRef::Float64(value) => write!(f, "{}", value),
            VariantRef::Str(boxed) => write!(f, "\"{}\"", escape::escape(boxed)),
            VariantRef::BigInt(boxed) => write!(f, "{}", BigInt::from_slice(boxed.header.header, &boxed.slice)),
            VariantRef::Array(boxed) => match &boxed.slice {
                [] => write!(f, "[]"),
                [first, rest @ ..] => {
                    write!(f, "[{first}")?;
                    rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
                    write!(f, "]")
                }
            },
            VariantRef::Tuple(boxed) => match &boxed.slice {
                [] => write!(f, "()"),
                [first] => write!(f, "({first},)"),
                [first, rest @ ..] => {
                    write!(f, "({first}")?;
                    rest.iter().try_for_each(|v| write!(f, ", {v}"))?;
                    write!(f, ")")
                }
            },
            VariantRef::Struct(boxed) => todo!(),
            VariantRef::Func(boxed) => write!(f, "<func:{}>", boxed.0), // FIXME: function name
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
            VariantRef::Str(_) => todo!(),
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
