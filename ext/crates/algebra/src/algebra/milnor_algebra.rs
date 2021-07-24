use itertools::Itertools;
use rustc_hash::FxHashMap as HashMap;
use std::sync::Mutex;

use crate::algebra::combinatorics;
use crate::algebra::{Algebra, Bialgebra, GeneratedAlgebra};
use fp::prime::{integer_power, Binomial, BitflagIterator, ValidPrime};
use fp::vector::{FpVector, Slice, SliceMut};
use once::OnceVec;

#[cfg(feature = "json")]
use {crate::algebra::JsonAlgebra, serde::Deserialize, serde_json::value::Value};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, space1},
    combinator::map,
    sequence::{delimited, pair},
    IResult,
};

// This is here so that the Python bindings can use modules defined for AdemAlgebraT with their own algebra enum.
// In order for things to work AdemAlgebraT cannot implement Algebra.
// Otherwise, the algebra enum for our bindings will see an implementation clash.
pub trait MilnorAlgebraT: Send + Sync + Algebra {
    fn milnor_algebra(&self) -> &MilnorAlgebra;
}

pub struct MilnorProfile {
    pub truncated: bool,
    pub q_part: u32,
    pub p_part: PPart,
}

impl MilnorProfile {
    pub fn is_trivial(&self) -> bool {
        !self.truncated && self.q_part == !0 && self.p_part.is_empty()
    }
}

#[derive(Default, Clone)]
pub struct QPart {
    degree: i32,
    q_part: u32,
}

#[cfg(feature = "odd-primes")]
pub type PPartEntry = u32;

#[cfg(not(feature = "odd-primes"))]
pub type PPartEntry = u8;

pub type PPart = Vec<PPartEntry>;

#[derive(Debug, Clone, Default)]
pub struct MilnorBasisElement {
    pub q_part: u32,
    pub p_part: PPart,
    pub degree: i32,
}

impl MilnorBasisElement {
    fn from_p(p: PPart, dim: i32) -> Self {
        Self {
            p_part: p,
            q_part: 0,
            degree: dim,
        }
    }

    pub fn clone_into(&self, other: &mut Self) {
        other.q_part = self.q_part;
        other.degree = self.degree;
        other.p_part.clear();
        other.p_part.extend_from_slice(&self.p_part);
    }
}

impl std::cmp::PartialEq for MilnorBasisElement {
    fn eq(&self, other: &Self) -> bool {
        #[cfg(feature = "odd-primes")]
        return self.p_part == other.p_part && self.q_part == other.q_part;

        #[cfg(not(feature = "odd-primes"))]
        return self.p_part == other.p_part;
    }
}

impl std::cmp::Eq for MilnorBasisElement {}

impl std::hash::Hash for MilnorBasisElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.p_part.hash(state);
        #[cfg(feature = "odd-primes")]
        self.q_part.hash(state);
    }
}

impl std::fmt::Display for MilnorBasisElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.degree == 0 {
            write!(f, "1")?;
            return Ok(());
        }
        if self.q_part != 0 {
            let q_part = BitflagIterator::set_bit_iterator(self.q_part as u64)
                .map(|idx| format!("Q_{}", idx))
                .format(" ");
            write!(f, "{}", q_part)?;
        }
        if !self.p_part.is_empty() {
            if self.q_part != 0 {
                write!(f, " ")?;
            }
            write!(f, "P({})", self.p_part.iter().format(", "))?;
        }
        Ok(())
    }
}

// A basis element of a Milnor Algebra is of the form Q(E) P(R). Nore that deg P(R) is *always* a
// multiple of q = 2p - 2. So qpart_table is a vector of length (2p - 2), each containing a list of
// possible Q(E) of appropriate residue class mod q, sorted in increasing order of degree. On the
// other hand, ppart_table[i] consists of a list of possible P(R) of degree qi. When we construct a
// list of basis elements from ppart_table and qpart_table given a degree d, we iterate through the
// elements of qpart_table[d % q], and then for each element, we iterate through the appropriate
// entry in ppart_table of the right degree.
pub struct MilnorAlgebra {
    pub profile: MilnorProfile,
    lock: Mutex<()>,
    p: ValidPrime,
    #[cfg(feature = "odd-primes")]
    generic: bool,
    ppart_table: OnceVec<Vec<PPart>>,
    qpart_table: Vec<OnceVec<QPart>>,
    pub basis_table: OnceVec<Vec<MilnorBasisElement>>,
    basis_element_to_index_map: OnceVec<HashMap<MilnorBasisElement, usize>>, // degree -> MilnorBasisElement -> index
    #[cfg(feature = "cache-multiplication")]
    multiplication_table: OnceVec<OnceVec<Vec<Vec<FpVector>>>>, // source_deg -> target_deg -> source_op -> target_op
}

