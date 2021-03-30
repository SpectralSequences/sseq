#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use algebra::Field;
use algebra::module::block_structure::{BlockStart, BlockStructure};
use crate::chain_complex::{
    AugmentedChainComplex, ChainComplex, FiniteAugmentedChainComplex, FiniteChainComplex,
    FreeChainComplex,
};
use algebra::module::homomorphism::{
    BoundedModuleHomomorphism, FiniteModuleHomomorphism, ModuleHomomorphism,
};
use algebra::module::{
    BoundedModule, FiniteModule, HomModule, Module, SumModule, TensorModule, ZeroModule,
};
use crate::CCC;
use fp::matrix::{Matrix, QuasiInverse, Subspace};
use fp::vector::{Slice, SliceMut, FpVector};
use parking_lot::Mutex;
use std::sync::Arc;

use bivec::BiVec;
use once::{OnceBiVec, OnceVec};

pub type SHM<M> = SumModule<HomModule<M>>;

pub struct HomChainComplex<
    CC1: FreeChainComplex<Algebra = M::Algebra>,
    M: BoundedModule,
    F: ModuleHomomorphism<Source = M, Target = M>,
> {
    lock: Mutex<()>,
    source_cc: Arc<CC1>,
    target_cc: Arc<FiniteChainComplex<M, F>>,
    modules: OnceVec<Arc<SHM<M>>>,
    zero_module: Arc<SHM<M>>,
    differentials: OnceVec<Arc<HomChainMap<CC1, M, F>>>,
}

impl<
        CC1: FreeChainComplex<Algebra = M::Algebra>,
        M: BoundedModule,
        F: ModuleHomomorphism<Source = M, Target = M>,
    > HomChainComplex<CC1, M, F>
{
    pub fn new(source_cc: Arc<CC1>, target_cc: Arc<FiniteChainComplex<M, F>>) -> Self {
        Self {
            lock: Mutex::new(()),
            modules: OnceVec::new(),
            differentials: OnceVec::new(),
            zero_module: Arc::new(SumModule::zero_module(
                Arc::new(Field::new(source_cc.prime())),
                0,
            )), //source_cc.min_degree() + target_cc.max_degree())),
            source_cc,
            target_cc,
        }
    }

    fn source_cc(&self) -> Arc<CC1> {
        Arc::clone(&self.source_cc)
    }

    fn target_cc(&self) -> Arc<FiniteChainComplex<M, F>> {
        Arc::clone(&self.target_cc)
    }
}

// impl<CC1 : FreeChainComplex, CC2 : FiniteChainComplex> ChainComplex for HomChainComplex<CC1, CC2> {
//     type Module = SHM<CC2::Module>;
//     type Homomorphism = HomChainMap<CC1, CC2>;

//     fn algebra(&self) -> Arc<SteenrodAlgebra> {
//         self.left_cc.algebra()
//     }

//     fn min_degree(&self) -> i32 {
//         self.left_cc.min_degree() + self.right_cc.min_degree()
//     }

//     fn zero_module(&self) -> Arc<Self::Module> {
//         Arc::clone(&self.zero_module)
//     }

//     fn module(&self, s : u32) -> Arc<Self::Module> {
//         Arc::clone(&self.modules[s as usize])
//     }

//     fn differential(&self, s : u32) -> Arc<Self::Homomorphism> {
//         Arc::clone(&self.differentials[s as usize])
//     }

//     fn compute_through_bidegree(&self, s : u32, t : i32) {
//         self.left_cc.compute_through_bidegree(s, t - self.right_cc.min_degree());
//         self.right_cc.compute_through_bidegree(s, t - self.left_cc.min_degree());

//         let _lock = self.lock.lock().unwrap();

//         for i in self.modules.len() as u32 ..= s {
//             let new_module_list : Vec<Arc<TensorModule<CC1::Module, CC2::Module>>> =
//                 (0 ..= i).map(
//                     |j| Arc::new(TensorModule::new(self.left_cc.module(j), self.right_cc.module(i - j)))
//                 ).collect::<Vec<_>>();
//             let new_module = Arc::new(SumModule::new(self.algebra(), new_module_list, self.min_degree()));
//             self.modules.push(new_module);
//         }

//         for module in self.modules.iter() {
//             module.compute_basis(t);
//         }

//         if self.differentials.len() == 0 {
//             self.differentials.push(Arc::new(TensorChainMap {
//                 left_cc: self.left_cc(),
//                 right_cc: self.right_cc(),
//                 source_s: 0,
//                 lock : Mutex::new(()),
//                 source : self.module(0),
//                 target : self.zero_module(),
//                 quasi_inverses : OnceBiVec::new(self.min_degree())
//             }));
//         }
//         for s in self.differentials.len() as u32 ..= s {
//             self.differentials.push(Arc::new(TensorChainMap {
//                 left_cc: self.left_cc(),
//                 right_cc: self.right_cc(),
//                 source_s: s,
//                 lock : Mutex::new(()),
//                 source : self.module(s),
//                 target : self.module(s - 1),
//                 quasi_inverses : OnceBiVec::new(self.min_degree())
//             }));
//         }
//     }

