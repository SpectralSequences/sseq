use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use ext::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FreeChainComplex,
};
use ext::utils;
use ext::yoneda::yoneda_representative_element;
use fp::matrix::Matrix;
use fp::vector::FpVector;
use itertools::Itertools;
use sseq::coordinates::{Bidegree, BidegreeElement};
use tensor_product_chain_complex::TensorChainComplex;

use std::io::{stderr, stdout, Write};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(utils::query_module_only("Module", None, false)?);
    let module = resolution.target().module(0);
    let p = resolution.prime();

    if resolution.target().max_s() != 1 || !module.is_unit() || *p != 2 {
        panic!("Can only run Steenrod on the sphere");
    }

    let b = Bidegree::n_s(
        query::raw("n of Ext class", str::parse),
        query::raw("s of Ext class", str::parse),
    );

    resolution.compute_through_bidegree(b + b);

    let class: Vec<u32> =
        query::vector("Input Ext class", resolution.number_of_gens_in_bidegree(b));

    let yoneda = Arc::new(yoneda_representative_element(
        Arc::clone(&resolution),
        b,
        &class,
    ));

    print!("Dimensions of Yoneda representative: 1");
    for s in 0..=b.s() {
        print!(" {}", yoneda.module(s).total_dimension());
    }
    println!();

    let square = Arc::new(TensorChainComplex::new(
        Arc::clone(&yoneda),
        Arc::clone(&yoneda),
    ));
    let doubled_b = b + b;

    let timer = utils::Timer::start();
    square.compute_through_bidegree(doubled_b);
    for s in 0..=doubled_b.s() {
        square
            .differential(s)
            .compute_auxiliary_data_through_degree(doubled_b.t());
    }
    timer.end(format_args!("Computed quasi-inverses"));

    eprintln!("Computing Steenrod operations: ");

    let mut delta = Vec::with_capacity(b.s() as usize);

    for i in 0..=b.s() {
        let mut maps: Vec<Arc<FreeModuleHomomorphism<_>>> =
            Vec::with_capacity(doubled_b.s() as usize - 1);

        for s in 0..=doubled_b.s() - i {
            let source = resolution.module(s);
            let target = square.module(s + i);

            let map = FreeModuleHomomorphism::new(Arc::clone(&source), Arc::clone(&target), 0);
            maps.push(Arc::new(map));
        }
        delta.push(maps);
    }

    /* #[cfg(feature = "concurrent")]
    let mut prev_i_receivers: Vec<Option<Receiver<()>>> = Vec::new();
    #[cfg(feature = "concurrent")]
    for _ in 0..=2 * s {
        prev_i_receivers.push(None);
    }

    #[cfg(feature = "concurrent")]
    let mut handles: Vec<Vec<JoinHandle<()>>> = Vec::with_capacity(s as usize + 1);*/

    let timer = utils::Timer::start();

    // We use the formula d Δ_i + Δ_i d = Δ_{i-1} + τΔ_{i-1}
    for i in 0..=b.s() {
        let shift_s = Bidegree::s_t(i, 0);
        // Δ_i is a map C_s -> C_{s + i}. So to hit C_{2s}, we only need to compute up to 2
        // * s - i
        //        #[cfg(not(feature = "concurrent"))]
        let start = std::time::Instant::now();

        /* #[cfg(feature = "concurrent")]
        let mut handles_inner: Vec<JoinHandle<()>> = Vec::with_capacity((2 * s - i + 1) as usize);

        #[cfg(feature = "concurrent")]
        let mut last_receiver: Option<Receiver<()>> = None;

        #[cfg(feature = "concurrent")]
        let top_s = 2 * s - i;*/

        for s in 0..=(doubled_b - shift_s).s() {
            if i == 0 && s == 0 {
                let map = &delta[0][0];
                map.add_generators_from_matrix_rows(
                    0,
                    Matrix::from_vec(p, &[vec![1]]).as_slice_mut(),
                );
                map.extend_by_zero(doubled_b.t());
                continue;
            }

            let square = Arc::clone(&square);

            let source = resolution.module(s);
            let target = square.module(s + i);

            let dtarget_module = square.module(s + i - 1);

            let d_res = resolution.differential(s);
            let d_target = square.differential(s + i);

            let map = Arc::clone(&delta[i as usize][s as usize]);
            let prev_map = match s {
                0 => None,
                _ => Some(Arc::clone(&delta[i as usize][s as usize - 1])),
            };

            let prev_delta = match i {
                0 => None,
                _ => Some(Arc::clone(&delta[i as usize - 1][s as usize])),
            };

            /* #[cfg(feature = "concurrent")]
            let (sender, new_receiver) = unbounded();
            #[cfg(feature = "concurrent")]
            let (prev_i_sender, new_prev_i_receiver) = unbounded();


            #[cfg(feature = "concurrent")]
            let prev_i_receiver =
                std::mem::replace(&mut prev_i_receivers[s as usize], Some(new_prev_i_receiver));*/

            // Define this as a closure so that we can easily switch between threaded and
            // un-threaded
            let fun = move || {
                /* #[cfg(feature = "concurrent")]
                let mut token = bucket.take_token();*/

                for t in 0..=doubled_b.t() {
                    let b = Bidegree::s_t(s, t);
                    /* #[cfg(feature = "concurrent")]
                    {
                        token = bucket.recv2_or_release(token, &last_receiver, &prev_i_receiver);
                    }*/

                    let num_gens = source.number_of_gens_in_degree(t);

                    let fx_dim = target.dimension(t);
                    let fdx_dim = dtarget_module.dimension(t);

                    if fx_dim == 0 || fdx_dim == 0 || num_gens == 0 {
                        map.extend_by_zero(t);

                        /* #[cfg(feature = "concurrent")]
                        {
                            if s < top_s {
                                sender.send(()).unwrap();
                                prev_i_sender.send(()).unwrap();
                            }
                        }*/

                        continue;
                    }

                    let mut output_matrix = Matrix::new(p, num_gens, fx_dim);
                    let mut result = FpVector::new(p, fdx_dim);
                    for j in 0..num_gens {
                        if let Some(m) = &prev_delta {
                            // Δ_{i-1} x
                            let prevd = m.output(t, j);

                            // τ Δ_{i-1}x
                            square.swap(&mut result, prevd, b + shift_s - Bidegree::s_t(1, 0));
                            result += prevd;
                        }

                        if let Some(m) = &prev_map {
                            let dx = d_res.output(t, j);
                            m.apply(result.as_slice_mut(), 1, t, dx.as_slice());
                        }
                        assert!(d_target.apply_quasi_inverse(
                            output_matrix[j].as_slice_mut(),
                            t,
                            result.as_slice(),
                        ));

                        result.set_to_zero();
                    }
                    map.add_generators_from_matrix_rows(t, output_matrix.as_slice_mut());

                    /* #[cfg(feature = "concurrent")]
                    {
                        if s < top_s {
                            sender.send(()).unwrap();
                            prev_i_sender.send(()).unwrap();
                        }
                    }*/
                }
            };

            /* #[cfg(feature = "concurrent")]
            {
                let handle = thread::Builder::new()
                    .name(format!("D_{}, s = {}", i, s))
                    .spawn(fun);
                last_receiver = Some(new_receiver);
                handles_inner.push(handle.unwrap());
            }*/
            // #[cfg(not(feature = "concurrent"))]
            fun();
        }
        /* #[cfg(feature = "concurrent")]
        handles.push(handles_inner); */

        // #[cfg(not(feature = "concurrent"))]
        {
            let final_map = &delta[i as usize][(doubled_b - shift_s).s() as usize];
            let num_gens = resolution.number_of_gens_in_bidegree(doubled_b - shift_s);
            print!("Sq^{} ", (b - shift_s).s());
            BidegreeElement::new(b, FpVector::from_slice(p, &class).as_slice()).print();

            print!(
                " = [{}]",
                (0..num_gens)
                    .map(|k| format!("{}", final_map.output(doubled_b.t(), k).entry(0)))
                    .format(", "),
            );
            stdout().flush().unwrap();
            eprint!(" ({:?})", start.elapsed());
            stderr().flush().unwrap();
            println!();
        }
    }

    /* #[cfg(feature = "concurrent")]
    for (i, handle_inner) in handles.into_iter().enumerate() {
        let i = i as u32;

        for handle in handle_inner {
            handle.join().unwrap();
        }
        let final_map = &delta[i as usize][(2 * s - i) as usize];
        let num_gens = resolution.number_of_gens_in_bidegree(2 * s - i, 2 * t);
        print!(
            "Sq^{} x_({}, {}, {}) = [{}]",
            s - i,
            t - s as i32,
            s,
            idx,
            (0..num_gens)
                .map(|k| format!("{}", final_map.output(2 * t, k).entry(0)))
                .collect::<Vec<_>>()
                .join(", "),
        );
        stdout().flush().unwrap();
        eprint!(" ({:?} total)", start.elapsed());
        stderr().flush().unwrap();
        println!();
    }*/

    timer.end(format_args!("Computed Steenrod operations"));
    Ok(())
}

