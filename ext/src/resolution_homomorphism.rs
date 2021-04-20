use std::sync::Arc;

use crate::chain_complex::{AugmentedChainComplex, FreeChainComplex};
use crate::resolution::Resolution;
use crate::CCC;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use algebra::SteenrodAlgebra;
use fp::matrix::Matrix;
use fp::vector::{FpVector, SliceMut};
use once::OnceVec;

#[cfg(feature = "concurrent")]
use {
    crossbeam_channel::{unbounded, Receiver},
    thread_token::TokenBucket,
};

pub struct ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex<Algebra = <CC2::Module as Module>::Algebra>,
    CC2: AugmentedChainComplex,
{
    #[allow(dead_code)]
    name: String,
    pub source: Arc<CC1>,
    pub target: Arc<CC2>,
    maps: OnceVec<FreeModuleHomomorphism<CC2::Module>>,
    pub shift_s: u32,
    pub shift_t: i32,
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex<Algebra = <CC2::Module as Module>::Algebra>,
    CC2: AugmentedChainComplex,
{
    pub fn new(
        name: String,
        source: Arc<CC1>,
        target: Arc<CC2>,
        shift_s: u32,
        shift_t: i32,
    ) -> Self {
        Self {
            name,
            source,
            target,
            maps: OnceVec::new(),
            shift_s,
            shift_t,
        }
    }

    fn get_map_ensure_length(&self, output_s: u32) -> &FreeModuleHomomorphism<CC2::Module> {
        self.maps.extend(output_s as usize, |output_s| {
            let input_s = output_s as u32 + self.shift_s;
            FreeModuleHomomorphism::new(
                self.source.module(input_s),
                self.target.module(output_s as u32),
                self.shift_t,
            )
        });
        &self.maps[output_s as usize]
    }

    pub fn get_map(&self, output_s: u32) -> &FreeModuleHomomorphism<CC2::Module> {
        &self.maps[output_s as usize]
    }

    pub fn into_chain_maps(self) -> Vec<FreeModuleHomomorphism<CC2::Module>> {
        self.maps.into_vec()
    }

    /// Extend the resolution homomorphism such that it is defined on degrees
    /// (`max_s`, `max_t`).
    pub fn extend(&self, max_s: u32, max_t: i32) {
        self.get_map_ensure_length(max_s);
        for s in self.shift_s..=max_s {
            let f_cur = self.get_map_ensure_length(s - self.shift_s);
            for t in f_cur.next_degree()..=max_t {
                self.extend_step(s, t, None);
            }
        }
    }

    pub fn extend_through_stem(&self, max_s: u32, max_f: i32) {
        self.get_map_ensure_length(max_s);
        for s in self.shift_s..=max_s {
            let f_cur = self.get_map_ensure_length(s - self.shift_s);
            for t in f_cur.next_degree()..=(max_f + s as i32) {
                self.extend_step(s, t, None);
            }
        }
    }

    #[cfg(feature = "concurrent")]
    pub fn extend_through_stem_concurrent(&self, max_s: u32, max_f: i32, bucket: &TokenBucket) {
        self.get_map_ensure_length(max_s);
        crossbeam_utils::thread::scope(|scope| {
            let mut last_receiver: Option<Receiver<()>> = None;
            for s in self.shift_s..=max_s {
                let (sender, receiver) = unbounded();
                scope
                    .builder()
                    .name(format!("s = {}", s))
                    .spawn(move |_| {
                        let mut token = bucket.take_token();
                        sender.send(()).ok();
                        for t in self.source.min_degree()..=(max_f + s as i32) {
                            token = bucket.recv_or_release(token, &last_receiver);
                            self.extend_step(s, t, None);
                            sender.send(()).ok();
                        }
                    })
                    .unwrap();
                last_receiver = Some(receiver);
            }
        })
        .unwrap();
    }

    pub fn extend_step(&self, input_s: u32, input_t: i32, extra_images: Option<&Matrix>) {
        let output_s = input_s - self.shift_s;
        let output_t = input_t - self.shift_t;
        assert!(self.target.has_computed_bidegree(output_s, output_t));
        assert!(self.source.has_computed_bidegree(input_s, input_t));
        assert!(input_s >= self.shift_s);

        let f_cur = self.get_map_ensure_length(output_s);
        if input_t < f_cur.next_degree() {
            assert!(extra_images.is_none());
            return;
        }

        let p = self.source.prime();

        let f_cur = self.get_map(output_s);
        let num_gens = f_cur.source().number_of_gens_in_degree(input_t);
        let fx_dimension = f_cur.target().dimension(output_t);

        let mut outputs = vec![FpVector::new(p, fx_dimension); num_gens];
        if num_gens == 0 || fx_dimension == 0 {
            f_cur.add_generators_from_rows(input_t, outputs);
            return;
        }
        if output_s == 0 {
            if let Some(extra_images_matrix) = extra_images {
                let target_chain_map = self.target.chain_map(output_s);
                let target_cc_dimension = target_chain_map.target().dimension(output_t);
                assert!(target_cc_dimension == extra_images_matrix.columns());

                target_chain_map.compute_auxiliary_data_through_degree(output_t);
                assert!(
                    num_gens == extra_images_matrix.rows(),
                    "num_gens : {} greater than rows : {} hom_deg : {}, int_deg : {}",
                    num_gens,
                    extra_images_matrix.rows(),
                    input_s,
                    input_t
                );
                for k in 0..num_gens {
                    target_chain_map.apply_quasi_inverse(
                        outputs[k].as_slice_mut(),
                        output_t,
                        extra_images_matrix[k].as_slice(),
                    );
                }
            }
            f_cur.add_generators_from_rows(input_t, outputs);
            return;
        }
        let d_source = self.source.differential(input_s);
        let d_target = self.target.differential(output_s);
        let f_prev = self.get_map(output_s - 1);
        assert!(Arc::ptr_eq(&d_source.source(), &f_cur.source()));
        assert!(Arc::ptr_eq(&d_source.target(), &f_prev.source()));
        assert!(Arc::ptr_eq(&d_target.source(), &f_cur.target()));
        assert!(Arc::ptr_eq(&d_target.target(), &f_prev.target()));
        let fdx_dimension = f_prev.target().dimension(output_t);
        let mut fdx_vector = FpVector::new(p, fdx_dimension);
        let mut extra_image_row = 0;
        for (k, output_row) in outputs.iter_mut().enumerate() {
            let dx_vector = d_source.output(input_t, k);
            if dx_vector.is_zero() {
                let target_chain_map = self.target.chain_map(output_s);
                let target_cc_dimension = target_chain_map.target().dimension(output_t);
                if let Some(extra_images_matrix) = &extra_images {
                    assert!(target_cc_dimension == extra_images_matrix.columns());
                }

                let extra_image_matrix = extra_images.as_ref().expect("Missing extra image rows");
                target_chain_map.compute_auxiliary_data_through_degree(output_t);
                target_chain_map.apply_quasi_inverse(
                    output_row.as_slice_mut(),
                    output_t,
                    extra_image_matrix[extra_image_row].as_slice(),
                );
                extra_image_row += 1;
            } else {
                d_target.compute_auxiliary_data_through_degree(output_t);
                f_prev.apply(fdx_vector.as_slice_mut(), 1, input_t, dx_vector.as_slice());
                d_target.apply_quasi_inverse(
                    output_row.as_slice_mut(),
                    output_t,
                    fdx_vector.as_slice(),
                );
                fdx_vector.set_to_zero();
            }
        }
        f_cur.add_generators_from_rows(input_t, outputs);
    }
}

use crate::chain_complex::{BoundedChainComplex, ChainComplex};
use algebra::module::homomorphism::FiniteModuleHomomorphism;
use algebra::module::{BoundedModule, FiniteModule};

impl<M, ACC, TCC> ResolutionHomomorphism<Resolution<CCC>, ACC>
where
    M: Module<Algebra = SteenrodAlgebra>,
    ACC: AugmentedChainComplex<Algebra = SteenrodAlgebra, TargetComplex = TCC>,
    TCC: BoundedChainComplex<Algebra = SteenrodAlgebra, Module = M>,
{
    pub fn from_module_homomorphism(
        name: String,
        source: Arc<Resolution<CCC>>,
        target: Arc<ACC>,
        f: &FiniteModuleHomomorphism<M>,
    ) -> Self {
        assert_eq!(source.target().max_s(), 1);
        assert_eq!(target.target().max_s(), 1);

        let source_module = source.target().module(0);
        let target_module = target.target().module(0);
        assert!(Arc::ptr_eq(&source_module, &f.source()));
        assert!(Arc::ptr_eq(&target_module, &f.target()));

        let p = source.prime();
        let degree_shift = f.degree_shift();

        let max_degree = match &*source_module {
            FiniteModule::FDModule(m) => m.max_degree(),
            FiniteModule::FPModule(m) => m.generators().get_max_generator_degree(),
            FiniteModule::RealProjectiveSpace(_) => panic!("Real Projective Space not supported"),
        };

        let hom = Self::new(name, source, target, 0, degree_shift);

        source_module.compute_basis(max_degree);
        target_module.compute_basis(degree_shift + max_degree);

        hom.source.compute_through_bidegree(0, max_degree);
        hom.target
            .compute_through_bidegree(0, degree_shift + max_degree);

        let source_chain_map = hom.source.chain_map(0);
        let target_chain_map = hom.target.chain_map(0);
        target_chain_map.compute_auxiliary_data_through_degree(degree_shift + max_degree);

        let g = hom.get_map_ensure_length(0);

        for t in source_module.min_degree()..=max_degree {
            let num_gens = hom.source.module(0).number_of_gens_in_degree(t);

            let mut fx = FpVector::new(p, target_module.dimension(t + degree_shift));

            let mut outputs_matrix = Matrix::new(
                p,
                num_gens,
                hom.target.module(0).dimension(t + degree_shift),
            );
            if num_gens == 0 || fx.is_empty() {
                g.add_generators_from_matrix_rows(t, outputs_matrix.as_slice_mut());
                continue;
            }
            for j in 0..num_gens {
                f.apply(
                    fx.as_slice_mut(),
                    1,
                    t,
                    source_chain_map.output(t, j).as_slice(),
                );
                target_chain_map.apply_quasi_inverse(
                    outputs_matrix[j].as_slice_mut(),
                    t + degree_shift,
                    fx.as_slice(),
                );
                fx.set_to_zero();
            }
            g.add_generators_from_matrix_rows(t, outputs_matrix.as_slice_mut());
        }
        hom
    }
}

impl<CC1, CC2> ResolutionHomomorphism<CC1, CC2>
where
    CC1: FreeChainComplex,
    CC2: AugmentedChainComplex + FreeChainComplex<Algebra = CC1::Algebra>,
{
    pub fn act(&self, mut result: SliceMut, s: u32, t: i32, idx: usize) {
        let source_s = s - self.shift_s;
        let source_t = t - self.shift_t;

        assert_eq!(
            result.as_slice().len(),
            self.source
                .module(source_s)
                .number_of_gens_in_degree(source_t)
        );

        let target_module = self.target.module(s);

        let map = self.get_map(s);
        for i in 0..result.as_slice().len() {
            let j = target_module.operation_generator_to_index(0, 0, t, idx);
            result.add_basis_element(i, map.output(t, i).entry(j));
        }
    }
}
