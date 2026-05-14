use std::marker::PhantomData;

use ena::unify::{InPlace, NoError, UnificationTable, UnifyKey, UnifyValue};
use hashbrown::HashMap;

use crate::arena::{Ctx, Symbol};
use crate::types::fold::TyFolder;
use crate::types::{
    FieldList, FloatTy, FloatVar, IntTy, IntVar, RowVar, Ty, TyKind, TyVar, UIntTy,
};

pub struct TypecheckCtx<'cx> {
    ctx: Ctx<'cx>,
    ty_table: UnificationTable<InPlace<TyVar<'cx>>>,
    int_table: UnificationTable<InPlace<IntVar<'cx>>>,
    float_table: UnificationTable<InPlace<FloatVar<'cx>>>,
    row_table: UnificationTable<InPlace<RowVar<'cx>>>,
    scopes: Vec<HashMap<Symbol, Ty<'cx>>>,
}

impl<'cx> TypecheckCtx<'cx> {
    pub fn new(ctx: Ctx<'cx>) -> Self {
        Self {
            ctx,
            ty_table: UnificationTable::new(),
            int_table: UnificationTable::new(),
            float_table: UnificationTable::new(),
            row_table: UnificationTable::new(),
            scopes: vec![HashMap::new()],
        }
    }

    pub fn bind_variable(&mut self, name: Symbol, ty: Ty<'cx>) {
        self.curr_scope_mut().insert(name, ty);
    }

