use core::cmp::Ordering;
use lazy_static;
use std::collections::HashMap;
use std::format;
use std::sync::Mutex;

use crate::once::OnceVec;
use crate::combinatorics;
use crate::combinatorics::MAX_XI_TAU;
use crate::algebra::Algebra;
// use crate::memory::CVec;
use crate::fp_vector::{FpVector, FpVectorT};
use serde_json::value::Value;


lazy_static!{
    static ref BOCKSTEIN_TABLE : Vec<Vec<u32>> = {
        let mut n_choose_k = 1;
        let mut table : Vec<Vec<u32>> = Vec::with_capacity(MAX_XI_TAU + 1);
        for k in 1 .. MAX_XI_TAU + 2 {
            table.push(Vec::with_capacity(n_choose_k));
            n_choose_k *= MAX_XI_TAU + 1 - k;
            n_choose_k /= k; 
        }

        for i in 0u32 .. (1<<MAX_XI_TAU) {
            let bits_set = i.count_ones() as usize;
            table[bits_set].push(i);
        }
        table
    };
}

/// The format of the AdemBasisElement is as follows. To encode
/// $$\beta^{\varepsilon_0} P^{i_0} \beta^{\varepsilon_1} P^{i_1} \cdots \beta^{\varepsilon_n}
/// P^{i_n} \beta^{\varepsilon_{n+1}},$$
/// we set
/// $$ \begin{aligned}
/// \mathtt{ps} &= [i_0, i_1, \ldots, i_n]\\\\
/// \mathtt{bocksteins} &= 000\cdots0\varepsilon_{n+1} \varepsilon_n \cdots \varepsilon_0
/// \end{aligned} $$
// #[derive(RustcDecodable, RustcEncodable)]
#[derive(Debug, Clone)]
pub struct AdemBasisElement {
    pub degree : i32,
    pub excess : i32,
    pub bocksteins : u32,
    pub ps : Vec<u32>
}

impl std::cmp::PartialEq for AdemBasisElement {
    fn eq(&self, other : &Self) -> bool {
        self.ps == other.ps && self.bocksteins == other.bocksteins
    }
}

impl std::cmp::Eq for AdemBasisElement {}

impl std::hash::Hash for AdemBasisElement {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bocksteins.hash(state);
        self.ps.hash(state);
    }
}

impl std::fmt::Display for AdemBasisElement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut first = true;
        for (i, n) in self.ps.iter().enumerate() {
            if !first {
                write!(f, " ")?;
            }
            if (self.bocksteins >> i) & 1 == 1 {
                write!(f, "b ")?;
            }
            write!(f, "{}{}", "P", n)?;
            first = false;
        }
        if self.ps.len() == 0 {
            if self.bocksteins & 1 == 1 {
                write!(f, "b")?;
            } else {
                write!(f, "1")?;
            }
            return Ok(())
        }
        if (self.bocksteins >> self.ps.len()) & 1 == 1{
            write!(f, " b")?;
        }
        Ok(())
    }
}

fn adem_basis_element_excess_sort_order(a : &AdemBasisElement, b : &AdemBasisElement) -> Ordering{
    match(a.excess, b.excess){
        (x,y) if x > y => Ordering::Greater,
        (x,y) if x == y => Ordering::Equal,
        (x,y) if x < y => Ordering::Less,
        _ => {assert!(false); Ordering::Equal}
    }
}

// We need this for generic basis generation.
fn adem_basis_element_length_sort_order(a : &AdemBasisElement, b : &AdemBasisElement) -> Ordering {
    match(a.ps.len(), b.ps.len()){
        (x,y) if x > y => Ordering::Greater,
        (x,y) if x == y => Ordering::Equal,
        (x,y) if x < y => Ordering::Less,
        _ => {assert!(false); Ordering::Equal}
    }
}

unsafe fn shift_vec<T>(v : Vec<T> , offset : isize) -> Vec<T> {
    let ptr = v.as_ptr();
    let len = v.len();
    let cap = v.capacity();
    std::mem::forget(v);        
    Vec::from_raw_parts((ptr as *mut T).offset(offset), (len as isize - offset) as usize, (cap as isize - offset) as usize)
}

pub struct AdemAlgebra {
    p : u32,
    name : String,
    pub generic : bool,
    // FiltrationOneProduct_list product_list; // This determines which indecomposibles have lines drawn for them.
    unstable : bool,
    next_degree : Mutex<i32>,
    even_basis_table : OnceVec<Vec<AdemBasisElement>>,
    basis_table : OnceVec<Vec<AdemBasisElement>>, // degree -> index -> AdemBasisElement
    basis_element_to_index_map : OnceVec<HashMap<AdemBasisElement, usize>>, // degree -> AdemBasisElement -> index
    multiplication_table : OnceVec<Vec<Vec<FpVector>>>,// degree -> first square -> admissibile sequence idx -> result vector
    excess_table : OnceVec<Vec<u32>>,
    sort_order : Option<fn(&AdemBasisElement, &AdemBasisElement) -> Ordering>,
    filtration_one_products : Vec<(String, i32, usize)> //Vec<Once<(i32, usize)>>
}

impl Algebra for AdemAlgebra {
    fn get_algebra_type(&self) -> &str {
        "adem"
    }

