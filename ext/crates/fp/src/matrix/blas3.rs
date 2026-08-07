//! BLAS3 (GEMM-based) row reduction over F₂.
//!
//! This is the CPU-resident Phase 1 of the plan in `BLAS3-ROW-REDUCTION.md`: a
//! right-looking blocked Gauss–Jordan reduction whose dominant work — the
//! trailing-submatrix update — is a single F₂ matrix product per column panel,
//! dispatched through `<&Matrix as Mul>::mul` (the AVX-512 tiled kernel today,
//! the Hopper `wgmma.b1` kernel once `fp-cuda` is wired in). It produces exactly
//! the same reduced row echelon form and pivot list as [`Matrix::row_reduce`],
//! which is what the proptests at the bottom assert.
//!
//! # Shape of the algorithm
//!
//! Sweep column panels `[c, c+b)` left to right, keeping a running pivot count
//! `r` (the finished pivot rows live at the top, in column order).
//!
//! * **Panel factorization** (`[c, c+b)`): Gauss–Jordan on the panel, but with
//!   the row operations restricted to the panel's limbs. This is the only place
//!   we touch individual columns, and it is narrow (`b` a small multiple of 64).
//!   As each pivot column `q_k` is cleared from a row `j`, the cleared bit is the
//!   multiplier `L[j][k]`; because the pivot rows are kept mutually reduced, that
//!   bit equals the pristine `A[j, q_k]` and the `L[j, ·]` are independent.
//! * **Trailing update**: `M[:, c+b:] ^= L · U`, where `U` is the pivot rows'
//!   trailing part. One F₂ GEMM. Because `c+b` is a multiple of 64 the trailing
//!   region starts at a limb boundary, so `U` is a contiguous limb slice and the
//!   XOR of the product back into `M` is a whole-limb, row-parallel operation.
//!
//! Total cost is `O(m · n · R)` with `R` the rank, GEMM-dominated and
//! proportional to rank — so the highly rank-deficient matrices this targets are
//! cheaper for free.

use crate::{limb::Limb, matrix::Matrix, prime::TWO};

/// Default panel width in columns. A multiple of 64; wide enough that the
/// trailing GEMM's inner (`k`) dimension is a healthy number of pivots, narrow
/// enough that the panel factorization stays a lower-order term.
const DEFAULT_BLOCK_COLS: usize = 256;

impl Matrix {
    /// GEMM-based reduction to reduced row echelon form; F₂ only.
    ///
    /// Bit-for-bit identical result and pivots to [`Matrix::row_reduce`]. Falls
    /// back to [`Matrix::row_reduce`] for `p ≠ 2`.
    ///
    /// # Returns
    /// The rank (number of non-zero rows after reduction), like `row_reduce`.
    pub fn row_reduce_blas3(&mut self) -> usize {
        self.row_reduce_blas3_block(DEFAULT_BLOCK_COLS)
    }

