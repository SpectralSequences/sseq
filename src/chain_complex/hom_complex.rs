use std::sync::Arc;

use once::{OnceVec, OnceBiVec};
use crate::algebra::AlgebraAny;
use crate::module::{Module, FreeModule, BoundedModule};
// use crate::module::homomorphism::ModuleHomomorphism;
use crate::module::homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::{ChainComplex, CochainComplex};
use crate::module::HomModule;
use crate::module::homomorphism::HomPullback;

pub struct HomComplex<CC : ChainComplex<Module=FreeModule, Homomorphism=FreeModuleHomomorphism<FreeModule>>, N : BoundedModule> {
    min_degree : i32,
    source : Arc<CC>,
    target : Arc<N>,
    zero_module : Arc<HomModule<N>>,
    modules : OnceVec<Arc<HomModule<N>>>,
    differentials : OnceVec<Arc<HomPullback<N>>>,
    cohomology_basis : OnceVec<OnceBiVec<Vec<usize>>>
}

impl<CC : ChainComplex<Module=FreeModule, Homomorphism=FreeModuleHomomorphism<FreeModule>>, N : BoundedModule>
    HomComplex<CC, N> {
    pub fn new(source : Arc<CC>, target : Arc<N>) -> Self {
        let min_degree = source.min_degree() - target.max_degree();
        let zero_module = Arc::new(HomModule::new(source.zero_module(), Arc::clone(&target)));
        Self {
            min_degree,
            source,
            target,
            zero_module,
            modules : OnceVec::new(),
            differentials : OnceVec::new(),
            cohomology_basis : OnceVec::new()
        }
    }
}

impl<CC : ChainComplex<Module=FreeModule, Homomorphism=FreeModuleHomomorphism<FreeModule>>, N : BoundedModule>
    CochainComplex for HomComplex<CC, N> {
    type Module = HomModule<N>;
    type Homomorphism = HomPullback<N>;

    fn algebra(&self) -> Arc<AlgebraAny> {
        self.zero_module.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn zero_module(&self) -> Arc<HomModule<N>> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, homological_degree : u32) -> Arc<HomModule<N>> {
        Arc::clone(&self.modules[homological_degree])
    }

    fn differential(&self, homological_degree : u32) -> Arc<HomPullback<N>> {
        Arc::clone(&self.differentials[homological_degree])
    }

    fn set_cohomology_basis(&self, homological_degree : u32, internal_degree : i32, cohomology_basis : Vec<usize>) {
        for _ in cohomology_basis.len() ..= homological_degree as usize {
            self.cohomology_basis.push(OnceBiVec::new(self.min_degree()));
        }
        assert!(self.cohomology_basis[homological_degree].len() == internal_degree);
        self.cohomology_basis[homological_degree as usize].push(cohomology_basis);
    }

    fn cohomology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize> {
        &self.cohomology_basis[homological_degree as usize][internal_degree]
    }

    fn max_cohomology_degree(&self, homological_degree : u32) -> i32 {
        let homological_degree = homological_degree as usize;
        if homological_degree >= self.cohomology_basis.len(){
            return self.min_degree() - 1;
        }
        return self.cohomology_basis[homological_degree].len();
    }

    fn compute_through_bidegree(&self, homological_degree : u32, degree : i32){
        self.source.compute_through_bidegree(homological_degree, degree);
        if self.modules.len() == 0 {
            self.modules.push(Arc::new(HomModule::new(self.source.module(0), Arc::clone(&self.target))));
            self.differentials.push(Arc::new(HomPullback::new(Arc::clone(&self.modules[0u32]), Arc::clone(&self.zero_module), self.source.differential(0))));
        }
        for i in self.modules.len() as u32 ..= homological_degree {
            self.modules.push(Arc::new(HomModule::new(self.source.module(i), Arc::clone(&self.target))));
            self.differentials.push(Arc::new(HomPullback::new(Arc::clone(&self.modules[i]), Arc::clone(&self.modules[i - 1]), self.source.differential(i))));
        }
    }
}