    fn prime(&self) -> u32 {
        self.p
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_filtration_one_products(&self) -> &Vec<(String, i32, usize)>{
        &self.filtration_one_products
    }

    fn set_default_filtration_one_products(&mut self) {
        let mut products = Vec::with_capacity(4);
        let max_degree;
        if self.generic {
            products.push(("a_0".to_string(), AdemBasisElement {
                degree : 1,
                bocksteins : 1,
                excess : 0,
                ps : vec![]
            }));
            products.push(("h_0".to_string(), AdemBasisElement {
                degree : (2*self.p-2) as i32,
                bocksteins : 0,
                excess : 0,
                ps : vec![1]
            }));
            max_degree = (2 * self.p - 2) as i32;
        } else {
            for i in 0..4 {
                let degree = 1 << i; // degree is 2^hi
                let ps = vec![degree as u32];
                products.push((format!("h_{}", i), AdemBasisElement {
                    degree,
                    bocksteins : 0,
                    excess : 0,
                    ps
                }));
            }
            max_degree = 1 << 3;
        }

        self.compute_basis(max_degree);
        self.filtration_one_products = products.into_iter()
            .map(|(name, b)| (name, b.degree, self.basis_element_to_index(&b)))
            .collect();
    }

    fn compute_basis(&self, max_degree : i32) {
        let mut next_degree = self.next_degree.lock().unwrap();
        if max_degree < *next_degree {
            return;
        }

        if self.generic {
            self.generate_basis_generic(*next_degree, max_degree);
            self.generate_basis_element_to_index_map(*next_degree, max_degree);
            self.generate_multiplication_table_generic(*next_degree, max_degree);
        } else {
            self.generate_basis2(*next_degree, max_degree);
            self.generate_basis_element_to_index_map(*next_degree, max_degree);
            self.generate_multiplication_table_2(*next_degree, max_degree);
        }

        *next_degree = max_degree + 1;
    }

    fn get_dimension(&self, degree : i32, excess : i32) -> usize {
        if degree < 0 {
            return 0;
        }
        return self.basis_table[degree as usize].len();
    }

    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, 
        r_degree : i32, r_index : usize, 
        s_degree : i32, s_index : usize, excess : i32)
    {
        self.multiply(result, coeff, r_degree, r_index, s_degree, s_index, excess);
    }

    fn json_to_basis(&self, json : Value) -> (i32, usize) {
        let op : Vec<u32> = serde_json::from_value(json).unwrap();
        let mut sqs = Vec::with_capacity(op.len());
        let p = self.p;
        let q;
        let mut degree = 0;
        let mut bocksteins = 0;
        if self.generic {
            q = 2*p-2;
            for (i, sq) in op.iter().enumerate() {
                if i % 2 == 0 {
                    degree += sq;
                    bocksteins |= sq << i/2;
                } else {
                    degree += q * sq;
                    sqs.push(*sq);
                }
            }
        } else {
            q = 1;
            for sq in op {
                degree += q * sq;
                sqs.push(sq);
            }
        }
        let b = AdemBasisElement {
            degree : degree as i32,
            excess : 0,
            bocksteins,
            ps : sqs
        };
        (degree as i32, *self.basis_element_to_index_map[degree as usize].get(&b).unwrap())
    }


    fn json_from_basis(&self, degree : i32, index : usize) -> Value {
        let b = self.basis_element_from_index(degree, index);
        let mut out_sqs = Vec::with_capacity(2*b.ps.len() + 1);
        let mut bocksteins = b.bocksteins;
        if self.generic {
            out_sqs.push(bocksteins & 1);
            bocksteins >>= 1;
            for sq in b.ps.iter() {
                out_sqs.push(*sq);
                out_sqs.push(bocksteins & 1);
                bocksteins >>= 1;                
            }
        } else {
            for sq in b.ps.iter() {
                out_sqs.push(*sq);
            }
        }
        let result = serde_json::to_value(out_sqs).unwrap();
        result
    }


    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("{}", self.basis_element_from_index(degree, idx))
    }

    fn get_generators(&self, degree : i32) -> Vec<usize> {
        let p = self.prime();
        if degree == 0 {
            return vec![];
        }
        if self.generic {
            if degree == 1 {
                return vec![0];
            }
            // Test if degree is q*p^k.
            let mut temp_degree = degree as u32;
            if temp_degree % (2*(p-1)) != 0 {
                return vec![];
            }
            temp_degree /= 2*(p - 1);
            while temp_degree % p == 0 {
                temp_degree /= p;
            }
            if temp_degree != 1 {
                return vec![];
            }
            let idx = self.basis_element_to_index(&AdemBasisElement {
                degree,
                excess : 0,
                bocksteins : 0,
                ps : vec![degree as u32/(2*p-2)]
            });
            return vec![idx];
        } else {
            // I guess we're assuming here that not generic ==> p == 2. There's probably tons of places we assume that though.
            if degree.count_ones() != 1 {
                return vec![];
            }
            let idx = self.basis_element_to_index(&AdemBasisElement {
                degree,
                excess : 0,
                bocksteins : 0,
                ps : vec![degree as u32]
            });
            return vec![idx];
        }
    }

    fn decompose_basis_element(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        if self.generic {
            self.decompose_basis_element_generic(degree, idx)
        } else {
            self.decompose_basis_element_2(degree, idx)
        }
    }

    /// We return Adem relations $b^2 = 0$, $P^i P^j = \cdots$ for $i < pj$, and $P^i b P^j = \cdots$ for $i < pj + 1$. It suffices to check these because
    /// they generate all relations.
    fn get_relations_to_check(&self, degree : i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>{
        if self.generic && degree == 2 {
            // beta^2 = 0 is an edge case
            return vec![vec![(1, (1, 0), (1, 0))]];
        }

        let p = self.prime();

        let inadmissible_pairs = combinatorics::get_inadmissible_pairs(p, self.generic, degree);
        let mut result = Vec::new();

        for (x, b, y) in inadmissible_pairs {
            let mut relation = Vec::new();
            // Adem relation
            let first_sq = self.get_beps_pn(0, x);
            let second_sq = self.get_beps_pn(b, y);
            relation.push((p - 1, first_sq, second_sq));
            for e1 in 0 .. b + 1 {
                let e2 = b - e1;
                // e1 and e2 determine where a bockstein shows up.
                // e1 determines if a bockstein shows up in front 
                // e2 determines if a bockstein shows up in middle
                // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
                for j in 0 ..= x/p {
                    let c = combinatorics::adem_relation_coefficient(p, x, y, j, e1, e2);
                    if c == 0 { continue; }
                    let idx = self.basis_element_to_index(&AdemBasisElement{
                        degree,
                        excess : 0,
                        ps : if j == 0 { vec![(x+y) as u32] } else { vec![(x + y - j) as u32, j as u32] },
                        bocksteins : e1 as u32 | ((e2 as u32) << 1)
                    });
                    relation.push((c as u32, (degree, idx), (0, 0)));
                }
            }
            result.push(relation);
        }
        return result;
    }
}

