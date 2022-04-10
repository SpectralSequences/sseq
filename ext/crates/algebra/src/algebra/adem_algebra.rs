//! The Steenrod algebra using the Adem basis.

use std::cmp::Ordering;
use std::fmt;
use std::sync::Mutex;

use itertools::Itertools;
use rustc_hash::FxHashMap as HashMap;

use fp::prime::{BinomialIterator, BitflagIterator, ValidPrime};
use fp::vector::{FpVector, SliceMut};
use once::OnceVec;

use crate::algebra::combinatorics::{self, MAX_XI_TAU};
use crate::algebra::{Algebra, Bialgebra, GeneratedAlgebra, UnstableAlgebra};

#[cfg(doc)]
use crate::algebra::SteenrodAlgebra;

/// An algebra that can be viewed as an Adem algebra.
///
/// This is here so that the Python bindings can use modules defined for AdemAlgebraT
/// with their version of [`SteenrodAlgebra`].
///
/// In order for things to work `AdemAlgebraT` cannot implement [`Algebra`].
/// Otherwise, the algebra enum for our bindings will see an implementation clash.
pub trait AdemAlgebraT: Send + Sync + Algebra {
    fn adem_algebra(&self) -> &AdemAlgebra;
}

impl AdemAlgebraT for AdemAlgebra {
    fn adem_algebra(&self) -> &AdemAlgebra {
        &*self
    }
}

/// An Adem basis element for the Steenrod algebra.
///
/// To encode an element
/// $$
///     \beta^{\varepsilon_0} P^{i_0}\
///     \beta^{\varepsilon_1} P^{i_1}
///     \cdots
///     \beta^{\varepsilon_n} P^{i_n}
///     \beta^{\varepsilon_{n+1}},
/// $$
/// we set
/// $$ \begin{aligned}
/// \mathtt{ps}         &= [i_0, i_1, \ldots, i_n] \\\\
/// \mathtt{bocksteins} &= 000\cdots0\varepsilon_{n+1} \varepsilon_n \cdots \varepsilon_0
/// \end{aligned} $$
// #[derive(RustcDecodable, RustcEncodable)]
#[derive(Debug, Clone)]
pub struct AdemBasisElement {
    /// The degree of the element.
    pub degree: i32,
    /// The excess (i.e., distance from this element being minimally admissible)
    /// of this basis element.
    ///
    /// See <https://doc.sagemath.org/html/en/reference/algebras/sage/algebras/steenrod/steenrod_algebra.html#sage.algebras.steenrod.steenrod_algebra.SteenrodAlgebra_generic.Element.excess>.
    ///
    /// This field is only used in `unstable_enabled` mode.
    pub excess: i32,
    /// A bitset of which $\beta$ Bocksteins are in the element's decomposition.
    pub bocksteins: u32,
    /// A list of which Steenrod powers are in the element's decompositions.
    pub ps: Vec<u32>,
    /// Whether to denote the generators as powers $P^i$ or squares $Sq^i$ when using string formatting;
    /// `true` denotes the former.
    pub p_or_sq: bool,
}

/// A Steenrod power $P^i$, or a Bockstein $\beta^\varepsilon$.
#[derive(Debug)]
pub enum PorBockstein {
    P(u32),
    Bockstein(bool),
}

impl AdemBasisElement {
    /// Returns an iterator over the element's decomposition.
    ///
    /// This returns alternating Bocksteins and Ps, but skipping any
    /// `\beta^0` factors.
    fn iter_filtered(&self) -> impl Iterator<Item = PorBockstein> + '_ {
        BitflagIterator::new(self.bocksteins as u64)
            .map(PorBockstein::Bockstein)
            .interleave(self.ps.iter().map(|b| PorBockstein::P(*b)))
            .filter(|b| !matches!(b, PorBockstein::Bockstein(false)))
    }
}

impl PartialEq for AdemBasisElement {
    fn eq(&self, other: &Self) -> bool {
        self.ps == other.ps && self.bocksteins == other.bocksteins
    }
}

impl Eq for AdemBasisElement {}

impl std::hash::Hash for AdemBasisElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bocksteins.hash(state);
        self.ps.hash(state);
    }
}

impl fmt::Display for AdemBasisElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let p_or_sq = if self.p_or_sq { "P" } else { "Sq" };
        let result = self
            .iter_filtered()
            .map(|e| match e {
                PorBockstein::P(exp) => format!("{}{}", p_or_sq, exp),
                PorBockstein::Bockstein(_) => "b".to_string(),
            })
            .format(" ");

        write!(f, "{}", result)?;
        Ok(())
    }
}

impl PartialOrd for AdemBasisElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.excess.partial_cmp(&other.excess)
    }
}

impl Ord for AdemBasisElement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.excess.cmp(&other.excess)
    }
}

/// Shifts a `Vec`'s elements back by `offset`.
///
/// # Safety
///
/// This function leaves `v` in an invalid state, since `ptr` does not point to
/// the beginning of an allocation after this function completes. Operations
/// which may trigger resizing *must* be avoided, since the allocator will
/// otherwise get extremely confused.
///
/// The sum of `offset`s to calls to this function for a particular `v` must
/// always be positive (i.e., to avoid pointing before the allocation), less
/// than the `v`'s capacity (to avoid pointing after), and zero when `v` is
/// destroyed or resized (for the reasons described above)
unsafe fn shift_vec<T>(v: &mut Vec<T>, offset: isize) {
    let ptr = v.as_ptr();
    let len = v.len();
    let cap = v.capacity();
    let w = std::mem::replace(
        v,
        Vec::from_raw_parts(
            (ptr as *mut T).offset(offset),
            (len as isize - offset) as usize,
            (cap as isize - offset) as usize,
        ),
    );
    std::mem::forget(w);
}

/// An [`Algebra`] implementing the Steenrod algebra, using the Adem basis.
pub struct AdemAlgebra {
    p: ValidPrime,
    generic: bool,
    unstable_enabled: bool,
    lock: Mutex<()>,

    even_basis_table: OnceVec<Vec<AdemBasisElement>>,
    /// degree -> index -> AdemBasisElement
    basis_table: OnceVec<Vec<AdemBasisElement>>,
    /// degree -> AdemBasisElement -> index
    basis_element_to_index_map: OnceVec<HashMap<AdemBasisElement, usize>>,
    /// degree -> first square -> admissible sequence idx -> result
    multiplication_table: OnceVec<Vec<Vec<FpVector>>>,
    excess_table: OnceVec<Vec<usize>>,
}

impl fmt::Display for AdemAlgebra {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AdemAlgebra(p={})", self.prime())
    }
}