mod sum_module {
    use bivec::BiVec;
    use once::OnceBiVec;

    use algebra::module::block_structure::{BlockStructure, GeneratorBasisEltPair};
    use algebra::module::{Module, ZeroModule};
    use fp::vector::SliceMut;

    use std::sync::Arc;

    pub struct SumModule<M: Module> {
        // We need these because modules might be empty
        algebra: Arc<M::Algebra>,
        min_degree: i32,
        pub modules: Vec<Arc<M>>,
        pub block_structures: OnceBiVec<BlockStructure>,
    }

    impl<M: Module> std::fmt::Display for SumModule<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            if self.modules.is_empty() {
                write!(f, "0")
            } else {
                write!(f, "{}", self.modules[0])?;
                for m in &self.modules[1..] {
                    write!(f, " (+) {m}")?;
                }
                Ok(())
            }
        }
    }

    impl<M: Module> SumModule<M> {
        pub fn new(algebra: Arc<M::Algebra>, modules: Vec<Arc<M>>, min_degree: i32) -> Self {
            SumModule {
                algebra,
                modules,
                min_degree,
                block_structures: OnceBiVec::new(min_degree),
            }
        }

        pub fn get_module_num(&self, degree: i32, index: usize) -> usize {
            self.block_structures[degree]
                .index_to_generator_basis_elt(index)
                .generator_index
        }

        pub fn offset(&self, degree: i32, module_num: usize) -> usize {
            self.block_structures[degree]
                .generator_to_block(degree, module_num)
                .start
        }
    }

    impl<M: Module> Module for SumModule<M> {
        type Algebra = M::Algebra;

        fn algebra(&self) -> Arc<Self::Algebra> {
            Arc::clone(&self.algebra)
        }

        fn min_degree(&self) -> i32 {
            self.min_degree
        }

        fn compute_basis(&self, degree: i32) {
            for module in &self.modules {
                module.compute_basis(degree);
            }
            for i in self.block_structures.len()..=degree {
                let mut block_sizes = BiVec::new(i);
                block_sizes.push(self.modules.iter().map(|m| m.dimension(i)).collect());
                self.block_structures
                    .push(BlockStructure::new(&block_sizes));
            }
        }

        fn max_computed_degree(&self) -> i32 {
            self.block_structures.len()
        }

        fn dimension(&self, degree: i32) -> usize {
            self.block_structures
                .get(degree)
                .map(BlockStructure::total_dimension)
                .unwrap_or(0)
        }

        fn act_on_basis(
            &self,
            mut result: SliceMut,
            coeff: u32,
            op_degree: i32,
            op_index: usize,
            mod_degree: i32,
            mod_index: usize,
        ) {
            let target_degree = mod_degree + op_degree;
            let GeneratorBasisEltPair {
                generator_index: module_num,
                basis_index,
                ..
            } = self.block_structures[mod_degree].index_to_generator_basis_elt(mod_index);
            let range =
                self.block_structures[target_degree].generator_to_block(target_degree, *module_num);
            let module = &self.modules[*module_num];

            module.act_on_basis(
                result.slice_mut(range.start, range.end),
                coeff,
                op_degree,
                op_index,
                mod_degree,
                *basis_index,
            );
        }

        fn basis_element_to_string(&self, degree: i32, index: usize) -> String {
            let GeneratorBasisEltPair {
                generator_index: module_num,
                basis_index,
                ..
            } = self.block_structures[degree].index_to_generator_basis_elt(index);
            self.modules[*module_num].basis_element_to_string(degree, *basis_index)
        }

        fn max_degree(&self) -> Option<i32> {
            self.modules
                .iter()
                .map(|m| m.max_degree())
                .max()
                .unwrap_or(Some(self.min_degree))
        }
    }

    impl<M: Module> ZeroModule for SumModule<M> {
        fn zero_module(algebra: Arc<M::Algebra>, min_degree: i32) -> Self {
            SumModule::new(algebra, vec![], min_degree)
        }
    }

    #[cfg(test)]
    mod tests {
        #![allow(non_snake_case)]

        use super::*;

        use algebra::module::FDModule;
        use algebra::AdemAlgebra;

        #[test]
        fn test_sum_modules() {
            let k = r#"{"type" : "finite dimensional module", "p": 2,  "gens": {"x0": 0}, "actions": []}"#;
            let k2 = r#"{"type" : "finite dimensional module", "p": 2, "gens": {"x0": 0, "y0":0}, "actions": []}"#;
            let zero =
                r#"{"type" : "finite dimensional module", "p": 2, "gens": {}, "actions": []}"#;
            let c2 = r#"{"type" : "finite dimensional module",  "p": 2, "gens": {"x0": 0, "x1": 1}, "actions": ["Sq1 x0 = x1"]}"#;
            let ceta = r#"{"type" : "finite dimensional module", "p": 2, "gens": {"x0": 0, "x2": 2}, "actions": ["Sq2 x0 = x2"]}"#;
            let c2sumceta = r#"{"type" : "finite dimensional module", "p": 2, "gens": {"x0": 0, "x1": 1,"y0": 0, "y2": 2}, "actions": ["Sq1 x0 = x1", "Sq2 y0 = y2"]}"#;

            test_sum_module(vec![], zero);
            test_sum_module(vec![k, k], k2);
            test_sum_module(vec![c2, ceta], c2sumceta);
        }

        fn test_sum_module(M: Vec<&str>, S: &str) {
            let p = fp::prime::ValidPrime::new(2);
            let A = Arc::new(AdemAlgebra::new(p, false));

            let M: Vec<Arc<FDModule<AdemAlgebra>>> = M
                .into_iter()
                .map(|s| {
                    let m = serde_json::from_str(s).unwrap();
                    Arc::new(FDModule::from_json(Arc::clone(&A), &m).unwrap())
                })
                .collect::<Vec<_>>();

            let sum = FDModule::from(&SumModule::new(Arc::clone(&A), M, 0));

            let S = serde_json::from_str(S).unwrap();
            let S = FDModule::from_json(Arc::clone(&A), &S).unwrap();

            if let Err(msg) = sum.test_equal(&S) {
                panic!("Test case failed. {msg}");
            }
        }
    }
}

