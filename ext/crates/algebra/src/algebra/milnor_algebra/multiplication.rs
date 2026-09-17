use std::cell::Cell;

use fp::{
    prime::{Binomial, Prime, ValidPrime, iter::BitflagIterator},
    vector::{FpSlice, FpSliceMut},
};

use super::{MilnorAlgebra, MilnorBasisElement, PPart, PPartEntry};
use crate::algebra::{Algebra, UnstableAlgebra};

// Multiplication logic
impl MilnorAlgebra {
    /// Return the degree and index of $Q_1^e P(x)$, or `None` if the element is not present
    /// (e.g. out of range or excluded by the profile).
    pub fn try_beps_pn(&self, e: u32, x: PPartEntry) -> Option<(i32, usize)> {
        // `x` and `e` are unbounded, so compute the degree in `i64` and only narrow it once the
        // bound has ruled the wide cases out.
        let degree = self.q() as i64 * x as i64 + e as i64;
        if degree > PPart::MAX_DEGREE as i64 || x > PPart::max_entry(0) {
            return None;
        }
        let degree = degree as i32;
        self.compute_basis(degree);
        self.try_basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part: e,
            p_part: PPart::from_iter([x]),
        })
        .map(|index| (degree, index))
    }

    /// Return the degree and index of $Q_1^e P(x)$.
    pub fn beps_pn(&self, e: u32, x: PPartEntry) -> (i32, usize) {
        self.try_beps_pn(e, x).unwrap()
    }

    fn multiply_qpart(&self, m1: MilnorBasisElement, f: u32) -> Vec<(u32, MilnorBasisElement)> {
        let mut new_result: Vec<(u32, MilnorBasisElement)> = vec![(1, m1)];
        let mut old_result: Vec<(u32, MilnorBasisElement)> = Vec::new();

        for k in BitflagIterator::set_bit_iterator(f as u64) {
            let k = k as u32;
            let pk = self.p.pow(k);
            std::mem::swap(&mut new_result, &mut old_result);
            new_result.clear();

            // We implement the formula
            // P(R) Q_k = Q_k P^R + Q_{k+1} P(R - p^k e_1) + Q_{k+2} P(R - p^k e_2) +
            // ... + Q_{k + i} P(R - p^k e_i) + ...
            // where e_i is the vector with value 1 in entry i and 0 otherwise (in the above
            // formula, the first xi is xi_1, hence the offset below). If R - p^k e_i has a
            // negative entry, the term is 0.
            //
            // We also use the fact that Q_k Q_j = -Q_j Q_k
            for (coef, term) in &old_result {
                for i in 0..=term.p_part.len() {
                    // If there is already Q_{k+i} on the other side, the result is 0
                    if term.q_part & (1 << (k + i as u32)) != 0 {
                        continue;
                    }
                    let mut new_p = term.p_part;
                    if i > 0 {
                        // Check if R - p^k e_i < 0. Only do this from the first term onwards.
                        let entry = new_p.get(i - 1);
                        if entry < pk {
                            continue;
                        }
                        new_p.set(i - 1, entry - pk);
                    }

                    // Now calculate the number of Q's we are moving past
                    let larger_q = (term.q_part >> (k + i as u32 + 1)).count_ones();

                    // Now put everything together
                    let m = MilnorBasisElement {
                        p_part: new_p,
                        q_part: term.q_part | (1 << (k + i as u32)),
                        degree: 0, // we don't really care about the degree here. The final degree of the whole calculation is known a priori
                    };
                    let c = if larger_q.is_multiple_of(2) {
                        *coef
                    } else {
                        *coef * (self.prime() - 1)
                    };

                    new_result.push((c, m));
                }
            }
        }
        new_result
    }

    pub fn multiply(
        &self,
        res: FpSliceMut,
        coef: u32,
        m1: MilnorBasisElement,
        m2: MilnorBasisElement,
    ) {
        PPartAllocation::with_local(|allocation| {
            self.multiply_with_allocation(res, coef, m1, m2, i32::MAX, allocation)
        });
    }

    pub fn multiply_with_allocation(
        &self,
        mut res: FpSliceMut,
        coef: u32,
        m1: MilnorBasisElement,
        m2: MilnorBasisElement,
        excess: i32,
        mut allocation: PPartAllocation,
    ) -> PPartAllocation {
        let target_deg = m1.degree + m2.degree;
        if self.generic() {
            let m1f = self.multiply_qpart(m1, m2.q_part);
            for (cc, basis) in m1f {
                let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
                    self.prime(),
                    basis.p_part,
                    m2.p_part,
                    allocation,
                    basis.q_part,
                    target_deg,
                );

                while let Some(c) = multiplier.next() {
                    let idx = self.basis_element_to_index(&multiplier.ans);
                    if idx < self.dimension_unstable(target_deg, excess) {
                        res.add_basis_element(idx, c * cc * coef);
                    }
                }
                allocation = multiplier.into_allocation()
            }
        } else {
            let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
                self.prime(),
                m1.p_part,
                m2.p_part,
                allocation,
                0,
                target_deg,
            );

            while let Some(c) = multiplier.next() {
                let idx = self.basis_element_to_index(&multiplier.ans);
                if idx < self.dimension_unstable(target_deg, excess) {
                    res.add_basis_element(idx, c * coef);
                }
            }
            allocation = multiplier.into_allocation()
        }
        allocation
    }

    pub fn multiply_basis_by_element(
        &self,
        res: FpSliceMut,
        coef: u32,
        m1: MilnorBasisElement,
        s_deg: i32,
        s: FpSlice,
    ) {
        PPartAllocation::with_local(|allocation| {
            self.multiply_basis_by_element_with_allocation(res, coef, m1, s_deg, s, allocation)
        });
    }

    pub(super) fn multiply_basis_by_element_with_allocation(
        &self,
        mut res: FpSliceMut,
        coef: u32,
        m1: MilnorBasisElement,
        s_deg: i32,
        s: FpSlice,
        mut allocation: PPartAllocation,
    ) -> PPartAllocation {
        for (i, c) in s.iter_nonzero() {
            allocation = self.multiply_with_allocation(
                res.copy(),
                coef * c,
                m1,
                self.basis_element_from_index(s_deg, i),
                i32::MAX,
                allocation,
            );
        }
        allocation
    }
}

