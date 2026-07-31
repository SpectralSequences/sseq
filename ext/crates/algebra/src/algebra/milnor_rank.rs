//! An arithmetic replacement for the `MilnorBasisElement -> index` hash map.
//!
//! Behind the off-by-default `milnor-rank` feature, and **not wired into [`MilnorAlgebra`]** even
//! when enabled -- `basis_element_to_index` still goes through the hash map. It is here so the
//! design and its measurements survive, ready to switch on when the renumbering below is worth
//! taking on.
//!
//! **This is not wired into [`MilnorAlgebra`].** It computes a *different* numbering of each
//! degree's basis than [`MilnorAlgebra::compute_basis`] produces, so adopting it would renumber
//! the basis and invalidate every saved resolution. See [`PPartRanker`] for why the numbering
//! cannot simply be made to match, and the crate benchmarks (`milnor_rank`) for what it costs
//! relative to the hash map it would replace.

use fp::prime::ValidPrime;

use crate::algebra::{combinatorics, milnor_algebra::PPart};

/// Computes a p-part's index within its degree arithmetically, instead of looking it up.
///
/// # How it works
///
/// `counts[i][d]` is the number of exponent sequences of degree `d` (in units of `q`) that use
/// only $\xi_1, \ldots, \xi_i$. Splitting on whether $r_i$ is zero gives the coin-change
/// recurrence
///
/// ```text
/// counts[i][d] = counts[i - 1][d] + counts[i][d - xi_i]
/// ```
///
/// so the table costs `O(MAX_LEN * max_degree)` to build, and `counts[MAX_LEN][d]` is the
/// dimension of the algebra in degree `d`.
///
/// Ranking then rests on one identity. Among the sequences of degree `d` using
/// $\xi_1, \ldots, \xi_i$, those with $r_i \ge u$ are in bijection with *all* sequences of degree
/// `d - u * xi_i` using $\xi_1, \ldots, \xi_i$, via $r_i \mapsto r_i - u$. So the number of them
/// with $r_i > v$ is `counts[i][d - (v + 1) * xi_i]`: a single lookup, with no summation. Walking
/// the entries from the top down therefore ranks a p-part in [`PPart::MAX_LEN`] lookups and adds,
/// against a table of a few hundred KiB that serves *every* degree at once — where the hash map
/// it replaces stores one entry per basis element.
///
/// # Speed: it depends entirely on scale
///
/// Measured against the hash map it would replace (`cargo bench --bench milnor_rank`), at p = 2:
///
/// ```text
/// degree   per-degree map   hashmap    ranker    ratio
///    120          0.10 MB    11.6us    26.7us    0.43x
///    300          3.12 MB     792us    1384us    0.57x
///    400         12.50 MB    4490us    5310us    0.85x
///    500         37.50 MB   33118us   15865us    2.09x
/// ```
///
/// The crossover is a cache effect, and it is not marginal. A lookup probes only its own degree's
/// map. While that fits in cache the map wins easily: one hash round and one probe, against six to
/// ten dependent table reads for a rank. Once it does not — the map is 37 MB in degree 500 — every
/// probe misses to DRAM at ~33 ns, whereas the ranker's whole table is ~43 KB, stays in L1, and
/// costs ~16 ns regardless of degree. The map's cost grows with the basis; the ranker's does not.
///
/// So the ranker is the wrong tool for small degrees and the right one for large. That matters
/// because large degrees are exactly where the algebra's memory becomes the problem worth solving.
///
/// The reverse direction does not share this, and is deliberately absent. An `unrank` -- recovering
/// the p-part at a given index, which would let `basis_element_from_index` drop `ppart_table`
/// altogether -- was written, tested and benchmarked, and removed again: it ran ~15x slower than
/// the array read it would replace, at every degree measured, with no sign of the crossover that
/// makes `rank` worthwhile. The reason is that `ppart_table` is only 8 bytes per element, against
/// ~43 for the hash map, so it stays cache-resident where the map does not. The bit-packing that
/// makes `rank` worth having is the same thing that makes `unrank` not. (See the history around
/// "Speed up unrank 1.5x" if it needs revisiting; the likelier route is enumerating the basis in
/// index order, which is O(1) amortised and matches how callers actually walk it.)
///
/// Tuning does not move this. Five arrangements were measured — nested vs flat count table, with
/// and without a zero-padded prefix to drop the branch, and one- vs two-pass to break the
/// dependency chain. None changed the small-degree verdict; the padded variant was worst, because
/// doubling the table pushed it out of L1.
///
/// # Why it is still not wired in
///
/// Not speed, but numbering. This ranks in colex order on $(r_{10}, \ldots, r_1)$. `compute_ppart` emits a different order:
/// it groups by the highest non-zero entry, and recurses by decrementing one entry at a time.
/// That order *is* rankable in principle, but its natural recursion has depth $\sum_i r_i$ — up to
/// `MAX_DEGREE` — which is far worse than hashing. Getting the `O(MAX_LEN)` cost requires adopting
/// the colex order, i.e. renumbering the basis.
///
/// A renumbering is not intrinsically hard — [`crate::Algebra::magic`] already exists to
/// discriminate save files — but it invalidates stored resolutions, so it is a migration rather
/// than a drop-in change. Note also that [`MilnorAlgebra`] re-sorts each degree by excess when
/// unstable support is enabled, which this does not model.
///
/// [`MilnorAlgebra`]: crate::MilnorAlgebra
/// [`MilnorAlgebra::compute_basis`]: crate::Algebra::compute_basis
pub struct PPartRanker {
    /// `counts[i][d]` flattened to `counts[i * stride + d]`, for `i` in `0..=PPart::MAX_LEN`.
    ///
    /// Flat rather than `Vec<Vec<_>>`, so a lookup is not a dependent pointer chase.
    counts: Vec<u64>,
    stride: usize,
    /// `effective_len[d]` is the number of $\xi_i$ of degree at most `d`.
    ///
    /// Entries beyond it cannot contribute to a rank in degree `d`: such an entry must be zero, and
    /// its `cut` is then `d - xi_i < 0`. At degree 120 this is 6 rather than 10, so it removes
    /// roughly a third of the work.
    effective_len: Vec<u8>,
    /// `xi[i]` is the degree of $\xi_{i+1}$, divided by `q`.
    xi: [i32; PPart::MAX_LEN],
    max_degree: i32,
}

