
#![allow(dead_code)]
#![allow(unused_variables)]

use bivec::BiVec;
use once::OnceBiVec;
use fp::prime::ValidPrime;
use fp::matrix::{Matrix, Basis, Subspace};
use fp::vector::{FpVector, FpVectorT};


use std::sync::{RwLock, RwLockWriteGuard, Mutex};
use std::collections::{BTreeMap, HashMap};

pub type Indecomposable = (i32, i32, usize);

#[derive(Clone)]
pub struct Monomial(pub Vec<(Indecomposable, i32)>);

impl Monomial {
    pub fn unit() -> Self {
        Self(vec!())
    }

    pub fn indecomposable(x : i32, y : i32, idx : usize) -> Self {
        Self(vec![((x, y, idx), 1)])
    }

    pub fn contains_indecomposable(&self, x : i32, y : i32, idx : usize) -> bool {
        self.0.iter().any(|&((x1, y1, idx1),_)|
            x1 == x && y1 == y && idx1 == idx
        )
    }

    pub fn contains_bidegree(&self, x : i32, y : i32) -> bool {
        self.0.iter().any(|&((x1, y1, _),_)|
            x1 == x && y1 == y
        )
    }

    pub fn multiply(&self, other : &Self) -> Self {
        let mut result = self.clone();
        for (v, exp) in &other.0 {
            let exp = *exp;
            if let Some(idx) = result.0.iter().position(|(w, _)| *v == *w) {
                result.0[idx].1 += exp;
            } else {
                result.0.push((v.clone(), exp));
            }
        }
        result
    }
}

pub struct BidegreeData {
    pub dimension : usize,
    pub basis : Basis,
    product_tensor : BiVec<BiVec<Vec<Vec<Option<FpVector>>>>>,
    pub decomposables : Subspace,
    pub indecomposable_decompositions : Vec<(FpVector, Monomial)>
}

impl BidegreeData {
    pub fn new(p : ValidPrime, dimension : usize, product_tensor : BiVec<BiVec<Vec<Vec<Option<FpVector>>>>>) -> Self {
        Self {
            dimension,
            basis : Basis::new(p, dimension),
            product_tensor,
            decomposables : Subspace::new(p, dimension + 1, dimension),
            indecomposable_decompositions : Vec::new()
        }
    }
}

pub struct DenseBigradedAlgebra {
    p : ValidPrime,
    pub min_x : i32,
    pub min_y : i32,
    pub data : OnceBiVec<OnceBiVec<RwLock<BidegreeData>>>,
    updated_bidegrees : Mutex<Vec<(i32, i32)>>,
}

impl DenseBigradedAlgebra {
    pub fn new(p : ValidPrime, min_x : i32, min_y : i32) -> Self {
        Self {
            p,
            min_x,
            min_y,
            data : OnceBiVec::new(min_x),
            named_indecomposables : RwLock::new(BTreeMap::new()),
            updated_bidegrees : Mutex::new(Vec::new()),
        }
    }

    pub fn max_x(&self) -> i32 {
        self.data.len()
    }

    pub fn max_y(&self) -> i32 {
        if self.max_x() > self.min_x {
            self.data[self.min_x].len()
        } else {
            self.min_y
        }
    }

    pub fn from_dimension_vec(p : ValidPrime, min_x : i32, min_y : i32, dimensions : &Vec<Vec<usize>>) -> Self {
        let result = Self::new(p, min_x, min_y);
        for (i, v) in dimensions.iter().enumerate() {
            for (j, &d) in v.iter().enumerate() {
                result.insert_bidegree(i as i32 + min_x, j as i32 + min_y, d);
            }
        }
        result
    }

    pub fn from_dimension_bivec(p : ValidPrime, dimensions : &BiVec<BiVec<usize>>) -> Self {
        assert!(!dimensions.is_empty());
        let min_x = dimensions.min_degree();
        let min_y = dimensions[min_x].min_degree();
        let result = Self::new(p, min_x, min_y);
        for (i, v) in dimensions.iter_enum() {
            for (j, &d) in v.iter_enum() {
                result.insert_bidegree(i, j, d);
            }
        }
        result
    }

    pub fn prime(&self) -> ValidPrime {
        self.p
    }

