//! This implements the notion of a split pair algebra in the sense of
//! <https://arxiv.org/abs/2105.07628v1>, whose notation we will use throughout.
//!
//! The Steenrod algebra admits a lift to a split pair algebra, which can be used to compute d2
//! differentials algorithmically.
//!
//! To keep the pair algebra business contained, we put the implementation of the Milnor algebra as
//! a pair algebra in this file instead of `milnor_algebra.rs`.

use fp::{
    prime::TWO,
    vector::{FpSlice, FpSliceMut, FpVector},
};
use rustc_hash::FxHasher;

use crate::{combinatorics, Algebra};

type HashMap<K, V> = hashbrown::HashMap<K, V, std::hash::BuildHasherDefault<FxHasher>>;

use std::io;

/// A lift of an algebra to a split pair algebra. See module introduction for more.
pub trait PairAlgebra: Algebra {
    /// An element in the cohomological degree zero part of the pair algebra. This tends to not be
    /// a ring over Fp, so we let the algebra specify how it wants to represent the elements.
    type Element: Send + Sync;

    fn element_is_zero(elt: &Self::Element) -> bool;

    /// Assert that `elt` is in the image of the differential. Drop the data recording the
    /// complement of the image of the differential.
    fn finalize_element(_elt: &mut Self::Element) {}

    /// Create a new zero element in the given degree.
    fn new_pair_element(&self, degree: i32) -> Self::Element;

    /// Given $r, s \in \pi_0(A)$, compute $\sigma(r) \sigma(s)$ and add the result to
    /// `result`.
    fn sigma_multiply_basis(
        &self,
        result: &mut Self::Element,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    );

    /// Same as [`PairAlgebra::sigma_multiply_basis`] but with non-basis elements.
    fn sigma_multiply(
        &self,
        result: &mut Self::Element,
        coeff: u32,
        r_degree: i32,
        r: FpSlice,
        s_degree: i32,
        s: FpSlice,
    ) {
        if coeff == 0 {
            return;
        }
        for (r_idx, c) in r.iter_nonzero() {
            for (s_idx, d) in s.iter_nonzero() {
                self.sigma_multiply_basis(result, c * d * coeff, r_degree, r_idx, s_degree, s_idx);
            }
        }
    }

    /// Compute $A(r, s)$ and write the result to `result`.
    fn a_multiply(
        &self,
        result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r: FpSlice,
        s_degree: i32,
        s: &Self::Element,
    );

    /// The element p is classified by a filtration on element in Ext of the underlying algebra,
    /// which is represented by an indecomposable in degree 1. This returns the index of said
    /// indecomposable.
    fn p_tilde(&self) -> usize;

    fn element_to_bytes(&self, elt: &Self::Element, buffer: &mut impl io::Write) -> io::Result<()>;
    fn element_from_bytes(
        &self,
        degree: i32,
        buffer: &mut impl io::Read,
    ) -> io::Result<Self::Element>;
}

use std::cell::RefCell;

use crate::{
    milnor_algebra::{MilnorBasisElement as MilnorElt, PPartAllocation, PPartMultiplier},
    MilnorAlgebra,
};

macro_rules! sub {
    ($elt:ident, $k:expr, $n:expr) => {
        if $k > 0 {
            if $elt.p_part[$k - 1] < (1 << $n) {
                continue;
            }
            $elt.p_part[$k - 1] -= 1 << $n;
            $elt.degree -= combinatorics::xi_degrees(TWO)[$k - 1] * (1 << $n);
        }
    };
}
macro_rules! unsub {
    ($elt:ident, $k:expr, $n:expr) => {
        if $k > 0 {
            $elt.p_part[$k - 1] += 1 << $n;
            $elt.degree += combinatorics::xi_degrees(TWO)[$k - 1] * (1 << $n);
        }
    };
}

pub struct MilnorPairElement {
    ones: FpVector,
    twos: FpVector,
    #[cfg(debug_assertions)]
    degree: i32,
    ys: Vec<Vec<FpVector>>,
}

impl PairAlgebra for MilnorAlgebra {
    type Element = MilnorPairElement;

    fn new_pair_element(&self, degree: i32) -> Self::Element {
        let p = self.prime();
        assert_eq!(p, TWO);

        let max_k = if degree == 0 {
            0
        } else {
            fp::prime::log2(degree as usize) + 1
        };
        let mut ys = Vec::with_capacity(max_k);
        for k in 0..max_k {
            let rem_degree = degree as usize + 1 - (1 << k);
            let max_l = fp::prime::log2(rem_degree) + 1;
            let mut row = Vec::with_capacity(max_l);
            for l in 0..max_l {
                row.push(FpVector::new(
                    p,
                    self.dimension((rem_degree - (1 << l)) as i32),
                ));
            }
            ys.push(row);
        }

        MilnorPairElement {
            ones: FpVector::new(p, self.dimension(degree)),
            twos: FpVector::new(p, self.dimension(degree)),
            ys,
            #[cfg(debug_assertions)]
            degree,
        }
    }