    pub fn get_variable(&self, name: Symbol) -> Result<Ty<'cx>, String> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name).copied())
            .ok_or_else(|| format!("unknown identifier: {}", self.ctx.get_str(name)))
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn curr_scope(&self) -> &HashMap<Symbol, Ty<'cx>> {
        self.scopes.last().unwrap()
    }

    fn curr_scope_mut(&mut self) -> &mut HashMap<Symbol, Ty<'cx>> {
        self.scopes.last_mut().unwrap()
    }

    pub fn new_ty_var(&mut self) -> Ty<'cx> {
        let ty_var = self.ty_table.new_key(None);
        Ty(self.ctx.intern_ty_kind(TyKind::TyVar(ty_var)))
    }

    pub fn new_int_var(&mut self) -> Ty<'cx> {
        let int_var = self.int_table.new_key(None);
        Ty(self.ctx.intern_ty_kind(TyKind::IntVar(int_var)))
    }

    pub fn new_float_var(&mut self) -> Ty<'cx> {
        let float_var = self.float_table.new_key(None);
        Ty(self.ctx.intern_ty_kind(TyKind::FloatVar(float_var)))
    }

    pub fn new_row_var(&mut self) -> RowVar<'cx> {
        self.row_table.new_key(None)
    }

    pub fn coerce(&mut self, from: Ty<'cx>, to: Ty<'cx>) -> Result<(), String> {
        if from == to {
            return Ok(());
        }

        match (from.kind(), to.kind()) {
            (TyKind::TyVar(from), TyKind::TyVar(to)) => {
                match (self.ty_table.probe_value(*from), self.ty_table.probe_value(*to)) {
                    (Some(from), Some(to)) => self.coerce(from, to),
                    (Some(from), None) => self.union_value(*to, from),
                    (None, Some(to)) => self.union_value(*from, to),
                    (None, None) => Ok(self.ty_table.union(*from, *to)),
                }
            }
            (TyKind::TyVar(from), _) => match self.ty_table.probe_value(*from) {
                Some(from) => self.coerce(from, to),
                None => self.union_value(*from, to),
            },
            (_, TyKind::TyVar(to)) => match self.ty_table.probe_value(*to) {
                Some(to) => self.coerce(from, to),
                None => self.union_value(*to, from),
            },
            (TyKind::IntVar(a), TyKind::IntVar(b)) => self.int_table.unify_var_var(*a, *b),
            (TyKind::IntVar(k), TyKind::Int(t)) | (TyKind::Int(t), TyKind::IntVar(k)) => {
                self.int_table.unify_var_value(*k, Some(IntValue::Int(*t)))
            }
            (TyKind::IntVar(k), TyKind::UInt(t)) | (TyKind::UInt(t), TyKind::IntVar(k)) => {
                self.int_table.unify_var_value(*k, Some(IntValue::UInt(*t)))
            }
            (TyKind::FloatVar(a), TyKind::FloatVar(b)) => self.float_table.unify_var_var(*a, *b),
            (TyKind::FloatVar(k), TyKind::Float(t)) | (TyKind::Float(t), TyKind::FloatVar(k)) => {
                self.float_table.unify_var_value(*k, Some(*t))
            }
            (TyKind::Array(a), TyKind::Array(b)) => self.coerce(*a, *b),
            (TyKind::Tuple(a), TyKind::Tuple(b)) => {
                if a.len() != b.len() {
                    return Err(format!("cannot coerce {:?} to {:?}", a, b));
                }
                for (a, b) in a.iter().zip(b.iter()) {
                    self.coerce(*a, *b)?;
                }
                Ok(())
            }
            (TyKind::Struct(from, from_tail), TyKind::Struct(to, to_tail)) => self.coerce_struct(
                RowValue { fields: *from, tail: *from_tail },
                RowValue { fields: *to, tail: *to_tail },
            ),
            (TyKind::Enum(from, from_tail), TyKind::Enum(to, to_tail)) => self.coerce_variant(
                RowValue { fields: *from, tail: *from_tail },
                RowValue { fields: *to, tail: *to_tail },
            ),
            (TyKind::Nullable(from), TyKind::Nullable(to)) => self.coerce(*from, *to),
            (_, TyKind::Nullable(to)) => self.coerce(from, *to),
            _ => Err(format!("cannot coerce {from:?} into {to:?}")),
        }
    }

    fn coerce_struct(&mut self, from: RowValue<'cx>, to: RowValue<'cx>) -> Result<(), String> {
        let RowValue { fields: from, tail: from_tail } = from;
        let RowValue { fields: to, tail: to_tail } = to;

        match (from_tail, to_tail) {
            (None, None) => {
                let _ = self.superset_fields(from, to, Self::coerce)?;
                Ok(())
            }

            (Some(from_tail), None) => {
                let (_, fields) = self.intersect_fields(from, to, Self::coerce)?;
                let from_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(from_tail) {
                    None => Ok(self.row_table.union_value(from_tail, Some(from_tail_value))),
                    Some(from_tail) => self.coerce_struct(from_tail, from_tail_value),
                }
            }

            (None, Some(to_tail)) => {
                let fields = self.superset_fields(from, to, Self::coerce)?;
                let to_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(to_tail) {
                    None => Ok(self.row_table.union_value(to_tail, Some(to_tail_value))),
                    Some(to_tail) => self.coerce_struct(to_tail, to_tail_value),
                }
            }

            (Some(from_tail), Some(to_tail)) => {
                let common_tail = self.new_row_var();
                let (a_fields, b_fields) = self.intersect_fields(from, to, Self::coerce)?;
                let a_tail_value = RowValue { fields: b_fields, tail: Some(common_tail) };
                let b_tail_value = RowValue { fields: a_fields, tail: Some(common_tail) };
                match self.row_table.probe_value(from_tail) {
                    None => self.row_table.union_value(from_tail, Some(a_tail_value)),
                    Some(a_tail) => self.coerce_struct(a_tail, a_tail_value)?,
                }
                match self.row_table.probe_value(to_tail) {
                    None => self.row_table.union_value(to_tail, Some(b_tail_value)),
                    Some(b_tail) => self.coerce_struct(b_tail, b_tail_value)?,
                }
                Ok(())
            }
        }
    }

    fn coerce_variant(&mut self, from: RowValue<'cx>, to: RowValue<'cx>) -> Result<(), String> {
        let RowValue { fields: from, tail: from_tail } = from;
        let RowValue { fields: to, tail: to_tail } = to;

        match (from_tail, to_tail) {
            (None, None) => {
                let _ = self.subset_fields(from, to, Self::coerce)?;
                Ok(())
            }

            (Some(from_tail), None) => {
                let fields = self.subset_fields(from, to, Self::coerce)?;
                let from_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(from_tail) {
                    None => Ok(self.row_table.union_value(from_tail, Some(from_tail_value))),
                    Some(from_tail) => self.unify_rows(from_tail, from_tail_value),
                }
            }

            (None, Some(to_tail)) => {
                let (fields, _) = self.intersect_fields(from, to, Self::coerce)?;
                let to_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(to_tail) {
                    None => Ok(self.row_table.union_value(to_tail, Some(to_tail_value))),
                    Some(to_tail) => self.coerce_struct(to_tail, to_tail_value),
                }
            }

            (Some(from_tail), Some(to_tail)) => {
                let common_tail = self.new_row_var();
                let (a_fields, b_fields) = self.intersect_fields(from, to, Self::coerce)?;
                let a_tail_value = RowValue { fields: b_fields, tail: Some(common_tail) };
                let b_tail_value = RowValue { fields: a_fields, tail: Some(common_tail) };
                match self.row_table.probe_value(from_tail) {
                    None => self.row_table.union_value(from_tail, Some(a_tail_value)),
                    Some(a_tail) => self.coerce_struct(a_tail, a_tail_value)?,
                }
                match self.row_table.probe_value(to_tail) {
                    None => self.row_table.union_value(to_tail, Some(b_tail_value)),
                    Some(b_tail) => self.coerce_struct(b_tail, b_tail_value)?,
                }
                Ok(())
            }
        }
    }

    pub fn unify(&mut self, a: Ty<'cx>, b: Ty<'cx>) -> Result<(), String> {
        if a == b {
            return Ok(());
        }

        match (a.kind(), b.kind()) {
            (TyKind::TyVar(a), TyKind::TyVar(b)) => {
                match (self.ty_table.probe_value(*a), self.ty_table.probe_value(*b)) {
                    (Some(a_ty), Some(b_ty)) => self.unify(a_ty, b_ty),
                    (Some(a_ty), None) => self.union_value(*b, a_ty),
                    (None, Some(b_ty)) => self.union_value(*a, b_ty),
                    (None, None) => Ok(self.ty_table.union(*a, *b)),
                }
            }
            (TyKind::TyVar(a), _) => match self.ty_table.probe_value(*a) {
                Some(a_ty) => self.unify(a_ty, b),
                None => self.union_value(*a, b),
            },
            (_, TyKind::TyVar(b)) => match self.ty_table.probe_value(*b) {
                Some(b_ty) => self.unify(a, b_ty),
                None => self.union_value(*b, a),
            },
            (TyKind::IntVar(a), TyKind::IntVar(b)) => self.int_table.unify_var_var(*a, *b),
            (TyKind::IntVar(k), TyKind::Int(t)) | (TyKind::Int(t), TyKind::IntVar(k)) => {
                self.int_table.unify_var_value(*k, Some(IntValue::Int(*t)))
            }
            (TyKind::IntVar(k), TyKind::UInt(t)) | (TyKind::UInt(t), TyKind::IntVar(k)) => {
                self.int_table.unify_var_value(*k, Some(IntValue::UInt(*t)))
            }
            (TyKind::FloatVar(a), TyKind::FloatVar(b)) => self.float_table.unify_var_var(*a, *b),
            (TyKind::FloatVar(k), TyKind::Float(t)) | (TyKind::Float(t), TyKind::FloatVar(k)) => {
                self.float_table.unify_var_value(*k, Some(*t))
            }
            (TyKind::Array(a), TyKind::Array(b)) => self.unify(*a, *b),
            (TyKind::Tuple(a), TyKind::Tuple(b)) => {
                if a.len() != b.len() {
                    return Err(format!("cannot unify {:?} and {:?}", a, b));
                }
                for (a, b) in a.iter().zip(b.iter()) {
                    self.unify(*a, *b)?;
                }
                Ok(())
            }
            (TyKind::Struct(a, a_tail), TyKind::Struct(b, b_tail)) => self.unify_rows(
                RowValue { fields: *a, tail: *a_tail },
                RowValue { fields: *b, tail: *b_tail },
            ),
            (TyKind::Enum(a, a_tail), TyKind::Enum(b, b_tail)) => self.unify_rows(
                RowValue { fields: *a, tail: *a_tail },
                RowValue { fields: *b, tail: *b_tail },
            ),
            (TyKind::Nullable(a), TyKind::Nullable(b)) => self.unify(*a, *b),
            _ => Err(format!("cannot unify {:?} and {:?}", a, b)),
        }
    }

    fn unify_rows(&mut self, a: RowValue<'cx>, b: RowValue<'cx>) -> Result<(), String> {
        let RowValue { fields: a, tail: a_tail } = a;
        let RowValue { fields: b, tail: b_tail } = b;

        match (a_tail, b_tail) {
            (None, None) => self.match_fields(a, b, Self::unify),

            (Some(a_tail), None) => {
                let fields = self.subset_fields(a, b, Self::unify)?;
                let a_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(a_tail) {
                    None => Ok(self.row_table.union_value(a_tail, Some(a_tail_value))),
                    Some(a_tail) => self.unify_rows(a_tail, a_tail_value),
                }
            }

            (None, Some(b_tail)) => {
                let fields = self.superset_fields(a, b, Self::unify)?;
                let b_tail_value = RowValue { fields, tail: None };
                match self.row_table.probe_value(b_tail) {
                    None => Ok(self.row_table.union_value(b_tail, Some(b_tail_value))),
                    Some(b_tail) => self.unify_rows(b_tail, b_tail_value),
                }
            }

            (Some(a_tail), Some(b_tail)) => {
                let common_tail = self.new_row_var();
                let (a_fields, b_fields) = self.intersect_fields(a, b, Self::unify)?;
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

    /// Checks that the fields match exactly
    fn match_fields(
        &mut self,
        a: FieldList<'cx>,
        b: FieldList<'cx>,
        merge: impl Fn(&mut Self, Ty<'cx>, Ty<'cx>) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut a_fields: HashMap<_, _> = a.iter().copied().collect();

        for (name, b_ty) in b {
            match a_fields.remove(&name) {
                Some(a_ty) => merge(self, a_ty, b_ty)?,
                None => Err("mismatched keys")?,
            }
        }

        if !a_fields.is_empty() {
            Err("mismatched keys")?;
        }

        Ok(())
    }

    /// Checks that the fields in `sub` are a subset of those in `sup`,
    /// returning the fields in `sup` not present in `sub` as a new `FieldList`
    fn subset_fields(
        &mut self,
        sub: FieldList<'cx>,
        sup: FieldList<'cx>,
        merge: impl Fn(&mut Self, Ty<'cx>, Ty<'cx>) -> Result<(), String>,
    ) -> Result<FieldList<'cx>, String> {
        let mut sup_fields: HashMap<_, _> = sup.iter().copied().collect();

        for (name, sub_ty) in sub {
            match sup_fields.remove(&name) {
                Some(sup_ty) => merge(self, sub_ty, sup_ty)?,
                None => Err("mismatched keys")?,
            }
        }

        let mut fields = sup_fields.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        fields.sort_by_cached_key(|(name, _)| self.ctx.get_str(*name));

        Ok(FieldList(self.ctx.intern_fields(&fields)))
    }

    /// Checks that the fields in `sup` are a superset of those in `sub`,
    /// returning the fields in `sup` not present in `sub` as a new `FieldList`
    fn superset_fields(
        &mut self,
        sup: FieldList<'cx>,
        sub: FieldList<'cx>,
        merge: impl Fn(&mut Self, Ty<'cx>, Ty<'cx>) -> Result<(), String>,
    ) -> Result<FieldList<'cx>, String> {
        self.subset_fields(sub, sup, |s, a, b| merge(s, b, a))
    }

    /// Finds the intersection of the fields in `a` and `b`, returning a tuple containing
    /// the unique fields in `a` followed by the unique fields in `b`, as new `FieldList`s
    fn intersect_fields(
        &mut self,
        a: FieldList<'cx>,
        b: FieldList<'cx>,
        merge: impl Fn(&mut Self, Ty<'cx>, Ty<'cx>) -> Result<(), String>,
    ) -> Result<(FieldList<'cx>, FieldList<'cx>), String> {
        let mut a_fields: HashMap<_, _> = a.iter().copied().collect();
        let mut b_fields = HashMap::new();

        for (name, b_ty) in b {
            match a_fields.remove(&name) {
                Some(a_ty) => merge(self, a_ty, b_ty)?,
                None => {
                    b_fields.insert(name, b_ty);
                }
            }
        }

        let mut a_fields = a_fields.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        let mut b_fields = b_fields.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>();
        a_fields.sort_by_cached_key(|(name, _)| self.ctx.get_str(*name));
        b_fields.sort_by_cached_key(|(name, _)| self.ctx.get_str(*name));

        Ok((
            FieldList(self.ctx.intern_fields(&a_fields)),
            FieldList(self.ctx.intern_fields(&b_fields)),
        ))
    }

    fn union_value(&mut self, var: TyVar<'cx>, ty: Ty<'cx>) -> Result<(), String> {
        if self.occurs(var, ty) {
            return Err(format!("infinite type"));
        }
        Ok(self.ty_table.union_value(var, Some(ty)))
    }

    pub fn resolve_partial(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, String> {
        struct Resolver<'a, 'cx>(&'a mut TypecheckCtx<'cx>);

        impl<'a, 'cx> TyFolder<'cx> for Resolver<'a, 'cx> {
            type Error = String;

            fn ctx(&self) -> Ctx<'cx> {
                self.0.ctx
            }

            fn fold_ty(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, Self::Error> {
                match ty.kind() {
                    &TyKind::Struct(fields, Some(tail)) => {
                        let fields = self.0.resolve_row(RowValue { fields, tail: Some(tail) })?;
                        let ty = Ty(self.0.ctx.intern_ty_kind(TyKind::Struct(fields, None)));
                        self.super_fold_ty(ty)
                    }
                    &TyKind::Enum(fields, Some(tail)) => {
                        let fields = self.0.resolve_row(RowValue { fields, tail: Some(tail) })?;
                        let ty = Ty(self.0.ctx.intern_ty_kind(TyKind::Enum(fields, None)));
                        self.super_fold_ty(ty)
                    }
                    TyKind::TyVar(var) => match self.0.ty_table.probe_value(*var) {
                        Some(ty) => self.fold_ty(ty),
                        None => Ok(ty),
                    },
                    TyKind::IntVar(var) => match self.0.int_table.probe_value(*var) {
                        Some(IntValue::Int(t)) => Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::Int(t)))),
                        Some(IntValue::UInt(t)) => {
                            Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::UInt(t))))
                        }
                        None => Ok(ty),
                    },
                    TyKind::FloatVar(var) => match self.0.float_table.probe_value(*var) {
                        Some(t) => Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::Float(t)))),
                        None => Ok(ty),
                    },
                    _ => self.super_fold_ty(ty),
                }
            }
        }

        Resolver(self).fold_ty(ty)
    }

    pub fn resolve(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, String> {
        struct Resolver<'a, 'cx>(&'a mut TypecheckCtx<'cx>);

        impl<'a, 'cx> TyFolder<'cx> for Resolver<'a, 'cx> {
            type Error = String;

            fn ctx(&self) -> Ctx<'cx> {
                self.0.ctx
            }

            fn fold_ty(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, Self::Error> {
                match ty.kind() {
                    &TyKind::Struct(fields, Some(tail)) => {
                        let fields = self.0.resolve_row(RowValue { fields, tail: Some(tail) })?;
                        let ty = Ty(self.0.ctx.intern_ty_kind(TyKind::Struct(fields, None)));
                        self.super_fold_ty(ty)
                    }
                    &TyKind::Enum(fields, Some(tail)) => {
                        let fields = self.0.resolve_row(RowValue { fields, tail: Some(tail) })?;
                        let ty = Ty(self.0.ctx.intern_ty_kind(TyKind::Enum(fields, None)));
                        self.super_fold_ty(ty)
                    }
                    TyKind::TyVar(var) => match self.0.ty_table.probe_value(*var) {
                        Some(ty) => self.fold_ty(ty),
                        _ => Err(format!("unresolved type")),
                    },
                    TyKind::IntVar(var) => match self.0.int_table.probe_value(*var) {
                        Some(IntValue::Int(t)) => Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::Int(t)))),
                        Some(IntValue::UInt(t)) => {
                            Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::UInt(t))))
                        }
                        None => Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)))),
                    },
                    TyKind::FloatVar(var) => match self.0.float_table.probe_value(*var) {
                        Some(t) => Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::Float(t)))),
                        None => Ok(Ty(self.0.ctx.intern_ty_kind(TyKind::Float(FloatTy::Float64)))),
                    },
                    _ => self.super_fold_ty(ty),
                }
            }
        }

        Resolver(self).fold_ty(ty)
    }

    pub fn resolve_row(&mut self, value: RowValue<'cx>) -> Result<FieldList<'cx>, String> {
        let mut fields_vec = vec![value.fields];
        let mut curr_tail = value.tail;

        while let Some(var) = curr_tail {
            match self.row_table.probe_value(var) {
                Some(RowValue { fields, tail }) => {
                    fields_vec.push(fields);
                    curr_tail = tail;
                }
                None => Err(format!("unbound row var"))?,
            }
        }

        match &fields_vec[..] {
            [fields] => Ok(*fields),
            list => {
                let fields = list.iter().flatten().collect::<Vec<_>>();
                let fields = self.ctx.intern_fields(&fields);
                Ok(FieldList(fields))
            }
        }
    }

    pub fn probe_row(&mut self, var: RowVar<'cx>) -> Result<RowValue<'cx>, String> {
        match self.row_table.probe_value(var) {
            Some(value) => Ok(value),
            None => Err("unbound row var")?,
        }
    }

    fn occurs(&mut self, var: TyVar<'cx>, ty: Ty<'cx>) -> bool {
        match ty.kind() {
            TyKind::Bool => false,
            TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_) => false,
            TyKind::Str => false,
            TyKind::Array(inner) => self.occurs(var, *inner),
            TyKind::Tuple(tys) => tys.iter().any(|t| self.occurs(var, *t)),
            TyKind::Struct(fields, tail) => {
                let in_fields = fields.iter().any(|(_, t)| self.occurs(var, *t));
                let in_tail = tail.map_or(false, |next| self.occurs_in_row(var, next));
                in_fields || in_tail
            }
            TyKind::Enum(fields, tail) => {
                let in_fields = fields.iter().any(|(_, t)| self.occurs(var, *t));
                let in_tail = tail.map_or(false, |next| self.occurs_in_row(var, next));
                in_fields || in_tail
            }
            TyKind::Nullable(inner) => self.occurs(var, *inner),
            TyKind::TyVar(other) => match self.ty_table.probe_value(*other) {
                Some(bound) => self.occurs(var, bound),
                None => self.ty_table.find(*other) == self.ty_table.find(var),
            },
            TyKind::IntVar(_) | TyKind::FloatVar(_) => false,
        }
    }

    fn occurs_in_row(&mut self, var: TyVar<'cx>, row_var: RowVar<'cx>) -> bool {
        match self.row_table.probe_value(row_var) {
            None => false, // unbound row var, can't contain anything yet
            Some(RowValue { fields, tail }) => {
                let in_fields = fields.iter().any(|(_, t)| self.occurs(var, *t));
                let in_tail = tail.map_or(false, |next| self.occurs_in_row(var, next));
                in_fields || in_tail
            }
        }
    }
}