#[derive(Debug, Default)]
struct Matrix2D {
    cols: usize,
    inner: Vec<PPartEntry>,
}

impl std::fmt::Display for Matrix2D {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for i in 0..self.inner.len() / self.cols {
            writeln!(f, "{:?}", &self[i][0..self.cols])?;
        }
        Ok(())
    }
}

impl Matrix2D {
    fn reset(&mut self, rows: usize, cols: usize) {
        self.cols = cols;
        self.inner.clear();
        self.inner.resize(rows * cols, 0);
    }
}

impl Matrix2D {
    fn with_capacity(rows: usize, cols: usize) -> Self {
        Self {
            cols: 0,
            inner: Vec::with_capacity(rows * cols),
        }
    }
}

impl std::ops::Index<usize> for Matrix2D {
    type Output = [PPartEntry];

    fn index(&self, row: usize) -> &Self::Output {
        // Computing the end point is fairly expensive and only serves as a safety check...
        &self.inner[row * self.cols..]
    }
}

impl std::ops::IndexMut<usize> for Matrix2D {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.inner[row * self.cols..]
    }
}

/// The parts of a PPartMultiplier that involve heap allocation.
///
/// This lets us reuse the allocation across multiple different multipliers. Reusing the whole
/// PPartMultiplier is finicky but doable due to lifetime issues. However, it appears to be less
/// performant.
#[derive(Default)]
pub struct PPartAllocation {
    m: Matrix2D,
    #[cfg(feature = "odd-primes")]
    diagonal: Vec<PPartEntry>,
}