impl std::fmt::Display for MilnorAlgebra {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MilnorAlgebra(p={})", self.prime())
    }
}

impl MilnorAlgebra {
    pub fn new(p: ValidPrime) -> Self {
        fp::vector::initialize_limb_bit_index_table(p);

        let profile = MilnorProfile {
            truncated: false,
            q_part: !0,
            p_part: Vec::new(),
        };

        Self {
            p,
            #[cfg(feature = "odd-primes")]
            generic: *p != 2,
            profile,
            lock: Mutex::new(()),
            ppart_table: OnceVec::new(),
            qpart_table: vec![OnceVec::new(); 2 * *p as usize - 2],
            basis_table: OnceVec::new(),
            basis_element_to_index_map: OnceVec::new(),
            #[cfg(feature = "cache-multiplication")]
            multiplication_table: OnceVec::new(),
        }
    }

    #[inline]
    pub fn generic(&self) -> bool {
        #[cfg(feature = "odd-primes")]
        {
            self.generic
        }

        #[cfg(not(feature = "odd-primes"))]
        {
            false
        }
    }

    pub fn q(&self) -> i32 {
        if self.generic() {
            2 * (*self.prime() as i32 - 1)
        } else {
            1
        }
    }

    pub fn basis_element_from_index(&self, degree: i32, idx: usize) -> &MilnorBasisElement {
        &self.basis_table[degree as usize][idx]
    }

    pub fn try_basis_element_to_index(&self, elt: &MilnorBasisElement) -> Option<usize> {
        self.basis_element_to_index_map[elt.degree as usize]
            .get(elt)
            .copied()
    }

    pub fn basis_element_to_index(&self, elt: &MilnorBasisElement) -> usize {
        self.try_basis_element_to_index(elt)
            .unwrap_or_else(|| panic!("Didn't find element: {:?}", elt))
    }
}

impl Algebra for MilnorAlgebra {
    fn prime(&self) -> ValidPrime {
        self.p
    }

    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        let mut products = Vec::with_capacity(4);
        let max_degree;
        if self.generic() {
            if self.profile.q_part & 1 != 0 {
                products.push((
                    "a_0".to_string(),
                    MilnorBasisElement {
                        degree: 1,
                        q_part: 1,
                        p_part: vec![],
                    },
                ));
            }
            if (self.profile.p_part.is_empty() && !self.profile.truncated)
                || (!self.profile.p_part.is_empty() && self.profile.p_part[0] > 0)
            {
                products.push((
                    "h_0".to_string(),
                    MilnorBasisElement {
                        degree: (2 * (*self.prime()) - 2) as i32,
                        q_part: 0,
                        p_part: vec![1],
                    },
                ));
            }
            max_degree = (2 * (*self.prime()) - 2) as i32;
        } else {
            let mut max = 4;
            if !self.profile.p_part.is_empty() {
                max = std::cmp::min(4, self.profile.p_part[0]);
            } else if self.profile.truncated {
                max = 0;
            }
            for i in 0..max {
                let degree = 1 << i; // degree is 2^hi
                products.push((
                    format!("h_{}", i),
                    MilnorBasisElement {
                        degree,
                        q_part: 0,
                        p_part: vec![1 << i],
                    },
                ));
            }
            max_degree = 1 << 3;
        }
        self.compute_basis(max_degree + 1);

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

        self.compute_ppart(max_degree);
        self.compute_qpart(next_degree, max_degree);

        if self.generic() {
            self.generate_basis_generic(next_degree, max_degree);
        } else {
            self.generate_basis_2(next_degree, max_degree);
        }

        // Populate hash map
        for d in next_degree as usize..=max_degree as usize {
            let basis = &self.basis_table[d];
            let mut map = HashMap::default();
            map.reserve(basis.len());
            for (i, b) in basis.iter().enumerate() {
                map.insert(b.clone(), i);
            }
            self.basis_element_to_index_map.push(map);
        }

