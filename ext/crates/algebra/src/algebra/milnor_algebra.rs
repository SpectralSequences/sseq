use itertools::Itertools;
use parking_lot::Mutex;
use serde_json::value::Value;
use std::collections::HashMap;

use once::OnceVec;
use fp::prime::{integer_power, ValidPrime, BitflagIterator};
use fp::vector::{FpVector, FpVectorT};
use crate::algebra::combinatorics;
use crate::algebra::{Algebra, Bialgebra};

use nom::{
    IResult,
    combinator::map,
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, space1},
    sequence::{delimited, pair},
};

// This is here so that the Python bindings can use modules defined for AdemAlgebraT with their own algebra enum.
// In order for things to work AdemAlgebraT cannot implement Algebra.
// Otherwise, the algebra enum for our bindings will see an implementation clash.
pub trait MilnorAlgebraT : Send + Sync + 'static + Algebra {
    fn milnor_algebra(&self) -> &MilnorAlgebra;
}


pub struct MilnorProfile {
    pub truncated : bool,
    pub q_part : u32,
    pub p_part : Vec<u32>
}

impl MilnorProfile {
    pub fn is_trivial(&self) -> bool {
        !self.truncated && self.q_part == !0 && self.p_part.is_empty()
    }
}

#[derive(Default, Clone)]
pub struct QPart {
    degree : i32,
    q_part : u32
}

type PPart = Vec<u32>;

#[derive(Debug, Clone)]
pub struct MilnorBasisElement {
    pub q_part : u32,
    pub p_part : PPart,
    pub degree : i32
}

const ZERO_QPART : QPart = QPart { degree : 0, q_part : 0 };

fn from_p (p : PPart, dim : i32) -> MilnorBasisElement {
    MilnorBasisElement { p_part : p, q_part : 0, degree : dim }
}

impl std::cmp::PartialEq for MilnorBasisElement {
    fn eq(&self, other : &Self) -> bool {
        self.p_part == other.p_part && self.q_part == other.q_part
    }
}

impl std::cmp::Eq for MilnorBasisElement {}

impl std::hash::Hash for MilnorBasisElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.p_part.hash(state);
        self.q_part.hash(state);
    }
}

impl std::fmt::Display for MilnorBasisElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        if self.degree == 0 {
            write!(f, "1")?;
            return Ok(());
        }
        let mut parts = Vec::new();
        if self.q_part != 0 {            
            let q_part_str = BitflagIterator::set_bit_iterator(self.q_part as u64)
                .map(|idx| format!("Q_{}", idx))
                .join(" ");
            parts.push(q_part_str);
        }
        if !self.p_part.is_empty() {
            parts.push(format!("P({})", self.p_part.iter().join(", ")));
        }
        write!(f, "{}", parts.join(" "))?;
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
    pub profile : MilnorProfile,
    name : String,
    next_degree : Mutex<i32>,
    p : ValidPrime,
    pub generic : bool,
    ppart_table : OnceVec<Vec<PPart>>,
    qpart_table : Vec<OnceVec<QPart>>,
    basis_table : OnceVec<Vec<MilnorBasisElement>>,
    basis_element_to_index_map : OnceVec<HashMap<MilnorBasisElement, usize>>, // degree -> MilnorBasisElement -> index
    #[cfg(feature = "cache-multiplication")]
    multiplication_table : OnceVec<OnceVec<Vec<Vec<FpVector>>>> // source_deg -> target_deg -> source_op -> target_op
}

impl MilnorAlgebra {
    pub fn new(p : ValidPrime) -> Self {
        fp::vector::initialize_limb_bit_index_table(p);

        let profile = MilnorProfile {
            truncated: false,
            q_part : !0,
            p_part : Vec::new()
        };

        let mut qpart_table = Vec::new();
        qpart_table.resize_with((2 * *p - 2) as usize, OnceVec::new);

        Self {
            p,
            generic : *p != 2,
            profile,
            name : format!("MilnorAlgebra(p={})", p),
            next_degree : Mutex::new(0),
            ppart_table : OnceVec::new(),
            qpart_table,
            basis_table : OnceVec::new(),
            basis_element_to_index_map : OnceVec::new(),
            #[cfg(feature = "cache-multiplication")]
            multiplication_table : OnceVec::new()
        }
    }

