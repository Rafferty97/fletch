use bumpalo::Bump;

use super::ty::{FloatTy, IntTy, Ty, TyKind, UIntTy};
use crate::interner::Interner;

#[derive(Debug)]
pub struct TyInterners<'ty> {
    pub ty_kind: Interner<'ty, TyKind<'ty>>,
    pub ty_slice: Interner<'ty, [Ty<'ty>]>,
    pub common_types: CommonTypes<'ty>,
}

impl<'ty> TyInterners<'ty> {
    pub fn new(arena: &'ty Bump) -> Self {
        let ty_kind = Interner::new();
        let ty_slice = Interner::new();
        let common_types = CommonTypes::new(arena, &ty_kind, &ty_slice);

        Self { ty_kind, ty_slice, common_types }
    }
}

#[derive(Debug)]
pub struct CommonTypes<'ty> {
    // Top and bottom types
    pub never: Ty<'ty>,
    pub any: Ty<'ty>,
    pub opt_never: Ty<'ty>,
    pub opt_any: Ty<'ty>,
    // Bool
    pub bool: Ty<'ty>,
    pub opt_bool: Ty<'ty>,
    // Integers
    pub int8: Ty<'ty>,
    pub int16: Ty<'ty>,
    pub int32: Ty<'ty>,
    pub int64: Ty<'ty>,
    pub uint8: Ty<'ty>,
    pub uint16: Ty<'ty>,
    pub uint32: Ty<'ty>,
    pub uint64: Ty<'ty>,
    pub int: Ty<'ty>,
    pub opt_int8: Ty<'ty>,
    pub opt_int16: Ty<'ty>,
    pub opt_int32: Ty<'ty>,
    pub opt_int64: Ty<'ty>,
    pub opt_uint8: Ty<'ty>,
    pub opt_uint16: Ty<'ty>,
    pub opt_uint32: Ty<'ty>,
    pub opt_uint64: Ty<'ty>,
    pub opt_int: Ty<'ty>,
    // Floats
    pub float32: Ty<'ty>,
    pub float64: Ty<'ty>,
    pub opt_float32: Ty<'ty>,
    pub opt_float64: Ty<'ty>,
    // String
    pub str: Ty<'ty>,
    pub opt_str: Ty<'ty>,
    // Empty compound types
    pub empty_array: Ty<'ty>,
    pub empty_tuple: Ty<'ty>,
    pub opt_empty_array: Ty<'ty>,
    pub opt_empty_tuple: Ty<'ty>,
    // Sentinal types
    pub infer: Ty<'ty>,
    pub pending: Ty<'ty>,
    pub opt_infer: Ty<'ty>,
    pub opt_pending: Ty<'ty>,
}

impl<'ty> CommonTypes<'ty> {
    pub fn new(arena: &'ty Bump, ty_kind: &Interner<'ty, TyKind<'ty>>, ty_slice: &Interner<'ty, [Ty<'ty>]>) -> Self {
        use paste::paste;

        macro_rules! make_tys {
            ($($ty:ident $(?)? : $def:expr ,)*) => {
                paste! {
                    $(
                        let $ty = Ty(ty_kind.intern(arena, $def));
                        let [<opt_ $ty>] = Ty(ty_kind.intern(arena, TyKind::Nullable($ty)));
                    )*

                    CommonTypes {
                        $($ty, [<opt_ $ty>],)*
                    }
                }
            };
        }

        let empty_tys = ty_slice.intern_slice(arena, &[]);

        make_tys! {
            never: TyKind::Never,
            any: TyKind::Any,
            bool: TyKind::Bool,
            int8: TyKind::Int(IntTy::Int8),
            int16: TyKind::Int(IntTy::Int16),
            int32: TyKind::Int(IntTy::Int32),
            int64: TyKind::Int(IntTy::Int64),
            uint8: TyKind::UInt(UIntTy::UInt8),
            uint16: TyKind::UInt(UIntTy::UInt16),
            uint32: TyKind::UInt(UIntTy::UInt32),
            uint64: TyKind::UInt(UIntTy::UInt64),
            int: TyKind::Integer,
            float32: TyKind::Float(FloatTy::Float32),
            float64: TyKind::Float(FloatTy::Float64),
            str: TyKind::Str,
            empty_array: TyKind::Array(never),
            empty_tuple: TyKind::Tuple(empty_tys),
            infer: TyKind::Infer,
            pending: TyKind::Pending,
        }
    }

    /// Returns the unit type, which is the value of a block or function that produces no value.
    pub fn unit(&self) -> Ty<'ty> {
        self.empty_tuple
    }
}
