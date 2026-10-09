//! Two-sided enumeration of Milnor products at the prime 2.

use super::PPart;

/// The largest submask of `upper_bound` that avoids `mask`, rounded so that decrementing it and
/// clearing `mask` walks the remaining candidates in decreasing order.
#[inline]
fn max_mask(upper_bound: u32, mask: u32) -> u32 {
    let mut m = upper_bound & mask;
    let mut n = 0;
    while {
        m >>= 1;
        m != 0
    } {
        n |= m;
    }
    (upper_bound | n) & !mask
}

/// Define a function enumerating the terms of $P(r) P(s)$ for exponent sequences of length at most
/// `$n`. The loop bounds are then compile time constants, which matters for speed, so we generate
/// one copy per length and pick the smallest one that fits at runtime.
///
/// The matrix is stored in arrays indexed by `row * (N + 1) + col`: `x` holds the entries, `xr` and
/// `xs` the parts of $R$ and $S$ that remain to be placed, and `xt` the bits already used on each
/// anti-diagonal. The matrix is filled column by column starting from the last non-zero row; each
/// entry starts at the largest value allowed by the remaining budgets and the bits already used on
/// its diagonal, and backtracking decreases it within the same lattice of submasks.
macro_rules! define_for_each_term {
    ($name:ident, $n:expr) => {
        /// Call `emit` with every $T$ such that $P(T)$ appears in $P(r) P(s)$. Equal terms may be
        /// emitted several times, and should be summed mod 2.
        fn $name(r: [u32; $n], s: [u32; $n], mut emit: impl FnMut([u32; $n])) {
            const N: usize = $n;
            const NCOL: usize = N + 1;
            let mut x = [0u32; NCOL * NCOL];
            let mut xr = [0u32; NCOL * NCOL];
            let mut xs = [0u32; NCOL * NCOL];
            let mut xt = [0u32; NCOL * NCOL];
            // `r_floor[row]` is the last non-zero row of `r` at or above `row`. Zero rows carry
            // nothing and are skipped.
            let mut r_floor = [0usize; N + 1];
            for row in 1..=N {
                if r[row - 1] != 0 {
                    r_floor[row] = row;
                    xr[row * NCOL + (N - row)] = r[row - 1];
                } else {
                    r_floor[row] = r_floor[row - 1];
                }
            }
            if r_floor[N] == 0 {
                emit(s);
                return;
            }
            let mut r_min = 0;
            while r_min < N {
                let row = r_min;
                r_min += 1;
                if r[row] != 0 {
                    break;
                }
            }
            let r_max = r_floor[N];
            for col in 1..=N - r_min {
                xs[r_floor[N - col] * NCOL + col] = s[col - 1];
            }
            for row in 1..=N {
                xt[r_floor[row] * NCOL + row - r_floor[row]] = 0;
            }
            for col in (N + 1 - r_min)..=N {
                x[col] = s[col - 1];
                let mut row = r_max;
                while row > 0 {
                    if col >= row {
                        xt[row * NCOL + col - row] = s[col - 1];
                        break;
                    }
                    row = r_floor[row - 1];
                }
            }
            let mut i = r_max;
            let mut j = N - i;
            let mut decrease = false;
            loop {
                let mut move_right = false;
                if j != 0 {
                    let index = i * NCOL + j;
                    let index_up = r_floor[i - 1] * NCOL + j;
                    // The entry on the same anti-diagonal in the next non-zero row up, namely row
                    // `r_floor[i - 1]` and column `j + i - r_floor[i - 1]`.
                    let index_up_right = r_floor[i - 1] * N + j + i;
                    let index_left = index - 1;
                    if i == r_min {
                        if decrease {
                            x[index] = x[index].wrapping_sub(1) & !(xt[index] | x[index_up_right]);
                            decrease = false;
                        } else {
                            x[index] = max_mask(
                                (xr[index] >> j).min(xs[index]),
                                xt[index] | x[index_up_right],
                            );
                        }
                        let x_index = x[index];
                        x[index_up] = xs[index].wrapping_sub(x_index);
                        if j >= r_min && (x[index_up] & xt[index - r_min]) != 0 {
                            if x_index != 0 {
                                decrease = true;
                            } else {
                                move_right = true;
                            }
                        } else {
                            xr[index_left] = xr[index].wrapping_sub(x_index << j);
                            xt[index_up_right] = xt[index] | x_index | x[index_up_right];
                            j -= 1;
                        }
                    } else {
                        if decrease {
                            x[index] = x[index].wrapping_sub(1) & !xt[index];
                            decrease = false;
                        } else {
                            x[index] = max_mask((xr[index] >> j).min(xs[index]), xt[index]);
                        }
                        let x_index = x[index];
                        xr[index_left] = xr[index].wrapping_sub(x_index << j);
                        xs[index_up] = xs[index].wrapping_sub(x_index);
                        xt[index_up_right] = xt[index] | x_index;
                        j -= 1;
                    }
                } else if i == r_min {
                    // The matrix is complete; it contributes if the leftover weight of the top
                    // row is disjoint from the first column.
                    if (xr[i * NCOL] & x[i]) == 0 {
                        xt[i] = xr[i * NCOL] | x[i];
                        xt[1..i].copy_from_slice(&x[1..i]);
                        let mut t = [0u32; N];
                        t.copy_from_slice(&xt[1..=N]);
                        emit(t);
                    }
                    move_right = true;
                } else {
                    let xt_index = xt[i * NCOL];
                    let xr_index = xr[i * NCOL];
                    if (xt_index & xr_index) != 0 {
                        move_right = true;
                    } else {
                        xt[r_floor[i - 1] * N + i] = xt_index | xr_index;
                        i = r_floor[i - 1];
                        j = N - i;
                    }
                }
                if move_right {
                    // Backtrack to the next non-zero entry that can still be decreased.
                    loop {
                        if i + j < N {
                            j += 1;
                        } else {
                            loop {
                                i += 1;
                                if i > r_max || r_floor[i] == i {
                                    break;
                                }
                            }
                            if i > r_max {
                                break;
                            }
                            j = 1;
                        }
                        if i > r_max || i + j > N {
                            break;
                        }
                        if x[i * NCOL + j] != 0 {
                            break;
                        }
                    }
                    if i > r_max || i + j > N {
                        break;
                    }
                    decrease = true;
                }
            }
        }
    };
}

