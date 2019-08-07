use std::rc::Rc;
use serde_json::Value;

use crate::once::{OnceVec, TempStorage};
use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace};
use crate::algebra::Algebra;
use crate::module::Module;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::chain_complex::ChainComplex;
use crate::resolution::Resolution;
use crate::resolution_homomorphism::ResolutionHomomorphism;

struct Cocycle {
    homological_degree : u32,
    internal_degree : i32,
    index : usize,
    name : String
}

struct SelfMap<
    M1 : Module, F1 : ModuleHomomorphism<M1, M1>, CC1 : ChainComplex<M1, F1>,
    M2 : Module, F2 : ModuleHomomorphism<M2, M2>, CC2 : ChainComplex<M2, F2>
> {
    homological_degree : u32,
    internal_degree : i32,
    name : String,
    map_data : TempStorage<Matrix>,
    map : ResolutionHomomorphism<M1, F1, CC1, M2, F2, CC2>
}

pub struct ResolutionWithChainMaps<
    M1 : Module, F1 : ModuleHomomorphism<M1, M1>, CC1 : ChainComplex<M1, F1>,
    M2 : Module, F2 : ModuleHomomorphism<M2, M2>, CC2 : ChainComplex<M2, F2>
> {
    pub resolution : Rc<Resolution<M1, F1, CC1>>,
    unit_resolution : Rc<Resolution<M2, F2, CC2>>,
    max_product_homological_degree : u32,
    product_list : Vec<Cocycle>,
    chain_maps_to_trivial_module : OnceVec<OnceVec<OnceVec<ResolutionHomomorphism<M1, F1, CC1, M2, F2, CC2>>>>,
    self_maps : Vec<SelfMap<M1, F1, CC1, M1, F1, CC1>>
}

use wasm_bindgen::prelude::*;
use web_sys::console;

impl<
    M1 : Module, F1 : ModuleHomomorphism<M1, M1>, CC1 : ChainComplex<M1, F1>,
    M2 : Module, F2 : ModuleHomomorphism<M2, M2>, CC2 : ChainComplex<M2, F2>