    /// [`Matrix::row_reduce_blas3`] with an explicit panel width (rounded down to
    /// a multiple of 64, and at least 64). Exposed for benchmarking the crossover.
    pub fn row_reduce_blas3_block(&mut self, block_cols: usize) -> usize {
        if self.prime() != 2 {
            return self.row_reduce();
        }

        let m = self.rows();
        let n = self.columns();
        self.initialize_pivots();
        if m == 0 || n == 0 {
            return 0;
        }

        // Panels are whole limbs. Columns are stored padded to a limb boundary
        // (`stride` limbs, bits past `n` are zero), so we can reason in limbs and
        // treat the final partial limb as a full one WLOG — the padding columns
        // are zero, never become pivots, and cost nothing. Only the pivot search,
        // which indexes real columns, is capped at `n`.
        let stride = self.stride(); // limbs per row
        let bl = (block_cols.max(64) & !63) / 64; // panel width in limbs

        // Reused scratch for a pivot row's panel limbs, so the inner elimination
        // loop is a plain limb XOR with no per-row re-borrow.
        let mut piv_panel = vec![0 as Limb; bl];

        let mut r = 0usize; // pivots found so far; pivot rows sit at [0, r)
        let mut limb_lo = 0usize; // current panel's first limb
        while limb_lo < stride {
            let limb_hi = (limb_lo + bl).min(stride); // panel's end limb (exclusive)
            let col_lo = limb_lo * 64;
            let col_hi = (limb_hi * 64).min(n); // last real column of the panel
            let r_start = r;

            // ---- Step A: panel factorization on columns [col_lo, col_hi) ----
            // Row ops are restricted to the panel's limbs [limb_lo, limb_hi); the
            // trailing columns [limb_hi, stride) are fixed up by the GEMM below.
            //
            // Multiplier bits, one column per pivot found. Only *deferred* rows
            // (rows below the pivots) get an entry; the pivot rows are made exact
            // by the promotion step instead.
            let mut l = Self::new(TWO, m, col_hi - col_lo);
            let mut pr = 0usize; // pivots found in this panel

            let panel_w = limb_hi - limb_lo;
            for q in col_lo..col_hi {
                let piv_row = r_start + pr;
                let qlimb = q / 64; // absolute limb of column q (in [limb_lo, limb_hi))
                let qbit = q % 64;

                // Pivot search (raw-limb): first row in [piv_row, m) with bit q
                // set. Rows [piv_row, m) already had this panel's earlier pivot
                // columns cleared, so a non-zero here is a genuine pivot.
                let mut found = None;
                {
                    let data = self.data();
                    for i in piv_row..m {
                        if (data[i * stride + qlimb] >> qbit) & 1 == 1 {
                            found = Some(i);
                            break;
                        }
                    }
                }
                let Some(i) = found else { continue };

                if i != piv_row {
                    self.swap_rows(i, piv_row);
                    // `l` is indexed by current row position, so it must track the
                    // same swap: a row moved up here may already carry deferred
                    // multiplier bits from earlier pivots in this panel.
                    l.swap_rows(i, piv_row);
                }

                // Promote `piv_row` to an exact echelon pivot row: realize its
                // deferred trailing by replaying this panel's earlier pivots into
                // its trailing columns. Those earlier pivots are stable — forward
                // elimination never reduces a pivot by a *later* one — so their
                // trailing is final and this catch-up is correct. Its panel limbs
                // were already reduced while it was a below row.
                {
                    let lstride = l.stride();
                    for kp in 0..pr {
                        if (l.data()[piv_row * lstride + kp / 64] >> (kp % 64)) & 1 == 1 {
                            xor_limb_range(self, piv_row, r_start + kp, limb_hi, stride);
                        }
                    }
                }
                zero_row(&mut l, piv_row);

                // Forward elimination (raw-limb): clear column q from the rows
                // *below* the pivot only, recording the multiplier and touching
                // just the panel limbs (trailing deferred to the GEMM). Rows above
                // are left for the back-substitution pass — reducing them here
                // would destabilize the pivot trailings the promotion relies on.
                {
                    let piv_base = piv_row * stride;
                    piv_panel[..panel_w]
                        .copy_from_slice(&self.data()[piv_base + limb_lo..piv_base + limb_hi]);
                }
                {
                    let lstride = l.stride();
                    let lword = pr / 64;
                    let lmask: Limb = 1 << (pr % 64);
                    let ldata = l.data_mut();
                    let data = self.data_mut();
                    for j in piv_row + 1..m {
                        let jbase = j * stride;
                        if (data[jbase + qlimb] >> qbit) & 1 == 1 {
                            ldata[j * lstride + lword] |= lmask;
                            for t in 0..panel_w {
                                data[jbase + limb_lo + t] ^= piv_panel[t];
                            }
                        }
                    }
                }

                self.pivots_mut()[q] = piv_row as isize;
                pr += 1;
            }
            r = r_start + pr;

            // ---- Step B: trailing update  M[:, limb_hi:] ^= L · U ----
            let trailing_limbs = stride - limb_hi;
            if pr > 0 && trailing_limbs > 0 {
                let t = n - limb_hi * 64; // real trailing columns (> 0 here)

                // Trim L to its m × pr occupied columns so the GEMM's inner
                // dimension is the pivots actually found, not the panel width.
                let n_l = pr.div_ceil(64);
                let mut l_trim = Self::new(TWO, m, pr);
                {
                    let l_stride = l.stride();
                    let lt_stride = l_trim.stride();
                    let src = l.data();
                    let dst = l_trim.data_mut();
                    for j in 0..m {
                        dst[j * lt_stride..j * lt_stride + n_l]
                            .copy_from_slice(&src[j * l_stride..j * l_stride + n_l]);
                    }
                }

                // U = pivot rows' trailing part, a contiguous limb slice.
                let mut u = Self::new(TWO, pr, t);
                {
                    let u_stride = u.stride(); // == trailing_limbs
                    debug_assert_eq!(u_stride, trailing_limbs);
                    let src = self.data();
                    let dst = u.data_mut();
                    for k in 0..pr {
                        let base = (r_start + k) * stride + limb_hi;
                        dst[k * u_stride..k * u_stride + trailing_limbs]
                            .copy_from_slice(&src[base..base + trailing_limbs]);
                    }
                }

                // One F₂ GEMM (GPU / AVX-512 tiled kernel), then a whole-limb XOR
                // of the m × t product into M's trailing columns.
                let g = &l_trim * &u;
                let g_stride = g.stride(); // == trailing_limbs
                {
                    let src = g.data();
                    let dst = self.data_mut();
                    for j in 0..m {
                        let m_base = j * stride + limb_hi;
                        let g_base = j * g_stride;
                        for col in 0..trailing_limbs {
                            dst[m_base + col] ^= src[g_base + col];
                        }
                    }
                }
            }

            limb_lo = limb_hi;
        }

        // ---- Back-substitution: echelon form → reduced row echelon form ----
        // The forward sweep left the R = `r` pivot rows in echelon form (rows
        // [0, r), pivot k in increasing column order, each already reduced by
        // *earlier* pivots). We now clear the *later* pivot columns from the rows
        // above them, blocked into the same `X·U` GEMM shape as the forward
        // trailing update — the mirror image, walking pivot blocks right-to-left.
        //
        // For a block of pivots (rows [s, e)): (1) reduce the block to RREF among
        // itself with a few full-width row ops, then (2) clear the block's pivot
        // columns from every row above via one GEMM. Blocks to the right are done
        // first, so a block's rows are already clear of every pivot column to
        // their right when we reach them, and rows above are reduced by the block
        // in one shot. Same O(R²·n) work as the naive sweep, but as GEMMs.
        let pivot_cols: Vec<usize> = (0..n).filter(|&q| self.pivots()[q] >= 0).collect();
        debug_assert_eq!(pivot_cols.len(), r);
        let bp = bl * 64; // pivots per back-substitution block

        let mut e = r;
        while e > 0 {
            let s = e.saturating_sub(bp);
            let bp_eff = e - s;

            // (1) Reduce block rows [s, e) to RREF among themselves: clear each
            // block pivot column from the earlier block rows. Processing high-to-
            // low keeps every source row fully reduced before it is used.
            for k in (s..e).rev() {
                let qk = pivot_cols[k];
                for j in s..k {
                    if self.row(j).entry(qk) != 0 {
                        self.safe_row_op(j, k, 1);
                    }
                }
            }

            // (2) Clear the block's pivot columns from all rows above [0, s) with
            // one GEMM: `M[0..s, :] ^= X · U`, where `X[j][i] = M[j, q_{s+i}]` and
            // `U` is the (now RREF) block rows. The block rows are zero below
            // column `q_s`, so the update starts at that limb boundary.
            if s > 0 {
                let start_limb = pivot_cols[s] / 64;
                let trailing_limbs = stride - start_limb;
                let width = n - start_limb * 64;

                // X = rows above, gathered at this block's pivot columns (s ×
                // bp_eff), raw-limb: read bit `q_{s+i}` of row j, set bit i of X.
                let mut x = Self::new(TWO, s, bp_eff);
                {
                    let x_stride = x.stride();
                    let src = self.data();
                    let dst = x.data_mut();
                    for j in 0..s {
                        for i in 0..bp_eff {
                            let qc = pivot_cols[s + i];
                            if (src[j * stride + qc / 64] >> (qc % 64)) & 1 == 1 {
                                dst[j * x_stride + i / 64] |= 1 << (i % 64);
                            }
                        }
                    }
                }

                // U = block rows [s, e), limbs [start_limb, stride) (bp_eff × width).
                let mut u = Self::new(TWO, bp_eff, width);
                {
                    let u_stride = u.stride(); // == trailing_limbs
                    debug_assert_eq!(u_stride, trailing_limbs);
                    let src = self.data();
                    let dst = u.data_mut();
                    for i in 0..bp_eff {
                        let base = (s + i) * stride + start_limb;
                        dst[i * u_stride..i * u_stride + trailing_limbs]
                            .copy_from_slice(&src[base..base + trailing_limbs]);
                    }
                }

                let g = &x * &u;
                let g_stride = g.stride(); // == trailing_limbs
                {
                    let src = g.data();
                    let dst = self.data_mut();
                    for j in 0..s {
                        let m_base = j * stride + start_limb;
                        let g_base = j * g_stride;
                        for col in 0..trailing_limbs {
                            dst[m_base + col] ^= src[g_base + col];
                        }
                    }
                }
            }

            e = s;
        }

        r
    }
}

