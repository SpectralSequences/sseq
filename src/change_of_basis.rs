use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra};
use crate::adem_algebra::{AdemAlgebra, AdemBasisElement};
use crate::milnor_algebra::{MilnorAlgebra, MilnorBasisElement};

// use std::rc::Rc;

pub fn adem_to_milnor_on_basis(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    let elt = adem_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let q = if milnor_algebra.generic { 2 * p - 2 } else { 1 };
    let dim = milnor_algebra.get_dimension(elt.degree, -1);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }    
    let mut tmp_vector_a = FpVector::get_scratch_vector(p, dim);
    let mut tmp_vector_b = FpVector::get_scratch_vector(p, dim);
    let mut bocksteins = elt.bocksteins;
    let mbe = MilnorBasisElement {
        degree : (q * elt.ps[0] + (bocksteins & 1)) as i32,
        q_part : bocksteins & 1,
        p_part : vec![elt.ps[0]]
    };
    bocksteins >>= 1;
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    let mut total_degree = mbe.degree;
    let cur_dim = milnor_algebra.get_dimension(total_degree, -1);
    tmp_vector_a = tmp_vector_a.set_scratch_vector_size(cur_dim);
    tmp_vector_a.set_entry(idx, 1);

    for i in 1 .. elt.ps.len() {
        let mbe = MilnorBasisElement {
            degree : (q * elt.ps[i] + (bocksteins & 1)) as i32,
            q_part : bocksteins & 1,
            p_part : vec![elt.ps[i]]
        };
        let idx = milnor_algebra.basis_element_to_index(&mbe);
        bocksteins >>= 1;
        let cur_dim = milnor_algebra.get_dimension(total_degree + mbe.degree, -1);
        tmp_vector_b = tmp_vector_b.set_scratch_vector_size(cur_dim);
        milnor_algebra.multiply_element_by_basis_element(&mut tmp_vector_b, 1, total_degree, &tmp_vector_a, mbe.degree, idx, -1);
        total_degree += mbe.degree;
        std::mem::swap(&mut tmp_vector_a, &mut tmp_vector_b);
        tmp_vector_b.set_to_zero();
    }
    if bocksteins & 1 != 0 {
        milnor_algebra.multiply_element_by_basis_element(result, coeff, total_degree, &tmp_vector_a, 1, 0, -1);
    } else {
        result.add(&mut tmp_vector_a, coeff);
    }
}

fn adem_to_milnor(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, input : &FpVector
){
    let p = milnor_algebra.prime();
    for (i, v) in input.iter().enumerate() {
        if v == 0 {
            continue;
        }
        adem_to_milnor_on_basis(adem_algebra, milnor_algebra, result, (coeff * v) % p, degree, i);
    }
}

// This is currently pretty inefficient... We should memoize results so that we don't repeatedly
// recompute the same inverse.
pub fn milnor_to_adem_on_basis(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, 
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    if milnor_algebra.generic {
        milnor_to_adem_on_basis_generic(adem_algebra, milnor_algebra, result, coeff, degree, idx);
    } else {
        milnor_to_adem_on_basis_2(adem_algebra, milnor_algebra, result, coeff, degree, idx);
    }
}

fn milnor_to_adem_on_basis_2(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    let elt = milnor_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.get_dimension(elt.degree, -1);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let mut t = vec![0;elt.p_part.len()];
    t[elt.p_part.len() - 1] = elt.p_part[elt.p_part.len() - 1];
    for i in (0 .. elt.p_part.len() - 1).rev() {
        t[i] = elt.p_part[i] + 2 * t[i + 1];
    }
    let t_idx = adem_algebra.basis_element_to_index(&AdemBasisElement {
        degree,
        excess : 0,
        bocksteins : 0,
        ps : t
    });
    let mut tmp_vector_a = FpVector::new(p, dim);
    adem_to_milnor_on_basis(adem_algebra, milnor_algebra, &mut tmp_vector_a, 1, degree, t_idx);
    assert!(tmp_vector_a.get_entry(idx) == 1);
    tmp_vector_a.set_entry(idx, 0);
    milnor_to_adem(adem_algebra, milnor_algebra, result, coeff, degree, &tmp_vector_a);
    result.add_basis_element(t_idx, coeff);
}


