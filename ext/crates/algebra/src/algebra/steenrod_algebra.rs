#[cfg(feature = "json")]
use crate::algebra::JsonAlgebra;
use crate::algebra::{
    AdemAlgebra, AdemAlgebraT, Algebra, Bialgebra, GeneratedAlgebra, MilnorAlgebra, MilnorAlgebraT,
};
use crate::dispatch_algebra;
use fp::prime::ValidPrime;
use fp::vector::{Slice, SliceMut};

use anyhow::anyhow;

use std::io::{Read, Write};

#[cfg(feature = "json")]
use {serde::Deserialize, serde_json::Value};

// This is here so that the Python bindings can use modules defined aor SteenrodAlgebraT with their own algebra enum.
// In order for things to work SteenrodAlgebraT cannot implement Algebra.
// Otherwise, the algebra enum for our bindings will see an implementation clash.
pub trait SteenrodAlgebraT: Send + Sync + Algebra {
    fn steenrod_algebra(&self) -> SteenrodAlgebraBorrow;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

pub enum SteenrodAlgebraBorrow<'a> {
    BorrowAdem(&'a AdemAlgebra),
    BorrowMilnor(&'a MilnorAlgebra),
}

pub enum SteenrodAlgebra {
    AdemAlgebra(AdemAlgebra),
    MilnorAlgebra(MilnorAlgebra),
}

impl From<AdemAlgebra> for SteenrodAlgebra {
    fn from(adem: AdemAlgebra) -> SteenrodAlgebra {
        SteenrodAlgebra::AdemAlgebra(adem)
    }
}

impl From<MilnorAlgebra> for SteenrodAlgebra {
    fn from(milnor: MilnorAlgebra) -> SteenrodAlgebra {
        SteenrodAlgebra::MilnorAlgebra(milnor)
    }
}

impl std::fmt::Display for SteenrodAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.fmt(f),
            SteenrodAlgebra::MilnorAlgebra(a) => a.fmt(f),
        }
    }
}

impl SteenrodAlgebraT for SteenrodAlgebra {
    fn steenrod_algebra(&self) -> SteenrodAlgebraBorrow {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => SteenrodAlgebraBorrow::BorrowAdem(a),
            SteenrodAlgebra::MilnorAlgebra(a) => SteenrodAlgebraBorrow::BorrowMilnor(a),
        }
    }
}

impl<A: SteenrodAlgebraT> AdemAlgebraT for A {
    fn adem_algebra(&self) -> &AdemAlgebra {
        match self.steenrod_algebra() {
            SteenrodAlgebraBorrow::BorrowAdem(a) => a,
            SteenrodAlgebraBorrow::BorrowMilnor(_) => panic!(),
        }
    }
}

impl<A: SteenrodAlgebraT> MilnorAlgebraT for A {
    fn milnor_algebra(&self) -> &MilnorAlgebra {
        match self.steenrod_algebra() {
            SteenrodAlgebraBorrow::BorrowAdem(_) => panic!(),
            SteenrodAlgebraBorrow::BorrowMilnor(a) => a,
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

impl Bialgebra for SteenrodAlgebra {
    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.decompose(op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => a.decompose(op_deg, op_idx),
        }
    }

    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.coproduct(op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => a.coproduct(op_deg, op_idx),
        }
    }
}

#[cfg(feature = "json")]
#[derive(Deserialize, Debug)]
struct AlgebraSpec {
    p: ValidPrime,
    algebra: Option<Vec<String>>,
    profile: Option<crate::algebra::milnor_algebra::MilnorProfile>,
}

#[cfg(feature = "json")]
impl SteenrodAlgebra {
    pub fn from_json(
        json: &Value,
        mut algebra_type: AlgebraType,
    ) -> anyhow::Result<SteenrodAlgebra> {
        let spec: AlgebraSpec = AlgebraSpec::deserialize(json)?;

        if let Some(list) = spec.algebra {
            let algebra_name = &algebra_type.to_string();
            if !list.iter().any(|x| x == algebra_name) {
                println!("Module does not support algebra {}", algebra_name);
                println!("Using {} instead", list[0]);
                algebra_type = list[0].parse()?;
            }
        }

        Ok(match algebra_type {
            AlgebraType::Adem => AdemAlgebra::new(spec.p, *spec.p != 2, false, false).into(),
            AlgebraType::Milnor => {
                MilnorAlgebra::new_with_profile(spec.p, spec.profile.unwrap_or_default()).into()
            }
        })
    }

