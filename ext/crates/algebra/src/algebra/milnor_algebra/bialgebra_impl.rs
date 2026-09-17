use super::{MilnorAlgebra, MilnorBasisElement, PPart};
use crate::algebra::{Algebra, Bialgebra, combinatorics};

impl MilnorAlgebra {
    /// Advance `element` to the next p-part bounded entrywise by `max`, in odometer order.
    ///
    /// Returns `true` once the odometer wraps, i.e. when `element` was already `max`.
    ///
    /// This carries *before* incrementing rather than after. The two orders enumerate the same
    /// sequence, but incrementing first would transiently store `max[i] + 1`, which need not fit
    /// in a packed field whose width is exactly saturated by `max[i]`.
    fn increment_p_part(element: &mut PPart, max: PPart) -> bool {
        for i in 0..max.len() {
            let entry = element.get(i);
            if entry < max.get(i) {
                element.set(i, entry + 1);
                return false;
            }
            element.set(i, 0);
        }
        true
    }
}

impl Bialgebra for MilnorAlgebra {
    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        assert_eq!(self.prime(), 2, "Coproduct at odd primes not supported");
        if op_deg == 0 {
            return vec![(0, 0, 0, 0)];
        }
        let xi_degrees = combinatorics::xi_degrees(self.prime());

        let p_part = self.basis_element_from_index(op_deg, op_idx).p_part;

        let mut len: usize = 1;
        for i in p_part.iter() {
            len *= i as usize + 1;
        }
        let mut result = Vec::with_capacity(len);

        let n = p_part.len();
        let mut cur_ppart = PPart::zero();
        loop {
            let mut left_degree: i32 = 0;
            let mut right_ppart = PPart::zero();
            for (i, &xi_degree) in xi_degrees.iter().enumerate().take(n) {
                let entry = cur_ppart.get(i);
                left_degree += entry as i32 * xi_degree;
                right_ppart.set(i, p_part.get(i) - entry);
            }
            let right_degree: i32 = op_deg - left_degree;

            let left_idx = self.basis_element_to_index(&MilnorBasisElement {
                degree: left_degree,
                q_part: 0,
                p_part: cur_ppart,
            });
            let right_idx = self.basis_element_to_index(&MilnorBasisElement {
                degree: right_degree,
                q_part: 0,
                p_part: right_ppart,
            });

            result.push((left_degree, left_idx, right_degree, right_idx));
            if Self::increment_p_part(&mut cur_ppart, p_part) {
                break;
            }
        }
        result
    }

    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)> {
        vec![(op_deg, op_idx)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `increment_p_part` walks up to and including `max`, whose top entry may saturate its field.
    /// Incrementing before carrying would overflow there.
    #[test]
    fn ppart_odometer_handles_saturated_field() {
        let top = PPart::MAX_LEN - 1;
        let mut max = PPart::from_slice(&[2]);
        max.set(top, PPart::max_entry(top));

        let mut count = 0;
        let mut cur = PPart::zero();
        loop {
            count += 1;
            if MilnorAlgebra::increment_p_part(&mut cur, max) {
                break;
            }
        }
        assert_eq!(count, 3 * (PPart::max_entry(top) as usize + 1));
        // Wrapping leaves the odometer back at zero.
        assert_eq!(cur, PPart::zero());
    }
}
