use std::ops::Deref;

use super::{FieldList, Ty, TyKind, TyList};
use crate::arena::Ctx;

pub trait TyFolder<'cx> {
    type Error;

    fn ctx(&self) -> Ctx<'cx>;

    fn fold_ty(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, Self::Error> {
        self.super_fold_ty(ty)
    }

    fn super_fold_ty(&mut self, ty: Ty<'cx>) -> Result<Ty<'cx>, Self::Error> {
        Ok(match ty.kind() {
            TyKind::Bool => ty,
            TyKind::Int(_) | TyKind::UInt(_) | TyKind::Float(_) => ty,
            TyKind::Str => ty,
            TyKind::Array(inner) => {
                let folded = self.fold_ty(*inner)?;
                match folded == *inner {
                    true => ty,
                    false => Ty(self.ctx().intern_ty_kind(TyKind::Array(folded))),
                }
            }
            TyKind::Tuple(inner) => {
                let folded: Vec<_> =
                    inner.iter().map(|ty| self.fold_ty(*ty)).collect::<Result<_, _>>()?;
                match folded == inner.deref() {
                    true => ty,
                    false => {
                        let folded = self.ctx().intern_tys(folded.deref());
                        Ty(self.ctx().intern_ty_kind(TyKind::Tuple(TyList(folded))))
                    }
                }
            }
            TyKind::Struct(fields, tail) => {
                let folded = self.fold_fields(*fields)?;
                match folded == *fields {
                    true => ty,
                    false => Ty(self.ctx().intern_ty_kind(TyKind::Struct(folded, *tail))),
                }
            }
            TyKind::Enum(fields, tail) => {
                let folded = self.fold_fields(*fields)?;
                match folded == *fields {
                    true => ty,
                    false => Ty(self.ctx().intern_ty_kind(TyKind::Enum(folded, *tail))),
                }
            }
            TyKind::Nullable(inner) => {
                let folded = self.fold_ty(*inner)?;
                match folded == *inner {
                    true => ty,
                    false => Ty(self.ctx().intern_ty_kind(TyKind::Nullable(folded))),
                }
            }
            TyKind::TyVar(_) | TyKind::NumVar(_) => ty,
        })
    }

    fn fold_tys(&mut self, tys: TyList<'cx>) -> Result<TyList<'cx>, Self::Error> {
        self.supe_fold_tys(tys)
    }

    fn supe_fold_tys(&mut self, tys: TyList<'cx>) -> Result<TyList<'cx>, Self::Error> {
        let folded: Vec<_> = tys.iter().map(|ty| self.fold_ty(*ty)).collect::<Result<_, _>>()?;
        Ok(match folded == tys.deref() {
            true => tys,
            false => TyList(self.ctx().intern_tys(folded.deref())),
        })
    }

    fn fold_fields(&mut self, fields: FieldList<'cx>) -> Result<FieldList<'cx>, Self::Error> {
        self.super_fold_fields(fields)
    }

    fn super_fold_fields(&mut self, fields: FieldList<'cx>) -> Result<FieldList<'cx>, Self::Error> {
        let folded: Vec<_> = fields
            .iter()
            .map(|(name, ty)| Ok((*name, self.fold_ty(*ty)?)))
            .collect::<Result<_, _>>()?;
        Ok(match folded == fields.deref() {
            true => fields,
            false => FieldList(self.ctx().intern_fields(folded.deref())),
        })
    }
}
