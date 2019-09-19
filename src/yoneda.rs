use crate::algebra::{Algebra, AlgebraAny, AdemAlgebra};
use crate::chain_complex::{ChainComplex, AugmentedChainComplex, FiniteAugmentedChainComplex, BoundedChainComplex, ChainMap};
use crate::fp_vector::{FpVector, FpVectorT};
use crate::matrix::{Matrix, Subspace};
use crate::module::homomorphism::{ModuleHomomorphism, BoundedModuleHomomorphism, ZeroHomomorphism, FiniteModuleHomomorphism, FreeModuleHomomorphism};
use crate::module::homomorphism::{TruncatedHomomorphism, TruncatedHomomorphismSource, QuotientHomomorphism, QuotientHomomorphismSource};
use crate::module::{Module, FDModule, FreeModule, BoundedModule, FiniteModule};
use crate::module::{QuotientModule as QM, TruncatedModule as TM};

use bivec::BiVec;

use std::collections::HashSet;
use std::sync::Arc;

const PENALTY_UNIT : i32 = 10000;

pub type Yoneda<CC> = FiniteAugmentedChainComplex<
        FiniteModule,
        FiniteModuleHomomorphism<FiniteModule>,
        FiniteModuleHomomorphism<<<CC as AugmentedChainComplex>::TargetComplex as ChainComplex>::Module>,
        <CC as AugmentedChainComplex>::TargetComplex
    >;

fn rate_operation(algebra : &Arc<AlgebraAny>, op_deg : i32, op_idx : usize) -> i32 {
    let mut pref = 0;
    match &**algebra {
        AlgebraAny::AdemAlgebra(a) => pref += rate_adem_operation(a, op_deg, op_idx),
        _ => ()
    };
    pref
}

fn rate_adem_operation(algebra : &AdemAlgebra, deg : i32, idx: usize) -> i32 {
    if algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    let mut pref = 0;
    for i in elt.ps.iter() {
        let mut i = *i;
        while i != 0 {
            pref += (i & 1) as i32;
            i >>= 1;
        }
    }
    pref
}

