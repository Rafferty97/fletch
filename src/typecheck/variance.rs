use super::{Ty, TyKind, VarId};

pub fn variance_in(ty: Ty, var: VarId) -> Variance {
    match ty.kind() {
        TyKind::Array(ty) => variance_in(ty, var),
        TyKind::Nullable(ty) => variance_in(ty, var),
        TyKind::Tuple(tys) => tys.iter().map(|ty| variance_in(*ty, var)).collect(),
        TyKind::Var(v) if v == var => Variance::Covariant,
        _ => Variance::Bivariant,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variance {
    Invariant,
    Covariant,
    Contravariant,
    Bivariant,
}

impl Variance {
    pub fn new(covariant: bool, contravariant: bool) -> Self {
        match (covariant, contravariant) {
            (false, false) => Self::Invariant,
            (true, false) => Self::Covariant,
            (false, true) => Self::Contravariant,
            (true, true) => Self::Bivariant,
        }
    }

    pub fn merge(a: Self, b: Self) -> Self {
        let covariant = a.is_covariant() && b.is_covariant();
        let contravariant = a.is_contravariant() && b.is_contravariant();
        Self::new(covariant, contravariant)
    }

    pub fn is_covariant(self) -> bool {
        matches!(self, Self::Covariant | Self::Bivariant)
    }

    pub fn is_contravariant(self) -> bool {
        matches!(self, Self::Contravariant | Self::Bivariant)
    }
}

impl FromIterator<Variance> for Variance {
    fn from_iter<T: IntoIterator<Item = Variance>>(iter: T) -> Self {
        iter.into_iter().fold(Self::Bivariant, Self::merge)
    }
}
