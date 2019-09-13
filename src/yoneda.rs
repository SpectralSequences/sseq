
use crate::chain_complex::{ChainComplex, AugmentedChainComplex};
use crate::module::{Module, FreeModule, QuotientModule as QM, TruncatedModule as TM, TruncatedHomomorphism, TruncatedHomomorphismSource, QuotientHomomorphism, QuotientHomomorphismSource};
use crate::module_homomorphism::{ModuleHomomorphism, FDModuleHomomorphism};
use crate::algebra::{Algebra, AlgebraAny, AdemAlgebra};

use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::Matrix;

use std::collections::HashSet;
use std::sync::Arc;

const PENALTY_UNIT : u32 = 100;

fn rate_operation(algebra : &AlgebraAny, op_deg : i32, op_idx : usize) -> u32 {
    match algebra {
        AlgebraAny::AdemAlgebra(a) => rate_adem_operation(a, op_deg, op_idx),
        _ => 1
    }
}

fn rate_adem_operation(algebra : &AdemAlgebra, deg : i32, idx: usize) -> u32{
    if algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
//    elt.ps.len() as u32
    let mut pref = 0;
    for i in elt.ps.iter() {
        let mut i = *i;
        while i != 0 {
            pref += i & 1;
            i >>= 1;
        }
    }
    pref
}

pub struct YonedaRepresentative<CC : AugmentedChainComplex> {
    modules : Vec<Arc<QM<TM<CC::Module>>>>,
    zero_module : Arc<QM<TM<CC::Module>>>,
    differentials : Vec<Arc<FDModuleHomomorphism<QM<TM<CC::Module>>, QM<TM<CC::Module>>>>>,
    target_cc : Arc<CC::TargetComplex>,
    chain_maps : Vec<Arc<FDModuleHomomorphism<QM<TM<CC::Module>>, <CC::ChainMap as ModuleHomomorphism>::Target>>>
}

