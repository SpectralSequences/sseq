use std::sync::Arc;

use crate::algebra::Algebra;
use crate::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use crate::module::{FreeModule, Module, ZeroModule};
use fp::matrix::Matrix;
use fp::vector::{FpVector, SliceMut};
use once::OnceBiVec;

#[cfg(feature = "json")]
use {
    crate::algebra::JsonAlgebra, bivec::BiVec, rustc_hash::FxHashMap as HashMap, serde_json::Value,
};

struct FPMIndexTable {
    gen_idx_to_fp_idx: Vec<isize>,
    fp_idx_to_gen_idx: Vec<usize>,
}

pub struct FinitelyPresentedModule<A: Algebra> {
    name: String,
    min_degree: i32,
    generators: Arc<FreeModule<A>>,
    relations: Arc<FreeModule<A>>,
    map: Arc<FreeModuleHomomorphism<FreeModule<A>>>,
    index_table: OnceBiVec<FPMIndexTable>,
}

impl<A: Algebra> std::fmt::Display for FinitelyPresentedModule<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<A: Algebra> PartialEq for FinitelyPresentedModule<A> {
    fn eq(&self, _other: &Self) -> bool {
        todo!()
    }
}

impl<A: Algebra> Eq for FinitelyPresentedModule<A> {}

impl<A: Algebra> ZeroModule for FinitelyPresentedModule<A> {
    fn zero_module(algebra: Arc<A>, min_degree: i32) -> Self {
        Self::new(algebra, "zero".to_string(), min_degree)
    }
}

impl<A: Algebra> FinitelyPresentedModule<A> {
    pub fn new(algebra: Arc<A>, name: String, min_degree: i32) -> Self {
        let generators = Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            format!("{}-gens", name),
            min_degree,
        ));
        let relations = Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            format!("{}-gens", name),
            min_degree,
        ));
        Self {
            name,
            min_degree,
            generators: Arc::clone(&generators),
            relations: Arc::clone(&relations),
            map: Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&relations),
                Arc::clone(&generators),
                0,
            )),
            index_table: OnceBiVec::new(min_degree),
        }
    }

    pub fn generators(&self) -> Arc<FreeModule<A>> {
        Arc::clone(&self.generators)
    }

    pub fn add_generators(&mut self, degree: i32, gen_names: Vec<String>) {
        let num_gens = gen_names.len();
        self.generators
            .add_generators(degree, num_gens, Some(gen_names));
    }

    pub fn add_relations(&mut self, degree: i32, relations_matrix: &mut Matrix) {
        let num_relns = relations_matrix.rows();
        self.relations.add_generators(degree, num_relns, None);
        self.map
            .add_generators_from_matrix_rows(degree, relations_matrix.as_slice_mut());
    }

    pub fn gen_idx_to_fp_idx(&self, degree: i32, idx: usize) -> isize {
        assert!(degree >= self.min_degree);
        self.index_table[degree].gen_idx_to_fp_idx[idx]
    }

    pub fn fp_idx_to_gen_idx(&self, degree: i32, idx: usize) -> usize {
        assert!(degree >= self.min_degree);
        self.index_table[degree].fp_idx_to_gen_idx[idx]
    }
}

#[cfg(feature = "json")]
impl<A: JsonAlgebra> FinitelyPresentedModule<A> {
    // Exact duplicate of function in fdmodule.rs...
    fn module_gens_from_json(
        gens: &Value,
    ) -> (
        BiVec<usize>,
        BiVec<Vec<String>>,
        HashMap<String, (i32, usize)>,
    ) {
        let gens = gens.as_object().unwrap();
        assert!(!gens.is_empty());
        let mut min_degree = 10000;
        let mut max_degree = -10000;
        for (_name, degree_value) in gens.iter() {
            let degree = degree_value.as_i64().unwrap() as i32;
            if degree < min_degree {
                min_degree = degree;
            }
            if degree + 1 > max_degree {
                max_degree = degree + 1;
            }
        }
        let mut gen_to_idx = HashMap::default();
        let mut graded_dimension = BiVec::with_capacity(min_degree, max_degree);
        let mut gen_names = BiVec::with_capacity(min_degree, max_degree);

        for _ in min_degree..max_degree {
            graded_dimension.push(0);
            gen_names.push(vec![]);
        }

        for (name, degree_value) in gens {
            let degree = degree_value.as_i64().unwrap() as i32;
            gen_names[degree].push(name.clone());
            gen_to_idx.insert(name.clone(), (degree, graded_dimension[degree]));
            graded_dimension[degree] += 1;
        }
        (graded_dimension, gen_names, gen_to_idx)
    }