    pub fn q(&self) -> i32 {
        if self.generic { 2*(*self.prime() as i32 - 1) } else { 1 }
    }

    pub fn basis_element_from_index(&self, degree : i32, idx : usize) -> &MilnorBasisElement {
        &self.basis_table[degree as usize][idx]
    }

    pub fn try_basis_element_to_index(&self, elt : &MilnorBasisElement) -> Option<usize> {
        self.basis_element_to_index_map[elt.degree as usize].get(elt).copied()
    }

    pub fn basis_element_to_index(&self, elt : &MilnorBasisElement) -> usize {
        self.try_basis_element_to_index(elt).unwrap_or_else(|| panic!("Didn't find element: {:?}", elt))
    }
}

impl Algebra for MilnorAlgebra {
    fn algebra_type(&self) -> &str {
        "milnor"
    }

    fn prime(&self) -> ValidPrime {
        self.p
    }

    fn name(&self) -> &str {
        &self.name
    }

    #[allow(clippy::useless_let_if_seq)]
    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        let mut products = Vec::with_capacity(4);
        let max_degree;
        if self.generic {
            if self.profile.q_part & 1 != 0  {
                products.push(("a_0".to_string(), MilnorBasisElement {
                    degree : 1,
                    q_part : 1,
                    p_part : vec![]
                }));
            }
            if (self.profile.p_part.is_empty() && !self.profile.truncated) ||
               (!self.profile.p_part.is_empty() && self.profile.p_part[0] > 0) {
                    products.push(("h_0".to_string(), MilnorBasisElement {
                        degree : (2* (*self.prime())-2) as i32,
                        q_part : 0,
                        p_part : vec![1]
                    }));
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
                products.push((format!("h_{}", i), MilnorBasisElement {
                    degree,
                    q_part : 0,
                    p_part : vec![1 << i],
                }));
            }
            max_degree = 1 << 3;
        }
        self.compute_basis(max_degree + 1);

