use crate::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, ChainMap, FiniteAugmentedChainComplex,
};
use algebra::module::homomorphism::{
    BoundedModuleHomomorphism, FiniteModuleHomomorphism, FreeModuleHomomorphism,
    ModuleHomomorphism, ZeroHomomorphism,
};
use algebra::module::homomorphism::{
    QuotientHomomorphism, QuotientHomomorphismSource, TruncatedHomomorphism,
    TruncatedHomomorphismSource,
};
use algebra::module::{BoundedModule, FDModule, FiniteModule, FreeModule, Module};
use algebra::module::{QuotientModule as QM, TruncatedModule as TM};
use algebra::{AdemAlgebra, Algebra, GeneratedAlgebra, SteenrodAlgebra};
use fp::matrix::{Matrix, Subspace};
use fp::vector::FpVector;

use bivec::BiVec;

use rustc_hash::FxHashSet as HashSet;
use std::sync::Arc;

const PENALTY_UNIT: i32 = 10000;

pub type Yoneda<CC> = FiniteAugmentedChainComplex<
    FiniteModule,
    FiniteModuleHomomorphism<FiniteModule>,
    FiniteModuleHomomorphism<
        <<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
    >,
    <CC as AugmentedChainComplex>::TargetComplex,
>;

#[allow(clippy::single_match)]
fn rate_operation(algebra: &Arc<SteenrodAlgebra>, op_deg: i32, op_idx: usize) -> i32 {
    let mut pref = 0;
    match &**algebra {
        SteenrodAlgebra::AdemAlgebra(a) => pref += rate_adem_operation(a, op_deg, op_idx),
        _ => (),
    };
    pref
}

fn rate_adem_operation(algebra: &AdemAlgebra, deg: i32, idx: usize) -> i32 {
    if *algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    let mut pref = 0;
    for i in &elt.ps {
        let mut i = *i;
        while i != 0 {
            pref += (i & 1) as i32;
            i >>= 1;
        }
    }
    pref
}

#[allow(dead_code)]
fn operation_drop(algebra: &AdemAlgebra, deg: i32, idx: usize) -> i32 {
    if *algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    if elt.ps.is_empty() {
        return 0;
    }

    let mut first = elt.ps[0];
    let mut drop = 1;
    while first & 1 == 0 {
        first >>= 1;
        drop *= 2;
    }
    deg - drop
}