mod tensor_product_chain_complex {
    use super::sum_module::SumModule;
    use algebra::module::homomorphism::ModuleHomomorphism;
    use algebra::module::{Module, TensorModule, ZeroModule};
    use algebra::{Algebra, Bialgebra};
    use ext::chain_complex::ChainComplex;
    use fp::matrix::AugmentedMatrix;
    use fp::vector::{FpVector, Slice, SliceMut};
    use sseq::coordinates::Bidegree;
    use std::sync::Arc;

    use once::{OnceBiVec, OnceVec};

    pub type Stm<M, N> = SumModule<TensorModule<M, N>>;

    pub struct TensorChainComplex<A, CC1, CC2>
    where
        A: Algebra + Bialgebra,
        CC1: ChainComplex<Algebra = A>,
        CC2: ChainComplex<Algebra = A>,
    {
        left_cc: Arc<CC1>,
        right_cc: Arc<CC2>,
        modules: OnceVec<Arc<Stm<CC1::Module, CC2::Module>>>,
        zero_module: Arc<Stm<CC1::Module, CC2::Module>>,
        differentials: OnceVec<Arc<TensorChainMap<A, CC1, CC2>>>,
    }

    impl<A, CC1, CC2> TensorChainComplex<A, CC1, CC2>
    where
        A: Algebra + Bialgebra,
        CC1: ChainComplex<Algebra = A>,
        CC2: ChainComplex<Algebra = A>,
    {
        pub fn new(left_cc: Arc<CC1>, right_cc: Arc<CC2>) -> Self {
            Self {
                modules: OnceVec::new(),
                differentials: OnceVec::new(),
                zero_module: Arc::new(SumModule::zero_module(
                    left_cc.algebra(),
                    left_cc.min_degree() + right_cc.min_degree(),
                )),
                left_cc,
                right_cc,
            }
        }

        fn left_cc(&self) -> Arc<CC1> {
            Arc::clone(&self.left_cc)
        }

        fn right_cc(&self) -> Arc<CC2> {
            Arc::clone(&self.right_cc)
        }

        fn left_min_shift(&self) -> Bidegree {
            Bidegree::s_t(0, self.left_cc.min_degree())
        }

        fn right_min_shift(&self) -> Bidegree {
            Bidegree::s_t(0, self.right_cc.min_degree())
        }
    }

