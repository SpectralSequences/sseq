//! Dense linear algebra over the PID $\mathbb{F}_2[\tau]$, with polynomials packed
//! as `u128` bitmasks (bit $i$ = coefficient of $\tau^i$).
//!
//! Used to read the motivic Adams $E_2$ as an $\mathbb{F}_2[\tau]$-module: the
//! $\tau$-torsion orders are the non-unit invariant factors of $\delta$
//! ([`invariant_factors`], Smith normal form), and Massey-product coset membership
//! is decided by reducing modulo a submodule ([`reduce_mod`]).

/// Degree of `a` (`-1` for the zero polynomial).
pub fn deg(a: u128) -> i32 {
    if a == 0 {
        -1
    } else {
        127 - a.leading_zeros() as i32
    }
}

/// Polynomial product over $\mathbb{F}_2$ (carryless multiply).
fn mul(mut a: u128, b: u128) -> u128 {
    let mut r = 0;
    let mut shift = 0;
    while a != 0 {
        if a & 1 == 1 {
            r ^= b << shift;
        }
        a >>= 1;
        shift += 1;
    }
    r
}

/// Quotient $\lfloor a / b \rfloor$ over $\mathbb{F}_2[\tau]$ (the remainder is
/// dropped). `b` must be nonzero.
fn div(a: u128, b: u128) -> u128 {
    let db = deg(b);
    let (mut q, mut rem) = (0u128, a);
    while deg(rem) >= db {
        let sh = deg(rem) - db;
        q ^= 1u128 << sh;
        rem ^= b << sh;
    }
    q
}

/// Reduce the vector `target` modulo the $\mathbb{F}_2[\tau]$-submodule spanned by
/// `rows`. Row-reduces `rows` to an echelon form (a minimal-degree pivot per
/// column, cleared below via Euclidean division), then reduces `target` against
/// the pivots. The returned remainder is a canonical coset representative — zero
/// iff `target` lies in the submodule.
#[allow(clippy::needless_range_loop)]
pub fn reduce_mod(mut rows: Vec<Vec<u128>>, mut target: Vec<u128>) -> Vec<u128> {
    let ncols = target.len();
    let nrows = rows.len();
    let mut r = 0; // next pivot row
    for col in 0..ncols {
        if r >= nrows {
            break;
        }
        // Euclidean-reduce column `col` among rows[r..] until one row carries the
        // gcd there and the rest are zero in that column.
        loop {
            let mut piv = None;
            for i in r..nrows {
                if rows[i][col] != 0
                    && piv.is_none_or(|p: usize| deg(rows[i][col]) < deg(rows[p][col]))
                {
                    piv = Some(i);
                }
            }
            let Some(pi) = piv else { break };
            rows.swap(r, pi);
            let p = rows[r][col];
            let mut changed = false;
            for i in 0..nrows {
                if i != r && rows[i][col] != 0 {
                    let q = div(rows[i][col], p);
                    if q != 0 {
                        for j in 0..ncols {
                            rows[i][j] ^= mul(q, rows[r][j]);
                        }
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        if rows[r][col] != 0 {
            // Reduce target's entry in this pivot column modulo the pivot.
            let q = div(target[col], rows[r][col]);
            if q != 0 {
                for j in 0..ncols {
                    target[j] ^= mul(q, rows[r][j]);
                }
            }
            r += 1;
        }
    }
    target
}

/// The non-unit invariant factors (degree $\ge 1$) of `m` over
/// $\mathbb{F}_2[\tau]$, by Smith normal form. `m` is consumed.
///
/// Standard Euclidean SNF: pivot on the minimum-degree nonzero entry, clear its
/// row and column by division, and pull any lower-degree residual back into the
/// pivot until the pivot divides the remaining block; then recurse on the
/// complementary submatrix.
// The row/column clears index two rows (or columns) at once — `m[i][j]` against
// `m[r0][j]` — so index loops are clearer than split-borrow iterator gymnastics.
#[allow(clippy::needless_range_loop)]
pub fn invariant_factors(mut m: Vec<Vec<u128>>) -> Vec<u128> {
    let rows = m.len();
    let cols = m.first().map_or(0, Vec::len);
    let mut factors = Vec::new();
    let (mut r0, mut c0) = (0, 0);
    while r0 < rows && c0 < cols {
        // Pivot = minimum-degree nonzero entry of the active submatrix.
        let mut piv: Option<(usize, usize)> = None;
        for i in r0..rows {
            for j in c0..cols {
                if m[i][j] != 0 && piv.is_none_or(|(pi, pj)| deg(m[i][j]) < deg(m[pi][pj])) {
                    piv = Some((i, j));
                }
            }
        }
        let Some((pi, pj)) = piv else { break };
        m.swap(r0, pi);
        for row in &mut m {
            row.swap(c0, pj);
        }

        loop {
            let mut changed = false;
            let p = m[r0][c0];
            for i in 0..rows {
                if i != r0 && m[i][c0] != 0 {
                    let q = div(m[i][c0], p);
                    if q != 0 {
                        for j in 0..cols {
                            m[i][j] ^= mul(q, m[r0][j]);
                        }
                        changed = true;
                    }
                }
            }
            let p = m[r0][c0];
            for j in 0..cols {
                if j != c0 && m[r0][j] != 0 {
                    let q = div(m[r0][j], p);
                    if q != 0 {
                        for i in 0..rows {
                            m[i][j] ^= mul(q, m[i][c0]);
                        }
                        changed = true;
                    }
                }
            }
            // A nonzero residual left in the pivot row/column (degree below the
            // pivot): swap it in and keep reducing.
            let mut resid = None;
            for i in r0 + 1..rows {
                if m[i][c0] != 0 {
                    resid = Some((i, c0));
                }
            }
            for j in c0 + 1..cols {
                if m[r0][j] != 0 {
                    resid = Some((r0, j));
                }
            }
            if let Some((i, j)) = resid {
                m.swap(r0, i);
                if j != c0 {
                    for row in &mut m {
                        row.swap(c0, j);
                    }
                }
                changed = true;
            }
            if !changed {
                break;
            }
        }

        if deg(m[r0][c0]) >= 1 {
            factors.push(m[r0][c0]);
        }
        r0 += 1;
        c0 += 1;
    }
    factors
}