impl Algebra for AdemAlgebra {
    fn prefix(&self) -> &str {
        "adem"
    }

    fn magic(&self) -> u32 {
        if self.unstable_enabled {
            1 + ((*self.prime() as u32) << 16)
        } else {
            (*self.prime() as u32) << 16
        }
    }

    fn prime(&self) -> ValidPrime {
        self.p
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        let mut products = Vec::with_capacity(4);
        let max_degree = if self.generic {
            products.push((
                "a_0".to_string(),
                AdemBasisElement {
                    degree: 1,
                    bocksteins: 1,
                    excess: 0,
                    ps: vec![],
                    p_or_sq: *self.prime() != 2,
                },
            ));
            products.push((
                "h_0".to_string(),
                AdemBasisElement {
                    degree: (2 * (*self.prime()) - 2) as i32,
                    bocksteins: 0,
                    excess: 0,
                    ps: vec![1],
                    p_or_sq: *self.prime() != 2,
                },
            ));
            (2 * (*self.prime()) - 2) as i32
        } else {
            for i in 0..4 {
                let degree = 1 << i; // degree is 2^hi
                let ps = vec![degree as u32];
                products.push((
                    format!("h_{}", i),
                    AdemBasisElement {
                        degree,
                        bocksteins: 0,
                        excess: 0,
                        ps,
                        p_or_sq: *self.prime() != 2,
                    },
                ));
            }
            1 << 3
        };

        self.compute_basis(max_degree);
        products
            .into_iter()
            .map(|(name, b)| (name, b.degree, self.basis_element_to_index(&b)))
            .collect()
    }

    fn compute_basis(&self, max_degree: i32) {
        let _lock = self.lock.lock().unwrap();

        let next_degree = self.basis_table.len() as i32;
        if max_degree < next_degree {
            return;
        }

        if self.generic {
            self.generate_basis_generic(next_degree, max_degree);
            self.generate_basis_element_to_index_map(next_degree, max_degree);
            self.generate_multiplication_table_generic(next_degree, max_degree);
        } else {
            self.generate_basis2(next_degree, max_degree);
            self.generate_basis_element_to_index_map(next_degree, max_degree);
            self.generate_multiplication_table_2(next_degree, max_degree);
        }

        if self.unstable_enabled {
            self.generate_excess_table(max_degree);
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < 0 {
            0
        } else {
            self.basis_table[degree as usize].len()
        }
    }

    fn multiply_basis_elements(
        &self,
        result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r_index: usize,
        s_degree: i32,
        s_index: usize,
    ) {
        self.multiply_inner(
            result,
            coeff,
            r_degree,
            r_index,
            s_degree,
            s_index,
            i32::MAX,
            false,
        );
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        format!("{}", self.basis_element_from_index(degree, idx))
    }

    fn basis_element_from_string(&self, mut elt: &str) -> Option<(i32, usize)> {
        use crate::steenrod_parser::{digits, p_or_sq};
        use nom::sequence::preceded;

        let q = self.q();

        let mut bocksteins = 0;
        let mut ps = Vec::new();
        let mut degree = 0;
        let mut bockstein_count = 0;

        loop {
            if elt.starts_with('b') {
                elt = &elt[std::cmp::min(2, elt.len())..];
                degree += 1;
                bocksteins += 1 << bockstein_count;
            }
            if elt.is_empty() {
                break;
            }
            let (rem, sqn) = preceded(p_or_sq, digits)(elt).ok()?;
            elt = rem;

            ps.push(sqn);
            degree += sqn as i32 * q;

            bockstein_count += 1;
        }

        self.compute_basis(degree);
        Some((
            degree,
            self.basis_element_to_index(&AdemBasisElement {
                ps,
                bocksteins,
                excess: 0,
                degree,
                p_or_sq: self.generic,
            }),
        ))
    }
}

impl GeneratedAlgebra for AdemAlgebra {
    fn generator_to_string(&self, degree: i32, _idx: usize) -> String {
        if self.generic {
            if degree == 1 {
                "b".to_string()
            } else {
                format!("P{}", degree as u32 / (2 * (*self.prime()) - 2))
            }
        } else {
            format!("Sq{}", degree)
        }
    }

    fn generators(&self, degree: i32) -> Vec<usize> {
        let p = *self.prime();
        if degree == 0 {
            return vec![];
        }
        if self.generic {
            if degree == 1 {
                return vec![0];
            }
            // Test if degree is q*p^k.
            let mut temp_degree = degree as u32;
            if temp_degree % (2 * (p - 1)) != 0 {
                return vec![];
            }
            temp_degree /= 2 * (p - 1);
            while temp_degree % p == 0 {
                temp_degree /= p;
            }
            if temp_degree != 1 {
                return vec![];
            }
            let idx = self.basis_element_to_index(&AdemBasisElement {
                degree,
                excess: 0,
                bocksteins: 0,
                ps: vec![degree as u32 / (2 * p - 2)],
                p_or_sq: *self.prime() != 2,
            });
            return vec![idx];
        } else {
            // I guess we're assuming here that not generic ==> p == 2. There's probably tons of places we assume that though.
            if degree.count_ones() != 1 {
                return vec![];
            }
            let idx = self.basis_element_to_index(&AdemBasisElement {
                degree,
                excess: 0,
                bocksteins: 0,
                ps: vec![degree as u32],
                p_or_sq: *self.prime() != 2,
            });
            return vec![idx];
        }
    }

    fn decompose_basis_element(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        if self.generic {
            self.decompose_basis_element_generic(degree, idx)
        } else {
            self.decompose_basis_element_2(degree, idx)
        }
    }

    /// We return Adem relations $b^2 = 0$, $P^i P^j = \cdots$ for $i < pj$, and $P^i b P^j = \cdots$ for $i < pj + 1$. It suffices to check these because
    /// they generate all relations.
    fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> {
        if self.generic && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }

        let p = self.prime();
        let inadmissible_pairs = combinatorics::inadmissible_pairs(p, self.generic, degree);
        let mut result = Vec::new();

        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation
            let first_sq = self.beps_pn(0, x);
            let second_sq = self.beps_pn(b, y);
            relation.push((*p - 1, first_sq, second_sq));
            for e1 in 0..=b {
                let e2 = b - e1;
                // e1 and e2 determine where a bockstein shows up.
                // e1 determines if a bockstein shows up in front
                // e2 determines if a bockstein shows up in middle
                // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                for j in 0..=x / *p {
                    let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                    if c == 0 {
                        continue;
                    }
                    let idx = self.basis_element_to_index(&AdemBasisElement {
                        degree,
                        excess: 0,
                        ps: if j == 0 {
                            vec![(x + y) as u32]
                        } else {
                            vec![(x + y - j) as u32, j as u32]
                        },
                        bocksteins: e1 as u32 | ((e2 as u32) << 1),
                        p_or_sq: *self.prime() != 2,
                    });
                    relation.push((c as u32, (degree, idx), (0, 0)));
                }
            }
            result.push(relation);
        }
        result
    }
}

