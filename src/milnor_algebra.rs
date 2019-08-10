use serde_json::value::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::combinatorics;
use crate::fp_vector::{FpVector, FpVectorT};
use crate::once::OnceVec;
use crate::algebra::Algebra;



pub struct MilnorProfile {

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
        let mut qpart = self.q_part;
        if qpart != 0 {
            let mut i = 0;
            while qpart != 0 {
                if qpart & 1 != 0 {
                    write!(f, "Q_{} ", i)?;
                }
                qpart = qpart >> 1;
                i += 1;
            }
        }
        if self.p_part.len() > 0 {
            write!(f, "P(")?;
            write!(f, "{}", self.p_part.iter()
                   .map(u32::to_string)
                   .collect::<Vec<String>>()
                   .join(", "))?;
            write!(f, ")")?;
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
    pub profile : MilnorProfile,
    name : String,
    next_degree : Mutex<i32>,
    p : u32,
    pub generic : bool,
    ppart_table : OnceVec<Vec<PPart>>,
    qpart_table : Vec<OnceVec<QPart>>,
    basis_table : OnceVec<Vec<MilnorBasisElement>>,
    basis_element_to_index_map : OnceVec<HashMap<MilnorBasisElement, usize>>, // degree -> MilnorBasisElement -> index
    filtration_one_products : Vec<(String, i32, usize)>
}

impl MilnorAlgebra {
    pub fn new(p : u32) -> Self {
        crate::fp_vector::initialize_limb_bit_index_table(p);

        let profile = MilnorProfile { };

        let mut qpart_table = Vec::new();
        qpart_table.resize_with((2 * p - 2) as usize, OnceVec::new);

        Self {
            p,
            generic : p != 2,
            profile: profile,
            name : format!("MilnorAlgebra(p={})", p),
            next_degree : Mutex::new(0),
            ppart_table : OnceVec::new(),
            qpart_table,
            basis_table : OnceVec::new(),
            basis_element_to_index_map : OnceVec::new(),
            filtration_one_products : Vec::new()
        }
    }

    pub fn basis_element_from_index(&self, degree : i32, idx : usize) -> &MilnorBasisElement {
        &self.basis_table[degree as usize][idx]
    }

    pub fn basis_element_to_index(&self, elt : &MilnorBasisElement) -> usize {
        if let Some(idx) = self.basis_element_to_index_map[elt.degree as usize].get(elt) {
            *idx
        } else {
            println!("Didn't find element: {:?}", elt);
            assert!(false);
            0
        }
    }
}

impl Algebra for MilnorAlgebra {
    fn get_algebra_type(&self) -> &str {
        "milnor"
    }

    fn get_prime(&self) -> u32 {
        self.p
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_filtration_one_products(&self) -> &Vec<(String, i32, usize)>{
        &self.filtration_one_products
    }

    fn set_default_filtration_one_products(&mut self) {

    }

    fn compute_basis(&self, max_degree : i32) {
        let mut next_degree = self.next_degree.lock().unwrap();

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
            for i in 0 .. basis.len() {
                map.insert(basis[i].clone(), i);
            }
            self.basis_element_to_index_map.push(map);
        }
        *next_degree = max_degree + 1;
    }

    fn get_dimension(&self, degree : i32, excess : i32) -> usize {
        self.basis_table[degree as usize].len()
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coef : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32) {
        self.multiply(result, coef, &self.basis_table[r_degree as usize][r_idx], &self.basis_table[s_degree as usize][s_idx]);
    }

    fn json_to_basis(&self, json : Value) -> (i32, usize) {
        let xi_degrees = combinatorics::get_xi_degrees(self.p);
        let tau_degrees = combinatorics::get_tau_degrees(self.p);

        let mut p_part = Vec::new();
        let mut q_part = 0;
        let mut degree = 0;

        if self.generic {
            let p_list = json[1].as_array().unwrap();
            let q_list = json[0].as_array().unwrap();
            let q = (2 * self.p - 2) as i32;

            for i in 0..p_list.len() {
                let val = p_list[i].as_u64().unwrap();
                p_part.push(val as u32);
                degree += (val as i32) * xi_degrees[i] * q;
            }

            for i in q_list {
                let k = i.as_u64().unwrap();
                q_part |= 1 << k;
                degree += tau_degrees[k as usize];
            }
        } else {
            let p_list = json.as_array().unwrap();
            for i in 0..p_list.len() {
                let val = p_list[i].as_u64().unwrap();
                p_part.push(val as u32);
                degree += (val as i32) * xi_degrees[i];
            }
        }
        let m = MilnorBasisElement { p_part, q_part, degree };
        (degree, self.basis_element_to_index(&m))
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
            return serde_json::to_value((q_list, &b.p_part)).unwrap();
        } else {
            return serde_json::to_value(&b.p_part).unwrap();
        }
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("{}", self.basis_table[degree as usize][idx])
    }

