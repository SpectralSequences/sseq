
// #![allow(unused_variables)]
// use std::sync::Arc;

// use std::collections::HashMap;


// use once::OnceVec;
// use fp::prime::ValidPrime;
// use fp::vector::{FpVector, FpVectorT};

// use crate::algebra::combinatorics::TruncatedPolynomialPartitions;
// use crate::algebra::{Algebra, AdemAlgebraT};
// use crate::module::Module;
// // use bivec::BiVec;

// #[derive(Clone, Eq, PartialEq)]
// pub struct UnstableAlgebraMonomial {
//     degree : i32,
//     poly : FpVector,
//     ext : FpVector
// }

// impl UnstableAlgebraMonomial {
//     pub fn new(p : ValidPrime) -> Self {
//         Self {
//             degree : 0xFEDCBA9, // Looks invalid to me!
//             poly : FpVector::new(p, 0),
//             ext : FpVector::new(ValidPrime::new(2), 0)
//         }
//     }
// }

// pub struct UnstableAlgebraTableEntry {    
//     index_to_monomial : Vec<UnstableAlgebraMonomial>, // degree -> index -> AdemBasisElement
//     monomial_to_index : HashMap<UnstableAlgebraMonomial, usize>, // degree -> AdemBasisElement -> index
// }

// impl UnstableAlgebraTableEntry {
//     pub fn new() -> Self {
//         Self {
//             index_to_monomial : Vec::new(),
//             monomial_to_index : HashMap::new()
//         }
//     }
// }

// impl std::hash::Hash for UnstableAlgebraMonomial {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.poly.hash(state);
//         self.ext.hash(state);
//     }
// }

// pub trait UnstableAlgebra {
//     type Algebra : AdemAlgebraT;

//     fn algebra_inner(&self) -> Arc<Self::Algebra>;
//     fn name_inner(&self) -> String;
//     fn polynomial_partitions(&self) -> &TruncatedPolynomialPartitions;
//     fn exterior_partitions(&self) -> &TruncatedPolynomialPartitions;
    
//     fn min_degree(&self) -> i32 { 0 }

//     fn polynomial_generators_in_degree(&self, degree : i32) -> usize;
//     fn exterior_generators_in_degree(&self, degree : i32) -> usize;

//     fn basis_table(&self) -> &OnceVec<UnstableAlgebraTableEntry>;
//     fn frobenius_on_generator(&self, degree : i32, index : usize) -> Option<usize>; 
//     fn compute_generating_set(&self, degree : i32);
    
//     fn prime(&self) -> ValidPrime {
//         self.algebra_inner().prime()
//     }
    
//     fn monomial_to_index(&self, monomial : &UnstableAlgebraMonomial) -> Option<usize> {
//         self.basis_table()[monomial.degree as usize].monomial_to_index.get(monomial).map(|x| *x)
//     }
    
//     fn index_to_monomial(&self, degree : i32, index : usize) -> &UnstableAlgebraMonomial {
//         &self.basis_table()[degree as usize].index_to_monomial[index]
//     }

//     fn frobenius_monomial(&self, target : &mut FpVector, source : &FpVector) {
//         let p = *self.prime() as i32;
//         for (i, c) in source.iter_nonzero() {
//             let (degree, in_idx) = self.polynomial_partitions().internal_idx_to_gen_deg(i);
//             let frob = self.frobenius_on_generator(degree, in_idx);
//             if let Some(e) = frob {
//                 let out_idx = self.polynomial_partitions().gen_deg_idx_to_internal_idx(p*degree, e);
//                 target.add_basis_element(out_idx, c);
//             }
//         }
//     }

//     fn nonzero_squares_on_polynomial_generator(&self, gen_degree : i32, gen_idx : usize) -> &Vec<i32>;
//     fn nonzero_squares_on_exterior_generator(&self, gen_degree : i32, gen_idx : usize) -> &Vec<i32>;

//     fn generator_to_monomial_sq(&self, result : &mut UnstableAlgebraMonomial, sq : i32, input_degree : i32, input_idx : usize);

