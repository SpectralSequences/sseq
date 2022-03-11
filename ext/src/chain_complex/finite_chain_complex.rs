use crate::chain_complex::{AugmentedChainComplex, BoundedChainComplex, ChainComplex};
use algebra::module::homomorphism::{FullModuleHomomorphism, ModuleHomomorphism, ZeroHomomorphism};
use algebra::module::{Module, ZeroModule};
use std::sync::Arc;

pub struct FiniteChainComplex<M, F>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M>,
{
    pub modules: Vec<Arc<M>>,
    pub zero_module: Arc<M>,
    pub differentials: Vec<Arc<F>>,
}

impl<M, F> FiniteChainComplex<M, F>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M> + ZeroHomomorphism<M, M>,
{
    pub fn max_degree(&self) -> i32 {
        unimplemented!()
    }

    pub fn pop(&mut self) {
        if self.modules.is_empty() {
            return;
        }
        self.modules.pop();
        if self.modules.is_empty() {
            self.differentials.drain(0..self.differentials.len() - 2);
        } else {
            let len = self.differentials.len();
            self.differentials.remove(len - 2);
            self.differentials[len - 3] = Arc::new(F::zero_homomorphism(
                self.zero_module(),
                Arc::clone(&self.modules[self.modules.len() - 1]),
                0,
            ));
        }
    }
}

impl<M, F> FiniteChainComplex<M, F>
where
    M: Module + ZeroModule,
    F: ModuleHomomorphism<Source = M, Target = M> + ZeroHomomorphism<M, M>,
{
    pub fn ccdz(module: Arc<M>) -> Self {
        let zero_module = Arc::new(M::zero_module(module.algebra(), module.min_degree()));
        let differentials = vec![
            Arc::new(F::zero_homomorphism(
                Arc::clone(&module),
                Arc::clone(&zero_module),
                0,
            )),
            Arc::new(F::zero_homomorphism(
                Arc::clone(&zero_module),
                Arc::clone(&module),
                0,
            )),
            Arc::new(F::zero_homomorphism(
                Arc::clone(&zero_module),
                Arc::clone(&zero_module),
                0,
            )),
        ];
        let modules = vec![module];
        Self {
            modules,
            zero_module,
            differentials,
        }
    }
}

impl<M: Module> FiniteChainComplex<M, FullModuleHomomorphism<M>> {
    pub fn map<N: Module<Algebra = M::Algebra>>(
        &self,
        mut f: impl FnMut(&M) -> N,
    ) -> FiniteChainComplex<N, FullModuleHomomorphism<N>> {
        let modules: Vec<Arc<N>> = self.modules.iter().map(|m| Arc::new(f(&*m))).collect();
        let zero_module = Arc::new(f(&*self.zero_module));
        let differentials: Vec<_> = self
            .differentials
            .iter()
            .enumerate()
            .map(|(s, d)| {
                if s == 0 {
                    Arc::new(
                        (**d)
                            .clone()
                            .replace_source(Arc::clone(&modules[0]))
                            .replace_target(Arc::clone(&zero_module)),
                    )
                } else {
                    Arc::new(
                        (**d)
                            .clone()
                            .replace_source(Arc::clone(modules.get(s).unwrap_or(&zero_module)))
                            .replace_target(Arc::clone(modules.get(s - 1).unwrap_or(&zero_module))),
                    )
                }
            })
            .collect();
        FiniteChainComplex {
            modules,
            zero_module,
            differentials,
        }
    }
}

impl<M, F> ChainComplex for FiniteChainComplex<M, F>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M>,
{
    type Algebra = M::Algebra;
    type Module = M;
    type Homomorphism = F;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.zero_module.algebra()
    }
    fn min_degree(&self) -> i32 {
        self.zero_module.min_degree()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, s: u32) -> Arc<Self::Module> {
        let s = s as usize;
        if s >= self.modules.len() {
            self.zero_module()
        } else {
            Arc::clone(&self.modules[s])
        }
    }

    fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
        let s = s as usize;
        let s = std::cmp::min(s, self.differentials.len() - 1); // The last entry is the zero homomorphism
        Arc::clone(&self.differentials[s])
    }

    fn compute_through_bidegree(&self, s: u32, t: i32) {
        for module in self.modules.iter().take(s as usize + 1) {
            module.compute_basis(t);
        }
    }

    fn has_computed_bidegree(&self, s: u32, t: i32) -> bool {
        s > self.modules.len() as u32 || t < self.module(s).max_computed_degree()
    }

    fn set_homology_basis(
        &self,
        _homological_degree: u32,
        _internal_degree: i32,
        _homology_basis: Vec<usize>,
    ) {
        unimplemented!()
    }
    fn homology_basis(&self, _homological_degree: u32, _internal_degree: i32) -> &Vec<usize> {
        unimplemented!()
    }
    fn max_homology_degree(&self, _homological_degree: u32) -> i32 {
        std::i32::MAX
    }

    fn next_homological_degree(&self) -> u32 {
        u32::MAX
    }
}