        #[cfg(feature = "cache-multiplication")]
        {
            for d in 0..=max_degree as usize {
                if self.multiplication_table.len() == d {
                    self.multiplication_table.push(OnceVec::new());
                }
                for e in self.multiplication_table[d].len()..=max_degree as usize - d {
                    self.multiplication_table[d].push(
                        (0..self.dimension(d as i32, -1))
                            .map(|i| {
                                (0..self.dimension(e as i32, -1))
                                    .map(|j| {
                                        let mut res = FpVector::new(
                                            self.prime(),
                                            self.dimension((d + e) as i32, -1),
                                        );
                                        self.multiply(
                                            &mut res,
                                            1,
                                            &self.basis_table[d][i],
                                            &self.basis_table[e][j],
                                        );
                                        res
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>(),
                    );
                }
            }
        }
    }

    fn dimension(&self, degree: i32, _excess: i32) -> usize {
        if degree < 0 {
            return 0;
        }
        self.basis_table[degree as usize].len()
    }

    #[cfg(not(feature = "cache-multiplication"))]
    fn multiply_basis_elements(
        &self,
        result: SliceMut,
        coef: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
        _excess: i32,
    ) {
        self.multiply(
            result,
            coef,
            &self.basis_table[r_degree as usize][r_idx],
            &self.basis_table[s_degree as usize][s_idx],
        );
    }

    #[cfg(feature = "cache-multiplication")]
    fn multiply_basis_elements(
        &self,
        result: SliceMut,
        coef: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
        _excess: i32,
    ) {
        result.shift_add(
            &self.multiplication_table[r_degree as usize][s_degree as usize][r_idx][s_idx]
                .as_slice(),
            coef,
        );
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        format!("{}", self.basis_table[degree as usize][idx])
    }
}

#[cfg(feature = "json")]
impl JsonAlgebra for MilnorAlgebra {
    fn prefix(&self) -> &str {
        "milnor"
    }

    fn json_to_basis(&self, json: &Value) -> error::Result<(i32, usize)> {
        let xi_degrees = combinatorics::xi_degrees(self.prime());
        let tau_degrees = combinatorics::tau_degrees(self.prime());

        let p_part: PPart;
        let mut q_part = 0;
        let mut degree = 0;

        if self.generic() {
            let (q_list, p_list): (Vec<u8>, PPart) = <_>::deserialize(json)?;
            let q = self.q();

            p_part = p_list;
            for (i, &val) in p_part.iter().enumerate() {
                degree += (val as i32) * xi_degrees[i] * q;
            }

            for k in q_list {
                q_part |= 1 << k;
                degree += tau_degrees[k as usize];
            }
        } else {
            p_part = <_>::deserialize(json)?;
            for (i, &val) in p_part.iter().enumerate() {
                degree += (val as i32) * xi_degrees[i];
            }
        }
        let m = MilnorBasisElement {
            q_part,
            p_part,
            degree,
        };
        Ok((degree, self.basis_element_to_index(&m)))
    }

    fn json_from_basis(&self, degree: i32, index: usize) -> Value {
        let b = self.basis_element_from_index(degree, index);
        if self.generic() {
            let mut q_part = b.q_part;
            let mut q_list = Vec::with_capacity(q_part.count_ones() as usize);
            while q_part != 0 {
                let tz = q_part.trailing_zeros();
                q_part ^= 1 << tz;
                q_list.push(tz);
            }
            serde_json::to_value((q_list, &b.p_part)).unwrap()
        } else {
            serde_json::to_value(&b.p_part).unwrap()
        }
    }
}

impl GeneratedAlgebra for MilnorAlgebra {
    // Same implementation as AdemAlgebra
    fn string_to_generator<'a, 'b>(&'a self, input: &'b str) -> IResult<&'b str, (i32, usize)> {
        let first = map(
            alt((
                delimited(char('P'), digit1, space1),
                delimited(tag("Sq"), digit1, space1),
            )),
            |elt| {
                let i = std::str::FromStr::from_str(elt).unwrap();
                self.beps_pn(0, i)
            },
        );

        let second = map(pair(char('b'), space1), |_| (1, 0));

        alt((first, second))(input)
    }

    fn generator_to_string(&self, degree: i32, _idx: usize) -> String {
        if self.generic() {
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
        if degree == 0 {
            return vec![];
        }
        if self.generic() && degree == 1 {
            return vec![0]; // Q_0
        }
        let p = *self.prime();
        let q = self.q() as u32;
        let mut temp_degree = degree as u32;
        if temp_degree % q != 0 {
            return vec![];
        }
        temp_degree /= q;
        let mut power = 0;
        while temp_degree % p == 0 {
            temp_degree /= p;
            power += 1;
        }
        if temp_degree != 1 {
            return vec![];
        }
        if (self.profile.p_part.is_empty() && self.profile.truncated)
            || (!self.profile.p_part.is_empty() && self.profile.p_part[0] <= power)
        {
            return vec![];
        }

        let idx = self.basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part: 0,
            p_part: vec![(degree as u32 / q) as PPartEntry],
        });
        return vec![idx];
    }

    fn decompose_basis_element(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = &self.basis_table[degree as usize][idx];
        // If qpart = 0, return self
        if basis.q_part == 0 {
            self.decompose_basis_element_ppart(degree, idx)
        } else {
            self.decompose_basis_element_qpart(degree, idx)
        }
    }

    fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>> {
        if self.generic() && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }
        let p = self.prime();
        let inadmissible_pairs = combinatorics::inadmissible_pairs(p, self.generic(), degree);
        let mut result = Vec::new();
        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation. Sometimes these don't exist because of profiles. Then just ignore it.
            (|| {
                let (first_degree, first_index) = self.try_beps_pn(0, x as PPartEntry)?;
                let (second_degree, second_index) = self.try_beps_pn(b, y as PPartEntry)?;
                relation.push((
                    *p - 1,
                    (first_degree, first_index),
                    (second_degree, second_index),
                ));
                for e1 in 0..=b {
                    let e2 = b - e1;
                    // e1 and e2 determine where a bockstein shows up.
                    // e1 determines whether a bockstein shows up in front
                    // e2 determines whether a bockstein shows up in middle
                    // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                    for j in 0..=x / *p {
                        let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                        if c == 0 {
                            continue;
                        }
                        if j == 0 {
                            relation.push((
                                c,
                                self.try_beps_pn(e1, (x + y) as PPartEntry)?,
                                (e2 as i32, 0),
                            ));
                            continue;
                        }
                        let first_sq = self.try_beps_pn(e1, (x + y - j) as PPartEntry)?;
                        let second_sq = self.try_beps_pn(e2, j as PPartEntry)?;
                        relation.push((c, first_sq, second_sq));
                    }
                }
                result.push(relation);
                Some(())
            })();
        }
        result
    }
}