fn milnor_to_adem_on_basis_generic(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, idx : usize
){
    let elt = milnor_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.get_dimension(elt.degree, -1);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let t_len = std::cmp::max(elt.p_part.len(), (31u32.saturating_sub(elt.q_part.leading_zeros())) as usize);
    let mut t = vec![0;t_len];
    let last_p_part = if t_len <= elt.p_part.len() { elt.p_part[t_len - 1] } else { 0 }; 
    t[t_len - 1] = last_p_part + ((elt.q_part >> (t_len)) & 1);
    for i in (0 .. t_len - 1).rev() {
        let p_part = if i < elt.p_part.len() { elt.p_part[i] } else { 0 };
        t[i] = p_part + ((elt.q_part >> (i + 1)) & 1) + p * t[i + 1];
    }
    let t_idx = adem_algebra.basis_element_to_index(&AdemBasisElement {
        degree,
        excess : 0,
        bocksteins : elt.q_part,
        ps : t
    });
    let mut tmp_vector_a = FpVector::new(p, dim);
    adem_to_milnor_on_basis(adem_algebra, milnor_algebra, &mut tmp_vector_a, 1, degree, t_idx);
    assert!(tmp_vector_a.get_entry(idx) == 1);
    tmp_vector_a.set_entry(idx, 0);
    tmp_vector_a.scale(p - 1);
    milnor_to_adem(adem_algebra, milnor_algebra, result, coeff, degree, &tmp_vector_a);
    result.add_basis_element(t_idx, coeff);
}


fn milnor_to_adem(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, input : &FpVector
){
    let p = milnor_algebra.prime();
    for (i, v) in input.iter().enumerate() {
        if v == 0 {
            continue;
        }
        milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, (coeff * v) % p, degree, i);
    }
}

pub fn get_adem_q(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, qi : u32
){
    let p = adem_algebra.prime();
    let degree = crate::combinatorics::get_tau_degrees(p)[qi as usize];
    let mbe;
    if adem_algebra.generic {
        mbe = MilnorBasisElement {
            degree,
            q_part : 1 << qi, 
            p_part : vec![]
        };
    } else {
        let mut p_part = vec![0; qi as usize + 1];
        p_part[qi as usize] = 1;
        mbe = MilnorBasisElement {
            degree,
            q_part: 0,
            p_part
        };
    }
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, coeff, degree, idx);
}

pub fn get_adem_plist(
    adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra,
    result : &mut FpVector, coeff : u32, degree : i32, p_part : Vec<u32>
){
    let mbe = MilnorBasisElement {
        degree,
        p_part,
        q_part : 0
    };
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, coeff, degree, idx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest_parametrize;
    
    #[test]
    fn test_cob_milnor_qs_to_adem(){
        let p = 2;
        let max_degree = 16;
        let adem = AdemAlgebra::new(p, p != 2, false);
        let milnor = MilnorAlgebra::new(p);//, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        for (qi, output) in vec![
            (0, "P1"),
            (1, "P3 + P2 P1"),
            (2, "P7 + P5 P2 + P6 P1 + P4 P2 P1")
        ] {
            let degree = (1 << (qi + 1)) - 1;
            let mut result = FpVector::new(p, adem.get_dimension(degree, -1));
            get_adem_q(&adem, &milnor, &mut result, 1, qi);
            println!("Q{} ==> {}", qi, adem.element_to_string(degree, &result));
            assert_eq!(adem.element_to_string(degree, &result), output)
        }
    }

    #[allow(non_snake_case)]
    #[rstest_parametrize(p, max_degree,
        case(2, 32),
        case(3, 60)//106 // reduced size of test because we use a slow implementation
    )]    
   fn test_cob_adem_to_milnor(p : u32, max_degree : i32){
        let adem = AdemAlgebra::new(p, p != 2, false);
        let milnor = MilnorAlgebra::new(p);//, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        
        for degree in 0 .. max_degree {
            println!("degree : {}", degree);
            let dim = adem.get_dimension(degree, -1);
            let mut milnor_result = FpVector::new(p, dim);
            let mut adem_result = FpVector::new(p, dim);
            for i in 0 .. dim {
                // println!("i : {}", i);
                milnor_to_adem_on_basis(&adem, &milnor, &mut adem_result, 1, degree, i);
                adem_to_milnor(&adem, &milnor, &mut milnor_result, 1, degree, &adem_result);
                assert!(milnor_result.get_entry(i) == 1, 
                    format!("{} ==> {} ==> {}", 
                        milnor.basis_element_to_string(degree, i),
                        adem.element_to_string(degree, &adem_result),
                        milnor.element_to_string(degree, &milnor_result)
                ));
                milnor_result.set_entry(i, 0);
                assert!(milnor_result.is_zero(),
                    format!("{} ==> {} ==> {}", 
                        milnor.basis_element_to_string(degree, i),
                        adem.element_to_string(degree, &adem_result),
                        milnor.element_to_string(degree, &milnor_result)
                ));
                println!("    {} ==> {}", milnor.basis_element_to_string(degree,i), adem.element_to_string(degree, &adem_result));
                adem_result.set_to_zero();
                milnor_result.set_to_zero();
            }
        }

    }

}