    fn element_is_zero(elt: &Self::Element) -> bool {
        elt.ones.is_zero()
            && elt.twos.is_zero()
            && elt.ys.iter().all(|v| v.iter().all(|x| x.is_zero()))
    }

    fn p_tilde(&self) -> usize {
        0
    }

    /// Assert that `elt` is in the image of the differential. Drop the data recording the
    /// complement of the image of the differential.
    fn finalize_element(elt: &mut Self::Element) {
        assert!(elt.ones.is_zero());
        elt.ones = FpVector::new(elt.twos.prime(), 0);
    }

    fn sigma_multiply_basis(
        &self,
        result: &mut Self::Element,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
    ) {
        #[cfg(debug_assertions)]
        assert_eq!(r_degree + s_degree, result.degree);

        // First write the Y terms
        let mut r = self.basis_element_from_index(r_degree, r_idx).clone();
        let mut s = self.basis_element_from_index(s_degree, s_idx).clone();

        PPartAllocation::with_local(|mut allocation| {
            for k in 0..s.p_part.len() {
                sub!(s, k + 1, 0);
                for n in 1..r.p_part.len() + 1 {
                    sub!(r, n, k);
                    for m in 0..n {
                        sub!(r, m, k);
                        allocation = self.multiply_with_allocation(
                            result.ys[m + k][n + k].as_slice_mut(),
                            coeff,
                            &r,
                            &s,
                            i32::MAX,
                            allocation,
                        );
                        unsub!(r, m, k);
                    }
                    unsub!(r, n, k);
                }
                unsub!(s, k + 1, 0);
            }

            // Now the product terms
            let mut multiplier = PPartMultiplier::<true>::new_from_allocation(
                TWO,
                &r.p_part,
                &s.p_part,
                allocation,
                0,
                r.degree + s.degree,
            );

            // coeff should always be 1, so no need to optimize the even case.
            while let Some(c) = multiplier.next() {
                let idx = self.basis_element_to_index(&multiplier.ans);
                let c = c * coeff;
                // TODO: optimize
                if c == 2 {
                    result.twos.add_basis_element(idx, 1);
                } else {
                    // c = 1 or 3
                    let existing = result.ones.entry(idx);
                    let c = c + existing;
                    result.ones.set_entry(idx, c & 1);
                    result.twos.add_basis_element(idx, (c >> 1) & 1);
                }
            }
            multiplier.into_allocation()
        });
    }

    fn a_multiply(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        r_degree: i32,
        r: FpSlice<'_>,
        s_degree: i32,
        s: &Self::Element,
    ) {
        assert!(s.ones.is_zero());

        if r_degree == 0 {
            return;
        }

        // The twos terms
        for (r_idx, c) in r.iter_nonzero() {
            let mut r = self.basis_element_from_index(r_degree, r_idx).clone();
            sub!(r, 1, 0);
            self.multiply_basis_by_element(
                result.copy(),
                coeff * c,
                &r,
                s_degree,
                s.twos.as_slice(),
            );
            unsub!(r, 1, 0);
        }

        // The Y terms
        for (k, row) in s.ys.iter().enumerate() {
            for (l, vec) in row.iter().enumerate() {
                if vec.is_zero() {
                    continue;
                }
                let degree_shift = (1 << k) + (1 << l) - 1;
                for (r_idx, c) in r.iter_nonzero() {
                    let r = self.basis_element_from_index(r_degree, r_idx);
                    let a_degree = r_degree + degree_shift - 1;
                    a_y_cached(self, r, k, l, |v| {
                        self.multiply_element_by_element(
                            result.copy(),
                            coeff * c,
                            a_degree,
                            v.as_slice(),
                            s_degree - degree_shift,
                            vec.as_slice(),
                        )
                    });
                }
            }
        }
    }

    fn element_to_bytes(&self, elt: &Self::Element, buffer: &mut impl io::Write) -> io::Result<()> {
        elt.twos.to_bytes(buffer)?;
        for row in &elt.ys {
            for v in row {
                v.to_bytes(buffer)?;
            }
        }
        Ok(())
    }