// static void AdemAlgebra__initializeFields(AdemAlgebraInternal *algebra, uint p, bool generic, bool unstable);
// uint AdemAlgebra__generateName(AdemAlgebra *algebra); // defined in adem_io
impl AdemAlgebra {
    pub fn new(p : u32, generic : bool, unstable : bool) -> Self {
        crate::fp_vector::initialize_limb_bit_index_table(p);
        let even_basis_table = OnceVec::new();
        let basis_table = OnceVec::new();
        let basis_element_to_index_map = OnceVec::new();
        let multiplication_table = OnceVec::new();
        let excess_table = OnceVec::new();
        Self {
            p,
            name : format!("AdemAlgebra(p={})", p),
            generic,
            next_degree : Mutex::new(0),
            unstable,
            even_basis_table,
            basis_table,
            basis_element_to_index_map,
            multiplication_table,
            excess_table,
            sort_order : None,
            filtration_one_products : Vec::new()
        }
    }

    fn generate_basis_even(&self, mut next_degree : i32, max_degree : i32){
        if next_degree == 0 {
            let mut table = Vec::with_capacity(1);
            table.push(
                AdemBasisElement {
                    degree : 0,
                    excess : 0,
                    bocksteins : 0,
                    ps : vec![]
                }
            );
            self.even_basis_table.push(table);
            next_degree += 1;
        }

        for n in next_degree ..= max_degree {
            self.generate_basis_even_degreen(n);
        }
    }

    fn generate_basis_even_degreen(&self, n : i32){
        let p = self.p as i32;
        let mut basis = Vec::new();
        // Put Sqn into the list.
        basis.push(
            AdemBasisElement {
                degree : n,
                excess : n,
                bocksteins : if self.generic { u32::max_value() << 2 } else { 0 },
                ps : vec![n as u32]
            }
        );

        // last = last term. We append (last,) to the end of
        // elements of degree n - last whose own last square is
        // at least p * last.
        // In order for this to be possible, this means that p last <= n - last, 
        // or (p+1) * last <= n or last <= n/(p+1). We order the squares in decreasing
        // order of their last element so that as we walk over the previous basis
        // when we find a square whose end is too small, we can break.
        for last in (1 .. n/(p+1) + 1).rev() {
            let previous_basis = &self.even_basis_table[(n-last) as usize];
            for prev_elt in previous_basis {
                let prev_elt_p_len = prev_elt.ps.len();
                let old_last_sq = prev_elt.ps[prev_elt_p_len - 1] as i32;
                if old_last_sq < p * last {
                    break;
                }
                // Write new basis element to basis element buffer
                
                let degree = prev_elt.degree + last;
                let excess = prev_elt.excess - (p-1)*last;
                // We're using bocksteins as a bit mask:
                // A bit in bocksteins shall be set if it's illegal for a bockstein to occur there.
                let mut bocksteins = prev_elt.bocksteins; 
                if self.generic{
                    bocksteins |= if old_last_sq == p*last { 1 << prev_elt_p_len } else { 0 };
                    bocksteins &= !(1 << (prev_elt_p_len +1));
                }
                let mut ps : Vec<u32> = Vec::with_capacity(prev_elt_p_len + 1);
                for k in &prev_elt.ps {
                    ps.push(*k);
                }
                ps.push(last as u32);
                basis.push(AdemBasisElement {
                    degree,
                    excess,
                    bocksteins,
                    ps
                });
            }
        }
        self.even_basis_table.push(basis);
    }


    fn generate_basis2(&self, next_degree : i32, max_degree : i32){
        self.generate_basis_even(next_degree, max_degree);
        for n in next_degree ..= max_degree {
            let table = &self.even_basis_table[n as usize];
            self.basis_table.push(table.clone());
        }
    }


    // Our approach is to pick the bocksteins and the P's separately and merge.
    fn generate_basis_generic(&self, next_degree : i32, max_degree : i32){
        self.generate_basis_even(next_degree, max_degree);
        for n in next_degree ..= max_degree {
            self.generate_basis_generic_degreen(n);
        }
    }

