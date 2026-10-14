//! Enumeration of Milnor products at the prime 2, one anti-diagonal at a time.

use super::PPart;

/// The number of interior entries (not in row or column 0) on anti-diagonals of index at most
/// [`PPart::MAX_LEN`].
const MAX_CELLS: usize = PPart::MAX_LEN * (PPart::MAX_LEN - 1) / 2;

/// Call `emit` with every p-part $T$ such that $P(T)$ appears in $P(r) P(s)$ at the prime 2, with
/// multiplicity: equal terms may be emitted several times and should be summed mod 2.
///
/// $P(r) P(s)$ is the sum, over matrices $X$ with $\sum_j 2^j x_{ij} = r_i$ for each row and
/// $\sum_i x_{ij} = s_j$ for each column, of $P(T(X))$, where $T_d$ is the sum of the $d$th
/// anti-diagonal. Mod 2 a matrix contributes exactly when the entries of each anti-diagonal have
/// pairwise disjoint binary digits.
///
/// We fill the interior entries one anti-diagonal at a time, from the last one down. The other
/// entries of row $d$ and column $d$ lie on later anti-diagonals, so on reaching anti-diagonal $d$
/// the slacks $x_{d0}$ and $x_{0d}$ are known. The anti-diagonal starts from their bitwise or, and
/// each interior entry only takes values disjoint from the bits used so far, so every completed
/// matrix contributes.
///
/// `degree` is an upper bound for the polynomial degree of the product. Entries on anti-diagonals
/// $d$ with $2^d - 1 >$ `degree` must vanish, so we never visit them.
pub(super) fn for_each_term(degree: i32, r: PPart, s: PPart, mut emit: impl FnMut(PPart)) {
    let rows = r.len();
    let cols = s.len();
    if rows == 0 || cols == 0 {
        emit(if rows == 0 { s } else { r });
        return;
    }
    let mut max_diag = (rows + cols).min(PPart::MAX_LEN);
    while max_diag > 1 && (1i64 << max_diag) - 1 > i64::from(degree) {
        max_diag -= 1;
    }

    // What is left of `r` and `s`, indexed from 1. Once row (resp. column) `d` is filled, this is
    // its entry in column (resp. row) 0.
    let mut r_left = [0u32; PPart::MAX_LEN + 1];
    let mut s_left = [0u32; PPart::MAX_LEN + 1];
    for i in 0..rows {
        r_left[i + 1] = r.get(i);
    }
    for j in 0..cols {
        s_left[j + 1] = s.get(j);
    }

    // The interior cells in fill order, and whether each is the last of its anti-diagonal.
    let mut row = [0u8; MAX_CELLS];
    let mut col = [0u8; MAX_CELLS];
    let mut last = [false; MAX_CELLS];
    let mut num_cells = 0;
    for d in (2..=max_diag).rev() {
        for i in d.saturating_sub(cols).max(1)..=rows.min(d - 1) {
            row[num_cells] = i as u8;
            col[num_cells] = (d - i) as u8;
            num_cells += 1;
        }
        last[num_cells - 1] = true;
    }

    let (a, b) = (r_left[max_diag], s_left[max_diag]);
    if a & b != 0 {
        return;
    }
    if max_diag == 1 {
        emit(PPart::from_bits(u64::from(a | b) << PPart::shift(0)));
        return;
    }

    // `value[k]` is the current entry of cell `k`, `bound[k]` the most it may be given the entries
    // before it, and `before[k]` the bits used on its anti-diagonal by the cells before it.
    let mut value = [0u32; MAX_CELLS];
    let mut bound = [0u32; MAX_CELLS];
    let mut before = [0u32; MAX_CELLS];
    // The packed value of $T_2, \ldots$, filled in as anti-diagonals are completed.
    let mut t_bits = 0u64;

    let mut k = 0;
    let mut tally = a | b;
    'enter: loop {
        // Value 0 never overlaps.
        let (i, j) = (row[k] as usize, col[k] as usize);
        before[k] = tally;
        bound[k] = (r_left[i] >> j).min(s_left[j]);
        value[k] = 0;
        'advance: loop {
            let used = before[k] | value[k];
            if !last[k] {
                k += 1;
                tally = used;
                continue 'enter;
            }
            // Anti-diagonal `d` is complete, and the slacks of `d - 1` are now known.
            let d = (row[k] + col[k]) as usize;
            let shift = PPart::shift(d - 1);
            t_bits = (t_bits & !(u64::from(PPart::max_entry(d - 1)) << shift))
                | (u64::from(used) << shift);
            let (a, b) = (r_left[d - 1], s_left[d - 1]);
            if a & b == 0 {
                if d == 2 {
                    emit(PPart::from_bits(
                        t_bits | u64::from(a | b) << PPart::shift(0),
                    ));
                } else {
                    k += 1;
                    tally = a | b;
                    continue 'enter;
                }
            }
            // Backtrack to the latest cell that can still be increased.
            loop {
                let (i, j) = (row[k] as usize, col[k] as usize);
                let v = value[k];
                let mask = before[k];
                let next = ((v | mask) + 1) & !mask;
                if next <= bound[k] {
                    r_left[i] -= (next - v) << j;
                    s_left[j] -= next - v;
                    value[k] = next;
                    continue 'advance;
                }
                r_left[i] += v << j;
                s_left[j] += v;
                if k == 0 {
                    return;
                }
                k -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::milnor_algebra::{PPartAllocation, PPartMultiplier};

    /// The polynomial degree of $P(x)$.
    fn degree(x: PPart) -> i32 {
        x.iter()
            .enumerate()
            .map(|(i, e)| ((1 << (i + 1)) - 1) * e as i32)
            .sum()
    }

    /// The terms of `P(r) P(s)` that survive mod 2, sorted, by anti-diagonal enumeration and by
    /// [`PPartMultiplier`].
    fn both(r: PPart, s: PPart) -> (Vec<PPart>, Vec<PPart>) {
        let degree = degree(r) + degree(s);
        let reduce = |mut terms: Vec<PPart>| {
            terms.sort_unstable();
            let mut odd = Vec::new();
            for chunk in terms.chunk_by(|a, b| a == b) {
                if chunk.len() % 2 == 1 {
                    odd.push(chunk[0]);
                }
            }
            odd
        };
        let mut new = Vec::new();
        for_each_term(degree, r, s, |t| new.push(t));
        let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
            fp::prime::TWO,
            r,
            s,
            PPartAllocation::default(),
            0,
            degree,
        );
        let mut old = Vec::new();
        while let Some(c) = multiplier.next() {
            if c % 2 == 1 {
                old.push(multiplier.ans.p_part);
            }
        }
        (reduce(new), reduce(old))
    }

    #[test]
    fn small_products() {
        let p = PPart::from_slice;
        // Sq(1) Sq(1) = 0, Sq(1) Sq(2) = Sq(3), Sq(2) Sq(1) = Sq(3) + Sq(0, 1).
        assert_eq!(both(p(&[1]), p(&[1])).0, vec![]);
        assert_eq!(both(p(&[1]), p(&[2])).0, vec![p(&[3])]);
        let (new, old) = both(p(&[2]), p(&[1]));
        assert_eq!(new, old);
        assert_eq!(new.len(), 2);
        // Sq(4, 2) Sq(4, 1) = Sq(4, 2, 1) + Sq(7, 1, 1) + Sq(1, 3, 1).
        let mut want = vec![p(&[4, 2, 1]), p(&[7, 1, 1]), p(&[1, 3, 1])];
        want.sort_unstable();
        assert_eq!(both(p(&[4, 2]), p(&[4, 1])).0, want);
    }

    /// Every pair of p-parts of length at most 4 with small entries.
    #[test]
    fn all_small_products_match() {
        const BOUND: [u32; 4] = [12, 6, 3, 2];
        let mut parts = vec![PPart::zero()];
        for len in 1..=BOUND.len() {
            let mut e = vec![0u32; len];
            loop {
                if e[len - 1] != 0 {
                    parts.push(PPart::from_slice(&e));
                }
                let Some(i) = (0..len).find(|&i| e[i] < BOUND[i]) else {
                    break;
                };
                e[..i].fill(0);
                e[i] += 1;
            }
        }
        let mut nonzero = 0;
        for &r in &parts {
            for &s in &parts {
                let (new, old) = both(r, s);
                assert_eq!(new, old, "P{r:?} P{s:?}");
                nonzero += usize::from(!new.is_empty());
            }
        }
        assert!(nonzero > 0);
    }

    /// Long p-parts, up to the largest length a [`PPart`] can hold.
    #[test]
    fn long_products_match() {
        let p = PPart::from_slice;
        for (r, s) in [
            (p(&[60, 30, 8, 2, 1]), p(&[20, 30, 20, 4, 1, 2])),
            (p(&[35, 12, 20, 14, 1, 3]), p(&[60, 30, 0, 2, 1])),
            (p(&[3, 1, 0, 0, 0, 0, 0, 0, 0, 1]), p(&[1, 2, 1])),
            (
                p(&[0, 0, 0, 0, 0, 0, 0, 0, 1]),
                p(&[5, 0, 0, 0, 0, 0, 0, 0, 1]),
            ),
        ] {
            let (new, old) = both(r, s);
            assert_eq!(new, old, "P{r:?} P{s:?}");
        }
    }
}
