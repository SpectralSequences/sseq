use std::sync::Arc;

use algebra::module::{
    homomorphism::{FullModuleHomomorphism, ModuleHomomorphism, ZeroHomomorphism},
    Module, ZeroModule,
};
use sseq::coordinates::Bidegree;

use crate::chain_complex::{AugmentedChainComplex, BoundedChainComplex, ChainComplex};

pub struct FiniteChainComplex<M, F = FullModuleHomomorphism<M>>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M>,
{
    modules: Vec<Arc<M>>,
    zero_module: Arc<M>,
    differentials: Vec<Arc<F>>,
}

impl<M, F> FiniteChainComplex<M, F>
where
    M: Module + ZeroModule,
    F: ModuleHomomorphism<Source = M, Target = M> + ZeroHomomorphism<M, M>,
{
    pub fn new(modules: Vec<Arc<M>>, differentials: Vec<Arc<F>>) -> Self {
        let zero_module = Arc::new(M::zero_module(
            modules[0].algebra(),
            modules[0].min_degree(),
        ));

        let mut all_differentials = Vec::with_capacity(differentials.len() + 2);
        all_differentials.push(Arc::new(F::zero_homomorphism(
            Arc::clone(&modules[0]),
            Arc::clone(&zero_module),
            0,
        )));
        all_differentials.extend(differentials);
        all_differentials.push(Arc::new(F::zero_homomorphism(
            Arc::clone(&zero_module),
            Arc::clone(&modules[modules.len() - 1]),
            0,
        )));

        Self {
            modules,
            zero_module,
            differentials: all_differentials,
        }
    }

    pub fn ccdz(module: Arc<M>) -> Self {
        Self::new(vec![module], vec![])
    }
}

impl<M, F> FiniteChainComplex<M, F>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M> + ZeroHomomorphism<M, M>,
{
    pub fn pop(&mut self) {
        if self.modules.is_empty() {
            return;
        }
        self.modules.pop();
        if self.modules.is_empty() {
            self.differentials.clear();
            self.differentials.push(Arc::new(F::zero_homomorphism(
                self.zero_module(),
                self.zero_module(),
                0,
            )));
        } else {
            self.differentials.pop();
            self.differentials.pop();
            self.differentials.push(Arc::new(F::zero_homomorphism(
                self.zero_module(),
                Arc::clone(&self.modules[self.modules.len() - 1]),
                0,
            )));
        }
    }
}

impl<M: Module> FiniteChainComplex<M, FullModuleHomomorphism<M>> {
    pub fn map<N: Module<Algebra = M::Algebra>>(
        &self,
        mut f: impl FnMut(&M) -> N,
    ) -> FiniteChainComplex<N, FullModuleHomomorphism<N>> {
        let modules: Vec<Arc<N>> = self.modules.iter().map(|m| Arc::new(f(m))).collect();
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
    type Homomorphism = F;
    type Module = M;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.zero_module.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.zero_module.min_degree()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, s: i32) -> Arc<Self::Module> {
        let s = s as usize;
        if s >= self.modules.len() {
            self.zero_module()
        } else {
            Arc::clone(&self.modules[s])
        }
    }

    fn differential(&self, s: i32) -> Arc<Self::Homomorphism> {
        let s = s as usize;
        let s = std::cmp::min(s, self.differentials.len() - 1); // The last entry is the zero homomorphism
        Arc::clone(&self.differentials[s])
    }

    fn compute_through_bidegree(&self, b: Bidegree) {
        for module in self.modules.iter().take(b.s() as usize + 1) {
            module.compute_basis(b.t());
        }
    }

    fn has_computed_bidegree(&self, b: Bidegree) -> bool {
        b.s() > self.modules.len() as i32 || b.t() < self.module(b.s()).max_computed_degree()
    }

    fn next_homological_degree(&self) -> i32 {
        i32::MAX
    }
}

impl<M, F> BoundedChainComplex for FiniteChainComplex<M, F>
where
    M: Module,
    F: ModuleHomomorphism<Source = M, Target = M>,
{
    fn max_s(&self) -> i32 {
        self.modules.len() as i32
    }
}

pub struct FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    cc: FiniteChainComplex<M, F1>,
    target_cc: Arc<CC>,
    chain_maps: Vec<Arc<F2>>,
}

impl<M, F1, F2, CC> ChainComplex for FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    type Algebra = M::Algebra;
    type Homomorphism = F1;
    type Module = M;

    fn algebra(&self) -> Arc<M::Algebra> {
        self.cc.algebra()
    }

    fn min_degree(&self) -> i32 {
        self.cc.min_degree()
    }

    fn has_computed_bidegree(&self, b: Bidegree) -> bool {
        self.cc.has_computed_bidegree(b)
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        self.cc.zero_module()
    }

    fn module(&self, s: i32) -> Arc<Self::Module> {
        self.cc.module(s)
    }

    fn differential(&self, s: i32) -> Arc<Self::Homomorphism> {
        self.cc.differential(s)
    }

    fn compute_through_bidegree(&self, b: Bidegree) {
        self.cc.compute_through_bidegree(b)
    }

    fn next_homological_degree(&self) -> i32 {
        self.cc.next_homological_degree()
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
        f: impl FnMut(&M) -> N,
    ) -> FiniteAugmentedChainComplex<
        N,
        FullModuleHomomorphism<N>,
        FullModuleHomomorphism<N, CC::Module>,
        CC,
    > {
        let cc = self.cc.map(f);
        let chain_maps: Vec<_> = std::iter::zip(&self.chain_maps, &cc.modules)
            .map(|(c, m)| Arc::new((**c).clone().replace_source(Arc::clone(m))))
            .collect();
        FiniteAugmentedChainComplex {
            cc,
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
    type ChainMap = F2;
    type TargetComplex = CC;

    fn target(&self) -> Arc<Self::TargetComplex> {
        Arc::clone(&self.target_cc)
    }

    /// This currently crashes if `s` is greater than the s degree of the class this came from.
    fn chain_map(&self, s: i32) -> Arc<Self::ChainMap> {
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
    fn from(c: FiniteAugmentedChainComplex<M, F1, F2, CC>) -> Self {
        c.cc
    }
}

impl<M, F1, F2, CC> BoundedChainComplex for FiniteAugmentedChainComplex<M, F1, F2, CC>
where
    M: Module,
    CC: ChainComplex<Algebra = M::Algebra>,
    F1: ModuleHomomorphism<Source = M, Target = M>,
    F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
{
    fn max_s(&self) -> i32 {
        self.cc.max_s()
    }
}

impl<M, F1> FiniteChainComplex<M, F1>
where
    M: Module,
    F1: ModuleHomomorphism<Source = M, Target = M>,
{
    pub fn augment<
        CC: ChainComplex<Algebra = M::Algebra>,
        F2: ModuleHomomorphism<Source = M, Target = CC::Module>,
    >(
        self,
        target_cc: Arc<CC>,
        chain_maps: Vec<Arc<F2>>,
    ) -> FiniteAugmentedChainComplex<M, F1, F2, CC> {
        FiniteAugmentedChainComplex {
            cc: self,
            target_cc,
            chain_maps,
        }
    }
}