>
ResolutionWithChainMaps<M1, F1, CC1, M2, F2, CC2> {
    pub fn new(resolution : Rc<Resolution<M1, F1, CC1>>, unit_resolution : Rc<Resolution<M2, F2, CC2>>) -> Self {
        Self {
            resolution,
            unit_resolution,
            max_product_homological_degree : 0,
            product_list : Vec::new(),
            chain_maps_to_trivial_module : OnceVec::new(),
            self_maps : Vec::new()
        }
    }

    pub fn get_prime(&self) -> u32 {
        self.resolution.get_prime()
    }

    pub fn get_algebra(&self) -> Rc<dyn Algebra> {
        self.resolution.get_algebra()
    }

    pub fn get_min_degree(&self) -> i32 {
        self.resolution.get_min_degree()
    }

    pub fn resolve_through_degree(&self, degree : i32){
        self.get_algebra().compute_basis(degree);
        let min_degree = self.get_min_degree();
        let max_hom_deg = degree as u32; //self.get_max_hom_deg();
        for int_deg in min_degree .. degree {
            let mut new_kernel = None;
            for hom_deg in 0 .. max_hom_deg {
                // println!("(hom_deg : {}, int_deg : {})", hom_deg, int_deg);
                new_kernel = Some(self.step(hom_deg, int_deg, new_kernel));
            }
        }
    }

    pub fn step(&self, homological_degree : u32, internal_degree : i32, old_kernel : Option<Subspace>) -> Subspace {
        let new_kernel = self.resolution.step(homological_degree, internal_degree, old_kernel);
        self.compute_products(homological_degree, internal_degree);
        self.compute_self_maps(homological_degree, internal_degree);  
        return new_kernel;
    }

    pub fn add_product(&mut self, homological_degree : u32, internal_degree : i32, index : usize, name : String) {
        if homological_degree > self.max_product_homological_degree {
            self.max_product_homological_degree = homological_degree;
        }
        self.product_list.push(Cocycle {
            homological_degree,
            internal_degree,
            index,
            name
        });
    }

    pub fn extend_maps(&self, homological_degree : u32, internal_degree : i32) {
        if self.max_product_homological_degree == 0 {
            return;
        }
        let p = self.get_prime();        
        let hom_deg_idx = homological_degree as usize;
        let int_deg_idx = (internal_degree - self.get_min_degree()) as usize;
        let max_hom_deg = std::cmp::min(homological_degree, self.max_product_homological_degree);
        let num_gens = self.resolution.get_module(homological_degree).get_number_of_gens_in_degree(internal_degree);
        if int_deg_idx == 0 {
            assert!(hom_deg_idx == self.chain_maps_to_trivial_module.len());
            self.chain_maps_to_trivial_module.push(OnceVec::new());
        } else {
            assert!(hom_deg_idx < self.chain_maps_to_trivial_module.len());
        }
        assert!(self.chain_maps_to_trivial_module[hom_deg_idx].len() == int_deg_idx);
        self.chain_maps_to_trivial_module[hom_deg_idx].push(OnceVec::new());
        if num_gens > 0 {
            let mut unit_vector = Matrix::new(p, num_gens, 1);
            for j in 0 .. num_gens {
                let f = ResolutionHomomorphism::new(
                    format!("(hom_deg : {}, int_deg : {}, idx : {})", homological_degree, internal_degree, j),
                    Rc::clone(&self.resolution), Rc::clone(&self.unit_resolution), 
                    homological_degree, internal_degree
                );
                unit_vector[j].set_entry(0, 1);
                f.extend_step(homological_degree, internal_degree, Some(&mut unit_vector));
                unit_vector[j].set_to_zero();
                self.chain_maps_to_trivial_module[hom_deg_idx][int_deg_idx].push(
                    f
                )
            }
        }

        let min_degree = self.get_min_degree();
        for i in 0 ..= max_hom_deg {
            for j in min_degree ..= internal_degree {
                let j_idx = (j - min_degree) as usize;
                let hom_deg = homological_degree - i;
                let num_gens = self.resolution.get_module(hom_deg).get_number_of_gens_in_degree(j);
                for k in 0 .. num_gens {
                    // printf("      cocyc (%d, %d, %d) to (%d, %d) \n", hom_deg, j, k,  i, internal_degree);
                    // println!("hom_def : {}, j : {}, k : {}", hom_deg, j, k);
                    let f = &self.chain_maps_to_trivial_module[hom_deg as usize][j_idx][k];
                    f.extend(homological_degree, internal_degree);
                }
            }
        }
    }

    pub fn compute_products(&self, homological_degree : u32, internal_degree : i32) {
        let res = &self.resolution;
        self.extend_maps(homological_degree, internal_degree);
        for elt in &self.product_list {
            if homological_degree < elt.homological_degree || internal_degree < elt.internal_degree {
                continue;
            }
            let source_homological_degree = homological_degree - elt.homological_degree;
            let source_degree = internal_degree - elt.internal_degree;
            for k in 0 .. res.get_number_of_gens_in_bidegree(source_homological_degree, source_degree) {
                self.compute_product( 
                    elt.homological_degree, elt.internal_degree, elt.index, &elt.name,
                    source_homological_degree, source_degree, k
                );
            }
        }
    }

    pub fn compute_product(
        &self, 
        elt_hom_deg : u32, elt_deg : i32, elt_idx : usize, elt_name : &str,
        source_hom_deg : u32, source_deg : i32, source_idx : usize
    ) {
        let p = self.get_prime();
        let source_hom_deg_idx = source_hom_deg as usize;
        let source_deg_idx = source_deg as usize;
        let res = &self.resolution;
        let f = &self.chain_maps_to_trivial_module[source_hom_deg_idx][source_deg_idx][source_idx];
        let target_hom_deg = source_hom_deg + elt_hom_deg;
        let target_deg = source_deg + elt_deg;
        let output_module = res.get_module(elt_hom_deg);
        let output_gens = output_module.get_number_of_gens_in_degree(elt_deg);
        let mut output = FpVector::new(p, output_module.get_dimension(elt_deg), 0);
        for l in 0 .. res.get_number_of_gens_in_bidegree(target_hom_deg, target_deg) {
            f.get_map(elt_hom_deg).apply_to_generator(&mut output, 1, target_deg, l);
            // TODO: Why the 0 here? The opgen_to_idx call makes sense and is needed in the C code.
            // Why is it misleading us here?? Is there a mistake in the quasi-inverse code? What happens when multiple
            // classes are in the same bidegree?
            let vector_idx = 0;//output_module.operation_generator_to_index(0, 0, elt_deg, elt_idx);
            if output.get_entry(0) != vector_idx {
                res.add_structline(
                    elt_name,
                    source_hom_deg, source_deg, source_idx, 
                    target_hom_deg, target_deg, l
                );
            }
        }
        println!("\n\n\n");
    }

    pub fn add_self_map(&mut self, homological_degree : u32, internal_degree : i32, name : String, map_data : Matrix) {
        self.self_maps.push(
            SelfMap {
                homological_degree,
                internal_degree,
                name,
                map_data : TempStorage::new(map_data),
                map : ResolutionHomomorphism::new("".to_string(), Rc::clone(&self.resolution), Rc::clone(&self.resolution), homological_degree, internal_degree)
            }
        );
    }
    
    pub fn compute_self_maps(&self, homological_degree : u32, mut internal_degree : i32) {
        let p = self.get_prime();
        for f in &self.self_maps {
            let hom_deg = f.homological_degree;
            let int_deg = f.internal_degree;
            if homological_degree < hom_deg || internal_degree < int_deg {
                continue;
            }
            if hom_deg == homological_degree && int_deg == internal_degree {
                let mut map_data = f.map_data.take();
                f.map.extend_step(hom_deg, int_deg, Some(&mut map_data));
            }
            f.map.extend(homological_degree, internal_degree);
            internal_degree -= 1;
            let output_homological_degree = homological_degree - f.homological_degree;
            let output_internal_degree = internal_degree - f.internal_degree;
            let source_module = self.resolution.get_module(homological_degree);
            let target_module = self.unit_resolution.get_module(output_homological_degree);
            let num_source_gens = source_module.get_number_of_gens_in_degree(internal_degree);
            let num_target_gens = target_module.get_number_of_gens_in_degree(output_internal_degree);
            if num_source_gens == 0 || num_target_gens == 0 {
                return;
            }
            let target_dim = target_module.get_dimension(output_internal_degree);
            let mut result = FpVector::new(p, target_dim, 0);
            // println!("hom_deg : {}, int_deg : {}, num_source_gens : {}, num_target_gens : {}", homological_degree, internal_degree, num_source_gens, num_target_gens);
            for j in 0 .. num_source_gens {
                f.map.get_map(output_homological_degree).apply_to_generator(&mut result, 1, internal_degree, j);
                for k in 0 .. num_target_gens {
                    let vector_idx = target_module.operation_generator_to_index(0, 0, output_internal_degree, k);
                    let coeff = result.get_entry(vector_idx);
                    if coeff != 0 {
                        self.resolution.add_structline(
                            &f.name,
                            output_homological_degree, output_internal_degree, k,
                            homological_degree, internal_degree, j
                        );
                    }
                }
            }
            internal_degree += 1;
        }
    }

    pub fn add_from_json(&mut self, json : &mut Value){
        let products_value = &json["products"];
        if products_value.is_null() {
            return;
        }
        let products = products_value.as_array().unwrap();
        for prod in products {
            let hom_deg = prod["hom_deg"].as_u64().unwrap() as u32;
            let int_deg = prod["int_deg"].as_i64().unwrap() as i32;
            let idx = prod["index"].as_u64().unwrap() as usize;
            let name = prod["name"].as_str().unwrap();
            self.add_product(hom_deg, int_deg, idx, name.to_string());
        }
    }
}

use crate::module::OptionModule;
use crate::module_homomorphism::ZeroHomomorphism;
use crate::chain_complex::ChainComplexConcentratedInDegreeZero;
pub type ModuleResolutionWithChainMaps<M1, M2>
    = ResolutionWithChainMaps<
        OptionModule<M1>,
        ZeroHomomorphism<OptionModule<M1>, OptionModule<M1>>,
        ChainComplexConcentratedInDegreeZero<M1>,
        OptionModule<M2>,
        ZeroHomomorphism<OptionModule<M2>, OptionModule<M2>>,
        ChainComplexConcentratedInDegreeZero<M2>
    >;