use core::cmp::Ordering;
use lazy_static;
use std::collections::HashMap;
use std::format;

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

// #[derive(RustcDecodable, RustcEncodable)]
#[derive(Debug)]
#[derive(Clone)]
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
    even_basis_table : OnceVec<Vec<AdemBasisElement>>,
    basis_table : OnceVec<Vec<AdemBasisElement>>, // degree -> index -> AdemBasisElement
    basis_element_to_index_map : OnceVec<HashMap<AdemBasisElement, usize>>, // degree -> AdemBasisElement -> index
    multiplication_table : OnceVec<Vec<Vec<FpVector>>>,// degree -> first square -> admissibile sequence idx -> result vector
    excess_table : OnceVec<Vec<u32>>,
    sort_order : Option<fn(&AdemBasisElement, &AdemBasisElement) -> Ordering>,
    filtration_one_products : Vec<(String, i32, usize)> //Vec<Once<(i32, usize)>>
}

impl Algebra for AdemAlgebra {
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

        self.compute_basis(max_degree + 1); // So that we can use basis_element_to_index, + 1 because compute_basis is non-inclusive
        self.filtration_one_products = products.into_iter()
            .map(|(name, b)| (name, b.degree, self.basis_element_to_index(&b)))
            .collect();
    }

    fn compute_basis(&self, mut max_degree : i32) {
        let genericq = if self.generic { 1 } else { 0 };
        // assert!(max_degree + genericq <= self.basis_table.len() as i32);
        combinatorics::initialize_prime(self.p);
        let old_max_degree = self.get_max_degree();
        if max_degree <= old_max_degree {
            return;
        }        
        if self.generic {
            // generateMultiplcationTableGeneric sometimes goes over by one due to its bockstein logic.
            // rather than testing for this, we take the lazy way out and calculate everything else out one extra step.
            max_degree += 1;
        }
        let mut max_degree = max_degree;
        let mut old_max_degree = old_max_degree;
        if self.generic {
            self.generate_basis_generic(old_max_degree, max_degree);
        } else {
            self.generate_basis2(old_max_degree, max_degree);
        }
        self.generate_basis_element_to_index_map(old_max_degree, max_degree);
        if self.generic {
            // AdemAlgebra__generateMultiplicationTable consumes the one extra degree we computed in the generic case
            max_degree -= 1;
            if old_max_degree > 0 {
                old_max_degree -= 1;
            }
        }
        self.generate_multiplication_table(old_max_degree, max_degree);
        // if self.max_degree 
        // println!("self.generate_multiplication_table({}, {})", old_max_degree, max_degree);
        // if self.unstable {
        //     self.generate_excess_table(old_max_degree, max_degree);
        // }
        // Make sure product_list reflects sort order.
        // for i in 0 .. self.filtrationOneProduct_basisElements.length {
        //     AdemBasisElement *b = self.filtrationOneProduct_basisElements.list[i];
        //     if(b->degree < max_degree){
        //         self->product_list.list[i].index = self.basis_element_to_index(b);
        //     }
        // }
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
                    bocksteins <<= 1;
                    bocksteins |= sq;
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
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        format!("{}", self.basis_element_from_index(degree, idx))
    }

    /// Every basis element is a generator. Surely we can do better than this...
    fn get_algebra_generators(&self, degree : i32) -> Vec<usize> {
        (0..self.get_dimension(degree, -1)).collect()
    }

    /// If every basis element is a generator, we never need to decompose!
    fn decompose_basis_element(&self, degree : i32, idx : usize) -> Vec<(u32, (i32, usize), (i32, usize))> {
        Vec::new()
    }
}

