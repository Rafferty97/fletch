use std::hint::unreachable_unchecked;
use std::mem::{self, ManuallyDrop, offset_of};
use std::ops::Deref;
use std::os::raw::c_void;
use std::ptr;
use std::rc::Rc;

use num_bigint::{BigInt, Sign};
use ordered_float::OrderedFloat;
use triomphe::{Arc, ThinArc};

use crate::ast::Symbol;
use crate::interner::IndexTable;
use crate::parser::{SymTable, escape};
use crate::thin_rc::{Head, ThinRc};
use crate::vm::chunk::Chunk;
use crate::vm::module::FuncId;

pub struct Value {
    #[cfg(target_pointer_width = "32")]
    half: u32,
    ptr: *const c_void,
}

#[derive(PartialEq, Eq, Debug)]
enum Variant {
    Unit,
    Null,
    Bool(bool),
    Int48(i64),
    Float32(OrderedFloat<f32>),
    Float64(OrderedFloat<f64>),
    Func(FuncId),
    Str(ArcStr),
    BigInt(ThinArc<Sign, u32>),
    Array(ThinArc<(), Value>),
    Tuple(ThinArc<(), Value>),
    Struct(ThinArc<(), (Symbol, Value)>),
    Variant(Arc<(Symbol, Value)>),
}