    /// We pick our generators to be Q_0 and all the P(...). This has room for improvement...
    fn get_generators(&self, degree : i32) -> Vec<usize> {
        if degree == 0 {
            return vec![];
        }
        if self.generic && degree == 1 {
            return vec![0]; // Q_0
        }
        let p = self.p;
        let q = if self.generic { 2 * p - 2 } else { 1 };
        let mut temp_degree = degree as u32;        
        if temp_degree % q != 0 {
            return vec![];
        }
        temp_degree /= q;
        while temp_degree % p == 0 {
            temp_degree /= p;
        }
        if temp_degree != 1 {
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
            return self.decompose_basis_element_ppart(degree, idx);
        } else {
            return self.decompose_basis_element_qpart(degree, idx);
        }
    }

    fn get_relations_to_check(&self, degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>{
        if self.generic && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }
        let p = self.get_prime();
        let q = if self.generic { 2*(p - 1) } else { 1 };
        let inadmissible_pairs = combinatorics::get_inadmissible_pairs(p, self.generic, degree);
        let mut result = Vec::new();
        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation
            let (first_degree, first_index) = self.get_beps_pn(0, x);
            let (second_degree, second_index) = self.get_beps_pn(b, y);
            relation.push((p - 1, (first_degree, first_index), (second_degree, second_index)));
            for e1 in 0 .. b + 1 {
                let e2 = b - e1;
                // e1 and e2 determine where a bockstein shows up.
                // e1 determines whether a bockstein shows up in front 
                // e2 determines whether a bockstein shows up in middle
                // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                for j in 0 .. x/p + 1 {
                    let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                    if c == 0 { continue; }
                    if j == 0 {
                        relation.push((c, self.get_beps_pn(e1, x + y), (e2 as i32, 0)));
                        continue;
                    }
                    let first_sq = self.get_beps_pn(e1, x + y - j);
                    let second_sq = self.get_beps_pn(e2, j);
                    relation.push((c, first_sq, second_sq));
                }
            }
            result.push(relation);
        }
        return result;
    }
}

