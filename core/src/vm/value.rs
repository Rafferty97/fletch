use std::ffi::c_void;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, offset_of};
use std::ops::Deref;
use std::slice;

use triomphe::{Arc, OffsetArc, ThinArc};

use crate::types::ty::{FloatTy, IntTy, UIntTy};
use crate::vm::{chunk::Chunk, instr::Width, module::FuncId};

///////

pub struct Value {
    #[cfg(target_pointer_width = "32")]
    half: u32,
    ptr: *const c_void,
}

enum ValueKind<'a> {
    Null,
    Scalar(Scalar),
    BoxedScalar(&'a BoxedScalar),
    Str(&'a BoxedStr),
    Values(&'a BoxedValues),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
struct Scalar(u64);

#[repr(transparent)]
struct BoxedScalar(ThinArc<Head, u64>);

#[repr(transparent)]
struct BoxedStr(ThinArc<Head, u8>);

#[repr(transparent)]
struct BoxedValues(ThinArc<Head, Value>);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Head {
    Scalar,
    Str,
    Values,
}

trait IntoObject {
    type Item;

    fn head(&self) -> Head;

    fn tail(&self) -> impl Iterator<Item = Self::Item> + ExactSizeIterator;
}

impl Default for Value {
    fn default() -> Self {
        Self::new_scalar(Scalar::default())
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for Value {}

impl Value {
    pub fn new_u32(value: u32) -> Self {
        Self::new_u64(value as u64)
    }

    pub fn new_u64(value: u64) -> Self {
        match Scalar::try_from(value) {
            Ok(scalar) => Self::new_scalar(scalar),
            Err(_) => Self::new_boxed(value),
        }
    }

    pub fn new_f32(value: f32) -> Self {
        Self::new_u32(u32::from_ne_bytes(value.to_ne_bytes()))
    }

    pub fn new_f64(value: f64) -> Self {
        Self::new_u64(u64::from_ne_bytes(value.to_ne_bytes()))
    }

    pub fn new_str(value: &str) -> Self {
        Self::new_boxed(value)
    }

    #[inline(always)]
    pub fn new_null() -> Self {
        Self {
            #[cfg(target_pointer_width = "32")]
            half: 0,
            ptr: std::ptr::null(),
        }
    }

    #[inline(always)]
    fn new_scalar(value: Scalar) -> Self {
        Self {
            #[cfg(target_pointer_width = "32")]
            half: (value.0 >> 32) as u32,
            ptr: std::ptr::without_provenance(value.0 as usize),
        }
    }

    fn new_boxed(value: impl IntoObject) -> Self {
        let ptr = ThinArc::from_header_and_iter(value.head(), value.tail());
        let ptr = ptr.into_raw();
        assert_eq!(ptr.addr() & 1, 0);
        Self {
            #[cfg(target_pointer_width = "32")]
            half: 0,
            ptr,
        }
    }

    pub fn as_null(&self) -> () {
        debug_assert!(matches!(self.kind(), ValueKind::Null));
    }

    pub fn as_u32(&self) -> u32 {
        self.as_u64() as u32
    }

    pub fn as_u64(&self) -> u64 {
        match self.kind() {
            ValueKind::Scalar(scalar) => scalar.value(),
            ValueKind::BoxedScalar(boxed) => boxed.0.slice[0],
            _ => panic!("unexpected value variant"),
        }
    }

    pub fn as_f32(&self) -> f32 {
        f32::from_ne_bytes(self.as_u32().to_ne_bytes())
    }

    pub fn as_f64(&self) -> f64 {
        f64::from_ne_bytes(self.as_u64().to_ne_bytes())
    }

    pub fn as_str(&self) -> &str {
        let ValueKind::Str(boxed) = self.kind() else {
            panic!("unexpected value variant");
        };
        unsafe { str::from_utf8_unchecked(&boxed.0.slice) }
    }

    #[inline(always)]
    fn kind(&self) -> ValueKind<'_> {
        if (self.ptr.addr() & 1) != 0 {
            let value = self.ptr.addr() as u64;
            #[cfg(target_pointer_width = "32")]
            let value = value | (self.half << 32);
            ValueKind::Scalar(Scalar(value))
        } else if self.ptr.is_null() {
            ValueKind::Null
        } else {
            let offset = offset_of!(<ThinArc<Head, ()> as Deref>::Target, header.header);
            let head = unsafe { &*(self.ptr.byte_add(offset) as *const Head) };
            match head {
                Head::Scalar => ValueKind::BoxedScalar(unsafe { std::mem::transmute(&self.ptr) }),
                Head::Str => ValueKind::Str(unsafe { std::mem::transmute(&self.ptr) }),
                Head::Values => ValueKind::Values(unsafe { std::mem::transmute(&self.ptr) }),
            }
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        match self.kind() {
            ValueKind::Null => {}
            ValueKind::Scalar(_) => {}
            ValueKind::BoxedScalar(boxed) => unsafe { std::mem::forget(std::ptr::read(boxed)) },
            ValueKind::Str(boxed) => unsafe { std::mem::forget(std::ptr::read(boxed)) },
            ValueKind::Values(boxed) => unsafe { std::mem::forget(std::ptr::read(boxed)) },
        }
    }
}

impl Default for Scalar {
    fn default() -> Self {
        Self(1)
    }
}

impl TryFrom<u64> for Scalar {
    type Error = ();

    fn try_from(value: u64) -> Result<Self, ()> {
        if ((value << 1) >> 1) == value {
            Ok(Self((value << 1) | 1))
        } else {
            Err(())
        }
    }
}

impl Scalar {
    pub const fn value(self) -> u64 {
        self.0 >> 1
    }
}

impl IntoObject for u64 {
    type Item = u64;

    fn head(&self) -> Head {
        Head::Scalar
    }

    fn tail(&self) -> impl Iterator<Item = u64> + ExactSizeIterator {
        std::iter::once(*self)
    }
}

impl IntoObject for &str {
    type Item = u8;

    fn head(&self) -> Head {
        Head::Str
    }

    fn tail(&self) -> impl Iterator<Item = u8> + ExactSizeIterator {
        self.bytes()
    }
}

///////

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

impl std::ops::Deref for FuncObjRef {
    type Target = FuncObj;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Value {
    pub fn new_unit() -> Self {
        todo!()
    }

    pub fn new_bool(value: bool) -> Self {
        todo!()
    }

    pub fn new_sint(value: i64, width: Width) -> Self {
        todo!()
    }

    pub fn new_uint(value: u64, width: Width) -> Self {
        todo!()
    }

    pub fn new_array<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        todo!()
    }

    pub fn new_tuple<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
        I::IntoIter: ExactSizeIterator,
    {
        todo!()
    }

    pub fn new_func(value: FuncId) -> Self {
        todo!()
    }

    pub fn is_null(&self) -> bool {
        matches!(self.kind(), ValueKind::Null)
    }

    pub fn as_bool(&self) -> bool {
        todo!()
    }

    pub fn as_sint(&self) -> i64 {
        todo!()
    }

    pub fn as_uint(&self) -> u64 {
        todo!()
    }

    pub fn as_array(&self) -> &[Value] {
        todo!()
    }

    pub fn as_tuple(&self) -> &[Value] {
        todo!()
    }

    pub fn as_func(&self) -> FuncId {
        todo!()
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip_primitives() {
        let null = Value::new_null();
        assert!(null.is_null());

        let null = Value::new_u64(123);
        assert_eq!(null.as_u64(), 123);

        let null = Value::new_f32(12.3456);
        assert_eq!(null.as_f32(), 12.3456);

        let null = Value::new_f64(12.3456);
        assert_eq!(null.as_f64(), 12.3456);
    }
}