fn split_mut_borrow<T>(v: &mut Vec<T>, i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative_element<TCM, TC, CC>(
    cc: Arc<CC>,
    s: u32,
    t: i32,
    idx: usize,
) -> Yoneda<CC>
where
    TCM: BoundedModule<Algebra = SteenrodAlgebra>,
    TC: BoundedChainComplex<Algebra = SteenrodAlgebra, Module = TCM>,
    CC: AugmentedChainComplex<
        Algebra = SteenrodAlgebra,
        TargetComplex = TC,
        Module = FreeModule<SteenrodAlgebra>,
        ChainMap = FreeModuleHomomorphism<TCM>,
    >,
{
    let p = cc.prime();

    let target = FDModule::new(cc.algebra(), "".to_string(), BiVec::from_vec(0, vec![1]));
    let map = FreeModuleHomomorphism::new(cc.module(s), Arc::new(target), t);
    let mut new_output = Matrix::new(p, cc.module(s).number_of_gens_in_degree(t), 1);
    new_output[idx].set_entry(0, 1);

    map.add_generators_from_matrix_rows(t, new_output.as_slice_mut());

    let cm = ChainMap {
        s_shift: s,
        chain_maps: vec![map],
    };
    yoneda_representative(cc, cm)
}

/// This function produces a quasi-isomorphic quotient of `cc` (as an augmented chain complex) that `map` factors through
pub fn yoneda_representative<TCM, TC, CC, CMM>(
    cc: Arc<CC>,
    map: ChainMap<FreeModuleHomomorphism<CMM>>,
) -> Yoneda<CC>
where
    TCM: BoundedModule<Algebra = SteenrodAlgebra>,
    TC: BoundedChainComplex<Algebra = SteenrodAlgebra, Module = TCM>,
    CC: AugmentedChainComplex<
        Algebra = SteenrodAlgebra,
        TargetComplex = TC,
        Module = FreeModule<SteenrodAlgebra>,
        ChainMap = FreeModuleHomomorphism<TCM>,
    >,
    CMM: BoundedModule<Algebra = SteenrodAlgebra>,
{
    yoneda_representative_with_strategy(
        cc,
        map,
        |module: &FreeModule<SteenrodAlgebra>, subspace: &Subspace, t: i32, i: usize| {
            let opgen = module.index_to_op_gen(t, i);

            let mut pref = rate_operation(
                &module.algebra(),
                opgen.operation_degree,
                opgen.operation_index,
            );

            for k in 0..subspace.matrix.rows() {
                if subspace[k].entry(i) != 0 {
                    pref += PENALTY_UNIT;
                }
            }
            pref
        },
    )
}

#[allow(clippy::cognitive_complexity)]
pub fn yoneda_representative_with_strategy<TCM, TC, CC, CMM, F>(
    cc: Arc<CC>,
    map: ChainMap<FreeModuleHomomorphism<CMM>>,
    strategy: F,
) -> Yoneda<CC>
where
    TCM: BoundedModule<Algebra = SteenrodAlgebra>,
    TC: BoundedChainComplex<Algebra = SteenrodAlgebra, Module = TCM>,
    CC: AugmentedChainComplex<
        Algebra = SteenrodAlgebra,
        TargetComplex = TC,
        Module = FreeModule<SteenrodAlgebra>,
        ChainMap = FreeModuleHomomorphism<TCM>,
    >,
    CMM: BoundedModule<Algebra = SteenrodAlgebra>,
    F: Fn(&CC::Module, &Subspace, i32, usize) -> i32,
{
    let p = cc.prime();
    let target_cc = cc.target();

    let t_shift: i32 = map.chain_maps[0].degree_shift();
    let s_shift: u32 = map.s_shift;

    let s_max = std::cmp::max(target_cc.max_s(), map.s_shift + map.chain_maps.len() as u32) - 1;
    let t_max = std::cmp::max(
        (0..target_cc.max_s())
            .map(|i| target_cc.module(i).max_degree())
            .max()
            .unwrap_or_else(|| target_cc.min_degree()),
        map.chain_maps[0].degree_shift()
            + map
                .chain_maps
                .iter()
                .map(|m| m.target().max_degree())
                .max()
                .unwrap(),
    );

    let t_min = cc.min_degree();

    let mut modules = (0..=s_max)
        .map(|s| QM::new(Arc::new(TM::new(cc.module(s), t_max))))
        .collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    for s in (1..=s_max).rev() {
        let mut differential_images: BiVec<Subspace> = BiVec::with_capacity(t_min, t_max + 1);

        if s < s_max {
            let prev = &modules[s as usize + 1];
            let curr = &modules[s as usize];
            let d = cc.differential(s + 1);

            for t in t_min..=t_max {
                let prev_dim = prev.dimension(t);
                let curr_orig_dim = curr.module.dimension(t);

                let mut differentials = Matrix::new(p, prev_dim, curr_orig_dim);

                for i in 0..prev_dim {
                    let j = prev.basis_list[t][i];
                    d.apply_to_basis_element(differentials[i].as_slice_mut(), 1, t, j);
                    curr.subspaces[t].reduce(differentials[i].as_slice_mut());
                }

                differentials.row_reduce();

                differential_images.push(Subspace {
                    matrix: differentials,
                });
            }
        }

        let (target, source) = split_mut_borrow(&mut modules, s as usize - 1, s as usize);
        let d = cc.differential(s);
        if s < s_max {
            let degrees_with_gens = source.module.module.get_degrees_with_gens(t_max);

            let mut prev_differentials: BiVec<Option<Subspace>> =
                BiVec::with_capacity(t_min, t_max + 1);
            let mut prev_subspaces: BiVec<Option<Subspace>> =
                BiVec::with_capacity(t_min, t_max + 1);
            let mut prev_basis_list: BiVec<Option<Vec<usize>>> =
                BiVec::with_capacity(t_min, t_max + 1);

            // This is use for sanity checking.
            let mut dim_drop: BiVec<Option<usize>> = BiVec::with_capacity(t_min, t_max + 1);

            for _ in t_min..=t_max {
                prev_differentials.push(None);
                prev_subspaces.push(None);
                prev_basis_list.push(None);
                dim_drop.push(None);
            }
            'gen_loop: for (i, &gen_dim) in degrees_with_gens.iter().enumerate() {
                // Check if augmentation map is non-zero on the generator
                if s < target_cc.max_s() && target_cc.module(s).dimension(gen_dim) > 0 {
                    let m = cc.chain_map(s);
                    let num_gens = source.module.module.number_of_gens_in_degree(gen_dim);
                    for i in 0..num_gens {
                        if !m.output(gen_dim, i).is_zero() {
                            continue 'gen_loop;
                        }
                    }
                }

                // Check if preserve map is non-zero on the generator
                if s >= s_shift && gen_dim >= t_shift {
                    if let Some(m) = map.chain_maps.get((s - s_shift) as usize) {
                        if m.target().dimension(gen_dim - t_shift) > 0 {
                            let num_gens = source.module.module.number_of_gens_in_degree(gen_dim);
                            for i in 0..num_gens {
                                if !m.output(gen_dim, i).is_zero() {
                                    continue 'gen_loop;
                                }
                            }
                        }
                    }
                }

                for t in gen_dim..=t_max {
                    let orig_diff_dimension = differential_images[t].dimension();
                    if orig_diff_dimension == 0 {
                        continue;
                    }

                    prev_differentials[t] = Some(differential_images[t].clone());
                    prev_subspaces[t] = Some(source.subspaces[t].clone());
                    prev_basis_list[t] = Some(source.basis_list[t].clone());

                    let start = source.module.module.generator_offset(t, gen_dim, 0);
                    let dim = source.module.module.dimension(t);

                    let end = if degrees_with_gens.get(i + 1).map_or(true, |&t_| t_ > t) {
                        dim
                    } else {
                        source
                            .module
                            .module
                            .generator_offset(t, degrees_with_gens[i + 1], 0)
                    };

                    let source_orig_dim = source.dimension(t);
                    source.quotient_basis_elements(t, start..end);
                    dim_drop[t] = Some(source_orig_dim - source.dimension(t));

                    for row in differential_images[t].matrix.iter_mut() {
                        if row.is_zero() {
                            break;
                        }
                        source.reduce(t, row.as_slice_mut());
                        if row.is_zero() {
                            for t in gen_dim..=t {
                                if prev_differentials[t].is_none() {
                                    continue;
                                }
                                differential_images[t] = prev_differentials[t].take().unwrap();
                                source.subspaces[t] = prev_subspaces[t].take().unwrap();
                                source.basis_list[t] = prev_basis_list[t].take().unwrap();
                                dim_drop[t] = None;
                            }
                            continue 'gen_loop;
                        }
                    }

                    differential_images[t].row_reduce();
                    if orig_diff_dimension != differential_images[t].dimension() {
                        for t in gen_dim..=t {
                            if prev_differentials[t].is_none() {
                                continue;
                            }
                            differential_images[t] = prev_differentials[t].take().unwrap();
                            source.subspaces[t] = prev_subspaces[t].take().unwrap();
                            source.basis_list[t] = prev_basis_list[t].take().unwrap();
                            dim_drop[t] = None;
                        }
                        continue 'gen_loop;
                    }
                }

                // We are free to clear this basis element. Do it
                for t in gen_dim..=t_max {
                    let start = source.module.module.generator_offset(t, gen_dim, 0);

                    let end = if degrees_with_gens.get(i + 1).map_or(true, |&t_| t_ > t) {
                        source.module.module.dimension(t)
                    } else {
                        source
                            .module
                            .module
                            .generator_offset(t, degrees_with_gens[i + 1], 0)
                    };

                    if prev_differentials[t].is_none() {
                        // We previously skipped this because there were no differentials to check

                        let source_orig_dim = source.dimension(t);
                        source.quotient_basis_elements(t, start..end);
                        dim_drop[t] = Some(source_orig_dim - source.dimension(t));
                    } else {
                        prev_differentials[t] = None;
                        prev_subspaces[t] = None;
                        prev_basis_list[t] = None;
                    }

                    let mut target_kills: Vec<FpVector> = Vec::with_capacity(end - start);
                    let target_dim = target.module.dimension(t);

                    let target_orig_dim = target.dimension(t);
                    for i in start..end {
                        let mut result = FpVector::new(p, target_dim);
                        d.apply_to_basis_element(result.as_slice_mut(), 1, t, i);
                        target_kills.push(result);
                    }
                    target.quotient_vectors(t, target_kills);
                    assert_eq!(
                        target.dimension(t),
                        target_orig_dim - dim_drop[t].take().unwrap()
                    );
                }
            }
        }

        for t in (t_min..=t_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }
            if cc.module(s).dimension(t) == 0 {
                continue;
            }

            // Now compute the image of the differentials, if any.
            let keep: Option<&Subspace> = if s < s_max {
                Some(&differential_images[t])
            } else {
                None
            };

            let augmentation_map = if s < target_cc.max_s() && target_cc.module(s).dimension(t) > 0
            {
                Some(cc.chain_map(s))
            } else {
                None
            };
            let preserve_map = if s >= s_shift && t >= t_shift {
                match map.chain_maps.get((s - s_shift) as usize) {
                    Some(m) => {
                        if m.target().dimension(t - t_shift) > 0 {
                            Some(m)
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            } else {
                None
            };

            // We can only quotient out by things in the kernel of the augmentation maps *and* the
            // steenrod operations. Moreover, the space we quotient out must be complementary to
            // the image of the differentials. The function computes the kernel and a list of
            // elements that span the image of the differentials.
            let (mut matrix, mut images) =
                compute_kernel_image(source, augmentation_map, preserve_map, keep, t);

            matrix.row_reduce();

            let subspace = &source.subspaces[t];
            let mut pivot_columns: Vec<(i32, usize)> = matrix
                .pivots()
                .iter()
                .enumerate()
                .filter(|&(_i, &v)| v >= 0)
                .map(|(i, _v)| (strategy(&*source.module.module, subspace, t, i), i))
                .collect::<Vec<_>>();
            pivot_columns.sort_unstable();

            let image_pivots =
                images.find_pivots_permutation(pivot_columns.iter().map(|(_p, i)| *i));

            let mut chosen_cols: HashSet<usize> = HashSet::default();

            for image in image_pivots {
                chosen_cols.insert(image);
            }

            let mut pivot_columns = pivot_columns.iter().map(|(_p, i)| i).collect::<Vec<_>>();
            pivot_columns.sort();

            let mut matrix: Vec<FpVector> = matrix.into();
            let mut source_kills: Vec<FpVector> = Vec::with_capacity(source.module.dimension(t));
            let mut target_kills: Vec<FpVector> = Vec::with_capacity(target.module.dimension(t));

            for col in pivot_columns.into_iter().rev() {
                let source_row = matrix.pop().unwrap();
                if chosen_cols.contains(col) {
                    continue;
                }

                let mut target_row = FpVector::new(p, target.module.dimension(t));
                d.apply(target_row.as_slice_mut(), 1, t, source_row.as_slice());

                source_kills.push(source_row);
                target_kills.push(target_row);
            }
            let goal_s_dim = if s == s_max {
                0
            } else {
                source.dimension(t) - source_kills.len()
            };
            let goal_t_dim = if s == s_max {
                0
            } else {
                target.dimension(t) - target_kills.len()
            };

            source.quotient_vectors(t, source_kills);
            target.quotient_vectors(t, target_kills);

            if s != s_max {
                assert_eq!(
                    source.dimension(t),
                    goal_s_dim,
                    "Failed s dimension check at (s, t) = ({}, {})",
                    s,
                    t
                );
                assert_eq!(
                    target.dimension(t),
                    goal_t_dim,
                    "Failed t dimension check at (s, t) = ({}, {})",
                    s,
                    t
                );
            }
        }
    }

    let zero_module = Arc::new(QM::new(Arc::new(TM::new(cc.zero_module(), t_max))));
    zero_module.compute_basis(t_max);
    let zero_module_fd = Arc::new(FiniteModule::FDModule(zero_module.to_fd_module()));

    let modules_fd = modules
        .iter()
        .map(|m| Arc::new(FiniteModule::FDModule(m.to_fd_module())))
        .collect::<Vec<_>>();
    let modules = modules.into_iter().map(Arc::new).collect::<Vec<_>>();

    let zero_differential = {
        let f = cc.differential(0);
        let tf = Arc::new(TruncatedHomomorphism::new(
            f,
            Arc::clone(&modules[0].module),
            Arc::clone(&zero_module.module),
        ));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphism::new(
            tf,
            Arc::clone(&modules[0]),
            Arc::clone(&zero_module),
        ));
        Arc::new(FiniteModuleHomomorphism::from(
            qf.replace_source(Arc::clone(&modules_fd[0]))
                .replace_target(Arc::clone(&zero_module_fd)),
        ))
    };

    let mut differentials = vec![zero_differential];
    differentials.extend((0..s_max).map(|s| {
        let f = cc.differential(s + 1);
        let s = s as usize;
        let tf = Arc::new(TruncatedHomomorphism::new(
            f,
            Arc::clone(&modules[s + 1].module),
            Arc::clone(&modules[s].module),
        ));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphism::new(
            tf,
            Arc::clone(&modules[s + 1]),
            Arc::clone(&modules[s]),
        ));
        Arc::new(FiniteModuleHomomorphism::from(
            qf.replace_source(Arc::clone(&modules_fd[s + 1]))
                .replace_target(Arc::clone(&modules_fd[s])),
        ))
    }));
    differentials.push(Arc::new(FiniteModuleHomomorphism::from(
        BoundedModuleHomomorphism::zero_homomorphism(
            Arc::clone(&zero_module_fd),
            Arc::clone(&modules_fd[s_max as usize]),
            0,
        ),
    )));
    differentials.push(Arc::new(FiniteModuleHomomorphism::from(
        BoundedModuleHomomorphism::zero_homomorphism(
            Arc::clone(&zero_module_fd),
            Arc::clone(&zero_module_fd),
            0,
        ),
    )));

    let chain_maps = (0..=s_max)
        .map(|s| {
            let f = cc.chain_map(s);
            let s = s as usize;
            let target = f.target();
            let tf = Arc::new(TruncatedHomomorphismSource::new(
                f,
                Arc::clone(&modules[s].module),
                Arc::clone(&target),
            ));
            let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphismSource::new(
                tf,
                Arc::clone(&modules[s]),
                target,
            ));
            Arc::new(FiniteModuleHomomorphism::from(
                qf.replace_source(Arc::clone(&modules_fd[s])),
            ))
        })
        .collect::<Vec<_>>();

    FiniteAugmentedChainComplex {
        modules: modules_fd,
        zero_module: zero_module_fd,
        differentials,
        target_cc: cc.target(),
        chain_maps,
    }
}

