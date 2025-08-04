use std::sync::Arc;

use fp::vector::{FpSliceMut, FpVector};
use itertools::Itertools;
use once::OnceBiVec;
use serde_json::Value;

use crate::{
    algebra::Algebra,
    module::{
        homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
        FreeModule, Module, ZeroModule,
    },
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
            format!("{name}-gens"),
            min_degree,
        ));
        let relations = Arc::new(FreeModule::new(
            Arc::clone(&algebra),
            format!("{name}-gens"),
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

    pub fn add_relations(&mut self, degree: i32, relations: Vec<FpVector>) {
        self.relations.add_generators(degree, relations.len(), None);
        self.map.add_generators_from_rows(degree, relations);
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

impl<A: Algebra> FinitelyPresentedModule<A> {
    pub fn from_json(algebra: Arc<A>, json: &Value) -> anyhow::Result<Self> {
        use anyhow::anyhow;
        use nom::{combinator::opt, Parser};

        use crate::steenrod_parser::digits;

        let p = algebra.prime();
        let name = json["name"].as_str().unwrap_or("").to_string();
        let (_, gen_names, gen_to_deg_idx) = crate::module_gens_from_json(&json["gens"]);

        let min_degree = gen_names.min_degree();
        let mut result = Self::new(Arc::clone(&algebra), name, min_degree);

        for (i, gen_names) in gen_names.into_iter_enum() {
            result.add_generators(i, gen_names);
        }

        // A list of relations, specified by the degree then the element to be killed
        let mut relations: Vec<(i32, FpVector)> = json[algebra.prefix().to_string() + "_relations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|reln| {
                let mut deg = 0;
                let mut v = FpVector::new(p, 0);

                for term in reln.as_str().unwrap().split(" + ") {
                    let (term, coef) = opt(digits).parse(term).unwrap();
                    let coef: u32 = coef.unwrap_or(1);

                    let (op, gen) = term.rsplit_once(' ').unwrap_or(("1", term));
                    let (op_deg, op_idx) = algebra
                        .basis_element_from_string(op)
                        .ok_or_else(|| anyhow!("Invalid term in relation: {term}"))?;
                    let (gen_deg, gen_idx) = gen_to_deg_idx(gen)?;

                    if v.is_empty() {
                        deg = op_deg + gen_deg;
                        algebra.compute_basis(deg - min_degree);
                        result.generators.compute_basis(deg);
                        v.set_scratch_vector_size(result.generators.dimension(deg));
                    } else if op_deg + gen_deg != deg {
                        return Err(anyhow!(
                            "Relation has inconsistent degree. Expected {deg} but {term} has \
                             degree {}",
                            op_deg + gen_deg
                        ));
                    }

                    let idx = result
                        .generators
                        .operation_generator_to_index(op_deg, op_idx, gen_deg, gen_idx);

                    v.add_basis_element(idx, coef);
                }
                Ok((deg, v))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        relations.sort_unstable_by_key(|x| x.0);
        for (degree, rels) in &relations.into_iter().chunk_by(|x| x.0) {
            for deg in result.relations.max_computed_degree() + 1..degree {
                result.add_relations(deg, vec![]);
            }
            result.add_relations(degree, rels.map(|x| x.1).collect());
        }
        Ok(result)
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
        mut result: FpSliceMut,
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

    fn max_generator_degree(&self) -> Option<i32> {
        self.generators.max_generator_degree()
    }
}