    pub fn from_json(algebra: Arc<A>, json: &Value) -> error::Result<Self> {
        let p = algebra.prime();
        let name = json["name"].as_str().unwrap_or("").to_string();
        let gens = &json["gens"];
        let (num_gens_in_degree, gen_names, gen_to_deg_idx) = Self::module_gens_from_json(gens);
        let relations_value = &json[algebra.prefix().to_string() + "_relations"];
        let relations_values = relations_value.as_array().unwrap();
        let min_degree = num_gens_in_degree.min_degree();
        let max_gen_degree = num_gens_in_degree.len();
        algebra.compute_basis(20);
        let relations: Vec<Vec<_>> = relations_values
            .iter()
            .map(|reln| {
                reln.as_array()
                    .unwrap()
                    .iter()
                    .map(|term| {
                        let op = &term["op"];
                        let (op_deg, op_idx) = algebra.json_to_basis(op).unwrap();
                        let gen_name = term["gen"].as_str().unwrap();
                        let (gen_deg, gen_idx) = gen_to_deg_idx[gen_name];
                        let coeff = term["coeff"].as_u64().unwrap() as u32;
                        let op_gen = crate::module::free_module::OperationGeneratorPair {
                            operation_degree: op_deg,
                            operation_index: op_idx,
                            generator_degree: gen_deg,
                            generator_index: gen_idx,
                        };
                        (coeff, op_gen)
                    })
                    .collect()
            })
            .collect();
        let max_relation_degree = relations
            .iter()
            .map(|reln| {
                let op_gen = &reln[0].1;
                op_gen.operation_degree + op_gen.generator_degree
            })
            .max()
            .unwrap();
        let mut relations_by_degree = BiVec::with_capacity(min_degree, max_relation_degree + 1);
        for _ in min_degree..=max_relation_degree {
            relations_by_degree.push(Vec::new());
        }
        for r in relations {
            let op_gen = &r[0].1;
            let degree = op_gen.operation_degree + op_gen.generator_degree;
            relations_by_degree[degree].push(r);
        }
        let max_degree = std::cmp::max(max_gen_degree, max_relation_degree);
        algebra.compute_basis(max_degree);
        let mut result = Self::new(Arc::clone(&algebra), name, min_degree);
        for i in min_degree..max_gen_degree {
            result.add_generators(i, gen_names[i].clone());
        }
        result.generators.extend_by_zero(max_degree);
        for i in min_degree..=max_relation_degree {
            let num_relns = relations_by_degree[i].len();
            let gens_dim = result.generators.dimension(i);
            let mut relations_matrix = Matrix::new(p, num_relns, gens_dim);
            for (j, relation) in relations_by_degree[i].iter().enumerate() {
                for term in relation {
                    let coeff = &term.0;
                    let op_gen = &term.1;
                    let basis_idx = result.generators.operation_generator_pair_to_idx(op_gen);
                    relations_matrix[j].set_entry(basis_idx, *coeff);
                }
            }
            result.add_relations(i, &mut relations_matrix);
        }
        Ok(result)
    }

    pub fn to_json(&self, json: &mut Value) {
        json["name"] = Value::String(self.to_string());
        json["type"] = Value::from("finitely presented module");
        // Because we only have one algebra, we must specify this.
        json["algebra"] = Value::from(vec![self.algebra().prefix()]);
        for (i, deg_i_gens) in self.generators.gen_names().iter_enum() {
            for gen in deg_i_gens {
                json["gens"][gen] = Value::from(i);
            }
        }
        json[format!("{}_relations", self.algebra().prefix())] = self.relations_to_json();
    }

    pub fn relations_to_json(&self) -> Value {
        let mut relations = Vec::new();
        for i in self.min_degree..=self.relations.max_computed_degree() {
            let num_relns = self.relations.number_of_gens_in_degree(i);
            for j in 0..num_relns {
                relations.push(
                    self.generators
                        .element_to_json(i, self.map.output(i, j).as_slice()),
                );
            }
        }
        Value::from(relations)
    }
}

impl<A: Algebra> Module for FinitelyPresentedModule<A> {
    type Algebra = A;

    fn algebra(&self) -> Arc<A> {
        self.generators.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.generators.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        self.generators.max_computed_degree()
    }

    fn compute_basis(&self, degree: i32) {
        self.generators.extend_by_zero(degree);
        self.relations.extend_by_zero(degree);
        self.map.compute_auxiliary_data_through_degree(degree);

        self.index_table.extend(degree, |i| {
            let qi = self.map.quasi_inverse(i).unwrap();
            let mut gen_idx_to_fp_idx = Vec::new();
            let mut fp_idx_to_gen_idx = Vec::new();
            for (i, &pivot) in qi.pivots().unwrap().iter().enumerate() {
                if pivot < 0 {
                    gen_idx_to_fp_idx.push(fp_idx_to_gen_idx.len() as isize);
                    fp_idx_to_gen_idx.push(i);
                } else {
                    gen_idx_to_fp_idx.push(-1);
                }
            }
            FPMIndexTable {
                gen_idx_to_fp_idx,
                fp_idx_to_gen_idx,
            }
        });
    }

    fn dimension(&self, degree: i32) -> usize {
        assert!(degree >= self.min_degree);
        self.index_table[degree].fp_idx_to_gen_idx.len()
    }

    fn act_on_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        let p = self.prime();
        let gen_idx = self.fp_idx_to_gen_idx(mod_degree, mod_index);
        let out_deg = mod_degree + op_degree;
        let gen_dim = self.generators.dimension(out_deg);
        let mut temp_vec = FpVector::new(p, gen_dim);
        self.generators.act_on_basis(
            temp_vec.as_slice_mut(),
            coeff,
            op_degree,
            op_index,
            mod_degree,
            gen_idx,
        );
        let image = self.map.image(out_deg).unwrap();
        image.reduce(temp_vec.as_slice_mut());
        for i in 0..result.as_slice().len() {
            let value = temp_vec.entry(self.fp_idx_to_gen_idx(out_deg, i));
            result.add_basis_element(i, value);
        }
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let gen_idx = self.fp_idx_to_gen_idx(degree, idx);
        self.generators.basis_element_to_string(degree, gen_idx)
    }
}
