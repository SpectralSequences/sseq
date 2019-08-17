use std::rc::Rc;

use crate::once::OnceVec;
use crate::algebra::AlgebraAny;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::finite_dimensional_module::FiniteDimensionalModuleT;
// use crate::module_homomorphism::ModuleHomomorphism;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::{ChainComplex, CochainComplex};
use crate::hom_space::HomSpace;
use crate::hom_pullback::HomPullback;

struct HomComplex<CC : ChainComplex<FreeModule, FreeModuleHomomorphism<FreeModule>>, N : FiniteDimensionalModuleT> {
    min_degree : i32,
    source : Rc<CC>,
    target : Rc<N>,
    zero_module : Rc<HomSpace<N>>,
    modules : OnceVec<Rc<HomSpace<N>>>,
    differentials : OnceVec<Rc<HomPullback<N>>>,
}

impl<CC : ChainComplex<FreeModule, FreeModuleHomomorphism<FreeModule>>, N : FiniteDimensionalModuleT>
    HomComplex<CC, N> {
    pub fn new(source : Rc<CC>, target : Rc<N>) -> Self {
        let min_degree = source.get_min_degree() - target.max_degree();
        let zero_module = Rc::new(HomSpace::new(source.get_zero_module(), Rc::clone(&target)));
        Self {
            min_degree,
            source,
            target,
            zero_module,
            modules : OnceVec::new(),
            differentials : OnceVec::new(),
        }
    }
}

impl<CC : ChainComplex<FreeModule, FreeModuleHomomorphism<FreeModule>>, N : FiniteDimensionalModuleT>
    CochainComplex<HomSpace<N>, HomPullback<N>> for HomComplex<CC, N> {
    fn get_algebra(&self) -> Rc<AlgebraAny> {
        self.zero_module.get_algebra()
    }

    fn get_min_degree(&self) -> i32 {
        self.min_degree
    }

    fn get_zero_module(&self) -> Rc<HomSpace<N>> {
        Rc::clone(&self.zero_module)
    }

    fn get_module(&self, homological_degree : u32) -> Rc<HomSpace<N>> {
        Rc::clone(&self.modules[homological_degree])
    }

    fn get_differential(&self, homological_degree : u32) -> Rc<HomPullback<N>> {
        Rc::clone(&self.differentials[homological_degree])
    }

    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32){
        self.source.compute_through_bidegree(homological_degree, degree);
        if self.modules.len() == 0 {
            self.modules.push(Rc::new(HomSpace::new(self.source.get_module(0), Rc::clone(&self.target))));
            self.differentials.push(Rc::new(HomPullback::new(Rc::clone(&self.modules[0u32]), Rc::clone(&self.zero_module), self.source.get_differential(0))));
        }
        for i in self.modules.len() as u32 ..= homological_degree {
            self.modules.push(Rc::new(HomSpace::new(self.source.get_module(i), Rc::clone(&self.target))));
            self.differentials.push(Rc::new(HomPullback::new(Rc::clone(&self.modules[i]), Rc::clone(&self.modules[i - 1]), self.source.get_differential(i))));
        }
    }
}