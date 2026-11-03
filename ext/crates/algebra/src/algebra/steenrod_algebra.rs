use std::io;

use anyhow::anyhow;
use fp::{
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut},
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    algebra::{
        AdemAlgebra, Algebra, Bialgebra, GeneratedAlgebra, MilnorAlgebra, UnstableAlgebra,
        milnor_algebra::{MilnorProfile, SteenrodKind},
    },
    pair_algebra::PairAlgebra,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlgebraType {
    Adem,
    Milnor,
}

impl std::fmt::Display for AlgebraType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Adem => "adem",
                Self::Milnor => "milnor",
            }
        )
    }
}

impl std::convert::TryFrom<&str> for AlgebraType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "adem" => Ok(Self::Adem),
            "milnor" => Ok(Self::Milnor),
            _ => Err(anyhow!("Invalid algebra name: {}", s)),
        }
    }
}

impl std::str::FromStr for AlgebraType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

#[allow(clippy::large_enum_variant)]
#[enum_dispatch::enum_dispatch(Algebra, Bialgebra, GeneratedAlgebra, UnstableAlgebra)]
pub enum SteenrodAlgebra {
    AdemAlgebra(AdemAlgebra),
    MilnorAlgebra(MilnorAlgebra),
}

impl std::fmt::Display for SteenrodAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::AdemAlgebra(a) => a.fmt(f),
            Self::MilnorAlgebra(a) => a.fmt(f),
        }
    }
}

impl<'a> TryInto<&'a AdemAlgebra> for &'a SteenrodAlgebra {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<&'a AdemAlgebra, Self::Error> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => Ok(a),
            SteenrodAlgebra::MilnorAlgebra(_) => {
                Err(anyhow!("Expected AdemAlgebra, found MilnorAlgebra"))
            }
        }
    }
}

impl<'a> TryInto<&'a MilnorAlgebra> for &'a SteenrodAlgebra {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<&'a MilnorAlgebra, Self::Error> {
        match self {
            SteenrodAlgebra::MilnorAlgebra(a) => Ok(a),
            SteenrodAlgebra::AdemAlgebra(_) => {
                Err(anyhow!("Expected MilnorAlgebra, found AdemAlgebra"))
            }
        }
    }
}

/// The algebra a module spec is over: its `p`, and the `algebra` object holding the rest.
#[derive(Debug)]
pub struct AlgebraSpec {
    pub p: ValidPrime,
    /// The basis the module's actions or relations are written in, if they need one.
    pub notation: Option<AlgebraType>,
    pub kind: SteenrodKind,
    pub profile: Option<MilnorProfile>,
}

/// The `algebra` object of a module spec.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct AlgebraFields {
    notation: Option<AlgebraType>,
    #[serde(default)]
    kind: SteenrodKind,
    profile: Option<MilnorProfile>,
}

impl AlgebraSpec {
    /// Read the algebra off a module spec.
    pub fn from_json(json: &Value) -> anyhow::Result<Self> {
        for key in ["profile", "kind"] {
            if !json[key].is_null() {
                return Err(anyhow!("`{key}` belongs in the `algebra` object"));
            }
        }
        let p = ValidPrime::deserialize(&json["p"])?;
        let fields = match &json["algebra"] {
            Value::Null => AlgebraFields::default(),
            algebra => AlgebraFields::deserialize(algebra)
                .map_err(|e| anyhow!("Invalid `algebra` object: {e}"))?,
        };
        let spec = Self {
            p,
            notation: fields.notation,
            kind: fields.kind,
            profile: fields.profile,
        };
        if spec.notation == Some(AlgebraType::Adem) {
            spec.check_adem()?;
        }
        Ok(spec)
    }

    /// Whether the Adem basis can express this algebra. It only exists for the classical one, and
    /// has no profiles.
    fn check_adem(&self) -> anyhow::Result<()> {
        if self.kind != SteenrodKind::Classical {
            return Err(anyhow!("The Adem basis does not exist for {}", self.kind));
        }
        if self.profile.is_some() {
            return Err(anyhow!("The Adem basis does not support a profile"));
        }
        Ok(())
    }

    /// Whether this is the classical Steenrod algebra with no profile.
    pub fn is_full_classical(&self) -> bool {
        self.kind == SteenrodKind::Classical && self.profile.is_none()
    }

    /// Whether the module can be built in `basis`.
    pub fn supports(&self, basis: AlgebraType) -> bool {
        self.basis(Some(basis)).is_ok()
    }

    /// The basis to build the module in.
    ///
    /// This is `requested` if given, else the module's notation, else Milnor.
    pub fn basis(&self, requested: Option<AlgebraType>) -> anyhow::Result<AlgebraType> {
        let basis = requested.or(self.notation).unwrap_or(AlgebraType::Milnor);
        if let Some(notation) = self.notation
            && notation != basis
        {
            return Err(anyhow!(
                "The module is written in {notation} notation, but {basis} was requested"
            ));
        }
        if basis == AlgebraType::Adem {
            self.check_adem()?;
        }
        Ok(basis)
    }
}