enum VariantRef<'a> {
    Unit,
    Null,
    Bool(bool),
    Int48(i64),
    Float32(OrderedFloat<f32>),
    Float64(OrderedFloat<f64>),
    Func(FuncId),
    Str(&'a <ArcStr as Deref>::Target),
    BigInt(&'a <ThinArc<Sign, u32> as Deref>::Target),
    Array(&'a <ThinArc<(), Value> as Deref>::Target),
    Tuple(&'a <ThinArc<(), Value> as Deref>::Target),
    Struct(&'a <ThinArc<(), (Symbol, Value)> as Deref>::Target),
    Variant(&'a <Arc<(Symbol, Value)> as Deref>::Target),
}

pub type Int = BigInt;

const NAN_MASK: u64 = 0x7FFF_0000_0000_0000;
const CANONICAL_NAN: u64 = 0x7FF0_0000_0000_0001;

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
        let variant = match i64::try_from(&value) {
            Ok(value) if (I48_MIN..=I48_MAX).contains(&value) => Variant::Int48(value),
            _ => {
                let (sign, digits) = value.to_u32_digits();
                Variant::BigInt(ThinArc::from_header_and_iter(sign, digits.into_iter()))
            }
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
        Self::from_variant(Variant::Str(value.into()))
    }

    pub fn new_array<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        Self::from_variant(Variant::Array(alloc))
    }

    pub fn new_tuple<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        let alloc = ThinArc::from_header_and_iter((), values.into_iter());
        Self::from_variant(Variant::Tuple(alloc))
    }

    pub fn new_variant(tag: Symbol, inner: Value) -> Self {
        Self::from_variant(Variant::Variant(Arc::new((tag, inner))))
    }

    pub fn new_func(func_id: FuncId) -> Self {
        Self::from_variant(Variant::Func(func_id))
    }

    pub fn as_bool(&self) -> bool {
        match self.variant_ref() {
            VariantRef::Bool(v) => v,
            _ => panic!("expected a boolean value"),
        }
    }

    pub fn as_int(&self) -> Int {
        match self.variant_ref() {
            VariantRef::Int48(value) => Int::from(value),
            VariantRef::BigInt(boxed) => Int::from_slice(boxed.header.header, &boxed.slice),
            _ => panic!("expected an integer value"),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self.variant_ref() {
            VariantRef::Float32(v) => v.0,
            _ => panic!("expected an f32 value"),
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self.variant_ref() {
            VariantRef::Float64(v) => v.0,
            _ => panic!("expected an f64 value"),
        }
    }

    pub fn as_func(&self) -> FuncId {
        match self.variant_ref() {
            VariantRef::Func(v) => v,
            _ => panic!("expected a function value"),
        }
    }

    pub fn as_array(&self) -> &[Value] {
        match self.variant_ref() {
            VariantRef::Array(boxed) => &boxed.slice,
            _ => panic!("expected an array value"),
        }
    }

    pub fn as_tuple(&self) -> &[Value] {
        match self.variant_ref() {
            VariantRef::Tuple(boxed) => &boxed.slice,
            _ => panic!("expected a tuple value"),
        }
    }

    fn new_inline(tag: u64, payload: u64) -> Self {
        let value = (tag << 48) | payload;
        Self {
            #[cfg(target_pointer_width = "32")]
            half: (value >> 32) as u32,
            ptr: ptr::without_provenance(value as usize),
        }
    }

    fn new_float(value: f64) -> Self {
        let value = match value.is_nan() {
            true => CANONICAL_NAN ^ NAN_MASK,
            false => value.to_bits() ^ NAN_MASK,
        };
        Self {
            #[cfg(target_pointer_width = "32")]
            half: (value >> 32) as u32,
            ptr: ptr::without_provenance(value as usize),
        }
    }

    #[cfg(target_pointer_width = "32")]
    fn new_boxed<T>(tag: usize, ptr: *const T) -> Self {
        Value { half: (tag as u32) << 16, ptr: ptr.cast() }
    }

    #[cfg(target_pointer_width = "64")]
    fn new_boxed<T>(tag: usize, ptr: *const T) -> Self {
        let ptr = ptr.map_addr(|a| a | (tag << 48)).cast();
        Value { ptr }
    }

    fn from_variant(variant: Variant) -> Self {
        match variant {
            Variant::Unit => Self::new_inline(0, 0),
            Variant::Null => Self::new_inline(1, 0),
            Variant::Bool(value) => Self::new_inline(2, value as u64),
            Variant::Int48(value) => Self::new_inline(3, ((value << 16) as u64) >> 16),
            Variant::Float32(value) => Self::new_inline(4, value.0.to_bits() as u64),
            Variant::Float64(value) => Self::new_float(*value),
            Variant::Func(func_id) => Self::new_inline(5, func_id.0 as u64),
            Variant::Str(value) => Self::new_boxed(6, value.0.into_raw()),
            Variant::BigInt(value) => Self::new_boxed(7, value.into_raw()),
            Variant::Array(value) => Self::new_boxed(8, value.into_raw()),
            Variant::Tuple(value) => Self::new_boxed(9, value.into_raw()),
            Variant::Struct(value) => Self::new_boxed(10, value.into_raw()),
            Variant::Variant(value) => Self::new_boxed(11, Arc::into_raw(value)),
        }
    }

    fn variant(&self) -> ManuallyDrop<Variant> {
        #[cfg(target_pointer_width = "32")]
        let value = ((self.half as u64) << 32) | (self.ptr.addr() as u64);
        #[cfg(target_pointer_width = "64")]
        let value = self.ptr.addr() as u64;

        #[cfg(target_pointer_width = "32")]
        let ptr = self.ptr;
        #[cfg(target_pointer_width = "64")]
        let ptr = self.ptr.map_addr(|a| a & 0xffff_ffff_ffff);

        let variant = match value >> 48 {
            0 => Variant::Unit,
            1 => Variant::Null,
            2 => Variant::Bool(value & 1 != 0),
            3 => Variant::Int48(((value << 16) as i64) >> 16),
            4 => Variant::Float32(f32::from_bits(value as u32).into()),
            5 => Variant::Func(FuncId(value as u32)),
            6 => unsafe { Variant::Str(ArcStr(ThinArc::from_raw(ptr))) },
            7 => unsafe { Variant::BigInt(ThinArc::from_raw(ptr)) },
            8 => unsafe { Variant::Array(ThinArc::from_raw(ptr)) },
            9 => unsafe { Variant::Tuple(ThinArc::from_raw(ptr)) },
            10 => unsafe { Variant::Struct(ThinArc::from_raw(ptr)) },
            11 => unsafe { Variant::Variant(Arc::from_raw(ptr.cast())) },
            _ => Variant::Float64(f64::from_bits(value ^ NAN_MASK).into()),
        };
        ManuallyDrop::new(variant)
    }

    fn variant_ref<'a>(&'a self) -> VariantRef<'a> {
        match &*self.variant() {
            Variant::Unit => VariantRef::Unit,
            Variant::Null => VariantRef::Null,
            Variant::Bool(value) => VariantRef::Bool(*value),
            Variant::Int48(value) => VariantRef::Int48(*value),
            Variant::Float32(value) => VariantRef::Float32(*value),
            Variant::Float64(value) => VariantRef::Float64(*value),
            Variant::Func(value) => VariantRef::Func(*value),
            Variant::Str(arc) => unsafe {
                let slice = &*(&arc.0.slice as *const _);
                VariantRef::Str(str::from_utf8_unchecked(slice))
            },
            Variant::BigInt(arc) => unsafe { VariantRef::BigInt(&*(&**arc as *const _)) },
            Variant::Array(arc) => unsafe { VariantRef::Array(&*(&**arc as *const _)) },
            Variant::Tuple(arc) => unsafe { VariantRef::Tuple(&*(&**arc as *const _)) },
            Variant::Struct(arc) => unsafe { VariantRef::Struct(&*(&**arc as *const _)) },
            Variant::Variant(arc) => unsafe { VariantRef::Variant(&*(&**arc as *const _)) },
        }
    }

    pub fn display_ctx<'a>(&'a self, sym_table: &'a SymTable<'a>) -> ValueWithCtx<'a> {
        ValueWithCtx { value: self, sym_table }
    }

    pub(crate) fn raw(&self) -> u64 {
        #[cfg(target_pointer_width = "32")]
        return (self.ptr.addr() as u64) | ((self.half as u64) << 32);
        #[cfg(target_pointer_width = "64")]
        return self.ptr.addr() as u64;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ValueWithCtx<'a> {
    value: &'a Value,
    sym_table: &'a SymTable<'a>,
}

impl<'a> ValueWithCtx<'a> {
    fn derive<'b>(self, value: &'b Value) -> ValueWithCtx<'b>
    where
        'a: 'b,
    {
        ValueWithCtx { value, ..self }
    }
}