impl<M, F> BoundedChainComplex for FiniteChainComplex<M, F>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M>,
{
    fn max_s(&self) -> u32 {
        self.modules.len() as u32
    }
}

pub struct FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    pub modules: Vec<Arc<M>>,
    pub zero_module: Arc<M>,
    pub differentials: Vec<Arc<F1>>,
    pub target_cc: Arc<CC>,
    pub chain_maps: Vec<Arc<F2>>,
}

impl<M, F1, F2, CC> ChainComplex for FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    type Algebra = M::Algebra;
    type Module = M;
    type Homomorphism = F1;

    fn algebra(&self) -> Arc<M::Algebra> {
        self.zero_module.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.zero_module.min_degree()
    }

    fn has_computed_bidegree(&self, s: u32, t: i32) -> bool {
        s > self.modules.len() as u32 || t < self.module(s).max_computed_degree()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, s: u32) -> Arc<Self::Module> {
        let s = s as usize;
        if s >= self.modules.len() {
            self.zero_module()
        } else {
            Arc::clone(&self.modules[s])
        }
    }

    fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
        let s = s as usize;
        let s = std::cmp::min(s, self.differentials.len() - 1); // The last entry is the zero homomorphism
        Arc::clone(&self.differentials[s])
    }

    fn compute_through_bidegree(&self, _homological_degree: u32, _internal_degree: i32) {}

    fn set_homology_basis(
        &self,
        _homological_degree: u32,
        _internal_degree: i32,
        _homology_basis: Vec<usize>,
    ) {
        unimplemented!()
    }
    fn homology_basis(&self, _homological_degree: u32, _internal_degree: i32) -> &Vec<usize> {
        unimplemented!()
    }
    fn max_homology_degree(&self, _homological_degree: u32) -> i32 {
        std::i32::MAX
    }

    fn next_homological_degree(&self) -> u32 {
        u32::MAX
    }
}

impl<M, CC>
    FiniteAugmentedChainComplex<
        M,
        FullModuleHomomorphism<M>,
        FullModuleHomomorphism<M, CC::Module>,
        CC,
    >
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
{
    pub fn map<N: Module<Algebra = M::Algebra>>(
        &self,
        mut f: impl FnMut(&M) -> N,
    ) -> FiniteAugmentedChainComplex<
        N,
        FullModuleHomomorphism<N>,
        FullModuleHomomorphism<N, CC::Module>,
        CC,
    > {
        let modules: Vec<Arc<N>> = self.modules.iter().map(|m| Arc::new(f(&*m))).collect();
        let zero_module = Arc::new(f(&*self.zero_module));
        let differentials: Vec<_> = self
            .differentials
            .iter()
            .enumerate()
            .map(|(s, d)| {
                if s == 0 {
                    Arc::new(
                        (**d)
                            .clone()
                            .replace_source(Arc::clone(&modules[0]))
                            .replace_target(Arc::clone(&zero_module)),
                    )
                } else {
                    Arc::new(
                        (**d)
                            .clone()
                            .replace_source(Arc::clone(modules.get(s).unwrap_or(&zero_module)))
                            .replace_target(Arc::clone(modules.get(s - 1).unwrap_or(&zero_module))),
                    )
                }
            })
            .collect();
        let chain_maps: Vec<_> = std::iter::zip(&self.chain_maps, &modules)
            .map(|(c, m)| Arc::new((**c).clone().replace_source(Arc::clone(m))))
            .collect();
        FiniteAugmentedChainComplex {
            modules,
            zero_module,
            differentials,
            chain_maps,
            target_cc: Arc::clone(&self.target_cc),
        }
    }
}

impl<M, F1, F2, CC> AugmentedChainComplex for FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    type TargetComplex = CC;
    type ChainMap = F2;

    fn target(&self) -> Arc<Self::TargetComplex> {
        Arc::clone(&self.target_cc)
    }

    /// This currently crashes if `s` is greater than the s degree of the class this came from.
    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s as usize])
    }
}

impl<M, F1, F2, CC> From<FiniteAugmentedChainComplex<M, F1, F2, CC>> for FiniteChainComplex<M, F1>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    fn from(c: FiniteAugmentedChainComplex<M, F1, F2, CC>) -> FiniteChainComplex<M, F1> {
        FiniteChainComplex {
            modules: c.modules.clone(),
            zero_module: Arc::clone(&c.zero_module),
            differentials: c.differentials.clone(),
        }
    }
}
impl<M, F1, F2, CC> BoundedChainComplex for FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    fn max_s(&self) -> u32 {
        self.modules.len() as u32
    }
}
