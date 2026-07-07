use std::ffi::c_void;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::Deref;

use triomphe::{Arc, ThinArc};

use crate::types::ty::{FloatTy, IntTy, UIntTy};
use crate::vm::{chunk::Chunk, instr::Width, module::FuncId};

///////

#[repr(transparent)]
pub struct Value(*const c_void);

enum ValueKind {
    Scalar(Scalar),
    Object(Object),
}

enum ValueKindRef<'a> {
    Scalar(Scalar),
    Object(ObjectRef<'a>),
}

#[repr(transparent)]
struct Scalar(u64);

#[repr(transparent)]
struct Object(ThinArc<Head, u64>);

#[repr(transparent)]
struct ObjectRef<'a>(&'a <ThinArc<Head, u64> as Deref>::Target);

// ManuallyDrop<ThinArc<Head, u64>>

#[derive(PartialEq, Eq, Clone, Debug)]
enum Head {
    // todo
}

impl Default for Value {
    fn default() -> Self {
        Self::new_scalar(0)
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
    #[inline(always)]
    pub fn new_scalar(value: u64) -> Self {
        // FIXME: box if high-bit is `1`
        let value = (value << 1) | 1;
        assert_eq!(value >> 1, value);
        let value = value.try_into().unwrap();
        Self(std::ptr::without_provenance(value))
    }

    #[inline(always)]
    fn is_scalar(&self) -> bool {
        (self.0.addr() & 1) != 0
    }

    #[inline(always)]
    fn is_ptr(&self) -> bool {
        !self.is_scalar()
    }

    #[inline(always)]
    fn as_kind(&self) -> ValueKindRef<'_> {
        match self.is_scalar() {
            true => ValueKindRef::Scalar(self.as_scalar()),
            // false => {
            //     let ptr = unsafe { ManuallyDrop::new(ThinArc::from_raw(self.0)) };
            //     ValueKindRef::Object(ObjectRef(&**ptr))
            // }
            false => todo!(),
        }
    }

    #[inline(always)]
    fn into_kind(self) -> ValueKind {
        match self.is_scalar() {
            true => ValueKind::Scalar(self.as_scalar()),
            false => {
                let ptr = unsafe { ThinArc::from_raw(self.0) };
                ValueKind::Object(Object(ptr))
            }
        }
    }

    #[inline(always)]
    fn as_scalar(&self) -> Scalar {
        Scalar(self.0.addr().try_into().unwrap())
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        let this = std::mem::take(self);
        std::mem::drop(this.into_kind())
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

    pub fn new_null() -> Self {
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

    pub fn new_f32(value: f32) -> Self {
        todo!()
    }

    pub fn new_f64(value: f64) -> Self {
        todo!()
    }

    pub fn new_str(value: impl Into<String>) -> Self {
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
        todo!()
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

    pub fn as_f32(&self) -> f32 {
        todo!()
    }

    pub fn as_f64(&self) -> f64 {
        todo!()
    }

    pub fn as_str(&self) -> &str {
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
