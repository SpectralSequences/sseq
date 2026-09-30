pub type PPartEntry = u32;

/// The exponent sequence $(r_1, r_2, \ldots)$ of a Milnor basis element $P(r_1, r_2, \ldots)$,
/// bit-packed into a single `u64`.
///
/// Entry $r_{i+1}$ occupies `WIDTHS[i]` bits starting at bit `SHIFTS[i]` (both private). The
/// widths are forced by the degree bound: at $p = 2$ the internal degree of $P(R)$ is
/// $\sum_i r_i (2^i - 1)$ and every term is non-negative, so an element of degree at most
/// [`Self::MAX_DEGREE`] has $r_i \le \mathrm{MAX\\_DEGREE}/(2^i - 1)$. At an odd prime the same
/// argument bounds $r_i$ by that quantity divided by $q = 2(p-1)$, so the $p = 2$ widths are valid
/// for every prime and this type is prime-agnostic.
///
/// Trailing zeros are not represented: $P(2, 1)$ and $P(2, 1, 0)$ have the same packed value. That
/// is what makes the packed value a canonical key, and it makes [`Self::len`] the position of the
/// highest non-zero entry rather than a stored field.
///
/// # Invariant
///
/// Every entry fits in its field. This holds for any element of degree at most
/// [`Self::MAX_DEGREE`], which [`MilnorAlgebra`](super::MilnorAlgebra)'s
/// [`compute_basis`](crate::algebra::Algebra::compute_basis) enforces up front, so the packing can
/// never silently truncate. [`Self::set`] asserts it anyway, and [`Self::try_from_slice`]
/// reports failure instead of panicking for input that has not been through that gate.
///
/// The derived ordering compares the packed words. That is a total order and consistent with
/// equality, which is all a sorted table needs, but it is *not* lexicographic in the exponents:
/// entry `i` sits at [`Self::shift`]`(i)`, so `r_1` is the least significant field and therefore
/// the last tie-breaker. Callers that need the exponents ordered lexicographically must compare
/// [`Self::iter`] instead.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PPart(u64);

