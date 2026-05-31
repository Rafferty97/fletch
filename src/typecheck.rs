use crate::interner::Interned;

pub use ty_ctx::TyCtx;

mod ty_ctx;
mod ty_interners;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Ty<'ty>(Interned<'ty, TyKind<'ty>>);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TyKind<'ty> {
    Never,
    Bool,
    Str,
    Nullable(Ty<'ty>),
    Array(Ty<'ty>),
    Tuple(Interned<'ty, [Ty<'ty>]>),
    Top,
    Err,
}

impl<'ty> Ty<'ty> {
    pub fn kind(self) -> TyKind<'ty> {
        *self.0
    }
}

impl<'a: 'ty, 'ty> TyCtx<'a, 'ty> {
    pub fn meet(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        match (lhs.kind(), rhs.kind()) {
            _ if lhs == rhs => lhs,

            (TyKind::Err, _) | (_, TyKind::Err) => self.mk_err(),

            (TyKind::Top, _) => rhs,
            (_, TyKind::Top) => lhs,

            (TyKind::Nullable(lhs), TyKind::Nullable(rhs)) => self.mk_nullable(self.meet(lhs, rhs)),
            (TyKind::Nullable(lhs), _) => self.meet(lhs, rhs),
            (_, TyKind::Nullable(rhs)) => self.meet(lhs, rhs),

            (TyKind::Array(lhs), TyKind::Array(rhs)) => self.mk_array(self.meet(lhs, rhs)),

            (TyKind::Tuple(lhs), TyKind::Tuple(rhs)) if lhs.len() == rhs.len() => {
                let tys: Vec<_> = lhs
                    .iter()
                    .zip(rhs.iter())
                    .map(|(&lhs, &rhs)| self.meet(lhs, rhs))
                    .collect();
                self.mk_tuple(&tys)
            }

            _ => self.mk_never(),
        }
    }

    pub fn join(self, lhs: Ty<'ty>, rhs: Ty<'ty>) -> Ty<'ty> {
        match (lhs.kind(), rhs.kind()) {
            _ if lhs == rhs => lhs,

            (TyKind::Err, _) | (_, TyKind::Err) => self.mk_err(),

            (TyKind::Never, _) => rhs,
            (_, TyKind::Never) => lhs,

            (TyKind::Nullable(lhs), TyKind::Nullable(rhs)) => self.mk_nullable(self.join(lhs, rhs)),
            (TyKind::Nullable(lhs), _) => self.mk_nullable(self.join(lhs, rhs)),
            (_, TyKind::Nullable(rhs)) => self.mk_nullable(self.join(lhs, rhs)),

            (TyKind::Array(lhs), TyKind::Array(rhs)) => self.mk_array(self.join(lhs, rhs)),

            (TyKind::Tuple(lhs), TyKind::Tuple(rhs)) if lhs.len() == rhs.len() => {
                let tys: Vec<_> = lhs
                    .iter()
                    .zip(rhs.iter())
                    .map(|(&lhs, &rhs)| self.join(lhs, rhs))
                    .collect();
                self.mk_tuple(&tys)
            }

            _ => self.mk_top(),
        }
    }

    pub fn is_subtype(self, sub: Ty<'ty>, sup: Ty<'ty>) -> bool {
        match (sub.kind(), sup.kind()) {
            _ if sub == sup => true,

            (TyKind::Err, _) | (_, TyKind::Err) => true,
            (TyKind::Never, _) | (_, TyKind::Top) => true,

            (TyKind::Nullable(sub), TyKind::Nullable(sup)) => self.is_subtype(sub, sup),
            (TyKind::Nullable(_), _) => false,
            (_, TyKind::Nullable(sup)) => self.is_subtype(sub, sup),

            (TyKind::Array(sub), TyKind::Array(sup)) => self.is_subtype(sub, sup),

            (TyKind::Tuple(sub), TyKind::Tuple(sup)) if sub.len() == sup.len() => sub
                .iter()
                .zip(sup.iter())
                .all(|(&sub, &sup)| self.is_subtype(sub, sup)),

            _ => false,
        }
    }
}