        products.into_iter()
            .map(|(name, b)| (name, b.degree, self.basis_element_to_index(&b)))
            .collect()
    }

    fn max_computed_degree(&self) -> i32 {
        *self.next_degree.lock() - 1
    }

    fn compute_basis(&self, max_degree : i32) {
        let mut next_degree = self.next_degree.lock();

        if max_degree < *next_degree {
            return;
        }

        self.compute_ppart(*next_degree, max_degree);
        self.compute_qpart(*next_degree, max_degree);

        self.basis_table.reserve((max_degree - *next_degree + 1) as usize);
        self.basis_element_to_index_map.reserve((max_degree - *next_degree + 1) as usize);

        if self.generic {
            self.generate_basis_generic(*next_degree, max_degree);
        } else {
            self.generate_basis_2(*next_degree, max_degree);
        }

        // Populate hash map
        for d in *next_degree as usize ..= max_degree as usize {
            let basis = &self.basis_table[d];
            let mut map = HashMap::with_capacity(basis.len());
            for (i, b) in basis.iter().enumerate() {
                map.insert(b.clone(), i);
            }
            self.basis_element_to_index_map.push(map);
        }

        #[cfg(feature = "cache-multiplication")]
        {
            for d in 0 ..= max_degree as usize {
                if self.multiplication_table.len() == d {
                    self.multiplication_table.push(OnceVec::new());
                }
                for e in self.multiplication_table[d].len() ..= max_degree as usize  - d {
                    self.multiplication_table[d].push(
                        (0..self.dimension(d as i32, -1)).map(|i|
                            (0 .. self.dimension(e as i32, -1)).map(|j| {
                                let mut res = FpVector::new(self.prime(), self.dimension((d + e) as i32, -1));
                                self.multiply(&mut res, 1, &self.basis_table[d][i], &self.basis_table[e][j]);
                                res
                            }).collect::<Vec<_>>()
                        ).collect::<Vec<_>>());
                }
            }
        }

        *next_degree = max_degree + 1;
    }

    fn dimension(&self, degree : i32, _excess : i32) -> usize {
        if degree < 0 {
            return 0;
        }
        self.basis_table[degree as usize].len()
    }

    #[cfg(not(feature = "cache-multiplication"))]
    fn multiply_basis_elements(&self, result : &mut FpVector, coef : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, _excess : i32) {
        self.multiply(result, coef, &self.basis_table[r_degree as usize][r_idx], &self.basis_table[s_degree as usize][s_idx]);
    }

    #[cfg(feature = "cache-multiplication")]
    fn multiply_basis_elements(&self, result : &mut FpVector, coef : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, _excess : i32) {
        result.shift_add(&self.multiplication_table[r_degree as usize][s_degree as usize][r_idx][s_idx], coef);
    }

    fn json_to_basis(&self, json : Value) -> error::Result<(i32, usize)> {
        let xi_degrees = combinatorics::xi_degrees(self.prime());
        let tau_degrees = combinatorics::tau_degrees(self.prime());

        let mut p_part = Vec::new();
        let mut q_part = 0;
        let mut degree = 0;

        if self.generic {
            let (q_list, p_list): (Vec<u8>, Vec<u32>) = serde_json::from_value(json)?;
            let q = (2 * (*self.prime()) - 2) as i32;

            for (i, val) in p_list.into_iter().enumerate() {
                p_part.push(val);
                degree += (val as i32) * xi_degrees[i] * q;
            }

            for k in q_list {
                q_part |= 1 << k;
                degree += tau_degrees[k as usize];
            }
        } else {
            let p_list: Vec<u32> = serde_json::from_value(json)?;
            for (i, val) in p_list.into_iter().enumerate() {
                p_part.push(val);
                degree += (val as i32) * xi_degrees[i];
            }
        }
        let m = MilnorBasisElement { p_part, q_part, degree };
        Ok((degree, self.basis_element_to_index(&m)))
    }

    fn json_from_basis(&self, degree : i32, index : usize) -> Value {
        let b = self.basis_element_from_index(degree, index);
        if self.generic {
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

    // Same implementation as AdemAlgebra
    fn string_to_generator<'a, 'b>(&'a self, input: &'b str) -> IResult<&'b str, (i32, usize)> {
        let first = map(alt((
            delimited(char('P'), digit1, space1),
            delimited(tag("Sq"), digit1, space1),
        )), |elt| {
            let i : u32 = std::str::FromStr::from_str(elt).unwrap();
            self.beps_pn(0, i)
        });

        let second = map(pair(char('b'), space1), |_| (1, 0));

        alt((first, second))(input)
    }

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

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("{}", self.basis_table[degree as usize][idx])
    }

    fn generators(&self, degree : i32) -> Vec<usize> {
        if degree == 0 {
            return vec![];
        }
        if self.generic && degree == 1 {
            return vec![0]; // Q_0
        }
        let p = *self.prime();
        let q = if self.generic { 2 * p - 2 } else { 1 };
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
        if (self.profile.p_part.is_empty() && self.profile.truncated) ||
           (!self.profile.p_part.is_empty() && self.profile.p_part[0] <= power) {
            return vec![];
        }

        let idx = self.basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part : 0,
            p_part : vec![degree as u32/q]
        });
        return vec![idx];
    }

    fn decompose_basis_element(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let basis = &self.basis_table[degree as usize][idx];
        // If qpart = 0, return self
        if basis.q_part == 0 {
            self.decompose_basis_element_ppart(degree, idx)
        } else {
            self.decompose_basis_element_qpart(degree, idx)
        }
    }

    fn relations_to_check(&self, degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>{
        if self.generic && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }
        let p = self.prime();
        let inadmissible_pairs = combinatorics::inadmissible_pairs(p, self.generic, degree);
        let mut result = Vec::new();
        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation. Sometimes these don't exist because of profiles. Then just ignore it.
            (|| {
                let (first_degree, first_index) = self.try_beps_pn(0, x)?;
                let (second_degree, second_index) = self.try_beps_pn(b, y)?;
                relation.push((*p - 1, (first_degree, first_index), (second_degree, second_index)));
                for e1 in 0 ..= b {
                    let e2 = b - e1;
                    // e1 and e2 determine where a bockstein shows up.
                    // e1 determines whether a bockstein shows up in front
                    // e2 determines whether a bockstein shows up in middle
                    // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                    for j in 0 ..= x / *p {
                        let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                        if c == 0 { continue; }
                        if j == 0 {
                            relation.push((c, self.try_beps_pn(e1, x + y)?, (e2 as i32, 0)));
                            continue;
                        }
                        let first_sq = self.try_beps_pn(e1, x + y - j)?;
                        let second_sq = self.try_beps_pn(e2, j)?;
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
    fn compute_ppart(&self, mut next_degree : i32, max_degree : i32) {
        if next_degree == 0 {
            self.ppart_table.push(vec![Vec::new()]);
            next_degree = 1;
        }

        let p = *self.prime() as i32;
        let q = if p == 2 {1} else {2 * p - 2};
        let new_deg = max_degree/q;
        let old_deg = (next_degree-1)/q;

        self.ppart_table.reserve((new_deg - old_deg) as usize);

        let xi_degrees = combinatorics::xi_degrees(self.prime());
        let mut profile_list = Vec::with_capacity(xi_degrees.len());
        for i in 0..xi_degrees.len() {
            if i < self.profile.p_part.len() {
                profile_list.push(fp::prime::integer_power(*self.prime(), self.profile.p_part[i]) - 1);
            } else if self.profile.truncated {
                profile_list.push(0);
            } else {
                profile_list.push(std::u32::MAX);
            }
        }
        for d in (old_deg + 1) ..= new_deg {
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
//            new_row.shrink_to_fit();
            self.ppart_table.push(new_row);
        }
    }

    fn compute_qpart(&self, next_degree : i32, max_degree : i32) {
        let q = (2 * (*self.prime()) - 2) as i32;
        let profile = !self.profile.q_part;

        if !self.generic {
            return;
        }

        let mut next_degree = next_degree;
        if next_degree == 0 {
            self.qpart_table[0].push( ZERO_QPART.clone());
            next_degree = 1;
        }

        let tau_degrees = combinatorics::tau_degrees(self.prime());
        let old_max_tau = tau_degrees.iter().position(|d| *d > next_degree - 1).unwrap(); // Use expect instead
        let new_max_tau = tau_degrees.iter().position(|d| *d > max_degree).unwrap();

        let bit_string_min : u32 = 1 << old_max_tau;
        let bit_string_max : u32 = 1 << new_max_tau;

        let mut residue : i32 = (old_max_tau as i32) % q;
        let mut total : i32 = tau_degrees[0 .. old_max_tau].iter().sum();

        for bit_string in bit_string_min..bit_string_max {
            // v has all the trailing zeros set. These are the bits that were set last time,
            // but aren't set anymore because of a carry. Shift right 1 because ???1000 ==> ???0111 xor ???1000 = 0001111.
            let mut v = (bit_string ^ (bit_string - 1)) >> 1;
            let mut c : usize = 0; // We're going to get the new bit that is set into c.
            while v != 0 {
                v >>= 1; // Subtract off the degree of each of the lost entries
                total -= tau_degrees [c];
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
                degree : total,
                q_part : bit_string
            });
        }
    }

    fn generate_basis_generic(&self, next_degree : i32, max_degree : i32) {
        let q = (2 * (*self.prime()) - 2) as usize;

        for d in next_degree as usize..= max_degree as usize {
            let mut new_table = Vec::new(); // Initialize size

            for q_part in self.qpart_table[d % q].iter() {
                // Elements in qpart_table are listed in increasing order in
                // degree. Abort if degree too large.
                if q_part.degree > d as i32 {
                    break;
                }

                for p_part in &self.ppart_table[(d - (q_part.degree as usize))/q] {
                    new_table.push( MilnorBasisElement { p_part : p_part.clone(), q_part : q_part.q_part, degree : d as i32 } );
                }
            }
//            new_table.shrink_to_fit();
            self.basis_table.push(new_table);
        }
    }

    fn generate_basis_2(&self, next_degree : i32, max_degree : i32) {
        for i in next_degree as usize ..= max_degree as usize {
            self.basis_table.push(
                self.ppart_table[i]
                .iter()
                .map(|p| from_p(p.clone(), i as i32))
                .collect());
        }
    }
}

// Multiplication logic
impl MilnorAlgebra {
    fn try_beps_pn(&self, e: u32, x: u32) -> Option<(i32, usize)> {
        let p = *self.prime();
        let q = if self.generic { 2*(p - 1) } else { 1 };
        let degree = (q * x + e) as i32;
        self.try_basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part : e,
            p_part : vec![x]
        }).map(|index| (degree, index))
    }

    fn beps_pn(&self, e : u32, x : u32) -> (i32, usize) {
        self.try_beps_pn(e, x).unwrap()
    }

    fn multiply_qpart (&self, m1 : &MilnorBasisElement, f : u32) -> Vec<(u32, MilnorBasisElement)>{
        let mut new_result : Vec<(u32, MilnorBasisElement)> = vec![(1, m1.clone())];
        let mut old_result : Vec<(u32, MilnorBasisElement)> = Vec::new();

        for k in BitflagIterator::set_bit_iterator(f as u64) {
            let k = k as u32;
            let pk = integer_power(*self.p, k);
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
                for i in 0..= term.p_part.len() {
                    // If there is already Q_{k+i} on the other side, the result is 0
                    if term.q_part & (1 << (k + i as u32)) != 0 {
                        continue;
                    }
                    // Check if R - p^k e_i < 0. Only do this from the first term onwards.
                    if i > 0 && term.p_part[i-1] < pk {
                        continue;
                    }

                    let mut new_p = term.p_part.clone();
                    if i > 0 {
                        new_p[i-1] -= pk;
                    }

                    // Now calculate the number of Q's we are moving past
                    let larger_q = (term.q_part >> (k + i as u32 + 1)).count_ones();

                    // If new_p ends with 0, drop them
                    while let Some(0) = new_p.last() {
                        new_p.pop();
                    }
                    // Now put everything together
                    let m = MilnorBasisElement {
                        p_part : new_p,
                        q_part : term.q_part | 1 << (k + i as u32),
                        degree : 0 // we don't really care about the degree here. The final degree of the whole calculation is known a priori
                    };
                    let c = if larger_q % 2 == 0 { *coef } else { *coef * (*self.prime() - 1) };

                    new_result.push((c, m));
                }
            }
        }
        new_result
    }

    fn multiply(&self, res : &mut FpVector, coef : u32, m1 : &MilnorBasisElement, m2 : &MilnorBasisElement) {
        let target_dim = m1.degree + m2.degree;
        if self.generic {
            let m1f = self.multiply_qpart(m1, m2.q_part);
            for (cc, basis) in m1f {
                let mut multiplier = PPartMultiplier::new(self.prime(), &(basis.p_part), &(m2.p_part), false);
                let mut new = MilnorBasisElement {
                    degree : target_dim,
                    q_part : basis.q_part,
                    p_part : Vec::with_capacity(multiplier.diag_num)
                };
                while let Some(c) = multiplier.next(&mut new) {
                    let idx = self.basis_element_to_index(&new);
                    res.add_basis_element(idx, c * cc * coef);
                }
            }
        } else {
            let mut multiplier = PPartMultiplier::new(self.prime(), &(m1.p_part), &(m2.p_part), false);
            let mut new = MilnorBasisElement {
                degree: target_dim,
                q_part: 0,
                p_part: Vec::with_capacity(multiplier.diag_num)
            };
            while let Some(c) = multiplier.next(&mut new) {
                let idx = self.basis_element_to_index(&new);
                res.add_basis_element(idx, c * coef);
            }
        }
    }

}