impl UnstableAlgebra for AdemAlgebra {
    fn dimension_unstable(&self, degree: i32, excess: i32) -> usize {
        if degree < 0 || excess < 0 {
            0
        } else if excess < degree {
            self.excess_table[degree as usize][excess as usize]
        } else {
            self.basis_table[degree as usize].len()
        }
    }

    fn multiply_basis_elements_unstable(
        &self,
        mut result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r_index: usize,
        s_degree: i32,
        s_index: usize,
        excess: i32,
    ) {
        self.multiply_inner(
            result.copy(),
            coeff,
            r_degree,
            r_index,
            s_degree,
            s_index,
            excess,
            true,
        );
    }
}
// static void AdemAlgebra__initializeFields(AdemAlgebraInternal *algebra, uint p, bool generic, bool unstable);
// uint AdemAlgebra__generateName(AdemAlgebra *algebra); // defined in adem_io
impl AdemAlgebra {
    /// Constructs a new [`AdemAlgebra`].
    // TODO: what do these argument names mean?
    pub fn new(p: ValidPrime, generic: bool, unstable_enabled: bool) -> Self {
        let even_basis_table = OnceVec::new();
        let basis_table = OnceVec::new();
        let basis_element_to_index_map = OnceVec::new();
        let multiplication_table = OnceVec::new();
        let excess_table = OnceVec::new();
        Self {
            p,
            generic,
            lock: Mutex::new(()),
            unstable_enabled,
            even_basis_table,
            basis_table,
            basis_element_to_index_map,
            multiplication_table,
            excess_table,
        }
    }

    pub fn generic(&self) -> bool {
        self.generic
    }

    pub fn q(&self) -> i32 {
        if self.generic {
            2 * (*self.prime() as i32 - 1)
        } else {
            1
        }
    }

    fn generate_basis_even(&self, mut next_degree: i32, max_degree: i32) {
        if next_degree == 0 {
            self.even_basis_table.push(vec![AdemBasisElement {
                degree: 0,
                excess: 0,
                bocksteins: 0,
                ps: vec![],
                p_or_sq: *self.prime() != 2,
            }]);
            next_degree += 1;
        }

        for n in next_degree..=max_degree {
            self.generate_basis_even_degreen(n);
        }
    }

    fn generate_basis_even_degreen(&self, n: i32) {
        let p = *self.prime() as i32;
        let mut basis = Vec::new();
        // Put Sqn into the list.
        basis.push(AdemBasisElement {
            degree: n,
            excess: n,
            bocksteins: if self.generic {
                u32::max_value() << 2
            } else {
                0
            },
            ps: vec![n as u32],
            p_or_sq: *self.prime() != 2,
        });

        // last = last term. We append (last,) to the end of
        // elements of degree n - last whose own last square is
        // at least p * last.
        // In order for this to be possible, this means that p last <= n - last,
        // or (p+1) * last <= n or last <= n/(p+1). We order the squares in decreasing
        // order of their last element so that as we walk over the previous basis
        // when we find a square whose end is too small, we can break.
        for last in (1..=n / (p + 1)).rev() {
            let previous_basis = &self.even_basis_table[(n - last) as usize];
            for prev_elt in previous_basis {
                let prev_elt_p_len = prev_elt.ps.len();
                let old_last_sq = prev_elt.ps[prev_elt_p_len - 1] as i32;
                if old_last_sq < p * last {
                    break;
                }
                // Write new basis element to basis element buffer

                let degree = prev_elt.degree + last;
                let excess = prev_elt.excess - (p - 1) * last;
                // We're using bocksteins as a bit mask:
                // A bit in bocksteins shall be set if it's illegal for a bockstein to occur there.
                let mut bocksteins = prev_elt.bocksteins;
                if self.generic {
                    bocksteins |= if old_last_sq == p * last {
                        1 << prev_elt_p_len
                    } else {
                        0
                    };
                    bocksteins &= !(1 << (prev_elt_p_len + 1));
                }
                let mut ps: Vec<u32> = Vec::with_capacity(prev_elt_p_len + 1);
                ps.extend_from_slice(&prev_elt.ps);
                ps.push(last as u32);
                basis.push(AdemBasisElement {
                    degree,
                    excess,
                    bocksteins,
                    ps,
                    p_or_sq: *self.prime() != 2,
                });
            }
        }
        self.even_basis_table.push(basis);
    }

    fn generate_basis2(&self, next_degree: i32, max_degree: i32) {
        self.generate_basis_even(next_degree, max_degree);
        for n in next_degree..=max_degree {
            let table = &self.even_basis_table[n as usize];
            // Sorting breaks the algorithm above.
            let mut new_table = table.clone();
            if self.unstable_enabled {
                new_table.sort();
            }
            self.basis_table.push(new_table);
        }
    }

    // Our approach is to pick the bocksteins and the P's separately and merge.
    fn generate_basis_generic(&self, next_degree: i32, max_degree: i32) {
        self.generate_basis_even(next_degree, max_degree);
        for n in next_degree..=max_degree {
            self.generate_basis_generic_degreen(n);
        }
    }