//     fn generator_to_polynomial_sq(&self, result : &mut FpVector, coeff : u32, sq : i32, input_degree : i32, input_idx : usize) {
//         let mut temp_monomial = UnstableAlgebraMonomial::new(self.prime());
//         temp_monomial.degree = sq + input_degree;
//         temp_monomial.poly.extend_dimension(self.polynomial_partitions().generators_up_to_degree(sq + input_degree));
//         temp_monomial.ext.extend_dimension(self.exterior_partitions().generators_up_to_degree(sq + input_degree));
//         self.generator_to_monomial_sq(&mut temp_monomial, sq, input_degree, input_idx);
//         result.add_basis_element(self.monomial_to_index(&temp_monomial).unwrap(), coeff);
//     }
    
//     fn monomial_to_polynomial_sq(&self,
//         result : &mut FpVector, c : u32,  sq : i32, input : &UnstableAlgebraMonomial
//     ) {
//         let ext_monos = self.exterior_partitions();
//         let poly_monos = self.polynomial_partitions();
//         let mut ext_sqs = Vec::new();
//         let mut poly_sqs = Vec::new();
//         for i in 0 .. input.ext_part.dimension() {
//             if input.ext_part.entry(i) == 0 {
//                 ext_sqs.push(&vec![]);
//             } else {
//                 let (deg, idx) = ext_monos.internal_idx_to_gen_deg(i);
//                 ext_sqs.push(self.nonzero_squares_on_exterior_generator(deg, idx));
//             }
//         }
//         for i in 0 .. input.poly_part.dimension() {
//             if input.poly_part.entry(i) == 0 {
//                 poly_sqs.push(&vec![]);
//             } else {
//                 let (deg, idx) = poly_monos.internal_idx_to_gen_deg(i);
//                 poly_sqs.push(self.nonzero_squares_on_polynomial_generator(deg, idx));
//             }
//         }


//         // let temp_monomial = UnstableAlgebraMonomial::new(self.prime());
//         let mut temp_poly_accumulate = Vec::new();
//         let mut temp_poly_source = Vec::new();
//         for i in 0 ..= sq {
//             let ext_deg = i;
//             let poly_deg = sq - i;
            
//             let ext_squares = TruncatedPolynomialSteenrodPartitionIterator(ext_deg, ext_sqs,  input);
//             let ext_polynomials = Vec::new();
//             for ext_sq in ext_squares.iter().enumerate() {
//                 for i in 0 .. ext_sq.len() {
//                     let (degree, idx) = poly_monos.internal_idx_to_gen_deg(i);
//                     temp_poly_source.set_to_zero_pure();
//                     ext_polynomials.push(
//                         self.generator_to_polynomial_sq(&mut temp_poly_source, 1, ext_sqi, degree, idx)
//                     );
//                 }
//             }

//             let poly_squares = TruncatedPolynomialSteenrodPartitionIterator(poly_deg, poly_sqs,  input);
//             let poly_polynomials = Vec::new();
//             for poly_sq in poly_squares {
//                 for i in 0 .. poly_sq.len() {
//                     let (degree, idx) = poly_monos.internal_idx_to_gen_deg(i);
//                     for j in 0..poly_sq[i].len() {
//                         temp_poly_source.set_to_zero_pure();
//                         self.generator_to_polynomial_sq(&mut temp_poly_source, 1, ext_part[i][j], degree, idx);
//                         self.multiply_polynomials(&mut temp_poly_accumulate, 1, &temp_poly_source)
//                     }
//                 }
//             }
//         }
        
//     }

//     fn multiply_monomials(&self, target : &mut UnstableAlgebraMonomial, source : &UnstableAlgebraMonomial) -> Option<()> {
//         target.degree += source.degree;
//         target.ext.extend_dimension(source.ext.dimension());
//         target.ext.add_truncate(&source.ext, 1)?;

//         let mut carry_vec = FpVector::new(self.prime(), 0);
//         let mut source_vec = source.poly.clone();
//         let mut carry_q = true;
//         while carry_q {
//             target.poly.extend_dimension(source.poly.dimension());
//             carry_vec.extend_dimension(target.poly.dimension());
//             carry_q = target.poly.add_carry(&source.poly, 1, &mut [carry_vec]);
//             if carry_q {
//                 source_vec.set_to_zero_pure();
//                 self.frobenius_monomial(&mut source_vec, &carry_vec);
//             }
//         }
//         Some(())
//     }