struct Matrix2D {
    cols: usize,
    inner: Vec<u32>,
}

impl Matrix2D {
    fn new(rows: usize, cols: usize) -> Self {
        Matrix2D {
            cols,
            inner: vec![0; rows * cols]
        }
    }
}

impl std::ops::Index<usize> for Matrix2D {
    type Output = [u32];

    fn index(&self, row: usize) -> &Self::Output {
        &self.inner[row * self.cols .. (row + 1) * self.cols]
    }
}

impl std::ops::IndexMut<usize> for Matrix2D {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.inner[row * self.cols .. (row + 1) * self.cols]
    }
}

#[allow(non_snake_case)]
pub struct PPartMultiplier<'a> {
    p : ValidPrime,
    M : Matrix2D,
    r : &'a PPart,
    mod_4: bool,
    rows : usize,
    cols : usize,
    diag_num : usize,
    init : bool,
    diagonal: Vec<u32>,
}

#[allow(non_snake_case)]
impl<'a>  PPartMultiplier<'a> {
    fn prime(&self) -> ValidPrime {
        self.p
    }

    #[allow(clippy::ptr_arg)]
    fn new (p : ValidPrime, r : &'a PPart, s : &'a PPart, mod_4: bool) -> PPartMultiplier<'a> {
        if mod_4 {
            assert_eq!(*p, 2);
        }
        let rows = r.len() + 1;
        let cols = s.len() + 1;
        let diag_num = r.len() + s.len();
        let diagonal = Vec::with_capacity(std::cmp::max(rows, cols));

