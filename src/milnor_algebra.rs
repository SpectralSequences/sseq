use std::sync::Mutex;

use crate::fp_vector::FpVector;
use crate::once::OnceVec;
use crate::algebra::Algebra;
use itertools::Itertools;
use std::collections::HashMap;
use serde_json::value::Value;

pub struct MilnorProfile {
    pub generic : bool
}

#[derive(Default, Clone)]
struct QPart {
    degree : i32,
    q_part : u32
}

type PPart = Vec<u32>;

#[derive(Debug, Clone)]
struct MilnorBasisElement {
    q_part : u32,
    p_part : PPart,
    degree : i32
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
            write!(f, "0")?;
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
            write!(f, "{}", self.p_part.iter().join(", "))?;
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
    max_degree : Mutex<i32>,
    p : u32,
    ppart_table : OnceVec<Vec<PPart>>,
    qpart_table : Vec<OnceVec<QPart>>,
    basis_table : OnceVec<Vec<MilnorBasisElement>>,
    basis_element_to_index_map : OnceVec<HashMap<MilnorBasisElement, usize>>, // degree -> MilnorBasisElement -> index
}

impl MilnorAlgebra {
    pub fn new(p : u32) -> Self {
        crate::combinatorics::initialize_prime(p);
        crate::combinatorics::initialize_xi_tau_degrees(p);
        crate::fp_vector::initialize_limb_bit_index_table(p);

        let profile = MilnorProfile {
            generic : p != 2
        };

        let mut qpart_table = Vec::new();
        qpart_table.resize_with((2 * p - 2) as usize, OnceVec::new);

        Self {
            p,
            profile: profile,
            name : format!("MilnorAlgebra(p={})", p),
            max_degree : Mutex::new(-1),
            ppart_table : OnceVec::new(),
            qpart_table,
            basis_table : OnceVec::new(),
            basis_element_to_index_map : OnceVec::new()
        }
    }
}

impl Algebra for MilnorAlgebra {
    fn get_prime(&self) -> u32 {
        self.p
    }

    fn get_max_degree(&self) -> i32 {
        self.basis_table.len() as i32
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_filtration_one_products(&self) -> Vec<(&str, i32, usize)> {Vec::new()} // Implement this

    fn compute_basis(&self, degree : i32) {
        let mut old_max_degree = self.max_degree.lock().unwrap();

        self.compute_ppart(degree, *old_max_degree);
        self.compute_qpart(degree, *old_max_degree);

        self.basis_table.reserve((degree - *old_max_degree) as usize);
        self.basis_element_to_index_map.reserve((degree - *old_max_degree) as usize);

        if self.profile.generic {
            self.generate_basis_generic(degree, *old_max_degree);
        } else {
            self.generate_basis_2(degree, *old_max_degree);
        }

        // Populate hash map
        for d in (*old_max_degree + 1) as usize..(degree + 1) as usize {
            let basis = &self.basis_table[d];
            let mut map = HashMap::with_capacity(basis.len());
            for i in 0 .. basis.len() {
                map.insert(basis[i].clone(), i);
            }
            self.basis_element_to_index_map.push(map);
        }
        *old_max_degree = degree;
    }

    fn get_dimension(&self, degree : i32, excess : i32) -> usize {
        self.basis_table[degree as usize].len()
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coef : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32) {
        self.multiply(result, coef, &self.basis_table[r_degree as usize][r_idx], &self.basis_table[s_degree as usize][s_idx]);
    }

    fn json_to_basis(&self, json : Value) -> (i32, usize) {
        let xi_degrees = crate::combinatorics::get_xi_degrees(self.p);
        let tau_degrees = crate::combinatorics::get_tau_degrees(self.p);

        let mut p_part = Vec::new();
        let mut q_part = 0;
        let mut degree = 0;

        println!("{:?}", json);
        if self.profile.generic {
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
                println!("{:?}", p_part);
            }
        }
        let m = MilnorBasisElement { p_part, q_part, degree };
        (degree, *self.basis_element_to_index_map[degree as usize].get(&m).unwrap())
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("{}", self.basis_table[degree as usize][idx])
    }
}

// Compute basis functions
impl MilnorAlgebra {
    fn compute_ppart(&self, degree : i32, old_max_degree : i32) {
        let mut old_max_degree = old_max_degree;
        if old_max_degree == -1 {
            self.ppart_table.push(vec![Vec::new()]);
            old_max_degree = 0;
        }

        let p = self.p as i32;
        let q = if p == 2 {1} else {2 * p - 2};
        let new_deg = degree/q;
        let old_deg = old_max_degree/q;

        self.ppart_table.reserve((new_deg - old_deg) as usize);

        let xi_degrees = crate::combinatorics::get_xi_degrees(self.p);

        for d in (old_deg + 1)..(new_deg + 1) {
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

    fn compute_qpart(&self, new_max_degree : i32, old_max_degree : i32) {
        let q = (2 * self.p - 2) as i32;

        if !self.profile.generic {
            return;
        }

        let mut old_max_degree = old_max_degree;
        if old_max_degree == -1 {
            self.qpart_table[0].push( ZERO_QPART.clone());
            old_max_degree = 0;
        }

        let tau_degrees = crate::combinatorics::get_tau_degrees(self.p);
        let old_max_tau = tau_degrees.iter().position(|d| *d > old_max_degree).unwrap(); // Use expect instead
        let new_max_tau = tau_degrees.iter().position(|d| *d > new_max_degree).unwrap();

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

    fn generate_basis_generic(&self, degree : i32, old_max_degree : i32) {
        let q = (2 * self.p - 2) as usize;

        for d in (old_max_degree + 1) as usize..(degree + 1) as usize {
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

    fn generate_basis_2(&self, degree:i32, old_max_degree : i32) {
        for i in ((old_max_degree + 1) as usize)..((degree + 1) as usize) {
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

        if !self.profile.generic {
            for (c, p) in PPartMultiplier::new(self.p, &(m1.p_part), &(m2.p_part)) {
                let idx = self.basis_element_to_index_map[target_dim as usize].get(&from_p(p, target_dim)).unwrap();
                res.add_basis_element(*idx, c * coef);
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
                    let idx = self.basis_element_to_index_map[target_dim as usize].get(&new).unwrap();
                    res.add_basis_element(*idx, c * cc * coef);
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