impl<'cx> UnifyKey for TyVar<'cx> {
    type Value = Option<Ty<'cx>>;

    fn index(&self) -> u32 {
        (*self).into()
    }

    fn from_index(u: u32) -> Self {
        Self::new(u)
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntValue {
    Int(IntTy),
    UInt(UIntTy),
}

impl<'cx> UnifyKey for IntVar<'cx> {
    type Value = Option<IntValue>;

    fn index(&self) -> u32 {
        (*self).into()
    }

    fn from_index(u: u32) -> Self {
        Self::new(u)
    }

    fn tag() -> &'static str {
        "IntVar"
    }
}

impl UnifyValue for IntValue {
    type Error = String;

    fn unify_values(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        (a == b).then_some(*a).ok_or_else(|| "mismatched int types".into())
    }
}

impl<'cx> UnifyKey for FloatVar<'cx> {
    type Value = Option<FloatTy>;

    fn index(&self) -> u32 {
        (*self).into()
    }

    fn from_index(u: u32) -> Self {
        Self::new(u)
    }

    fn tag() -> &'static str {
        "FloatVar"
    }
}

impl UnifyValue for FloatTy {
    type Error = String;

    fn unify_values(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        (a == b).then_some(*a).ok_or_else(|| "mismatched float types".into())
    }
}