        let mut M = Matrix2D::new(rows, cols);

        for i in 1 .. rows {
            M[i][0] = r[i - 1];
        }
        M[0][1..cols].clone_from_slice(&s[0..(cols - 1)]);

        PPartMultiplier { p, M, r, rows, cols, diag_num, diagonal, init : true, mod_4 }
    }

    fn update(&mut self) -> bool {
        for i in 1..self.rows {
            let mut total = self.M[i][0];
            let mut p_to_the_j = 1;
            for j in 1..self.cols {
                p_to_the_j *= *self.prime();
                if total < p_to_the_j {
                    // We don't have enough weight left in the entries above this one in the column to increment this cell.
                    // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                    total += self.M[i][j] * p_to_the_j;
                    continue;
                }
                // Check if any entry in column j above row i is nonzero. I'm still not sure why tbh.
                if (0..i).find(|&k| self.M[k][j] != 0).is_some() {
                    // If so, we found our next matrix.
                    for row in 1..i {
                        self.M[row][0] = self.r[row-1];
                        for col in 1..self.cols{
                            self.M[0][col] += self.M[row][col];
                            self.M[row][col] = 0;
                        }
                    }
                    for col in 1..j {
                        self.M[0][col] += self.M[i][col];
                        self.M[i][col] = 0;
                    }
                    self.M[0][j] -= 1;
                    self.M[i][j] += 1;
                    self.M[i][0] = total - p_to_the_j;
                    return true;
                }
                // All the cells above this one are zero so we didn't find our next matrix.
                // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                total += self.M[i][j] * p_to_the_j;
            }
        }
        false
    }

    fn next(&mut self, basis: &mut MilnorBasisElement) -> Option<u32> {
        let new_p = &mut basis.p_part;
        new_p.clear();
        let mut coef = 1;

        if self.init {
            self.init = false;
            for i in 1 .. std::cmp::min(self.cols, self.rows) {
                if self.mod_4 {
                    coef *= fp::prime::binomial4(self.M[i][0] + self.M[0][i], self.M[0][i]);
                } else {
                    coef *= fp::prime::binomial(self.prime(), (self.M[i][0] + self.M[0][i]) as i32, self.M[0][i] as i32);
                }
            }
            if coef == 0 {
                self.next(basis)
            } else {
                new_p.extend_from_slice(&self.M[0][1..self.cols]);
                if self.rows > self.cols {
                    new_p.resize(self.r.len(), 0);
                }
                for (i, &entry) in self.r.iter().enumerate() {
                    new_p[i] += entry;
                }
                Some(coef)
            }
        } else if self.update() {
            for diag_idx in 1..=self.diag_num {
                let i_min = if diag_idx + 1 > self.cols { diag_idx + 1 - self.cols } else {0} ;
                let i_max = std::cmp::min(1 + diag_idx, self.rows);
                let mut sum = 0;

                self.diagonal.clear();

                for i in i_min..i_max {
                    self.diagonal.push(self.M[i][diag_idx - i]);
                    sum += self.M[i][diag_idx - i];
                }
                new_p.push(sum);

                if sum == 0  {
                    continue;
                }
                if self.mod_4 {
                    if coef == 2 {
                        coef *= fp::prime::multinomial2(&self.diagonal);
                    } else {
                        coef *= fp::prime::multinomial4(&self.diagonal);
                    }
                    coef %= 4;
                } else {
                    coef *= fp::prime::multinomial(self.prime(), &mut self.diagonal);
                    coef %= *self.prime();
                }
                if coef == 0 {
                    return self.next(basis);
                }
            }
            // If new_p ends with 0, drop them
            while let Some(0) = new_p.last() {
                new_p.pop();
            }

            Some(coef)
        } else {
            None
        }
    }
}