/// This function does the following computation:
///
/// Given the source module `source` and a subspace `keep`, the function returns the subspace of all
/// elements in `source` of degree `t` that are killed by all non-trivial actions of the algebra,
/// followed by a list of elements that span the intersection between this subspace and `keep`.
///
/// If `keep` is `None`, it is interpreted as the empty subspace.

fn compute_kernel_image<M: BoundedModule, F: ModuleHomomorphism, G: ModuleHomomorphism>(
    source: &QM<M>,
    augmentation_map: Option<Arc<F>>,
    preserve_map: Option<&G>,
    keep: Option<&Subspace>,
    t: i32,
) -> (Matrix, Matrix)
where
    M::Algebra: GeneratedAlgebra,
{
    let algebra = source.algebra();
    let p = algebra.prime();

    let mut generators: Vec<(i32, usize)> = Vec::new();
    let mut target_degrees = Vec::new();
    let mut padded_target_degrees: Vec<usize> = Vec::new();

    let source_orig_dimension = source.module.dimension(t);
    let source_dimension = source.dimension(t);

    for op_deg in 1..=source.max_degree() - t {
        for op_idx in algebra.generators(op_deg) {
            generators.push((op_deg, op_idx));
            target_degrees.push(source.module.dimension(t + op_deg));
            padded_target_degrees
                .push(FpVector::padded_len(p, source.module.dimension(t + op_deg)));
        }
    }

    if let Some(m) = &augmentation_map {
        target_degrees.push(m.target().dimension(t));
        padded_target_degrees.push(FpVector::padded_len(p, m.target().dimension(t)));
    }

    if let Some(m) = &preserve_map {
        let dim = m.target().dimension(t - m.degree_shift());
        target_degrees.push(dim);
        padded_target_degrees.push(FpVector::padded_len(p, dim));
    }

    let total_padded_degree: usize = padded_target_degrees.iter().sum();

    let padded_source_degree: usize = FpVector::padded_len(p, source_orig_dimension);
    let total_cols: usize = total_padded_degree + padded_source_degree + source_orig_dimension;

    let mut matrix_rows: Vec<FpVector> = Vec::with_capacity(source_dimension);

    let mut projection_off_keep = FpVector::new(p, source_orig_dimension);

    for i in 0..source_dimension {
        let mut result = FpVector::new(p, total_cols);

        let i = source.basis_list[t][i];
        let mut offset = 0;

        let mut target_idx = 0;
        for (op_deg, op_idx) in &generators {
            source.act_on_original_basis(
                result.slice_mut(offset, offset + target_degrees[target_idx]),
                1,
                *op_deg,
                *op_idx,
                t,
                i,
            );
            offset += padded_target_degrees[target_idx];
            target_idx += 1;
        }

        if let Some(m) = &augmentation_map {
            m.apply_to_basis_element(
                result.slice_mut(offset, offset + target_degrees[target_idx]),
                1,
                t,
                i,
            );
            offset += padded_target_degrees[target_idx];
            target_idx += 1;
        }

        if let Some(m) = &preserve_map {
            m.apply_to_basis_element(
                result.slice_mut(offset, offset + target_degrees[target_idx]),
                1,
                t,
                i,
            );
            offset += padded_target_degrees[target_idx];
        }

        if let Some(keep) = &keep {
            projection_off_keep.set_to_zero();
            projection_off_keep.set_entry(i, 1);
            keep.reduce(projection_off_keep.as_slice_mut());
            result
                .slice_mut(offset, offset + source_orig_dimension)
                .assign(projection_off_keep.as_slice());
        } else {
            result.set_entry(offset + i, 1);
        }

        result.set_entry(padded_source_degree + total_padded_degree + i, 1);
        matrix_rows.push(result);
    }
    let mut matrix = Matrix::from_rows(p, matrix_rows, total_cols);
    matrix.row_reduce();

    let first_kernel_row = match &matrix.pivots()[0..total_padded_degree]
        .iter()
        .rposition(|&i| i >= 0)
    {
        Some(n) => matrix.pivots()[*n] as usize + 1,
        None => 0,
    };
    let first_image_row = match &matrix.pivots()
        [total_padded_degree..total_padded_degree + source_orig_dimension]
        .iter()
        .rposition(|&i| i >= 0)
    {
        Some(n) => matrix.pivots()[*n + total_padded_degree] as usize + 1,
        None => first_kernel_row,
    };

    matrix.trim(
        first_kernel_row,
        source_dimension,
        total_padded_degree + padded_source_degree,
    );

    let first_image_row = first_image_row - first_kernel_row;

    let mut images = Vec::with_capacity(matrix.rows() - first_image_row);
    for i in first_image_row..matrix.rows() {
        images.push(matrix[i].clone());
    }
    let image_matrix = Matrix::from_rows(p, images, source_orig_dimension);
    (matrix, image_matrix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_complex::FiniteChainComplex;
    use crate::resolution::Resolution;
    use crate::resolution_homomorphism::ResolutionHomomorphism;
    use crate::CCC;
    use algebra::module::homomorphism::IdentityHomomorphism;

    use fp::prime::ValidPrime;

    #[test]
    fn test() {
        let algebra = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(
            ValidPrime::new(2),
            false,
            false,
            false,
        )));
        let module = Arc::new(FiniteModule::from(FDModule::new(
            algebra,
            "".to_string(),
            BiVec::from_vec(0, vec![1]),
        )));
        let chain_complex: Arc<CCC> = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));
        let resolution = Arc::new(Resolution::new(chain_complex));

        let x: i32 = 30;
        let s: u32 = 6;
        let idx: usize = 0;

        let t = s as i32 + x;
        resolution.compute_through_bidegree(s, t);

        let yoneda = Arc::new(yoneda_representative_element(
            Arc::clone(&resolution),
            s,
            t,
            idx,
        ));

        let f = ResolutionHomomorphism::from_module_homomorphism(
            "".to_string(),
            Arc::clone(&resolution),
            Arc::clone(&yoneda),
            &FiniteModuleHomomorphism::identity_homomorphism(Arc::clone(&module)),
        );

        f.extend(s, t);
        let final_map = f.get_map(s);
        let num_gens = resolution.number_of_gens_in_bidegree(s, t);
        for i_ in 0..num_gens {
            assert_eq!(final_map.output(t, i_).len(), 1);
            if i_ == idx {
                assert_eq!(final_map.output(t, i_).entry(0), 1);
            } else {
                assert_eq!(final_map.output(t, i_).entry(0), 0);
            }
        }

        let mut check = BiVec::from_vec(0, vec![0; t as usize + 1]);
        for s in 0..=s {
            let module = yoneda.module(s);

            for t in 0..=t {
                check[t] += (if s % 2 == 0 { 1 } else { -1 }) * module.dimension(t) as i32;
            }
        }
        for t in 0..=t {
            assert_eq!(
                check[t],
                module.dimension(t) as i32,
                "Incorrect Euler characteristic at t = {}",
                t
            );
        }
    }
}
//static mut MEMOIZED_SIZE : Option<once::OnceVec<Vec<u32>>> = None;
//unsafe fn compute_size(algebra : &Arc<SteenrodAlgebra>, deg : i32) {
//    if MEMOIZED_SIZE.is_none() {
//        MEMOIZED_SIZE = Some(once::OnceVec::new());
//    }
//    let p = algebra.prime();
//    if let Some(size) = &MEMOIZED_SIZE {
//        while size.len() <= deg as usize {
//            let t_max = size.len() as i32;
//            let dim = algebra.dimension(t_max, -1);
//
//            let mut new_sizes = Vec::with_capacity(dim);
//
//            for i in 0 .. dim {
//                let start_module = FreeModule::new(Arc::clone(algebra), "".to_string(), 0);
//                start_module.add_generators(0, 1, None);
//                start_module.extend_by_zero(t_max);
//
//                let mut test_module = QM::new(Arc::new(TM::new(Arc::new(start_module), t_max)));
//                test_module.compute_basis(t_max);
//
//                let mut basis_list : Vec<usize> = (0 .. dim).collect::<Vec<_>>();
//                basis_list.swap_remove(i);
//
//                test_module.quotient_basis_elements(t_max, basis_list);
//
//                for t in (0 .. t_max).rev() {
//                    let mut generators : Vec<(i32, usize)> = Vec::new();
//                    let mut target_degrees = Vec::new();
//                    let mut padded_target_degrees : Vec<usize> = Vec::new();
//
//                    let cur_dimension = test_module.dimension(t);
//                    for op_deg in 1 ..= t_max - t {
//                        for op_idx in algebra.generators(op_deg) {
//                            generators.push((op_deg, op_idx));
//                            target_degrees.push(test_module.module.dimension(t + op_deg));
//                            padded_target_degrees.push(FpVector::padded_dimension(p, test_module.module.dimension(t + op_deg)));
//                        }
//                    }
//
//                    let total_padded_degree : usize = padded_target_degrees.iter().sum();
//                    let total_cols : usize = total_padded_degree + cur_dimension;
//
//                    let mut matrix_rows : Vec<FpVector> = Vec::with_capacity(cur_dimension);
//
//                    for j in 0 .. cur_dimension {
//                        let mut result = FpVector::new(p, total_cols);
//
//                        let mut offset = 0;
//
//                        for (gen_idx, (op_deg, op_idx)) in generators.iter().enumerate() {
//                            result.set_slice(offset, offset + target_degrees[gen_idx]);
//                            test_module.act_on_original_basis(&mut result, 1, *op_deg, *op_idx, t, j);
//                            result.clear_slice();
//                            offset += padded_target_degrees[gen_idx];
//                        }
//                        result.set_entry(total_padded_degree + j, 1);
//                        matrix_rows.push(result);
//                    }
//
//                    let mut matrix = Matrix::from_rows(p, matrix_rows, total_cols);
//                    let mut pivots = vec![-1; total_cols];
//                    matrix.row_reduce(&mut pivots);
//
//                    let first_kernel_row = match &pivots[0..total_padded_degree].iter().rposition(|&i| i >= 0) {
//                        Some(n) => pivots[*n] as usize + 1,
//                        None => 0
//                    };
//
//                    matrix.set_slice(first_kernel_row, cur_dimension, total_padded_degree, total_cols);
//                    matrix.into_slice();
//
//                    let kill_rows = matrix.into_vec();
//
//                    test_module.quotient_vectors(t, kill_rows);
//                }
//                new_sizes.push(test_module.total_dimension() as u32);
//            }
//            size.push(new_sizes);
//        }
//    }
//}