define_for_each_term!(for_each_term_1, 1);
define_for_each_term!(for_each_term_2, 2);
define_for_each_term!(for_each_term_3, 3);
define_for_each_term!(for_each_term_4, 4);
define_for_each_term!(for_each_term_5, 5);
define_for_each_term!(for_each_term_6, 6);
define_for_each_term!(for_each_term_7, 7);
define_for_each_term!(for_each_term_8, 8);
define_for_each_term!(for_each_term_9, 9);
define_for_each_term!(for_each_term_10, 10);

/// The number of exponents an element of polynomial degree `degree` can have: $\xi_n$ has degree
/// $2^n - 1$, so only $\xi_1, \ldots, \xi_n$ with $2^n - 1 \leq$ `degree` can occur. This is at
/// most [`PPart::MAX_LEN`], since longer p-parts cannot be represented anyway.
fn length_for_degree(degree: i32) -> usize {
    let mut n = 0;
    while n < PPart::MAX_LEN && (1i64 << (n + 1)) - 1 <= i64::from(degree) {
        n += 1;
    }
    n
}

/// Call `emit` with every p-part $T$ such that $P(T)$ appears in $P(r) P(s)$ at the prime 2, with
/// multiplicity: equal terms may be emitted several times and should be summed mod 2.
///
/// $P(r) P(s)$ is the sum, over matrices $X$ with $\sum_j 2^j x_{ij} = r_i$ for each row and
/// $\sum_i x_{ij} = s_j$ for each column, of $P(T(X))$, where $T_n$ is the sum of the $n$th
/// anti-diagonal. The coefficient is a product of multinomial coefficients, one per anti-diagonal,
/// and is odd exactly when the entries of each anti-diagonal have pairwise disjoint binary digits.
/// [`PPartMultiplier`](super::PPartMultiplier) walks the matrices satisfying both sum conditions
/// and checks the coefficient of each one it completes, which in high degrees discards almost all
/// of them. Here each entry is chosen within the budgets left by $r$ and $s$ as a submask avoiding
/// the bits already used on its anti-diagonal, so every completed matrix contributes a term.
///
/// `degree` is an upper bound for the polynomial degree of the product, i.e. $\sum_i (2^i - 1)
/// (r_i + s_i)$. It only determines how long the exponent arrays have to be.
///
/// This is a port of `mul_packed_xi_v3` from L. Wu's `ext` crate
/// (<https://github.com/wulx02/ext>, MIT licensed).
pub(super) fn for_each_term(degree: i32, r: PPart, s: PPart, mut emit: impl FnMut(PPart)) {
    macro_rules! with_length {
        ($f:ident, $n:expr) => {{
            let r: [u32; $n] = std::array::from_fn(|i| r.get(i));
            let s: [u32; $n] = std::array::from_fn(|i| s.get(i));
            $f(r, s, |t| {
                // Pack once rather than going through `PPart::set` per entry. Every entry fits in
                // its field because the product has degree at most `PPart::MAX_DEGREE`.
                let mut bits = 0;
                for (i, &entry) in t.iter().enumerate() {
                    debug_assert!(entry <= PPart::max_entry(i));
                    bits |= (entry as u64) << PPart::shift(i);
                }
                emit(PPart::from_bits(bits))
            })
        }};
    }
    match length_for_degree(degree) {
        0 => emit(PPart::zero()),
        1 => with_length!(for_each_term_1, 1),
        2 => with_length!(for_each_term_2, 2),
        3 => with_length!(for_each_term_3, 3),
        4 => with_length!(for_each_term_4, 4),
        5 => with_length!(for_each_term_5, 5),
        6 => with_length!(for_each_term_6, 6),
        7 => with_length!(for_each_term_7, 7),
        8 => with_length!(for_each_term_8, 8),
        9 => with_length!(for_each_term_9, 9),
        _ => with_length!(for_each_term_10, 10),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::milnor_algebra::{PPartAllocation, PPartMultiplier};

    /// The terms of `P(r) P(s)` that survive mod 2, sorted, by both enumerations.
    fn both(degree: i32, r: PPart, s: PPart) -> (Vec<PPart>, Vec<PPart>) {
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
        let mut two_sided = Vec::new();
        for_each_term(degree, r, s, |t| two_sided.push(t));
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
        (reduce(two_sided), reduce(old))
    }

    #[test]
    fn length_for_degree_covers_xi() {
        assert_eq!(length_for_degree(0), 0);
        assert_eq!(length_for_degree(1), 1);
        assert_eq!(length_for_degree(2), 1);
        assert_eq!(length_for_degree(3), 2);
        assert_eq!(length_for_degree(6), 2);
        assert_eq!(length_for_degree(7), 3);
        assert_eq!(length_for_degree(PPart::MAX_DEGREE), PPart::MAX_LEN);
        assert_eq!(length_for_degree(i32::MAX), PPart::MAX_LEN);
    }

    #[test]
    fn small_products() {
        let p = PPart::from_slice;
        // Sq(1) Sq(1) = 0, Sq(1) Sq(2) = Sq(3), Sq(2) Sq(1) = Sq(3) + Sq(0, 1).
        assert_eq!(both(2, p(&[1]), p(&[1])).0, vec![]);
        assert_eq!(both(3, p(&[1]), p(&[2])).0, vec![p(&[3])]);
        let (new, old) = both(3, p(&[2]), p(&[1]));
        assert_eq!(new, old);
        assert_eq!(new.len(), 2);
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
            let degree = [r, s]
                .iter()
                .flat_map(|x| x.iter().enumerate())
                .map(|(i, e)| ((1 << (i + 1)) - 1) * e as i32)
                .sum();
            let (new, old) = both(degree, r, s);
            assert_eq!(new, old, "P{r:?} P{s:?}");
        }
    }
}