impl MilnorAlgebra {
    fn decompose_basis_element_qpart(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))>{
        let basis = &self.basis_table[degree as usize][idx];
        // Look for left-most non-zero qpart
        let i = basis.q_part.trailing_zeros();
        // If the basis element is just Q_{k+1}, we decompose Q_{k+1} = P(p^k) Q_k - Q_k P(p^k).
        if basis.q_part == 1 << i && basis.p_part.is_empty() {
            let ppow = fp::prime::integer_power(*self.prime(), i - 1);

            let q_degree = (2 * ppow - 1) as i32;
            let p_degree = (ppow * (2 * (*self.prime()) - 2)) as i32;

            let p_idx = self.basis_element_to_index(&from_p(vec![ppow], p_degree)).to_owned();

            let q_idx =  self.basis_element_to_index(
                &MilnorBasisElement {
                    q_part : 1 << (i-1),
                    p_part : Vec::new(),
                    degree : q_degree
                }).to_owned();

            return vec![(1, (p_degree, p_idx), (q_degree, q_idx)), (*self.prime() - 1, (q_degree, q_idx), (p_degree, p_idx))];
        }

        // Otherwise, separate out the first Q_k.
        let first_degree = combinatorics::tau_degrees(self.prime())[i as usize];
        let second_degree = degree - first_degree;

        let first_idx = self.basis_element_to_index(
            &MilnorBasisElement {
                q_part : 1 << i,
                p_part : Vec::new(),
                degree : first_degree
            });

        let second_idx = self.basis_element_to_index(
            &MilnorBasisElement {
                q_part : basis.q_part ^ 1 << i,
                p_part : basis.p_part.clone(),
                degree : second_degree
            });

        vec![(1, (first_degree, first_idx), (second_degree, second_idx))]        
    }

    // use https://monks.scranton.edu/files/pubs/bases.pdf page 8
    #[allow(clippy::useless_let_if_seq)]
    fn decompose_basis_element_ppart(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))>{
        let p = self.prime();
        let b = &self.basis_table[degree as usize][idx];
        let first;
        let second;
        if b.p_part.len() > 1 {
            let mut t1 = 0;
            let mut pow = 1;
            for r in &b.p_part {
                t1 += r * pow;
                pow *= *p;
            }
            first = self.beps_pn(0, t1);
            let second_degree = degree - first.0;
            let second_idx = self.basis_element_to_index(&MilnorBasisElement {
                q_part : 0,
                p_part : b.p_part[1..].to_vec(),
                degree : second_degree
            });
            second = (second_degree, second_idx);
        } else {
            // return vec![(1, (degree, idx), (0, 0))];
            let sq = b.p_part[0];
            let mut pow = 1;
            {
                let mut temp_sq = sq;
                while temp_sq % *p == 0 {
                    temp_sq /= *p;
                    pow *= *p;
                }
            }
            if sq == pow {
                return vec![(1, (degree, idx), (0, 0))];
            }
            first = self.beps_pn(0, pow);
            second = self.beps_pn(0, sq - pow);
        }
        let mut out_vec = FpVector::new(p, self.dimension(degree, -1));
        self.multiply_basis_elements(&mut out_vec, 1, first.0, first.1, second.0, second.1, -1);
        let mut result = Vec::new();
        let c = out_vec.entry(idx);
        assert!(c != 0);
        out_vec.set_entry(idx, 0);
        let c_inv = fp::prime::inverse(p, *p - c);
        result.push((((*p - 1) * c_inv) % *p, first, second));
        for (i, v) in out_vec.iter_nonzero() {
            for (c, t1, t2) in self.decompose_basis_element_ppart(degree, i){
                result.push(((c_inv * c * v) % *p, t1, t2));
            }
        }
        result
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest(p, max_degree,
        case(2, 32),
        case(3, 106)    
    )]
    fn test_milnor_basis(p : u32, max_degree : i32){
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p);//p != 2
        algebra.compute_basis(max_degree);
        for i in 1 .. max_degree {
            let dim = algebra.dimension(i, -1);
            for j in 0 .. dim {
                let b = algebra.basis_element_from_index(i, j);
                assert_eq!(algebra.basis_element_to_index(&b), j);
                let json = algebra.json_from_basis(i, j);
                let new_b = algebra.json_to_basis(json).unwrap();
                assert_eq!(new_b, (i, j));
            }
        }
    }

    #[rstest(p, max_degree,
        case(2, 32),
        case(3, 106)    
    )]
    fn test_milnor_decompose(p : u32, max_degree : i32){
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p);
        algebra.compute_basis(max_degree);
        for i in 1 .. max_degree {
            let dim = algebra.dimension(i, -1);
            let gens = algebra.generators(i);
            // println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim);
            for j in 0 .. dim {
                if gens.contains(&j){
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in algebra.decompose_basis_element(i, j) {
                    // print!("{} * {} * {}  +  ", coeff, algebra.basis_element_to_string(first_degree,first_idx), algebra.basis_element_to_string(second_degree, second_idx));
                    algebra.multiply_basis_elements(&mut out_vec, coeff, first_degree, first_idx, second_degree, second_idx, -1);
                }
                assert!(out_vec.entry(j) == 1, 
                    format!("{} != {}", algebra.basis_element_to_string(i, j), algebra.element_to_string(i, &out_vec)));
                out_vec.set_entry(j, 0);
                assert!(out_vec.is_zero(), 
                    format!("\n{} != {}", 
                        algebra.basis_element_to_string(i, j), algebra.element_to_string(i, &out_vec)));
            }
        }
    }

    use crate::module::ModuleFailedRelationError;
    #[rstest(p, max_degree,
        case(2, 32),
        case(3, 106)    
    )]
    fn test_adem_relations(p : u32, max_degree : i32){
        let p = ValidPrime::new(p);
        let algebra = MilnorAlgebra::new(p); // , p != 2
        algebra.compute_basis(max_degree + 2);
        let mut output_vec = FpVector::new(p, 0);
        for i in 1 .. max_degree {
            output_vec.clear_slice();
            let output_dim = algebra.dimension(i, -1);
            if output_dim > output_vec.dimension() {
                output_vec = FpVector::new(p, output_dim);
            }
            output_vec.set_slice(0, output_dim);
            let relations = algebra.relations_to_check(i);
            println!("{:?}", relations);
            for relation in relations {
                for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    algebra.multiply_basis_elements(&mut output_vec, *coeff, *deg_1, *idx_1, *deg_2, *idx_2, -1);
                }
                if !output_vec.is_zero() {
                    let mut relation_string = String::new();
                    for (coeff, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                        relation_string.push_str(&format!("{} * {} * {}  +  ", 
                            *coeff, 
                            &algebra.basis_element_to_string(*deg_1, *idx_1), 
                            &algebra.basis_element_to_string(*deg_2, *idx_2))
                        );
                    }
                    relation_string.pop(); relation_string.pop(); relation_string.pop();
                    relation_string.pop(); relation_string.pop();
                    let value_string = algebra.element_to_string(i as i32, &output_vec);
                    panic!("{}", ModuleFailedRelationError {relation : relation_string, value : value_string});
                }
            }
        }
    }    
}

