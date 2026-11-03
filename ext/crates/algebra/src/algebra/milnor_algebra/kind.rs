//! The dual Steenrod algebras that a [`MilnorAlgebra`] can be a sub-Hopf-algebra of.

use fp::prime::{TWO, ValidPrime};
use serde::{Deserialize, Serialize};

use super::{Exterior, MilnorAlgebra, MilnorAlgebraInner, MilnorProfile, NoExterior};
use crate::algebra::Algebra;

/// Which Hopf algebra a Steenrod algebra is a sub-Hopf-algebra of.
///
/// This is a property of the algebra rather than of its basis: the Adem basis only exists for the
/// classical kind, and [`MilnorAlgebra::kind`] reports the kind of a Milnor one.
///
/// [`MilnorAlgebra::new`] builds the classical kind, and [`MilnorAlgebra::motivic_mod_tau`] and
/// [`MilnorAlgebra::reduced_powers`] the others. Every pair of a prime and a
/// [shape](super::MilnorShape) is exactly one kind, so the kind names what the shape encodes.
///
/// A module spec names it in its `kind` field, in snake case, e.g. `"motivic_mod_tau"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SteenrodKind {
    /// The Steenrod algebra $A$, at any prime.
    #[default]
    Classical,
    /// The mod-$\tau$ $\mathbb{C}$-motivic Steenrod algebra $A^{\mathbb{C}}/\tau$, at `p = 2`.
    MotivicModTau,
    /// The algebra of reduced powers $P = A/\!/E(Q_0, Q_1, \dots)$, at odd primes, whose dual is
    /// $\mathbb{F}_p[\xi_1, \xi_2, \dots]$.
    ReducedPowers,
}

impl SteenrodKind {
    /// The kind with the given prime and shape.
    pub(super) fn from_shape(p: ValidPrime, has_exterior: bool) -> Self {
        match (p == 2, has_exterior) {
            (true, false) | (false, true) => Self::Classical,
            (true, true) => Self::MotivicModTau,
            (false, false) => Self::ReducedPowers,
        }
    }
}

impl std::fmt::Display for SteenrodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Classical => "A",
            Self::MotivicModTau => "A_C/tau",
            Self::ReducedPowers => "P",
        })
    }
}

impl MilnorAlgebra {
    /// $A^{\mathbb{C}}/\tau$ restricted to `profile`. The default profile gives all of it.
    pub fn motivic_mod_tau(profile: MilnorProfile, unstable_enabled: bool) -> Self {
        MilnorAlgebraInner::<Exterior>::new_with_profile(TWO, profile, unstable_enabled).into()
    }

    /// The algebra of reduced powers at `p` restricted to `profile`. The default profile gives all
    /// of it.
    ///
    /// # Panics
    ///
    /// If `p = 2`, where the polynomial shape is the classical algebra.
    pub fn reduced_powers(p: ValidPrime, profile: MilnorProfile, unstable_enabled: bool) -> Self {
        assert!(p != 2, "the reduced powers are only defined at odd primes");
        MilnorAlgebraInner::<NoExterior>::new_with_profile(p, profile, unstable_enabled).into()
    }

    /// The Hopf algebra this is a sub-Hopf-algebra of.
    pub fn kind(&self) -> SteenrodKind {
        SteenrodKind::from_shape(self.prime(), self.has_exterior())
    }
}