impl PPartRanker {
    /// Build the table for degrees `0..=max_degree`, where `max_degree` is measured in units of
    /// `q` (so it is the internal degree at `p = 2`, and the internal degree divided by
    /// `2(p - 1)` otherwise).
    pub fn new(p: ValidPrime, max_degree: i32) -> Self {
        assert!(max_degree >= 0);
        let mut xi = [0; PPart::MAX_LEN];
        xi.copy_from_slice(&combinatorics::xi_degrees(p)[..PPart::MAX_LEN]);

        let stride = max_degree as usize + 1;

        let mut counts = vec![0; (PPart::MAX_LEN + 1) * stride];
        // The empty sequence is the unique sequence of degree 0 using no generators.
        counts[0] = 1;
        for i in 1..=PPart::MAX_LEN {
            for d in 0..stride {
                // Either r_i is zero, or we can subtract one from it.
                counts[i * stride + d] = counts[(i - 1) * stride + d];
                if d >= xi[i - 1] as usize {
                    counts[i * stride + d] += counts[i * stride + d - xi[i - 1] as usize];
                }
            }
        }

        let effective_len = (0..=max_degree)
            .map(|d| xi.iter().filter(|&&x| x <= d).count() as u8)
            .collect();

        Self {
            counts,
            stride,
            effective_len,
            xi,
            max_degree,
        }
    }

    /// The number of p-parts of degree `degree`, i.e. what `MilnorAlgebra::dimension` returns for
    /// the p-part factor of the basis.
    pub fn dimension(&self, degree: i32) -> u64 {
        if degree < 0 || degree > self.max_degree {
            0
        } else {
            self.counts[PPart::MAX_LEN * self.stride + degree as usize]
        }
    }

