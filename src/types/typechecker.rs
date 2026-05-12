use std::marker::PhantomData;

use ena::unify::{InPlace, NoError, UnificationTable, UnifyKey, UnifyValue};
use hashbrown::HashMap;

use crate::arena::Ctx;
use crate::types::{FieldList, RowVar, Ty, TyKind, TyVar};

pub struct TypecheckCtx<'cx> {
    ctx: Ctx<'cx>,
    table: UnificationTable<InPlace<TyVar<'cx>>>,
    row_table: UnificationTable<InPlace<RowVar<'cx>>>,
}

impl<'cx> TypecheckCtx<'cx> {
    pub fn new(ctx: Ctx<'cx>) -> Self {
        Self {
            ctx,
            table: UnificationTable::new(),
            row_table: UnificationTable::new(),
        }
    }

    pub fn new_ty_var(&mut self) -> Ty<'cx> {
        let tyvar = self.table.new_key(None);
        Ty(self.ctx.intern_ty_kind(TyKind::Infer(tyvar)))
    }

    pub fn new_row_var(&mut self) -> RowVar<'cx> {
        self.row_table.new_key(None)
    }

    pub fn unify(&mut self, a: Ty<'cx>, b: Ty<'cx>) -> Result<(), String> {
        if a == b {
            return Ok(());
        }

        match (a.kind(), b.kind()) {
            (TyKind::Infer(a), TyKind::Infer(b)) => {
                match (self.table.probe_value(*a), self.table.probe_value(*b)) {
                    (Some(a_ty), Some(b_ty)) => self.unify(a_ty, b_ty),
                    (Some(a_ty), None) => Ok(self.table.union_value(*b, Some(a_ty))),
                    (None, Some(b_ty)) => Ok(self.table.union_value(*a, Some(b_ty))),
                    (None, None) => Ok(self.table.union(*a, *b)),
                }
            }
            (TyKind::Infer(a), _) => match self.table.probe_value(*a) {
                Some(a_ty) => self.unify(a_ty, b),
                None => Ok(self.table.union_value(*a, Some(b))),
            },
            (_, TyKind::Infer(b)) => match self.table.probe_value(*b) {
                Some(b_ty) => self.unify(a, b_ty),
                None => Ok(self.table.union_value(*b, Some(a))),
            },
            (TyKind::Array(a), TyKind::Array(b)) => self.unify(*a, *b),
            (TyKind::Tuple(a), TyKind::Tuple(b)) => {
                let (a, b) = (a.tys(), b.tys());
                if a.len() != b.len() {
                    return Err(format!("cannot unify {:?} and {:?}", a, b));
                }
                for (a, b) in a.iter().zip(b) {
                    self.unify(*a, *b)?;
                }
                Ok(())
            }
            (TyKind::Struct(a, a_tail), TyKind::Struct(b, b_tail)) => self.unify_rows(
                RowValue { fields: *a, tail: *a_tail },
                RowValue { fields: *b, tail: *b_tail },
            ),
            (TyKind::Enum(a), TyKind::Enum(b)) => self.unify_fields(*a, *b),
            _ => Err(format!("cannot unify {:?} and {:?}", a, b)),
        }
    }

    fn unify_rows(&mut self, a: RowValue<'cx>, b: RowValue<'cx>) -> Result<(), String> {
        let RowValue { fields: a, tail: a_tail } = a;
        let RowValue { fields: b, tail: b_tail } = b;

        match (a_tail, b_tail) {
            (None, None) => self.unify_fields(a, b),
            (Some(a_tail), None) => {
                let fields = self.subtype_fields(a, b)?;
                let a_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(a_tail) {
                    None => Ok(self.row_table.union_value(a_tail, Some(a_tail_value))),
                    Some(a_tail) => self.unify_rows(a_tail, a_tail_value),
                }
            }
            (None, Some(b_tail)) => {
                let fields = self.subtype_fields(b, a)?;
                let b_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(b_tail) {
                    None => Ok(self.row_table.union_value(b_tail, Some(b_tail_value))),
                    Some(b_tail) => self.unify_rows(b_tail, b_tail_value),
                }
            }
            (Some(a_tail), Some(b_tail)) => {
                let common_tail = self.new_row_var();
                let (a_fields, b_fields) = self.intersect_fields(a, b)?;
                let a_tail_value = RowValue { fields: b_fields, tail: Some(common_tail) };
                let b_tail_value = RowValue { fields: a_fields, tail: Some(common_tail) };
                match self.row_table.probe_value(a_tail) {
                    None => self.row_table.union_value(a_tail, Some(a_tail_value)),
                    Some(a_tail) => self.unify_rows(a_tail, a_tail_value)?,
                }
                match self.row_table.probe_value(b_tail) {
                    None => self.row_table.union_value(b_tail, Some(b_tail_value)),
                    Some(b_tail) => self.unify_rows(b_tail, b_tail_value)?,
                }
                Ok(())
            }
        }
    }

    pub fn resolve(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, String> {
        match ty.kind() {
            TyKind::Bool => Ok(ty),
            TyKind::Int(_) | TyKind::UInt(_) => Ok(ty),
            TyKind::Str => Ok(ty),
            TyKind::Array(inner) => {
                let inner = self.resolve(*inner)?;
                Ok(Ty(self.ctx.intern_ty_kind(TyKind::Array(inner))))
            }
            TyKind::Infer(var) => match self.table.probe_value(*var) {
                Some(ty) => self.resolve(ty),
                _ => Err(format!("unresolved type")),
            },
            _ => todo!("{ty:?}"),
        }
    }

    pub fn resolve_row(&mut self, var: RowVar<'cx>) -> Result<RowValue<'cx>, String> {
        match self.row_table.probe_value(var) {
            Some(value) => Ok(value),
            None => Err("unbound row var")?,
        }
    }

    fn unify_fields(&mut self, a: FieldList<'cx>, b: FieldList<'cx>) -> Result<(), String> {
        let mut fields: HashMap<_, _> = a.fields().iter().copied().collect();

        for &(name, ty) in b.fields() {
            match fields.remove(&name) {
                Some(lhs_ty) => self.unify(lhs_ty, ty)?,
                None => Err("mismatched keys")?,
            }
        }

        if !fields.is_empty() {
            Err("mismatched keys")?;
        }

        Ok(())
    }

    fn subtype_fields(
        &mut self,
        subset: FieldList<'cx>,
        superset: FieldList<'cx>,
    ) -> Result<FieldList<'cx>, String> {
        let mut fields: HashMap<_, _> = superset.fields().iter().copied().collect();

        for &(name, ty) in subset.fields() {
            match fields.remove(&name) {
                Some(lhs_ty) => self.unify(lhs_ty, ty)?,
                None => Err("mismatched keys")?,
            }
        }

        let fields = fields.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();

        Ok(FieldList(self.ctx.intern_fields(&fields)))
    }

    fn intersect_fields(
        &mut self,
        a: FieldList<'cx>,
        b: FieldList<'cx>,
    ) -> Result<(FieldList<'cx>, FieldList<'cx>), String> {
        let mut a_fields: HashMap<_, _> = a.fields().iter().copied().collect();
        let mut b_fields = HashMap::new();

        for &(name, ty) in b.fields() {
            match a_fields.remove(&name) {
                Some(lhs_ty) => self.unify(lhs_ty, ty)?,
                None => {
                    b_fields.insert(name, ty);
                }
            }
        }

        let a_fields = a_fields.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        let b_fields = b_fields.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();

        Ok((
            FieldList(self.ctx.intern_fields(&a_fields)),
            FieldList(self.ctx.intern_fields(&b_fields)),
        ))
    }
}