// Compute basis functions
impl MilnorAlgebra {
    fn compute_ppart(&self, mut next_degree : i32, max_degree : i32) {
        if next_degree == 0 {
            self.ppart_table.push(vec![Vec::new()]);
            next_degree = 1;
        }

        let p = self.p as i32;
        let q = if p == 2 {1} else {2 * p - 2};
        let new_deg = max_degree/q;
        let old_deg = (next_degree-1)/q;

        self.ppart_table.reserve((new_deg - old_deg) as usize);

        let xi_degrees = combinatorics::get_xi_degrees(self.p);
        for d in (old_deg + 1) ..= new_deg {
            let mut new_row = Vec::new(); // Improve this
            for i in 0..xi_degrees.len() {
                if xi_degrees[i] > d {
                    break;
                }

                let rem = (d - xi_degrees[i]) as usize;
                for old in self.ppart_table[rem].iter() {
                    // ppart_table[rem] is arranged in increasing order of highest
                    // xi_i. If we get something too large, we may abort;
                    if old.len() > i + 1 {
                        break;
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
        let q = (2 * self.p - 2) as i32;

        if !self.generic {
            return;
        }

        let mut next_degree = next_degree;
        if next_degree == 0 {
            self.qpart_table[0].push( ZERO_QPART.clone());
            next_degree = 1;
        }

        let tau_degrees = crate::combinatorics::get_tau_degrees(self.p);
        let old_max_tau = tau_degrees.iter().position(|d| *d > next_degree - 1).unwrap(); // Use expect instead
        let new_max_tau = tau_degrees.iter().position(|d| *d > max_degree).unwrap();

        let bit_string_min : u32 = 1 << old_max_tau;
        let bit_string_max : u32 = 1 << new_max_tau;

        let mut residue : i32 = (old_max_tau as i32) % q;
        let mut total : i32 = 0;
        for i in 0..old_max_tau {
            total += tau_degrees[i];
        }

        for bit_string in bit_string_min..bit_string_max {
            let mut v = (bit_string ^ (bit_string - 1)) >> 1;
            let mut c : usize = 0;
            while v != 0 {
                v >>= 1;
                total -= tau_degrees [c];
                c += 1;
            }
            total += tau_degrees[c];
            residue += 1 - c as i32;
            residue = residue % q;
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
        let q = (2 * self.p - 2) as usize;

        for d in next_degree as usize..= max_degree as usize {
            let mut new_table = Vec::new(); // Initialize size

            for q_part in self.qpart_table[d % q].iter() {
                // Elements in qpart_table are listed in increasing order in
                // degree. Abort if degree too large.
                if q_part.degree > d as i32 {
                    break;
                }

                for p_part in self.ppart_table[(d - (q_part.degree as usize))/q].iter() {
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
    fn get_beps_pn(&self, e : u32, x : u32) -> (i32, usize) {
        let p = self.get_prime();
        let q = if self.generic { 2*(p - 1) } else { 1 };
        let degree = (q * x + e) as i32;
        let index = self.basis_element_to_index(&MilnorBasisElement {
            degree,
            q_part : e,
            p_part : vec![x]
        });
        return (degree, index);
    }

    fn multiply_qpart (&self, m1 : &MilnorBasisElement, f : u32) -> Vec<(u32, MilnorBasisElement)>{
        let tau_degrees = crate::combinatorics::get_tau_degrees(self.p);
        let xi_degrees = crate::combinatorics::get_xi_degrees(self.p);

        let mut new_result : Vec<(u32, MilnorBasisElement)> = vec![(1, m1.clone())];
        let mut old_result : Vec<(u32, MilnorBasisElement)> = Vec::new();

        let mut pk : u32 = 1;
        let mut k : u32 = 0;
        while f & !((1 << k) - 1) != 0 {
            if f & (1<<k) == 0 { // If only we had goto (or C-style for-loops)
                k+=1;
                pk *= self.p;
                continue;
            }

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
                    if i > 0 && term.p_part[i-1] < pk {
                        continue;
                    }

                    let mut new_p = term.p_part.clone();
                    if i > 0 {
                        new_p[i-1] -= pk;
                    }

                    // Now calculate the number of Q's we are moving past
                    let mut larger_q = 0;
                    let mut v = term.q_part >> (k + i as u32 + 1);
                    while v != 0 {
                        larger_q += v & 1;
                        v = v >> 1;
                    }

                    // If new_p ends with 0, drop them
                    loop {
                        match new_p.last() {
                            Some(0) => new_p.pop(),
                            _ => break,
                        };
                    }
                    // Now put everything together
                    let m = MilnorBasisElement {
                        p_part : new_p,
                        q_part : term.q_part | 1 << (k + i as u32),
                        degree : 0 // we don't really care about the degree here. The final degree of the whole calculation is known a priori
                    };
                    let c = if larger_q % 2 == 0 { *coef } else { *coef * (self.p - 1) };

                    new_result.push((c, m));
                }
            }

            k += 1;
            pk *= self.p;
        }
        new_result
    }

    fn multiply(&self, res : &mut FpVector, coef : u32, m1 : &MilnorBasisElement, m2 : &MilnorBasisElement) {
        let target_dim = m1.degree + m2.degree;

        if !self.generic {
            for (c, p) in PPartMultiplier::new(self.p, &(m1.p_part), &(m2.p_part)) {
                let idx = self.basis_element_to_index(&from_p(p, target_dim));
                res.add_basis_element(idx, c * coef);
            }
        } else {
            let m1f = self.multiply_qpart(m1, m2.q_part);
            for (cc, basis) in m1f {
                let prod = PPartMultiplier::new(self.p, &(basis.p_part), &(m2.p_part));
                for (c, p) in prod {
                    let new = MilnorBasisElement {
                        degree : target_dim,
                        q_part : basis.q_part,
                        p_part : p
                    };
                    let idx = self.basis_element_to_index(&new);
                    res.add_basis_element(idx, c * cc * coef);
                }
            }
        }
    }
}

#[allow(non_snake_case)]
struct PPartMultiplier<'a> {
    p : u32,
    M : Vec<Vec<u32>>,
    r : &'a PPart,
    s : &'a PPart,
    rows : usize,
    cols : usize,
    diag_num : usize,
    cont : bool
}

#[allow(non_snake_case)]
impl<'a>  PPartMultiplier<'a> {
    fn new (p : u32, r : &'a PPart, s : &'a PPart) -> PPartMultiplier<'a> {
        let rows = r.len() + 1;
        let cols = s.len() + 1;
        let diag_num = r.len() + s.len();

        let mut M = vec![vec![0; cols]; rows];

        for i in 1..rows {
            M[i][0] = r[i - 1];
        }
        for i in 1..cols {
            M[0][i] = s[i - 1];
        }
        PPartMultiplier { p, M, r, s, rows, cols, diag_num, cont : true }
    }

    fn update(&mut self) -> bool {
        for i in 1..self.rows {
            let mut total = self.M[i][0];
            let mut p_to_the_j = 1;
            for j in 1..self.cols {
                p_to_the_j *= self.p;
                if total < p_to_the_j {
                    // We don't have enough weight left in the entries above this one in the column to increment this cell.
                    // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                    total += self.M[i][j] * p_to_the_j;
                    continue;
                }
                // Check if any entry in column j above row i is nonzero. I'm still not sure why tbh.
                for k in 0..i {
                    if self.M[k][j] != 0 {
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
                }
                // All the cells above this one are zero so we didn't find our next matrix.
                // Add the weight from this cell to the total, we can use it to increment a cell lower down.
                total += self.M[i][j] * p_to_the_j;
            }
        }
        return false
    }
}

impl<'a> Iterator for PPartMultiplier<'a> {
    type Item = (u32, PPart);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.cont {
            return None;
        }

        let mut coef = 1;
        let mut new_p = Vec::new();
        let mut diagonal = Vec::with_capacity(std::cmp::max(self.rows, self.cols));

        for diag_idx in 1..=self.diag_num {
            let i_min = if diag_idx + 1 > self.cols { diag_idx + 1 - self.cols } else {0} ;
            let i_max = std::cmp::min(1 + diag_idx, self.rows);
            let mut sum = 0;

            diagonal.clear();

            for i in i_min..i_max {
                diagonal.push(self.M[i][diag_idx - i]);
                sum += self.M[i][diag_idx - i];
            }
            new_p.push(sum);

            if sum == 0  {
                continue;
            }
            coef *= crate::combinatorics::multinomial(self.p, &diagonal);
            coef = coef % self.p;
            if coef == 0 {
                self.cont = self.update();
                return self.next();
            }
        }
        // If new_p ends with 0, drop them
        loop {
            match new_p.last() {
                Some(0) => new_p.pop(),
                _ => break,
            };
        }

        self.cont = self.update();
        Some((coef, new_p))
    }
}

impl MilnorAlgebra {
    fn decompose_basis_element_qpart(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))>{
        let basis = &self.basis_table[degree as usize][idx];
        // Look for left-most non-zero qpart
        let i = basis.q_part.trailing_zeros();
        // If the basis element is just Q_{k+1}, we decompose Q_{k+1} = P(p^k) Q_k - Q_k P(p^k).
        if basis.q_part == 1 << i && basis.p_part.len() == 0 {
            let ppow = crate::combinatorics::integer_power(self.p, i - 1);

            let q_degree = (2 * ppow - 1) as i32;
            let p_degree = (ppow * (2 * self.p - 2)) as i32;

            let p_idx = self.basis_element_to_index(&from_p(vec![ppow], p_degree)).to_owned();

            let q_idx =  self.basis_element_to_index(
                &MilnorBasisElement {
                    q_part : 1 << (i-1),
                    p_part : Vec::new(),
                    degree : q_degree
                }).to_owned();

            return vec![(1, (p_degree, p_idx), (q_degree, q_idx)), (self.p - 1, (q_degree, q_idx), (p_degree, p_idx))];
        }

        // Otherwise, separate out the first Q_k.
        let first_degree = crate::combinatorics::get_tau_degrees(self.p)[i as usize];
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
    fn decompose_basis_element_ppart(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))>{
        let p = self.p;
        let q = if self.generic { 2*p - 2 } else { 1 };
        let b = &self.basis_table[degree as usize][idx];
        let first;
        let second;
        if b.p_part.len() > 1 {
            let mut t1 = 0;
            let mut pow = 1;
            for r in &b.p_part {
                t1 += r * pow;
                pow *= p;
            }
            first = self.get_beps_pn(0, t1);
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
                while temp_sq % p == 0 {
                    temp_sq /= p;
                    pow *= p;
                }
            }
            if sq == pow {
                return vec![(1, (degree, idx), (0, 0))];
            }
            first = self.get_beps_pn(0, pow);
            second = self.get_beps_pn(0, sq - pow);
        }
        let mut out_vec = FpVector::new(p, self.get_dimension(degree, -1), 0);
        self.multiply_basis_elements(&mut out_vec, 1, first.0, first.1, second.0, second.1, -1);
        let mut result = Vec::new();
        let c = out_vec.get_entry(idx);
        assert!(c != 0);
        out_vec.set_entry(idx, 0);
        let c_inv = crate::combinatorics::inverse(p, p - c);
        result.push((((p - 1) * c_inv) % p, first, second));
        for (i, v) in out_vec.iter().enumerate() {
            if v == 0 {
                continue;
            }
            for (c, t1, t2) in self.decompose_basis_element_ppart(degree, i){
                result.push(((c_inv * c * v) % p, t1, t2));
            }
        }
        return result;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest_parametrize;

    #[rstest_parametrize(p, max_degree,
        case(2, 32),
        case(3, 106)    
    )]
    fn test_milnor_basis(p : u32, max_degree : i32){
        let algebra = MilnorAlgebra::new(p);//p != 2
        algebra.compute_basis(max_degree);
        for i in 1 .. max_degree {
            let dim = algebra.get_dimension(i, -1);
            for j in 0 .. dim {
                let b = algebra.basis_element_from_index(i, j);
                assert_eq!(algebra.basis_element_to_index(&b), j);
                let json = algebra.json_from_basis(i, j);
                let new_b = algebra.json_to_basis(json);
                assert_eq!(new_b, (i, j));
            }
        }
    }

    #[rstest_parametrize(p, max_degree,
        case(2, 32),
        case(3, 106)    
    )]
    fn test_milnor_decompose(p : u32, max_degree : i32){
        let algebra = MilnorAlgebra::new(p);
        algebra.compute_basis(max_degree);
        for i in 1 .. max_degree {
            let dim = algebra.get_dimension(i, -1);
            let gens = algebra.get_generators(i);
            // println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim, 0);
            for j in 0 .. dim {
                if gens.contains(&j){
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in algebra.decompose_basis_element(i, j) {
                    // print!("{} * {} * {}  +  ", coeff, algebra.basis_element_to_string(first_degree,first_idx), algebra.basis_element_to_string(second_degree, second_idx));
                    algebra.multiply_basis_elements(&mut out_vec, coeff, first_degree, first_idx, second_degree, second_idx, -1);
                }
                assert!(out_vec.get_entry(j) == 1, 
                    format!("{} != {}", algebra.basis_element_to_string(i, j), algebra.element_to_string(i, &out_vec)));
                out_vec.set_entry(j, 0);
                assert!(out_vec.is_zero(), 
                    format!("\n{} != {}", 
                        algebra.basis_element_to_string(i, j), algebra.element_to_string(i, &out_vec)));
            }
        }
    }

    use crate::module::ModuleFailedRelationError;
    #[rstest_parametrize(p, max_degree,
        case(2, 32),
        case(3, 106)    
    )]
    fn test_adem_relations(p : u32, max_degree : i32){
        let algebra = MilnorAlgebra::new(p); // , p != 2
        algebra.compute_basis(max_degree + 2);
        let mut output_vec = FpVector::new(p, 0, 0);
        for i in 1 .. max_degree {
            output_vec.clear_slice();
            let output_dim = algebra.get_dimension(i, -1);
            if output_dim > output_vec.get_dimension() {
                output_vec = FpVector::new(p, output_dim, 0);
            }
            output_vec.set_slice(0, output_dim);
            let relations = algebra.get_relations_to_check(i);
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
                    assert!(false,
                        format!("{}", ModuleFailedRelationError {relation : relation_string, value : value_string})
                    );
                }
            }
        }
    }    
}