    pub fn dimension(&self, x : i32, y : i32) -> usize {
        self.data[x][y].read().unwrap().dimension
    }

    fn insert_bidegree(&self, x : i32, y : i32, dimension : usize){
        assert!(x < self.data.len() && y == self.data[x].len() || x == self.data.len() && y == self.min_y);
        if x == self.data.len() {
            self.data.push(OnceBiVec::new(self.min_y));
        }
        let mut x_vec = BiVec::with_capacity(self.min_x, x);
        for i in self.min_x..=x {
            let mut y_vec = BiVec::with_capacity(self.min_y, y);
            for j in self.min_y..=y {
                let left_dim = if i < x || j < y {
                    self.data[i][j].read().unwrap().dimension
                } else {
                    dimension
                };
                let right_dim = if i > 0 || j > 0 {
                    self.data[x - i][y - j].read().unwrap().dimension
                } else {
                    dimension
                };
                
                let mut left_idx_vec = Vec::with_capacity(left_dim);
                for _ in 0..left_dim {
                    left_idx_vec.push(vec![None; right_dim]);
                }
                y_vec.push(left_idx_vec);
            }
            x_vec.push(y_vec);
        }
        let new_data = RwLock::new(BidegreeData::new(self.p, dimension, x_vec));
        self.data[x].push(new_data);
    }

    pub fn set_product(&self, 
        left_x : i32, left_y : i32, left_idx : usize, 
        right_x : i32, right_y : i32, right_idx : usize, 
        output : FpVector
    ){
        assert!(output.prime() == self.prime());
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        let mut data = self.data[out_x][out_y].write().unwrap();
        assert!(data.dimension == output.dimension());
        data.product_tensor[left_x][left_y][left_idx][right_idx] = Some(output);
    }

    pub fn set_basis(&self, x : i32, y : i32, new_basis : &Matrix) {
        let mut data = self.data[x][y].write().unwrap();
        self.updated_bidegrees.lock().unwrap().push((x, y));
        assert!(new_basis.rows() == data.basis.matrix.rows());
        assert!(new_basis.columns() == data.basis.matrix.columns());
        for r in 0..new_basis.rows() {
            if data.basis.matrix[r] != new_basis[r] {
                data.basis.matrix[r].assign(&new_basis[r]);
            }
        }
        data.basis.calculate_inverse();
        drop(data);
        let mut sv = FpVector::new(self.prime(), 0);
        drop(self.compute_indecomposables_in_bidegree(x, y, &mut sv));
    }

    pub fn multiply_basis_element_by_basis_element(&self, 
        result : &mut FpVector,
        coeff : u32,
        left_x : i32, left_y : i32, left_index : usize, 
        right_x : i32, right_y : i32, right_index : usize,
        sv_left : &mut FpVector, sv_right : &mut FpVector, sv_out : &mut FpVector
    ) -> Result<(),()> {
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        let left = self.data[left_x][left_y].read().unwrap();
        let right = self.data[right_x][right_y].read().unwrap();
        let out = self.data[out_x][out_y].read().unwrap();
        assert!(out.dimension == result.dimension());
        sv_left.set_scratch_vector_size(left.dimension);
        sv_right.set_scratch_vector_size(right.dimension);
        sv_out.set_scratch_vector_size(out.dimension);
        sv_left.assign(&left.basis.matrix[left_index]);
        sv_right.assign(&right.basis.matrix[right_index]);
        for (i, v) in sv_left.iter_nonzero() {
            for(j, w) in sv_right.iter_nonzero(){
                let entry = (v * w * coeff) % *self.prime();
                sv_out.add(out.product_tensor[left_x][left_y][i][j].as_ref().ok_or(())?, entry);
            }
        }
        out.basis.apply_inverse(result, coeff, sv_out);
        Ok(())
    }