    /// The index of `p_part` among the p-parts of degree `degree`, in the colex order described on
    /// [`PPartRanker`].
    ///
    /// `degree` must be the degree of `p_part` (in units of `q`), and at most the `max_degree`
    /// this was built with.
    ///
    /// # Cost
    ///
    /// One table read per entry, serialised through the running `remaining`. That is the reason
    /// this loses to the hash map it was meant to replace, and no arrangement of the table fixes
    /// it — see the module docs.
    #[inline]
    pub fn rank(&self, p_part: PPart, degree: i32) -> usize {
        debug_assert!(degree >= 0 && degree <= self.max_degree);
        let mut rank = 0;
        let mut remaining = degree;
        // Only the entries with `xi_i <= degree` can contribute; the rest are zero with a negative
        // cut. At degree 120 that is 6 iterations rather than 10.
        for i in (0..self.effective_len[degree as usize] as usize).rev() {
            let entry = p_part.get(i) as i32;
            // Everything with a larger entry here sorts earlier, and there are exactly
            // `counts[i + 1][remaining - (entry + 1) * xi_i]` of them.
            let cut = remaining - (entry + 1) * self.xi[i];
            if cut >= 0 {
                rank += self.counts[(i + 1) * self.stride + cut as usize];
            }
            remaining -= entry * self.xi[i];
        }
        debug_assert_eq!(remaining, 0, "degree does not match the p-part");
        rank as usize
    }
}

#[cfg(test)]
mod tests {
    use fp::prime::Prime;
    use rstest::rstest;

    use super::*;
    use crate::{Algebra, MilnorAlgebra};

    /// `counts[MAX_LEN]` must agree with the algebra's own count of p-parts in each degree.
    #[rstest]
    #[case(2, 120)]
    #[case(3, 40)]
    fn table_matches_ppart_table(#[case] p: u32, #[case] max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p, false);
        let q = if p == 2 { 1 } else { 2 * (p.as_i32() - 1) };
        algebra.compute_basis(max_degree * q);

        let ranker = PPartRanker::new(p, max_degree);
        for d in 0..=max_degree {
            assert_eq!(
                ranker.dimension(d),
                algebra.ppart_table(d).len() as u64,
                "dimension mismatch in degree {d}"
            );
        }
    }

    /// The whole point: `rank` must be a bijection from the p-parts of each degree onto
    /// `0..dimension`. If it is, it is a valid numbering and could replace the hash map.
    #[rstest]
    #[case(2, 120)]
    #[case(3, 40)]
    fn rank_is_a_bijection(#[case] p: u32, #[case] max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p, false);
        let q = if p == 2 { 1 } else { 2 * (p.as_i32() - 1) };
        algebra.compute_basis(max_degree * q);

        let ranker = PPartRanker::new(p, max_degree);
        for d in 0..=max_degree {
            let table = algebra.ppart_table(d);
            let mut seen = vec![false; table.len()];
            for &p_part in table {
                let rank = ranker.rank(p_part, d);
                assert!(rank < table.len(), "rank {rank} out of range in degree {d}");
                assert!(!seen[rank], "rank {rank} hit twice in degree {d}");
                seen[rank] = true;
            }
        }
    }

    /// Ranking in colex order really is a different numbering than the one the algebra uses. This
    /// is the reason the ranker is not wired in, so pin it down rather than leave it to prose.
    #[test]
    fn rank_disagrees_with_the_current_basis_order() {
        let p = ValidPrime::new(2);
        let algebra = MilnorAlgebra::new(p, false);
        algebra.compute_basis(120);
        let ranker = PPartRanker::new(p, 120);

        let mut agree = 0;
        let mut total = 0;
        for d in 0..=120 {
            for (i, &p_part) in algebra.ppart_table(d).iter().enumerate() {
                total += 1;
                if ranker.rank(p_part, d) == i {
                    agree += 1;
                }
            }
        }
        assert!(
            agree * 100 < total,
            "expected the orders to differ on almost everything, but {agree}/{total} agreed"
        );
    }
}
