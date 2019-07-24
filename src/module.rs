use crate::fp_vector::FpVector;
use crate::algebra::Algebra;
use crate::module_homomorphism::ModuleHomomorphism;
use crate::module_homomorphism::ZeroHomomorphism;
use crate::chain_complex::ChainComplex;

// enum Module_Type {

// }

pub trait Module {
    fn get_prime(&self) -> u32 {
        self.get_algebra().get_prime()
    }
    fn get_algebra(&self) -> &Algebra;
    fn get_name(&self) -> &str;
    // fn get_type() -> Module_Type;
    // int min_degree;
    // int max_degree; // Rename to max_allocated_degree?
    // int max_computed_degree;
// Methods:
    fn compute_basis(&mut self, degree : i32) {}
    fn get_dimension(&self, degree : i32) -> usize;
    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize);

    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
        assert!(input.get_dimension() == self.get_dimension(input_degree));
        let p = self.get_algebra().get_prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act_on_basis(result, (coeff * v) % p, op_degree, op_index, input_degree, i);
        }
    }
}

struct ZeroModule<'a> {algebra : &'a Algebra, name : String }

impl<'a> ZeroModule<'a> {
    fn new(algebra : &'a Algebra) -> Self {
        let name = format!("Zero Module over {}", algebra.get_name());
        ZeroModule {
            algebra,
            name
        }
    }
}

impl<'a> Module for ZeroModule<'a> {
    fn get_algebra(&self) -> &Algebra {
        self.algebra
    }
    
    fn get_name(&self) -> &str{
        return &self.name;
    }

    fn get_dimension(&self, degree : i32) -> usize {
        0
    }

    // Since the dimension is 0, the input of this function is an element of the basis which is the empty set.
    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        assert!(false);
    }

}

struct ChainComplexConcentratedInDegreeZero<'a> {
    module : &'a Module,
    zero_module : ZeroModule<'a>,
    d0 : ZeroHomomorphism<'a>,
    d1 : ZeroHomomorphism<'a>,
    other_ds : ZeroHomomorphism<'a>
}

impl<'a> ChainComplexConcentratedInDegreeZero<'a> {
    pub fn new(module : &'a Module) -> Self {
        let zero_module = ZeroModule::new(module.get_algebra());
        // Warning: Stupid Rust acrobatics! Make Rust forget that zero_module_ref depends on zero_module.
        let zero_module_ptr : *const ZeroModule = &zero_module;
        let zero_module_ref : &'a ZeroModule = unsafe{std::mem::transmute(zero_module_ptr)};
        let d0  = ZeroHomomorphism::new(module, zero_module_ref);
        let d1 = ZeroHomomorphism::new(zero_module_ref, module);
        let other_ds = ZeroHomomorphism::new(zero_module_ref, zero_module_ref);
        Self {
            module,
            zero_module,
            d0,
            d1,
            other_ds
        }
    }
}

impl<'a> ChainComplex for ChainComplexConcentratedInDegreeZero<'a> {
    fn get_module(&self, homological_degree : usize) -> &Module {
        if homological_degree == 0 {
            return self.module;
        } else {
            return &self.zero_module;
        }
    }

    fn get_differential(&self, homological_degree : usize) -> &ModuleHomomorphism {
        match homological_degree {
            0 => &self.d0,
            1 => &self.d1,
            _ => &self.other_ds
        } 
    }
}