impl SteenrodAlgebra {
    /// The Hopf algebra this is a sub-Hopf-algebra of. The Adem basis only exists for the
    /// classical one.
    pub fn kind(&self) -> SteenrodKind {
        match self {
            Self::AdemAlgebra(_) => SteenrodKind::Classical,
            Self::MilnorAlgebra(a) => a.kind(),
        }
    }

    /// Build the algebra of a module spec, in the basis given by [`AlgebraSpec::basis`].
    pub fn from_json(
        json: &Value,
        requested: impl Into<Option<AlgebraType>>,
        unstable: bool,
    ) -> anyhow::Result<Self> {
        let spec = AlgebraSpec::from_json(json)?;
        let basis = spec.basis(requested.into())?;

        let p = spec.p;
        let profile = spec.profile.unwrap_or_default();
        if !profile.is_valid() {
            return Err(anyhow!("Invalid profile: {profile:?}"));
        }

        Ok(match basis {
            AlgebraType::Adem => Self::AdemAlgebra(AdemAlgebra::new(p, unstable)),
            AlgebraType::Milnor => Self::MilnorAlgebra(match spec.kind {
                SteenrodKind::Classical => MilnorAlgebra::new_with_profile(p, profile, unstable),
                SteenrodKind::MotivicModTau if p == 2 => {
                    MilnorAlgebra::motivic_mod_tau(profile, unstable)
                }
                SteenrodKind::ReducedPowers if p != 2 => {
                    MilnorAlgebra::reduced_powers(p, profile, unstable)
                }
                kind => return Err(anyhow!("Algebra {kind} is not defined at p = {p}")),
            }),
        })
    }
}

macro_rules! dispatch_steenrod {
    () => {};
    ($vis:vis fn $method:ident$(<$($lt:lifetime),+>)?(&$($lt2:lifetime)?self$(, $arg:ident: $ty:ty )*$(,)?) $(-> $ret:ty)?; $($tail:tt)*) => {
        $vis fn $method$(<$($lt),+>)?(&$($lt2)?self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                SteenrodAlgebra::AdemAlgebra(a) => a.$method($($arg),*),
                SteenrodAlgebra::MilnorAlgebra(a) => a.$method($($arg),*),
            }
        }
        dispatch_steenrod!{$($tail)*}
    };
}

impl PairAlgebra for AdemAlgebra {
    type Element = crate::pair_algebra::MilnorPairElement;

    fn element_is_zero(_elt: &Self::Element) -> bool {
        unimplemented!()
    }

    fn finalize_element(_elt: &mut Self::Element) {
        unimplemented!()
    }

    fn p_tilde(&self) -> usize {
        0
    }

    fn new_pair_element(&self, _degree: i32) -> Self::Element {
        unimplemented!()
    }

    fn sigma_multiply_basis(
        &self,
        _result: &mut Self::Element,
        _coeff: u32,
        _r_degree: i32,
        _r_idx: usize,
        _s_degree: i32,
        _s_idx: usize,
    ) {
        unimplemented!()
    }

    fn a_multiply(
        &self,
        _result: FpSliceMut,
        _coeff: u32,
        _r_degree: i32,
        _r: FpSlice,
        _s_degree: i32,
        _s: &Self::Element,
    ) {
        unimplemented!()
    }

    fn element_to_bytes(
        &self,
        _elt: &Self::Element,
        _buffer: &mut impl io::Write,
    ) -> io::Result<()> {
        unimplemented!()
    }

    fn element_from_bytes(
        &self,
        _degree: i32,
        _buffer: &mut impl io::Read,
    ) -> io::Result<Self::Element> {
        unimplemented!()
    }
}

impl PairAlgebra for SteenrodAlgebra {
    type Element = crate::pair_algebra::MilnorPairElement;

    dispatch_steenrod! {
        fn p_tilde(&self) -> usize;
        fn new_pair_element(&self, degree: i32) -> Self::Element;
        fn sigma_multiply_basis(&self, result: &mut Self::Element, coeff: u32, r_degree: i32, r_idx: usize, s_degree: i32, s_idx: usize);
        fn sigma_multiply(&self, result: &mut Self::Element, coeff: u32, r_degree: i32, r: FpSlice, s_degree: i32, s: FpSlice);
        fn a_multiply(&self, result: FpSliceMut, coeff: u32, r_degree: i32, r: FpSlice, s_degree: i32, s: &Self::Element);
        fn element_to_bytes(&self, elt: &Self::Element, buffer: &mut impl io::Write) -> io::Result<()>;
        fn element_from_bytes(&self, degree: i32, buffer: &mut impl io::Read) -> io::Result<Self::Element>;
    }

    fn element_is_zero(elt: &Self::Element) -> bool {
        MilnorAlgebra::element_is_zero(elt)
    }