    impl<A, CC> TensorChainComplex<A, CC, CC>
    where
        A: Algebra + Bialgebra,
        CC: ChainComplex<Algebra = A>,
    {
        /// This function sends a (x) b to b (x) a. This makes sense only if left_cc and right_cc are
        /// equal, but we don't check that.
        pub fn swap(&self, result: &mut FpVector, vec: &FpVector, b: Bidegree) {
            let s = b.s() as usize;

            for left_s in 0..=s {
                let right_s = s - left_s;
                let module = &self.modules[s];

                let source_offset = module.offset(b.t(), left_s);
                let target_offset = module.offset(b.t(), right_s);

                for left_t in 0..=b.t() {
                    let right_t = b.t() - left_t;

                    let left_dim = module.modules[left_s].left.dimension(left_t);
                    let right_dim = module.modules[left_s].right.dimension(right_t);

                    if left_dim == 0 || right_dim == 0 {
                        continue;
                    }

                    let source_inner_offset = module.modules[left_s].offset(b.t(), left_t);
                    let target_inner_offset = module.modules[right_s].offset(b.t(), right_t);

                    for i in 0..left_dim {
                        for j in 0..right_dim {
                            let value =
                                vec.entry(source_offset + source_inner_offset + i * right_dim + j);
                            if value != 0 {
                                result.add_basis_element(
                                    target_offset + target_inner_offset + j * left_dim + i,
                                    value,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    impl<A, CC1, CC2> ChainComplex for TensorChainComplex<A, CC1, CC2>
    where
        A: Algebra + Bialgebra,
        CC1: ChainComplex<Algebra = A>,
        CC2: ChainComplex<Algebra = A>,
    {
        type Algebra = A;
        type Module = Stm<CC1::Module, CC2::Module>;
        type Homomorphism = TensorChainMap<A, CC1, CC2>;

        fn algebra(&self) -> Arc<A> {
            self.left_cc.algebra()
        }

        fn min_degree(&self) -> i32 {
            self.left_cc.min_degree() + self.right_cc.min_degree()
        }

        fn zero_module(&self) -> Arc<Self::Module> {
            Arc::clone(&self.zero_module)
        }

        fn has_computed_bidegree(&self, b: Bidegree) -> bool {
            self.left_cc
                .has_computed_bidegree(b - self.left_min_shift())
                && self
                    .right_cc
                    .has_computed_bidegree(b - self.left_min_shift())
                && self.differentials.len() > b.s() as usize
        }

        fn module(&self, s: u32) -> Arc<Self::Module> {
            Arc::clone(&self.modules[s as usize])
        }

        fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
            Arc::clone(&self.differentials[s as usize])
        }

        fn compute_through_bidegree(&self, b: Bidegree) {
            self.left_cc
                .compute_through_bidegree(b - self.right_min_shift());
            self.right_cc
                .compute_through_bidegree(b - self.left_min_shift());

            self.modules.extend(b.s() as usize, |i| {
                let i = i as u32;
                let new_module_list: Vec<Arc<TensorModule<CC1::Module, CC2::Module>>> = (0..=i)
                    .map(|j| {
                        Arc::new(TensorModule::new(
                            self.left_cc.module(j),
                            self.right_cc.module(i - j),
                        ))
                    })
                    .collect::<Vec<_>>();
                Arc::new(SumModule::new(
                    self.algebra(),
                    new_module_list,
                    self.min_degree(),
                ))
            });

            for module in self.modules.iter() {
                module.compute_basis(b.t());
            }

            self.differentials.extend(b.s() as usize, |s| {
                let s = s as u32;
                if s == 0 {
                    Arc::new(TensorChainMap {
                        left_cc: self.left_cc(),
                        right_cc: self.right_cc(),
                        source_s: 0,
                        source: self.module(0),
                        target: self.zero_module(),
                        quasi_inverses: OnceBiVec::new(self.min_degree()),
                    })
                } else {
                    Arc::new(TensorChainMap {
                        left_cc: self.left_cc(),
                        right_cc: self.right_cc(),
                        source_s: s,
                        source: self.module(s),
                        target: self.module(s - 1),
                        quasi_inverses: OnceBiVec::new(self.min_degree()),
                    })
                }
            });
        }

        fn next_homological_degree(&self) -> u32 {
            self.modules.len() as u32
        }
    }

    pub struct TensorChainMap<A, CC1, CC2>
    where
        A: Algebra + Bialgebra,
        CC1: ChainComplex<Algebra = A>,
        CC2: ChainComplex<Algebra = A>,
    {
        left_cc: Arc<CC1>,
        right_cc: Arc<CC2>,
        source_s: u32,
        source: Arc<Stm<CC1::Module, CC2::Module>>,
        target: Arc<Stm<CC1::Module, CC2::Module>>,
        quasi_inverses: OnceBiVec<Vec<Option<Vec<(usize, usize, FpVector)>>>>,
    }

    impl<A, CC1, CC2> ModuleHomomorphism for TensorChainMap<A, CC1, CC2>
    where
        A: Algebra + Bialgebra,
        CC1: ChainComplex<Algebra = A>,
        CC2: ChainComplex<Algebra = A>,
    {
        type Source = Stm<CC1::Module, CC2::Module>;
        type Target = Stm<CC1::Module, CC2::Module>;

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
            mut result: SliceMut,
            coeff: u32,
            degree: i32,
            input_idx: usize,
        ) {
            // Source is of the form ⊕_i L_i ⊗ R_(s - i). This i indexes the s degree. First figure out
            // which i this belongs to.
            let left_s = self.source.get_module_num(degree, input_idx);
            let right_s = self.source_s as usize - left_s;

            let source_module = &self.source.modules[left_s];

            let first_offset = self.source.offset(degree, left_s);
            let inner_index = input_idx - first_offset;

            // Now redefine L = L_i, R = R_(degree - i). Then L ⊗ R is itself a sum of terms of
            // the form L_i ⊗ R_(degree - i), where we are now summing over the t degree.
            let left_t = source_module.seek_module_num(degree, inner_index);
            let right_t = degree - left_t;

            let inner_index = inner_index - source_module.offset(degree, left_t);

            let source_right_dim = source_module.right.dimension(right_t);
            let right_index = inner_index % source_right_dim;
            let left_index = inner_index / source_right_dim;

            // Now calculate 1 (x) d
            if right_s > 0 {
                let target_module = &self.target.modules[left_s];
                let target_offset = self.target.offset(degree, left_s)
                    + self.target.modules[left_s].offset(degree, left_t);
                let target_right_dim = target_module.right.dimension(right_t);

                let result = result.slice_mut(
                    target_offset + left_index * target_right_dim,
                    target_offset + (left_index + 1) * target_right_dim,
                );
                self.right_cc
                    .differential(right_s as u32)
                    .apply_to_basis_element(result, coeff, right_t, right_index);
            }

            // Now calculate d (x) 1
            if left_s > 0 {
                let target_module = &self.target.modules[left_s - 1];
                let target_offset = self.target.offset(degree, left_s - 1)
                    + self.target.modules[left_s - 1].offset(degree, left_t);
                let target_right_dim = target_module.right.dimension(right_t);

                let mut dl = FpVector::new(self.prime(), target_module.left.dimension(left_t));
                self.left_cc
                    .differential(left_s as u32)
                    .apply_to_basis_element(dl.as_slice_mut(), coeff, left_t, left_index);
                for i in 0..dl.len() {
                    result.add_basis_element(
                        target_offset + i * target_right_dim + right_index,
                        dl.entry(i),
                    );
                }
            }
        }

        fn compute_auxiliary_data_through_degree(&self, degree: i32) {
            self.quasi_inverses
                .extend(degree, |i| self.calculate_quasi_inverse(i));
        }

        fn apply_quasi_inverse(&self, mut result: SliceMut, degree: i32, input: Slice) -> bool {
            let qis = &self.quasi_inverses[degree];
            assert_eq!(input.len(), qis.len());

            for (i, x) in input.iter_nonzero() {
                if let Some(qi) = &qis[i] {
                    for (offset_start, offset_end, data) in qi.iter() {
                        result
                            .slice_mut(*offset_start, *offset_end)
                            .add(data.as_slice(), x);
                    }
                }
            }
            true
        }
    }

    impl<A, CC1, CC2> TensorChainMap<A, CC1, CC2>
    where
        A: Algebra + Bialgebra,
        CC1: ChainComplex<Algebra = A>,
        CC2: ChainComplex<Algebra = A>,
    {
        #[allow(clippy::range_minus_one)]
        fn calculate_quasi_inverse(
            &self,
            degree: i32,
        ) -> Vec<Option<Vec<(usize, usize, FpVector)>>> {
            let p = self.prime();
            // start, end, preimage
            let mut quasi_inverse_list: Vec<Option<Vec<(usize, usize, FpVector)>>> =
                vec![None; self.target.dimension(degree)];

            for left_t in self.left_cc.min_degree()..=degree - self.right_cc.min_degree() {
                let right_t = degree - left_t;

                let source_dim = self
                    .source
                    .modules
                    .iter()
                    .map(|m| m.left.dimension(left_t) * m.right.dimension(right_t))
                    .sum();
                let target_dim = self
                    .target
                    .modules
                    .iter()
                    .map(|m| m.left.dimension(left_t) * m.right.dimension(right_t))
                    .sum();

                if source_dim == 0 || target_dim == 0 {
                    continue;
                }

                let mut matrix = AugmentedMatrix::new(p, source_dim, [target_dim, source_dim]);

                // Compute 1 (x) d
                let mut target_offset = 0;
                let mut row_count = 0;
                for s in 0..=self.source_s - 1 {
                    let source_module = &self.source.modules[s as usize]; // C_s (x) D_{source_s - s}
                    let target_module = &self.target.modules[s as usize]; // C_s (x) D_{source_s - s - 1}

                    let source_right_dim = source_module.right.dimension(right_t);
                    let source_left_dim = source_module.left.dimension(left_t);
                    let target_right_dim = target_module.right.dimension(right_t);
                    let target_left_dim = target_module.left.dimension(left_t);
                    assert_eq!(target_left_dim, source_left_dim);

                    let mut result = FpVector::new(p, target_right_dim);
                    for ri in 0..source_right_dim {
                        self.right_cc
                            .differential(self.source_s - s)
                            .apply_to_basis_element(result.as_slice_mut(), 1, right_t, ri);
                        for li in 0..source_left_dim {
                            let row = &mut matrix[row_count + li * source_right_dim + ri];
                            row.slice_mut(
                                target_offset + li * target_right_dim,
                                target_offset + (li + 1) * target_right_dim,
                            )
                            .assign(result.as_slice());
                        }
                        result.set_to_zero();
                    }
                    target_offset += target_right_dim * target_left_dim;
                    row_count += source_right_dim * source_left_dim;
                }

                // Compute d (x) 1
                let mut target_offset = 0;
                let mut row_count = {
                    let m = &self.source.modules[0usize];
                    m.left.dimension(left_t) * m.right.dimension(right_t)
                };
                for s in 1..=self.source_s {
                    let source_module = &self.source.modules[s as usize]; // C_s (x) D_{source_s - s}
                    let target_module = &self.target.modules[s as usize - 1]; // C_{s - 1} (x) D_{source_s - s}

                    let source_right_dim = source_module.right.dimension(right_t);
                    let source_left_dim = source_module.left.dimension(left_t);
                    let target_right_dim = target_module.right.dimension(right_t);
                    let target_left_dim = target_module.left.dimension(left_t);
                    assert_eq!(target_right_dim, source_right_dim);

                    let mut result = FpVector::new(p, target_left_dim);
                    for li in 0..source_left_dim {
                        self.left_cc.differential(s).apply_to_basis_element(
                            result.as_slice_mut(),
                            1,
                            left_t,
                            li,
                        );
                        for ri in 0..source_right_dim {
                            let row = &mut matrix[row_count];
                            for (i, x) in result.iter_nonzero() {
                                row.add_basis_element(target_offset + i * target_right_dim + ri, x);
                            }
                            row_count += 1;
                        }
                        result.set_to_zero();
                    }
                    target_offset += target_right_dim * target_left_dim;
                }

                matrix.segment(1, 1).add_identity();
                matrix.row_reduce();

                let mut index = 0;
                let mut row = 0;
                for s in 0..self.source_s as usize {
                    let target_module = &self.target.modules[s]; // C_s (x) D_{source_s - s - 1}

                    let target_right_dim = target_module.right.dimension(right_t);
                    let target_left_dim = target_module.left.dimension(left_t);

                    for li in 0..target_left_dim {
                        for ri in 0..target_right_dim {
                            if matrix.pivots()[index] >= 0 {
                                let true_index = self.target.offset(degree, s)
                                    + self.target.modules[s].offset(degree, left_t)
                                    + li * target_right_dim
                                    + ri;
                                let mut entries = Vec::new();
                                let mut offset = 0;
                                for s_ in 0..=self.source_s as usize {
                                    let dim = {
                                        let m = &self.source.modules[s_];
                                        m.left.dimension(left_t) * m.right.dimension(right_t)
                                    };
                                    if dim == 0 {
                                        continue;
                                    }

                                    let mut entry = FpVector::new(p, dim);
                                    entry.as_slice_mut().assign(
                                        matrix.row_segment(row, 1, 1).slice(offset, offset + dim),
                                    );

                                    if !entry.is_zero() {
                                        let true_slice_start = self.source.offset(degree, s_)
                                            + self.source.modules[s_].offset(degree, left_t);
                                        let true_slice_end = true_slice_start + dim;
                                        entries.push((true_slice_start, true_slice_end, entry));
                                    }

                                    offset += dim;
                                }
                                assert!(quasi_inverse_list[true_index].is_none());
                                assert!(!entries.is_empty());
                                quasi_inverse_list[true_index] = Some(entries);
                                row += 1;
                            }
                            index += 1;
                        }
                    }
                }
            }
            quasi_inverse_list
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use ext::resolution_homomorphism::ResolutionHomomorphism;
        use ext::utils::construct;
        use ext::yoneda::yoneda_representative_element;

        use rstest::rstest;

        #[rstest]
        #[trace]
        #[case(Bidegree::n_s(0, 1), &[1], &[1])]
        #[case(Bidegree::n_s(0, 2), &[1], &[1])]
        #[case(Bidegree::n_s(1, 1), &[1], &[1])]
        #[case(Bidegree::n_s(3, 1), &[1], &[1])]
        #[case(Bidegree::n_s(14, 4), &[1], &[1])]
        fn test_square_cc(#[case] b: Bidegree, #[case] class: &[u32], #[case] output: &[u32]) {
            let doubled_b = b + b;
            let resolution = Arc::new(construct("S_2", None).unwrap());
            let p = resolution.prime();
            resolution.compute_through_bidegree(doubled_b);

            let yoneda = Arc::new(yoneda_representative_element(
                Arc::clone(&resolution),
                b,
                class,
            ));

            let square = Arc::new(TensorChainComplex::new(
                Arc::clone(&yoneda),
                Arc::clone(&yoneda),
            ));
            square.compute_through_bidegree(doubled_b);

            let f = ResolutionHomomorphism::new(
                "".to_string(),
                Arc::clone(&resolution),
                square,
                Bidegree::zero(),
            );
            f.extend_step_raw(Bidegree::zero(), Some(vec![FpVector::from_slice(p, &[1])]));

            f.extend(doubled_b);
            let final_map = f.get_map(doubled_b.s());

            for (i, &v) in output.iter().enumerate() {
                assert_eq!(final_map.output(doubled_b.t(), i).len(), 1);
                assert_eq!(final_map.output(doubled_b.t(), i).entry(0), v);
            }
        }
    }
}