    // Now handle the bocksteins.
    // We have our Ps in even_basis_table and they contain in their bockstein field
    // a bit flag that indicates where bocksteins are allowed to go.
    #[allow(non_snake_case)]
    fn generate_basis_generic_degreen(&self, n: i32) {
        let p = *self.prime() as i32;
        let q = 2 * (p - 1);
        let residue = n % q;
        let mut basis: Vec<AdemBasisElement> = Vec::new();
        // First we need to know how many bocksteins we'll use so we know how much degree
        // to assign to the Ps. The Ps all have degree divisible by q=2p-2, so num_bs needs to
        // be congruent to degree mod q.
        let num_bs_bound = std::cmp::min(MAX_XI_TAU, (n + 1) as usize);
        for num_bs in (residue as usize..num_bs_bound).step_by(q as usize) {
            let P_deg = (n as usize - num_bs) / q as usize;
            // AdemBasisElement_list P_list
            let even_basis = &self.even_basis_table[P_deg];
            for i in (0..even_basis.len()).rev() {
                let P = &even_basis[i];
                // We pick our P first.
                if P.ps.len() + 1 < num_bs {
                    // Not enough space to fit the bs.
                    continue; // Ps ordered in descending length, so none of the later ones will have space either
                }
                for bocksteins in BinomialIterator::new(num_bs) {
                    if 32 - bocksteins.leading_zeros() > P.ps.len() as u32 + 1 {
                        // Too large of a b. We sorted the Ps in descending length order so we can break now.
                        break;
                    }
                    // P->bocksteins contains 1 in locations where the sequence is "just barely admissible" and so
                    // adding a bockstein would make it inadmissible.
                    if bocksteins & P.bocksteins != 0 {
                        continue;
                    }
                    // Okay, everything's good with this bocksteins, P pair so let's add it to our basis.
                    // Write new basis element to basis element buffer
                    let degree = n;
                    let mut excess = 2 * P.excess; // Ps contribute 2 to excess
                    excess += (bocksteins & 1) as i32; // leading bockstein increases excess by 1
                    let nonleading_bocksteins = bocksteins & !1;
                    excess -= nonleading_bocksteins.count_ones() as i32; // remaining bocksteins reduce excess by 1
                    let ps = P.ps.clone();
                    basis.push(AdemBasisElement {
                        degree,
                        excess,
                        bocksteins,
                        ps,
                        p_or_sq: *self.prime() != 2,
                    });
                    if num_bs == 0 {
                        break;
                    }
                }
            }
        }
        if self.unstable_enabled {
            basis.sort();
        }
        self.basis_table.push(basis);
    }

    fn generate_basis_element_to_index_map(&self, next_degree: i32, max_degree: i32) {
        for n in next_degree..=max_degree {
            let basis = &self.basis_table[n as usize];
            let mut map = HashMap::default();
            map.reserve(basis.len());
            for (i, basis) in basis.iter().enumerate() {
                map.insert(basis.clone(), i);
            }
            self.basis_element_to_index_map.push(map);
        }
    }

    pub fn basis_element_from_index(&self, degree: i32, idx: usize) -> &AdemBasisElement {
        &self.basis_table[degree as usize][idx]
    }

    pub fn try_basis_element_to_index(&self, elt: &AdemBasisElement) -> Option<usize> {
        self.basis_element_to_index_map[elt.degree as usize]
            .get(elt)
            .copied()
    }

    pub fn basis_element_to_index(&self, elt: &AdemBasisElement) -> usize {
        self.try_basis_element_to_index(elt)
            .unwrap_or_else(|| panic!("Didn't find element: {:?}", elt))
    }

    fn tail_of_basis_element_to_index(
        &self,
        elt: &mut AdemBasisElement,
        idx: u32,
        q: u32,
    ) -> usize {
        let degree = elt.degree;
        let bocksteins = elt.bocksteins;
        for i in 0..idx as usize {
            elt.degree -= (q * elt.ps[i] + (elt.bocksteins & 1)) as i32;
            elt.bocksteins >>= 1;
        }
        unsafe {
            shift_vec(&mut elt.ps, idx as isize);
        }
        let result = self.basis_element_to_index(elt);
        unsafe {
            shift_vec(&mut elt.ps, -(idx as isize));
        }
        elt.degree = degree;
        elt.bocksteins = bocksteins;
        result
    }

    fn generate_multiplication_table_2(&self, mut next_degree: i32, max_degree: i32) {
        // degree -> first_square -> admissibile sequence idx -> result vector
        if next_degree == 0 {
            self.multiplication_table.push(Vec::new());
            next_degree += 1;
        }

        for n in next_degree..=max_degree {
            let mut table: Vec<Vec<FpVector>> = Vec::with_capacity((n + 1) as usize);
            table.push(Vec::with_capacity(0));
            for x in 1..=n {
                let dimension = self.dimension(n - x);
                table.push(Vec::with_capacity(dimension));
            }
            for x in (1..=n).rev() {
                for idx in 0..self.dimension(n - x) {
                    let res = self.generate_multiplication_table_2_step(&table, n, x, idx);
                    table[x as usize].push(res);
                }
            }
            self.multiplication_table.push(table);
        }
    }

    fn generate_multiplication_table_2_step(
        &self,
        table: &[Vec<FpVector>],
        n: i32,
        x: i32,
        idx: usize,
    ) -> FpVector {
        let output_dimension = self.dimension(n);
        let mut result = FpVector::new(self.prime(), output_dimension);
        let cur_basis_elt = self.basis_element_from_index(n - x, idx);
        let x = x as u32;
        let mut working_elt = cur_basis_elt.clone();

        // Be careful to deal with the case that cur_basis_elt has length 0
        // If the length is 0 or the sequence is already admissible, we can just write a 1 in the answer
        // and continue.
        if cur_basis_elt.ps.is_empty() || x >= 2 * cur_basis_elt.ps[0] {
            working_elt.ps.insert(0, x);
            working_elt.degree = n;
            let out_idx = self.basis_element_to_index(&working_elt);
            result.add_basis_element(out_idx, 1);
            return result;
        }

        // We now want to decompose Sq^x Sq^y = \sum_j *coef* Sq^{x + y - j} Sq^j.
        let y = working_elt.ps[0];

        let tail_idx = self.tail_of_basis_element_to_index(&mut working_elt, 1, 1);

        for j in 0..=x / 2 {
            if combinatorics::adem_relation_coefficient(fp::prime::TWO, x, y, j, 0, 0) == 0 {
                continue;
            }
            if j == 0 {
                working_elt.ps[0] = x + y;
                working_elt.degree += x as i32;
                // In this case the result is guaranteed to be admissible so we can immediately add it to result
                let out_idx = self.basis_element_to_index(&working_elt);
                result.add_basis_element(out_idx, 1);
                continue;
            }
            // Now we need to reduce Sqj * (rest of Sqs)
            // The answer to this is in the table we're currently making.
            // total degree -> first sq -> idx of rest of squares
            let rest_reduced =
                &self.multiplication_table[(n as u32 - (x + y) + j) as usize][j as usize][tail_idx];
            for (i, _coeff) in rest_reduced.iter_nonzero() {
                // Reduce Sq^{x+y-j} * whatever square using the table in the same degree, larger index
                // Since we're doing the first squares in decreasing order and x + y - j > x,
                // we already calculated this.
                let source = &table[(x + y - j) as usize][i];
                result.add(source, 1);
            }
        }
        result
    }