// Compute basis functions
impl MilnorAlgebra {
    fn compute_ppart(&self, max_degree: i32) {
        self.ppart_table.extend(0, |_| vec![Vec::new()]);

        let p = *self.prime() as i32;
        let q = if p == 2 { 1 } else { 2 * p - 2 };
        let new_deg = max_degree / q;

        let xi_degrees = combinatorics::xi_degrees(self.prime());
        let mut profile_list = Vec::with_capacity(xi_degrees.len());
        for i in 0..xi_degrees.len() {
            if i < self.profile.p_part.len() {
                profile_list.push(
                    (integer_power(*self.prime(), self.profile.p_part[i] as u32) - 1) as PPartEntry,
                );
            } else if self.profile.truncated {
                profile_list.push(0);
            } else {
                profile_list.push(PPartEntry::MAX);
            }
        }

        self.ppart_table.extend(new_deg as usize, |d| {
            let d = d as i32;
            let mut new_row = Vec::new(); // Improve this
            for i in 0..xi_degrees.len() {
                if xi_degrees[i] > d {
                    break;
                }
                if profile_list[i] == 0 {
                    continue;
                }

                let rem = (d - xi_degrees[i]) as usize;
                for old in &self.ppart_table[rem] {
                    // ppart_table[rem] is arranged in increasing order of highest
                    // xi_i. If we get something too large, we may abort;
                    if old.len() > i + 1 {
                        break;
                    }
                    if old.len() == i + 1 && old[i] == profile_list[i] {
                        continue;
                    }
                    let mut new = old.clone();
                    if new.len() < i + 1 {
                        new.resize(i + 1, 0);
                    }
                    new[i] += 1;
                    new_row.push(new);
                }
            }
            new_row
        });
    }

    fn compute_qpart(&self, next_degree: i32, max_degree: i32) {
        let q = (2 * (*self.prime()) - 2) as i32;
        let profile = !self.profile.q_part;

        if !self.generic() {
            return;
        }

        let mut next_degree = next_degree;
        if next_degree == 0 {
            self.qpart_table[0].push_checked(
                QPart {
                    degree: 0,
                    q_part: 0,
                },
                0,
            );
            next_degree = 1;
        }

        let tau_degrees = combinatorics::tau_degrees(self.prime());
        let old_max_tau = tau_degrees
            .iter()
            .position(|d| *d > next_degree - 1)
            .unwrap(); // Use expect instead
        let new_max_tau = tau_degrees.iter().position(|d| *d > max_degree).unwrap();

        let bit_string_min: u32 = 1 << old_max_tau;
        let bit_string_max: u32 = 1 << new_max_tau;

        let mut residue: i32 = (old_max_tau as i32) % q;
        let mut total: i32 = tau_degrees[0..old_max_tau].iter().sum();

        for bit_string in bit_string_min..bit_string_max {
            // v has all the trailing zeros set. These are the bits that were set last time,
            // but aren't set anymore because of a carry. Shift right 1 because ???1000 ==> ???0111 xor ???1000 = 0001111.
            let mut v = (bit_string ^ (bit_string - 1)) >> 1;
            let mut c: usize = 0; // We're going to get the new bit that is set into c.
            while v != 0 {
                v >>= 1; // Subtract off the degree of each of the lost entries
                total -= tau_degrees[c];
                c += 1;
            }
            total += tau_degrees[c];
            residue += 1 - c as i32;
            if bit_string & profile != 0 {
                continue;
            }
            residue %= q;
            if residue < 0 {
                residue += q;
            }
            self.qpart_table[residue as usize].push(QPart {
                degree: total,
                q_part: bit_string,
            });
        }
    }

