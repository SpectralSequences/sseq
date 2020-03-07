use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra, AdemAlgebra, MilnorAlgebra};
use crate::algebra::adem_algebra::AdemBasisElement;
use crate::steenrod_parser::BocksteinOrSq;
use crate::module::Module;
use crate::steenrod_parser::*;
use crate::change_of_basis;
use std::error::Error;
use std::collections::HashMap;

pub struct SteenrodCalculator {
    adem_algebra : AdemAlgebra,
    milnor_algebra : MilnorAlgebra
}

impl SteenrodCalculator {
    pub fn new(p : ValidPrime) -> Self {
        Self {
            adem_algebra : AdemAlgebra::new(p, *p != 2, false),
            milnor_algebra : MilnorAlgebra::new(p)
        }
    }

    pub fn compute_basis(&self, degree : i32){
        self.adem_algebra.compute_basis(degree);
        self.milnor_algebra.compute_basis(degree);
    }

    pub fn evaluate_adem_to_string(&self, input : &str) -> Result<String, Box<dyn Error>>{
        self.evaluate_adem(input).map(|(d, vect)| self.adem_algebra.element_to_string(d, &vect))
    }

    pub fn evaluate_milnor_to_string(&self, input : &str) -> Result<String, Box<dyn Error>>{
        self.evaluate_milnor(input).map(|(d, vect)| self.milnor_algebra.element_to_string(d, &vect))//
    }

    pub fn evaluate_adem(&self, input : &str) -> Result<(i32, FpVector), Box<dyn Error>> {
        evaluate_algebra_adem(&self.adem_algebra, &self.milnor_algebra, input)
    }

    pub fn evaluate_milnor(&self, input : &str) -> Result<(i32, FpVector), Box<dyn Error>> {
        evaluate_algebra_milnor(&self.adem_algebra, &self.milnor_algebra, input)
    }
}

// Outputs in the Adem basis.
pub fn evaluate_algebra_adem(adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, input : &str) -> Result<(i32, FpVector), Box<dyn Error>> {
    evaluate_algebra_tree(adem_algebra, milnor_algebra, parse_algebra(input)?)
}

// Outputs in the Milnor basis
pub fn evaluate_algebra_milnor(adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, input : &str) -> Result<(i32, FpVector), Box<dyn Error>> {
    let adem_result = evaluate_algebra_adem(adem_algebra, milnor_algebra, input);
    if let Ok((degree, adem_vector)) = adem_result {
        let mut milnor_vector = FpVector::new(adem_vector.prime(), adem_vector.dimension());
        change_of_basis::adem_to_milnor(adem_algebra, milnor_algebra, &mut milnor_vector, 1, degree, &adem_vector);
        Ok((degree, milnor_vector))
    } else {
        adem_result
    }
}

fn evaluate_algebra_tree(adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, tree : AlgebraParseNode) -> Result<(i32, FpVector), Box<dyn Error>> {
    evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, None, tree)
}

fn evaluate_algebra_tree_helper(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    mut output_degree : Option<i32>, 
    tree : AlgebraParseNode
) -> Result<(i32, FpVector), Box<dyn Error>> {
    let p = adem_algebra.prime();
    match tree {
        AlgebraParseNode::Sum(left, right) => {
            let (degree_left, mut output_left) = evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, output_degree, *left)?;
            let (_degree_right, output_right) = evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, Some(degree_left), *right)?;
            output_left += &output_right;
            Ok((degree_left, output_left))
        }
        AlgebraParseNode::Product(left, right) => {
            let (degree_left, output_left) = evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, None, *left)?;
            if let Some(degree) = output_degree {
                if degree < degree_left {
                    return Err(Box::new(DegreeError{}));
                }
                output_degree = Some(degree - degree_left);
            }
            let (degree_right, output_right) = evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, output_degree, *right)?;
            let degree = degree_left + degree_right;
            adem_algebra.compute_basis(degree);
            milnor_algebra.compute_basis(degree);            
            let mut result = FpVector::new(p, adem_algebra.dimension(degree, -1));
            adem_algebra.multiply_element_by_element(&mut result, 1, degree_left, &output_left, degree_right, &output_right, -1);
            Ok((degree, result))
        },
        AlgebraParseNode::BasisElt(basis_elt) => {
            evaluate_basis_element(adem_algebra, milnor_algebra, output_degree, basis_elt)
        },
        AlgebraParseNode::Scalar(x) => {
            if let Some(degree) = output_degree {
                if degree != 0 {
                    return Err(Box::new(DegreeError{}));
                }
            }
            let mut result = FpVector::new(p, 1);
            let p = *p as i32;
            result.set_entry(0, (((x % p) + p) % p) as u32);
            Ok((0, result))
        }
    }
}