    fn generate_multiplication_table_generic(&self, mut next_degree: i32, max_degree: i32) {
        // degree -> first_square -> admissibile sequence idx -> result vector
        if next_degree == 0 {
            self.multiplication_table.push(Vec::new());
            next_degree += 1;
        }
        let q = 2 * (*self.prime()) as i32 - 2;
        for n in next_degree..=max_degree {
            let mut table: Vec<Vec<FpVector>> = Vec::with_capacity(2 * (n / q + 1) as usize);
            for i in 0..=n / q {
                for b in 0..=1 {
                    // This corresponds to x = 2i + b
                    let dimension = self.dimension(n - q * i - b);
                    table.push(Vec::with_capacity(dimension));
                }
            }
            for i in (0..=n / q).rev() {
                for idx in 0..self.dimension(n - q * i - 1) {
                    let res =
                        self.generate_multiplication_table_generic_step(&table, n, 2 * i + 1, idx);
                    table[1 + 2 * i as usize].push(res);
                }
                if i != 0 {
                    for idx in 0..self.dimension(n - q * i) {
                        let res =
                            self.generate_multiplication_table_generic_step(&table, n, 2 * i, idx);
                        table[2 * i as usize].push(res);
                    }
                }
            }
            self.multiplication_table.push(table);
        }
    }

    /// This function expresses $Sq^x$ (current) in terms of the admissible basis and returns
    /// the result as an FpVector, where (current) is the admissible monomial of degree $n - qx$
    /// (so that $Sq^x)$ (current) has degree $n$) and index `idx`.
    ///
    /// Here $Sq^x$ means $P^{x/2}$ if $x$ is even and $\beta P^{(x-1)/2}$ if $x$ is odd.
    ///
    /// Note that x is always positive.
    fn generate_multiplication_table_generic_step(
        &self,
        table: &[Vec<FpVector>],
        n: i32,
        x: i32,
        idx: usize,
    ) -> FpVector {
        let p: i32 = *self.prime() as i32; // we use p for the i32 version and self.p for the u32 version
        let q: i32 = 2 * p - 2;

        let x: u32 = x as u32;

        let output_dimension = self.dimension(n);
        let mut result = FpVector::new(self.prime(), output_dimension);

        // If x is just \beta, this is super easy.
        if x == 1 {
            let mut elt = self.basis_element_from_index(n - 1, idx).clone();
            if elt.bocksteins & 1 == 0 {
                elt.bocksteins |= 1;
                elt.degree += 1;
                let index = self.basis_element_to_index(&elt);
                result.add_basis_element(index, 1);
            }
            return result;
        }

        // If x is \beta P^i, it is also easy.
        if x & 1 != 0 {
            let rest_reduced = &self.multiplication_table[n as usize - 1][x as usize - 1][idx];
            for (id, coef) in rest_reduced.iter().enumerate() {
                let mut elt = self.basis_element_from_index(n - 1, id).clone();
                // We dispose of all terms with a leading Bockstein
                if elt.bocksteins & 1 == 0 {
                    elt.bocksteins |= 1;
                    elt.degree += 1;
                    let index = self.basis_element_to_index(&elt);
                    result.add_basis_element(index, coef);
                }
            }
            return result;
        }

        // Now there is no Bockstein. We first check if the result is already admissible.
        let i: u32 = x / 2;
        let mut working_elt = self
            .basis_element_from_index(n - (q * i as i32), idx)
            .clone();

        let b: u32 = working_elt.bocksteins & 1;
        if working_elt.ps.is_empty() || i >= (*self.prime()) * working_elt.ps[0] + b {
            working_elt.ps.insert(0, i);
            working_elt.bocksteins <<= 1;
            working_elt.degree = n;

            let out_idx = self.basis_element_to_index(&working_elt);
            result.add_basis_element(out_idx, 1);
            return result;
        }

        // In other cases, use the Adem relations.
        let j: u32 = working_elt.ps[0];

        let tail_idx = self.tail_of_basis_element_to_index(&mut working_elt, 1, q as u32);

        if b == 0 {
            // We use P^i P^j = \sum ... P^{i + j - k} P^k
            for k in 0..=i / (*self.prime()) {
                let c = combinatorics::adem_relation_coefficient(self.prime(), i, j, k, 0, 0);
                if c == 0 {
                    continue;
                }
                if k == 0 {
                    // We will never need working_elt in the future. We can leave it messed up
                    working_elt.ps[0] = i + j;
                    working_elt.degree = n;
                    let new_index = self.basis_element_to_index(&working_elt);
                    result.add_basis_element(new_index, c);
                    continue;
                }

                let rest_reduced = &self.multiplication_table
                    [(n - q * (i + j - k) as i32) as usize][2 * k as usize][tail_idx];
                for (id, coeff) in rest_reduced.iter().enumerate() {
                    let source = &table[2 * (i + j - k) as usize][id];
                    result.add(source, (c * coeff) % *self.prime());
                }
            }
        } else {
            // First treat the k = 0 case.
            // \beta P^{i + j - k} P^i
            let c = combinatorics::adem_relation_coefficient(self.prime(), i, j, 0, 1, 0);
            working_elt.ps[0] = i + j;
            working_elt.degree = n;
            let index = self.basis_element_to_index(&working_elt);
            result.add_basis_element(index, c);

            // P^{i + j - k} \beta P^k. Check if there is \beta following P^k
            if working_elt.bocksteins & 2 == 0 {
                let c = combinatorics::adem_relation_coefficient(self.prime(), i, j, 0, 0, 1);
                working_elt.bocksteins ^= 3; // flip the first two bits, so that it now ends with 10
                let index = self.basis_element_to_index(&working_elt);
                result.add_basis_element(index, c);
            }

            for k in 1..=i / (*self.prime()) {
                // \beta P^{i + j - k} P^k
                let c = combinatorics::adem_relation_coefficient(self.prime(), i, j, k, 1, 0);
                if c != 0 {
                    let rest_reduced = &self.multiplication_table
                        [(n - q * (i + j - k) as i32 - 1) as usize][2 * k as usize][tail_idx];
                    for (id, coeff) in rest_reduced.iter().enumerate() {
                        let source = &table[1 + 2 * (i + j - k) as usize][id];
                        result.add(source, (c * coeff) % *self.prime());
                    }
                }

                // P^{i + j - k} \beta P^k
                let c = combinatorics::adem_relation_coefficient(self.prime(), i, j, k, 0, 1);
                if c != 0 {
                    let rest_reduced = &self.multiplication_table
                        [(n - q * (i + j - k) as i32) as usize][1 + 2 * k as usize][tail_idx];
                    for (id, coeff) in rest_reduced.iter().enumerate() {
                        let source = &table[2 * (i + j - k) as usize][id];
                        result.add(source, (c * coeff) % *self.prime());
                    }
                }
            }
        }
        result
    }