//     fn multiply_polynomials(&self, target : &mut FpVector, coeff : i32, left_degree : i32, left : &FpVector, right_degree : i32, right : &FpVector) {
//         target.extend_dimension(self.dimension(left_degree + right_degree));
//         for (i, c) in left.iter_nonzero() {
//             for (j, d) in right.iter_nonzero() {
//                 let mut target_mono = self.index_to_monomial(left_degree, left).clone();
//                 let source_mono = self.index_to_monomial(right_degree, right);
//                 self.multiply_monomials(&mut target_mono,  &source_mono);
//                 let idx = self.monomial_to_index(target_mono);
//                 target.add_basis_element(idx, (c*d*coeff)%p);
//             }
//         }
//     }
// }


// impl<Adem : AdemAlgebraT, A: UnstableAlgebra<Algebra=Adem> + Send + Sync + 'static> Module for A {
//     type Algebra = Adem;
//     fn algebra(&self) -> Arc<Self::Algebra> {
//         self.algebra_inner()
//     }

//     fn name(&self) -> String {
//         self.name_inner()
//     }

//     fn min_degree(&self) -> i32 { 0 }

//     fn max_computed_degree(&self) -> i32 {
//         self.basis_table().len() as i32 - 1
//     }


//     fn dimension(&self, degree: i32) -> usize {
//         self.basis_table()[degree as usize].index_to_monomial.len()
//     }


//     fn compute_basis(&self, degree : i32) {
//         self.compute_generating_set(degree);
//         for i in self.max_computed_degree() + 1 ..= degree {
//             let num_poly_gens = self.polynomial_generators_in_degree(i);
//             let num_ext_gens = self.exterior_generators_in_degree(i);
//             let poly_parts = self.polynomial_partitions();
//             let ext_parts = self.exterior_partitions();
//             poly_parts.add_gens_and_calculate_parts(degree, num_poly_gens);
//             ext_parts.add_gens_and_calculate_parts(degree, num_ext_gens);
//             let mut table = UnstableAlgebraTableEntry::new();
//             for poly_deg in 0 ..= degree {
//                 let ext_deg = degree - poly_deg;
//                 for p in poly_parts.parts(poly_deg) {
//                     for e in ext_parts.parts(ext_deg) {
//                         let index = table.index_to_monomial.len();
//                         let m = UnstableAlgebraMonomial {
//                             degree,
//                             poly : p.clone(),
//                             ext : e.clone()
//                         };
//                         table.monomial_to_index.insert(m.clone(), index);
//                         table.index_to_monomial.push(m);
//                     }
//                 }
//             }
//             self.basis_table().push(table);
//         }
//     }

//     fn basis_element_to_string(&self, _degree: i32, _index: usize) -> String {
//         "".to_string()
//     }


//     fn act_on_basis(&self,
//         result : &mut FpVector, c : u32,  op_deg : i32, op_index : usize, input_degree : i32, input_index : usize
//     ) {
//         // let mut temp_monomial = UnstableAlgebraMonomial::temp(self.prime());
//         let mono = self.index_to_monomial(input_degree, input_index);
//         self.monomial_to_polynomial_action(result, c, op_deg, op_index, mono);
//     }

//     fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
//         let _scratch_summand = UnstableAlgebraMonomial::new(self.prime());
//         let _scratch_carry = UnstableAlgebraMonomial::new(self.prime());
//         let p = self.prime();
//         for (i, v) in input.iter_nonzero() {
//             self.act_on_basis(
//                 result,
//                 (coeff * v) % *p,
//                 op_degree,
//                 op_index,
//                 input_degree,
//                 i,
//             );
//         }
//     }

//     fn act_by_element(
//         &self,
//         result: &mut FpVector,
//         coeff: u32,
//         op_degree: i32,
//         op: &FpVector,
//         input_degree: i32,
//         input: &FpVector,
//     ) {
//         assert_eq!(input.dimension(), self.dimension(input_degree));
//         let p = self.prime();
//         for (i, v) in op.iter_nonzero() {
//             self.act(result, (coeff * v) % *p, op_degree, i, input_degree, input);
//         }
//     }


// }