//     fn set_homology_basis(&self, _homological_degree : u32, _internal_degree : i32, _homology_basis : Vec<usize>) { unimplemented!() }
//     fn homology_basis(&self, _homological_degree : u32, _internal_degree : i32) -> &Vec<usize> { unimplemented!() }
//     fn max_homology_degree(&self, _homological_degree : u32) -> i32 { unimplemented!() }
// }

pub struct HomChainMap<
    CC1: FreeChainComplex<Algebra = M::Algebra>,
    M: BoundedModule,
    F: ModuleHomomorphism<Source = M, Target = M>,
> {
    source_cc: Arc<CC1>,
    target_cc: Arc<FiniteChainComplex<M, F>>,
    lock: Mutex<()>,
    source: Arc<SHM<M>>,
    target: Arc<SHM<M>>,
    quasi_inverses: OnceBiVec<Vec<Option<Vec<(usize, usize, FpVector)>>>>,
}

impl<
        CC1: FreeChainComplex<Algebra = M::Algebra>,
        M: BoundedModule,
        F: ModuleHomomorphism<Source = M, Target = M>,
    > HomChainMap<CC1, M, F>
{
    // fn pullback_basis_element(&self, result : &mut FpVector, coeff : u32, hom_deg : i32, int_deg : i32, fn_idx : usize) {
    //     println!("fn_deg : {}, fn_idx : {}", fn_degree, fn_idx);
    //     let hom_deg_output = 0; // TODO: Figure out hom_deg_output from fn_idx.
    //     let source_module = self.source.modules[(chainmap_output + hom_deg) as usize];
    //     let intermediate_module = self.target.modules[hom_deg_output as usize]
    //     let target_module = self.target.modules[hom_deg_output as usize - 1];
    //     for out_deg in target_module.min_degree() ..= target_module.target().max_degree() {
    //         let x_degree = fn_degree + out_deg;
    //         let num_gens = source_module.source().number_of_gens_in_degree(x_degree);
    //         let old_slice = result.slice();
    //         for i in 0 .. num_gens {
    //             // let x_elt = self.source_cc.differential(??).output(x_degree, i);
    //             let BlockStart {block_start_index, block_size} = self.source.block_structures[fn_degree].generator_to_block(x_degree, i);
    //             result.set_slice(*block_start_index, *block_start_index + block_size);
    //             target_module.evaluate_basis_map_on_element(result, coeff, fn_degree, fn_idx, x_degree, &x_elt);
    //             result.restore_slice(old_slice);
    //         }
    //     }
    // }

    // fn pushforward_basis_element(&self, result : &mut FpVector, coeff : u32, fn_degree : i32, chainmap_output : u32, fn_idx : usize) {
    //     let source_module = self.source.modules[chainmap_output as usize];
    //     let target_module = self.target.modules[chainmap_output as usize - 1];
    //     for out_deg in target_module.min_degree() ..= target_module.target().max_degree() {

    //     }
    // }
}

impl<
        CC1: FreeChainComplex<Algebra = M::Algebra>,
        M: BoundedModule,
        F: ModuleHomomorphism<Source = M, Target = M>,
    > ModuleHomomorphism for HomChainMap<CC1, M, F>
{
    type Source = SHM<M>;
    type Target = SHM<M>;

    fn source(&self) -> Arc<Self::Source> {
        Arc::clone(&self.source)
    }
    fn target(&self) -> Arc<Self::Target> {
        Arc::clone(&self.target)
    }
    fn degree_shift(&self) -> i32 {
        0
    }

    /// At the moment, this is off by a sign. However, we only use this for p = 2
    fn apply_to_basis_element(
        &self,
        result: SliceMut,
        coeff: u32,
        degree: i32,
        input_idx: usize,
    ) {
        // Source is of the form ⊕_i L_i ⊗ R_(s - i). This i indexes the s degree. First figure out
        // which i this belongs to.
    }

    fn kernel(&self, _degree: i32) -> &Subspace {
        panic!("Kernels not calculated for TensorChainMap");
    }

    fn quasi_inverse(&self, _degree: i32) -> &QuasiInverse {
        panic!("Use apply_quasi_inverse instead");
    }

    fn compute_kernels_and_quasi_inverses_through_degree(&self, degree: i32) {
        let next_degree = self.quasi_inverses.len();
        if next_degree > degree {
            return;
        }

        let _lock = self.lock.lock();

        for i in next_degree..=degree {
            self.calculate_quasi_inverse(i);
        }
    }

    fn apply_quasi_inverse(&self, mut result: SliceMut, degree: i32, input: Slice) {
        let qis = &self.quasi_inverses[degree];
        assert_eq!(input.dimension(), qis.len());

        for (i, x) in input.iter().enumerate() {
            if x == 0 {
                continue;
            }
            if let Some(qi) = &qis[i] {
                for (offset_start, offset_end, data) in qi.iter() {
                    result.slice_mut(*offset_start, *offset_end)
                        .add(data.as_slice(), x);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    // use super::*;
}