impl<CC : AugmentedChainComplex> ChainComplex for YonedaRepresentative<CC> {
    type Module = QM<TM<CC::Module>>;
    type Homomorphism = FDModuleHomomorphism<QM<TM<CC::Module>>, QM<TM<CC::Module>>>;

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
    type ChainMap = FDModuleHomomorphism<QM<TM<CC::Module>>, <<CC as AugmentedChainComplex>::ChainMap as ModuleHomomorphism>::Target>;

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

pub fn yoneda_representative<CC>(cc : Arc<CC>, s_max : u32, t_max : i32, idx : usize) -> YonedaRepresentative<CC>
where CC : AugmentedChainComplex<Module=FreeModule> {
    assert!(s_max > 0);
    let p = cc.prime();
    let algebra = cc.algebra();

    let mut modules = (0 ..= s_max).map(|s| QM::new(Arc::new(TM::new(cc.module(s), t_max)))).collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    // These are the generators for each s that have been chosen to keep.
    let mut chosen_generators : Vec<HashSet<(i32, usize)>> = vec![HashSet::new(); s_max as usize];

    for t in (0 ..= t_max).rev() {
        let mut differential_target : Option<Matrix> = None;
        for s in (0 .. s_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }
            let d = cc.differential(s + 1);

            let (target, source) = split_mut_borrow(&mut modules, s as usize, s as usize + 1);

            let mut keep : HashSet<usize> = HashSet::new();

            // We find the list of things we want to keep, in terms of the original basis.
            // First we look for things with non-zero Steenrod operations.

            let mut generators : Vec<(i32, usize)> = Vec::new();
            for op_deg in 1 ..= t_max - t {
                for op_idx in algebra.generators(op_deg) {
                    generators.push((op_deg, op_idx));
                }
            }

            if s + 1 == s_max {
                if t == t_max {
                    keep.insert(idx);
                }
            } else {
                for i in 0 .. source.dimension(t) {
                    // This check should be outside, but we keep it in so that we indent
                    // less

                    let i = source.basis_list[t][i];
                    // Check if there are non-zero Steenrod operations.
                    for (op_deg, op_idx) in generators.iter() {
                        let mut result = FpVector::new(p, source.module.dimension(t + *op_deg));
                        source.act_on_original_basis(&mut result, 1, *op_deg, *op_idx, t, i);
                        if !result.is_zero() {
                            keep.insert(i);

                            let opgen = source.module.module.index_to_op_gen(t, i);
                            chosen_generators[s as usize].insert((opgen.generator_degree, opgen.generator_index));
                            break;
                        }
                    }
                }
            }

            // Add differentials to the list of targets to keep.
            if let Some(mut diffs) = differential_target {
                // We now assign preferences to the basis elements of source
                let mut prefs : Vec<u32> = vec![0; source.module.dimension(t)];
                let subspace = &source.subspaces[t];
                for i in 0 .. source.module.dimension(t) {
                    if subspace.column_to_pivot_row[i] >= 0 {
                        // We should never get to use this
                        prefs[i] = PENALTY_UNIT * 100;
                        continue;
                    }

                    if keep.contains(&i) {
                        continue;
                    }

                    let opgen = source.module.module.index_to_op_gen(t, i);

                    prefs[i] += rate_operation(&*algebra, opgen.operation_degree, opgen.operation_index);

                    if !chosen_generators[s as usize].contains(&(opgen.generator_degree, opgen.generator_index)) {
                        prefs[i] += PENALTY_UNIT;
                    }

                    for k in 0 .. subspace.matrix.rows() {
                        // This means we have quotiented out by something
                        if subspace.matrix[k].entry(i) != 0 {
                            prefs[i] += PENALTY_UNIT * 2;
                            break;
                        }
                    }
                }
                let mut prefs = prefs.iter().enumerate().map(|(x, y)| (y, x)).collect::<Vec<_>>();
                prefs.sort_unstable();
                // Sort uses lexicographical ordering, so this sorts by the preference
                let perms = prefs.into_iter().map(|(x, y)| y);

                let new_keep = diffs.find_pivots_permutation(perms);

                for i in new_keep {
                    let opgen = source.module.module.index_to_op_gen(t, i);
                    chosen_generators[s as usize].insert((opgen.generator_degree, opgen.generator_index));
                    keep.insert(i);
                }
            }

            // Now do the quotienting
            let mut source_kills : Vec<usize> = Vec::with_capacity(source.module.dimension(t));
            let mut target_kills : Vec<FpVector> = Vec::with_capacity(target.module.dimension(t));
            for i in 0 .. source.dimension(t) {
                let i = source.basis_list[t][i];
                if keep.contains(&i) {
                    continue;
                }
                let mut target_kill_vec = FpVector::new(p, target.module.dimension(t));
                d.apply_to_basis_element(&mut target_kill_vec, 1, t, i);
                target_kills.push(target_kill_vec);
                source_kills.push(i);
            }
            source.quotient_basis_elements(t, source_kills);
            target.quotient_vectors(t, target_kills);

            // Finally, record the differentials.
            let source_dim = source.dimension(t);
            let target_dim = target.module.dimension(t);

            let mut differentials = Vec::with_capacity(source_dim);

            for i in 0 .. source_dim {
                let i = source.basis_list[t][i];
                // We should be intelligent and skip if i is the target of the differential, for
                // which we know d of this will be in the span of other stuff (it's not necessarily
                // zero, because quotienting is weird).

                let mut target_kill_vec = FpVector::new(p, target_dim);
                d.apply_to_basis_element(&mut target_kill_vec, 1, t, i);
                target.subspaces[t].reduce(&mut target_kill_vec);
                differentials.push(target_kill_vec);
            }

            differential_target = Some(Matrix::from_rows(p, differentials));
        }
    }

    let zero_module = Arc::new(QM::new(Arc::new(TM::new(cc.zero_module(), t_max))));

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

    YonedaRepresentative {
        modules,
        zero_module,
        differentials,
        target_cc : cc.target(),
        chain_maps
    }
}