thread_local! {
    static ALLOCATION: Cell<PPartAllocation> = Cell::new(PPartAllocation::with_capacity(9));
}

impl PPartAllocation {
    /// This creates a PPartAllocation with enough capacity to handle mulitiply elements with
    /// of total degree < 2^n - ε at p = 2.
    pub fn with_capacity(n: usize) -> Self {
        Self {
            m: Matrix2D::with_capacity(n + 1, n),
            #[cfg(feature = "odd-primes")]
            diagonal: Vec::with_capacity(n),
        }
    }

    pub fn with_local(f: impl FnOnce(Self) -> Self) {
        ALLOCATION.with(|alloc| {
            alloc.set(f(alloc.take()));
        });
    }
}

/// The least `l > k` whose binary digits are disjoint from those of `sum`, **given that `k` is
/// itself disjoint from `sum`**.
///
/// Equivalently, the least such `l` with $\binom{\mathrm{sum} + l}{l}$ odd: adding `l` to `sum`
/// carries exactly where they share a set bit, and the 2-adic valuation of that binomial is the
/// number of carries (Kummer). So this steps straight to the next value that keeps a Milnor
/// coefficient non-zero mod 2, instead of testing candidates and discarding them.
///
/// # Panics
///
/// In debug builds, if `k & sum != 0`. The identity genuinely needs it: the increment is allowed
/// to carry through `k`'s own bits but not through `sum`'s, so a `k` that overlaps `sum` can come
/// back *smaller* than `k` (`next_disjoint(2, 2) == 1`). Every caller is walking a matrix whose
/// entries are already pairwise disjoint along each anti-diagonal, so the precondition holds.
///
/// The result can exceed any bound the caller has in mind; compare it against that separately.
pub const fn next_disjoint(sum: PPartEntry, k: PPartEntry) -> PPartEntry {
    debug_assert!(k & sum == 0, "next_disjoint needs k disjoint from sum");
    ((k | sum) + 1) & !sum
}

#[allow(non_snake_case)]
pub struct PPartMultiplier<const MOD4: bool> {
    p: ValidPrime,
    M: Matrix2D,
    r: PPart,
    rows: usize,
    cols: usize,
    diag_num: usize,
    init: bool,
    pub ans: MilnorBasisElement,
    #[cfg(feature = "odd-primes")]
    diagonal: Vec<PPartEntry>,
}

#[allow(non_snake_case)]
impl<const MOD4: bool> PPartMultiplier<MOD4> {
    fn prime(&self) -> ValidPrime {
        self.p
    }

    #[allow(unused_mut)] // Mut is only used with odd primes
    pub fn new_from_allocation(
        p: ValidPrime,
        r: PPart,
        s: PPart,
        mut allocation: PPartAllocation,
        q_part: u32,
        degree: i32,
    ) -> Self {
        if MOD4 {
            assert_eq!(p, 2);
        }
        let rows = r.len() + 1;
        let cols = s.len() + 1;
        let diag_num = r.len() + s.len();
        #[cfg(feature = "odd-primes")]
        {
            allocation.diagonal.clear();
            allocation.diagonal.reserve_exact(std::cmp::max(rows, cols));
        }

        let mut M = allocation.m;
        M.reset(rows, cols);

        for i in 1..rows {
            M[i][0] = r.entry(i - 1);
        }
        // Iterated rather than indexed because the loop variable would index the single row
        // `M[0]`, unlike the loop above, which indexes a different row each time.
        for (k, entry) in M[0][1..cols].iter_mut().enumerate() {
            *entry = s.entry(k);
        }

        let ans = MilnorBasisElement {
            q_part,
            p_part: PPart::zero(),
            degree,
        };
        Self {
            #[cfg(feature = "odd-primes")]
            diagonal: allocation.diagonal,
            p,
            M,
            r,
            rows,
            cols,
            diag_num,
            ans,
            init: true,
        }
    }