// static void AdemAlgebra__initializeFields(AdemAlgebraInternal *algebra, uint p, bool generic, bool unstable);
// uint AdemAlgebra__generateName(AdemAlgebra *algebra); // defined in adem_io
impl AdemAlgebra {
    pub fn new(p : u32, generic : bool, unstable : bool) -> Self {
        crate::combinatorics::initialize_prime(p);
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

    fn get_max_degree(&self) -> i32 {
        return self.basis_table.len() as i32;
    }

    fn generate_basis_even(&self, mut old_max_degree : i32, max_degree : i32){
        if old_max_degree == 0 {
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
            old_max_degree += 1;
        }

        for n in old_max_degree .. max_degree {
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


    fn generate_basis2(&self, old_max_degree : i32, max_degree : i32){
        self.generate_basis_even(old_max_degree, max_degree);
        for n in old_max_degree as usize .. max_degree as usize {
            let table = &self.even_basis_table[n];
            self.basis_table.push(table.clone());
        }
        // if let Some(f) = self.sort_order {
        //     for 
        // }

    }


    // Our approach is to pick the bocksteins and the P's separately and merge.
    fn generate_basis_generic(&self, old_max_degree : i32, max_degree : i32){
        self.generate_basis_even(old_max_degree, max_degree);
        for n in old_max_degree .. max_degree {
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

    fn generate_basis_element_to_index_map(&self, old_max_degree : i32, max_degree : i32){
        for n in old_max_degree as usize .. max_degree as usize {
            let basis = &self.basis_table[n];
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

    fn generate_multiplication_table(&self, old_max_degree : i32, max_degree : i32){
        if self.generic {
            self.generate_multiplication_table_generic(old_max_degree, max_degree);
        } else {
            self.generate_multiplication_table_2(old_max_degree, max_degree);
        }
    }    

    fn generate_multiplication_table_2(&self, mut old_max_degree : i32, max_degree : i32){
        // degree -> first_square -> admissibile sequence idx -> result vector
        if old_max_degree == 0 {
            self.multiplication_table.push(Vec::new());
            old_max_degree += 1;
        }
        for n in old_max_degree .. max_degree {
            let mut table : Vec<Vec<FpVector>> = Vec::with_capacity((n + 1) as usize);
            table.push(Vec::with_capacity(0));
            for x in 1 .. n + 1 {
                let dimension = self.get_dimension(n - x, -1);
                table.push(Vec::with_capacity(dimension));
            }
            for x in (1 .. n + 1).rev() {
                for idx in 0 .. self.get_dimension(n - x, -1) {
                    let entry;
                    unsafe {
                        entry = &mut *(&mut table[x as usize] as *mut Vec<FpVector>);
                    }
                    entry.push(self.generate_multiplication_table2_step(&table, n, x, idx));
                }
                let dimension = self.get_dimension(n - x, -1);
                assert!(table[x as usize].len() == dimension);
            }
            self.multiplication_table.push(table);
        }
    }

    fn generate_multiplication_table2_step(&self, table : &Vec<Vec<FpVector>>, n : i32, x : i32, idx : usize) -> FpVector {
        let output_dimension = self.get_dimension(n, -1);
        let mut result = FpVector::new(self.p, output_dimension, 0);
        let cur_basis_elt = self.basis_element_from_index(n-x, idx);
        let mut working_elt = AdemBasisElement {
            degree : n,
            excess : 0,
            bocksteins : 0,
            ps : Vec::with_capacity(cur_basis_elt.ps.len() + 1)
        };
        working_elt.ps.push(x as u32);
        for cur_p in &cur_basis_elt.ps {
            working_elt.ps.push(*cur_p);
        }
        // println!("working_elt: {:?}", working_elt);
        assert!(working_elt.ps.len() == working_elt.ps.capacity());
        // Be careful to deal with the case that cur_basis_elt has length 0            
        // If the length is 0 or the sequence is already admissible, we can just write a 1 in the answer
        // and continue.
        if cur_basis_elt.ps.len() == 0 || x as u32 >= 2*cur_basis_elt.ps[0] {
            let out_idx = self.basis_element_to_index(&working_elt);
            result.add_basis_element(out_idx, 1);
            return result;
        }
        let y = working_elt.ps[1] as i32;
        // We only needed the extra first entry to perform the lookup if our element
        // happened to be admissible. Otherwise, take the rest of the list and forget about it.
        working_elt.degree -= working_elt.ps[0] as i32;
        unsafe { working_elt.ps = shift_vec(working_elt.ps, 1) };
        for j in 0 .. 1 + x/2 {
            if combinatorics::binomial(2, y - j - 1, x - 2*j) == 0 {
                continue;
            }
            if j==0 {
                working_elt.ps[0] = (x + y) as u32;
                working_elt.degree += x;
                // In this case the result is guaranteed to be admissible so we can immediately add it to result
                let out_idx = self.basis_element_to_index(&working_elt);
                result.add_basis_element(out_idx, 1);
                continue;
            }
            // Now we need to reduce Sqj * (rest of Sqs)
            // The answer to this is in the table we're currently making.
            let tuple = self.tail_of_basis_element_to_index(working_elt, 1, 1);
            working_elt = tuple.0;
            let working_elt_idx = tuple.1;
            // total degree -> first sq -> idx of rest of squares
            let rest_reduced = &self.multiplication_table[(n as i32 - (x + y) + j) as usize][j as usize][working_elt_idx];
            for (i, coeff) in rest_reduced.iter().enumerate() {
                if coeff == 0 {
                    continue;
                }
                // Reduce Sq^{x+y-j} * whatever square using the table in the same degree, larger index
                // Since we're doing the first squares in decreasing order and x + y - j > x, 
                // we already calculated this.
                let source = &table[x as usize + y as usize -j as usize][i as usize];
                result.add(source, 1);
            }
        }
        unsafe { working_elt.ps = shift_vec(working_elt.ps, -1) };
        return result;
    }

    fn generate_multiplication_table_generic(&self, mut old_max_degree : i32, max_degree : i32){
        // degree -> first_square -> admissibile sequence idx -> result vector
        let p = self.p as i32;
        let q = 2*p-2;
        if old_max_degree == 0 {
            self.multiplication_table.push(Vec::new());
            old_max_degree += 1;
        }
        // Okay so this is really confusing. The way the table is represented, first_square = 2n represents P^n 
        // and first_square = 2n + 1 represents b P^n
        // It is very easy to work out the product b P^n * x from the product P^n * x because b multiplication is exact. 
        // We drop terms starting with a b and add a b to the start of terms that don't start with a b.
        // b (P^n * x) lands in a degree one higher than the degree of P^n * x. So we fill out the even first_square entries of
        // the degree n table at the same time as the odd first_square entries of the n + 1 table. At the end we get a half filled
        // table which we throw away. When we start the next time, we have to start back one degree and reconstruct the odd half
        // of the table in degree old_max_degree (we also reconstruct the even part of the table in degree old_max_degree - 1 and 
        // throw it away).
        // This logic makes the next ~30 lines of code a little confusing.
        let mut tables : Vec<Option<Vec<Vec<FpVector>>>> = Vec::with_capacity((max_degree - old_max_degree + 2) as usize);
        for n in old_max_degree - 1 .. max_degree + 1 {
            let output_dimension = self.get_dimension(n, -1);
            let num_entries = 2*(n/q + 1) as usize;
            let mut table : Vec<Vec<FpVector>> = Vec::with_capacity(num_entries);
            for i in 0 .. n/q + 1 {
                for b in 0 .. 2 {
                    let dimension = self.get_dimension(n - q*i - b, -1);
                    let entry : Vec<FpVector> = Vec::with_capacity(dimension);                    
                    table.push(entry);
                }
            }
            tables.push(Some(table));
        }
        let mut table;
        let mut next_table = tables[0].take().unwrap();
        for n in old_max_degree - 1 .. max_degree {
            table = next_table;
            next_table = tables[(n - old_max_degree + 2) as usize].take().unwrap();
            for x in (0 .. n/q + 1).rev() {
                let x_index = x << 1;
                let beta_x_index = x_index | 1;
                for idx in 0 .. self.get_dimension(n - q*x, -1) {
                    let (result, beta_result) = self.generate_multiplication_table_generic_step(&table, n, x, idx);
                    table[x_index as usize].push(result);
                    next_table[beta_x_index as usize].push(beta_result);
                }
            }        
            if n >= old_max_degree {
                self.multiplication_table.push(table);
            }
        }
    }
// self.multiplication_table[n as usize][x_index as usize].push(result);
// self.multiplication_table[n as usize + 1][beta_x_index as usize].push(beta_result);

    fn generate_multiplication_table_generic_step(&self, table : &Vec<Vec<FpVector>>,  n : i32, x : i32, idx : usize) -> (FpVector, FpVector){
        let p = self.p;
        let q = (2*p-2) as i32;
        let output_dimension = self.get_dimension(n, -1);
        let beta_output_dimension = self.get_dimension(n + 1, -1);
        let mut result = FpVector::new(self.p, output_dimension, 0);
        let mut beta_result = FpVector::new(self.p, beta_output_dimension, 0);
        
        let cur_basis_elt = self.basis_element_from_index(n - q * x, idx);

        let x_len = (x>0) as usize;
        let mut working_elt = AdemBasisElement {
            degree : n,
            excess : 0,
            bocksteins : cur_basis_elt.bocksteins << x_len,
            ps : Vec::with_capacity(cur_basis_elt.ps.len() + x_len)
        };
        
        if x > 0 {
            working_elt.ps.push(x as u32);
        }
        for cur_p in &cur_basis_elt.ps {
            working_elt.ps.push(*cur_p);
        }

        // Enough space to fit Sq^x * (rest of Sqs)
        // Be careful to deal with the case that cur_basis_elt has length 0            
        // If the length is 0 or the sequence is already admissible, we can just write a 1 in the answer
        // and continue.
        let b = cur_basis_elt.bocksteins & 1;
        if cur_basis_elt.ps.len() == 0 || x == 0 || x >= (p*cur_basis_elt.ps[0] + b) as i32 {
            let mut out_idx = self.basis_element_to_index(&working_elt);
            result.add_basis_element(out_idx, 1);
            if working_elt.bocksteins & 1 == 1 {
                // Two bocksteins run into each other (only possible when x=0)
                return (result, beta_result);
            }
            working_elt.bocksteins |= 1;
            working_elt.degree += 1;
            out_idx = self.basis_element_to_index(&working_elt);
            beta_result.add_basis_element(out_idx, 1);
            return (result, beta_result);
        }
        let y = cur_basis_elt.ps[0] as i32;     
        // We only needed the extra first entry to perform the lookup if our element
        // happened to be admissible. Otherwise, take the rest of the list and forget about it.
        // (To prevent segfault, we have to reverse this before working_elt goes out of scope!)
        working_elt.degree -= q*x;
        working_elt.degree -= (working_elt.bocksteins & 1) as i32;
        working_elt.bocksteins >>= 1;
        let start_working_elt_degree = working_elt.degree;
        let start_working_elt_bocksteins = working_elt.bocksteins;
        unsafe { working_elt.ps = shift_vec(working_elt.ps, 1); }
        // Adem relation
        for e1 in 0 .. b + 1 {
            let e2 = b - e1;
            // e1 and e2 determine where a bockstein shows up.
            // e1 determines if a bockstein shows up in front 
            // e2 determines if a bockstein shows up in middle
            // So our output term looks like b^{e1} P^{x+y-j} b^{e2} P^{j}
            let pi32 = p as i32;
            for j in 0 .. x/pi32 + 1 {
                let mut c = combinatorics::binomial(p, (y-j) * (pi32-1) + e1 as i32 - 1, x - pi32*j - e2 as i32);
                if c == 0 {
                    continue;
                }
                c *= combinatorics::minus_one_to_the_n(p, (x + j) as u32 + e2);
                c = c % p;
                if j == 0 {
                    if e2 & (working_elt.bocksteins >> 1) == 1 {
                        // Two bocksteins run into each other:
                        // P^x b P^y b --> P^{x+y} b b = 0
                        continue;
                    }
                    working_elt.ps[0] = (x + y) as u32;
                    working_elt.degree += q * x;
                    // Mask out bottom bit of original bocksteins.
                    working_elt.bocksteins &= !1;
                    // Now either the front bit or idx + 1 might need to be set depending on e1 and e2.
                    working_elt.bocksteins |= e1;
                    working_elt.bocksteins |= e2 << 1; 
                    // In this case the result is guaranteed to be admissible so we can immediately add it to result
                    let out_idx = self.basis_element_to_index(&working_elt);
                    result.add_basis_element(out_idx, c);
                    if e1==0 {
                        working_elt.bocksteins |= 1;
                        working_elt.degree += 1;
                        let out_idx = self.basis_element_to_index(&working_elt);
                        beta_result.add_basis_element(out_idx, c);
                    }
                    working_elt.degree = start_working_elt_degree;
                    working_elt.bocksteins = start_working_elt_bocksteins;
                    continue;
                }
                working_elt.degree = n - q*(x + y) - b as i32;
                working_elt.bocksteins >>= 1;
                // Now we need to reduce b^{e2} P^j * (rest of term)
                // The answer to this is in the table we're currently making.
                unsafe { working_elt.ps = shift_vec(working_elt.ps, 1); }
                let working_elt_idx = self.basis_element_to_index(&working_elt);
                unsafe { working_elt.ps = shift_vec(working_elt.ps, -1); }
                let bj_idx = (j<<1) as u32 + e2;
                // (rest of term) has degree n - q*(x + y) - b, 
                // b^{e2} P^j has degree q*j + e2, so the degree of the product is the sum of these two quantities.
                let bj_degree = q*j + (e2 as i32);
                let bpj_rest_degree = working_elt.degree + bj_degree;
                // total degree ==> b^eP^j ==> rest of term idx ==> Vector
                let rest_of_term = &self.multiplication_table[bpj_rest_degree as usize][bj_idx as usize][working_elt_idx];
                for (rest_of_term_idx, rest_of_term_coeff) in rest_of_term.iter().enumerate() {
                    if rest_of_term_coeff == 0 {
                        continue;
                    }
                    // Reduce P^{x+y-j} * whatever square using the table in the same degree, larger index
                    // Since we're doing the first squares in decreasing order and x + y - j > x, 
                    // we already calculated this.
                    let bj_idx = ((x+y-j) << 1) + e1 as i32;
                    let output_vector = &table[bj_idx as usize][rest_of_term_idx];
                    result.add(output_vector, (c*rest_of_term_coeff)%p);
                    for (output_index, output_value) in output_vector.iter().enumerate() {
                        if output_value == 0 {
                            continue;
                        }
                        let mut z = self.basis_element_from_index(n, output_index).clone();
                        // let z = self.basis_element_from_index_mut(n, output_index);
                        if z.bocksteins & 1 == 0 {
                            z.bocksteins |= 1;
                            z.degree += 1;
                            let idx = self.basis_element_to_index(&z);
                            beta_result.add_basis_element(idx, (output_value * c * rest_of_term_coeff) % p);
                        }
                    }
                }
                working_elt.degree = start_working_elt_degree;
                working_elt.bocksteins = start_working_elt_bocksteins;
            }
        }
        unsafe { working_elt.ps = shift_vec(working_elt.ps, -1); }
        return (result, beta_result)
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
        let mut result1 = FpVector::new(p, A.get_dimension(out_deg, 0), 0);
        let mut result2 = FpVector::new(p, A.get_dimension(out_deg, 0), 3);
        A.multiply_basis_elements(&mut result1, 1, r_deg, r_idx, s_deg, s_idx, 0);
        A.multiply_basis_elements(&mut result2, 1, r_deg, r_idx, s_deg, s_idx, 0);
        println!("result : {}", A.element_to_string(out_deg, result1));
        println!("result : {}", A.element_to_string(out_deg, result2));
    }

}

// def test_Adem_exhaustive(algebra_type, p, max_deg):
//     sage_products = sage_products_dict[(algebra_type, p)]
//     A = cAlgebra.getAlgebra(algebra_type + "Algebra", p=p, max_degree=max_deg)
//     for degree_d_products in sage_products:
//         for entry in degree_d_products:
//             if(len(entry[0]) == 0 or len(entry[1])==0):
//                 continue
//             x = A.py_algebra.get_basis_element(basis_elt_to_tuples(entry[0]))
//             y = A.py_algebra.get_basis_element(basis_elt_to_tuples(entry[1]))
//             res = A.multiply(x,y)
//             sage_res_dict = {}
//             for k,v in entry[2]:
//                 k = basis_elt_to_tuples(k)
//                 sage_res_dict[k] = v
//             sage_res = A.py_algebra.get_element(sage_res_dict)
//             assert res == sage_res
//             if res != sage_res:
//                 return
