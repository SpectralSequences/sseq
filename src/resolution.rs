#![allow(unused_imports)]

use std::cmp::max;

use crate::fp_vector::FpVector;
use crate::matrix::Matrix;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::ChainComplex;


pub struct Resolution<'a,'b,'c, 'd, 'e> {
    complex : &'a ChainComplex,
    modules : Vec<FreeModule<'b>>,
    differentials : Vec<FreeModuleHomomorphism<'c, 'd, 'e>>,
    chain_maps : Vec<FreeModuleHomomorphism<'c, 'd, 'e>>,
    add_class : Option<fn(hom_deg : usize, int_deg : i32, name : &str)>,
    add_structline : Option<fn(
        sl_type : &str,
        source_hom_deg : usize, source_int_deg : i32, source_idx : usize, 
        target_hom_deg : usize, target_int_deg : i32, target_idx : usize
    )>
}

impl<'a,'b,'c, 'd, 'e> Resolution<'a,'b,'c, 'd, 'e> {
    pub fn new(complex : &'a ChainComplex, max_degree : usize) -> Self {
        let modules = Vec::new();
        let differentials = Vec::new();
        let chain_maps = Vec::new();
        Self {
            complex,
            modules,
            differentials,
            chain_maps,
            add_class : None,
            add_structline : None
        }
    }
    
    pub fn get_prime(&self) -> u32 {
        self.complex.get_prime()
    }

    pub fn get_min_degree(&self) -> i32 {
        self.complex.get_min_degree()
    }

    // pub fn generate_old_kernel_and_compute_new_kernel(&mut self, homological_degree : u32, degree : i32){
    //     let min_degree = self.get_min_degree();
    //     assert!(degree >= homological_degree as i32 + min_degree);
    //     let homological_degree = homological_degree as usize;
    //     let degree_idx = (degree - min_degree) as usize;
    //     let p = self.get_prime();
    //     let current_differential  = self.differentials[homological_degree];
    //     let source = current_differential.source;
    //     let target = current_differential.target;
    //     let source_dimension = source.get_dimension(degree);
    //     let target_dimension = target.get_dimension(degree);
    //     // The Homomorphism matrix has size source_dimension x target_dimension, but we are going to augment it with an
    //     // identity matrix so that gives a matrix with dimensions source_dimension x (target_dimension + source_dimension).
    //     // Later we're going to write into this same matrix an isomorphism source/image + new vectors --> kernel
    //     // This has size target_dimension x (2*target_dimension).
    //     // This latter matrix may be used to find a preimage of an element under the differential.

    //     // Pad the target dimension so that it ends in an aligned position.
    //     let first_source_index = FpVector::get_padded_dimension(p, target_dimension, 0);
    //     let rows = max(source_dimension, target_dimension);
    //     let columns = first_source_index + source_dimension + rows;
    //     let matrix = Matrix::new(p, rows, columns);

    //     let source_module_table = source.construct_table(degree);
    //     current_differential.get_matrix_with_table(&mut matrix, &source_module_table, degree);

    //     let pivots = vec![-1; matrix.columns];
    //     // matrix.compute_kernel(padded_target_dimension, pivots);
    //     let kernel = &current_differential.kernel[degree_idx];

    //     // Now add generators to hit kernel of previous differential. 
    //     let previous_cycles = &self.differentials[homological_degree - 1].kernel[degree_idx];
    //     let first_new_row = source_dimension - kernel.matrix.rows;
    //     // We stored the kernel rows somewhere else so we're going to write over them.
    //     // Add new free module generators to hit basis for previous kernel
    //     let new_generators = matrix.extend_image(first_new_row, pivots, previous_cycles, None);
    //     current_differential.source.add_generators(degree, source_module_table, new_generators);
    //     current_differential.add_generators_from_matrix_rows(degree, &mut matrix, first_new_row, new_generators);

    //     // The part of the matrix that contains interesting information is occupied_rows x (target_dimension + source_dimension + kernel_size).
    //     // Allocate a matrix coimage_to_image with these dimensions.
        // let image_rows = first_new_row + new_generators;

        // let new_pivots = vec![-1; matrix.columns];
        // matrix.row_reduce(&mut new_pivots);
        // let image = matrix.get_image(image_rows, target_dimension, new_pivots);
    // }
}