    fn generate_basis_generic(&self, next_degree: i32, max_degree: i32) {
        let q = (2 * (*self.prime()) - 2) as usize;

        for d in next_degree as usize..=max_degree as usize {
            let mut new_table = Vec::new(); // Initialize size

            for q_part in self.qpart_table[d % q].iter() {
                // Elements in qpart_table are listed in increasing order in
                // degree. Abort if degree too large.
                if q_part.degree > d as i32 {
                    break;
                }

                for p_part in &self.ppart_table[(d - (q_part.degree as usize)) / q] {
                    new_table.push(MilnorBasisElement {
                        p_part: p_part.clone(),
                        q_part: q_part.q_part,
                        degree: d as i32,
                    });
                }
            }
            //            new_table.shrink_to_fit();
            self.basis_table.push(new_table);
        }
    }

    fn generate_basis_2(&self, next_degree: i32, max_degree: i32) {
        for i in next_degree as usize..=max_degree as usize {
            self.basis_table.push(
                self.ppart_table[i]
                    .iter()
                    .map(|p| MilnorBasisElement::from_p(p.clone(), i as i32))
                    .collect(),
            );
        }
    }
}

// Multiplication logic
impl MilnorAlgebra {
    fn try_beps_pn(&self, e: u32, x: PPartEntry) -> Option<(i32, usize)> {
        let q = self.q() as u32;
        let degree = (q * x as u32 + e) as i32;
        self.try_basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part: e,
            p_part: vec![x as PPartEntry],
        })
        .map(|index| (degree, index))
    }

    fn beps_pn(&self, e: u32, x: PPartEntry) -> (i32, usize) {
        self.try_beps_pn(e, x).unwrap()
    }

    fn multiply_qpart(&self, m1: &MilnorBasisElement, f: u32) -> Vec<(u32, MilnorBasisElement)> {
        let mut new_result: Vec<(u32, MilnorBasisElement)> = vec![(1, m1.clone())];
        let mut old_result: Vec<(u32, MilnorBasisElement)> = Vec::new();

        for k in BitflagIterator::set_bit_iterator(f as u64) {
            let k = k as u32;
            let pk = integer_power(*self.p, k) as PPartEntry;
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
                    // Check if R - p^k e_i < 0. Only do this from the first term onwards.
                    if i > 0 && term.p_part[i - 1] < pk {
                        continue;
                    }

                    let mut new_p = term.p_part.clone();
                    if i > 0 {
                        new_p[i - 1] -= pk;
                    }

                    // Now calculate the number of Q's we are moving past
                    let larger_q = (term.q_part >> (k + i as u32 + 1)).count_ones();

                    // If new_p ends with 0, drop them
                    while let Some(0) = new_p.last() {
                        new_p.pop();
                    }
                    // Now put everything together
                    let m = MilnorBasisElement {
                        p_part: new_p,
                        q_part: term.q_part | 1 << (k + i as u32),
                        degree: 0, // we don't really care about the degree here. The final degree of the whole calculation is known a priori
                    };
                    let c = if larger_q % 2 == 0 {
                        *coef
                    } else {
                        *coef * (*self.prime() - 1)
                    };

                    new_result.push((c, m));
                }
            }
        }
        new_result
    }

    pub fn multiply(
        &self,
        res: SliceMut,
        coef: u32,
        m1: &MilnorBasisElement,
        m2: &MilnorBasisElement,
    ) {
        self.multiply_with_allocation(res, coef, m1, m2, PPartAllocation::default());
    }

    pub fn multiply_with_allocation(
        &self,
        mut res: SliceMut,
        coef: u32,
        m1: &MilnorBasisElement,
        m2: &MilnorBasisElement,
        mut allocation: PPartAllocation,
    ) -> PPartAllocation {
        let target_deg = m1.degree + m2.degree;
        if self.generic() {
            let m1f = self.multiply_qpart(m1, m2.q_part);
            for (cc, basis) in m1f {
                let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
                    self.prime(),
                    &basis.p_part,
                    &m2.p_part,
                    allocation,
                    basis.q_part,
                    target_deg,
                );

                while let Some(c) = multiplier.next() {
                    let idx = self.basis_element_to_index(&multiplier.ans);
                    res.add_basis_element(idx, c * cc * coef);
                }
                allocation = multiplier.into_allocation()
            }
        } else {
            let mut multiplier = PPartMultiplier::<false>::new_from_allocation(
                self.prime(),
                &m1.p_part,
                &m2.p_part,
                allocation,
                0,
                target_deg,
            );

            while let Some(c) = multiplier.next() {
                let idx = self.basis_element_to_index(&multiplier.ans);
                res.add_basis_element(idx, c * coef);
            }
            allocation = multiplier.into_allocation()
        }
        allocation
    }

    pub fn multiply_element_by_basis_with_allocation(
        &self,
        mut res: SliceMut,
        coef: u32,
        r_deg: i32,
        r: Slice,
        m2: &MilnorBasisElement,
        mut allocation: PPartAllocation,
    ) -> PPartAllocation {
        for (i, c) in r.iter_nonzero() {
            allocation = self.multiply_with_allocation(
                res.copy(),
                coef * c,
                self.basis_element_from_index(r_deg, i),
                m2,
                allocation,
            );
        }
        allocation
    }
}

