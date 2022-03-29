use crate::algebra::adem_algebra::AdemBasisElement;
use crate::algebra::milnor_algebra::{MilnorBasisElement, PPart, PPartEntry};
use crate::algebra::{AdemAlgebra, Algebra, MilnorAlgebra};
use fp::vector::FpVector;

/// Translate from the adem basis to the milnor basis, adding `coeff` times the result to `result`.
/// This uses the fact that that $P^n = P(n)$ and $Q_1 = \beta$ and multiplies out the admissible
/// monomial.
pub fn adem_to_milnor_on_basis(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    idx: usize,
) {
    let elt = adem_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.dimension(elt.degree);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let mut tmp_vector_a = FpVector::new(p, 1);
    let mut tmp_vector_b = FpVector::new(p, 0);

    tmp_vector_a.set_entry(0, 1);

    let mut bocksteins = elt.bocksteins;
    let mut total_degree = 0;

    for &sqn in &elt.ps {
        let (deg, idx) = milnor_algebra.beps_pn(bocksteins & 1, sqn as PPartEntry);
        bocksteins >>= 1;

        tmp_vector_b.set_scratch_vector_size(milnor_algebra.dimension(total_degree + deg));
        milnor_algebra.multiply_element_by_basis_element(
            tmp_vector_b.as_slice_mut(),
            1,
            total_degree,
            tmp_vector_a.as_slice(),
            deg,
            idx,
        );
        total_degree += deg;
        std::mem::swap(&mut tmp_vector_a, &mut tmp_vector_b);
    }
    if bocksteins & 1 == 0 {
        result.add(&tmp_vector_a, coeff);
    } else {
        milnor_algebra.multiply_element_by_basis_element(
            result.as_slice_mut(),
            coeff,
            total_degree,
            tmp_vector_a.as_slice(),
            1,
            0,
        );
    }
}

pub fn adem_to_milnor(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    input: &FpVector,
) {
    let p = milnor_algebra.prime();
    for (i, v) in input.iter_nonzero() {
        adem_to_milnor_on_basis(
            adem_algebra,
            milnor_algebra,
            result,
            (coeff * v) % *p,
            degree,
            i,
        );
    }
}

// This is currently pretty inefficient... We should memoize results so that we don't repeatedly
// recompute the same inverse.
pub fn milnor_to_adem_on_basis(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    idx: usize,
) {
    if milnor_algebra.generic() {
        milnor_to_adem_on_basis_generic(adem_algebra, milnor_algebra, result, coeff, degree, idx);
    } else {
        milnor_to_adem_on_basis_2(adem_algebra, milnor_algebra, result, coeff, degree, idx);
    }
}

fn milnor_to_adem_on_basis_2(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    idx: usize,
) {
    let elt = milnor_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.dimension(elt.degree);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let mut t: Vec<u32> = vec![0; elt.p_part.len()];
    t[elt.p_part.len() - 1] = elt.p_part[elt.p_part.len() - 1] as u32;
    for i in (0..elt.p_part.len() - 1).rev() {
        t[i] = elt.p_part[i] as u32 + 2 * t[i + 1];
    }
    let t_idx = adem_algebra.basis_element_to_index(&AdemBasisElement {
        degree,
        excess: 0,
        bocksteins: 0,
        ps: t,
        p_or_sq: *adem_algebra.prime() != 2,
    });
    let mut tmp_vector_a = FpVector::new(p, dim);
    adem_to_milnor_on_basis(
        adem_algebra,
        milnor_algebra,
        &mut tmp_vector_a,
        1,
        degree,
        t_idx,
    );
    assert!(tmp_vector_a.entry(idx) == 1);
    tmp_vector_a.set_entry(idx, 0);
    milnor_to_adem(
        adem_algebra,
        milnor_algebra,
        result,
        coeff,
        degree,
        &tmp_vector_a,
    );
    result.add_basis_element(t_idx, coeff);
}

fn milnor_to_adem_on_basis_generic(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    idx: usize,
) {
    let elt = milnor_algebra.basis_element_from_index(degree, idx);
    let p = milnor_algebra.prime();
    let dim = milnor_algebra.dimension(elt.degree);
    if dim == 1 {
        result.set_entry(0, coeff);
        return;
    }
    let t_len = std::cmp::max(
        elt.p_part.len(),
        (31u32.saturating_sub(elt.q_part.leading_zeros())) as usize,
    );
    let mut t = vec![0; t_len];
    let last_p_part = if t_len <= elt.p_part.len() {
        elt.p_part[t_len - 1] as u32
    } else {
        0
    };
    t[t_len - 1] = last_p_part + ((elt.q_part >> (t_len)) & 1);
    for i in (0..t_len - 1).rev() {
        let p_part = if i < elt.p_part.len() {
            elt.p_part[i] as u32
        } else {
            0
        };
        t[i] = p_part + ((elt.q_part >> (i + 1)) & 1) + *p * t[i + 1];
    }
    let t_idx = adem_algebra.basis_element_to_index(&AdemBasisElement {
        degree,
        excess: 0,
        bocksteins: elt.q_part,
        ps: t,
        p_or_sq: *adem_algebra.prime() != 2,
    });
    let mut tmp_vector_a = FpVector::new(p, dim);
    adem_to_milnor_on_basis(
        adem_algebra,
        milnor_algebra,
        &mut tmp_vector_a,
        1,
        degree,
        t_idx,
    );
    assert!(tmp_vector_a.entry(idx) == 1);
    tmp_vector_a.set_entry(idx, 0);
    tmp_vector_a.scale(*p - 1);
    milnor_to_adem(
        adem_algebra,
        milnor_algebra,
        result,
        coeff,
        degree,
        &tmp_vector_a,
    );
    result.add_basis_element(t_idx, coeff);
}

