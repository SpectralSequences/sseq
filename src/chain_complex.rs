use crate::module::Module;
use crate::module_homomorphism::ModuleHomomorphism;

pub trait ChainComplex {
    fn get_module(&self, homological_degree : usize) -> &Module;
    fn get_differential(&self, homological_degree : usize) -> &ModuleHomomorphism;
    fn compute_through_bidegree(&mut self, homological_degree : usize, degree : i32) {}
}