    pub fn into_allocation(self) -> PPartAllocation {
        PPartAllocation {
            m: self.M,
            #[cfg(feature = "odd-primes")]
            diagonal: self.diagonal,
        }
    }

    /// This compute the first l > k such that (sum + l) choose l != 0 mod p, stopping if we reach
    /// max + 1. This is useful for incrementing the matrix.
    ///
    /// TODO: Improve odd prime performance
    fn next_val(&self, sum: PPartEntry, k: PPartEntry, max: PPartEntry) -> PPartEntry {
        match self.prime().as_u32() {
            2 => {
                if MOD4 {
                    // x.count_ones() + y.count_ones() - (x + y).count_ones() is the number of
                    // carries when adding x to y.
                    //
                    // The p-adic valuation of (n + r) choose r is the number of carries when
                    // adding r to n in base p.
                    (k + 1..max + 1)
                        .find(|&l| {
                            sum & l == 0
                                || (sum.count_ones() + l.count_ones()) - (sum + l).count_ones() == 1
                        })
                        .unwrap_or(max + 1)
                } else {
                    next_disjoint(sum, k)
                }
            }
            _ => (k + 1..max + 1)
                .find(|&l| !PPartEntry::binomial_odd_is_zero(self.prime(), sum + l, l))
                .unwrap_or(max + 1),
        }
    }

    /// We have a matrix of the form
    ///    | s₁  s₂  s₃ ...
    /// --------------------
    /// r₁ |
    /// r₂ |     x_{ij}
    /// r₃ |
    ///
    /// We think of ourselves as modifiying the center pieces x_{ij}, while the r_i's and s_j's are
    /// only there to ensure the x_{ij}'s don't get too big. The idea is to sweep through the
    /// matrix row by row, from top-to-bottom, and left-to-right. In each pass, we find the first
    /// entry that can be incremented. We then increment it and zero out all the entries that
    /// appear before it. This will give us all valid entries.
    fn update(&mut self) -> bool {
        for i in 1..self.rows {
            // total is sum x_{ij} p^j up to the jth column
            let mut total = self.M[i][0];
            let mut p_to_the_j = 1;
            for j in 1..self.cols {
                p_to_the_j *= self.prime().as_u32();
                if total < p_to_the_j {
                    // We don't have enough weight left in the entries above this one in the column to increment this cell.
                    // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                    total += self.M[i][j] * p_to_the_j;
                    continue;
                }
                let col_sum: PPartEntry = (0..i).map(|k| self.M[k][j]).sum();
                if col_sum == 0 {
                    total += self.M[i][j] * p_to_the_j;
                    continue;
                }

                let max_inc = std::cmp::min(col_sum, total / p_to_the_j);

                // Compute the sum of entries along the diagonal to the bottom-left
                let mut sum = 0;
                for c in (i + j + 1).saturating_sub(self.rows)..j {
                    sum += self.M[i + j - c][c];
                }

                // Find the next possible value we can increment M[i][j] to without setting the
                // coefficient to 0. The coefficient is the multinomial coefficient of the
                // diagonal, and if the multinomial coefficient of any subset is zero, so is the
                // coefficient of the whole diagonal.
                let next_val = self.next_val(sum, self.M[i][j], max_inc + self.M[i][j]);
                let inc = next_val - self.M[i][j];

                // The remaining obstacle to incrementing this entry is the column sum condition.
                // For this, we only need a non-zero entry in the column j above row i.
                if inc <= max_inc {
                    // If so, we found our next matrix.
                    for row in 1..i {
                        self.M[row][0] = self.r.entry(row - 1);
                        for col in 1..self.cols {
                            self.M[0][col] += self.M[row][col];
                            self.M[row][col] = 0;
                        }
                    }
                    for col in 1..j {
                        self.M[0][col] += self.M[i][col];
                        self.M[i][col] = 0;
                    }
                    self.M[0][j] -= inc;
                    self.M[i][j] += inc;
                    self.M[i][0] = total - p_to_the_j * inc;
                    return true;
                }
                // All the cells above this one are zero so we didn't find our next matrix.
                // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                total += self.M[i][j] * p_to_the_j;
            }
        }
        false
    }
}