fn evaluate_basis_element(
    adem_algebra : &AdemAlgebra, 
    milnor_algebra : &MilnorAlgebra, 
    output_degree : Option<i32>, basis_elt : AlgebraBasisElt
) -> Result<(i32, FpVector), Box<dyn Error>> {
    let p = adem_algebra.prime();
    let q = if adem_algebra.generic { 2 * (*p) - 2 } else { 1 };
    let degree : i32;
    let mut result;
    match basis_elt {
        AlgebraBasisElt::AList(p_or_b_list) => {
            let degree_result = evaluate_p_or_b_list(adem_algebra, &p_or_b_list);
            degree = degree_result.0;
            result = degree_result.1;
        }
        AlgebraBasisElt::PList(p_list) => {
            let xi_degrees = crate::algebra::combinatorics::xi_degrees(p);
            let mut temp_deg = 0;
            for (i, v) in p_list.iter().enumerate() {
                temp_deg += *v * q * xi_degrees[i] as u32;
            }
            degree = temp_deg as i32;
            adem_algebra.compute_basis(degree);
            milnor_algebra.compute_basis(degree);            
            result = FpVector::new(p, adem_algebra.dimension(degree, -1));
            change_of_basis::adem_plist(adem_algebra, milnor_algebra, &mut result, 1, degree, p_list);
        }
        AlgebraBasisElt::P(x) => {
            let q = if adem_algebra.generic { 2 * *adem_algebra.prime() - 2} else {1};
            adem_algebra.compute_basis((x * q) as i32);
            milnor_algebra.compute_basis((x * q) as i32);
            let tuple = adem_algebra.beps_pn(0, x);
            degree = tuple.0;
            let idx = tuple.1;
            result = FpVector::new(p, adem_algebra.dimension(degree, -1));
            result.set_entry(idx, 1);
        }
        AlgebraBasisElt::Q(x) => {
            let tau_degrees = crate::algebra::combinatorics::tau_degrees(p);
            degree = tau_degrees[x as usize];
            adem_algebra.compute_basis(degree);
            milnor_algebra.compute_basis(degree);            
            result = FpVector::new(p, adem_algebra.dimension(degree, -1));
            change_of_basis::adem_q(adem_algebra, milnor_algebra, &mut result, 1, x);
        }
    }
    if let Some(requested_degree) = output_degree {
        if degree != requested_degree {
            return Err(Box::new(DegreeError{}));
        }
    }
    Ok((degree, result))
}


pub fn evaluate_module<M : Module>(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    module : &M, 
    basis_elt_lookup : &HashMap<String, (i32, usize)>, 
    input : &str
) -> Result<(i32, FpVector), Box<dyn Error>> {
    evaluate_module_tree(adem_algebra, milnor_algebra, module, basis_elt_lookup, parse_module(input)?)
}

fn evaluate_module_tree<M : Module>(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    module : &M, 
    basis_elt_lookup : &HashMap<String, (i32, usize)>, 
    tree : ModuleParseNode
) -> Result<(i32, FpVector), Box<dyn Error>> {
    evaluate_module_tree_helper(adem_algebra, milnor_algebra, module, basis_elt_lookup, None, tree)
}

fn evaluate_module_tree_helper<M : Module>(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    module : &M, 
    basis_elt_lookup : &HashMap<String, (i32, usize)>,    
    mut output_degree : Option<i32>, 
    tree : ModuleParseNode
) -> Result<(i32, FpVector), Box<dyn Error>> {
    let p = adem_algebra.prime();
    match tree {
        ModuleParseNode::Sum(left, right) => {
            let (degree_left, mut output_left) = evaluate_module_tree_helper(adem_algebra, milnor_algebra, module, basis_elt_lookup, output_degree, *left)?;
            let (_degree_right, output_right) = evaluate_module_tree_helper(adem_algebra, milnor_algebra, module, basis_elt_lookup, Some(degree_left), *right)?;
            output_left += &output_right;
            Ok((degree_left, output_left))
        }
        ModuleParseNode::Act(left, right) => {
            let (degree_left, output_left) = evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, None, *left)?;
            if let Some(degree) = output_degree {
                if degree < degree_left {
                    return Err(Box::new(DegreeError{}));
                }
                output_degree = Some(degree - degree_left);
            }
            let (degree_right, output_right) = evaluate_module_tree_helper(adem_algebra, milnor_algebra, module, basis_elt_lookup, output_degree, *right)?;
            let degree = degree_left + degree_right;
            let mut result = FpVector::new(p, module.dimension(degree));
            module.act_by_element(&mut result, 1, degree_left, &output_left, degree_right, &output_right);
            Ok((degree, result))
        },
        ModuleParseNode::ModuleBasisElt(basis_elt) => {
            evaluate_module_basis_element(adem_algebra, milnor_algebra, module, basis_elt_lookup, output_degree, basis_elt)
        },
    }
}

fn evaluate_module_basis_element<M : Module>(
    adem_algebra : &AdemAlgebra, 
    _milnor_algebra : &MilnorAlgebra,
    module : &M,
    basis_elt_lookup : &HashMap<String, (i32, usize)>, 
    output_degree : Option<i32>, basis_elt : String
) -> Result<(i32, FpVector), Box<dyn Error>> {
    let p = adem_algebra.prime();
    let entry = basis_elt_lookup.get(&basis_elt);
    let degree;
    let idx;
    match entry {
        Some(tuple) => {degree = tuple.0; idx = tuple.1;},
        None => return Err(Box::new(UnknownBasisElementError { name : basis_elt })) // Should be basis element not found error or something.
    }
    
    if let Some(requested_degree) = output_degree {
        if degree != requested_degree {
            return Err(Box::new(DegreeError{}));
        }
    }
    let mut result = FpVector::new(p, module.dimension(degree));
    result.set_entry(idx, 1);
    Ok((degree, result))
}

