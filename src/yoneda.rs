
use crate::chain_complex::{ChainComplex, AugmentedChainComplex};
use crate::module::{Module, QuotientModule, TruncatedModule, BoundedModule, TruncatedHomomorphism, TruncatedHomomorphismSource, QuotientHomomorphism, QuotientHomomorphismSource};
use crate::module_homomorphism::{ModuleHomomorphism, FDModuleHomomorphism};
use crate::algebra::{Algebra, AlgebraAny};

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::Matrix;
use std::sync::Arc;

pub struct YonedaRepresentative<CC : AugmentedChainComplex> {
    modules : Vec<Arc<QuotientModule<TruncatedModule<CC::Module>>>>,
    zero_module : Arc<QuotientModule<TruncatedModule<CC::Module>>>,
    differentials : Vec<Arc<FDModuleHomomorphism<QuotientModule<TruncatedModule<CC::Module>>, QuotientModule<TruncatedModule<CC::Module>>>>>,
    target_cc : Arc<CC::TargetComplex>,
    chain_maps : Vec<Arc<FDModuleHomomorphism<QuotientModule<TruncatedModule<CC::Module>>, <<CC as AugmentedChainComplex>::ChainMap as ModuleHomomorphism>::Target>>>
}

impl<CC : AugmentedChainComplex> ChainComplex for YonedaRepresentative<CC> {
    type Module = QuotientModule<TruncatedModule<CC::Module>>;
    type Homomorphism = FDModuleHomomorphism<QuotientModule<TruncatedModule<CC::Module>>, QuotientModule<TruncatedModule<CC::Module>>>;

    fn algebra(&self) -> Arc<AlgebraAny> {
        self.target_cc.algebra()
    }
    fn min_degree(&self) -> i32 {
        self.target_cc.min_degree()
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, s : u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s as usize])
    }

    fn differential(&self, s : u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn compute_through_bidegree(&self, homological_degree : u32, internal_degree : i32) {}

    fn set_homology_basis(&self, homological_degree : u32, internal_degree : i32, homology_basis : Vec<usize>) { unimplemented!() }
    fn homology_basis(&self, homological_degree : u32, internal_degree : i32) -> &Vec<usize> { unimplemented!() }
    fn max_homology_degree(&self, homological_degree : u32) -> i32 { std::i32::MAX }

    fn max_computed_homological_degree(&self) -> u32 { std::u32::MAX }
    fn max_computed_degree(&self) -> i32 { std::i32::MAX }
}

impl<CC : AugmentedChainComplex> AugmentedChainComplex for YonedaRepresentative<CC> {
    type TargetComplex = CC::TargetComplex;
    type ChainMap = FDModuleHomomorphism<QuotientModule<TruncatedModule<CC::Module>>, <<CC as AugmentedChainComplex>::ChainMap as ModuleHomomorphism>::Target>;

    fn target(&self) -> Arc<Self::TargetComplex> {
        Arc::clone(&self.target_cc)
    }

    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s as usize])
    }
}

