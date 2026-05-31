use bumpalo::Bump;

use super::{Ty, TyKind};
use crate::interner::Interner;

pub struct TyInterners<'ty> {
    pub ty_kind: Interner<'ty, TyKind<'ty>>,
    pub ty_slice: Interner<'ty, [Ty<'ty>]>,
    pub common_types: CommonTypes<'ty>,
}

pub struct CommonTypes<'ty> {
    pub never: Ty<'ty>,
    pub bool: Ty<'ty>,
    pub str: Ty<'ty>,
    pub opt_never: Ty<'ty>,
    pub opt_bool: Ty<'ty>,
    pub opt_str: Ty<'ty>,
    pub empty_array: Ty<'ty>,
    pub empty_tuple: Ty<'ty>,
    pub top: Ty<'ty>,
    pub err: Ty<'ty>,
}

impl<'ty> TyInterners<'ty> {
    pub fn new(arena: &'ty Bump) -> Self {
        // Create interners
        let ty_kind = Interner::new();
        let ty_slice = Interner::new();

        // Intern common types
        let mk_ty = |kind| Ty(ty_kind.intern(arena, kind));

        let never = mk_ty(TyKind::Never);
        let bool = mk_ty(TyKind::Bool);
        let str = mk_ty(TyKind::Str);
        let opt_never = mk_ty(TyKind::Nullable(never));
        let opt_bool = mk_ty(TyKind::Nullable(bool));
        let opt_str = mk_ty(TyKind::Nullable(str));
        let empty_array = mk_ty(TyKind::Array(never));
        let empty_ty_slice = ty_slice.intern_slice(arena, &[]);
        let empty_tuple = mk_ty(TyKind::Tuple(empty_ty_slice));
        let top = mk_ty(TyKind::Top);
        let err = mk_ty(TyKind::Err);

        let common_types = CommonTypes {
            never,
            bool,
            str,
            opt_never,
            opt_bool,
            opt_str,
            empty_array,
            empty_tuple,
            top,
            err,
        };

        Self {
            ty_kind,
            ty_slice,
            common_types,
        }
    }
}