impl<'cx> UnifyKey for RowVar<'cx> {
    type Value = Option<RowValue<'cx>>;

    fn index(&self) -> u32 {
        (*self).into()
    }

    fn from_index(u: u32) -> Self {
        Self::new(u)
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
    use crate::types::{FieldList, IntTy, Ty, TyKind, TyList};
    use crate::{arena, diagnostics};
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

    #[test]
    fn unify_int_ty() {
        setup!(arena, ctx, tc);
        let var = tc.new_int_var();
        let int16 = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int16)));
        tc.unify(var, int16).unwrap();
        assert_eq!(tc.resolve(var).unwrap(), int16);
    }

    #[test]
    fn unify_float_ty_with_int() {
        setup!(arena, ctx, tc);
        let var = tc.new_float_var();
        let int16 = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int16)));
        assert!(tc.unify(var, int16).is_err());
    }

    #[test]
    fn unify_int_var_with_ty_var() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let var2 = tc.new_int_var();
        let int16 = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int16)));
        tc.unify(var1, var2).unwrap();
        tc.unify(var2, int16).unwrap();
        assert_eq!(tc.resolve(var1).unwrap(), int16);
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

    macro_rules! mk_enum {
        ($ctx:expr, $tc:expr, [$(($name:expr, $ty:expr)),+], $tail:expr) => {{
            let fields = [$(($ctx.intern_str($name), $ty)),+];
            let r = $ctx.intern_fields(&fields);
            Ty($ctx.intern_ty_kind(TyKind::Enum(FieldList(r), $tail)))
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
        let resolved_row = tc.probe_row(row).unwrap();
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
        let resolved_a = tc.probe_row(row_a).unwrap();
        assert_eq!(resolved_a.fields, FieldList(ctx.intern_fields(&[(age, int)])));
        assert!(resolved_a.tail.is_some());
        // row_b should be bound to { name: Str | common_tail }
        let resolved_b = tc.probe_row(row_b).unwrap();
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
        let resolved_row = tc.probe_row(row).unwrap();
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

    // Fields are stored in canonical (lexicographic) order regardless of insertion order
    #[test]
    fn canonical_field_ordering_in_row_tail() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row = tc.new_row_var();
        // open struct with "name", closed struct has "age" and "name" - "age" comes first lexically
        let open = mk_struct!(ctx, tc, [("name", str)], Some(row));
        let closed = mk_struct!(ctx, tc, [("zebra", int), ("age", int), ("name", str)], None);
        tc.unify(open, closed).unwrap();
        let resolved = tc.probe_row(row).unwrap();
        let fields = resolved.fields;
        // fields in the tail should be sorted lexicographically
        assert_eq!(fields[0].0, ctx.intern_str("age"));
        assert_eq!(fields[1].0, ctx.intern_str("zebra"));
    }

    // Two calls producing the same row tail should produce identical FieldLists via pointer equality
    #[test]
    fn canonical_ordering_produces_identical_interned_types() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));

        // First unification
        let row_a = tc.new_row_var();
        let open_a = mk_struct!(ctx, tc, [("name", str)], Some(row_a));
        let closed_a = mk_struct!(ctx, tc, [("age", int), ("name", str)], None);
        tc.unify(open_a, closed_a).unwrap();

        // Second unification with same types but different insertion order
        let row_b = tc.new_row_var();
        let open_b = mk_struct!(ctx, tc, [("name", str)], Some(row_b));
        let closed_b = mk_struct!(ctx, tc, [("name", str), ("age", int)], None);
        tc.unify(open_b, closed_b).unwrap();

        // Both row tails should have resolved to identical interned FieldLists
        let resolved_a = tc.probe_row(row_a).unwrap();
        let resolved_b = tc.probe_row(row_b).unwrap();
        assert_eq!(resolved_a.fields, resolved_b.fields);
        // pointer equality — same interned allocation
        assert!(std::ptr::eq(resolved_a.fields.as_ptr(), resolved_b.fields.as_ptr()));
    }

    // intersect_fields produces canonically ordered results
    #[test]
    fn canonical_ordering_in_two_open_struct_tails() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let bool = Ty(ctx.intern_ty_kind(TyKind::Bool));
        let row_a = tc.new_row_var();
        let row_b = tc.new_row_var();
        // a has "name" and "zebra", b has "age" and "zebra"
        // "zebra" is common, "name" ends up in b's tail, "age" ends up in a's tail
        let open_a = mk_struct!(ctx, tc, [("zebra", bool), ("name", str)], Some(row_a));
        let open_b = mk_struct!(ctx, tc, [("age", int), ("zebra", bool)], Some(row_b));
        tc.unify(open_a, open_b).unwrap();
        // a's tail should contain "age" (b's unique field)
        let resolved_a = tc.probe_row(row_a).unwrap();
        let a_fields = resolved_a.fields;
        assert_eq!(a_fields.len(), 1);
        assert_eq!(a_fields[0].0, ctx.intern_str("age"));
        // b's tail should contain "name" (a's unique field)
        let resolved_b = tc.probe_row(row_b).unwrap();
        let b_fields = resolved_b.fields;
        assert_eq!(b_fields.len(), 1);
        assert_eq!(b_fields[0].0, ctx.intern_str("name"));
    }

    // Determinism — running the same unification twice produces identical results
    #[test]
    fn unification_is_deterministic() {
        let arena1 = Bump::new();
        let mut handler1 = diagnostics::Diagnostics::new();
        let mut ctx1 = arena::Ctx::new(&arena1, &mut handler1);
        let mut tc1 = super::TypecheckCtx::new(ctx1);

        let arena2 = Bump::new();
        let mut handler2 = diagnostics::Diagnostics::new();
        let mut ctx2 = arena::Ctx::new(&arena2, &mut handler2);
        let mut tc2 = super::TypecheckCtx::new(ctx2);

        // Same unification in both contexts
        let str1 = Ty(ctx1.intern_ty_kind(TyKind::Str));
        let int1 = Ty(ctx1.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row1 = tc1.new_row_var();
        let open1 = mk_struct!(ctx1, tc1, [("name", str1)], Some(row1));
        let closed1 = mk_struct!(ctx1, tc1, [("zebra", int1), ("age", int1), ("name", str1)], None);
        tc1.unify(open1, closed1).unwrap();

        let str2 = Ty(ctx2.intern_ty_kind(TyKind::Str));
        let int2 = Ty(ctx2.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let row2 = tc2.new_row_var();
        let open2 = mk_struct!(ctx2, tc2, [("name", str2)], Some(row2));
        let closed2 = mk_struct!(ctx2, tc2, [("zebra", int2), ("age", int2), ("name", str2)], None);
        tc2.unify(open2, closed2).unwrap();

        // Both should produce the same field ordering in the tail
        let resolved1 = tc1.probe_row(row1).unwrap();
        let resolved2 = tc2.probe_row(row2).unwrap();
        let fields1: Vec<_> = resolved1.fields.iter().map(|(k, _)| *k).collect();
        let fields2: Vec<_> = resolved2.fields.iter().map(|(k, _)| *k).collect();
        assert_eq!(fields1, fields2);
    }

    // Unifying a type variable with a type that contains itself should fail
    #[test]
    fn occurs_check_direct() {
        setup!(arena, ctx, tc);
        let var = tc.new_ty_var();
        let array_var = Ty(ctx.intern_ty_kind(TyKind::Array(var)));
        assert!(tc.unify(var, array_var).is_err());
    }

    // Unifying a type variable with a deeply nested type that contains itself should fail
    #[test]
    fn occurs_check_nested() {
        setup!(arena, ctx, tc);
        let var = tc.new_ty_var();
        let array_var = Ty(ctx.intern_ty_kind(TyKind::Array(var)));
        let array_array_var = Ty(ctx.intern_ty_kind(TyKind::Array(array_var)));
        assert!(tc.unify(var, array_array_var).is_err());
    }

    // Unifying two variables where one is already bound to a type containing the other should fail
    #[test]
    fn occurs_check_indirect() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let var2 = tc.new_ty_var();
        let array_var1 = Ty(ctx.intern_ty_kind(TyKind::Array(var1)));
        // bind var2 to Array(var1)
        tc.unify(var2, array_var1).unwrap();
        // now trying to bind var1 to something containing var2 should fail
        // since var2 = Array(var1), this would create var1 = Array(Array(var1))
        let array_var2 = Ty(ctx.intern_ty_kind(TyKind::Array(var2)));
        assert!(tc.unify(var1, array_var2).is_err());
    }

    // Unifying a type variable with a struct containing itself should fail
    #[test]
    fn occurs_check_in_struct() {
        setup!(arena, ctx, tc);
        let var = tc.new_ty_var();
        let name = ctx.intern_str("self");
        let fields = ctx.intern_fields(&[(name, var)]);
        let struct_ty = Ty(ctx.intern_ty_kind(TyKind::Struct(FieldList(fields), None)));
        assert!(tc.unify(var, struct_ty).is_err());
    }

    // Unifying a type variable with a struct whose row tail contains itself should fail
    #[test]
    fn occurs_check_in_row_tail() {
        setup!(arena, ctx, tc);
        let var = tc.new_ty_var();
        let row = tc.new_row_var();
        // open struct where the row tail will be bound to something containing var
        let open = mk_struct!(ctx, tc, [("name", var)], Some(row));
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        // this is fine — binds var to Str, row to { age: Int }
        let closed = mk_struct!(ctx, tc, [("name", str), ("age", int)], None);
        tc.unify(open, closed).unwrap();
        // now try to create a circular type through var
        let array_var = Ty(ctx.intern_ty_kind(TyKind::Array(var)));
        assert!(tc.unify(var, array_var).is_err());
    }

    // Unifying a type variable with a tuple containing itself should fail
    #[test]
    fn occurs_check_in_tuple() {
        setup!(arena, ctx, tc);
        let var = tc.new_ty_var();
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let tys = ctx.intern_tys(&[str, var]);
        let tuple = Ty(ctx.intern_ty_kind(TyKind::Tuple(TyList(tys))));
        assert!(tc.unify(var, tuple).is_err());
    }

    // Occurs check through a chain of variables
    #[test]
    fn occurs_check_through_var_chain() {
        setup!(arena, ctx, tc);
        let var1 = tc.new_ty_var();
        let var2 = tc.new_ty_var();
        let var3 = tc.new_ty_var();
        // chain: var1 -> var2 -> var3
        tc.unify(var1, var2).unwrap();
        tc.unify(var2, var3).unwrap();
        // now try to bind var3 to something containing var1
        // this would create a cycle: var1 = var2 = var3 = Array(var1)
        let array_var1 = Ty(ctx.intern_ty_kind(TyKind::Array(var1)));
        assert!(tc.unify(var3, array_var1).is_err());
    }

    // Sanity check — a non-circular type still unifies correctly after occurs check is added
    #[test]
    fn occurs_check_does_not_break_valid_unification() {
        setup!(arena, ctx, tc);
        let var = tc.new_ty_var();
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let array_str = Ty(ctx.intern_ty_kind(TyKind::Array(str)));
        assert!(tc.unify(var, array_str).is_ok());
        assert_eq!(tc.resolve(var).unwrap(), array_str);
    }

    #[test]
    fn coerce_struct() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let ty1 = mk_struct!(ctx, tc, [("name", str), ("age", int)], None);
        let ty2 = mk_struct!(ctx, tc, [("age", int)], None);
        assert!(tc.coerce(ty1, ty2).is_ok());
        assert!(tc.coerce(ty2, ty1).is_err());
    }

    #[test]
    fn coerce_enum() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let ty1 = mk_enum!(ctx, tc, [("name", str), ("age", int)], None);
        let ty2 = mk_enum!(ctx, tc, [("age", int)], None);
        assert!(tc.coerce(ty2, ty1).is_ok());
        assert!(tc.coerce(ty1, ty2).is_err());
    }

    #[test]
    fn coerce_nullable() {
        setup!(arena, ctx, tc);
        let str = Ty(ctx.intern_ty_kind(TyKind::Str));
        let int = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let nullable_int = Ty(ctx.intern_ty_kind(TyKind::Nullable(int)));

        assert!(tc.coerce(int, nullable_int).is_ok());
        assert!(tc.coerce(nullable_int, int).is_err());

        let ty1 = mk_struct!(ctx, tc, [("name", str), ("age", int)], None);
        let ty2 = mk_struct!(ctx, tc, [("age", nullable_int)], None);
        assert!(tc.coerce(ty1, ty2).is_ok());
        assert!(tc.coerce(ty2, ty1).is_err());

        let ty1 = mk_struct!(ctx, tc, [("name", str), ("age", nullable_int)], None);
        let ty2 = mk_struct!(ctx, tc, [("age", int)], None);
        assert!(tc.coerce(ty1, ty2).is_err());
        assert!(tc.coerce(ty2, ty1).is_err());
    }

    #[test]
    fn coerce_generic_return_into_nullable_array() {
        setup!(arena, ctx, tc);

        // Simulate the return type of empty<T>() -> T[]
        // T is a fresh type variable, return type is T[]
        let t = tc.new_ty_var();
        let return_ty = Ty(ctx.intern_ty_kind(TyKind::Array(t)));

        // Expected type from annotation: (i32[])?
        let i32_ty = Ty(ctx.intern_ty_kind(TyKind::Int(IntTy::Int32)));
        let i32_array = Ty(ctx.intern_ty_kind(TyKind::Array(i32_ty)));
        let expected = Ty(ctx.intern_ty_kind(TyKind::Nullable(i32_array)));

        // Coerce return type against expected type
        assert!(tc.coerce(return_ty, expected).is_ok());

        // T should have been resolved to i32
        assert_eq!(tc.resolve(t).unwrap(), i32_ty);

        // The return type should resolve to i32[]
        assert_eq!(tc.resolve(return_ty).unwrap(), i32_array);
    }
}
