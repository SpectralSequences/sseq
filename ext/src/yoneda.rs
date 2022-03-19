use crate::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, ChainMap,
    FiniteAugmentedChainComplex, FiniteChainComplex, FreeChainComplex,
};
use algebra::module::homomorphism::{
    FreeModuleHomomorphism, FullModuleHomomorphism, ModuleHomomorphism,
};
use algebra::module::homomorphism::{QuotientHomomorphism, QuotientHomomorphismSource};
use algebra::module::QuotientModule as QM;
use algebra::module::{FDModule, FreeModule, Module};
use algebra::{AdemAlgebra, Algebra, GeneratedAlgebra, MilnorAlgebra, SteenrodAlgebra};

use fp::matrix::{Matrix, Subspace};
use fp::vector::FpVector;

use bivec::BiVec;

use rustc_hash::FxHashSet as HashSet;
use std::sync::Arc;

const PENALTY_UNIT: i32 = 10000;

pub type Yoneda<CC> = FiniteAugmentedChainComplex<
    FDModule<<CC as ChainComplex>::Algebra>,
    FullModuleHomomorphism<FDModule<<CC as ChainComplex>::Algebra>>,
    FullModuleHomomorphism<
        FDModule<<CC as ChainComplex>::Algebra>,
        <<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
    >,
    <CC as AugmentedChainComplex>::TargetComplex,
>;

fn rate_operation<A: Algebra>(algebra: &Arc<A>, op_deg: i32, op_idx: usize) -> i32 {
    let algebra = &**algebra as &dyn std::any::Any;

    if let Some(algebra) = algebra.downcast_ref::<SteenrodAlgebra>() {
        match algebra {
            SteenrodAlgebra::AdemAlgebra(a) => rate_adem_operation(a, op_deg, op_idx),
            SteenrodAlgebra::MilnorAlgebra(a) => rate_milnor_operation(a, op_deg, op_idx),
        }
    } else if let Some(algebra) = algebra.downcast_ref::<MilnorAlgebra>() {
        rate_milnor_operation(algebra, op_deg, op_idx)
    } else if let Some(algebra) = algebra.downcast_ref::<AdemAlgebra>() {
        rate_adem_operation(algebra, op_deg, op_idx)
    } else {
        0
    }
}

fn rate_milnor_operation(algebra: &MilnorAlgebra, deg: i32, idx: usize) -> i32 {
    let elt = algebra.basis_element_from_index(deg, idx);
    elt.p_part
        .iter()
        .enumerate()
        .map(|(i, &r)| r.count_ones() << i)
        .sum::<u32>() as i32
}