impl<'cx> UnifyKey for TyVar<'cx> {
    type Value = Option<Ty<'cx>>;

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(u: u32) -> Self {
        Self(u, PhantomData)
    }

    fn tag() -> &'static str {
        "TyVar"
    }
}

impl<'cx> UnifyValue for Ty<'cx> {
    type Error = NoError;

    fn unify_values(_: &Self, _: &Self) -> Result<Self, Self::Error> {
        panic!("this should never be called")
    }
}

impl<'cx> UnifyKey for RowVar<'cx> {
    type Value = Option<RowValue<'cx>>;

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(u: u32) -> Self {
        Self(u, PhantomData)
    }

    fn tag() -> &'static str {
        "RowVar"
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RowValue<'cx> {
    fields: FieldList<'cx>,
    tail: Option<RowVar<'cx>>,
}

impl<'cx> UnifyValue for RowValue<'cx> {
    type Error = NoError;

    fn unify_values(_: &Self, _: &Self) -> Result<Self, Self::Error> {
        panic!("this should never be called")
    }
}

#[cfg(test)]
mod test {
    use crate::{
        arena, diagnostics,
        types::{FieldList, IntTy, Ty, TyKind, TyList},
    };
    use bumpalo::Bump;

    macro_rules! setup {
        ($arena:ident, $ctx:ident, $tc:ident) => {
            let $arena = Bump::new();
            let mut handler = diagnostics::Diagnostics::new();
            let mut $ctx = arena::Ctx::new(&$arena, &mut handler);
            let mut $tc = super::TypecheckCtx::new($ctx);
        };
    }

