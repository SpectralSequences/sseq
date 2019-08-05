use crate::matrix::{ Subspace }; //QuasiInverse
use crate::algebra::Algebra;
use crate::module::{Module, ZeroModule, OptionModule};
use crate::module_homomorphism::{ModuleHomomorphism, ZeroHomomorphism};
use std::rc::Rc;


pub trait ChainComplex<M : Module, F : ModuleHomomorphism<M, M>> {
    fn get_prime(&self) -> u32 {
        self.get_algebra().get_prime()
    }
    fn get_algebra(&self) -> Rc<dyn Algebra>;
    fn get_min_degree(&self) -> i32;
    fn get_module(&self, homological_degree : u32) -> Rc<M>;
    fn get_differential(&self, homological_degree : u32) -> &F;
    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32);
    // fn computed_through_bidegree_q(&self, homological_degree : u32, degree : i32) -> bool { true }

    fn compute_kernel_and_image(&self,  homological_degree : u32, degree : i32){
        let p = self.get_prime();
        let d = self.get_differential(homological_degree);
        if d.get_max_kernel_degree() >= degree {
            return;
        }
        let mut lock = d.get_lock();
        if homological_degree == 0 {
            let module = self.get_module(0);
            let dim = module.get_dimension(degree);
            let kernel = Subspace::entire_space(p, dim);
            d.set_kernel(&lock, degree, kernel);
            *lock += 1;
            return;
        }
        d.compute_kernel_and_image(&mut lock, degree);
    }
}


pub struct ChainComplexConcentratedInDegreeZero<M : Module> {
    module : Rc<OptionModule<M>>,
    zero_module : Rc<OptionModule<M>>,
    d0 : ZeroHomomorphism<OptionModule<M>, OptionModule<M>>,
    d1 : ZeroHomomorphism<OptionModule<M>, OptionModule<M>>,
    other_ds : ZeroHomomorphism<OptionModule<M>, OptionModule<M>>
}

impl<M : Module> ChainComplexConcentratedInDegreeZero<M> {
    pub fn new(module : Rc<M>) -> Self {
        let p = module.get_prime();
        let zero_module_inner = Rc::new(ZeroModule::new(Rc::clone(&module.get_algebra())));
        let zero_module = Rc::new(OptionModule::Zero(Rc::clone(&zero_module_inner)));
        let some_module = Rc::new(OptionModule::Some(Rc::clone(&module)));
        Self {
            d0 : ZeroHomomorphism::new(Rc::clone(&some_module), Rc::clone(&zero_module)),
            d1 : ZeroHomomorphism::new(Rc::clone(&zero_module), Rc::clone(&some_module)),
            other_ds : ZeroHomomorphism::new(Rc::clone(&zero_module), Rc::clone(&zero_module)),
            module : some_module,
            zero_module
        }
    }
}

impl<M : Module> ChainComplex<OptionModule<M>, ZeroHomomorphism<OptionModule<M>, OptionModule<M>>> for ChainComplexConcentratedInDegreeZero<M> {
    fn get_algebra(&self) -> Rc<dyn Algebra> {
        self.module.get_algebra()
    }

    fn get_module(&self, homological_degree : u32) -> Rc<OptionModule<M>> {
        if homological_degree == 0 {
            Rc::clone(&self.module)
        } else {
            Rc::clone(&self.zero_module)
        }
    }

    fn get_min_degree(&self) -> i32 {
        self.module.get_min_degree()
    }

    fn get_differential(&self, homological_degree : u32) -> &ZeroHomomorphism<OptionModule<M>, OptionModule<M>> {
        match homological_degree {
            0 => &self.d0,
            1 => &self.d1,
            _ => &self.other_ds
        }
    }

    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32) {
        if homological_degree == 0 {
            self.module.compute_basis(degree);
        }
    }


}