    // Now handle the bocksteins.
    // We have our Ps in even_basis_table and they contain in their bockstein field
    // a bit flag that indicates where bocksteins are allowed to go.
    #[allow(non_snake_case)]
    fn generate_basis_generic_degreen(&self, n : i32){
        let p = self.p as i32;
        let q = 2*(p-1);        
        let residue = n % q;
        let mut basis : Vec<AdemBasisElement> = Vec::new();
        // First we need to know how many bocksteins we'll use so we know how much degree
        // to assign to the Ps. The Ps all have degree divisible by q=2p-2, so num_bs needs to
        // be congruent to degree mod q.
        let num_bs_bound = std::cmp::min(MAX_XI_TAU, (n + 1) as usize);
        for num_bs in (residue as usize .. num_bs_bound).step_by(q as usize) {
            let P_deg = (n as usize - num_bs)/ q as usize;
            // AdemBasisElement_list P_list 
            let even_basis = &self.even_basis_table[P_deg];
            for i in (0 .. even_basis.len()).rev() {
                let P = &even_basis[i];
                // We pick our P first.
                if P.ps.len() + 1 < num_bs { // Not enough space to fit the bs.
                    continue; // Ps ordered in descending length, so none of the later ones will have space either
                }
                let bflags = &BOCKSTEIN_TABLE[num_bs];
                for bocksteins in bflags {
                    let bocksteins = *bocksteins;
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
                    let mut excess = 2*P.excess; // Ps contribute 2 to excess
                    excess += (bocksteins & 1) as i32; // leading bockstein increases excess by 1
                    let nonleading_bocksteins = bocksteins & ((1<<P.ps.len()) - 1) & !1;
                    excess -= nonleading_bocksteins.count_ones() as i32; // remaining bocksteins reduce excess by 1
                    let ps = P.ps.to_vec();
                    basis.push(AdemBasisElement {
                        degree,
                        excess,
                        bocksteins,
                        ps
                    })
                }
            }
        }
        self.basis_table.push(basis);
        // if let Some(f) = self.sort_order {
            // qsort(basisElementBuffer, cur_basis_len, sizeof(AdemBasisElement), algebra->public_algebra.sort_order);
        // }
    }

    fn generate_basis_element_to_index_map(&self, next_degree : i32, max_degree : i32){
        for n in next_degree ..= max_degree {
            let basis = &self.basis_table[n as usize];
            let mut map = HashMap::with_capacity(basis.len());
            for i in 0 .. basis.len() {
                map.insert(basis[i].clone(), i);
            }
            self.basis_element_to_index_map.push(map);
        }
    }

    pub fn basis_element_from_index(&self, degree : i32, idx : usize) -> &AdemBasisElement {
        &self.basis_table[degree as usize][idx]
    }

    pub fn basis_element_to_index(&self, elt : &AdemBasisElement) -> usize {
        if let Some(idx) = self.basis_element_to_index_map[elt.degree as usize].get(elt) {
            *idx
        } else {
            println!("Didn't find element: {:?}", elt);
            assert!(false);
            0
        }
    }

    fn tail_of_basis_element_to_index(&self, mut elt : AdemBasisElement, idx : u32, q : u32) -> (AdemBasisElement, usize) {
        let degree = elt.degree;
        let bocksteins = elt.bocksteins;
        for i in 0..idx as usize {
            elt.degree -= (q * elt.ps[i] + (elt.bocksteins & 1)) as i32;
            elt.bocksteins >>= 1;            
        }
        unsafe { elt.ps = shift_vec(elt.ps, idx as isize); }
        let result  = self.basis_element_to_index(&elt);
        unsafe { elt.ps = shift_vec(elt.ps, -(idx as isize)); }
        elt.degree = degree;
        elt.bocksteins = bocksteins;
        return (elt, result);
    }

    fn generate_multiplication_table_2(&self, mut next_degree : i32, max_degree : i32){
        // degree -> first_square -> admissibile sequence idx -> result vector
        if next_degree == 0 {
            self.multiplication_table.push(Vec::new());
            next_degree += 1;
        }

        for n in next_degree ..= max_degree {
            let mut table : Vec<Vec<FpVector>> = Vec::with_capacity((n + 1) as usize);
            table.push(Vec::with_capacity(0));
            for x in 1 ..= n {
                let dimension = self.get_dimension(n - x, -1);
                table.push(Vec::with_capacity(dimension));
            }
            for x in (1 ..= n).rev() {
                for idx in 0 .. self.get_dimension(n - x, -1) {
                    let res = self.generate_multiplication_table_2_step(&table, n, x, idx);
                    table[x as usize].push(res);
                }
            }
            self.multiplication_table.push(table);
        }
    }