    // Two variables unified with each other, then one with a concrete type.
    // Tests that ena's union-find chasing works correctly.
    #[test]
    fn unify_var_var_then_concrete() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let var2 = tc.new_ty_var();
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        tc.unify(var1, var2).unwrap();
        tc.unify(var2, str).unwrap();
        assert_eq!(tc.resolve(var1).unwrap(), str);
    }

    // Unifying two different concrete types should fail.
    #[test]
    fn unify_concrete_mismatch() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        assert!(tc.unify(str, int).is_err());
    }

    // A variable nested inside a concrete type gets resolved correctly.
    // Tests the structural recursion in resolve.
    #[test]
    fn resolve_nested_var() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let var2 = tc.new_ty_var();
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let array_var1 = Ty(ctx.intern_ty_kind(TyKind::Array(var1)));
        let array_var2 = Ty(ctx.intern_ty_kind(TyKind::Array(var2)));
        let array_str = Ty(ctx.intern_ty_kind(TyKind::Array(str)));
        // var1 and var2 are linked, then var2 is resolved to Str
        tc.unify(array_var1, array_var2).unwrap();
        tc.unify(var2, str).unwrap();
        assert_eq!(tc.resolve(array_var1).unwrap(), array_str);
    }

    // Unifying two structs with matching fields unifies their field types.
    #[test]
    fn unify_structs() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let name = ctx.intern_str("name");
        let age = ctx.intern_str("age");
        let struct_a = ctx.intern_fields(&[(name, var1), (age, int)]);
        let struct_a = Ty(ctx.intern_ty_kind(TyKind::Struct(FieldList(struct_a), None)));
        let struct_b = ctx.intern_fields(&[(name, str), (age, int)]);
        let struct_b = Ty(ctx.intern_ty_kind(TyKind::Struct(FieldList(struct_b), None)));
        tc.unify(struct_a, struct_b).unwrap();
        assert_eq!(tc.resolve(var1).unwrap(), str);
    }

    // Unifying two structs with different field names should fail.
    #[test]
    fn unify_structs_field_mismatch() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let name = ctx.intern_str("name");
        let title = ctx.intern_str("title");
        let struct_a = ctx.intern_fields(&[(name, str)]);
        let struct_a = Ty(ctx.intern_ty_kind(TyKind::Struct(FieldList(struct_a), None)));
        let struct_b = ctx.intern_fields(&[(title, str)]);
        let struct_b = Ty(ctx.intern_ty_kind(TyKind::Struct(FieldList(struct_b), None)));
        assert!(tc.unify(struct_a, struct_b).is_err());
    }

    // Resolving an unbound variable should fail.
    #[test]
    fn resolve_unbound_var() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        assert!(tc.resolve(var1).is_err());
    }

    // Unifying a variable with itself should succeed trivially.
    #[test]
    fn unify_var_with_itself() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        assert!(tc.unify(var1, var1).is_ok());
    }

    // A chain of three variables, all eventually resolving to the same concrete type.
    #[test]
    fn unify_var_chain() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let var2 = tc.new_ty_var();
        let var3 = tc.new_ty_var();
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        tc.unify(var1, var2).unwrap();
        tc.unify(var2, var3).unwrap();
        tc.unify(var3, str).unwrap();
        assert_eq!(tc.resolve(var1).unwrap(), str);
        assert_eq!(tc.resolve(var2).unwrap(), str);
        assert_eq!(tc.resolve(var3).unwrap(), str);
    }

    // Unifying a tuple with mismatched arity should fail.
    #[test]
    fn unify_tuple_arity_mismatch() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let tuple_a = ctx.intern_tys(&[str, int]);
        let tuple_a = Ty(ctx.intern_ty_kind(TyKind::Tuple(TyList(tuple_a))));
        let tuple_b = ctx.intern_tys(&[str]);
        let tuple_b = Ty(ctx.intern_ty_kind(TyKind::Tuple(TyList(tuple_b))));
        assert!(tc.unify(tuple_a, tuple_b).is_err());
    }

    macro_rules! mk_struct {
        ($ctx:expr, $tc:expr, [$(($name:expr, $ty:expr)),+], $tail:expr) => {{
            let fields = [$(($ctx.intern_str($name), $ty)),+];
            let r = $ctx.intern_fields(&fields);
            Ty($ctx.intern_ty_kind(TyKind::Struct(FieldList(r), $tail)))
        }};
    }

    // A closed struct unifies with itself
    #[test]
    fn unify_closed_struct() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let struct_a = mk_struct!(ctx, tc, [("name", str)], None);
        let struct_b = mk_struct!(ctx, tc, [("name", str)], None);
        assert!(tc.unify(struct_a, struct_b).is_ok());
    }

    // An open struct unifies with a closed struct that is a superset of its fields.
    // The row variable gets bound to the extra fields.
    #[test]
    fn unify_open_struct_with_closed_superset() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let age = ctx.intern_str("age");
        let row = tc.new_row_var();
        let open = mk_struct!(ctx, tc, [("name", str)], Some(row));
        let closed = mk_struct!(ctx, tc, [("name", str), ("age", int)], None);
        assert!(tc.unify(open, closed).is_ok());
        // row variable should now be bound to { age: Int }
        let resolved_row = tc.resolve_row(row).unwrap();
        assert_eq!(resolved_row.fields, FieldList(ctx.intern_fields(&[(age, int)])));
        assert_eq!(resolved_row.tail, None);
    }

    // An open struct cannot unify with a closed struct that is missing fields.
    #[test]
    fn unify_open_struct_with_closed_subset_fails() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row = tc.new_row_var();
        let open = mk_struct!(ctx, tc, [("name", str), ("age", int)], Some(row));
        let closed = mk_struct!(ctx, tc, [("name", str)], None);
        assert!(tc.unify(open, closed).is_err());
    }

    // Two open structs unify — each tail gets the other's unique fields,
    // and both share a common tail for unknown remaining fields.
    #[test]
    fn unify_two_open_structs() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let age = ctx.intern_str("age");
        let name = ctx.intern_str("name");
        let row_a = tc.new_row_var();
        let row_b = tc.new_row_var();
        // a has "name", b has "age", neither knows about the other's fields
        let open_a = mk_struct!(ctx, tc, [("name", str)], Some(row_a));
        let open_b = mk_struct!(ctx, tc, [("age", int)], Some(row_b));
        assert!(tc.unify(open_a, open_b).is_ok());
        // row_a should be bound to { age: Int | common_tail }
        let resolved_a = tc.resolve_row(row_a).unwrap();
        assert_eq!(resolved_a.fields, FieldList(ctx.intern_fields(&[(age, int)])));
        assert!(resolved_a.tail.is_some());
        // row_b should be bound to { name: Str | common_tail }
        let resolved_b = tc.resolve_row(row_b).unwrap();
        assert_eq!(resolved_b.fields, FieldList(ctx.intern_fields(&[(name, str)])));
        assert!(resolved_b.tail.is_some());
        // both tails should share the same common tail
        assert_eq!(resolved_a.tail, resolved_b.tail);
    }

    // A function accepting an open struct can be called with a closed superset.
    // Models the core row polymorphism use case.
    #[test]
    fn row_polymorphic_function_call() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row = tc.new_row_var();
        // function expects { name: Str | ρ }
        let param_ty = mk_struct!(ctx, tc, [("name", str)], Some(row));
        // caller passes { name: Str, age: Int }
        let arg_ty = mk_struct!(ctx, tc, [("name", str), ("age", int)], None);
        // unifying argument with parameter type
        assert!(tc.unify(arg_ty, param_ty).is_ok());
        // row variable should be bound to the extra fields
        let resolved_row = tc.resolve_row(row).unwrap();
        let age = ctx.intern_str("age");
        assert_eq!(resolved_row.fields, FieldList(ctx.intern_fields(&[(age, int)])));
        assert_eq!(resolved_row.tail, None);
    }

    // Field type mismatch in open struct unification should fail.
    #[test]
    fn unify_open_struct_field_type_mismatch() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row = tc.new_row_var();
        let open = mk_struct!(ctx, tc, [("name", str)], Some(row));
        let closed = mk_struct!(ctx, tc, [("name", int), ("age", int)], None);
        assert!(tc.unify(open, closed).is_err());
    }

    // Two open structs with a conflicting field type should fail.
    #[test]
    fn unify_two_open_structs_field_type_mismatch() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row_a = tc.new_row_var();
        let row_b = tc.new_row_var();
        // both have "name" but with different types
        let open_a = mk_struct!(ctx, tc, [("name", str)], Some(row_a));
        let open_b = mk_struct!(ctx, tc, [("name", int)], Some(row_b));
        assert!(tc.unify(open_a, open_b).is_err());
    }
}