pub fn milnor_to_adem(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    input: &FpVector,
) {
    let p = milnor_algebra.prime();
    for (i, v) in input.iter_nonzero() {
        milnor_to_adem_on_basis(
            adem_algebra,
            milnor_algebra,
            result,
            (coeff * v) % *p,
            degree,
            i,
        );
    }
}

/// Express $Q_{qi}$ in the adem basis.
pub fn adem_q(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    qi: u32,
) {
    let p = adem_algebra.prime();
    let degree = crate::algebra::combinatorics::tau_degrees(p)[qi as usize];
    let mbe = if adem_algebra.generic {
        MilnorBasisElement {
            degree,
            q_part: 1 << qi,
            p_part: vec![],
        }
    } else {
        let mut p_part = vec![0; qi as usize + 1];
        p_part[qi as usize] = 1;
        MilnorBasisElement {
            degree,
            q_part: 0,
            p_part,
        }
    };
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, coeff, degree, idx);
}

/// Express P(...) in the Adem basis.
pub fn adem_plist(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    result: &mut FpVector,
    coeff: u32,
    degree: i32,
    p_part: PPart,
) {
    let mbe = MilnorBasisElement {
        degree,
        p_part,
        q_part: 0,
    };
    let idx = milnor_algebra.basis_element_to_index(&mbe);
    milnor_to_adem_on_basis(adem_algebra, milnor_algebra, result, coeff, degree, idx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp::prime::ValidPrime;
    use rstest::rstest;

    #[test]
    fn test_cob_milnor_qs_to_adem() {
        let p = ValidPrime::new(2);
        let max_degree = 16;
        let adem = AdemAlgebra::new(p, *p != 2, false, false);
        let milnor = MilnorAlgebra::new(p);
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);
        for (qi, output) in &[
            (0, "Sq1"),
            (1, "Sq3 + Sq2 Sq1"),
            (2, "Sq7 + Sq5 Sq2 + Sq6 Sq1 + Sq4 Sq2 Sq1"),
        ] {
            let degree = (1 << (*qi + 1)) - 1;
            let mut result = FpVector::new(p, adem.dimension(degree));
            adem_q(&adem, &milnor, &mut result, 1, *qi);
            println!(
                "Q{} ==> {}",
                qi,
                adem.element_to_string(degree, result.as_slice())
            );
            assert_eq!(adem.element_to_string(degree, result.as_slice()), *output)
        }
    }

    #[allow(non_snake_case)]
    #[rstest(p, max_degree,
        case(2, 32),
        case(3, 60)//106 // reduced size of test because we use a slow implementation
    )]
    #[trace]
    fn test_cob_adem_to_milnor(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let adem = AdemAlgebra::new(p, *p != 2, false, false);
        let milnor = MilnorAlgebra::new(p); //, p != 2
        adem.compute_basis(max_degree);
        milnor.compute_basis(max_degree);

        for degree in 0..max_degree {
            println!("degree : {}", degree);
            let dim = adem.dimension(degree);
            let mut milnor_result = FpVector::new(p, dim);
            let mut adem_result = FpVector::new(p, dim);
            for i in 0..dim {
                // println!("i : {}", i);
                milnor_to_adem_on_basis(&adem, &milnor, &mut adem_result, 1, degree, i);
                adem_to_milnor(&adem, &milnor, &mut milnor_result, 1, degree, &adem_result);
                assert!(
                    milnor_result.entry(i) == 1,
                    "{} ==> {} ==> {}",
                    milnor.basis_element_to_string(degree, i),
                    adem.element_to_string(degree, adem_result.as_slice()),
                    milnor.element_to_string(degree, milnor_result.as_slice())
                );
                milnor_result.set_entry(i, 0);
                assert!(
                    milnor_result.is_zero(),
                    "{} ==> {} ==> {}",
                    milnor.basis_element_to_string(degree, i),
                    adem.element_to_string(degree, adem_result.as_slice()),
                    milnor.element_to_string(degree, milnor_result.as_slice())
                );
                println!(
                    "    {} ==> {}",
                    milnor.basis_element_to_string(degree, i),
                    adem.element_to_string(degree, adem_result.as_slice())
                );
                adem_result.set_to_zero();
                milnor_result.set_to_zero();
            }
        }
    }
}
