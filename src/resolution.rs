#![allow(unused_imports)]

use std::cmp::max;

use crate::fp_vector::FpVector;
use crate::matrix::{Matrix, Subspace};
use crate::algebra::Algebra;
use crate::module::{Module, ZeroModule};
use crate::free_module::FreeModule;
use crate::module_homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::ChainComplex;

pub struct ResolutionModules<'a> {
    complex : &'a ChainComplex,
    modules : Vec<FreeModule<'a>>,
    zero_module : ZeroModule<'a>,
}

pub struct ResolutionHomomorphisms<'b> {
    differentials : Vec<FreeModuleHomomorphism<'b, 'b>>,
    chain_maps : Vec<FreeModuleHomomorphism<'b, 'b>>,
}

rental! {
    pub mod rent_res {
        use super::*;
        #[rental]
        pub struct ResolutionInner<'a> {
            modules : Box<ResolutionModules<'a>>,
            homomorphisms : ResolutionHomomorphisms<'modules>
        }
    }
}


pub struct Resolution<'a> {
    res_inner : rent_res::ResolutionInner<'a>,
    max_degree : i32,
    add_class : Option<Box<Fn(u32, i32, &str)>>,
    add_structline : Option<Box<Fn(
        &str,
        u32, i32, usize, 
        u32, i32, usize
    )>>
}

impl<'a> Resolution<'a> {  
    pub fn new(
        complex : &'a ChainComplex, max_degree : i32,
        add_class : Option<Box<Fn(u32, i32, &str)>>,
        add_structline : Option<Box<Fn(
            &str,
            u32, i32, usize, 
            u32, i32, usize
        )>>
    ) -> Self {
        let algebra = complex.get_algebra();
        let zero_module = ZeroModule::new(algebra);
        let min_degree = complex.get_min_degree();
        assert!(max_degree >= min_degree);
        let num_degrees = (max_degree - min_degree) as usize;
        let mut modules = Vec::with_capacity(num_degrees);
        for i in 0..num_degrees {
            modules.push(FreeModule::new(algebra, format!("F{}", i), min_degree, max_degree));
        }

        let res_modules = ResolutionModules {
            complex,
            modules,
            zero_module
        };

        let res_modules_box = Box::new(res_modules);
        
        let res_inner = rent_res::ResolutionInner::new(
            res_modules_box,
            |res_modules| {
                let mut differentials = Vec::with_capacity(num_degrees);
                let mut chain_maps = Vec::with_capacity(num_degrees);                
                for i in 0..num_degrees {
                    let complex_module;
                    unsafe {
                        complex_module = std::mem::transmute::<_,&'static Module>(complex.get_module(i as u32));
                    }
                    chain_maps.push(FreeModuleHomomorphism::new(&res_modules.modules[i], complex_module, min_degree, 0, max_degree));
                }
                differentials.push(FreeModuleHomomorphism::new(&res_modules.modules[0], &res_modules.zero_module, min_degree, 0, max_degree));                
                for i in 1..num_degrees {
                    differentials.push(FreeModuleHomomorphism::new(&res_modules.modules[i], &res_modules.modules[i-1], min_degree, 0, max_degree));
                }
                ResolutionHomomorphisms {
                    differentials,
                    chain_maps
                }
            }
        );
        Self {
            res_inner,
            max_degree,
            add_class,
            add_structline,
        }
    }

    pub fn get_max_degree(&self) -> i32 {
        self.max_degree
    }

    pub fn get_max_hom_deg(&self) -> u32 {
        (self.get_max_degree() - self.get_min_degree()) as u32
    }
    
    pub fn get_complex(&self) -> &ChainComplex {
        self.res_inner.head().complex
    }

    pub fn get_module(&self, homological_degree : u32) -> &FreeModule {
        &self.res_inner.head().modules[homological_degree as usize]
    }

