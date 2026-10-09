//! Hash-free indexing of the Milnor basis.

use std::sync::Arc;

use super::{MilnorAlgebraInner, MilnorBasisElement, MilnorShape, PPart};
use crate::algebra::{Algebra, combinatorics};

/// Flat, contiguous storage for the "seqno" (hash-free index) computation.
///
/// Row-major with a fixed `width` (the number of ξ-degrees), so entry `(e, h)` lives at
/// `g[e * width + h]`; degrees `0..=max_degree` are populated.
pub(super) struct SeqnoTables {
    max_degree: i32,
    width: usize,
    g: Vec<usize>,
}

/// A borrowed view of the seqno tables, acquired once for a batch of lookups.
///
/// Holding this pins the revision of the tables that was current at acquisition, so
/// the per-lookup cost is the rank itself with no atomic. It is therefore valid only for
/// the degrees that revision covered: a ranker held across a concurrent grow will not see
/// the new degrees, and ranking one panics.
pub struct SeqnoRanker {
    tables: Arc<SeqnoTables>,
    xi: &'static [i32],
}

impl SeqnoRanker {
    /// The index of `P(p_part)` in the Milnor basis of `degree`.
    #[inline]
    pub fn rank(&self, p_part: PPart, degree: i32) -> usize {
        let t = &*self.tables;
        let w = t.width;
        debug_assert_eq!(
            degree,
            p_part
                .iter()
                .zip(self.xi)
                .map(|(r, &x)| r as i32 * x)
                .sum::<i32>(),
            "degree {degree} does not match the p-part {p_part:?}"
        );
        debug_assert!(
            degree <= t.max_degree,
            "degree {degree} exceeds seqno tables built to {}",
            t.max_degree
        );
        let mut cur_d = degree;
        let mut rank = 0;
        for h in (1..p_part.len()).rev() {
            let r = p_part.get(h) as i32;
            if r == 0 {
                continue;
            }
            let below = cur_d - r * self.xi[h];
            rank += t.g[cur_d as usize * w + h] - t.g[below as usize * w + h];
            cur_d = below;
        }
        rank
    }
}

impl<F: MilnorShape> MilnorAlgebraInner<F> {
    /// Whether seqno (hash-free index) can be used.
    pub(super) fn seqno_applicable(&self) -> bool {
        !F::HAS_EXTERIOR
            && self.p == fp::prime::TWO
            && !self.unstable_enabled
            && self.profile.is_trivial()
    }

    /// Index of `elt`, or `None` if it is not a basis element of the computed range.
    ///
    /// [`Self::seqno`] trusts its input, so this rejects an element with a Q part, a degree that
    /// does not match its p-part, or a degree beyond the tables.
    pub(super) fn try_seqno(&self, elt: &MilnorBasisElement) -> Option<usize> {
        let xi = combinatorics::xi_degrees(self.p);
        let degree: i32 = elt.p_part.iter().zip(xi).map(|(r, &x)| r as i32 * x).sum();
        let in_range = self
            .seqno_tables
            .load()
            .as_ref()
            .is_some_and(|t| (0..=t.max_degree).contains(&elt.degree));
        (elt.q_part == 0 && degree == elt.degree && in_range)
            .then(|| self.seqno(elt.p_part, elt.degree))
    }

    /// Build the flat SeqnoTables up to `max_degree`.
    ///
    /// Idempotent: if the stored tables already reach `max_degree` this returns immediately;
    /// otherwise it rebuilds the whole table and atomically swaps it in.
    pub fn compute_seqno_tables(&self, max_degree: i32) {
        assert!(self.seqno_applicable());
        assert!(
            (0..=PPart::MAX_DEGREE).contains(&max_degree),
            "seqno tables only supported for degrees 0..={}, got {max_degree}",
            PPart::MAX_DEGREE,
        );
        if let Some(t) = &*self.seqno_tables.load()
            && t.max_degree >= max_degree
        {
            return;
        }

        let xi = combinatorics::xi_degrees(self.prime());
        let width = xi.len();
        let rows = max_degree as usize + 1;

        let mut n = vec![0usize; rows * width];
        for e in 0..=max_degree {
            let base = e as usize * width;
            for m in 0..width {
                let without = if m == 0 {
                    (e == 0) as usize
                } else {
                    n[base + m - 1]
                };
                let with = if xi[m] <= e {
                    n[(e - xi[m]) as usize * width + m]
                } else {
                    0
                };
                n[base + m] = without + with;
            }
        }

        let mut g = vec![0usize; rows * width];
        for e in 0..=max_degree {
            let base = e as usize * width;
            for h in 1..width {
                let head = n[base + h - 1];
                let tail = if xi[h] <= e {
                    g[(e - xi[h]) as usize * width + h]
                } else {
                    0
                };
                g[base + h] = head + tail;
            }
        }

        let new_tables = Arc::new(SeqnoTables {
            max_degree,
            width,
            g,
        });
        self.seqno_tables.rcu(|current| match current.as_deref() {
            Some(t) if t.max_degree >= max_degree => current.clone(),
            _ => Some(new_tables.clone()),
        });
    }

    /// A handle to acquire seqno tables once for a batch of lookups.
    pub fn seqno_ranker(&self) -> SeqnoRanker {
        debug_assert!(self.seqno_applicable());
        SeqnoRanker {
            tables: self
                .seqno_tables
                .load_full()
                .expect("seqno tables not built; call compute_seqno_tables first"),
            xi: combinatorics::xi_degrees(self.prime()),
        }
    }

    /// The index of `P(p_part)` in the Milnor basis of `degree`.
    ///
    /// Computed hash-free from precomputed tables. Assumes seqno is applicable and that
    /// `p_part` is a genuine basis element (trimmed, in range) of `degree`.
    pub fn seqno(&self, p_part: PPart, degree: i32) -> usize {
        self.seqno_ranker().rank(p_part, degree)
    }
}
