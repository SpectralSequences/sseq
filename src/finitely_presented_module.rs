use std::rc::Rc;
use std::collections::HashMap;
use serde_json::Value;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::Matrix;
use crate::once::OnceVec;
use crate::algebra::{Algebra, AlgebraAny};
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
    pub fn new(algebra : Rc<AlgebraAny>, name : String, min_degree : i32) -> Self {
        let generators = Rc::new(FreeModule::new(Rc::clone(&algebra), format!("{}-gens", name), min_degree));
        let relations = Rc::new(FreeModule::new(Rc::clone(&algebra), format!("{}-gens", name), min_degree));
        Self {
            name,
            min_degree,
            generators : Rc::clone(&generators),
            relations : Rc::clone(&relations),
            map : FreeModuleHomomorphism::new(Rc::clone(&relations), Rc::clone(&generators), 0),
            index_table : OnceVec::new()
        }
    }


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
    fn get_algebra(&self) -> Rc<AlgebraAny> {
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
            self.map.compute_quasi_inverse(&mut lock, i);
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
        qi.image.as_ref().unwrap().reduce(&mut temp_vec);
        for i in 0..result.get_dimension() {
            let value = temp_vec.get_entry(self.fp_idx_to_gen_idx(out_deg, i));
            result.set_entry(i, value);
        }
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        let gen_idx = self.fp_idx_to_gen_idx(degree, idx);
        self.generators.basis_element_to_string(degree, gen_idx)
    }

    fn from_json(algebra : Rc<AlgebraAny>, algebra_name: &str, json : &mut Value) -> Self {
        let p = algebra.get_prime();
        let name = json["name"].as_str().unwrap().to_string();        
        let gens = json["gens"].take();
        let (min_degree, num_gens_in_degree, gen_to_deg_idx) = Self::module_gens_from_json(&gens);
        let mut relations_value = json[algebra_name.to_owned() + "_relations"].take();
        let relations_values = relations_value.as_array_mut().unwrap();
        let max_gen_degree = num_gens_in_degree.len() as i32 + min_degree;
        algebra.compute_basis(20);
        let relations : Vec<Vec<_>> = relations_values.iter_mut().map(|reln| 
            reln.take().as_array_mut().unwrap().iter_mut().map(
                |term| {
                    let op = term["op"].take();
                    let (op_deg, op_idx) = algebra.json_to_basis(op);
                    let gen_name = term["gen"].as_str().unwrap();
                    let (gen_deg, gen_idx) = gen_to_deg_idx[&gen_name.to_string()];
                    let coeff = term["coeff"].as_u64().unwrap() as u32;
                    let op_gen = crate::free_module::OperationGeneratorPair {
                        operation_degree : op_deg,
                        operation_index : op_idx,
                        generator_degree : gen_deg,
                        generator_index : gen_idx
                    };
                    return (coeff, op_gen);
                }
            ).collect()
        ).collect();
        let max_relation_degree = relations.iter().map(|reln| {
            let op_gen = &reln[0].1;
            op_gen.operation_degree + op_gen.generator_degree
        }).max().unwrap();
        let num_relation_degrees = (max_relation_degree - min_degree + 1) as usize;
        let mut relations_by_degree = Vec::with_capacity(num_relation_degrees);
        for i in 0..num_relation_degrees {
            relations_by_degree.push(Vec::new());
        }
        for r in relations {
            let op_gen = &r[0].1;
            let degree = op_gen.operation_degree + op_gen.generator_degree;
            let degree_idx = (degree - min_degree) as usize;
            println!("degree : {}", degree);
            relations_by_degree[degree_idx].push(r);
        }
        let max_degree = std::cmp::max(max_gen_degree, max_relation_degree);
        algebra.compute_basis(max_degree);
        let result = Self::new(Rc::clone(&algebra), name, min_degree);
        for i in min_degree .. max_gen_degree {
            let idx = (i - min_degree) as usize;
            result.generators.add_generators_immediate(i, num_gens_in_degree[idx]);
        }
        result.generators.extend_by_zero(max_degree);
        for i in min_degree ..= max_relation_degree {
            let idx = (i - min_degree) as usize;
            let num_relns = relations_by_degree[idx].len();
            result.relations.add_generators_immediate(i, num_relns);
            println!("degree : {}, num_relns : {}", i, num_relns);
            let gens_dim = result.generators.get_dimension(i);
            let mut relations_matrix = Matrix::new(p, num_relns, gens_dim);
            for (j, relation) in relations_by_degree[idx].iter().enumerate() {
                for term in relation {
                    let coeff = &term.0;
                    let op_gen = &term.1;
                    let basis_idx = result.generators.operation_generator_pair_to_idx(&op_gen);
                    relations_matrix[j].set_entry(basis_idx, *coeff);
                }
            }
            let mut map_lock = result.map.get_lock();
            result.map.add_generators_from_matrix_rows(&map_lock, i, &mut relations_matrix, 0, 0, num_relns);
            *map_lock += 1;
        }
        return result;
    }

}