    fn get_differential<'b>(&'b self, homological_degree : u32) -> &'b FreeModuleHomomorphism {
        self.res_inner.rent(|res_homs| {
            let result = &res_homs.differentials[homological_degree as usize];
            unsafe {
                std::mem::transmute::<_, &'b FreeModuleHomomorphism<'b, 'b>>(result)
            }
        })
    }

    fn get_chain_map<'b>(&'b self, homological_degree : u32) -> &'b FreeModuleHomomorphism {
        self.res_inner.rent(|res_homs| {
            let result = &res_homs.chain_maps[homological_degree as usize];
            unsafe {
                std::mem::transmute::<_, &'b FreeModuleHomomorphism<'b, 'b>>(result)
            }
        }) 
    }

    pub fn get_cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        let p = self.get_prime();
        let d = self.get_differential(hom_deg);
        let source = self.get_module(hom_deg);
        let target = d.get_target();
        let dimension = target.get_dimension(int_deg);
        let basis_idx = source.operation_generator_to_index(0, 0, int_deg, idx);
        let mut result_vector = crate::fp_vector::FpVector::new(p, dimension, 0);
        d.apply_to_basis_element(&mut result_vector, 1, int_deg, basis_idx);
        return target.element_to_string(int_deg, &result_vector);
    }

    pub fn resolve_through_degree(&self, degree : i32){
        let min_degree = self.get_min_degree();
        let max_hom_deg = self.get_max_hom_deg();
        for int_deg in min_degree .. degree {
            for hom_deg in 0 .. max_hom_deg { // int_deg as u32 + 1 {
                // println!("(hom_deg : {}, int_deg : {})", hom_deg, int_deg);
                self.step(hom_deg, int_deg);
            }
        }
    }

    pub fn step(&self, homological_degree : u32, degree : i32){
        // if homological_degree == 0 {
        //     let dminus1 = self.get_differential(0);
        //     let module = self.get_complex().get_module(0);
        //     let module_dim = module.get_dimension(degree);
        //     let subspace = Subspace::entire_space(self.get_prime(), module_dim);
        //     dminus1.set_kernel(degree, subspace);
        // }
        
        self.get_complex().compute_through_bidegree(homological_degree, degree);
        self.generate_old_kernel_and_compute_new_kernel(homological_degree, degree);
        let module = self.get_module(homological_degree);
        let num_gens = module.get_number_of_gens_in_degree(degree);
        if let Some(f) = &self.add_class {
            for i in 0..num_gens {
                f(homological_degree, degree, &format!("{}", i));
            }
        }
        if let Some(_) = &self.add_structline {
            for i in 0..num_gens {
                self.compute_filtration_one_products(homological_degree, degree, i);
            }
        }
    }

    fn compute_filtration_one_products(&self, homological_degree : u32, degree : i32, source_idx : usize){
        if homological_degree == 0 {
            return;
        }
        if let Some(add_structline) = &self.add_structline {
            let d = self.get_differential(homological_degree);
            let target = self.get_module(homological_degree - 1);
            let dx = d.get_output(degree, source_idx);
            for (op_name, op_degree, op_index) in self.get_algebra().get_filtration_one_products() {
                let gen_degree = degree - op_degree;

                if gen_degree < self.get_min_degree(){
                    break;
                }

                let num_target_generators = target.get_number_of_gens_in_degree(gen_degree);
                for target_idx in 0 .. num_target_generators {
                    let vector_idx = target.operation_generator_to_index(op_degree, op_index, gen_degree, target_idx);
                    if vector_idx >= dx.get_dimension() {
                        // println!("Out of bounds index when computing product:");
                        // println!("  ==  degree: {}, hom_deg: {}, dim: {}, idx: {}", degree, homological_degree, dx.dimension, vector_idx);
                    } else {
                        // printf("hom_deg: %d, deg: %d, source_idx: %d, op_deg: %d, entry: %d\n", homological_degree, degree, source_idx, op_degree, Vector_getEntry(dx, vector_idx));
                        if dx.get_entry(vector_idx) != 0 {
                            // There was a product!
                            add_structline(op_name, homological_degree - 1, gen_degree, target_idx, homological_degree, degree, source_idx);
                        }
                    }
                }
            }
        }
    }    

    // pub fn set_empty(&self, homological_degree : u32, degree : i32){
    //     let current_differential = self.get_differential(homological_degree);
    //     let source = current_differential.source;
    //     let source_module_table = source.construct_table(degree);
    // }

    pub fn generate_old_kernel_and_compute_new_kernel(&self, homological_degree : u32, degree : i32){
        let min_degree = self.get_min_degree();
        // println!("====hom_deg : {}, int_deg : {}", homological_degree, degree);
        let degree_idx = (degree - min_degree) as usize;
        let p = self.get_prime();
        let current_differential = self.get_differential(homological_degree);
        let current_chain_map = self.get_chain_map(homological_degree);
        let source = current_differential.source;
        let target_cc = current_chain_map.target;
        let target_res = current_differential.target;
        let source_module_table = source.construct_table(degree);
        let source_dimension = source.get_dimension_with_table(degree, &source_module_table);
        let target_cc_dimension = target_cc.get_dimension(degree);
        let target_res_dimension = target_res.get_dimension(degree);
        let target_dimension = target_cc_dimension + target_res_dimension;
        // The Homomorphism matrix has size source_dimension x target_dimension, but we are going to augment it with an
        // identity matrix so that gives a matrix with dimensions source_dimension x (target_dimension + source_dimension).
        // Later we're going to write into this same matrix an isomorphism source/image + new vectors --> kernel
        // This has size target_dimension x (2*target_dimension).
        // This latter matrix may be used to find a preimage of an element under the differential.

        // Pad the target dimension so that it ends in an aligned position.
        let padded_target_cc_dimension = FpVector::get_padded_dimension(p, target_cc_dimension, 0);
        let padded_target_res_dimension = FpVector::get_padded_dimension(p, target_res_dimension, 0);
        let padded_target_dimension = padded_target_res_dimension + padded_target_cc_dimension;
        let rows = max(source_dimension, target_dimension);
        let columns = padded_target_dimension + source_dimension + rows;
        let mut matrix = Matrix::new(p, rows, columns);
        matrix.set_slice(0, source_dimension, 0, padded_target_dimension + source_dimension);
        current_chain_map.get_matrix_with_table(&mut matrix, &source_module_table, degree, 0, 0);
        current_differential.get_matrix_with_table(&mut matrix, &source_module_table, degree, 0, padded_target_cc_dimension);
        for i in 0 .. source_dimension {
            matrix[i].set_entry(padded_target_dimension + i, 1);
        }
        // println!("{}", matrix);
        // println!("     rows: {}, cols: {}", matrix.get_rows(), matrix.get_columns());

        let mut pivots = vec![-1;matrix.get_columns()];
        matrix.row_reduce(&mut pivots);

        let kernel = matrix.compute_kernel(&pivots, padded_target_dimension);
        let kernel_rows = kernel.matrix.get_rows();
        current_differential.set_kernel(degree, kernel);

        matrix.clear_slice();
        // Now add generators to hit kernel of previous differential. 
        let prev_res_cycles;
        let prev_cc_cycles;
        if homological_degree > 0 {
            prev_cc_cycles = self.get_complex().get_differential(homological_degree - 1).get_kernel(degree);
            prev_res_cycles = self.get_differential(homological_degree - 1).get_kernel(degree);
        } else {
            prev_cc_cycles = None;
            prev_res_cycles = None;
        }
        let first_new_row = source_dimension - kernel_rows;
        
        let cur_cc_image = self.get_complex().get_differential(homological_degree).get_image(degree)
                                .map(|subspace| &subspace.column_to_pivot_row);
        // We stored the kernel rows somewhere else so we're going to write over them.
        // Add new free module generators to hit basis for previous kernel
        let mut new_generators = matrix.extend_image(first_new_row, 0, target_cc_dimension, &pivots, prev_cc_cycles, cur_cc_image);
        new_generators += matrix.extend_image(first_new_row, padded_target_cc_dimension, padded_target_cc_dimension + target_res_dimension, &pivots, prev_res_cycles, None);
        source.add_generators(degree, source_module_table, new_generators);
        current_chain_map.add_generators_from_matrix_rows(degree, &mut matrix, first_new_row, 0, new_generators);
        current_differential.add_generators_from_matrix_rows(degree, &mut matrix, first_new_row, padded_target_cc_dimension, new_generators);
        
        // println!("small matrix?");
        // println!("{}", matrix);
        // The part of the matrix that contains interesting information is occupied_rows x (target_dimension + source_dimension + kernel_size).
        // Allocate a matrix coimage_to_image with these dimensions.
        let image_rows = first_new_row + new_generators;

        let mut new_pivots = vec![-1;matrix.get_columns()];
        matrix.row_reduce(&mut new_pivots);

        // let quasi_inverse = matrix.compute_quasi_inverse(&pivots, vec![padded_target_cc_dimension, padded_target_dimension]);
        
    }

    pub fn graded_dimension_string(&self) -> String {
        let mut result = String::new();
        let min_degree = self.get_min_degree();
        let max_degree = self.get_max_degree();
        let max_hom_deg = self.get_max_hom_deg();
        result.push_str("[\n");
        for i in (0 .. max_hom_deg).rev() {
            result.push_str("[");
            let module = self.get_module(i);
            for j in min_degree + i as i32 .. max_degree {
                result.push_str(&format!("{}, ", module.get_number_of_gens_in_degree(j)));
            }
            result.push_str("]\n");
        }
        result.push_str("\n]\n");
        return result;
    }

}


impl<'a> ChainComplex for Resolution<'a> {
    fn get_algebra(&self) -> &Algebra {
        self.get_complex().get_algebra()
    }

    fn get_module(&self, homological_degree : u32) -> &Module {
        self.get_module(homological_degree)
    }

    fn get_min_degree(&self) -> i32 {
        self.get_complex().get_min_degree()
    }

    fn get_differential<'b>(&'b self, homological_degree : u32) -> &'b ModuleHomomorphism {
        self.get_differential(homological_degree)
    }

    // TODO: implement this.
    fn compute_through_bidegree(&self, hom_deg : u32, int_deg : i32) {

    }

    // fn computed_through_bidegree_q(&self, hom_deg : u32, int_deg : i32) -> bool {
    //     self.res_inner.rent(|res_homs| {
    //         res_homs.differentials.len() > hom_deg 
    //             && res_homs.differentials[hom_deg as usize].
    //     })
    // }



    

    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> QuasiInverse {
    //     let qi_pivots = self.image_deg_zero[degree].get();
    //     QuasiInverse {
            
    //     }
    // }
}