    pub fn multiply_inner(
        &self,
        mut result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r_index: usize,
        s_degree: i32,
        s_index: usize,
        excess: i32,
        unstable: bool,
    ) {
        if coeff == 0 {
            return;
        }
        assert!(r_index < self.dimension(r_degree));
        assert!(s_index < self.dimension_unstable(s_degree, excess));

        if s_degree == 0 {
            if unstable && r_index >= self.dimension_unstable(r_degree, excess) {
                return;
            }
            result.add_basis_element(r_index, coeff);
            return;
        }
        let r = self.basis_element_from_index(r_degree, r_index);
        let s = self.basis_element_from_index(s_degree, s_index);
        let mut monomial = AdemBasisElement {
            degree: r.degree + s.degree,
            excess: 0,
            bocksteins: 0,
            ps: Vec::with_capacity(r.ps.len() + s.ps.len()),
            p_or_sq: *self.prime() != 2,
        };
        if self.generic && (r.bocksteins >> r.ps.len()) & s.bocksteins & 1 == 1 {
            // If there is a bockstein at the end of r and one at the beginning of s, these run into each other
            // and the output is 0.
            return;
        } else if self.generic {
            monomial.bocksteins = r.bocksteins;
            monomial.bocksteins |= s.bocksteins << (r.ps.len());
        }

        monomial.ps.extend_from_slice(&r.ps);
        monomial.ps.extend_from_slice(&s.ps);

        let stop_early = true;
        let index_to_check_for_admissibility = r.ps.len() as i32 - 1;
        if self.generic {
            // If r ends in a bockstein, we need to move it over because we consider
            // the monomial from right to left in chunks like bP^i. The b from the end of r gets donated
            // to the P from the beginning of s.
            let leading_degree = r.degree - ((r.bocksteins >> r.ps.len()) & 1) as i32;
            self.make_mono_admissible_generic(
                result,
                coeff,
                &mut monomial,
                index_to_check_for_admissibility,
                leading_degree,
                excess,
                stop_early,
                unstable,
            );
        } else {
            let leading_degree = r.degree;
            self.make_mono_admissible_2(
                result,
                &mut monomial,
                index_to_check_for_admissibility,
                leading_degree,
                excess,
                stop_early,
                unstable,
            );
        }
    }

    pub fn make_mono_admissible(
        &self,
        result: SliceMut,
        coeff: u32,
        monomial: &mut AdemBasisElement,
        excess: i32,
        unstable: bool,
    ) {
        let q = if self.generic {
            2 * (*self.prime()) - 2
        } else {
            1
        };
        let mut leading_degree = monomial.degree - (q * monomial.ps[monomial.ps.len() - 1]) as i32;
        let idx = monomial.ps.len() as i32 - 2;
        let stop_early = false;
        if self.generic {
            leading_degree -= ((monomial.bocksteins >> (monomial.ps.len() - 1)) & 1) as i32;
            self.make_mono_admissible_generic(
                result,
                coeff,
                monomial,
                idx,
                leading_degree,
                excess,
                stop_early,
                unstable,
            );
        } else {
            self.make_mono_admissible_2(
                result,
                monomial,
                idx,
                leading_degree,
                excess,
                stop_early,
                unstable,
            );
        }
    }

    /// Reduce a Steenrod monomial at the prime 2.
    /// # Arguments:
    ///  * `algebra` - an Adem algebra. This would be a method of class AdemAlgebra.
    ///  * `result`  - Where we put the result
    ///  * `monomial` - a not necessarily admissible Steenrod monomial which we will reduce.
    ///                We destroy monomial->Ps.
    ///  * `idx` - the only index to check for inadmissibility in the input (we assume that we've gotten
    ///           our input as a product of two admissible sequences.)
    ///  * `leading_degree` - the degree of the squares between 0 and idx (so of length idx + 1)
    fn make_mono_admissible_2(
        &self,
        mut result: SliceMut,
        monomial: &mut AdemBasisElement,
        mut idx: i32,
        mut leading_degree: i32,
        excess: i32,
        stop_early: bool,
        unstable: bool,
    ) {
        while idx < 0
            || idx as usize == monomial.ps.len() - 1
            || monomial.ps[idx as usize] >= 2 * monomial.ps[idx as usize + 1]
        {
            if idx < 0 || stop_early {
                // Admissible so write monomial to result.
                let idx = self.basis_element_to_index(monomial);
                // If excess is too large, quit. It's faster to check this by comparing idx to dimension
                // than to use fromIndex because fromIndex dereferences a hash map.
                if unstable && idx >= self.dimension_unstable(monomial.degree, excess) {
                    return;
                }
                result.add_basis_element(idx, 1);
                return;
            }
            leading_degree -= monomial.ps[idx as usize] as i32;
            idx -= 1;
        }
        let idx = idx as usize;
        let adm_idx = self.tail_of_basis_element_to_index(monomial, idx as u32 + 1, 1);
        let x = monomial.ps[idx] as i32;
        let tail_degree = monomial.degree - leading_degree + x;
        let reduced_tail = &self.multiplication_table[tail_degree as usize][x as usize][adm_idx];

        let mut new_monomial = AdemBasisElement {
            degree: monomial.degree,
            excess: -1,
            bocksteins: 0,
            ps: monomial.ps[0..idx].to_vec(),
            p_or_sq: *self.prime() != 2,
        };

        for (it_idx, _value) in reduced_tail.iter_nonzero() {
            let cur_tail_basis_elt = self.basis_element_from_index(tail_degree, it_idx);
            new_monomial.ps.truncate(idx);
            new_monomial.ps.extend_from_slice(&cur_tail_basis_elt.ps);
            self.make_mono_admissible_2(
                result.copy(),
                &mut new_monomial,
                idx as i32 - 1,
                leading_degree - x,
                excess,
                stop_early,
                unstable,
            );
        }
    }