impl std::fmt::Display for ValueWithCtx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value.variant_ref() {
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
                    write!(f, "[{}", self.derive(first))?;
                    rest.iter().try_for_each(|v| write!(f, ", {}", self.derive(v)))?;
                    write!(f, "]")
                }
            },
            VariantRef::Tuple(boxed) => match &boxed.slice {
                [] => write!(f, "()"),
                [first] => write!(f, "({},)", self.derive(first)),
                [first, rest @ ..] => {
                    write!(f, "({}", self.derive(first))?;
                    rest.iter().try_for_each(|v| write!(f, ", {}", self.derive(v)))?;
                    write!(f, ")")
                }
            },
            VariantRef::Struct(boxed) => match &boxed.slice {
                [] => write!(f, "{{}}"),
                [first, rest @ ..] => {
                    write!(f, "{{ {}: {}", self.sym_table.get_str(first.0), self.derive(&first.1))?;
                    rest.iter()
                        .try_for_each(|v| write!(f, ", {}: {}", self.sym_table.get_str(v.0), self.derive(&v.1)))?;
                    write!(f, " }}")
                }
            },
            VariantRef::Variant(boxed) => {
                let (tag, value) = &*boxed;
                let tag = self.sym_table.get_str(*tag);
                write!(f, "{}({})", tag, self.derive(value))
            }
            VariantRef::Func(boxed) => write!(f, "<func:{}>", boxed.0), // FIXME: function name
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value({:?})", self.variant())
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match &*self.variant() {
            Variant::Str(boxed) => mem::forget(boxed.clone()),
            Variant::BigInt(boxed) => mem::forget(boxed.clone()),
            Variant::Array(boxed) => mem::forget(boxed.clone()),
            Variant::Tuple(boxed) => mem::forget(boxed.clone()),
            Variant::Struct(boxed) => mem::forget(boxed.clone()),
            _ => {}
        }
        Self {
            #[cfg(target_pointer_width = "32")]
            half: self.half,
            ptr: self.ptr,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.ptr == other.ptr {
            return true;
        }
        return self.variant() == other.variant();
    }
}

impl Eq for Value {}

impl Drop for Value {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.variant()) }
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

#[derive(PartialEq, Eq, Clone, Debug)]
struct ArcStr(ThinArc<(), u8>);

impl From<&str> for ArcStr {
    fn from(value: &str) -> Self {
        Self(ThinArc::from_header_and_slice((), value.as_bytes()))
    }
}

impl std::ops::Deref for ArcStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { str::from_utf8_unchecked(&self.0.slice) }
    }
}

#[cfg(test)]
mod test {
    use crate::interner::IndexTable;
    use crate::vm::value::Value;

    #[test]
    fn roundtrip_primitive_values() {
        let sym_table = &IndexTable::empty();
        let print = |value: Value| value.display_ctx(sym_table).to_string();

        assert_eq!(print(Value::new_unit()), "()");
        assert_eq!(print(Value::new_null()), "null");
        assert_eq!(print(Value::new_bool(false)), "false");
        assert_eq!(print(Value::new_bool(true)), "true");
        assert_eq!(print(Value::new_int(0.into())), "0");
        assert_eq!(print(Value::new_int(1.into())), "1");
        assert_eq!(print(Value::new_int(2.into())), "2");
        assert_eq!(print(Value::new_int(i32::MAX.into())), i32::MAX.to_string());
        assert_eq!(print(Value::new_int(i32::MIN.into())), i32::MIN.to_string());
        assert_eq!(print(Value::new_int(i64::MAX.into())), i64::MAX.to_string());
        assert_eq!(print(Value::new_int(i64::MIN.into())), i64::MIN.to_string());
        assert_eq!(print(Value::new_int(i128::MAX.into())), i128::MAX.to_string());
        assert_eq!(print(Value::new_int(i128::MIN.into())), i128::MIN.to_string());
        assert_eq!(print(Value::new_str("")), r#""""#);
        assert_eq!(print(Value::new_str("hello world")), r#""hello world""#);
        assert_eq!(print(Value::new_str("hello\nworld")), r#""hello\nworld""#);
        assert_eq!(print(Value::new_f32(0.0)), "0");
        assert_eq!(print(Value::new_f32(1.0)), "1");
        assert_eq!(print(Value::new_f32(10.0)), "10");
        assert_eq!(print(Value::new_f32(123.25)), "123.25");
        assert_eq!(print(Value::new_f64(0.0)), "0");
        assert_eq!(print(Value::new_f64(1.0)), "1");
        assert_eq!(print(Value::new_f64(10.0)), "10");
        assert_eq!(print(Value::new_f64(123.25)), "123.25");
    }
}
