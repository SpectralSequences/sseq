use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::Algebra;
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
use crate::steenrod_parser::*;
use crate::change_of_basis;
use std::error::Error;

pub fn evaluate_algebra(adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, input : &str) -> Result<(i32, FpVector), Box<dyn Error>> {
    evaluate_algebra_tree(adem_algebra, milnor_algebra, parse_algebra(input)?)
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
            let (degree_right, output_right) = evaluate_algebra_tree_helper(adem_algebra, milnor_algebra, Some(degree_left), *right)?;
            output_left.add(&output_right, 1);
            return Ok((degree_left, output_left));
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
            let mut result = FpVector::new(p, adem_algebra.get_dimension(degree, -1));
            adem_algebra.multiply_element_by_element(&mut result, 1, degree_left, &output_left, degree_right, &output_right, -1);
            return Ok((degree, result));
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
            let p = p as i32;
            result.set_entry(0, (((x % p) + p) % p) as u32);
            return Ok((0, result));
        }
    }
}

fn evaluate_basis_element(
    adem_algebra : &AdemAlgebra, 
    milnor_algebra : &MilnorAlgebra, 
    output_degree : Option<i32>, basis_elt : AlgebraBasisElt
) -> Result<(i32, FpVector), Box<dyn Error>> {
    let p = adem_algebra.prime();
    let q = if adem_algebra.generic { 2 * p - 2 } else { 1 };
    let degree : i32;
    let mut result;
    match basis_elt {
        AlgebraBasisElt::PList(p_list) => {
            let xi_degrees = crate::combinatorics::get_tau_degrees(p);
            let mut temp_deg = 0;
            for (i, v) in p_list.iter().enumerate() {
                temp_deg += *v * xi_degrees[i] as u32;
            }
            degree = temp_deg as i32;
            result = FpVector::new(p, adem_algebra.get_dimension(degree, -1));
            change_of_basis::get_adem_plist(adem_algebra, milnor_algebra, &mut result, 1, degree, p_list);
        }
        AlgebraBasisElt::P(x) => {
            let tuple = adem_algebra.get_beps_pn(0, x);
            degree = tuple.0;
            let idx = tuple.1;
            result = FpVector::new(p, adem_algebra.get_dimension(degree, -1));
            result.set_entry(idx, 1);
        }
        AlgebraBasisElt::Q(x) => {
            let tau_degrees = crate::combinatorics::get_tau_degrees(p);
            degree = tau_degrees[x as usize];
            result = FpVector::new(p, adem_algebra.get_dimension(degree, -1));
            change_of_basis::get_adem_q(adem_algebra, milnor_algebra, &mut result, 1, x);
        }
    }
    if let Some(requested_degree) = output_degree {
        if degree != requested_degree {
            return Err(Box::new(DegreeError{}));
        }
    }
    return Ok((degree, result)); 
}


#[derive(Debug)]
pub struct DegreeError {}

impl std::fmt::Display for DegreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Encountered inhomogenous sum.")
    }
}

impl Error for DegreeError {
    fn description(&self) -> &str {
        "Encountered inhomogenous sum"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest_parametrize;

    #[test]
    fn test_evaluate(){
        let p = 2;
        let max_degree = 30;
        let adem = AdemAlgebra::new(p, p != 2, false);
        let milnor = MilnorAlgebra::new(p);//, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        println!("{:?}", milnor.basis_element_from_index(1, 0));

        for (input, output) in vec![
            ("Sq2 * Sq2", "P3 P1"),
            ("Sq2 * Sq2 * Sq2 + Sq3 * Sq3", "0"),
            ("Sq2 * (Sq2 * Sq2 + Sq4)", "P6"),
            ("Sq7 + Q2","P5 P2 + P6 P1 + P4 P2 P1"),            
            ("(Q2 + Sq7) * Q1", "P6 P3 P1"),
        ]{
            let (degree, result) = evaluate_algebra(&adem, &milnor, input).unwrap();
            println!("{} ==> {}", input, adem.element_to_string(degree, &result));
            assert_eq!(adem.element_to_string(degree, &result), output);
        }
    }
}