    fn element_from_bytes(
        &self,
        degree: i32,
        buffer: &mut impl io::Read,
    ) -> io::Result<Self::Element> {
        let p = self.prime();
        assert_eq!(p, TWO);

        let max_k = if degree == 0 {
            0
        } else {
            fp::prime::log2(degree as usize) + 1
        };

        let twos = FpVector::from_bytes(p, self.dimension(degree), buffer)?;

        let mut ys = Vec::with_capacity(max_k);
        for k in 0..max_k {
            let rem_degree = degree as usize + 1 - (1 << k);
            let max_l = fp::prime::log2(rem_degree) + 1;
            let mut row = Vec::with_capacity(max_l);
            for l in 0..max_l {
                row.push(FpVector::from_bytes(
                    p,
                    self.dimension((rem_degree - (1 << l)) as i32),
                    buffer,
                )?);
            }
            ys.push(row);
        }

        Ok(MilnorPairElement {
            ones: FpVector::new(p, 0),
            twos,
            ys,
            #[cfg(debug_assertions)]
            degree,
        })
    }
}

// Use thread-local storage to memoize a_y computation. Since the possible values of k, l grow as
// log n, in practice it is going to be at most, say, 64, and the memory usage here should be
// dwarfed by that of storing a single quasi-inverse

use std::hash::{BuildHasher, Hash, Hasher};

thread_local! {
    static AY_CACHE: RefCell<HashMap<(MilnorElt, (usize, usize)), FpVector>> = RefCell::new(HashMap::default());
}

/// Compute $A(Sq(R), Y_{k, l})$ where $a = Sq(R)$. This queries the cache and computes it using
/// [`a_y_inner`] if not available.
fn a_y_cached(
    algebra: &MilnorAlgebra,
    a: &MilnorElt,
    k: usize,
    l: usize,
    f: impl FnOnce(&FpVector),
) {
    AY_CACHE.with(|cache| {
        let cache = &mut *cache.try_borrow_mut().unwrap();
        let mut hasher = cache.hasher().build_hasher();
        a.hash(&mut hasher);
        (k, l).hash(&mut hasher);

        let raw_entry = cache.raw_entry();
        let result = raw_entry
            .from_hash(hasher.finish(), |v| &v.0 == a && v.1 == (k, l))
            .map(|(_, y)| y);

        match result {
            Some(v) => f(v),
            None => {
                let v = a_y_inner(algebra, a, k, l);
                f(&v);
                cache.insert((a.clone(), (k, l)), v);
            }
        }
    })
}

/// Actually computes $A(a, Y_{k, l})$ and returns the result.
fn a_y_inner(algebra: &MilnorAlgebra, a: &MilnorElt, k: usize, l: usize) -> FpVector {
    let mut a = a.clone();
    let mut result = FpVector::new(TWO, algebra.dimension(a.degree + (1 << k) + (1 << l) - 2));
    let mut t = MilnorElt {
        q_part: 0,
        p_part: vec![],
        degree: 0,
    };

    for i in 0..=a.p_part.len() {
        if i + k < l {
            continue;
        }

        sub!(a, i, k);
        for j in 0..=std::cmp::min(i + k - l, a.p_part.len()) {
            sub!(a, j, l);

            t.p_part.clear();
            t.p_part.resize(k + i, 0);

            t.p_part[k + i - 1] += 1;
            t.p_part[l + j - 1] += 1;

            t.degree = (1 << (k + i)) + (1 << (l + j)) - 2;

            // We can just read off the value of the product instead of passing through the
            // algorithm, but this is cached so problem for another day...
            algebra.multiply(result.as_slice_mut(), 1, &t, &a);

            unsub!(a, j, l);
        }
        unsub!(a, i, k);
    }
    result
}

#[cfg(test)]
mod tests {
    use expect_test::{expect, Expect};

    use super::*;
    use crate::milnor_algebra::PPartEntry;

    fn from_p_part(p_part: &[PPartEntry]) -> MilnorElt {
        let degree = p_part
            .iter()
            .enumerate()
            .map(|(i, &n)| combinatorics::xi_degrees(TWO)[i] * (n as i32))
            .sum();

        MilnorElt {
            q_part: 0,
            p_part: p_part.into(),
            degree,
        }
    }

    #[test]
    fn test_a_y() {
        let algebra = MilnorAlgebra::new(TWO, false);

        let mut result = FpVector::new(TWO, 0);

        let mut check = |p_part: &[PPartEntry], k, l, ans: Expect| {
            let a = from_p_part(p_part);

            let target_deg = a.degree + (1 << k) + (1 << l) - 2;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg));
            a_y_cached(&algebra, &a, k, l, |v| result.add(v, 1));
            ans.assert_eq(&algebra.element_to_string(target_deg, result.as_slice()));
        };

        check(&[1], 0, 1, expect![["P(2)"]]);
        check(&[1], 1, 2, expect![["0"]]);
        check(&[0, 1], 0, 1, expect![["P(1, 1)"]]);
        check(&[0, 2], 1, 3, expect![["P(0, 0, 2)"]]);
        check(&[1, 2], 0, 1, expect![["P(2, 2)"]]);
    }
}