    fn make_mono_admissible_generic(
        &self,
        mut result: SliceMut,
        coeff: u32,
        monomial: &mut AdemBasisElement,
        mut idx: i32,
        mut leading_degree: i32,
        excess: i32,
        stop_early: bool,
        unstable: bool,
    ) {
        let p = *self.prime();
        let q = 2 * p - 2;
        // Check for admissibility
        let b1 = if idx >= 0 {
            (monomial.bocksteins >> idx) & 1
        } else {
            0
        };
        let b2 = (monomial.bocksteins >> (idx + 1)) & 1;
        while idx < 0
            || idx == monomial.ps.len() as i32 - 1
            || monomial.ps[idx as usize] >= p * monomial.ps[idx as usize + 1] + b2
        {
            if idx < 0 || stop_early {
                // Admissible so write monomial to result.
                let idx = self.basis_element_to_index(monomial);
                if unstable && idx >= self.dimension_unstable(monomial.degree, excess) {
                    return;
                }
                result.add_basis_element(idx, coeff);
                return;
            }
            leading_degree -= (q * monomial.ps[idx as usize]) as i32;
            leading_degree -= ((monomial.bocksteins >> idx) & 1) as i32;
            idx -= 1;
        }
        let idx = idx as usize;
        let adm_idx = self.tail_of_basis_element_to_index(monomial, idx as u32 + 1, q);
        // Notice how much we avoid bockstein twiddling here. It's all hidden in multiplication_table =)
        let x = monomial.ps[idx];
        let bx = (x << 1) + b1;
        let tail_degree = monomial.degree - leading_degree + (q * x + b1) as i32;
        let reduced_tail = &self.multiplication_table[tail_degree as usize][bx as usize][adm_idx];
        let mut new_monomial = AdemBasisElement {
            degree: monomial.degree,
            excess: -1,
            bocksteins: 0,
            ps: monomial.ps[0..idx].to_vec(),
            p_or_sq: *self.prime() != 2,
        };

        for (it_idx, it_value) in reduced_tail.iter_nonzero() {
            let cur_tail_basis_elt = self.basis_element_from_index(tail_degree, it_idx);
            new_monomial.ps.truncate(idx);
            new_monomial.ps.extend_from_slice(&cur_tail_basis_elt.ps);
            new_monomial.bocksteins = monomial.bocksteins & ((1 << idx) - 1);
            new_monomial.bocksteins |= cur_tail_basis_elt.bocksteins << idx;
            let new_leading_degree = leading_degree - (q * x + b1) as i32;
            self.make_mono_admissible_generic(
                result.copy(),
                (coeff * it_value) % p,
                &mut new_monomial,
                idx as i32 - 1,
                new_leading_degree,
                excess,
                stop_early,
                unstable,
            );
        }
    }

    fn decompose_basis_element_2(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let b = self.basis_element_from_index(degree, idx);
        if b.ps.len() > 1 {
            let degree_first = b.ps[0] as i32;
            let degree_rest = b.degree - b.ps[0] as i32;
            let ps_rest = b.ps[1..].to_vec();
            let idx_first = self.basis_element_to_index(&AdemBasisElement {
                degree: degree_first,
                excess: 0,
                bocksteins: 0,
                ps: vec![b.ps[0]],
                p_or_sq: *self.prime() != 2,
            });
            let idx_rest = self.basis_element_to_index(&AdemBasisElement {
                degree: degree_rest,
                excess: 0,
                bocksteins: 0,
                ps: ps_rest,
                p_or_sq: *self.prime() != 2,
            });
            return vec![(1, (degree_first, idx_first), (degree_rest, idx_rest))];
        }
        let sq = b.ps[0];
        let tz = sq.trailing_zeros();
        let first_sq = 1 << tz;
        let second_sq = sq ^ first_sq;
        let first_degree = first_sq as i32;
        let second_degree = second_sq as i32;
        let first_idx = self.basis_element_to_index(&AdemBasisElement {
            degree: first_degree,
            excess: 0,
            bocksteins: 0,
            ps: vec![first_sq],
            p_or_sq: *self.prime() != 2,
        });
        let second_idx = self.basis_element_to_index(&AdemBasisElement {
            degree: second_degree,
            excess: 0,
            bocksteins: 0,
            ps: vec![second_sq],
            p_or_sq: *self.prime() != 2,
        });
        let mut out_vec = FpVector::new(fp::prime::TWO, self.dimension(degree));
        self.multiply_basis_elements(
            out_vec.as_slice_mut(),
            1,
            first_degree,
            first_idx,
            second_degree,
            second_idx,
        );
        out_vec.set_entry(idx, 0);
        let mut result = vec![(1, (first_degree, first_idx), (second_degree, second_idx))];
        for (i, _v) in out_vec.iter_nonzero() {
            result.extend(self.decompose_basis_element_2(degree, i));
        }
        result
    }

    fn decompose_basis_element_generic(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let p = self.prime();
        let b = self.basis_element_from_index(degree, idx);
        let leading_bockstein_idx = 1; // << (b.ps.len());
        if b.bocksteins & leading_bockstein_idx != 0 {
            let mut b_new = b.clone();
            b_new.bocksteins ^= leading_bockstein_idx;
            b_new.degree -= 1;
            let first_degree = 1;
            let first_idx = 0;
            let rest_degree = b_new.degree;
            let rest_idx = self.basis_element_to_index(&b_new);
            return vec![(1, (first_degree, first_idx), (rest_degree, rest_idx))];
        }
        if b.bocksteins != 0 || b.ps.len() != 1 {
            let first_degree = (b.ps[0] * 2 * (*p - 1)) as i32;
            let rest_degree = b.degree - first_degree;
            let ps_first = vec![b.ps[0]];
            let ps_rest = b.ps[1..].to_vec();
            let first = AdemBasisElement {
                degree: first_degree,
                bocksteins: 0,
                excess: 0,
                ps: ps_first,
                p_or_sq: *self.prime() != 2,
            };
            let rest = AdemBasisElement {
                degree: rest_degree,
                bocksteins: b.bocksteins >> 1,
                excess: 0,
                ps: ps_rest,
                p_or_sq: *self.prime() != 2,
            };
            let first_idx = self.basis_element_to_index(&first);
            let rest_idx = self.basis_element_to_index(&rest);
            return vec![(1, (first_degree, first_idx), (rest_degree, rest_idx))];
        }

        let sq = b.ps[0];
        let mut pow = 1;
        {
            let mut temp_sq = sq;
            while temp_sq % *p == 0 {
                temp_sq /= *p;
                pow *= *p;
            }
        }

        let first_sq = pow;
        let second_sq = sq - first_sq;
        let first_degree = (first_sq * 2 * (*p - 1)) as i32;
        let second_degree = (second_sq * 2 * (*p - 1)) as i32;
        let first_idx = self.basis_element_to_index(&AdemBasisElement {
            degree: first_degree,
            excess: 0,
            bocksteins: 0,
            ps: vec![first_sq],
            p_or_sq: *self.prime() != 2,
        });
        let second_idx = self.basis_element_to_index(&AdemBasisElement {
            degree: second_degree,
            excess: 0,
            bocksteins: 0,
            ps: vec![second_sq],
            p_or_sq: *self.prime() != 2,
        });
        let mut out_vec = FpVector::new(p, self.dimension(degree));
        self.multiply_basis_elements(
            out_vec.as_slice_mut(),
            1,
            first_degree,
            first_idx,
            second_degree,
            second_idx,
        );
        let mut result = Vec::new();
        let c = out_vec.entry(idx);
        assert!(c != 0);
        let c_inv = fp::prime::inverse(p, *p - c);
        result.push((
            ((*p - 1) * c_inv) % *p,
            (first_degree, first_idx),
            (second_degree, second_idx),
        ));
        out_vec.set_entry(idx, 0);
        for (i, v) in out_vec.iter_nonzero() {
            let (c, t1, t2) = self.decompose_basis_element_generic(degree, i)[0];
            result.push(((c_inv * c * v) % *p, t1, t2));
        }
        result
    }

