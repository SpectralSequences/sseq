use std::rc::Rc;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::once::OnceVec;
use crate::algebra::Algebra;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::ChainComplex;

struct FPMIndexTable {
    gen_idx_to_fp_idx : Vec<isize>,
    fp_idx_to_gen_idx : Vec<usize>
}

pub struct FinitelyPresentedModule {
    name : String,
    min_degree : i32,
    generators : Rc<FreeModule>,
    relations : Rc<FreeModule>,
    map : FreeModuleHomomorphism<FreeModule>,
    index_table : OnceVec<FPMIndexTable>
}

impl FinitelyPresentedModule {
    pub fn new(generators : Rc<FreeModule>, relations : Rc<FreeModule>, map : FreeModuleHomomorphism<FreeModule>) -> Self {
        let min_degree = generators.get_min_degree();
        Self {
            name : "".to_string(),
            min_degree,
            generators,
            relations,
            map,
            index_table : OnceVec::new()
        }
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