fn operation_drop(algebra : &AdemAlgebra, deg : i32, idx: usize) -> i32 {
    if algebra.prime() != 2 {
        return 1;
    }
    let elt = algebra.basis_element_from_index(deg, idx);
    if elt.ps.len() == 0 {
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

fn split_mut_borrow<T> (v : &mut Vec<T>, i : usize, j : usize) -> (&mut T, &mut T) {
    assert!(i < j);
    let (first, second) = v.split_at_mut(j);
    (&mut first[i], &mut second[0])
}

pub fn yoneda_representative_element<TCM, TC, CC>(cc : Arc<CC>, s : u32, t : i32, idx : usize) -> Yoneda<CC>
where TCM : BoundedModule,
      TC : ChainComplex<Module=TCM> + BoundedChainComplex,
      CC : AugmentedChainComplex<TargetComplex=TC, Module=FreeModule> {
    let p = cc.prime();

    let target = FDModule::new(cc.algebra(), "".to_string(), BiVec::from_vec(0, vec![1]));
    let map = FreeModuleHomomorphism::new(cc.module(s), Arc::new(target), t);
    let mut new_output = Matrix::new(p, cc.module(s).number_of_gens_in_degree(t), 1);
    new_output[idx].set_entry(0, 1);

    let lock = map.lock();
    map.add_generators_from_matrix_rows(&lock, t, &mut new_output, 0, 0);
    drop(lock);

    let cm = ChainMap {
        source : Arc::clone(&cc),
        s_shift : s,
        chain_maps : vec![map]
    };
    yoneda_representative(cc, cm)
}

/// This function produces a quasi-isomorphic quotient of `cc` (as an augmented chain complex) that `map` factors through
pub fn yoneda_representative<TCM, TC, CC, CMM, CMF>(cc : Arc<CC>, map : ChainMap<CC, CMF>) -> Yoneda<CC>
where TCM : BoundedModule,
      TC : ChainComplex<Module=TCM> + BoundedChainComplex,
      CC : AugmentedChainComplex<TargetComplex=TC, Module=FreeModule>,
      CMM : BoundedModule,
      CMF : ModuleHomomorphism<Source=CC::Module, Target=CMM>
{
    yoneda_representative_with_strategy(cc, map,
        |module : &FreeModule, subspace : &Subspace, t : i32, i : usize| {
            let opgen = module.index_to_op_gen(t, i);

            let mut pref = rate_operation(&module.algebra(), opgen.operation_degree, opgen.operation_index);

            for k in 0 .. subspace.matrix.rows() {
                if subspace.matrix[k].entry(i) != 0 {
                    pref += PENALTY_UNIT;
                }
            }
            pref
        })
}

pub fn yoneda_representative_with_strategy<TCM, TC, CC, CMM, CMF, F>(cc : Arc<CC>, map : ChainMap<CC, CMF>, strategy : F) -> Yoneda<CC>
where TCM : BoundedModule,
      TC : ChainComplex<Module=TCM> + BoundedChainComplex,
      CC : AugmentedChainComplex<TargetComplex=TC>,
      CMM : BoundedModule,
      CMF : ModuleHomomorphism<Source=CC::Module, Target=CMM>,
      F : Fn(&CC::Module, &Subspace, i32, usize) -> i32 {
    let p = cc.prime();
    let algebra = cc.algebra();
    let target_cc = cc.target();

    let t_shift : i32 = map.chain_maps[0].degree_shift();
    let s_shift : u32 = map.s_shift;

    let s_max = std::cmp::max(target_cc.max_s(), map.s_shift + map.chain_maps.len() as u32) - 1;
    let t_max = std::cmp::max(
        (0 .. target_cc.max_s()).map(|i| target_cc.module(i).max_degree()).max().unwrap_or(target_cc.min_degree()),
        map.chain_maps[0].degree_shift() + map.chain_maps.iter().map(|m| m.target().max_degree()).max().unwrap()
    );

    let mut modules = (0 ..= s_max).map(|s| QM::new(Arc::new(TM::new(cc.module(s), t_max)))).collect::<Vec<_>>();

    for m in &modules {
        m.compute_basis(t_max); // populate masks/basis
    }

    for t in (0 ..= t_max).rev() {
        for s in (1 ..= s_max).rev() {
            if t - (s as i32) < cc.min_degree() {
                continue;
            }

            if cc.module(s).dimension(t) == 0 {
                continue;
            }

            let keep : Option<Subspace>;
            // Now compute the image of the differentials, if any.
            if s < s_max {
                let prev = &modules[s as usize + 1];
                let curr = &modules[s as usize];
                let d = cc.differential(s + 1);

                let prev_dim = prev.dimension(t);
                let curr_orig_dim = curr.module.dimension(t);

                let mut differentials = Matrix::new(p, prev_dim, curr_orig_dim);

                for i in 0 .. prev_dim {
                    let j = prev.basis_list[t][i];
                    d.apply_to_basis_element(&mut differentials[i], 1, t, j);
                    curr.subspaces[t].reduce(&mut differentials[i]);
                }

                let mut pivots = vec![-1; curr_orig_dim];
                differentials.row_reduce(&mut pivots);

                keep = Some(Subspace {
                    matrix : differentials,
                    column_to_pivot_row : pivots
                });
            } else {
                keep = None;
            }


            let (target, source) = split_mut_borrow(&mut modules, s as usize - 1, s as usize);

            let augmentation_map = if s < target_cc.max_s() && target_cc.module(s).dimension(t) > 0 { Some(cc.chain_map(s)) } else { None };
            let preserve_map = if s >= s_shift && t >= t_shift {
                match map.chain_maps.get((s - s_shift) as usize) {
                    Some(m) => if m.target().dimension(t - t_shift) > 0 { Some(m) } else { None },
                    None => None
                }
            } else { None };

            // We can only quotient out by things in the kernel of the augmentation maps *and* the
            // steenrod operations. Moreover, the space we quotient out must be complementary to
            // the image of the differentials. The function computes the kernel and a list of
            // elements that span the image of the differentials.
            let (mut matrix, mut images) = compute_kernel_image(source, augmentation_map, preserve_map, keep, t);

            let mut pivots = vec![-1; matrix.columns()];
            matrix.row_reduce(&mut pivots);

            let subspace = &source.subspaces[t];
            let mut pivot_columns : Vec<(i32, usize)> = pivots
                .into_iter()
                .enumerate()
                .filter(|&(i, v)| v >= 0)
                .map(|(i, v)| {
                    (strategy(&*source.module.module, subspace, t, i), i)
                })
                .collect::<Vec<_>>();
            pivot_columns.sort();

            let image_pivots = images.find_pivots_permutation(pivot_columns.iter().map(|(p, i)| *i));

            let mut chosen_cols : HashSet<usize> = HashSet::new();

            for image in image_pivots.into_iter() {
                chosen_cols.insert(image);
            }

            let mut pivot_columns = pivot_columns.iter().map(|(p, i)| i).collect::<Vec<_>>();
            pivot_columns.sort();

            let d = cc.differential(s);

            let mut matrix = matrix.into_vec();
            let mut source_kills : Vec<FpVector> = Vec::with_capacity(source.module.dimension(t));
            let mut target_kills : Vec<FpVector> = Vec::with_capacity(target.module.dimension(t));

            for col in pivot_columns.into_iter().rev() {
                let source_row = matrix.pop().unwrap();
                if chosen_cols.contains(&col) {
                    continue;
                }

                let mut target_row = FpVector::new(p, target.module.dimension(t));
                d.apply(&mut target_row, 1, t, &source_row);

                source_kills.push(source_row);
                target_kills.push(target_row);
            }
            let mut goal_s_dim = 0;
            let mut goal_t_dim = 0;
            if s != s_max {
                goal_s_dim = source.dimension(t) - source_kills.len();
                goal_t_dim = target.dimension(t) - target_kills.len();
            }
            source.quotient_vectors(t, source_kills);
            target.quotient_vectors(t, target_kills);
            if s != s_max {
                assert_eq!(source.dimension(t), goal_s_dim, "Failed s dimension check at (s, t) = ({}, {})", s, t);
                assert_eq!(target.dimension(t), goal_t_dim, "Failed t dimension check at (s, t) = ({}, {})", s, t);
            }

        }
    }

    let zero_module = Arc::new(QM::new(Arc::new(TM::new(cc.zero_module(), t_max))));
    zero_module.compute_basis(t_max);
    let zero_module_fd = Arc::new(FiniteModule::FDModule(zero_module.to_fd_module()));

    let modules_fd = modules.iter().map(|m| Arc::new(FiniteModule::FDModule(m.to_fd_module()))).collect::<Vec<_>>();
    let modules = modules.into_iter().map(Arc::new).collect::<Vec<_>>();

    let zero_differential = {
        let f = cc.differential(0);
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[0].module), Arc::clone(&zero_module.module)));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphism::new(tf, Arc::clone(&modules[0]), Arc::clone(&zero_module)));
        Arc::new(FiniteModuleHomomorphism::from(
            qf.replace_source(Arc::clone(&modules_fd[0]))
              .replace_target(Arc::clone(&zero_module_fd))))
    };

    let mut differentials = vec![zero_differential];
    differentials.extend((0 .. s_max).into_iter().map(|s| {
        let f = cc.differential(s + 1);
        let s = s as usize;
        let tf = Arc::new(TruncatedHomomorphism::new(f, Arc::clone(&modules[s + 1].module), Arc::clone(&modules[s].module)));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphism::new(tf, Arc::clone(&modules[s + 1]), Arc::clone(&modules[s])));
        Arc::new(FiniteModuleHomomorphism::from(
            qf.replace_source(Arc::clone(&modules_fd[s + 1]))
              .replace_target(Arc::clone(&modules_fd[s]))))
    }));
    differentials.push(Arc::new(FiniteModuleHomomorphism::from(BoundedModuleHomomorphism::zero_homomorphism(Arc::clone(&zero_module_fd), Arc::clone(&modules_fd[s_max as usize]), 0))));
    differentials.push(Arc::new(FiniteModuleHomomorphism::from(BoundedModuleHomomorphism::zero_homomorphism(Arc::clone(&zero_module_fd), Arc::clone(&zero_module_fd), 0))));

    let chain_maps = (0 ..= s_max).into_iter().map(|s| {
        let f = cc.chain_map(s);
        let s = s as usize;
        let target = f.target();
        let tf = Arc::new(TruncatedHomomorphismSource::new(f, Arc::clone(&modules[s].module), Arc::clone(&target)));
        let qf = BoundedModuleHomomorphism::from(&QuotientHomomorphismSource::new(tf, Arc::clone(&modules[s]), target));
        Arc::new(FiniteModuleHomomorphism::from(qf.replace_source(Arc::clone(&modules_fd[s]))))
    }).collect::<Vec<_>>();

    FiniteAugmentedChainComplex {
        modules: modules_fd,
        zero_module: zero_module_fd,
        differentials,
        target_cc : cc.target(),
        chain_maps
    }
}