#[derive(Debug, Default)]
struct Matrix2D {
    cols: usize,
    inner: PPart,
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

/// The parts of a PPartMultiplier that involve heap allocation. This lets us reuse the allocation
/// across multiple different multipliers. Reusing the whole PPartMultiplier is finicky but doable
/// due to lifetime issues, but it appears to be less performant.
#[derive(Default)]
pub struct PPartAllocation {
    m: Matrix2D,
    #[cfg(feature = "odd-primes")]
    diagonal: PPart,
    p_part: PPart,
}

impl PPartAllocation {
    /// This creates a PPartAllocation with enough capacity to handle mulitiply elements with
    /// of total degree < 2^n - ε at p = 2.
    pub fn with_capacity(n: usize) -> Self {
        Self {
            m: Matrix2D::with_capacity(n + 1, n),
            #[cfg(feature = "odd-primes")]
            diagonal: Vec::with_capacity(n),
            // This size should be the number of diagonals. Even though the answer cannot be that
            // long, we still insert zeros then pop them out later.
            p_part: Vec::with_capacity(2 * n),
        }
    }
}

#[allow(non_snake_case)]
pub struct PPartMultiplier<'a, const MOD4: bool> {
    p: ValidPrime,
    M: Matrix2D,
    r: &'a PPart,
    rows: usize,
    cols: usize,
    diag_num: usize,
    init: bool,
    pub ans: MilnorBasisElement,
    #[cfg(feature = "odd-primes")]
    diagonal: PPart,
}

#[allow(non_snake_case)]
impl<'a, const MOD4: bool> PPartMultiplier<'a, MOD4> {
    fn prime(&self) -> ValidPrime {
        self.p
    }

    #[allow(clippy::ptr_arg)]
    #[allow(unused_mut)] // Mut is only used with odd primes
    pub fn new_from_allocation(
        p: ValidPrime,
        r: &'a PPart,
        s: &'a PPart,
        mut allocation: PPartAllocation,
        q_part: u32,
        degree: i32,
    ) -> Self {
        if MOD4 {
            assert_eq!(*p, 2);
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
            M[i][0] = r[i - 1];
        }
        // This is somehow quite significantly faster than copy_from_slice
        #[allow(clippy::manual_memcpy)]
        for k in 1..cols {
            M[0][k] = s[k - 1];
        }

        let ans = MilnorBasisElement {
            q_part,
            p_part: allocation.p_part,
            degree,
        };
        PPartMultiplier {
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
            p_part: self.ans.p_part,
        }
    }

