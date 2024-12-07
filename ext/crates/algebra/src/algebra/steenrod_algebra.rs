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
        AdemAlgebra, AdemAlgebraT, Algebra, Bialgebra, GeneratedAlgebra, MilnorAlgebra,
        MilnorAlgebraT,
    },
    dispatch_algebra,
};

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

impl std::fmt::Display for SteenrodAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::AdemAlgebra(a) => a.fmt(f),
            Self::MilnorAlgebra(a) => a.fmt(f),
        }
    }
}

impl SteenrodAlgebraT for SteenrodAlgebra {
    fn steenrod_algebra(&self) -> SteenrodAlgebraBorrow {
        match self {
            Self::AdemAlgebra(a) => SteenrodAlgebraBorrow::BorrowAdem(a),
            Self::MilnorAlgebra(a) => SteenrodAlgebraBorrow::BorrowMilnor(a),
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
            Self::AdemAlgebra(a) => a.decompose(op_deg, op_idx),
            Self::MilnorAlgebra(a) => a.decompose(op_deg, op_idx),
        }
    }

    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        match self {
            Self::AdemAlgebra(a) => a.coproduct(op_deg, op_idx),
            Self::MilnorAlgebra(a) => a.coproduct(op_deg, op_idx),
        }
    }
}

#[derive(Deserialize, Debug)]
struct AlgebraSpec {
    p: ValidPrime,
    algebra: Option<Vec<String>>,
    profile: Option<crate::algebra::milnor_algebra::MilnorProfile>,
}

impl SteenrodAlgebra {
    pub fn from_json(
        json: &Value,
        mut algebra_type: AlgebraType,
        unstable: bool,
    ) -> anyhow::Result<Self> {
        let spec: AlgebraSpec = AlgebraSpec::deserialize(json)?;

        if let Some(list) = spec.algebra {
            let algebra_name = &algebra_type.to_string();
            if !list.iter().any(|x| x == algebra_name) {
                println!("Module does not support algebra {algebra_name}");
                println!("Using {} instead", list[0]);
                algebra_type = list[0].parse()?;
            }
        }

        Ok(match algebra_type {
            AlgebraType::Adem => Self::AdemAlgebra(AdemAlgebra::new(spec.p, unstable)),
            AlgebraType::Milnor => Self::MilnorAlgebra(MilnorAlgebra::new_with_profile(
                spec.p,
                spec.profile.unwrap_or_default(),
                unstable,
            )),
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

dispatch_algebra!(SteenrodAlgebra, dispatch_steenrod);

/// An algebra with a specified list of generators and generating relations. This data can be used
/// to specify modules by specifying the actions of the generators.
impl GeneratedAlgebra for SteenrodAlgebra {
    dispatch_steenrod! {
        fn generators(&self, degree: i32) -> Vec<usize>;
        fn generator_to_string(&self, degree: i32, idx: usize) -> String;

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

impl crate::pair_algebra::PairAlgebra for SteenrodAlgebra {
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

impl crate::UnstableAlgebra for SteenrodAlgebra {
    dispatch_steenrod! {
        fn dimension_unstable(&self, degree: i32, excess: i32) -> usize;
        fn multiply_basis_elements_unstable(&self, result: FpSliceMut, coeff: u32, r_degree: i32, r_index: usize, s_degree: i32, s_index: usize, excess: i32);
        fn multiply_basis_element_by_element_unstable(&self, result: FpSliceMut, coeff: u32, r_degree: i32, r_idx: usize, s_degree: i32, s: FpSlice, excess: i32);
        fn multiply_element_by_basis_element_unstable(&self, result: FpSliceMut, coeff: u32, r_degree: i32, r: FpSlice, s_degree: i32, s_idx: usize, excess: i32);
        fn multiply_element_by_element_unstable(&self, result: FpSliceMut, coeff: u32, r_degree: i32, r: FpSlice, s_degree: i32, s: FpSlice, excess: i32);
    }
}
