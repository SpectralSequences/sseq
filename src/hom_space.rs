use std::rc::Rc;

use bivec::BiVec;

use crate::once::OnceBiVec;
use crate::fp_vector::{FpVector, FpVectorT};
use crate::block_structure::BlockStructure;
use crate::algebra::AlgebraAny;
use crate::field::Field;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::finite_dimensional_module::FiniteDimensionalModuleT;

struct HomModule<M : FiniteDimensionalModuleT> {
    algebra : Rc<AlgebraAny>,
    source : Rc<FreeModule>,
    target : Rc<M>,
    block_structures : OnceBiVec<BlockStructure>
}

impl<M : FiniteDimensionalModuleT> HomModule<M> {
    pub fn new(p : u32, source : Rc<FreeModule>, target : Rc<M>) -> Self {
        let algebra = Rc::new(AlgebraAny::from(Field::new(p)));
        let min_degree = source.get_min_degree() - target.max_degree();
        Self {
            algebra,
            source,
            target,
            block_structures : OnceBiVec::new(min_degree)
        }
    }

    pub fn evaluate_basis_map_on_element(&self, result : &mut FpVector, coeff : u32, degree : i32, f_idx : usize, x : &FpVector){
        let gen_basis_elt = self.block_structures[degree].index_to_generator_basis_elt(f_idx);
        let gen_deg = gen_basis_elt.generator_degree;
        let gen_idx = gen_basis_elt.generator_index;
        let op_deg = degree - gen_deg;
        let mod_idx = gen_basis_elt.basis_index;
        let block_start = self.source.operation_generator_to_index(op_deg, 0, gen_deg, gen_idx);
        let block_dim = self.get_algebra().get_dimension(op_deg); 
        let block_end = block_start + block_dim;
        for i in block_start .. block_end {
            let v = x.get_entry(i);
            self.target.act_on_basis(result, (coeff * v) % p, op_deg, i, degree, mod_idx);
        }
    }

    pub fn evaluate_on_basis(&self, result : &mut FpVector, coeff : u32, degree : i32, f : &FpVector, x_idx : usize) {
        assert!(degree <= block_structures.max_degree());        
        assert!(f.get_dimension() == self.get_dimension(degree));
        let operation_generator = self.source.index_to_op_gen(x_idx);
        let gen_deg = operation_generator.generator_degree;
        let gen_idx = operation_generator.generator_index;
        let op_deg = operation_generator.operation_degree;
        let op_idx = operation_generator.operation_index;
        let (block_start, block_size) = self.block_structures[degree].generator_to_block(generator_degree, generator_index);
        let old_slice = f.get_slice();
        f.set_slice(block_min, block_max);
        self.target.act(result, coeff, op_deg, op_idx, gen_deg, f);
        f.restore_slice(old_slice);
    }

    pub fn evaluate(&self, result : &mut FpVector, coeff : u32, degree : i32, f : FpVector, x : FpVector) {
        assert!(degree <= block_structures.max_degree());
        assert!(f.get_dimension() == self.get_dimension(degree));
        assert!(x.get_dimension() == self.source.get_dimension(degree));
        if generator_degree >= self.get_min_degree() {
            let output_on_generator = self.get_output(generator_degree, generator_index);
            self.target.act(result, coeff, operation_degree, operation_index, generator_degree - self.degree_shift, output_on_generator);            
        }
        for (i, v) in x.iter().enumerate() {
            self.evaluate_on_basis(result, (coeff * v) % p, degree, f, x_idx)
        }
    }
}

impl<M : FiniteDimensionalModuleT> Module for HomModule<M> {
    fn get_algebra(&self) -> Rc<AlgebraAny> {
        Rc::clone(&self.algebra)
    }

    fn get_name(&self) -> &str {
        &""
    }

    fn get_min_degree(&self) -> i32 {
        self.block_structures.min_degree()
    }

    fn compute_basis(&self, degree : i32) {
        // assertion about source:
        // self.source.compute_basis(degree + self.target.max_degree());
        for d in self.get_min_degree() ..= degree {
            let mut block_sizes = BiVec::with_capacity(self.target.get_min_degree(), self.target.max_degree() + 1);
            for i in self.target.get_min_degree() .. self.target.max_degree() {
                let target_dim = self.target.get_dimension(d + i);
                if target_dim == 0 {
                    block_sizes.push(Vec::new());
                    continue;
                }
                let num_gens = self.source.get_number_of_gens_in_degree(d + i);
                let mut block_sizes_entry = Vec::with_capacity(num_gens);                
                for i in 0 .. num_gens {
                    block_sizes_entry.push(target_dim)
                }
                block_sizes.push(block_sizes_entry);
            }
            self.block_structures.push(BlockStructure::new(&block_sizes));
        }
    }

    fn get_dimension(&self, degree : i32) -> usize {
        self.block_structures[degree].total_dimension
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) {
        assert!(op_degree == 0);
        assert!(op_index == 0);
        result.add_basis_element(mod_index, coeff);    
    }
    
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        "".to_string()
    }
}