fn bockstein_or_sq_to_adem_basis_elt(e : &BocksteinOrSq, q : i32) -> AdemBasisElement {
    match e {
        BocksteinOrSq::Bockstein => if q == 1 {
            AdemBasisElement {
                degree : 1,
                excess : 1,
                bocksteins : 0,
                ps : vec![1]
            }
        } else {
            AdemBasisElement {
                degree : 1,
                excess : 1,
                bocksteins : 1,
                ps : vec![]
            }            
        },
        BocksteinOrSq::Sq(x) => AdemBasisElement {
            degree : (*x) as i32 * q,
            excess : 2 * (*x) as i32,
            bocksteins : 0,
            ps : vec![*x]         
        }
    }
}

fn evaluate_p_or_b_list(adem_algebra : &AdemAlgebra, list : &[BocksteinOrSq]) -> (i32, FpVector) {
    let p = adem_algebra.prime();
    let q = if adem_algebra.generic { 2*(*p) as i32 - 2} else { 1 };
    let first_elt = bockstein_or_sq_to_adem_basis_elt(&list[0], q);
    let mut total_degree = first_elt.degree;
    adem_algebra.compute_basis(total_degree);
    let idx = adem_algebra.basis_element_to_index(&first_elt);
    let cur_dim = adem_algebra.dimension(total_degree, -1);

    let mut tmp_vector_a = FpVector::new(p, cur_dim);
    let mut tmp_vector_b = FpVector::new(p, 0);

    tmp_vector_a.set_entry(idx, 1);

    for item in &list[1..] {
        let cur_elt = bockstein_or_sq_to_adem_basis_elt(&item, q);
        let idx = adem_algebra.basis_element_to_index(&cur_elt);
        let cur_dim = adem_algebra.dimension(total_degree + cur_elt.degree, -1);
        tmp_vector_b.set_scratch_vector_size(cur_dim);
        adem_algebra.multiply_element_by_basis_element(&mut tmp_vector_b, 1, total_degree, &tmp_vector_a, cur_elt.degree, idx, -1);
        total_degree += cur_elt.degree;
        std::mem::swap(&mut tmp_vector_a, &mut tmp_vector_b);
        tmp_vector_b.set_to_zero();
    }
    (total_degree, tmp_vector_a)
}

#[derive(Debug)]
pub struct DegreeError {}

impl std::fmt::Display for DegreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Encountered inhomogenous sum")
    }
}

impl Error for DegreeError {
    fn description(&self) -> &str {
        "Encountered inhomogenous sum"
    }
}

#[derive(Debug)]
pub struct UnknownBasisElementError {
    name : String
}

impl std::fmt::Display for UnknownBasisElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown basis element '{}'", self.name)
    }
}

impl Error for UnknownBasisElementError {
    fn description(&self) -> &str {
        "Uknown basis element"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use rstest::rstest_parametrize;

    #[test]
    fn test_evaluate(){
        let p = ValidPrime::new(2);
        let max_degree = 30;
        let adem = AdemAlgebra::new(p, *p != 2, false);
        let milnor = MilnorAlgebra::new(p);//, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        println!("{:?}", milnor.basis_element_from_index(1, 0));

        for (input, output) in &[
            ("Sq2 * Sq2", "P3 P1"),
            ("A(2 2)", "P3 P1"),
            ("Sq2 * Sq2 * Sq2 + Sq3 * Sq3", "0"),
            ("Sq2 * (Sq2 * Sq2 + Sq4)", "P6"),
            ("Sq7 + Q2","P5 P2 + P6 P1 + P4 P2 P1"),            
            ("(Q2 + Sq7) * Q1", "P6 P3 P1"),
        ]{
            let (degree, result) = evaluate_algebra_adem(&adem, &milnor, input).unwrap();
            println!("{} ==> {}", input, adem.element_to_string(degree, &result));
            assert_eq!(adem.element_to_string(degree, &result), *output);
        }
        let p = ValidPrime::new(3);
        let max_degree = 30;
        let adem = AdemAlgebra::new(p, *p != 2, false);
        let milnor = MilnorAlgebra::new(p);//, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        for (input, output) in &[
            ("P1 * P1", "2 * P2"),
            ("A(1 1)", "2 * P2"),
            ("A(1 b 1)", "b P2 + P2 b"),
            ("A(4 2)", "2 * P5 P1"),
        ]{
            let (degree, result) = evaluate_algebra_adem(&adem, &milnor, input).unwrap();
            println!("{} ==> {}", input, adem.element_to_string(degree, &result));
            assert_eq!(adem.element_to_string(degree, &result), *output);
        }
    }
}