/// This function does the following computation:
///
/// Given the source module `source` and a subspace `keep`, the function returns the subspace of all
/// elements in `source` of degree `t` that are killed by all non-trivial actions of the algebra,
/// followed by a list of elements that span the intersection between this subspace and `keep`.
///
/// If `keep` is `None`, it is interpreted as the empty subspace.

fn compute_kernel_image<M : BoundedModule, F : ModuleHomomorphism, G : ModuleHomomorphism>(
    source : &QM<M>,
    augmentation_map : Option<Arc<F>>,
    preserve_map : Option<&G>,
    keep : Option<Subspace>,
    t : i32) -> (Matrix, Matrix) {

    let algebra = source.algebra();
    let p = algebra.prime();

    let mut generators : Vec<(i32, usize)> = Vec::new();
    let mut target_degrees = Vec::new();
    let mut padded_target_degrees : Vec<usize> = Vec::new();

    let source_orig_dimension = source.module.dimension(t);
    let source_dimension = source.dimension(t);

    for op_deg in 1 ..= source.max_degree() - t {
        for op_idx in algebra.generators(op_deg) {
            generators.push((op_deg, op_idx));
            target_degrees.push(source.module.dimension(t + op_deg));
            padded_target_degrees.push(FpVector::padded_dimension(p, source.module.dimension(t + op_deg)));
        }
    }

    if let Some(m) = &augmentation_map {
        target_degrees.push(m.target().dimension(t));
        padded_target_degrees.push(FpVector::padded_dimension(p, m.target().dimension(t)));
    }

    if let Some(m) = &preserve_map {
        let dim = m.target().dimension(t - m.degree_shift());
        target_degrees.push(dim);
        padded_target_degrees.push(FpVector::padded_dimension(p, dim));
    }

    let total_padded_degree : usize = padded_target_degrees.iter().sum();

    let padded_source_degree : usize = FpVector::padded_dimension(p, source_orig_dimension);
    let total_cols : usize = total_padded_degree + padded_source_degree + source_orig_dimension;

    let mut matrix_rows : Vec<FpVector> = Vec::with_capacity(source_dimension);

    let mut projection_off_keep = FpVector::new(p, source_orig_dimension);

    for i in 0 .. source_dimension {
        let mut result = FpVector::new(p, total_cols);

        let i = source.basis_list[t][i];
        let mut offset = 0;

        let mut target_idx = 0;
        for (op_deg, op_idx) in generators.iter() {
            result.set_slice(offset, offset + target_degrees[target_idx]);
            source.act_on_original_basis(&mut result, 1, *op_deg, *op_idx, t, i);
            result.clear_slice();
            offset += padded_target_degrees[target_idx];
            target_idx += 1;
        }

        if let Some(m) = &augmentation_map {
            result.set_slice(offset, offset + target_degrees[target_idx]);
            m.apply_to_basis_element(&mut result, 1, t, i);
            result.clear_slice();
            offset += padded_target_degrees[target_idx];
            target_idx += 1;
        }

        if let Some(m) = &preserve_map {
            result.set_slice(offset, offset + target_degrees[target_idx]);
            m.apply_to_basis_element(&mut result, 1, t, i);
            result.clear_slice();
            offset += padded_target_degrees[target_idx];
        }

        if let Some(keep) = &keep {
            projection_off_keep.set_to_zero();
            projection_off_keep.set_entry(i, 1);
            keep.reduce(&mut projection_off_keep);
            result.set_slice(offset, offset + source_orig_dimension);
            result.assign(&projection_off_keep);
            result.clear_slice();
        } else {
            result.set_entry(offset + i, 1);
        }

        result.set_entry(padded_source_degree + total_padded_degree + i, 1);
        matrix_rows.push(result);
    }
    let mut matrix = Matrix::from_rows(p, matrix_rows);
    let mut pivots = vec![-1; total_cols];
    matrix.row_reduce(&mut pivots);

    let first_kernel_row = match &pivots[0..total_padded_degree].iter().rposition(|&i| i >= 0) {
        Some(n) => pivots[*n] as usize + 1,
        None => 0
    };
    let first_image_row = match &pivots[total_padded_degree .. total_padded_degree + source_orig_dimension].iter().rposition(|&i| i >= 0) {
        Some(n) => pivots[*n + total_padded_degree] as usize + 1,
        None => first_kernel_row
    };

    matrix.set_slice(first_kernel_row, source_dimension, total_padded_degree + padded_source_degree, total_cols);
    matrix.into_slice();

    let first_image_row = first_image_row - first_kernel_row;

    let mut images = Vec::with_capacity(matrix.rows() - first_image_row);
    for i in first_image_row .. matrix.rows() {
        images.push(matrix[i].clone());
    }
    let image_matrix;
    if images.len() > 0 {
        image_matrix = Matrix::from_rows(p, images);
    } else {
        image_matrix = Matrix::new(p, 0, source_orig_dimension);
    }
    (matrix, image_matrix)
}

//static mut MEMOIZED_SIZE : Option<once::OnceVec<Vec<u32>>> = None;
//unsafe fn compute_size(algebra : &Arc<AlgebraAny>, deg : i32) {
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
//                start_module.add_generators_immediate(0, 1, None);
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
//                    let mut matrix = Matrix::from_rows(p, matrix_rows);
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