    fn generate_multiplication_table_2_step(&self, table : &Vec<Vec<FpVector>>, n : i32, x : i32, idx : usize) -> FpVector {
        let output_dimension = self.get_dimension(n, -1);
        let mut result = FpVector::new(self.p, output_dimension);
        let cur_basis_elt = self.basis_element_from_index(n-x, idx);
        let x = x as u32;        
        let mut working_elt = cur_basis_elt.clone();

        // Be careful to deal with the case that cur_basis_elt has length 0
        // If the length is 0 or the sequence is already admissible, we can just write a 1 in the answer
        // and continue.
        if cur_basis_elt.ps.len() == 0 || x >= 2*cur_basis_elt.ps[0] {
            working_elt.ps.insert(0, x);
            working_elt.degree = n;
            let out_idx = self.basis_element_to_index(&working_elt);
            result.add_basis_element(out_idx, 1);
            return result;
        }

        // We now want to decompose Sq^x Sq^y = \sum_j *coef* Sq^{x + y - j} Sq^j.
        let y = working_elt.ps[0];

        let tuple = self.tail_of_basis_element_to_index(working_elt, 1, 1);
        working_elt = tuple.0;
        let tail_idx = tuple.1;

        for j in 0 ..= x/2 {
            if combinatorics::adem_relation_coefficient(2, x, y, j, 0, 0) == 0 {
                continue;
            }
            if j==0 {
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
            let rest_reduced = &self.multiplication_table[(n as u32 - (x + y) + j) as usize][j as usize][tail_idx];
            for (i, coeff) in rest_reduced.iter().enumerate() {
                if coeff == 0 {
                    continue;
                }
                // Reduce Sq^{x+y-j} * whatever square using the table in the same degree, larger index
                // Since we're doing the first squares in decreasing order and x + y - j > x, 
                // we already calculated this.
                let source = &table[(x + y - j) as usize][i];
                result.add(source, 1);
            }
        }
        result
    }

    fn generate_multiplication_table_generic(&self, mut next_degree : i32, max_degree : i32){
        // degree -> first_square -> admissibile sequence idx -> result vector
        if next_degree == 0 {
            self.multiplication_table.push(Vec::new());
            next_degree += 1;
        }
        let q = 2 * self.p as i32 - 2;
        for n in next_degree ..= max_degree {
            let mut table : Vec<Vec<FpVector>> = Vec::with_capacity(2*(n/q + 1) as usize);
            for i in 0 ..= n/q {
                for b in 0 ..= 1 {
                    // This corresponds to x = 2i + b
                    let dimension = self.get_dimension(n - q * i - b, -1);
                    table.push(Vec::with_capacity(dimension));
                }
            }
            for i in (0 ..= n/q).rev() {
                for idx in 0 .. self.get_dimension(n - q * i - 1, -1) {
                    let res = self.generate_multiplication_table_generic_step(&table, n, 2 * i + 1, idx);
                    table[1 + 2 * i as usize].push(res);
                }
                if i != 0 {
                    for idx in 0 .. self.get_dimension(n - q * i, -1) {
                        let res = self.generate_multiplication_table_generic_step(&table, n, 2 * i, idx);
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
    fn generate_multiplication_table_generic_step(&self, table : &Vec<Vec<FpVector>>,  n : i32, x : i32, idx : usize) -> FpVector {
        let p : i32 = self.p as i32; // we use p for the i32 version and self.p for the u32 version
        let q : i32 = 2*p - 2;

        let x : u32 = x as u32;

        let output_dimension = self.get_dimension(n, -1);
        let mut result = FpVector::new(self.p, output_dimension);

        // If x is just \beta, this is super easy.
        if x == 1 {
            let mut elt = self.basis_element_from_index(n-1, idx).clone();
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
            let rest_reduced = &self.multiplication_table[n as usize - 1][x as usize -1][idx];
            for (id, coef) in rest_reduced.iter().enumerate() {
                let mut elt = self.basis_element_from_index(n-1, id).clone();
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
        let i : u32 = x / 2;
        let mut working_elt = self.basis_element_from_index(n - (q * i as i32), idx).clone();

        let b : u32 = working_elt.bocksteins & 1;
        if working_elt.ps.len() == 0 || i >= self.p*working_elt.ps[0] + b {
            working_elt.ps.insert(0, i);
            working_elt.bocksteins <<= 1;
            working_elt.degree = n;

            let out_idx = self.basis_element_to_index(&working_elt);
            result.add_basis_element(out_idx, 1);
            return result;
        }

        // In other cases, use the Adem relations.
        let j : u32 = working_elt.ps[0];

        let tuple = self.tail_of_basis_element_to_index(working_elt, 1, q as u32);
        working_elt = tuple.0;
        let tail_idx = tuple.1;

        if b == 0 {
            // We use P^i P^j = \sum ... P^{i + j - k} P^k
            for k in 0 ..= i/self.p {
                let c = combinatorics::adem_relation_coefficient(self.p, i, j, k, 0, 0);
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

                let rest_reduced = &self.multiplication_table[(n - q * (i + j - k) as i32) as usize][2 * k as usize][tail_idx];
                for (id, coeff) in rest_reduced.iter().enumerate() {
                    let source = &table[2 * (i + j - k) as usize][id];
                    result.add(source, (c * coeff) % self.p);
                }
            }
        } else {
            // First treat the k = 0 case.
            // \beta P^{i + j - k} P^i
            let c = combinatorics::adem_relation_coefficient(self.p, i, j, 0, 1, 0);
            working_elt.ps[0] = i + j;
            working_elt.degree = n;
            let index = self.basis_element_to_index(&working_elt);
            result.add_basis_element(index, c);

            // P^{i + j - k} \beta P^k. Check if there is \beta following P^k
            if working_elt.bocksteins & 2 == 0 {
                let c = combinatorics::adem_relation_coefficient(self.p, i, j, 0, 0, 1);
                working_elt.bocksteins ^= 3; // flip the first two bits, so that it now ends with 10
                let index = self.basis_element_to_index(&working_elt);
                result.add_basis_element(index, c);
            }

            for k in 1 ..= i/self.p {
                // \beta P^{i + j - k} P^k
                let c = combinatorics::adem_relation_coefficient(self.p, i, j, k, 1, 0);
                if c != 0 {
                    let rest_reduced = &self.multiplication_table[(n - q * (i + j - k) as i32 - 1) as usize][2 * k as usize][tail_idx];
                    for (id, coeff) in rest_reduced.iter().enumerate() {
                        let source = &table[1 + 2 * (i + j - k) as usize][id];
                        result.add(source, (c * coeff) % self.p);
                    }
                }

                // P^{i + j - k} \beta P^k
                let c = combinatorics::adem_relation_coefficient(self.p, i, j, k, 0, 1);
                if c != 0 {
                    let rest_reduced = &self.multiplication_table[(n - q * (i + j - k) as i32) as usize][1 + 2 * k as usize][tail_idx];
                    for (id, coeff) in rest_reduced.iter().enumerate() {
                        let source = &table[2 * (i + j - k) as usize][id];
                        result.add(source, (c * coeff) % self.p);
                    }
                }
            }
        }
        result
    }


    pub fn multiply(&self, result : &mut FpVector, coeff : u32, 
                            r_degree : i32, r_index : usize, 
                            s_degree : i32, s_index : usize, excess : i32)
    {
        if coeff == 0 {
            return;
        }
        assert!(r_index < self.get_dimension(r_degree, excess + s_degree));
        assert!(s_index < self.get_dimension(s_degree, excess));

        if s_degree == 0 {
            // If s is of length 0 then max_idx "r->P_length" is off the edge of the list and it segfaults.
            // Avoid this by returning early in this case.
            result.add_basis_element(r_index, coeff);
            return;
        }
        let r = self.basis_element_from_index(r_degree, r_index);
        let s = self.basis_element_from_index(s_degree, s_index);
        let mut monomial = AdemBasisElement {
            degree : r.degree + s.degree,
            excess : 0,
            bocksteins : 0,
            ps : Vec::with_capacity(r.ps.len() + s.ps.len())
        };
        if self.generic && (r.bocksteins >> r.ps.len()) & s.bocksteins & 1 == 1 {
            // If there is a bockstein at the end of r and one at the beginning of s, these run into each other
            // and the output is 0.
            return;
        } else if self.generic {
            monomial.bocksteins = r.bocksteins;
            monomial.bocksteins |= s.bocksteins << (r.ps.len());
        }
        
        for cur_p in &r.ps {
            monomial.ps.push(*cur_p);
        }
        for cur_p in &s.ps {
            monomial.ps.push(*cur_p);
        }        
        assert!(monomial.ps.len() == r.ps.len() + s.ps.len());
        if self.generic {
            // If r ends in a bockstein, we need to move it over because we consider
            // the monomial from right to left in chunks like bP^i. The b from the end of r gets donated
            // to the P from the beginning of s.
            let leading_degree = r.degree - ((r.bocksteins >> r.ps.len()) & 1) as i32;
            self.make_mono_admissible_generic(result, coeff, monomial, r.ps.len() as i32 - 1, leading_degree, excess, true);
        } else {
            self.make_mono_admissible_2(result, monomial, r.ps.len() as i32 - 1, r.degree, excess, true);
        }
    }

    pub fn make_mono_admissible(&self, result : &mut FpVector, coeff : u32, monomial : AdemBasisElement, excess : i32){
        let q = if self.generic { 2 * self.p - 2 } else { 1 };
        let mut leading_degree = monomial.degree - (q * monomial.ps[monomial.ps.len() - 1]) as i32;
        let idx = monomial.ps.len() as i32 - 2;    
        if self.generic {
            leading_degree -= ((monomial.bocksteins >> (monomial.ps.len() - 1)) & 1) as i32;
            self.make_mono_admissible_generic(result, coeff, monomial, idx, leading_degree, excess, false);
        } else {
            self.make_mono_admissible_2(result, monomial, idx, leading_degree, excess, false);
        }
    }

    /**
    * Reduce a Steenrod monomial at the prime 2.
    * # Arguments:
    *  * `algebra` - an Adem algebra. This would be a method of class AdemAlgebra.
    *  * `result`  - Where we put the result
    *  * `monomial` - a not necessarily admissible Steenrod monomial which we will reduce.
    *                We destroy monomial->Ps.
    *  * `idx` - the only index to check for inadmissibility in the input (we assume that we've gotten
    *           our input as a product of two admissible sequences.)
    *  * `leading_degree` - the degree of the squares between 0 and idx (so of length idx + 1)
    */
    fn make_mono_admissible_2(
        &self, result : &mut FpVector, mut monomial : AdemBasisElement,
        mut idx : i32, mut leading_degree : i32, excess : i32, stop_early : bool
    ){
        while idx < 0 || idx as usize == monomial.ps.len() - 1 || monomial.ps[idx as usize] >= 2*monomial.ps[idx as usize + 1] {
            if idx < 0 || stop_early {
                // Admissible so write monomial to result.
                let idx = self.basis_element_to_index(&monomial);
                // If excess is too large, quit. It's faster to check this by comparing idx to dimension
                // than to use fromIndex because fromIndex  dereferences a hash map.
                if self.unstable && idx >= self.get_dimension(monomial.degree, excess) {
                    return;
                }
                result.add_basis_element(idx, 1);
                return;
            }
            leading_degree -= monomial.ps[idx as usize] as i32;
            idx -= 1;
        }
        let tuple = self.tail_of_basis_element_to_index(monomial, idx as u32 + 1, 1);
        monomial = tuple.0;
        let adm_idx = tuple.1;
        let x = monomial.ps[idx as usize] as i32;
        let tail_degree = monomial.degree - leading_degree + x;
        let reduced_tail = &self.multiplication_table[tail_degree as usize][x as usize][adm_idx];
        for (it_idx, it_value) in reduced_tail.iter().enumerate() {
            if it_value == 0 {
                continue;
            }
            let cur_tail_basis_elt = self.basis_element_from_index(tail_degree, it_idx);
            let mut new_monomial = AdemBasisElement {
                degree : monomial.degree,
                excess : -1,
                bocksteins : 0,
                ps : Vec::with_capacity(idx as usize + cur_tail_basis_elt.ps.len())
            };
            for i in 0..idx {
                new_monomial.ps.push(monomial.ps[i as usize]);
            }
            for cur_p in &cur_tail_basis_elt.ps {
                new_monomial.ps.push(*cur_p);
            }
            self.make_mono_admissible_2(result, new_monomial, idx - 1, leading_degree - x, excess, stop_early);
        }
    }

    fn make_mono_admissible_generic(
        &self, result : &mut FpVector, coeff : u32, mut monomial : AdemBasisElement,
        mut idx : i32, mut leading_degree : i32, excess : i32, stop_early : bool        
    ){
        let p = self.p;
        let q = 2*p-2;
        // Check for admissibility
        let mut b1 = 0;
        if idx >= 0 {
            b1 = (monomial.bocksteins >> idx) & 1;
        }
        let b2 = (monomial.bocksteins >> (idx+1)) & 1;
        while idx < 0 || idx == monomial.ps.len() as i32 - 1 || monomial.ps[idx as usize] >= p*monomial.ps[idx as usize + 1] + b2 {
            if idx < 0 || stop_early {
                // Admissible so write monomial to result.
                let idx = self.basis_element_to_index(&monomial);
                if self.unstable && idx >= self.get_dimension(monomial.degree, excess) {
                    return;
                }
                result.add_basis_element(idx, coeff);
                return;
            }
            leading_degree -= (q * monomial.ps[idx as usize]) as i32;
            leading_degree -= ((monomial.bocksteins >> idx) & 1) as i32;
            idx -= 1;
        }
        let tuple = self.tail_of_basis_element_to_index(monomial, idx as u32 + 1, q);
        monomial = tuple.0;
        let adm_idx = tuple.1;
        // Notice how much we avoid bockstein twiddling here. It's all hidden in multiplication_table =)
        let x = monomial.ps[idx as usize];
        let bx = (x << 1) + b1;
        let tail_degree = monomial.degree - leading_degree + (q*x + b1) as i32;
        let reduced_tail = &self.multiplication_table[tail_degree as usize][bx as usize][adm_idx];
        let dim = self.get_dimension(tail_degree, excess);    
        for (it_idx, it_value) in reduced_tail.iter().enumerate() {
            if it_value == 0 {
                continue;
            }
            if it_idx >= dim {
                break;
            }
            let cur_tail_basis_elt = self.basis_element_from_index(tail_degree, it_idx);
            let mut new_monomial = AdemBasisElement {
                degree : monomial.degree,
                excess : -1,
                bocksteins : 0,
                ps : Vec::with_capacity(idx as usize + cur_tail_basis_elt.ps.len())
            };            
            for i in 0..idx {
                new_monomial.ps.push(monomial.ps[i as usize]);
            }
            for cur_p in &cur_tail_basis_elt.ps {
                new_monomial.ps.push(*cur_p);
            }
            new_monomial.bocksteins = monomial.bocksteins & ((1<<idx)-1);
            new_monomial.bocksteins |= cur_tail_basis_elt.bocksteins << idx;
            let new_leading_degree = leading_degree - (q*x + b1) as i32;
            self.make_mono_admissible_generic(result, (coeff * it_value) % p, new_monomial, idx - 1, new_leading_degree, excess, stop_early);
        }
    }


    fn decompose_basis_element_2(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let b = self.basis_element_from_index(degree, idx); 
        if b.ps.len() > 1 {
            let degree_first = b.ps[0] as i32;
            let degree_rest = b.degree - b.ps[0] as i32;
            let ps_rest = b.ps[1..].to_vec();
            let idx_first = self.basis_element_to_index(&AdemBasisElement {
                degree : degree_first,
                excess : 0,
                bocksteins : 0,
                ps : vec![b.ps[0]]
            });
            let idx_rest = self.basis_element_to_index(&AdemBasisElement {
                degree : degree_rest,
                excess : 0,
                bocksteins : 0,
                ps : ps_rest
            });
            return vec![(1,(degree_first, idx_first), (degree_rest, idx_rest))];
        }
        let sq = b.ps[0];
        let tz = sq.trailing_zeros();
        let first_sq = 1 << tz;
        let second_sq = sq ^ first_sq;
        let first_degree = first_sq as i32;
        let second_degree = second_sq as i32;
        let first_idx = self.basis_element_to_index(&AdemBasisElement {
            degree : first_degree,
            excess : 0,
            bocksteins : 0,
            ps : vec![first_sq]
        });
        let second_idx = self.basis_element_to_index(&AdemBasisElement {
            degree : second_degree,
            excess : 0,
            bocksteins : 0,
            ps : vec![second_sq]
        });
        let mut out_vec = FpVector::new(2, self.get_dimension(degree, -1));
        self.multiply_basis_elements(&mut out_vec, 1, first_degree, first_idx, second_degree, second_idx, -1);
        out_vec.set_entry(idx, 0);
        let mut result = Vec::new();
        result.push((1, (first_degree, first_idx), (second_degree, second_idx)));
        for (i, v) in out_vec.iter().enumerate() {
            if v == 0 {
                continue;
            }
            result.extend(self.decompose_basis_element_2(degree, i));
        }
        return result;
    }

    fn decompose_basis_element_generic(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        let p = self.prime();
        let b = self.basis_element_from_index(degree, idx); 
        let leading_bockstein_idx = 1;// << (b.ps.len());
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
            let first_degree = (b.ps[0] * 2 * (p-1)) as i32;
            let rest_degree = b.degree - first_degree;
            let ps_first = vec![b.ps[0]];
            let ps_rest = b.ps[1..].to_vec();
            let first = AdemBasisElement {
                degree : first_degree,
                bocksteins : 0,
                excess : 0,
                ps : ps_first
            };
            let rest = AdemBasisElement {
                degree : rest_degree,
                bocksteins : b.bocksteins >> 1,
                excess : 0,
                ps : ps_rest
            };
            let first_idx = self.basis_element_to_index(&first);
            let rest_idx = self.basis_element_to_index(&rest);
            return vec![(1, (first_degree, first_idx), (rest_degree, rest_idx))];
        }
        
        let sq = b.ps[0];
        let mut pow = 1;
        {
            let mut temp_sq = sq;
            while temp_sq % p == 0 {
                temp_sq /= p;
                pow *= p;
            }
        }

        let first_sq = pow;
        let second_sq = sq - first_sq;
        let first_degree = (first_sq * 2*(p-1)) as i32;
        let second_degree = (second_sq * 2*(p-1)) as i32;
        let first_idx = self.basis_element_to_index(&AdemBasisElement {
            degree : first_degree,
            excess : 0,
            bocksteins : 0,
            ps : vec![first_sq]
        });
        let second_idx = self.basis_element_to_index(&AdemBasisElement {
            degree : second_degree,
            excess : 0,
            bocksteins : 0,
            ps : vec![second_sq]
        });
        let mut out_vec = FpVector::new(p, self.get_dimension(degree, -1));
        self.multiply_basis_elements(&mut out_vec, 1, first_degree, first_idx, second_degree, second_idx, -1);
        let mut result = Vec::new();
        let c = out_vec.get_entry(idx);
        assert!(c != 0);
        let c_inv = combinatorics::inverse(p, p - c);        
        result.push((((p - 1) * c_inv) % p, (first_degree, first_idx), (second_degree, second_idx)));
        out_vec.set_entry(idx, 0);
        for (i, v) in out_vec.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let (c, t1, t2) = self.decompose_basis_element_generic(degree, i)[0];
            result.push(((c_inv * c * v) % p, t1, t2));
        }
        return result;
    }

    pub fn get_beps_pn(&self, e : u32, x : u32) -> (i32, usize) {
        let p = self.prime();
        let q = if self.generic { 2 * p - 2} else { 1 };
        let degree = (x * q + e) as i32;
        let index = self.basis_element_to_index(&AdemBasisElement {
            degree,
            excess : 0,
            bocksteins : e,
            ps : vec![x]
        });
        return (degree, index);
    }
}


// void AdemAlgebra__generateExcessTable(AdemAlgebraInternal *algebra, int old_max_degree, int max_degree){
//     algebra->excess_table = realloc(algebra->excess_table, sizeof(uint*)*max_degree);
//     for(int n=old_max_degree; n<max_degree; n++){
//         uint dim = AdemAlgebra_getDimension((Algebra*)algebra, n, -1);
//         algebra->excess_table[n] = malloc(n * sizeof(uint));
//         uint cur_excess = 0;
//         for(uint i=0; i < dim; i++){
//             AdemBasisElement *elt = AdemAlgebra_basisElement_fromIndex((AdemAlgebra*)algebra, n, i);
//             for(int j=cur_excess; j<elt->excess; j++){
//                 algebra->excess_table[n][j] = i;
//             }
//             cur_excess = elt->excess;
//         }
//         for(int j=cur_excess; j<n; j++){
//             algebra->excess_table[n][j] = dim;
//         }        
//     }
// }

// uint AdemAlgebra_getDimension_unstable(Algebra *this, int degree, int excess){
//     assert(degree < this->max_degree);
//     if(degree < 0){
//         return 0;
//     }
//     AdemAlgebraInternal *algebra = (AdemAlgebraInternal*) this;
//     if(excess >= degree){
//         return algebra->basis_table[degree].length;
//     }
//     return algebra->excess_table[degree][excess];
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_adem(){
        let p = 2;
        let A = AdemAlgebra::new(p, p != 2, false);
        A.compute_basis(10);
        let r_deg = 4;
        let r_idx = 0;
        let s_deg = 5;
        let s_idx = 0;
        let out_deg = r_deg + s_deg;
        let mut result1 = FpVector::new(p, A.get_dimension(out_deg, 0));
        let mut result2 = FpVector::new(p, A.get_dimension(out_deg, 0) + 3);
        result2.set_slice(3, 3 + result1.get_dimension());

        A.multiply_basis_elements(&mut result1, 1, r_deg, r_idx, s_deg, s_idx, 0);
        A.multiply_basis_elements(&mut result2, 1, r_deg, r_idx, s_deg, s_idx, 0);
        println!("result : {}", A.element_to_string(out_deg, &result1));
        println!("result : {}", A.element_to_string(out_deg, &result2));
    }

    use rstest::rstest_parametrize;

    #[rstest_parametrize(p, max_degree,
        case(2, 32),
        case(3, 120)
    )]
    fn test_adem_basis(p : u32, max_degree : i32){
        let algebra = AdemAlgebra::new(p, p != 2, false);
        algebra.compute_basis(max_degree);
        for i in 1 ..= max_degree {
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
        case(3, 120)
    )]
    fn test_adem_decompose(p : u32, max_degree : i32){
        let algebra = AdemAlgebra::new(p, p != 2, false);
        algebra.compute_basis(max_degree);
        for i in 1 ..= max_degree {
            let dim = algebra.get_dimension(i, -1);
            let gens = algebra.get_generators(i);
            println!("i : {}, gens : {:?}", i, gens);
            let mut out_vec = FpVector::new(p, dim);
            for j in 0 .. dim {
                if gens.contains(&j){
                    continue;
                }
                for (coeff, (first_degree, first_idx), (second_degree, second_idx)) in algebra.decompose_basis_element(i, j) {
                    print!("{} * {} * {}  +  ", coeff, algebra.basis_element_to_string(first_degree,first_idx), algebra.basis_element_to_string(second_degree, second_idx));
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
        case(3, 120)
    )]
    fn test_adem_relations(p : u32, max_degree : i32){
        let algebra = AdemAlgebra::new(p, p != 2, false);
        algebra.compute_basis(max_degree);
        let mut output_vec = FpVector::new(p, 0);
        for i in 1 ..= max_degree {
            output_vec.clear_slice();
            let output_dim = algebra.get_dimension(i, -1);
            if output_dim > output_vec.get_dimension() {
                output_vec = FpVector::new(p, output_dim);
            }
            output_vec.set_slice(0, output_dim);
            let relations = algebra.get_relations_to_check(i);
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
