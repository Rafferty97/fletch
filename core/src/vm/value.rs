use std::hint::unreachable_unchecked;
use std::ptr;
use std::rc::Rc;

use arcstr::ArcStr;
use triomphe::ThinArc;

use crate::ast::Symbol;
use crate::thin_rc::{Head, ThinRc};

pub struct Value {
    tag: Tag,
    data: Data,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Unit,
    Bool,
    Integer,
    Float32,
    Float64,
    Str,
    Array,
    Tuple,
    Struct,
}

struct Data {
    #[cfg(target_pointer_width = "32")]
    half: u32,
    ptr: *const (),
}

impl Value {
    fn new_null(tag: Tag) -> Self {
        Self { tag, data: Data::new_null() }
    }

    #[inline(always)]
    fn from_variant<V: Variant>(v: V) -> Self {
        Value { tag: V::TAG, data: v.into_raw() }
    }

    #[inline(always)]
    fn drop_inner<V: Variant>(&mut self) {
        assert_eq!(self.tag, V::TAG);
        let data = unsafe { ptr::read(&self.data) };
        std::mem::drop(unsafe { V::from_raw(data) });
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        match self.tag {
            Tag::Unit | Tag::Bool | Tag::Float32 | Tag::Float64 => {}
            Tag::Integer => self.drop_inner::<Integer>(),
            Tag::Str => todo!(),
            Tag::Array => todo!(),
            Tag::Tuple => todo!(),
            Tag::Struct => todo!(),
        }
    }
}

impl Data {
    const fn new_null() -> Self {
        Self::from_ptr(ptr::null::<()>())
    }

    const fn from_u64(value: u64) -> Self {
        Self {
            #[cfg(target_pointer_width = "32")]
            half: (value >> 32) as u32,
            ptr: ptr::without_provenance(value as usize),
        }
    }

    const fn from_ptr<T>(ptr: *const T) -> Self {
        Self {
            #[cfg(target_pointer_width = "32")]
            half: 0,
            ptr: ptr.cast(),
        }
    }

    fn as_u64(&self) -> u64 {
        let value = self.ptr.addr() as u64;
        #[cfg(target_pointer_width = "32")]
        let value = value | (self.half << 32);
        value
    }

    fn as_ptr<T>(&self) -> *const T {
        self.ptr.cast()
    }

    fn as_mut_ptr<T: Sized>(&self) -> *mut T {
        self.ptr.cast_mut().cast()
    }
}

trait Variant {
    const TAG: Tag;

    unsafe fn from_raw(value: Data) -> Self;

    fn into_raw(self) -> Data;
}

#[derive(Default)]
struct Bool(bool);

impl Variant for Bool {
    const TAG: Tag = Tag::Bool;

    unsafe fn from_raw(value: Data) -> Self {
        match value.as_u64() {
            1 => Self(false),
            2 => Self(true),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn into_raw(self) -> Data {
        let value = match self.0 {
            false => 1,
            true => 2,
        };
        Data::from_u64(value)
    }
}

struct Integer(*const ());

impl Variant for Integer {
    const TAG: Tag = Tag::Integer;

    unsafe fn from_raw(value: Data) -> Self {
        todo!()
    }

    fn into_raw(self) -> Data {
        todo!()
    }
}

struct Float64(f64);

impl Float64 {
    pub fn new(value: f64) -> Self {
        if value.to_bits() == 0 {
            Self(f64::from_bits(1 << 63))
        } else {
            Self(value)
        }
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Variant for Float64 {
    const TAG: Tag = Tag::Float64;

    unsafe fn from_raw(value: Data) -> Self {
        Self(f64::from_bits(value.as_u64()))
    }

    fn into_raw(self) -> Data {
        Data::from_u64(self.0.to_bits())
    }
}

impl Variant for ArcStr {
    const TAG: Tag = Tag::Str;

    unsafe fn from_raw(value: Data) -> Self {
        unsafe {
            let ptr = ptr::NonNull::new_unchecked(value.as_mut_ptr());
            ArcStr::from_raw(ptr)
        }
    }

    fn into_raw(self) -> Data {
        Data::from_ptr(ArcStr::into_raw(self).as_ptr())
    }
}

struct TupleHead {
    fields: Rc<[Tag]>,
}

impl Head for TupleHead {
    type T = Data;

    fn tail_len(&self) -> usize {
        self.fields.len()
    }

    fn drop_tail(&mut self, tail: &mut [Data]) {
        for (&tag, data) in self.fields.iter().zip(tail.iter()) {
            let data = unsafe { std::ptr::read(data) };
            let value = Value { tag, data };
            std::mem::drop(value);
        }
    }
}

struct Tuple(ThinRc<TupleHead>);

impl Variant for Tuple {
    const TAG: Tag = Tag::Tuple;

    unsafe fn from_raw(value: Data) -> Self {
        todo!()
    }

    fn into_raw(self) -> Data {
        todo!()
    }
}

struct Struct {
    fields: Rc<[Field]>,
    values: [Data],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Field {
    name: Symbol,
    tag: Tag,
}