    pub fn multiply_element_by_basis_element(&self, 
        result : &mut FpVector,
        coeff : u32,
        left_x : i32, left_y : i32, left_input : &FpVector, 
        right_x : i32, right_y : i32, right_index : usize,
        sv_left : &mut FpVector, sv_right : &mut FpVector, sv_out : &mut FpVector
    ) -> Result<(),()> {
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        let left = self.data[left_x][left_y].read().unwrap();
        let right = self.data[right_x][right_y].read().unwrap();
        let out = self.data[out_x][out_y].read().unwrap();
        assert!(left.dimension == left_input.dimension());
        assert!(out.dimension == result.dimension());
        sv_left.set_scratch_vector_size(left.dimension);
        sv_right.set_scratch_vector_size(right.dimension);
        sv_out.set_scratch_vector_size(out.dimension);
        left.basis.apply(sv_left, 1, left_input);
        sv_right.assign(&right.basis.matrix[right_index]);
        for (i, v) in sv_left.iter_nonzero() {
            for(j, w) in sv_right.iter_nonzero(){
                let entry = (v * w * coeff) % *self.prime();
                sv_out.add(out.product_tensor[left_x][left_y][i][j].as_ref().ok_or(())?, entry);
            }
        }
        out.basis.apply_inverse(result, coeff, sv_out);
        Ok(())
    }

    pub fn multiply_basis_element_by_element(&self, 
        result : &mut FpVector,
        coeff : u32,
        left_x : i32, left_y : i32, left_index : usize, 
        right_x : i32, right_y : i32, right_input : &FpVector,
        sv_left : &mut FpVector, sv_right : &mut FpVector, sv_out : &mut FpVector
    ) -> Result<(),()> {
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        let left = self.data[left_x][left_y].read().unwrap();
        let right = self.data[right_x][right_y].read().unwrap();
        let out = self.data[out_x][out_y].read().unwrap();
        assert!(right.dimension == right_input.dimension());
        assert!(out.dimension == result.dimension());
        sv_left.set_scratch_vector_size(left.dimension);
        sv_right.set_scratch_vector_size(right.dimension);
        sv_out.set_scratch_vector_size(out.dimension);
        sv_left.assign(&left.basis.matrix[left_index]);
        right.basis.apply(sv_right, 1, right_input);
        for (i, v) in sv_left.iter_nonzero() {
            for(j, w) in sv_right.iter_nonzero(){
                let entry = (v * w * coeff) % *self.prime();
                sv_out.add(out.product_tensor[left_x][left_y][i][j].as_ref().ok_or(())?, entry);
            }
        }
        out.basis.apply_inverse(result, coeff, sv_out);
        Ok(())
    }

    pub fn multiply_element_by_element(&self, 
        result : &mut FpVector,
        coeff : u32,
        left_x : i32, left_y : i32, left_input : &FpVector, 
        right_x : i32, right_y : i32, right_input : &FpVector,
        sv_left : &mut FpVector, sv_right : &mut FpVector, sv_out : &mut FpVector
    ) -> Result<(),()> {
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        let left = self.data[left_x][left_y].read().unwrap();
        let right = self.data[right_x][right_y].read().unwrap();
        let out = self.data[out_x][out_y].read().unwrap();
        assert!(left.dimension == left_input.dimension());
        assert!(right.dimension == right_input.dimension());
        assert!(out.dimension == result.dimension());
        sv_left.set_scratch_vector_size(left.dimension);
        sv_right.set_scratch_vector_size(right.dimension);
        sv_out.set_scratch_vector_size(out.dimension);
        left.basis.apply(sv_left, 1, left_input);
        right.basis.apply(sv_right, 1, right_input);
        for (i, v) in sv_left.iter_nonzero() {
            for(j, w) in sv_right.iter_nonzero(){
                let entry = (v * w * coeff) % *self.prime();
                sv_out.add(out.product_tensor[left_x][left_y][i][j].as_ref().ok_or(())?, entry);
            }
        }
        out.basis.apply_inverse(result, coeff, sv_out);
        Ok(())
    }

    pub fn compute_indecomposables_in_bidegree(&self, x : i32, y : i32, sv : &mut FpVector) {
        let mut data = self.data[x][y].write().unwrap();
        data.decomposables.set_to_zero();
        sv.set_scratch_vector_size(data.dimension);
        let product_tensor = std::mem::take(&mut data.product_tensor);
        for (x_left, t1) in product_tensor.iter_enum(){ 
            for (y_left, t2) in t1.iter_enum() {
                if (x_left == 0 && y_left == 0) 
                || (x_left == x && y_left == y) {
                    continue;
                }
                for v_opt in t2.iter().flat_map(|x| x.iter()) {
                    if let Some(v) = v_opt {
                        sv.set_to_zero_pure();
                        data.basis.apply(sv, 1, v);
                        data.decomposables.add_vector(sv);
                    }
                }
            }
        }
        data.product_tensor = product_tensor;
        sv.set_to_zero_pure();
    }

