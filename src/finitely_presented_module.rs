use std::rc::Rc;
use std::collections::HashMap;
use serde_json::Value;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::once::OnceVec;
use crate::algebra::Algebra;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
// use crate::chain_complex::ChainComplex;

struct FPMIndexTable {
    gen_idx_to_fp_idx : Vec<isize>,
    fp_idx_to_gen_idx : Vec<usize>
}

pub struct FinitelyPresentedModule {
    name : String,
    min_degree : i32,
    pub generators : Rc<FreeModule>,
    pub relations : Rc<FreeModule>,
    pub map : FreeModuleHomomorphism<FreeModule>,
    index_table : OnceVec<FPMIndexTable>
}

impl FinitelyPresentedModule {
    pub fn new(algebra : Rc<dyn Algebra>, name : String, min_degree : i32) -> Self {
        let generators = Rc::new(FreeModule::new(Rc::clone(&algebra), format!("{}-gens", name), min_degree));
        let relations = Rc::new(FreeModule::new(Rc::clone(&algebra), format!("{}-gens", name), min_degree));
        Self {
            name,
            min_degree,
            generators : Rc::clone(&generators),
            relations : Rc::clone(&relations),
            map : FreeModuleHomomorphism::new(Rc::clone(&relations), Rc::clone(&generators), 0, 0),
            index_table : OnceVec::new()
        }
    }

    // pub fn from_json(algebra : Rc<dyn Algebra>, algebra_name: &str, json : &mut Value) -> Self {
    //     let name = json["name"].as_str().unwrap().to_string();        
    //     let gens = json["gens"].take();
    //     let (min_degree, num_gens_in_degree, gen_to_idx) = Self::module_gens_from_json(&gens);
    //     let mut relations_value = json[algebra_name.to_owned() + "_relations"].take();
    //     let relations = relations_value.as_array_mut().unwrap();
    //     let max_gen_degree = num_gens_in_degree.iter().max();
    //     let max_relation_degree = relations.iter().map(|reln| reln["degree"].as_i64().unwrap()).max();
    //     let num_relation_degrees = (max_relation_degree - min_degree) as usize;
    //     relations_vec = Vec::with_capacity(num_relation_degrees);
    //     for i in 0..num_relation_degrees {
    //         relations_vec.push(Vec::new());
    //     }
    //     for r in relations.iter() {
    //         let degree_idx = (r["degree"].as_i64() - min_degree) as usize;
    //         relations_vec[degree_idx].push(r);
    //     }
    //     let num_relations_in_degree = vec![0; (max_relation_degree - min_degree) as usize];
    //     let max_degree = std::cmp::max(max_gen_degree, max_relation_degree);
    //     algebra.compute_basis(max_degree);
    //     let mut result = Self::new(Rc::clone(&algebra), name, min_degree);
    //     for i in 0..
    //     result.generators.
    //     for action in actions.iter_mut() {
    //         let op = action["op"].take();
    //         let (degree, idx) = algebra.json_to_basis(op);

    //         let input_name = action["input"].as_str().unwrap();
    //         let (input_degree, input_idx) = gen_to_idx[&input_name.to_string()];
    //         let output_vec = result.get_action_mut(degree, idx, input_degree, input_idx);
    //         let outputs = action["output"].as_array().unwrap();
    //         for basis_elt in outputs {
    //             let output_name = basis_elt["gen"].as_str().unwrap();
    //             let output_idx = gen_to_idx[&output_name.to_string()].1;
    //             let output_coeff = basis_elt["coeff"].as_u64().unwrap() as u32;
    //             output_vec.set_entry(output_idx, output_coeff);
    //         }
    //     }
    //     return result;
    // }

    // Exact duplicate of function in fdmodule.rs...
    fn module_gens_from_json(gens : &Value) -> (i32, Vec<usize>, HashMap<&String, (i32, usize)>) {
        let gens = gens.as_object().unwrap();
        assert!(gens.len() > 0);
        let mut min_degree : i32 = 10000;
        let mut max_degree : i32 = -10000;
        for (_name, degree_value) in gens.iter() {
            let degree = degree_value.as_i64().unwrap() as i32;
            if degree < min_degree {
                min_degree = degree;
            }
            if degree + 1 > max_degree {
                max_degree = degree + 1;
            }
        }
        let mut gen_to_idx = HashMap::new();
        let mut graded_dimension = vec!(0; (max_degree - min_degree) as usize);
        for (name, degree_value) in gens.iter() {
            let degree = degree_value.as_i64().unwrap() as i32;
            let degree_idx = (degree - min_degree) as usize;
            gen_to_idx.insert(name, (degree as i32, graded_dimension[degree_idx]));
            graded_dimension[degree_idx] += 1;
        }
        return (min_degree as i32, graded_dimension, gen_to_idx);
    }

    fn gen_idx_to_fp_idx(&self, degree : i32, idx : usize) -> isize {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        self.index_table[degree_idx].gen_idx_to_fp_idx[idx]
    }

    fn fp_idx_to_gen_idx(&self, degree : i32, idx : usize) -> usize {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        self.index_table[degree_idx].fp_idx_to_gen_idx[idx]
    }
}

impl Module for FinitelyPresentedModule {
    fn get_algebra(&self) -> Rc<dyn Algebra> {
        self.generators.get_algebra()
    }

    fn get_min_degree(&self) -> i32 {
        self.generators.get_min_degree()
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn compute_basis(&self, degree : i32) {
        self.generators.extend_by_zero(degree);
        self.relations.extend_by_zero(degree);
        let min_degree = self.get_min_degree();
        for i in self.index_table.len() as i32 + min_degree ..= degree {
            let mut lock = self.map.get_lock();
            self.map.compute_kernel_and_image(&mut lock, i);
            let qi = self.map.get_quasi_inverse(degree).unwrap();
            let image = qi.image.as_ref().unwrap();
            let mut gen_idx_to_fp_idx = Vec::new();
            let mut fp_idx_to_gen_idx = Vec::new();
            let pivots = &image.column_to_pivot_row;
            for i in 0 .. pivots.len() {
                if pivots[i] < 0 {
                    gen_idx_to_fp_idx.push(fp_idx_to_gen_idx.len() as isize);
                    fp_idx_to_gen_idx.push(i);
                } else {
                    gen_idx_to_fp_idx.push(-1);
                }
            }
            self.index_table.push(FPMIndexTable {
                gen_idx_to_fp_idx,
                fp_idx_to_gen_idx
            });
        }
    }

    fn get_dimension(&self, degree : i32) -> usize {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        self.index_table[degree_idx].fp_idx_to_gen_idx.len()
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) {
        let p = self.get_prime();
        let gen_idx = self.fp_idx_to_gen_idx(mod_degree, mod_index);
        let out_deg = mod_degree + op_degree;
        let gen_dim = self.generators.get_dimension(out_deg);
        let mut temp_vec = FpVector::new(p, gen_dim, 0);
        self.generators.act_on_basis(&mut temp_vec, 1, op_degree, op_index, mod_degree, gen_idx);
        let qi = self.map.get_quasi_inverse(out_deg).unwrap();
        qi.reduce(&mut temp_vec);
        for i in 0..result.get_dimension() {
            let value = temp_vec.get_entry(self.fp_idx_to_gen_idx(out_deg, i));
            result.set_entry(i, value);
        }
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        let gen_idx = self.fp_idx_to_gen_idx(degree, idx);
        self.generators.basis_element_to_string(degree, gen_idx)
    }
}