impl PPart {
    /// `FIELD_OF_BIT[b]` is the index of the entry owning bit `b`, letting [`Self::len`] turn a
    /// `leading_zeros` into an entry index without looping.
    const FIELD_OF_BIT: [u8; 64] = {
        let mut table = [0; 64];
        let mut i = 0;
        while i < Self::MAX_LEN {
            let mut b = Self::SHIFTS[i];
            while b < Self::SHIFTS[i + 1] {
                table[b as usize] = i as u8;
                b += 1;
            }
            i += 1;
        }
        table
    };
    /// The largest internal degree whose exponent sequences are guaranteed to fit.
    ///
    /// This is the largest bound for which the field widths sum to at most 64. It is far beyond
    /// anything currently reachable.
    pub const MAX_DEGREE: i32 = 2045;
    /// The number of entries that can be stored. This equals `fp`'s `MAX_MULTINOMIAL_LEN`.
    pub const MAX_LEN: usize = 10;
    /// `SHIFTS[i]` is the bit offset of entry `i`; `SHIFTS[MAX_LEN]` is the total width, 64.
    const SHIFTS: [u32; Self::TABLE_LEN] = {
        let mut shifts = [0; Self::TABLE_LEN];
        let mut i = 0;
        while i < Self::MAX_LEN {
            shifts[i + 1] = shifts[i] + Self::WIDTHS[i];
            i += 1;
        }
        shifts
    };
    /// The length of the layout tables.
    ///
    /// This is [`Self::MAX_LEN`] rounded up to a power of two so that [`Self::entry`] can mask its
    /// index instead of bounds-checking it; entries at or past `MAX_LEN` are given width 0, so they
    /// read as zero.
    const TABLE_LEN: usize = 16;
    /// `WIDTHS[i]` is the number of bits holding $r_{i+1}$: the number of bits needed to represent
    /// `MAX_DEGREE / (2^(i+1) - 1)`.
    const WIDTHS: [u32; Self::TABLE_LEN] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 1, 0, 0, 0, 0, 0, 0];

    /// The largest value entry `i` can hold.
    pub const fn max_entry(i: usize) -> PPartEntry {
        ((1u64 << Self::WIDTHS[i]) - 1) as PPartEntry
    }

    /// The number of bits holding entry `i`.
    ///
    /// Together with [`Self::shift`] this lets callers build a mask over [`Self::bits`] directly,
    /// e.g. to test many entries in one comparison.
    pub const fn width(i: usize) -> u32 {
        Self::WIDTHS[i]
    }

    /// The bit offset of entry `i` within [`Self::bits`].
    pub const fn shift(i: usize) -> u32 {
        Self::SHIFTS[i]
    }

    const fn mask(i: usize) -> u64 {
        ((1u64 << Self::WIDTHS[i]) - 1) << Self::SHIFTS[i]
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    /// The raw packed value.
    ///
    /// Two exponent sequences are equal exactly when their bits are, so this is a complete hash
    /// key, and it can be compared against a packed mask in one operation (see
    /// `MilnorSubalgebra::packed_signature` in `ext`).
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Reinterpret a raw packed value.
    ///
    /// Callers that assemble entries by shifting must uphold the type invariant themselves: each
    /// entry must lie within its field, which holds for any element of degree at most
    /// [`Self::MAX_DEGREE`]. This exists so hot loops can accumulate into a plain `u64` and store
    /// once, rather than read-modify-write through [`Self::set`] per entry.
    pub(crate) const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Entry `i`, for `i < TABLE_LEN`, with no bounds check.
    #[inline]
    pub(super) const fn entry(self, i: usize) -> PPartEntry {
        debug_assert!(i < Self::TABLE_LEN);
        let i = i & (Self::TABLE_LEN - 1);
        ((self.0 >> Self::SHIFTS[i]) & ((1 << Self::WIDTHS[i]) - 1)) as PPartEntry
    }

    /// Entry `i`, or 0 if `i` is past the end. Accepts any index.
    #[inline]
    pub const fn get(self, i: usize) -> PPartEntry {
        if i >= Self::MAX_LEN { 0 } else { self.entry(i) }
    }

    /// Set entry `i` to `v`.
    ///
    /// # Panics
    ///
    /// If `i >= MAX_LEN`, or `v` does not fit in entry `i`.
    #[inline]
    pub fn set(&mut self, i: usize, v: PPartEntry) {
        assert!(i < Self::MAX_LEN, "p-part index {i} out of range");
        assert!(
            v <= Self::max_entry(i),
            "p-part entry {v} does not fit in the {} bits at index {i}",
            Self::WIDTHS[i],
        );
        self.0 = (self.0 & !Self::mask(i)) | ((v as u64) << Self::SHIFTS[i]);
    }

    /// The number of entries up to and including the last non-zero one.
    #[inline]
    pub const fn len(self) -> usize {
        if let Some(idx) = self.0.highest_one() {
            Self::FIELD_OF_BIT[idx as usize] as usize + 1
        } else {
            0
        }
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Zero every entry from `n` onwards, i.e. the packed form of `self[..n]`.
    #[inline]
    pub const fn truncate(self, n: usize) -> Self {
        if n >= Self::MAX_LEN {
            self
        } else {
            Self(self.0 & ((1 << Self::SHIFTS[n]) - 1))
        }
    }

    /// Iterate over the entries in order from lowest to highest index.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = PPartEntry> + ExactSizeIterator {
        (0..self.len()).map(move |i| self.get(i))
    }

    /// Pack `entries`, returning `None` if they do not fit. Use this for anything derived from
    /// user input; use [`Self::from_slice`] when the degree bound already guarantees a fit.
    pub fn try_from_slice(entries: &[PPartEntry]) -> Option<Self> {
        let mut result = Self::zero();
        for (i, &entry) in entries.iter().enumerate() {
            // A zero past the end is just padding, which the packed form drops anyway.
            if entry == 0 {
                continue;
            }
            if i >= Self::MAX_LEN || entry > Self::max_entry(i) {
                return None;
            }
            result.set(i, entry);
        }
        Some(result)
    }

    /// Pack `entries`, panicking if they do not fit.
    pub fn from_slice(entries: &[PPartEntry]) -> Self {
        Self::try_from_slice(entries).unwrap_or_else(|| {
            panic!(
                "p-part {entries:?} exceeds the degree {} bound",
                Self::MAX_DEGREE
            )
        })
    }
}