fn split_mut_borrow<T> (v : &mut Vec<T>, i : usize, j : usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative<CC : AugmentedChainComplex>(cc : Arc<CC>, s_max : u32, t_max : i32, idx : usize) -> YonedaRepresentative<CC> {
    assert!(s_max > 0);
    let p = cc.prime();
    let algebra = cc.algebra();

    let mut modules = (0 ..= s_max).map(|s| QuotientModule::new(Arc::new(TruncatedModule::new(cc.module(s), t_max)))).collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    for t in (0 ..= t_max).rev() {
        let mut differential_target = vec![-1; modules[s_max as usize].dimension(t)];
        for s in (0 .. s_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }
            let d = cc.differential(s + 1);

            let (target, source) = split_mut_borrow(&mut modules, s as usize, s as usize + 1);

            let mut source_kills : Vec<usize> = Vec::with_capacity(source.module.dimension(t));
            let mut target_kills : Vec<FpVector> = Vec::with_capacity(target.module.dimension(t));
            'outer: for i in 0 .. source.dimension(t) {
                if t == t_max && s + 1 == s_max && i == idx {
                    continue;
                }

                let i = source.basis_list[t][i];
                if differential_target[i] >= 0 {
                    continue;
                }
                // Check if there are non-zero Steenrod operations.
                for op_deg in 1 ..= t_max - t {
                    let mut result = FpVector::new(p, source.module.dimension(t + op_deg));
                    for op_idx in algebra.generators(op_deg) {
                        source.act_on_original_basis(&mut result, 1, op_deg, op_idx, t, i);
                        if !result.is_zero() {
                            continue 'outer;
                        }
                    }
                }
                let mut target_kill_vec = FpVector::new(p, target.module.dimension(t));
                // There are none. We can kill this.
                d.apply_to_basis_element(&mut target_kill_vec, 1, t, i);
                target_kills.push(target_kill_vec);
                source_kills.push(i);
            }
            source.quotient_basis_elements(t, source_kills);
            target.quotient_vectors(t, target_kills);

            let source_dim = source.dimension(t);
            let target_dim = target.module.dimension(t);
            let mut new_differential_target = vec![-1; target_dim];

            let mut differentials = Vec::with_capacity(source_dim);

            for i in 0 .. source_dim {
                let i = source.basis_list[t][i];
                if differential_target[i] >= 0 {
                    continue;
                }

                let mut target_kill_vec = FpVector::new(p, target_dim);
                d.apply_to_basis_element(&mut target_kill_vec, 1, t, i);
                target.subspaces[t].reduce(&mut target_kill_vec);
                differentials.push(target_kill_vec);
            }

            // We avoid pickingg elements b where ... + b is quotiented out
            let tdim = target_dim - target.subspaces[t].dimension();
            let mut permutation = Vec::with_capacity(tdim);
            let mut permutation_second = Vec::with_capacity(tdim);
            let mut row = 0;
            let subspace = &target.subspaces[t];
            'outer2: for i in 0 .. target_dim {
                if subspace.column_to_pivot_row[i] >= 0 {
                    row += 1;
                } else {
                    for k in 0 .. row {
                        if subspace.matrix[k].entry(i) != 0 {
                            permutation_second.push(i);
                            continue 'outer2;
                        }
                    }
                    permutation.push(i);
                }
            }
            permutation.extend(permutation_second.into_iter());
            let permutation = permutation.into_iter();

            let mut matrix = Matrix::from_rows(p, differentials);
            matrix.row_reduce_permutation(&mut new_differential_target, permutation);

            differential_target = new_differential_target;
        }
    }

    let zero_module = Arc::new(QuotientModule::new(Arc::new(TruncatedModule::new(cc.zero_module(), t_max))));

    let modules = modules.into_iter().map(Arc::new).collect::<Vec<_>>();

    let zero_differential = {
        let f = cc.differential(0);
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[0].module), Arc::clone(&zero_module.module)));
        Arc::new(FDModuleHomomorphism::from(QuotientHomomorphism::new(tf, Arc::clone(&modules[0]), Arc::clone(&zero_module))))
    };

    let mut differentials = vec![zero_differential];
    differentials.extend((0 .. s_max).into_iter().map(|s| {
        let f = cc.differential(s + 1);
        let s = s as usize;
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[s + 1].module), Arc::clone(&modules[s].module)));
        Arc::new(FDModuleHomomorphism::from(QuotientHomomorphism::new(tf, Arc::clone(&modules[s + 1]), Arc::clone(&modules[s]))))
    }));

    let chain_maps = (0 ..= s_max).into_iter().map(|s| {
        let f = cc.chain_map(s);
        let s = s as usize;
        let target = f.target();
        let tf = Arc::new(TruncatedHomomorphismSource::new(f, Arc::clone(&modules[s].module), Arc::clone(&target)));
        Arc::new(FDModuleHomomorphism::from(QuotientHomomorphismSource::new(tf, Arc::clone(&modules[s]), target)))
    }).collect::<Vec<_>>();

    let mut check = vec![0; t_max as usize + 1];
    for s in 0 ..= s_max as usize {
        println!("Dimension of {}th module is {} ({})", s, modules[s].total_dimension(), modules[s].module.total_dimension());

        for t in 0 ..= t_max {
            for i in 0 .. modules[s].dimension(t) {
                println!("{}: {}", t, modules[s].basis_element_to_string(t, i));
            }
            check[t as usize] += (if s % 2 == 0 { 1 } else { -1 }) * modules[s].dimension(t) as i32;
        }
    }
    println!("Check sum: {:?}", check);

    YonedaRepresentative {
        modules,
        zero_module,
        differentials,
        target_cc : cc.target(),
        chain_maps
    }
}