    pub fn initialize_indecomposable_decompositions(&self, unit_idx : usize){
        let mut data = self.data[0][0].write().unwrap();
        let mut vec = FpVector::new(self.prime(), data.dimension);
        vec.set_entry(unit_idx, 1);
        data.indecomposable_decompositions.push(
            (vec, Monomial::unit())
        );
        // self.update_indecomposable_decompositions_helper(
        //     &new_indecs,
        //     prev_decompositions,
        //     &mut sv_left, &mut sv_right, &mut sv_out
        // )
    }

    pub fn update_indecomposable_decompositions(&self) -> Result<(), ()> {
        let invalidated_bidegrees = self.updated_bidegrees.lock().unwrap();
        for mut data in self.data.iter().flat_map(|x| x.iter()).map(|x| x.write().unwrap()) {
            data.indecomposable_decompositions.drain_filter(|(vect, mono)|
                invalidated_bidegrees.iter().any(|&(x,y)| mono.contains_bidegree(x, y) )
            );
        }
        let prev_decompositions = {
            let mut prev_decompositions = Vec::new();
            for (x, r) in self.data.iter_enum() {
                for (y, data) in r.iter_enum() {
                    prev_decompositions.extend(
                        data.read().unwrap().indecomposable_decompositions.iter()
                        .map(|t| t.clone())
                        .map(|(vec, mono)|
                            (x, y, vec, mono)
                        )
                    );
                }
            }
            prev_decompositions
        };
        let mut sv_left = FpVector::new(self.prime(), 0);
        let mut sv_right = FpVector::new(self.prime(), 0);
        let mut sv_out = FpVector::new(self.prime(), 0);
        let new_indecs = self.new_named_indecomposables.lock().unwrap();
        self.update_indecomposable_decompositions_helper(
            &new_indecs,
            prev_decompositions,
            &mut sv_left, &mut sv_right, &mut sv_out
        )
    }

    fn update_indecomposable_decompositions_helper(&self, 
        new_indecs : &[(i32, i32, usize)], 
        prev_decompositions : Vec<(i32, i32, FpVector, Monomial)>,
        sv_left : &mut FpVector, sv_right : &mut FpVector, 
        sv_out : &mut FpVector
    ) -> Result<(), ()> {
        for (i, &(gen_x, gen_y, gen_idx)) in new_indecs.iter().enumerate() {
            let mut new_decompositions = Vec::new();
            for (dec_x, dec_y, dec_vec, mono) in &prev_decompositions {
                let dec_x = *dec_x;
                let dec_y = *dec_y;
                let out_x = gen_x + dec_x;
                let out_y = gen_y + dec_y;
                let mut result = FpVector::new(self.prime(), self.dimension(out_x, out_y));
                self.multiply_element_by_basis_element(
                    &mut result, 1,
                    dec_x, dec_y, &dec_vec, 
                    gen_x, gen_y, gen_idx,
                    sv_left, sv_right, sv_out
                )?;
                if result.is_zero(){
                    continue;
                }
                let result_mono = mono.multiply(&Monomial::indecomposable(gen_x, gen_y, gen_idx));
                self.data[out_x][out_y].write().unwrap().indecomposable_decompositions
                    .push((result.clone(), result_mono.clone()));
                new_decompositions.push((out_x, out_y, result, result_mono));
            }
            if !new_decompositions.is_empty() {
                self.update_indecomposable_decompositions_helper(
                    &new_indecs[i..], new_decompositions, 
                    sv_left, sv_right, sv_out
                )?;
            }
        }
        Ok(())
    }   
}

#[cfg(test)]
mod tests {
    use super::*;
    // use rand::Rng;
    // use rstest::rstest;
    
    #[test]
    fn test_dba(){
        let p = 2;
        let p_ = ValidPrime::new(p);
        // let A = DenseBigradedAlgebra::from_dimension_array(p, 
        //     BiVec::from_vec(0, 
        //         vec![


        //         ]
        //     ));
        // A.
    }
}