impl FromIterator<PPartEntry> for PPart {
    fn from_iter<I: IntoIterator<Item = PPartEntry>>(iter: I) -> Self {
        let mut result = Self::zero();
        for (i, entry) in iter.into_iter().enumerate() {
            if entry != 0 {
                result.set(i, entry);
            }
        }
        result
    }
}

impl std::fmt::Debug for PPart {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::combinatorics;

    /// The packing is only sound because each field is wide enough for every entry that can occur
    /// at degree at most `MAX_DEGREE`. Check that against the $\xi$-degrees directly, so that
    /// changing `MAX_DEGREE` or `WIDTHS` without the other fails loudly.
    #[test]
    fn ppart_widths_cover_max_degree() {
        let xi_degrees = combinatorics::xi_degrees(fp::prime::TWO);
        for (i, &xi_degree) in xi_degrees.iter().enumerate().take(PPart::MAX_LEN) {
            // deg P(R) = sum_i r_i (2^i - 1) with non-negative terms, so r_i <= deg / (2^i - 1).
            let bound = PPart::MAX_DEGREE / xi_degree;
            assert!(
                bound <= PPart::max_entry(i) as i32,
                "entry {i} needs to hold {bound} but only holds up to {}",
                PPart::max_entry(i),
            );
        }
        // There is no entry beyond `MAX_LEN` to store: the xi-degree table itself stops there, so
        // `compute_ppart` cannot produce a longer p-part. If `fp` ever raises
        // `MAX_MULTINOMIAL_LEN`, this fires and `WIDTHS` has to be revisited.
        assert_eq!(xi_degrees.len(), PPart::MAX_LEN);
        // ... and the layout uses the whole word, so `MAX_DEGREE` is as large as it can be.
        assert_eq!(
            PPart::shift(PPart::MAX_LEN - 1) + PPart::width(PPart::MAX_LEN - 1),
            64
        );
    }

    #[test]
    fn ppart_accessors() {
        let mut p = PPart::from_slice(&[3, 0, 5]);
        assert_eq!(p.len(), 3);
        assert_eq!(p.iter().collect::<Vec<_>>(), vec![3, 0, 5]);
        assert_eq!(p.get(1), 0);
        assert_eq!(p.get(2), 5);
        // Reading past the end is zero, not a panic.
        assert_eq!(p.get(7), 0);
        assert_eq!(p.get(PPart::MAX_LEN), 0);

        // Trailing zeros are not represented, so they do not affect equality, length or hashing.
        assert_eq!(PPart::from_slice(&[3, 0, 5, 0, 0]), p);
        assert_eq!(PPart::from_slice(&[]), PPart::zero());
        assert_eq!(PPart::from_slice(&[0, 0]), PPart::zero());
        assert_eq!(PPart::zero().len(), 0);
        assert!(PPart::zero().is_empty());

        assert_eq!(p.truncate(2), PPart::from_slice(&[3]));
        assert_eq!(p.truncate(0), PPart::zero());
        assert_eq!(p.truncate(PPart::MAX_LEN + 3), p);

        p.set(1, 7);
        assert_eq!(p, PPart::from_slice(&[3, 7, 5]));
        p.set(2, 0);
        assert_eq!(p, PPart::from_slice(&[3, 7]));
    }

    #[test]
    fn ppart_rejects_out_of_range() {
        // Too many entries, and an entry too large for its field.
        assert_eq!(PPart::try_from_slice(&[1; PPart::MAX_LEN + 1]), None);
        assert_eq!(PPart::try_from_slice(&[0, PPart::max_entry(1) + 1]), None);
        // ... but a zero past the end is only padding.
        let mut padded = vec![0; PPart::MAX_LEN + 4];
        padded[0] = 2;
        assert_eq!(
            PPart::try_from_slice(&padded),
            Some(PPart::from_slice(&[2]))
        );
    }

    #[test]
    #[should_panic(expected = "does not fit")]
    fn ppart_set_out_of_range_panics() {
        PPart::zero().set(0, PPart::max_entry(0) + 1);
    }
}