impl MilnorAlgebra {
    /// Returns `true` if the new element is not within the bounds
    fn increment_p_part(element: &mut Vec<u32>, max : &[u32]) -> bool {
        element[0] += 1;
        for i in 0 .. element.len() - 1{
            if element[i] > max[i] {
                element[i] = 0;
                element[i + 1] += 1;
            }
        }
        element.last().unwrap() > max.last().unwrap()
    }
}

impl Bialgebra for MilnorAlgebra {
    fn coproduct(&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize, i32, usize)> {
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

        let mut cur_ppart : Vec<u32> = vec![0; p_part.len()];
        loop {
            let mut left_degree : i32 = 0;
            for i in 0 .. cur_ppart.len() {
                left_degree += cur_ppart[i] as i32 * xi_degrees[i];
            }
            let right_degree : i32 = op_deg - left_degree;

            let mut left_ppart = cur_ppart.clone();
            while let Some(0) = left_ppart.last() {
                left_ppart.pop();
            }

            let mut right_ppart = cur_ppart.iter().enumerate().map(|(i, v)| p_part[i] - *v).collect::<Vec<_>>();
            while let Some(0) = right_ppart.last() {
                right_ppart.pop();
            }

            let left_idx = self.basis_element_to_index(&MilnorBasisElement {
                    degree : left_degree,
                    q_part : 0,
                    p_part : left_ppart
                });
            let right_idx = self.basis_element_to_index(&MilnorBasisElement {
                    degree: right_degree,
                    q_part : 0,
                    p_part : right_ppart
                });

            result.push((left_degree, left_idx, right_degree, right_idx));
            if Self::increment_p_part(&mut cur_ppart, p_part) {
                break;
            }
        }
        result
    }
    fn decompose(&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize)> {
        vec![(op_deg, op_idx)]
    }
}
