#[cfg(feature = "json")]
use crate::algebra::JsonAlgebra;
use crate::algebra::{
    AdemAlgebra, AdemAlgebraT, Algebra, Bialgebra, GeneratedAlgebra, MilnorAlgebra, MilnorAlgebraT,
};
use crate::dispatch_algebra;
use fp::prime::ValidPrime;
use fp::vector::{Slice, SliceMut};

#[cfg(feature = "json")]
use {serde::Deserialize, serde_json::Value};

// This is here so that the Python bindings can use modules defined aor SteenrodAlgebraT with their own algebra enum.
// In order for things to work SteenrodAlgebraT cannot implement Algebra.
// Otherwise, the algebra enum for our bindings will see an implementation clash.
pub trait SteenrodAlgebraT: Send + Sync + 'static + Algebra {
    fn steenrod_algebra(&self) -> SteenrodAlgebraBorrow;
}

#[derive(Copy, Clone, Debug)]
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
    type Error = error::GenericError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "adem" => Ok(Self::Adem),
            "milnor" => Ok(Self::Milnor),
            _ => Err(error::GenericError::new(format!(
                "Invalid algebra name: {}",
                s
            ))),
        }
    }
}

impl std::str::FromStr for AlgebraType {
    type Err = error::GenericError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "adem" => Ok(Self::Adem),
            "milnor" => Ok(Self::Milnor),
            _ => Err(error::GenericError::new(format!(
                "Invalid algebra name: {}",
                s
            ))),
        }
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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
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
struct MilnorProfileOption {
    truncated: Option<bool>,
    q_part: Option<u32>,
    p_part: Option<crate::algebra::milnor_algebra::PPart>,
}

#[cfg(feature = "json")]
#[derive(Deserialize, Debug)]
struct AlgebraSpec {
    p: u32,
    algebra: Option<Vec<String>>,
    profile: Option<MilnorProfileOption>,
}

#[cfg(feature = "json")]
impl SteenrodAlgebra {
    pub fn from_json(
        json: &Value,
        mut algebra_type: AlgebraType,
    ) -> error::Result<SteenrodAlgebra> {
        // This line secretly redefines the lifetime of algebra_name so that we can reassign it
        // later on.
        let spec: AlgebraSpec = serde_json::from_value(json.clone())?;

        let p = ValidPrime::try_new(spec.p)
            .ok_or_else(|| error::GenericError::new(format!("Invalid prime: {}", spec.p)))?;

        if let Some(list) = spec.algebra.as_ref() {
            let algebra_name = &algebra_type.to_string();
            if !list.iter().any(|x| x == algebra_name) {
                println!("Module does not support algebra {}", algebra_name);
                println!("Using {} instead", list[0]);
                algebra_type = list[0].parse()?;
            }
        }

        Ok(match algebra_type {
            AlgebraType::Adem => {
                SteenrodAlgebra::AdemAlgebra(AdemAlgebra::new(p, *p != 2, false, false))
            }
            AlgebraType::Milnor => {
                let mut algebra_inner = MilnorAlgebra::new(p);
                if let Some(profile) = spec.profile {
                    if let Some(truncated) = profile.truncated {
                        algebra_inner.profile.truncated = truncated;
                    }
                    if let Some(q_part) = profile.q_part {
                        algebra_inner.profile.q_part = q_part;
                    }
                    if let Some(p_part) = profile.p_part {
                        algebra_inner.profile.p_part = p_part;
                    }
                }
                SteenrodAlgebra::MilnorAlgebra(algebra_inner)
            }
        })
    }

    pub fn to_json(&self, json: &mut Value) {
        match self {
            SteenrodAlgebra::MilnorAlgebra(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::from(a.generic());

                if !a.profile.is_trivial() {
                    json["algebra"] = Value::from(vec!["milnor"]);
                    json["profile"] = Value::Object(serde_json::map::Map::with_capacity(3));
                    if a.profile.truncated {
                        json["profile"]["truncated"] = Value::Bool(true);
                    }
                    if a.profile.q_part != !0 {
                        json["profile"]["q_part"] = Value::from(a.profile.q_part);
                    }
                    if !a.profile.p_part.is_empty() {
                        json["profile"]["p_part"] = Value::from(a.profile.p_part.clone());
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

#[derive(Debug)]
struct InvalidAlgebraError {
    name: String,
}

impl std::fmt::Display for InvalidAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid algebra: {}", &self.name)
    }
}

impl std::error::Error for InvalidAlgebraError {
    fn description(&self) -> &str {
        "Invalid algebra supplied"
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
        fn json_to_basis(&self, json: serde_json::Value) -> error::Result<(i32, usize)>;
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
