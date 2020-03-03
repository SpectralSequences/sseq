pub mod adem_algebra;
pub mod milnor_algebra;
pub mod field;

pub use adem_algebra::AdemAlgebra;
pub use milnor_algebra::MilnorAlgebra;
pub use field::Field;
use std::error::Error;

use fp::vector::FpVector;
use fp::prime::ValidPrime;
use nom::IResult;
use serde::Deserialize;
use serde_json::Value;
use enum_dispatch::enum_dispatch;

/// A graded algebra over F_p, finite dimensional in each degree, equipped with a choice of ordered
/// basis in each dimension. Basis elements of the algebra are referred to by their degree and
/// index, and general elements are referred to by the degree and an `FpVector` listing the
/// coefficients of the element in terms of the basis.
///
/// Since the graded algebra is often infinite dimensional, we cannot construct a complete
/// description of the algebra. Instead, we use the function `compute_basis(degree)`. When called,
/// the algebra should compute relevant data to be able to perform calculations up to degree
/// `degree`. It is the responsibility of users to ensure `compute_degree(degree)` is called before
/// calling other functions with the `degree` parameter.
///
/// The algebra should also come with a specified choice of algebra generators, which are
/// necessarily basis elements. It gives us a simpler way of describing finite modules by only
/// specifying the action of the generators.
#[enum_dispatch]
pub trait Algebra : Send + Sync + 'static {
    /// The "type" of the algebra, which is "adem" or "milnor". When reading module definitions,
    /// this informs whether we should look at adem_actions or milnor_actions.
    fn algebra_type(&self) -> &str;

    /// Returns the prime the algebra is over.
    fn prime(&self) -> ValidPrime;
    fn name(&self) -> &str { "" }

    /// Computes the list of basis elements up to and including degree `degree`. This should include any
    /// other preparation needed to evaluate all the other functions that involve a degree
    /// parameter. One should be able to call compute_basis multiple times, and there should be
    /// little overhead when calling `compute_basis(degree)` multiple times with the same `degree`.
    fn compute_basis(&self, degree : i32);

    /// Gets the dimension of the algebra in degree `degree`.
    fn dimension(&self, degree : i32, excess : i32) -> usize;

    /// Computes the product `r * s` of the two basis elements, and *adds* the result to `result`.
    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32);

    fn multiply_basis_element_by_element(&self, result : &mut FpVector, coeff : u32, r_degree : i32, r_idx : usize, s_degree : i32, s : &FpVector, excess : i32){
        let p = self.prime();
        for (i, v) in s.iter().enumerate() {
            if v == 0 { continue; }
            self.multiply_basis_elements(result, (coeff * v) % *p, r_degree, r_idx, s_degree, i, excess);
        }
    }

    fn multiply_element_by_basis_element(&self, result : &mut FpVector, coeff : u32, r_degree : i32, r : &FpVector, s_degree : i32, s_idx : usize, excess : i32){
        let p = self.prime();
        for (i, v) in r.iter().enumerate() {
            if v == 0 { continue; }
            self.multiply_basis_elements(result, (coeff * v) % *p, r_degree, i, s_degree, s_idx, excess);
        }
    }

    fn multiply_element_by_element(&self, result : &mut FpVector, coeff : u32, r_degree : i32, r : &FpVector, s_degree : i32, s : &FpVector, excess : i32){
        let p = self.prime();
        for (i, v) in s.iter().enumerate() {
            if v == 0 { continue; }
            self.multiply_element_by_basis_element(result, (coeff * v) % *p, r_degree, r, s_degree, i, excess);
        }
    }

    /// A filtration one element in Ext(k, k) is the same as an indecomposable element of the
    /// algebra.  This function returns a default list of such elements in the format `(name,
    /// degree, index)` for whom we want to compute products with in the resolutions.
    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> { Vec::new() }

    /// Converts a JSON object into a basis element. The way basis elements are represented by JSON
    /// objects is to be specified by the algebra itself, and will be used by module
    /// specifications.
    fn json_to_basis(&self, _json : Value) -> (i32, usize) { unimplemented!() }
    fn json_from_basis(&self, _degree : i32, _idx : usize) -> Value { unimplemented!() }

    /// Converts a basis element into a string for display.
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("a_{{{}, {}}}", degree, idx)
    }

    /// Converts an element into a string for display.
    fn element_to_string(&self, degree : i32, element : &FpVector) -> String {
        let mut result = String::new();
        let mut zero = true;
        for (idx, value) in element.iter().enumerate() {
            if value == 0 {
                continue;
            }
            zero = false;
            if value != 1 {
                result.push_str(&format!("{} * ", value));
            }
            let b = self.basis_element_to_string(degree, idx);
            result.push_str(&format!("{} + ", b));
        }
        if zero {
            result.push_str("0");
        } else {
            // Remove trailing " + "
            result.pop();
            result.pop();
            result.pop();
        }
        result
    }    

    /// Given a degree `degree`, the function returns a list of algebra generators in that degree.
    /// This return value is the list of indices of the basis elements that are generators. The
    /// list need not be in any particular order.
    ///
    /// This method need not be fast, because they will only be performed when constructing the module,
    /// and will often only involve low dimensional elements.
    fn generators(&self, _degree : i32) -> Vec<usize> { unimplemented!() }

    /// This returns the name of a generator. Note that the index is the index of the generator
    /// in the list of all basis elements. It is undefined behaviour to call this function with a
    /// (degree, index) pair that is not a generator.
    ///
    /// The default implementation calls `self.basis_element_to_string`, but occassionally the
    /// generators might have alternative, more concise names that are preferred.
    ///
    /// This function MUST be inverse to `string_to_generator`.
    fn generator_to_string(&self, degree: i32, idx: usize) -> String {
        self.basis_element_to_string(degree, idx)
    }

    /// This parses a string and returns the generator described by the string. The signature of
    /// this function is the same `nom` combinators.
    ///
    /// This function MUST be inverse to `string_to_generator` (and not `basis_element_to_string`).
    fn string_to_generator<'a, 'b>(&'a self, _input: &'b str) -> IResult<&'b str, (i32, usize)> { unimplemented!() }

    /// Given a non-generator basis element of the algebra, decompose it in terms of algebra
    /// generators. Recall each basis element is given by a pair $(d, i))$, where $d$ is the degree of
    /// the generator, and $i$ is the index of the basis element. Given a basis element $A$, the
    /// function returns a list of triples $(c_i, A_i, B_i)$ where each $A_i$ and $B_i$ are basis
    /// elements of strictly smaller degree than the original, and
    /// $$ A = \sum_i c_i A_i B_i.$$
    /// This allows us to recursively compute the action of the algebra.
    ///
    /// This method need not be fast, because they will only be performed when constructing the module,
    /// and will often only involve low dimensional elements.
    fn decompose_basis_element(&self, _degree : i32, _idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> { unimplemented!() }

    /// Get any relations that the algebra wants checked to ensure the consistency of module.
    fn relations_to_check(&self, _degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> { unimplemented!() }
}

#[enum_dispatch(Algebra)]
pub enum SteenrodAlgebra {
    AdemAlgebra,
    MilnorAlgebra,
}

impl Bialgebra for SteenrodAlgebra {
    fn decompose (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize)> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.decompose(op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => a.decompose(op_deg, op_idx),
        }
    }

    fn coproduct (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize, i32, usize)> {
        match self {
            SteenrodAlgebra::AdemAlgebra(a) => a.coproduct(op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => a.coproduct(op_deg, op_idx),
        }
    }
}


#[derive(Deserialize, Debug)]
struct MilnorProfileOption {
    truncated : Option<bool>,
    q_part : Option<u32>,
    p_part : Option<Vec<u32>>
}

#[derive(Deserialize, Debug)]
struct AlgebraSpec {
    p : u32,
    algebra : Option<Vec<String>>,
    profile : Option<MilnorProfileOption>
}

impl SteenrodAlgebra {
    pub fn from_json(json : &Value, mut algebra_name : String) -> Result<SteenrodAlgebra, Box<dyn Error>> {
        let spec : AlgebraSpec = serde_json::from_value(json.clone())?;

        let p = ValidPrime::new(spec.p);
        if let Some(mut list) = spec.algebra {
            if !list.contains(&algebra_name) {
                println!("Module does not support algebra {}", algebra_name);
                println!("Using {} instead", list[0]);
                algebra_name = list.remove(0);
            }
        }

        let algebra : SteenrodAlgebra;
        match algebra_name.as_ref() {
            "adem" => algebra = SteenrodAlgebra::from(AdemAlgebra::new(p, *p != 2, false)),
            "milnor" => {
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
                algebra = SteenrodAlgebra::from(algebra_inner);
            }
            _ => { return Err(Box::new(InvalidAlgebraError { name : algebra_name })); }
        };
        Ok(algebra)
    }

    pub fn to_json(&self, json: &mut Value) {
        match self {
            SteenrodAlgebra::MilnorAlgebra(a) => {
                json["p"] = Value::from(*a.prime());
                json["generic"] = Value::from(a.generic);

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
    name : String
}

impl std::fmt::Display for InvalidAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid algebra: {}", &self.name)
    }
}

impl Error for InvalidAlgebraError {
    fn description(&self) -> &str {
        "Invalid algebra supplied"
    }
}

pub trait Bialgebra : Algebra {
    /// This function decomposes an element of the algebra as a product of elements, each of whose
    /// coproduct is easy to calculate. The product is laid out such that the first element of the
    /// vector is applied to the module element first. This is to be used in conjunction with
    /// `coproduct`.
    ///
    /// This structure is motivated by the fact that in the admissible basis for the Adem algebra,
    /// an element naturally decomposes into a product of Steenrod squares, each of which has an
    /// easy coproduct formula.
    fn decompose (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize)>;

    /// Expresses Delta(x) as sum_j (A_{ij} (x) B_{ij}). Here x must be one of the elements
    /// returned by `decompose`.
    fn coproduct (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize, i32, usize)>;
}