/// Zero every limb of row `row` in matrix `m` (used to discharge a pivot row's
/// deferred multipliers once its trailing has been realized).
fn zero_row(m: &mut Matrix, row: usize) {
    let stride = m.stride();
    let base = row * stride;
    m.data_mut()[base..base + stride].fill(0);
}

/// XOR the limbs `[lo, hi)` of row `src` into the same limbs of row `dst`.
/// `dst != src` required. Restricted to a limb range so it can touch a panel
/// without disturbing the trailing columns.
fn xor_limb_range(m: &mut Matrix, dst: usize, src: usize, lo: usize, hi: usize) {
    debug_assert_ne!(dst, src);
    let stride = m.stride();
    let d = dst * stride;
    let s = src * stride;
    let data = m.data_mut();
    // Split at the higher row's start so the two rows land in disjoint halves.
    if d < s {
        let (left, right) = data.split_at_mut(s);
        for l in lo..hi {
            left[d + l] ^= right[l];
        }
    } else {
        let (left, right) = data.split_at_mut(d);
        for l in lo..hi {
            right[l] ^= left[s + l];
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::{
        matrix::{Matrix, arbitrary::MatrixArbParams},
        prime::TWO,
    };

    /// Arbitrary F₂ matrices across a spread of shapes, including sizes above and
    /// below the 32-row padding boundary and the 64-column limb boundary, and
    /// wider than one default panel.
    fn arb_matrix(max_dim: usize) -> impl Strategy<Value = Matrix> {
        (1usize..=max_dim, 1usize..=max_dim).prop_flat_map(|(rows, cols)| {
            Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(rows).boxed(),
                columns: Just(cols).boxed(),
            })
        })
    }

    /// A deliberately rank-deficient F₂ matrix: `A · B` with a small shared
    /// dimension `rank_bound`, so the product has rank at most `rank_bound`.
    fn arb_low_rank() -> impl Strategy<Value = Matrix> {
        (40usize..300, 40usize..300, 1usize..30).prop_flat_map(|(rows, cols, rank_bound)| {
            let a = Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(rows).boxed(),
                columns: Just(rank_bound).boxed(),
            });
            let b = Matrix::arbitrary_with(MatrixArbParams {
                p: Some(TWO),
                rows: Just(rank_bound).boxed(),
                columns: Just(cols).boxed(),
            });
            (a, b).prop_map(|(a, b)| &a * &b)
        })
    }

    fn assert_matches_row_reduce(m: &Matrix, block: usize) -> Result<(), TestCaseError> {
        let mut reference = m.clone();
        let ref_rank = reference.row_reduce();

        let mut blocked = m.clone();
        let blocked_rank = blocked.row_reduce_blas3_block(block);

        prop_assert_eq!(blocked_rank, ref_rank, "rank mismatch");
        prop_assert_eq!(blocked.pivots(), reference.pivots(), "pivot mismatch");
        prop_assert_eq!(&blocked, &reference, "RREF mismatch");
        Ok(())
    }

    // Deterministic sweep over small shapes that straddle the 64-column limb
    // boundary (so multiple panels and partial-limb trailings are exercised).
    // On mismatch it prints the exact input, unlike a shrunk proptest dump.
    #[test]
    fn matches_row_reduce_deterministic() {
        // xorshift RNG for reproducibility (no external rng dep needed here).
        let mut state: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for rows in 1usize..12 {
            for cols in 65usize..200 {
                // multiple panels at block 64, various partial-limb trailings
                for _ in 0..40 {
                    let input: Vec<Vec<u32>> = (0..rows)
                        .map(|_| (0..cols).map(|_| (next() & 1) as u32).collect())
                        .collect();
                    let base = Matrix::from_vec(TWO, &input);
                    let mut reference = base.clone();
                    reference.row_reduce();
                    let mut blocked = base.clone();
                    blocked.row_reduce_blas3_block(64);
                    if blocked != reference || blocked.pivots() != reference.pivots() {
                        panic!(
                            "MISMATCH rows={rows} cols={cols}\ninput={input:?}\nexpected \
                             pivots={:?}\n got pivots={:?}\nexpected=\n{reference}\n \
                             got=\n{blocked}",
                            reference.pivots(),
                            blocked.pivots()
                        );
                    }
                }
            }
        }
    }

    proptest! {
        #[test]
        fn matches_row_reduce_small(m in arb_matrix(80)) {
            assert_matches_row_reduce(&m, 64)?;
        }

        #[test]
        fn matches_row_reduce_default_block(m in arb_matrix(200)) {
            assert_matches_row_reduce(&m, 256)?;
        }

        // A small block forces many panels, exercising the panel-to-panel
        // invariant (trailing update of already-reduced pivot rows).
        #[test]
        fn matches_row_reduce_tiny_block(m in arb_matrix(200)) {
            assert_matches_row_reduce(&m, 64)?;
        }

        #[test]
        fn matches_row_reduce_low_rank(m in arb_low_rank()) {
            assert_matches_row_reduce(&m, 128)?;
        }
    }
}