    /// This compute the first l > k such that (sum + l) choose l != 0 mod p, stopping if we reach
    /// max + 1. This is useful for incrementing the matrix.
    ///
    /// TODO: Improve odd prime performance
    fn next_val(&self, sum: PPartEntry, k: PPartEntry, max: PPartEntry) -> PPartEntry {
        match *self.prime() {
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
                    ((k | sum) + 1) & !sum
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
                p_to_the_j *= *self.prime() as PPartEntry;
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
                        self.M[row][0] = self.r[row - 1];
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

impl<'a, const MOD4: bool> Iterator for PPartMultiplier<'a, MOD4> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        let p = *self.prime() as PPartEntry;
        'outer: loop {
            self.ans.p_part.clear();
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
                for &k in &self.M[0][1..self.cols] {
                    self.ans.p_part.push(k);
                }
                if self.rows > self.cols {
                    self.ans.p_part.resize(self.r.len(), 0);
                }
                for (i, &entry) in self.r.iter().enumerate() {
                    self.ans.p_part[i] += entry;
                }
                return Some(coef as u32);
            } else if self.update() {
                for diag_idx in 1..=self.diag_num {
                    let i_min = if diag_idx + 1 > self.cols {
                        diag_idx + 1 - self.cols
                    } else {
                        0
                    };
                    let i_max = std::cmp::min(1 + diag_idx, self.rows);
                    let mut sum = 0;

                    if *self.prime() == 2 {
                        if MOD4 {
                            for i in i_min..i_max {
                                let entry = self.M[i][diag_idx - i];
                                sum += entry;
                                if coef % 2 == 0 {
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
                    self.ans.p_part.push(sum);
                }
                // If new_p ends with 0, drop them
                while let Some(0) = self.ans.p_part.last() {
                    self.ans.p_part.pop();
                }

                return Some(coef as u32);
            } else {
                return None;
            }
        }
    }
}
impl MilnorAlgebra {
    fn decompose_basis_element_qpart(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = &self.basis_table[degree as usize][idx];
        // Look for left-most non-zero qpart
        let i = basis.q_part.trailing_zeros();
        // If the basis element is just Q_{k+1}, we decompose Q_{k+1} = P(p^k) Q_k - Q_k P(p^k).
        if basis.q_part == 1 << i && basis.p_part.is_empty() {
            let ppow = fp::prime::integer_power(*self.prime(), i - 1);

            let q_degree = (2 * ppow - 1) as i32;
            let p_degree = (ppow * (2 * (*self.prime()) - 2)) as i32;

            let p_idx = self
                .basis_element_to_index(&MilnorBasisElement::from_p(
                    vec![ppow as PPartEntry],
                    p_degree,
                ))
                .to_owned();

            let q_idx = self
                .basis_element_to_index(&MilnorBasisElement {
                    q_part: 1 << (i - 1),
                    p_part: Vec::new(),
                    degree: q_degree,
                })
                .to_owned();

            return vec![
                (1, (p_degree, p_idx), (q_degree, q_idx)),
                (*self.prime() - 1, (q_degree, q_idx), (p_degree, p_idx)),
            ];
        }

        // Otherwise, separate out the first Q_k.
        let first_degree = combinatorics::tau_degrees(self.prime())[i as usize];
        let second_degree = degree - first_degree;

        let first_idx = self.basis_element_to_index(&MilnorBasisElement {
            q_part: 1 << i,
            p_part: Vec::new(),
            degree: first_degree,
        });

        let second_idx = self.basis_element_to_index(&MilnorBasisElement {
            q_part: basis.q_part ^ 1 << i,
            p_part: basis.p_part.clone(),
            degree: second_degree,
        });

        vec![(1, (first_degree, first_idx), (second_degree, second_idx))]
    }

    // use https://monks.scranton.edu/files/pubs/bases.pdf page 8
    #[allow(clippy::useless_let_if_seq)]
    fn decompose_basis_element_ppart(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let p = self.prime();
        let pp = *self.prime() as PPartEntry;
        let b = &self.basis_table[degree as usize][idx];
        let first;
        let second;
        if b.p_part.len() > 1 {
            let mut t1 = 0;
            let mut pow = 1;
            for r in &b.p_part {
                t1 += r * pow;
                pow *= pp;
            }
            first = self.beps_pn(0, t1);
            let second_degree = degree - first.0;
            let second_idx = self.basis_element_to_index(&MilnorBasisElement {
                q_part: 0,
                p_part: b.p_part[1..].to_vec(),
                degree: second_degree,
            });
            second = (second_degree, second_idx);
        } else {
            // return vec![(1, (degree, idx), (0, 0))];
            let sq = b.p_part[0];
            let mut pow = 1;
            {
                let mut temp_sq = sq;
                while temp_sq % pp == 0 {
                    temp_sq /= pp;
                    pow *= pp;
                }
            }
            if sq == pow {
                return vec![(1, (degree, idx), (0, 0))];
            }
            first = self.beps_pn(0, pow);
            second = self.beps_pn(0, sq - pow);
        }
        let mut out_vec = FpVector::new(p, self.dimension(degree, -1));
        self.multiply_basis_elements(
            out_vec.as_slice_mut(),
            1,
            first.0,
            first.1,
            second.0,
            second.1,
            -1,
        );
        let mut result = Vec::new();
        let c = out_vec.entry(idx);
        assert!(c != 0);
        out_vec.set_entry(idx, 0);
        let c_inv = fp::prime::inverse(p, *p - c);
        result.push((((*p - 1) * c_inv) % *p, first, second));
        for (i, v) in out_vec.iter_nonzero() {
            for (c, t1, t2) in self.decompose_basis_element_ppart(degree, i) {
                result.push(((c_inv * c * v) % *p, t1, t2));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use expect_test::expect;
    use rstest::rstest;

    #[rstest(p, max_degree, case(2, 32), case(3, 106))]
    #[trace]
    fn test_milnor_basis(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p); //p != 2
        algebra.compute_basis(max_degree);
        for i in 1..max_degree {
            let dim = algebra.dimension(i, -1);
            for j in 0..dim {
                let b = algebra.basis_element_from_index(i, j);
                assert_eq!(algebra.basis_element_to_index(b), j);
                let json = algebra.json_from_basis(i, j);
                let new_b = algebra.json_to_basis(&json).unwrap();
                assert_eq!(new_b, (i, j));
            }
        }
    }

    #[rstest(p, max_degree, case(2, 32), case(3, 106))]
    #[trace]
    fn test_milnor_decompose(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p);
        algebra.compute_basis(max_degree);
        for i in 1..max_degree {
            let dim = algebra.dimension(i, -1);
            let gens = algebra.generators(i);
            // println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim);
            for j in 0..dim {
                if gens.contains(&j) {
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in
                    algebra.decompose_basis_element(i, j)
                {
                    // print!("{} * {} * {}  +  ", coeff, algebra.basis_element_to_string(first_degree,first_idx), algebra.basis_element_to_string(second_degree, second_idx));
                    algebra.multiply_basis_elements(
                        out_vec.as_slice_mut(),
                        coeff,
                        first_degree,
                        first_idx,
                        second_degree,
                        second_idx,
                        -1,
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
    #[rstest(p, max_degree, case(2, 32), case(3, 106))]
    #[trace]
    fn test_adem_relations(p: u32, max_degree: i32) {
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p); // , p != 2
        algebra.compute_basis(max_degree + 2);
        let mut output_vec = FpVector::new(p, 0);
        for i in 1..max_degree {
            let output_dim = algebra.dimension(i, -1);
            output_vec.set_scratch_vector_size(output_dim);
            let relations = algebra.generating_relations(i);
            println!("{:?}", relations);
            for relation in relations {
                for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    algebra.multiply_basis_elements(
                        output_vec.as_slice_mut(),
                        *coeff,
                        *deg_1,
                        *idx_1,
                        *deg_2,
                        *idx_2,
                        -1,
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

    #[test]
    fn test_clone_into() {
        let mut other = MilnorBasisElement::default();

        let mut check = |a: &MilnorBasisElement| {
            a.clone_into(&mut other);
            assert_eq!(a, &other);
        };

        check(&MilnorBasisElement {
            q_part: 3,
            p_part: vec![3, 2],
            degree: 12,
        });
        check(&MilnorBasisElement {
            q_part: 1,
            p_part: vec![3],
            degree: 11,
        });
        check(&MilnorBasisElement {
            q_part: 5,
            p_part: vec![1, 3, 5, 2],
            degree: 7,
        });
        check(&MilnorBasisElement {
            q_part: 0,
            p_part: vec![],
            degree: 2,
        });
    }

    #[test]
    fn test_ppart_multiplier_2() {
        let r = vec![1, 4];
        let s = vec![2, 4];
        let mut m = PPartMultiplier::<false>::new_from_allocation(
            ValidPrime::new(2),
            &r,
            &s,
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
        let r = vec![3, 4];
        let s = vec![1, 4];
        let mut m = PPartMultiplier::<false>::new_from_allocation(
            ValidPrime::new(3),
            &r,
            &s,
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

impl MilnorAlgebra {
    /// Returns `true` if the new element is not within the bounds
    fn increment_p_part(element: &mut PPart, max: &[PPartEntry]) -> bool {
        element[0] += 1;
        for i in 0..element.len() - 1 {
            if element[i] > max[i] {
                element[i] = 0;
                element[i + 1] += 1;
            }
        }
        element.last().unwrap() > max.last().unwrap()
    }
}

impl Bialgebra for MilnorAlgebra {
    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)> {
        assert_eq!(*self.prime(), 2, "Coproduct at odd primes not supported");
        if op_deg == 0 {
            return vec![(0, 0, 0, 0)];
        }
        let xi_degrees = combinatorics::xi_degrees(self.prime());

        let mut len = 1;
        let p_part = &self.basis_element_from_index(op_deg, op_idx).p_part;

        for i in p_part.iter() {
            len *= i + 1;
        }
        let len = len as usize;
        let mut result = Vec::with_capacity(len);

        let mut cur_ppart: PPart = vec![0; p_part.len()];
        loop {
            let mut left_degree: i32 = 0;
            for i in 0..cur_ppart.len() {
                left_degree += cur_ppart[i] as i32 * xi_degrees[i];
            }
            let right_degree: i32 = op_deg - left_degree;

            let mut left_ppart = cur_ppart.clone();
            while let Some(0) = left_ppart.last() {
                left_ppart.pop();
            }

            let mut right_ppart = cur_ppart
                .iter()
                .enumerate()
                .map(|(i, v)| p_part[i] - *v)
                .collect::<Vec<_>>();
            while let Some(0) = right_ppart.last() {
                right_ppart.pop();
            }

            let left_idx = self.basis_element_to_index(&MilnorBasisElement {
                degree: left_degree,
                q_part: 0,
                p_part: left_ppart,
            });
            let right_idx = self.basis_element_to_index(&MilnorBasisElement {
                degree: right_degree,
                q_part: 0,
                p_part: right_ppart,
            });

            result.push((left_degree, left_idx, right_degree, right_idx));
            if Self::increment_p_part(&mut cur_ppart, p_part) {
                break;
            }
        }
        result
    }
    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)> {
        vec![(op_deg, op_idx)]
    }
}