fn rate_adem_operation(algebra: &AdemAlgebra, deg: i32, idx: usize) -> i32 {
    let elt = algebra.basis_element_from_index(deg, idx);
    elt.ps.iter().map(|&r| r.count_ones()).sum::<u32>() as i32
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

fn split_mut_borrow<T>(v: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative_element<CC>(cc: Arc<CC>, s: u32, t: i32, idx: usize) -> Yoneda<CC>
where
    CC: FreeChainComplex
        + AugmentedChainComplex<
            ChainMap = FreeModuleHomomorphism<
                <<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
            >,
        >,
    CC::TargetComplex: BoundedChainComplex,
    CC::Algebra: GeneratedAlgebra,
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
pub fn yoneda_representative<CC>(
    cc: Arc<CC>,
    map: ChainMap<FreeModuleHomomorphism<impl Module<Algebra = CC::Algebra>>>,
) -> Yoneda<CC>
where
    CC: FreeChainComplex
        + AugmentedChainComplex<
            ChainMap = FreeModuleHomomorphism<
                <<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
            >,
        >,
    CC::TargetComplex: BoundedChainComplex,
    CC::Algebra: GeneratedAlgebra,
{
    yoneda_representative_with_strategy(
        cc,
        map,
        |module: &FreeModule<CC::Algebra>, subspace: &Subspace, t: i32, i: usize| {
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
pub fn yoneda_representative_with_strategy<CC>(
    cc: Arc<CC>,
    map: ChainMap<FreeModuleHomomorphism<impl Module<Algebra = CC::Algebra>>>,
    strategy: impl Fn(&CC::Module, &Subspace, i32, usize) -> i32,
) -> Yoneda<CC>
where
    CC: FreeChainComplex
        + AugmentedChainComplex<
            ChainMap = FreeModuleHomomorphism<
                <<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module,
            >,
        >,
    CC::TargetComplex: BoundedChainComplex,
    CC::Algebra: GeneratedAlgebra,
{
    let p = cc.prime();
    let target_cc = cc.target();
    let algebra = cc.algebra();

    let t_shift: i32 = map.chain_maps[0].degree_shift();
    let s_shift: u32 = map.s_shift;

    let s_max = std::cmp::max(target_cc.max_s(), map.s_shift + map.chain_maps.len() as u32) - 1;
    let t_max = std::cmp::max(
        (0..target_cc.max_s())
            .map(|i| target_cc.module(i).max_degree().unwrap())
            .max()
            .unwrap_or_else(|| target_cc.min_degree()),
        map.chain_maps[0].degree_shift()
            + map
                .chain_maps
                .iter()
                .map(|m| m.target().max_degree().unwrap())
                .max()
                .unwrap(),
    );

    let t_min = cc.min_degree();

    let mut modules = (0..=s_max)
        .map(|s| QM::new(cc.module(s), t_max))
        .collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    for s in (1..=s_max).rev() {
        let mut differential_images: BiVec<Subspace> = {
            let mut result = BiVec::new(t_min);

            if s < s_max {
                let prev = &modules[s as usize + 1];
                let curr = &modules[s as usize];
                let d = cc.differential(s + 1);

                result.extend_with(t_max, |t| {
                    let mut differentials =
                        Matrix::new(p, prev.dimension(t), curr.module.dimension(t));

                    for (i, row) in differentials.iter_mut().enumerate() {
                        let j = prev.basis_list[t][i];
                        d.apply_to_basis_element(row.as_slice_mut(), 1, t, j);
                        curr.reduce(t, row.as_slice_mut());
                    }

                    differentials.row_reduce();

                    let mut result = Subspace {
                        matrix: differentials,
                    };
                    let dim = result.dimension();
                    result.trim(0, dim, 0);
                    result
                });
            }
            result
        };

        let (target, source) = split_mut_borrow(&mut modules, s as usize - 1, s as usize);
        let d = cc.differential(s);

        // This is used for sanity checking.
        let dim_diff: BiVec<isize> = {
            let mut dim_diff = BiVec::new(t_min);
            dim_diff.extend_with(t_max, |t| {
                source.dimension(t) as isize - target.dimension(t) as isize
            });
            dim_diff
        };

        macro_rules! check {
            ($t:ident) => {
                assert_eq!(
                    source.dimension($t) as isize - target.dimension($t) as isize,
                    dim_diff[$t],
                    "Failed dimension check at (s, t) = ({s}, {t})",
                    t = $t
                );
            };
        }

        // First, we try to kill off whole generators
        if s < s_max {
            let mut prev_differentials: BiVec<Option<Subspace>> =
                BiVec::with_capacity(t_min, t_max + 1);
            let mut prev_subspaces: BiVec<Option<Subspace>> =
                BiVec::with_capacity(t_min, t_max + 1);
            let mut prev_basis_list: BiVec<Option<Vec<usize>>> =
                BiVec::with_capacity(t_min, t_max + 1);

            for _ in t_min..=t_max {
                prev_differentials.push(None);
                prev_subspaces.push(None);
                prev_basis_list.push(None);
            }

            // Collect for borrow checker purposes
            let source_gens: Vec<_> = source.module.iter_gens(t_max).collect();
            'gen_loop: for (gen_deg, gen_idx) in source_gens {
                // Check if augmentation map is non-zero on the generator
                if s < target_cc.max_s() && target_cc.module(s).dimension(gen_deg) > 0 {
                    let m = cc.chain_map(s);
                    if !m.output(gen_deg, gen_idx).is_zero() {
                        continue 'gen_loop;
                    }
                }

                // Check if preserve map is non-zero on the generator
                if s >= s_shift && gen_deg >= t_shift {
                    if let Some(m) = map.chain_maps.get((s - s_shift) as usize) {
                        if m.target().dimension(gen_deg - t_shift) > 0
                            && !m.output(gen_deg, gen_idx).is_zero()
                        {
                            continue 'gen_loop;
                        }
                    }
                }

                for t in gen_deg..=t_max {
                    let diff_im = &mut differential_images[t];
                    if diff_im.is_empty() {
                        continue;
                    }

                    prev_differentials[t] = Some(diff_im.clone());
                    prev_subspaces[t] = Some(source.subspaces[t].clone());
                    prev_basis_list[t] = Some(source.basis_list[t].clone());

                    let start = source.module.generator_offset(t, gen_deg, gen_idx);
                    let end = start + algebra.dimension(t - gen_deg);

                    source.quotient_basis_elements(t, start..end);

                    macro_rules! revert {
                        () => {
                            for t in gen_deg..=t {
                                if prev_differentials[t].is_none() {
                                    continue;
                                }
                                differential_images[t] = prev_differentials[t].take().unwrap();
                                source.subspaces[t] = prev_subspaces[t].take().unwrap();
                                source.basis_list[t] = prev_basis_list[t].take().unwrap();
                            }
                            continue 'gen_loop;
                        };
                    }

                    for row in diff_im.matrix.iter_mut() {
                        source.reduce(t, row.as_slice_mut());
                        if row.is_zero() {
                            revert!();
                        }
                    }

                    diff_im.row_reduce();
                    if diff_im.row(diff_im.rows() - 1).is_zero() {
                        revert!();
                    }
                }

                // We are free to clear this basis element. Do it
                for t in gen_deg..=t_max {
                    let start = source.module.generator_offset(t, gen_deg, gen_idx);
                    let end = start + algebra.dimension(t - gen_deg);

                    if prev_differentials[t].is_none() {
                        // We previously skipped this because there were no differentials to check
                        source.quotient_basis_elements(t, start..end);
                    } else {
                        prev_differentials[t] = None;
                        prev_subspaces[t] = None;
                        prev_basis_list[t] = None;
                    }

                    let mut indices = start..end;
                    target.quotient_vectors(t, |row| {
                        d.apply_to_basis_element(row, 1, t, indices.next()?);
                        Some(())
                    });
                    check!(t);
                }
            }
        }

        // Now we clean up the module degree by degree from above.
        for t in (t_min..=t_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }
            if source.dimension(t) == 0 {
                continue;
            }

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
                map.chain_maps.get((s - s_shift) as usize).and_then(|m| {
                    if m.target().dimension(t - t_shift) > 0 {
                        Some(m)
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            // We can only quotient out by things in the kernel of the augmentation maps *and* the
            // steenrod operations. Moreover, the space we quotient out must be complementary to
            // the image of the differentials. The function computes the kernel and a list of
            // elements that span the image of the differentials.
            let (mut matrix, mut images) =
                compute_kernel_image(source, augmentation_map.as_deref(), preserve_map, keep, t);

            matrix.row_reduce();

            let subspace = &source.subspaces[t];
            let mut pivot_columns: Vec<(i32, usize)> = matrix
                .pivots()
                .iter()
                .enumerate()
                .filter(|&(_, &v)| v >= 0)
                .map(|(i, _)| (strategy(&*source.module, subspace, t, i), i))
                .collect::<Vec<_>>();
            pivot_columns.sort_unstable();

            let chosen_cols: HashSet<usize> = images
                .find_pivots_permutation(pivot_columns.iter().map(|(_, i)| *i))
                .into_iter()
                .collect();

            let mut pivot_columns = pivot_columns.iter().map(|(_, i)| i).collect::<Vec<_>>();
            pivot_columns.sort();

            let mut source_iter =
                std::iter::zip(&matrix, pivot_columns.iter()).filter_map(|(row, col)| {
                    if chosen_cols.contains(col) {
                        None
                    } else {
                        Some(row.as_slice())
                    }
                });

            let mut source_iter2 = source_iter.clone();
            source.quotient_vectors(t, |mut row| {
                row.add(source_iter.next()?, 1);
                Some(())
            });

            target.quotient_vectors(t, |row| {
                d.apply(row, 1, t, source_iter2.next()?);
                Some(())
            });

            if s != s_max {
                check!(t);
            }
        }
    }

    let modules = modules.into_iter().map(Arc::new).collect::<Vec<_>>();

    let differentials: Vec<_> = (0..s_max)
        .map(|s| {
            Arc::new(FullModuleHomomorphism::from(&QuotientHomomorphism::new(
                cc.differential(s + 1),
                Arc::clone(&modules[s as usize + 1]),
                Arc::clone(&modules[s as usize]),
            )))
        })
        .collect();

    let chain_maps = (0..=s_max)
        .map(|s| {
            Arc::new(FullModuleHomomorphism::from(
                &QuotientHomomorphismSource::new(cc.chain_map(s), Arc::clone(&modules[s as usize])),
            ))
        })
        .collect::<Vec<_>>();

    let yoneda_rep =
        FiniteChainComplex::new(modules, differentials).augment(cc.target(), chain_maps);

    yoneda_rep.map(|m| FDModule::from(m))
}

/// This function does the following computation:
///
/// Given the source module `source` and a subspace `keep`, the function returns the subspace of all
/// elements in `source` of degree `t` that are killed by all non-trivial actions of the algebra,
/// followed by a list of elements that span the intersection between this subspace and `keep`.
///
/// If `keep` is `None`, it is interpreted as the empty subspace.

fn compute_kernel_image<M: Module, F: ModuleHomomorphism, G: ModuleHomomorphism>(
    source: &QM<M>,
    augmentation_map: Option<&F>,
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

    for op_deg in 1..=source.max_degree().unwrap() - t {
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