    fn finalize_element(elt: &mut Self::Element) {
        MilnorAlgebra::finalize_element(elt);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Build the algebra of `json` in the Milnor basis.
    fn milnor(json: Value) -> anyhow::Result<SteenrodAlgebra> {
        SteenrodAlgebra::from_json(&json, AlgebraType::Milnor, false)
    }

    #[test]
    fn notation_picks_the_default_basis() {
        let json = json!({ "p": 2, "algebra": { "notation": "adem" } });
        let algebra = SteenrodAlgebra::from_json(&json, None, false).unwrap();
        assert!(matches!(algebra, SteenrodAlgebra::AdemAlgebra(_)));
        let algebra = SteenrodAlgebra::from_json(&json!({ "p": 2 }), None, false).unwrap();
        assert!(matches!(algebra, SteenrodAlgebra::MilnorAlgebra(_)));
    }

    /// A request the module cannot honour is an error rather than a silent switch.
    #[test]
    fn unsupported_basis_is_an_error() {
        let json = json!({ "p": 2, "algebra": { "notation": "milnor" } });
        assert!(SteenrodAlgebra::from_json(&json, AlgebraType::Adem, false).is_err());
        let profile = json!({ "truncated": true, "p_part": [1] });
        let json = json!({ "p": 2, "algebra": { "profile": profile } });
        assert!(SteenrodAlgebra::from_json(&json, AlgebraType::Adem, false).is_err());
        let algebra = SteenrodAlgebra::from_json(&json, None, false).unwrap();
        assert!(matches!(algebra, SteenrodAlgebra::MilnorAlgebra(_)));
        assert!(milnor(json!({ "p": 2, "algebra": { "notation": "serre" } })).is_err());
    }

    /// The Adem basis would silently drop the profile.
    #[test]
    fn profile_rejects_adem_notation() {
        let profile = json!({ "truncated": true, "p_part": [1] });
        let json = json!({ "p": 2, "algebra": { "notation": "adem", "profile": profile } });
        assert!(milnor(json).is_err());
    }

    /// The fields of the old flat format are refused rather than ignored.
    #[test]
    fn algebra_fields_at_top_level() {
        assert!(milnor(json!({ "p": 2, "algebra": ["milnor"] })).is_err());
        assert!(milnor(json!({ "p": 2, "algebra": { "bases": ["milnor"] } })).is_err());
        let profile = json!({ "truncated": true, "p_part": [1] });
        assert!(milnor(json!({ "p": 2, "profile": profile })).is_err());
        assert!(milnor(json!({ "p": 2, "kind": "classical" })).is_err());
    }

    #[test]
    fn kind_defaults_to_classical() {
        let algebra = milnor(json!({ "p": 2 })).unwrap();
        assert_eq!(algebra.kind(), SteenrodKind::Classical);
    }

    #[test]
    fn motivic_mod_tau() {
        let algebra = milnor(json!({ "p": 2, "algebra": { "kind": "motivic_mod_tau" } })).unwrap();
        assert_eq!(algebra.kind(), SteenrodKind::MotivicModTau);
        // `Q_0` and `P^1`, which is `Sq^2` in the classical algebra.
        algebra.compute_basis(2);
        assert_eq!(algebra.dimension(1), 1);
        assert_eq!(algebra.dimension(2), 1);
    }

    #[test]
    fn other_kinds_have_no_adem_basis() {
        let json = json!({ "p": 2, "algebra": { "kind": "motivic_mod_tau" } });
        assert!(SteenrodAlgebra::from_json(&json, AlgebraType::Adem, false).is_err());
        let json = json!({ "p": 2, "algebra": { "notation": "adem", "kind": "motivic_mod_tau" } });
        assert!(milnor(json).is_err());
    }

    #[test]
    fn kind_must_exist_at_the_prime() {
        assert!(milnor(json!({ "p": 2, "algebra": { "kind": "reduced_powers" } })).is_err());
        #[cfg(feature = "odd-primes")]
        assert!(milnor(json!({ "p": 3, "algebra": { "kind": "motivic_mod_tau" } })).is_err());
    }

    #[test]
    fn unknown_kind() {
        assert!(milnor(json!({ "p": 2, "algebra": { "kind": "motivic" } })).is_err());
    }

    #[test]
    fn invalid_profile() {
        let profile = json!({ "truncated": true, "p_part": [1, 2] });
        let json = json!({ "p": 2, "algebra": { "profile": profile } });
        assert!(milnor(json).is_err());
    }

    #[cfg(feature = "odd-primes")]
    #[test]
    fn reduced_powers() {
        let algebra = milnor(json!({ "p": 3, "algebra": { "kind": "reduced_powers" } })).unwrap();
        assert_eq!(algebra.kind(), SteenrodKind::ReducedPowers);
        // There is no Bockstein, so nothing below `P^1`.
        assert_eq!(algebra.basis_element_from_string("b"), None);
        assert_eq!(algebra.basis_element_from_string("P1"), Some((4, 0)));
    }

    #[test]
    fn kind_is_reported_in_both_bases() {
        let adem = SteenrodAlgebra::AdemAlgebra(AdemAlgebra::new(fp::prime::TWO, false));
        assert_eq!(adem.kind(), SteenrodKind::Classical);
        let ctau = MilnorAlgebra::motivic_mod_tau(MilnorProfile::default(), false);
        assert_eq!(
            SteenrodAlgebra::MilnorAlgebra(ctau).kind(),
            SteenrodKind::MotivicModTau
        );
    }
}