    pub fn beps_pn(&self, e: u32, x: u32) -> (i32, usize) {
        if x == 0 && e == 1 {
            return (1, 0);
        } else if x == 0 {
            return (0, 0);
        }

        let p = *self.prime();
        let q = if self.generic { 2 * p - 2 } else { 1 };
        let degree = (x * q + e) as i32;
        let index = self.basis_element_to_index(&AdemBasisElement {
            degree,
            excess: 0,
            bocksteins: e,
            ps: vec![x],
            p_or_sq: *self.prime() != 2,
        });
        (degree, index)
    }
}

impl AdemAlgebra {
    fn generate_excess_table(&self, max_degree: i32) {
        self.excess_table.extend(max_degree as usize, |n| {
            let mut new_entry = Vec::with_capacity(n);
            let mut cur_excess = 0;
            for (i, elt) in self.basis_table[n].iter().enumerate() {
                for _ in cur_excess..elt.excess {
                    new_entry.push(i);
                }
                cur_excess = elt.excess;
            }
            let dim = self.dimension(n as i32);
            for _ in cur_excess..n as i32 {
                new_entry.push(dim);
            }
            new_entry
        });
    }
}

impl Bialgebra for AdemAlgebra {
    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)> {
        let elt = &self.basis_table[op_deg as usize][op_idx];
        if self.generic {
            let mut result: Vec<(i32, usize)> = Vec::with_capacity(elt.ps.len() * 2 + 1);
            let mut bockstein = elt.bocksteins;
            for item in &elt.ps {
                if bockstein & 1 == 1 {
                    result.push((1, 0));
                }
                bockstein >>= 1;
                result.push(self.beps_pn(0, *item));
            }
            if bockstein & 1 == 1 {
                result.push((1, 0));
            }
            result.reverse();
            result
        } else {
            elt.ps
                .iter()
                .rev()
                .map(|i| (*i as i32, 0))
                .collect::<Vec<_>>()
        }
    }

    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        if self.generic {
            if op_deg == 1 {
                vec![(1, 0, 0, 0), (0, 0, 1, 0)]
            } else {
                let q = *self.prime() * 2 - 2;
                let op_deg = op_deg as u32;
                assert_eq!(op_deg % q, 0);

                (0..=op_deg / q)
                    .map(|j| {
                        let first = self.beps_pn(0, j);
                        let last = self.beps_pn(0, op_deg / q - j);
                        (first.0, first.1, last.0, last.1)
                    })
                    .collect::<Vec<_>>()
            }
        } else {
            assert_eq!(op_idx, 0);
            (0..=op_deg)
                .map(|j| (j, 0, op_deg - j, 0))
                .collect::<Vec<_>>()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest(p, max_degree, case(2, 32), case(3, 120))]
    #[trace]
    fn test_adem_decompose(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = AdemAlgebra::new(p, *p != 2, false);
        algebra.compute_basis(max_degree);
        for i in 1..=max_degree {
            let dim = algebra.dimension(i);
            let gens = algebra.generators(i);
            println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim);
            for j in 0..dim {
                if gens.contains(&j) {
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in
                    algebra.decompose_basis_element(i, j)
                {
                    print!(
                        "{} * {} * {}  +  ",
                        coeff,
                        algebra.basis_element_to_string(first_degree, first_idx),
                        algebra.basis_element_to_string(second_degree, second_idx)
                    );
                    algebra.multiply_basis_elements(
                        out_vec.as_slice_mut(),
                        coeff,
                        first_degree,
                        first_idx,
                        second_degree,
                        second_idx,
                    );
                }
                assert!(
                    out_vec.entry(j) == 1,
                    "{} != {}",
                    algebra.basis_element_to_string(i, j),
                    algebra.element_to_string(i, out_vec.as_slice())
                );
                out_vec.set_entry(j, 0);
                assert!(
                    out_vec.is_zero(),
                    "\n{} != {}",
                    algebra.basis_element_to_string(i, j),
                    algebra.element_to_string(i, out_vec.as_slice())
                );
            }
        }
    }

    use crate::module::ModuleFailedRelationError;
    #[rstest(p, max_degree, case(2, 32), case(3, 120))]
    #[trace]
    fn test_adem_relations(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = AdemAlgebra::new(p, *p != 2, false);
        algebra.compute_basis(max_degree);
        let mut output_vec = FpVector::new(p, 0);
        for i in 1..=max_degree {
            let output_dim = algebra.dimension(i);
            output_vec.set_scratch_vector_size(output_dim);
            let relations = algebra.generating_relations(i);
            for relation in relations {
                for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    algebra.multiply_basis_elements(
                        output_vec.as_slice_mut(),
                        *coeff,
                        *deg_1,
                        *idx_1,
                        *deg_2,
                        *idx_2,
                    );
                }
                if !output_vec.is_zero() {
                    let mut relation_string = String::new();
                    for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                        relation_string.push_str(&format!(
                            "{} * {} * {}  +  ",
                            *coeff,
                            &algebra.basis_element_to_string(*deg_1, *idx_1),
                            &algebra.basis_element_to_string(*deg_2, *idx_2)
                        ));
                    }
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    relation_string.pop();
                    let value_string = algebra.element_to_string(i as i32, output_vec.as_slice());
                    panic!(
                        "{}",
                        ModuleFailedRelationError {
                            relation: relation_string,
                            value: value_string
                        }
                    );
                }
            }
        }
    }

    #[rstest]
    #[trace]
    #[case(2, 32)]
    #[case(3, 106)]
    fn test_adem_string(#[case] p: u32, #[case] max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = AdemAlgebra::new(p, *p != 2, false);
        algebra.compute_basis(max_degree);
        for t in 0..max_degree {
            for i in 0..algebra.dimension(t) {
                let elt = algebra.basis_element_to_string(t, i);
                assert_eq!(
                    Some((t, i)),
                    algebra.basis_element_from_string(&elt),
                    "Error parsing {elt}"
                );
            }
            for i in algebra.generators(t) {
                let elt = algebra.generator_to_string(t, i);
                assert_eq!(
                    Some((t, i)),
                    algebra.basis_element_from_string(&elt),
                    "Error parsing {elt}"
                );
            }
        }
    }
}