impl<const MOD4: bool> Iterator for PPartMultiplier<MOD4> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let p = self.prime().as_u32();
        'outer: loop {
            let mut coef = 1;

            if self.init {
                self.init = false;
                for i in 1..std::cmp::min(self.cols, self.rows) {
                    if MOD4 {
                        coef *= PPartEntry::binomial4(self.M[i][0] + self.M[0][i], self.M[0][i]);
                        coef %= 4;
                    } else {
                        coef *= PPartEntry::binomial(
                            self.prime(),
                            self.M[i][0] + self.M[0][i],
                            self.M[0][i],
                        );
                        coef %= p;
                    }
                    if coef == 0 {
                        continue 'outer;
                    }
                }
                // The answer is the top row of the matrix plus `r`, entrywise. Accumulate into
                // a plain word and store once.
                let mut ans = 0;
                for i in 0..std::cmp::max(self.cols, self.rows) - 1 {
                    let mut entry = self.r.entry(i);
                    if i + 1 < self.cols {
                        entry += self.M[0][i + 1];
                    }
                    debug_assert!(entry <= PPart::max_entry(i));
                    ans |= (entry as u64) << PPart::shift(i);
                }
                self.ans.p_part = PPart::from_bits(ans);
                return Some(coef);
            } else if self.update() {
                let mut ans = 0;
                for diag_idx in 1..=self.diag_num {
                    let i_min = (diag_idx + 1).saturating_sub(self.cols);
                    let i_max = std::cmp::min(diag_idx + 1, self.rows);
                    let mut sum = 0;

                    if self.prime() == 2 {
                        if MOD4 {
                            for i in i_min..i_max {
                                let entry = self.M[i][diag_idx - i];
                                sum += entry;
                                if coef.is_multiple_of(2) {
                                    coef *= PPartEntry::binomial2(sum, entry);
                                } else {
                                    coef *= PPartEntry::binomial4(sum, entry);
                                }
                                coef %= 4;
                                if coef == 0 {
                                    continue 'outer;
                                }
                            }
                        } else {
                            let mut or = 0;
                            for i in i_min..i_max {
                                sum += self.M[i][diag_idx - i];
                                or |= self.M[i][diag_idx - i];
                            }
                            if sum != or {
                                continue 'outer;
                            }
                        }
                    } else {
                        #[cfg(feature = "odd-primes")]
                        {
                            self.diagonal.clear();
                            for i in i_min..i_max {
                                self.diagonal.push(self.M[i][diag_idx - i]);
                                sum += self.M[i][diag_idx - i];
                            }

                            coef *= PPartEntry::multinomial_odd(self.prime(), &mut self.diagonal);
                            coef %= p;
                            if coef == 0 {
                                continue 'outer;
                            }
                        }
                    }
                    // `diag_num` counts diagonals of the working matrix, which can exceed the
                    // number of entries a p-part of this degree can have; those trailing
                    // diagonals are necessarily zero and need not be stored.
                    if diag_idx <= PPart::MAX_LEN {
                        debug_assert!(sum <= PPart::max_entry(diag_idx - 1));
                        ans |= (sum as u64) << PPart::shift(diag_idx - 1);
                    } else {
                        debug_assert_eq!(sum, 0);
                    }
                }
                self.ans.p_part = PPart::from_bits(ans);

                return Some(coef);
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;
    use crate::algebra::milnor_algebra::MilnorProfile;

    #[test]
    fn try_beps_pn_milnor() {
        let p = ValidPrime::new(2);

        // On the full algebra, `try_beps_pn` agrees with the panicking `beps_pn`
        // for valid inputs and never returns `None` (every P(x), x >= 1, exists).
        let algebra = MilnorAlgebra::new(p, false);
        for x in 1..16 {
            assert_eq!(algebra.try_beps_pn(0, x), Some(algebra.beps_pn(0, x)));
            assert!(algebra.try_beps_pn(0, x).is_some());
        }

        // On A(2) (profile [3, 2, 1], truncated), the first xi exponent is bounded
        // by 2^3 - 1 = 7, so P(7) is present but P(8) is excluded by the profile.
        let a2 = MilnorAlgebra::new_with_profile(
            p,
            MilnorProfile {
                q_part: !0,
                p_part: vec![3, 2, 1],
                truncated: true,
            },
            false,
        );
        // Valid input: agrees with `beps_pn`.
        assert_eq!(a2.try_beps_pn(0, 7), Some(a2.beps_pn(0, 7)));
        // Invalid input: excluded by the profile, so `None` instead of a panic.
        assert_eq!(a2.try_beps_pn(0, 8), None);
    }

    /// `try_beps_pn` is the non-panicking half of `beps_pn`; an out-of-range `x` must not trip
    /// overflow on the way to the bounds check.
    #[test]
    fn try_beps_pn_rejects_overflowing_x() {
        for p in [2, 3] {
            let algebra = MilnorAlgebra::new(ValidPrime::new(p), false);
            assert_eq!(algebra.try_beps_pn(0, PPartEntry::MAX), None);
            assert_eq!(algebra.try_beps_pn(0, PPartEntry::MAX / 2), None);
            assert_eq!(algebra.try_beps_pn(1, PPartEntry::MAX), None);
        }
    }

    /// `next_disjoint` is the jump-to-valid form of the "is this binomial odd" test that the
    /// Milnor coefficient needs; check it against the brute-force search it replaces, over every
    /// input satisfying its precondition.
    #[test]
    fn next_disjoint_matches_brute_force() {
        for sum in 0..64u32 {
            for k in (0..64u32).filter(|k| k & sum == 0) {
                let expected = (k + 1..)
                    .find(|l| l & sum == 0)
                    .expect("a disjoint value always exists");
                assert_eq!(next_disjoint(sum, k), expected, "sum = {sum}, k = {k}");
                // The characterisation it is actually used for.
                assert_ne!(u32::binomial2(sum + expected, expected), 0);
            }
        }
    }

    #[test]
    #[should_panic(expected = "next_disjoint needs k disjoint from sum")]
    fn next_disjoint_rejects_overlapping_k() {
        next_disjoint(2, 2);
    }

    #[test]
    fn test_ppart_multiplier_2() {
        let r = PPart::from_slice(&[1, 4]);
        let s = PPart::from_slice(&[2, 4]);
        let mut m = PPartMultiplier::<false>::new_from_allocation(
            fp::prime::TWO,
            r,
            s,
            PPartAllocation::default(),
            0,
            0,
        );

        expect![[r#"
            [0, 2, 4]
            [1, 0, 0]
            [4, 0, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(1));

        expect![[r#"
            [0, 0, 4]
            [1, 0, 0]
            [0, 2, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(1));

        expect![[r#"
            [0, 2, 3]
            [1, 0, 0]
            [0, 0, 1]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), None);
    }

    #[test]
    fn test_ppart_multiplier_3() {
        let r = PPart::from_slice(&[3, 4]);
        let s = PPart::from_slice(&[1, 4]);
        let mut m = PPartMultiplier::<false>::new_from_allocation(
            ValidPrime::new(3),
            r,
            s,
            PPartAllocation::default(),
            0,
            0,
        );

        expect![[r#"
            [0, 1, 4]
            [3, 0, 0]
            [4, 0, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(1));

        expect![[r#"
            [0, 1, 4]
            [3, 0, 0]
            [4, 0, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), Some(2));

        expect![[r#"
            [0, 0, 4]
            [3, 0, 0]
            [1, 1, 0]
        "#]]
        .assert_eq(&m.M.to_string());

        assert_eq!(m.next(), None);
    }
}