    pub fn to_json(&self, json: &mut Value) {
        match self {
            SteenrodAlgebra::MilnorAlgebra(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::from(a.generic());
                let profile = a.profile();

                if !profile.is_trivial() {
                    json["algebra"] = Value::from(vec!["milnor"]);
                    json["profile"] = Value::Object(serde_json::map::Map::with_capacity(3));
                    if profile.truncated {
                        json["profile"]["truncated"] = Value::Bool(true);
                    }
                    if profile.q_part != !0 {
                        json["profile"]["q_part"] = Value::from(profile.q_part);
                    }
                    if !profile.p_part.is_empty() {
                        json["profile"]["p_part"] = Value::from(profile.p_part.clone());
                    }
                }
            }
            SteenrodAlgebra::AdemAlgebra(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::Bool(a.generic);
            }
        }
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

dispatch_algebra!(SteenrodAlgebra, dispatch_steenrod);

#[cfg(feature = "json")]
impl JsonAlgebra for SteenrodAlgebra {
    dispatch_steenrod! {
        fn prefix(&self) -> &str;
        fn json_to_basis(&self, json: &serde_json::Value) -> anyhow::Result<(i32, usize)>;
        fn json_from_basis(&self, degree: i32, idx: usize) -> serde_json::Value;
    }
}

/// An algebra with a specified list of generators and generating relations. This data can be used
/// to specify modules by specifying the actions of the generators.
impl GeneratedAlgebra for SteenrodAlgebra {
    dispatch_steenrod! {
        fn generators(&self, degree: i32) -> Vec<usize>;
        fn generator_to_string(&self, degree: i32, idx: usize) -> String;
        fn string_to_generator<'a, 'b>(&'a self, input: &'b str) -> nom::IResult<&'b str, (i32, usize)>;

        fn decompose_basis_element(
            &self,
            degree: i32,
            idx: usize,
        ) -> Vec<(u32, (i32, usize), (i32, usize))>;

        fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>;
    }
}

impl crate::pair_algebra::PairAlgebra for AdemAlgebra {
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
        _result: SliceMut,
        _coeff: u32,
        _r_degree: i32,
        _r: Slice,
        _s_degree: i32,
        _s: &Self::Element,
    ) {
        unimplemented!()
    }

    fn element_to_bytes(
        &self,
        _elt: &Self::Element,
        _buffer: &mut impl Write,
    ) -> std::io::Result<()> {
        unimplemented!()
    }

    fn element_from_bytes(
        &self,
        _degree: i32,
        _buffer: &mut impl Read,
    ) -> std::io::Result<Self::Element> {
        unimplemented!()
    }
}

impl crate::pair_algebra::PairAlgebra for SteenrodAlgebra {
    type Element = crate::pair_algebra::MilnorPairElement;

    fn element_is_zero(elt: &Self::Element) -> bool {
        MilnorAlgebra::element_is_zero(elt)
    }
    fn finalize_element(elt: &mut Self::Element) {
        MilnorAlgebra::finalize_element(elt);
    }

    dispatch_steenrod! {
        fn p_tilde(&self) -> usize;
        fn new_pair_element(&self, degree: i32) -> Self::Element;
        fn sigma_multiply_basis(&self, result: &mut Self::Element, coeff: u32, r_degree: i32, r_idx: usize, s_degree: i32, s_idx: usize);
        fn sigma_multiply(&self, result: &mut Self::Element, coeff: u32, r_degree: i32, r: Slice, s_degree: i32, s: Slice);
        fn a_multiply(&self, result: SliceMut, coeff: u32, r_degree: i32, r: Slice, s_degree: i32, s: &Self::Element);
        fn element_to_bytes(&self, elt: &Self::Element, buffer: &mut impl Write) -> std::io::Result<()>;
        fn element_from_bytes(&self, degree: i32, buffer: &mut impl Read) -> std::io::Result<Self::Element>;
    }
}
