use std::sync::Arc;

use algebra::{
    module::{
        homomorphism::{
            FreeModuleHomomorphism, FullModuleHomomorphism, IdentityHomomorphism,
            ModuleHomomorphism, QuotientHomomorphism, QuotientHomomorphismSource,
        },
        FDModule, FreeModule, Module, QuotientModule as QM,
    },
    AdemAlgebra, Algebra, GeneratedAlgebra, MilnorAlgebra, SteenrodAlgebra,
};
use bivec::BiVec;
use fp::{
    matrix::{AugmentedMatrix, Matrix, Subspace},
    vector::FpVector,
};
use rustc_hash::FxHashSet as HashSet;
use sseq::coordinates::Bidegree;

use crate::{
    chain_complex::{
        AugmentedChainComplex, BoundedChainComplex, ChainComplex, ChainMap,
        FiniteAugmentedChainComplex, FiniteChainComplex, FreeChainComplex,
    },
    resolution_homomorphism::ResolutionHomomorphism,
};

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

fn split_mut_borrow<T>(v: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative_element<CC>(cc: Arc<CC>, b: Bidegree, class: &[u32]) -> Yoneda<CC>
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
    let map = FreeModuleHomomorphism::new(cc.module(b.s()), Arc::new(target), b.t());
    let mut rows = vec![FpVector::new(p, 1); cc.number_of_gens_in_bidegree(b)];
    for (&i, row) in std::iter::zip(class, &mut rows) {
        row.set_entry(0, i);
    }

    map.add_generators_from_rows(b.t(), rows);

    let cm = ChainMap {
        s_shift: b.s(),
        chain_maps: vec![map],
    };
    let yoneda = Arc::new(yoneda_representative(Arc::clone(&cc), cm));

    // We now do some safety checks
    let module = cc.target().module(0);

    for t in cc.min_degree()..=b.t() {
        assert_eq!(
            yoneda.euler_characteristic(t),
            module.dimension(t) as isize,
            "Incorrect Euler characteristic at t = {t}",
        );
    }

    let f = ResolutionHomomorphism::from_module_homomorphism(
        "".to_string(),
        Arc::clone(&cc),
        Arc::clone(&yoneda),
        &FullModuleHomomorphism::identity_homomorphism(module),
    );

    f.extend_through_stem(b);
    let final_map = f.get_map(b.s());
    for (i, &v) in class.iter().enumerate() {
        assert_eq!(final_map.output(b.t(), i).len(), 1);
        assert_eq!(final_map.output(b.t(), i).entry(0), v);
    }

    drop(f);
    Arc::try_unwrap(yoneda).unwrap_or_else(|_| unreachable!())
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

            for row in subspace.iter() {
                if row.entry(i) != 0 {
                    pref += PENALTY_UNIT;
                }
            }
            pref
        },
    )
}

#[tracing::instrument(skip_all)]
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

    let t_max = {
        // The maximum t required by the augmentation maps
        let t_max_aug: Vec<i32> = (0..=s_max)
            .map(|s| {
                let mut t_max = cc.min_degree();
                if s < target_cc.max_s() {
                    t_max = std::cmp::max(t_max, target_cc.module(s).max_degree().unwrap())
                }
                if s >= map.s_shift {
                    if let Some(f) = map.chain_maps.get((s - map.s_shift) as usize) {
                        t_max = std::cmp::max(
                            t_max,
                            f.degree_shift() + f.target().max_degree().unwrap(),
                        );
                    }
                }
                t_max
            })
            .collect();

        let mut t_max = vec![cc.min_degree(); s_max as usize + 1];
        for s in (0..=s_max as usize).rev() {
            t_max[s] = t_max_aug[s];
            if s < s_max as usize {
                // the differential of the classes required to exist by the augmentation
                t_max[s] = std::cmp::max(t_max[s], t_max_aug[s + 1]);

                // The rest of the contents of t_max[s + 1] arise from the images of differentials,
                // which have zero differential. So we can subtract one.
                t_max[s] = std::cmp::max(t_max[s], t_max[s + 1] - 1);
            }
        }
        t_max
    };

    let t_min = cc.min_degree();

    let mut modules = (0..=s_max)
        .map(|s| QM::new(cc.module(s), t_max[s as usize]))
        .collect::<Vec<_>>();

    for s in (1..=s_max).rev() {
        let span = tracing::info_span!("Cleaning yoneda representative", s);
        let _tracing_guard = span.enter();
        let t_max = t_max[s as usize];
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

                    let mut result = Subspace::from_matrix(differentials);

                    let dim = result.dimension();
                    result.update_then_row_reduce(|result_matrix| result_matrix.trim(0, dim, 0));
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
                if $t <= target.truncation {
                    assert_eq!(
                        source.dimension($t) as isize - target.dimension($t) as isize,
                        dim_diff[$t],
                        "Failed dimension check at (s, t) = ({s}, {t})",
                        t = $t
                    );
                }
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

                    let needs_to_revert = diff_im.update_then_row_reduce(|diff_im_matrix| {
                        for row in diff_im_matrix.iter_mut() {
                            source.reduce(t, row.as_slice_mut());
                            if row.as_slice().is_zero() {
                                return true;
                            }
                        }

                        diff_im_matrix.row_reduce();
                        if diff_im_matrix.row(diff_im_matrix.rows() - 1).is_zero() {
                            return true;
                        }
                        false
                    });

                    if needs_to_revert {
                        for t in gen_deg..=t {
                            if prev_differentials[t].is_none() {
                                continue;
                            }
                            differential_images[t] = prev_differentials[t].take().unwrap();
                            source.subspaces[t] = prev_subspaces[t].take().unwrap();
                            source.basis_list[t] = prev_basis_list[t].take().unwrap();
                        }
                        continue 'gen_loop;
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
                .filter_map(|(i, &v)| {
                    if v >= 0 {
                        Some((strategy(&source.module, subspace, t, i), i))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let orig_pivot_columns: Vec<usize> = pivot_columns.iter().map(|&(_, i)| i).collect();

            pivot_columns.sort_unstable();

            let chosen_cols: HashSet<usize> = HashSet::from_iter(
                images.find_pivots_permutation(pivot_columns.iter().map(|(_, i)| *i)),
            );

            let mut source_iter =
                std::iter::zip(&matrix, orig_pivot_columns.iter()).filter_map(|(row, col)| {
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
    let mut target_dims = Vec::new();

    let source_orig_dimension = source.module.dimension(t);
    let source_dimension = source.dimension(t);

    for op_deg in 1..=source.max_degree().unwrap() - t {
        for op_idx in algebra.generators(op_deg) {
            generators.push((op_deg, op_idx));
            target_dims.push(source.module.dimension(t + op_deg));
        }
    }

    if let Some(m) = &augmentation_map {
        target_dims.push(m.target().dimension(t));
    }

    if let Some(m) = &preserve_map {
        let dim = m.target().dimension(t - m.degree_shift());
        target_dims.push(dim);
    }

    let total_dimension: usize = target_dims.iter().sum();
    let mut matrix = AugmentedMatrix::new(
        p,
        source_dimension,
        [
            total_dimension + source_orig_dimension,
            source_orig_dimension,
        ],
    );

    for (row_idx, &i) in source.basis_list[t].iter().enumerate() {
        let mut offset = 0;
        let mut row = matrix.row_segment_mut(row_idx, 0, 0);

        let mut cols = target_dims.iter().copied();
        for (op_deg, op_idx) in &generators {
            let len = cols.next().unwrap();
            source.act_on_original_basis(
                row.slice_mut(offset, offset + len),
                1,
                *op_deg,
                *op_idx,
                t,
                i,
            );
            offset += len;
        }

        if let Some(m) = &augmentation_map {
            let len = cols.next().unwrap();
            m.apply_to_basis_element(row.slice_mut(offset, offset + len), 1, t, i);
            offset += len;
        }

        if let Some(m) = &preserve_map {
            let len = cols.next().unwrap();
            m.apply_to_basis_element(row.slice_mut(offset, offset + len), 1, t, i);
            offset += len;
        }

        let mut slice = row.slice_mut(offset, offset + source_orig_dimension);
        slice.set_entry(i, 1);
        if let Some(keep) = &keep {
            keep.reduce(slice);
        }

        let mut row = matrix.row_segment_mut(row_idx, 1, 1);
        row.set_entry(i, 1);
    }
    matrix.row_reduce();

    let first_kernel_row = matrix.find_first_row_in_block(total_dimension);
    let first_image_row = matrix.find_first_row_in_block(matrix.start[1]);
    let image_rows = matrix.rows() - first_image_row;

    let matrix = matrix.into_tail_segment(first_kernel_row, source_dimension, 1);

    let mut image_matrix = Matrix::new(p, image_rows, source_orig_dimension);
    for (target, source) in std::iter::zip(
        image_matrix.iter_mut(),
        matrix.iter().skip(first_image_row - first_kernel_row),
    ) {
        target.assign(source);
    }
    (matrix, image_matrix)
}
