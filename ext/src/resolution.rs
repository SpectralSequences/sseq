//! This module exports the [`Resolution`] object, which is a chain complex resolving a module. In
//! particular, this contains the core logic that compute minimal resolutions.
use std::sync::{Arc, Mutex, mpsc};

use algebra::{
    Algebra, Field, Solvable, MuAlgebra, Ring,
    linear_algebra::NextStageInput,
    module::{
        Module, MuFreeModule,
        homomorphism::{ModuleHomomorphism, MuFreeModuleHomomorphism},
    },
};
use anyhow::Context;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use dashmap::DashMap;
use fp::{
    matrix::{AugmentedMatrix, QuasiInverse, Subspace},
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::Itertools;
use once::{OnceBiVec, OnceVec};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use crate::{
    chain_complex::{AugmentedChainComplex, ChainComplex},
    save::{SaveDirectory, SaveKind},
    utils::parallel::ParallelGuard,
};

/// In [`MuResolution::compute_through_stem`] and [`MuResolution::compute_through_bidegree`], we pass
/// this struct around to inform the supervisor what bidegrees have been computed. We use an
/// explicit struct instead of a tuple to avoid an infinite type problem.
struct SenderData {
    b: Bidegree,
    /// Whether this bidegree was newly calculated or have already been calculated.
    new: bool,
    /// Whether this job should be retried due to priority inversion avoidance.
    retry: bool,
    /// The sender object used to send the `SenderData`. We put this in the struct and pass it
    /// around the mpsc, so that when all senders are dropped, we know the computation has
    /// completed. Compared to keeping track of calculations manually, this has the advantage of
    /// behaving correctly when a thread panicking.
    sender: mpsc::Sender<Self>,
}

impl SenderData {
    fn send(b: Bidegree, new: bool, sender: mpsc::Sender<Self>) {
        sender
            .send(Self {
                b,
                new,
                retry: false,
                sender: sender.clone(),
            })
            .unwrap()
    }

    fn send_retry(b: Bidegree, sender: mpsc::Sender<Self>) {
        sender
            .send(Self {
                b,
                new: false,
                retry: true,
                sender: sender.clone(),
            })
            .unwrap()
    }
}

/// This is the maximum number of new generators we expect in each bidegree. This affects how much
/// space we allocate when we are extending our resolutions. Having more than this many new
/// generators will result in a slowdown but not an error. It is relatively cheap to increment this
/// number if needs be, but up to the 140th stem we only see at most 8 new generators.
const MAX_NEW_GENS: usize = 10;

pub type Resolution<CC> = MuResolution<false, CC>;
pub type UnstableResolution<CC> = MuResolution<true, CC>;

/// A minimal resolution of a chain complex. The functions [`MuResolution::compute_through_stem`] and
/// [`MuResolution::compute_through_bidegree`] extends the minimal resolution to the given bidegree.
pub struct MuResolution<const U: bool, CC: ChainComplex>
where
    CC::Algebra: MuAlgebra<U>,
{
    name: String,
    lock: Mutex<()>,
    complex: Arc<CC>,
    modules: OnceBiVec<Arc<MuFreeModule<U, CC::Algebra>>>,
    zero_module: Arc<MuFreeModule<U, CC::Algebra>>,
    chain_maps: OnceBiVec<Arc<MuFreeModuleHomomorphism<U, CC::Module>>>,
    differentials: OnceVec<Arc<MuFreeModuleHomomorphism<U, MuFreeModule<U, CC::Algebra>>>>,

    ///  For each *internal* degree, store the kernel of the most recently calculated chain map as
    ///  returned by `generate_old_kernel_and_compute_new_kernel`, to be used if we run
    ///  compute_through_degree again.
    kernels: DashMap<Bidegree, Subspace>,
    save_dir: SaveDirectory,

    /// Whether we should save newly computed data to the disk. This has no effect if there is no
    /// save file. Defaults to `self.save_dir.is_some()`.
    pub should_save: bool,

    /// Whether we should keep the quasi-inverses of the differentials.
    ///
    /// If set to false,
    ///  - If there is no save file, then the quasi-inverse will not be computed.
    ///  - If there is a save file, then the quasi-inverse will be computed, written to disk, and
    ///    dropped from memory. We will not load quasi-inverses from save files.
    ///
    /// Note that this only applies to quasi-inverses of differentials. The quasi-inverses to the
    /// augmentation map are useful when the target chain complex is not concentrated in one
    /// degree, and they tend to be quite small anyway.
    pub load_quasi_inverse: bool,
}

impl<const U: bool, CC: ChainComplex> MuResolution<U, CC>
where
    CC::Algebra: MuAlgebra<U>,
{
    pub fn new(complex: Arc<CC>) -> Self {
        // It doesn't error if the save file is None
        Self::new_with_save(complex, None).unwrap()
    }

    pub fn new_with_save(
        complex: Arc<CC>,
        save_dir: impl Into<SaveDirectory>,
    ) -> anyhow::Result<Self> {
        let save_dir = save_dir.into();
        let algebra = complex.algebra();
        let min_degree = complex.min_degree();
        let zero_module = Arc::new(MuFreeModule::new(algebra, "F_{-1}".to_string(), min_degree));

        if let Some(p) = save_dir.write() {
            for subdir in SaveKind::resolution_data() {
                subdir.create_dir(p)?;
            }
        }

        Ok(Self {
            name: String::new(),
            complex,
            zero_module,
            should_save: save_dir.is_some(),
            save_dir,
            lock: Mutex::new(()),

            chain_maps: OnceBiVec::new(0),
            modules: OnceBiVec::new(0),
            differentials: OnceVec::new(),
            kernels: DashMap::new(),
            load_quasi_inverse: true,
        })
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn add_generators(&self, b: Bidegree, num_new_gens: usize) {
        let gen_names = (0..num_new_gens)
            .map(|idx| format!("x_{:#}", BidegreeGenerator::new(b, idx)))
            .collect();
        self.module(b.s())
            .add_generators(b.t(), num_new_gens, Some(gen_names));
    }

    /// This function prepares the Resolution object to perform computations up to the
    /// specified s degree. It does *not* perform any computations by itself. It simply lengthens
    /// the `OnceVec`s `modules`, `chain_maps`, etc. to the right length.
    fn extend_through_degree(&self, max_s: i32) {
        let min_degree = self.min_degree();

        for i in self.modules.len()..=max_s {
            self.modules.push(Arc::new(MuFreeModule::new(
                Arc::clone(&self.algebra()),
                format!("F{i}"),
                min_degree,
            )));
            self.chain_maps.push(Arc::new(MuFreeModuleHomomorphism::new(
                Arc::clone(&self.modules[i]),
                Arc::clone(&self.complex.module(i)),
                0,
            )));
        }

        if self.differentials.is_empty() {
            self.differentials
                .push(Arc::new(MuFreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[0]),
                    Arc::clone(&self.zero_module),
                    0,
                )));
        }

        for i in self.differentials.len() as i32..=max_s {
            self.differentials
                .push(Arc::new(MuFreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[i]),
                    Arc::clone(&self.modules[i - 1]),
                    0,
                )));
        }
    }

    /// Gets the kernel of the differential starting at $(s, t)$. If this was previously computed,
    /// we simply retrieve the value (and remove it from the cache). Otherwise, we compute the
    /// kernel. This requires the differential to be computed at $(s, t - 1)$, but not $(s, t)$
    /// itself. Indeed, the new generators added to $(s, t)$ are by construction not in the kernel.
    #[tracing::instrument(skip(self), fields(%b))]
    fn get_kernel(&self, b: Bidegree) -> Subspace {
        if let Some((_, v)) = self.kernels.remove(&b) {
            return v;
        }

        if b.s() == 0 {
            self.zero_module.extend_by_zero(b.t());
        }

        let p = self.prime();

        if let Some(dir) = self.save_dir.read()
            && let Some(mut f) = self.save_file(SaveKind::Kernel, b).open_file(dir.clone())
        {
            return Subspace::from_bytes(p, &mut f)
                .with_context(|| format!("Failed to read kernel at {b}"))
                .unwrap();
        }

        let complex = self.target();
        complex.compute_through_bidegree(b);

        let current_differential = self.differential(b.s());
        let current_chain_map = self.chain_map(b.s());

        let source = self.module(b.s());
        let target_cc = complex.module(b.s());
        let target_res = current_differential.target(); // This is self.module(s - 1) unless s = 0.

        source.compute_basis(b.t());
        target_res.compute_basis(b.t());

        let source_dimension = source.dimension(b.t());
        let target_cc_dimension = target_cc.dimension(b.t());
        let target_res_dimension = target_res.dimension(b.t());

        let mut matrix = AugmentedMatrix::<3>::new(
            p,
            source_dimension,
            [target_cc_dimension, target_res_dimension, source_dimension],
        );

        {
            let _guard = ParallelGuard::new();
            current_chain_map.get_matrix(matrix.segment(0, 0), b.t());
            current_differential.get_matrix(matrix.segment(1, 1), b.t());
        }
        matrix.segment(2, 2).add_identity();
        matrix.row_reduce();

        let kernel = matrix.compute_kernel();

        if self.should_save
            && let Some(dir) = self.save_dir.write()
        {
            let mut f = self
                .save_file(SaveKind::Kernel, b)
                .create_file(dir.clone(), true);
            kernel
                .to_bytes(&mut f)
                .with_context(|| format!("Failed to write kernel at {b}"))
                .unwrap();
        }
        kernel
    }

    /// Call our resolution $X$, and the chain complex to resolve $C$. This is a legitimate
    /// resolution if the map $f: X \to C$ induces an isomorphism on homology. This is the same as
    /// saying the cofiber is exact. The cofiber is given by the complex
    ///
    /// $$ X_s \oplus C_{s+1} \to X_{s-1} \oplus C_s \to X_{s-2} \oplus C_{s-1} \to \cdots $$
    ///
    /// where the differentials are given by
    ///
    /// $$ \begin{pmatrix} d_X & 0 \\\\ (-1)^s f & d_C \end{pmatrix} $$
    ///
    /// Our method of producing $X_{s, t}$ and the chain maps are as follows. Suppose we have already
    /// built the chain map and differential for $X_{s-1, t}$ and $X_{s, t-1}$. Since $X_s$ is a
    /// free module, the generators in degree $< t$ gives us a bunch of elements in $X_s$ already,
    /// and we know exactly where they get mapped to. Let $T$ be the $\\mathbb{F}_p$ vector space
    /// generated by these elements. Then we already have a map
    ///
    /// $$ T \to X_{s-1, t} \oplus C_{s, t}$$
    ///
    /// and we know this hits the kernel of the map
    ///
    /// $$ D = X_{s-1, t} \oplus C_{s, t} \to X_{s-2, t} \oplus C_{s-1, t}. $$
    ///
    /// What we need to do now is to add generators to $X_{s, t}$ to hit the entirity of this
    /// kernel.  Note that we don't *have* to do this. Some of the elements in the kernel might be
    /// hit by $C_{s+1, t}$ and we don't have to hit them, but we opt to add generators to hit it
    /// anyway.
    ///
    /// If we do it this way, then we know the composite of the map
    ///
    /// $$ T \to X_{s-1, t} \oplus C_{s, t} \to C_{s, t} $$
    ///
    /// has to be surjective, since the image of $C_{s, t}$ under $D$ is also in the image of $X_{s-1, t}$.
    /// So our first step is to add generators to $X_{s, t}$ such that this composite is
    /// surjective.
    ///
    /// After adding these generators, we need to decide where to send them to. We know their
    /// values in the $C_{s, t}$ component, but we need to use a quasi-inverse to find the element in
    /// $X_{s-1, t}$ that hits the corresponding image of $C_{s, t}$. This tells us the $X_{s-1,
    /// t}$ component.
    ///
    /// Finally, we need to add further generators to $X_{s, t}$ to hit all the elements in the
    /// kernel of
    ///
    /// $$ X_{s-1, t} \to X_{s-2, t} \oplus C_{s-1, t}. $$
    ///
    /// This kernel was recorded by the previous iteration of the method in `old_kernel`, so this
    /// step is doable as well.
    ///
    /// Note that if we add our new generators conservatively, then the kernel of the maps
    ///
    /// $$
    /// \begin{aligned}
    /// T &\to X_{s-1, t} \oplus C_{s, t} \\\\
    /// X_{s, t} &\to X_{s-1, t} \oplus C_{s, t}
    /// \end{aligned}
    /// $$
    /// agree.
    ///
    /// In the code, we first row reduce the matrix of the map from $T$. This lets us record
    /// the kernel which is what the function returns at the end. This computation helps us perform
    /// the future steps since we need to know about the cokernel of this map.
    ///
    /// # Arguments
    ///  * `s` - The s degree to calculate
    ///  * `t` - The t degree to calculate
    ///
    /// To run `step_resolution(s, t)`, we must have already had run `step_resolution(s, t - 1)`
    /// and `step_resolution(s - 1, t - 1)`. It is more efficient if we have in fact run
    /// `step_resolution(s - 1, t)`, so try your best to arrange calls to be run in this order.
    #[tracing::instrument(skip(self), fields(%b, num_new_gens, density))]
    fn step_resolution(&self, b: Bidegree) {
        if b.s() == 0 {
            self.zero_module.extend_by_zero(b.t());
        }

        let p = self.prime();
        let ring = self.base_ring();

        //                           current_chain_map
        //                X_{s, t} --------------------> C_{s, t}
        //                   |                               |
        //                   | current_differential          |
        //                   v                               v
        // old_kernel <= X_{s-1, t} -------------------> C_{s-1, t}

        let complex = self.target();
        complex.compute_through_bidegree(b);

        let current_differential = self.differential(b.s());
        let current_chain_map = self.chain_map(b.s());
        let complex_cur_differential = complex.differential(b.s());

        match current_differential.next_degree().cmp(&b.t()) {
            std::cmp::Ordering::Greater => {
                // Already computed this degree.
                return;
            }
            std::cmp::Ordering::Less => {
                // Haven't computed far enough yet
                panic!("We're not ready to compute bidegree {b} yet.");
            }
            std::cmp::Ordering::Equal => (),
        };

        let source = self.module(b.s());
        let target_cc = complex.module(b.s());
        let target_res = current_differential.target(); // This is self.module(s - 1) unless s = 0.

        source.compute_basis(b.t());

        // The Homomorphism matrix has size source_dimension x target_dimension, but we are going to augment it with an
        // identity matrix so that gives a matrix with dimensions source_dimension x (target_dimension + source_dimension).
        // Later we're going to write into this same matrix an isomorphism source/image + new vectors --> kernel
        // This has size target_dimension x (2*target_dimension).
        // This latter matrix may be used to find a preimage of an element under the differential.
        let source_dimension = source.dimension(b.t());
        let target_cc_dimension = target_cc.dimension(b.t());
        target_res.compute_basis(b.t());
        let target_res_dimension = target_res.dimension(b.t());

        if let Some(dir) = self.save_dir.read()
            && let Some(mut f) = self
                .save_file(SaveKind::Differential, b)
                .open_file(dir.clone())
        {
            let num_new_gens = f.read_u64::<LittleEndian>().unwrap() as usize;
            // This need not be equal to `target_res_dimension`. If we saved a big resolution
            // and now only want to load up to a small stem, then `target_res_dimension` will
            // be smaller. If we have previously saved a small resolution up to a stem and now
            // want to resolve further, it will be bigger.
            let saved_target_res_dimension = f.read_u64::<LittleEndian>().unwrap() as usize;
            assert_eq!(
                target_cc_dimension,
                f.read_u64::<LittleEndian>().unwrap() as usize,
                "Malformed data: mismatched augmentation target dimension"
            );

            self.add_generators(b, num_new_gens);

            let mut d_targets = Vec::with_capacity(num_new_gens);
            let mut a_targets = Vec::with_capacity(num_new_gens);

            for _ in 0..num_new_gens {
                d_targets
                    .push(FpVector::from_bytes(p, saved_target_res_dimension, &mut f).unwrap());
            }
            for _ in 0..num_new_gens {
                a_targets.push(FpVector::from_bytes(p, target_cc_dimension, &mut f).unwrap());
            }
            drop(f);
            current_differential.add_generators_from_rows(b.t(), d_targets);
            current_chain_map.add_generators_from_rows(b.t(), a_targets);

            // res qi
            if self.load_quasi_inverse {
                if let Some(mut f) = self.save_file(SaveKind::ResQi, b).open_file(dir.clone()) {
                    let res_qi = QuasiInverse::from_bytes(p, &mut f).unwrap();

                    assert_eq!(
                        res_qi.source_dimension(),
                        source_dimension + num_new_gens,
                        "Malformed data: mismatched source dimension in resolution qi at {b}"
                    );

                    current_differential.set_quasi_inverse(b.t(), Some(res_qi));
                } else {
                    current_differential.set_quasi_inverse(b.t(), None);
                }
            } else {
                current_differential.set_quasi_inverse(b.t(), None);
            }

            if let Some(mut f) = self
                .save_file(SaveKind::AugmentationQi, b)
                .open_file(dir.clone())
            {
                let cm_qi = QuasiInverse::from_bytes(p, &mut f).unwrap();

                assert_eq!(
                    cm_qi.target_dimension(),
                    target_cc_dimension,
                    "Malformed data: mismatched augmentation target dimension in qi at {b}"
                );
                assert_eq!(
                    cm_qi.source_dimension(),
                    source_dimension + num_new_gens,
                    "Malformed data: mismatched source dimension in augmentation qi at {b}"
                );

                current_chain_map.set_quasi_inverse(b.t(), Some(cm_qi));
            } else {
                current_chain_map.set_quasi_inverse(b.t(), None);
            }

            current_differential.set_kernel(b.t(), None);
            current_differential.set_image(b.t(), None);

            current_chain_map.set_kernel(b.t(), None);
            current_chain_map.set_image(b.t(), None);
            return;
        }

        let field = Field::new(p);

        // Realize the step's chain map `f` and differential `d` as a vector-space map over the base
        // ring (block 2).
        let matrix = {
            let _guard = ParallelGuard::new();
            field.differential_matrix(
                source_dimension,
                target_cc_dimension,
                target_res_dimension,
                MAX_NEW_GENS,
                |i, row| current_chain_map.apply_to_basis_element(row, ring.one(), b.t(), i),
                |i, row| current_differential.apply_to_basis_element(row, ring.one(), b.t(), i),
            )
        };

        // Compute the kernel (block 3), caching it for the next homological degree.
        if !self.has_computed_bidegree(b + Bidegree::s_t(1, 0)) {
            let kernel = field.step_kernel(&matrix);
            if self.should_save
                && let Some(dir) = self.save_dir.write()
            {
                let mut f = self
                    .save_file(SaveKind::Kernel, b)
                    .create_file(dir.clone(), true);

                kernel
                    .to_bytes(&mut f)
                    .with_context(|| format!("Failed to write kernel at {b}"))
                    .unwrap();
            }

            self.kernels.insert(b, kernel);
        }

        // Construct the next stage (block 4): new generators surjecting onto C_{s,t} and hitting the
        // old kernel, their differential, and the quasi-inverses of `f` and `d`.
        let complex_differential_target_dim = complex_cur_differential.target().dimension(b.t());
        let next = if b.s() > 0 {
            let prev_chain_map = self.chain_map(b.s() - 1);
            let previous_kernel = self.get_kernel(b - Bidegree::s_t(1, 0));
            field.construct_next_stage(
                matrix,
                NextStageInput {
                    previous_chain_map_quasi_inverse: prev_chain_map.quasi_inverse(b.t()),
                    previous_kernel: Some(&previous_kernel),
                    complex_differential_target_dim,
                },
                |column, dfx| {
                    complex_cur_differential.apply_to_basis_element(dfx, ring.one(), b.t(), column);
                },
                MAX_NEW_GENS,
            )
        } else {
            field.construct_next_stage(
                matrix,
                NextStageInput {
                    previous_chain_map_quasi_inverse: None,
                    previous_kernel: None,
                    complex_differential_target_dim,
                },
                |_, _| {},
                MAX_NEW_GENS,
            )
        };

        let num_new_gens = next.num_new_gens;
        self.add_generators(b, num_new_gens);
        current_chain_map.add_generators_from_rows(b.t(), next.chain_map_rows);
        current_differential.add_generators_from_rows(b.t(), next.differential_rows);
        let cm_qi = next.chain_map_quasi_inverse;
        let res_qi = next.differential_quasi_inverse;

        tracing::Span::current().record("num_new_gens", num_new_gens);
        tracing::Span::current().record(
            "density",
            current_differential.differential_density(b.t()) * 100.0,
        );

        if self.should_save
            && let Some(dir) = self.save_dir.write()
        {
            // Write differentials last, because if we were terminated halfway, we want the
            // differentials to exist iff everything has been written. However, we start by
            // opening the differentials first to make sure we are not overwriting anything.

            // Open differentials file
            let mut f = self
                .save_file(SaveKind::Differential, b)
                .create_file(dir.clone(), false);

            // Write resolution qi
            res_qi
                .to_bytes(
                    &mut self
                        .save_file(SaveKind::ResQi, b)
                        .create_file(dir.clone(), true),
                )
                .unwrap();

            // Write augmentation qi
            cm_qi
                .to_bytes(
                    &mut self
                        .save_file(SaveKind::AugmentationQi, b)
                        .create_file(dir.clone(), true),
                )
                .unwrap();

            // Write differentials
            f.write_u64::<LittleEndian>(num_new_gens as u64).unwrap();
            f.write_u64::<LittleEndian>(target_res_dimension as u64)
                .unwrap();
            f.write_u64::<LittleEndian>(target_cc_dimension as u64)
                .unwrap();

            for n in 0..num_new_gens {
                current_differential
                    .output(b.t(), n)
                    .to_bytes(&mut f)
                    .unwrap();
            }
            for n in 0..num_new_gens {
                current_chain_map.output(b.t(), n).to_bytes(&mut f).unwrap();
            }
            drop(f);

            // Delete kernel
            if b.s() > 0 {
                self.save_file(SaveKind::Kernel, b - Bidegree::s_t(1, 0))
                    .delete_file(dir.clone())
                    .unwrap();
            }
        }

        if self.load_quasi_inverse {
            current_differential.set_quasi_inverse(b.t(), Some(res_qi));
        } else {
            current_differential.set_quasi_inverse(b.t(), None);
        }

        // This tends to be small and is always needed if the target is not concentrated in a
        // single homological degree
        current_chain_map.set_quasi_inverse(b.t(), Some(cm_qi));
        current_chain_map.set_kernel(b.t(), None);
        current_chain_map.set_image(b.t(), None);

        current_differential.set_kernel(b.t(), None);
        current_differential.set_image(b.t(), None);
    }

    pub fn compute_through_bidegree_with_callback(
        &self,
        max: Bidegree,
        mut cb: impl FnMut(Bidegree),
    ) {
        let min_degree = self.min_degree();
        let _lock = self.lock.lock();

        self.target().compute_through_bidegree(max);
        self.extend_through_degree(max.s());
        self.algebra().compute_basis(max.t() - min_degree);

        let tracing_span = tracing::Span::current();
        maybe_rayon::in_place_scope(|scope| {
            let _tracing_guard = tracing_span.enter();

            // Things that we have finished computing.
            let mut progress: Vec<i32> = vec![min_degree - 1; max.s() as usize + 1];
            // We will kickstart the process by pretending we have computed (0, min_degree - 1). So
            // we must pretend we have only computed up to (0, min_degree - 2);
            progress[0] = min_degree - 2;

            let (sender, receiver) = mpsc::channel();
            SenderData::send(Bidegree::s_t(0, min_degree - 1), false, sender);

            let f = |b, sender| {
                if self.has_computed_bidegree(b) {
                    SenderData::send(b, false, sender);
                } else {
                    let tracing_span = tracing_span.clone();
                    scope.spawn(move |_| {
                        let _tracing_guard = tracing_span.enter();
                        if crate::utils::parallel::is_in_parallel() {
                            SenderData::send_retry(b, sender);
                            return;
                        }
                        self.step_resolution(b);
                        SenderData::send(b, true, sender);
                    });
                }
            };

            while let Ok(SenderData {
                b,
                new,
                retry,
                sender,
            }) = receiver.recv()
            {
                if retry {
                    f(b, sender);
                    continue;
                }
                assert!(progress[b.s() as usize] == b.t() - 1);
                progress[b.s() as usize] = b.t();

                if b.t() < max.t() && (b.s() == 0 || progress[b.s() as usize - 1] > b.t()) {
                    // We are computing a normal step
                    f(b + Bidegree::s_t(0, 1), sender.clone());
                }
                if b.s() < max.s() && progress[b.s() as usize + 1] == b.t() - 1 {
                    f(b + Bidegree::s_t(1, 0), sender);
                }
                if new {
                    cb(b);
                }
            }
        });
    }

    /// This function resolves up till a fixed stem instead of a fixed t.
    #[tracing::instrument(skip(self), fields(self = self.name, %max))]
    pub fn compute_through_stem(&self, max: Bidegree) {
        self.compute_through_stem_with_callback(max, |_| ());
    }

    pub fn compute_through_stem_with_callback(&self, max: Bidegree, mut cb: impl FnMut(Bidegree)) {
        let min_degree = self.min_degree();
        let _lock = self.lock.lock();

        self.target().compute_through_bidegree(max);
        self.extend_through_degree(max.s());
        self.algebra().compute_basis(max.t() - min_degree);

        let tracing_span = tracing::Span::current();
        maybe_rayon::in_place_scope(|scope| {
            let _tracing_guard = tracing_span.enter();

            // Things that we have finished computing.
            let mut progress: Vec<i32> = vec![min_degree - 1; max.s() as usize + 1];
            // We will kickstart the process by pretending we have computed (0, min_degree - 1). So
            // we must pretend we have only computed up to (0, min_degree - 2);
            progress[0] = min_degree - 2;

            let (sender, receiver) = mpsc::channel();
            SenderData::send(Bidegree::s_t(0, min_degree - 1), false, sender);

            let f = |b, sender| {
                if self.has_computed_bidegree(b) {
                    SenderData::send(b, false, sender);
                } else {
                    let tracing_span = tracing_span.clone();
                    scope.spawn(move |_| {
                        let _tracing_guard = tracing_span.enter();
                        if crate::utils::parallel::is_in_parallel() {
                            SenderData::send_retry(b, sender);
                            return;
                        }
                        self.step_resolution(b);
                        SenderData::send(b, true, sender);
                    });
                }
            };

            while let Ok(SenderData {
                b,
                new,
                retry,
                sender,
            }) = receiver.recv()
            {
                if retry {
                    f(b, sender);
                    continue;
                }
                assert!(progress[b.s() as usize] == b.t() - 1);
                progress[b.s() as usize] = b.t();

                // How far we are from the last one for this s.
                let distance = max.n() - b.n() + 1;

                if b.s() < max.s() && progress[b.s() as usize + 1] == b.t() - 1 {
                    f(b + Bidegree::s_t(1, 0), sender.clone());
                }

                if distance > 1 && (b.s() == 0 || progress[b.s() as usize - 1] > b.t()) {
                    // We are computing a normal step
                    f(b + Bidegree::s_t(0, 1), sender);
                } else if distance == 1 && b.s() < max.s() {
                    // We compute the kernel at the edge if necessary
                    let next_b = b + Bidegree::s_t(0, 1);
                    if !self.has_computed_bidegree(b + Bidegree::s_t(1, 1))
                        && (self.save_dir.is_none()
                            || !self
                                .save_file(SaveKind::Differential, b + Bidegree::s_t(1, 1))
                                .exists(self.save_dir.read().cloned().unwrap()))
                    {
                        scope.spawn(move |_| {
                            self.kernels.insert(next_b, self.get_kernel(next_b));
                            SenderData::send(next_b, false, sender);
                        });
                    } else {
                        SenderData::send(next_b, false, sender);
                    }
                }
                if new {
                    cb(b);
                }
            }
        });
    }
}

impl<const U: bool, CC: ChainComplex> ChainComplex for MuResolution<U, CC>
where
    CC::Algebra: MuAlgebra<U>,
{
    type Algebra = CC::Algebra;
    type Homomorphism = MuFreeModuleHomomorphism<U, MuFreeModule<U, Self::Algebra>>;
    type Module = MuFreeModule<U, Self::Algebra>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.target().algebra()
    }

    fn module(&self, s: i32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s])
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn min_degree(&self) -> i32 {
        self.target().min_degree()
    }

    fn has_computed_bidegree(&self, b: Bidegree) -> bool {
        self.differentials.len() > b.s() as usize && self.differential(b.s()).next_degree() > b.t()
    }

    fn differential(&self, s: i32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    #[tracing::instrument(skip(self), fields(self = self.name, %b))]
    fn compute_through_bidegree(&self, b: Bidegree) {
        self.compute_through_bidegree_with_callback(b, |_| ())
    }

    fn next_homological_degree(&self) -> i32 {
        self.modules.len()
    }

    fn apply_quasi_inverse<T, S>(&self, results: &mut [T], b: Bidegree, inputs: &[S]) -> bool
    where
        for<'a> &'a mut T: Into<FpSliceMut<'a>>,
        for<'a> &'a S: Into<FpSlice<'a>>,
    {
        assert_eq!(results.len(), inputs.len());

        if let Some(qi) = self.differential(b.s()).quasi_inverse(b.t()) {
            for (input, result) in inputs.iter().zip_eq(results) {
                qi.apply(result.into(), 1, input.into());
            }
            true
        } else if let Some(dir) = self.save_dir.read() {
            if let Some(mut f) = self.save_file(SaveKind::ResQi, b).open_file(dir.clone()) {
                QuasiInverse::stream_quasi_inverse(self.prime(), &mut f, results, inputs).unwrap();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn save_dir(&self) -> &SaveDirectory {
        &self.save_dir
    }
}

impl<const U: bool, CC: ChainComplex> AugmentedChainComplex for MuResolution<U, CC>
where
    CC::Algebra: MuAlgebra<U>,
{
    type ChainMap = MuFreeModuleHomomorphism<U, CC::Module>;
    type TargetComplex = CC;

    fn target(&self) -> Arc<Self::TargetComplex> {
        Arc::clone(&self.complex)
    }

    fn chain_map(&self, s: i32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s])
    }
}

// The secondary lift of a `Resolution` lives here, beside the primary object it lifts, rather than
// in the monolithic `secondary` module. This keeps `secondary.rs` to the shared lift machinery and
// pairs each variant with its primary for locality. The module is `pub(crate)`; `SecondaryResolution`
// is re-exported from `crate::secondary` so the public API path is unchanged.
pub(crate) mod secondary {
    use std::sync::Arc;

    use algebra::{module::Module, pair_algebra::PairAlgebra};
    use dashmap::DashMap;
    use fp::vector::FpVector;
    use once::OnceBiVec;
    use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator, BidegreeRange};

    use crate::{
        chain_complex::FreeChainComplex,
        save::{SaveDirectory, SaveKind},
        secondary::{CompositeData, SecondaryHomotopy, SecondaryLift},
    };

    pub struct SecondaryResolution<CC: FreeChainComplex>
    where
        CC::Algebra: PairAlgebra,
    {
        underlying: Arc<CC>,
        /// s -> t -> idx -> homotopy
        pub(crate) homotopies: OnceBiVec<SecondaryHomotopy<CC::Algebra>>,
        intermediates: DashMap<BidegreeGenerator, FpVector>,
    }

    impl<CC: FreeChainComplex> SecondaryLift for SecondaryResolution<CC>
    where
        CC::Algebra: PairAlgebra,
    {
        type Algebra = CC::Algebra;
        type Source = CC;
        type Target = CC;
        type Underlying = CC;

        fn underlying(&self) -> Arc<CC> {
            Arc::clone(&self.underlying)
        }

        fn algebra(&self) -> Arc<Self::Algebra> {
            self.underlying.algebra()
        }

        fn source(&self) -> Arc<Self::Source> {
            Arc::clone(&self.underlying)
        }

        fn target(&self) -> Arc<Self::Target> {
            Arc::clone(&self.underlying)
        }

        fn shift(&self) -> Bidegree {
            Bidegree::s_t(2, 0)
        }

        fn max(&self) -> BidegreeRange<'_, Self> {
            BidegreeRange::new(
                self,
                self.underlying.next_homological_degree(),
                &|selff, s| {
                    std::cmp::min(
                        selff.underlying.module(s).max_computed_degree(),
                        selff.underlying.module(s - 2).max_computed_degree() + 1,
                    ) + 1
                },
            )
        }

        fn homotopies(&self) -> &OnceBiVec<SecondaryHomotopy<CC::Algebra>> {
            &self.homotopies
        }

        fn intermediates(&self) -> &DashMap<BidegreeGenerator, FpVector> {
            &self.intermediates
        }

        fn save_dir(&self) -> &SaveDirectory {
            self.underlying.save_dir()
        }

        fn composite(&self, s: i32) -> CompositeData<CC::Algebra> {
            let d1 = self.underlying.differential(s);
            let d0 = self.underlying.differential(s - 1);
            vec![(1, d1, d0)]
        }

        fn compute_intermediate(&self, g: BidegreeGenerator) -> FpVector {
            let p = self.prime();
            let target = self.underlying.module(g.s() - 3);
            let mut result = FpVector::new(p, target.dimension(g.t() - 1));
            let d = self.underlying.differential(g.s());
            self.homotopies[g.s() - 1].act(
                result.as_slice_mut(),
                1,
                g.t(),
                d.output(g.t(), g.idx()).as_slice(),
                false,
            );
            result
        }
    }

    impl<CC: FreeChainComplex> SecondaryResolution<CC>
    where
        CC::Algebra: PairAlgebra,
    {
        pub fn new(cc: Arc<CC>) -> Self {
            if let Some(p) = cc.save_dir().write() {
                for subdir in SaveKind::secondary_data() {
                    subdir.create_dir(p).unwrap();
                }
            }

            Self {
                underlying: cc,
                homotopies: OnceBiVec::new(2),
                intermediates: DashMap::new(),
            }
        }

        pub fn homotopy(&self, s: i32) -> &SecondaryHomotopy<CC::Algebra> {
            &self.homotopies[s]
        }

        pub fn e3_page(&self) -> sseq::Sseq<2, sseq::Adams> {
            let p = self.prime();

            let mut sseq = self.underlying.to_sseq();

            let mut source_vec = FpVector::new(p, 0);
            let mut target_vec = FpVector::new(p, 0);

            for b in self.underlying.iter_stem() {
                if b.t() > 0
                    && self
                        .underlying
                        .has_computed_bidegree(b + Bidegree::n_s(-1, 2))
                {
                    let m = self.homotopy(b.s() + 2).homotopies.hom_k(b.t());
                    if m.is_empty() || m[0].is_empty() {
                        continue;
                    }

                    source_vec.set_scratch_vector_size(m.len());
                    target_vec.set_scratch_vector_size(m[0].len());

                    for (i, row) in m.into_iter().enumerate() {
                        source_vec.set_to_zero();
                        source_vec.set_entry(i, 1);
                        target_vec.copy_from_slice(&row);

                        let source = BidegreeElement::new(b, source_vec);
                        sseq.add_differential(2, &source, target_vec.as_slice());

                        source_vec = source.into_vec();
                    }
                }
            }

            for b in self.underlying.iter_stem() {
                if sseq.invalid(b) {
                    sseq.update_degree(b);
                }
            }
            sseq
        }
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;
    use crate::{chain_complex::FreeChainComplex, utils::construct_standard};

    #[test]
    fn test_restart_stem() {
        let res = construct_standard::<false, _, _>("S_2", None).unwrap();
        res.compute_through_stem(Bidegree::n_s(14, 8));
        res.compute_through_bidegree(Bidegree::s_t(5, 19));

        expect![[r#"
            ·                             
            ·                     ·       
            ·                   · ·     · 
            ·                 ·   ·     · 
            ·             ·   ·         · · 
            ·     ·       · · ·         · ·   
            ·   · ·     · · ·           · · ·   
            · ·   ·       ·               ·       
            ·                                       
        "#]]
        .assert_eq(&res.graded_dimension_string());
    }

    #[test]
    fn test_apply_quasi_inverse() {
        let tempdir = tempfile::TempDir::new().unwrap();

        let mut res =
            construct_standard::<false, _, _>("S_2", Some(tempdir.path().into())).unwrap();
        res.load_quasi_inverse = false;

        let b = Bidegree::s_t(8, 8);
        res.compute_through_bidegree(b);

        assert!(res.differential(8).quasi_inverse(8).is_none());

        let v = FpVector::new(res.prime(), res.module(7).dimension(8));
        let mut w = FpVector::new(res.prime(), res.module(8).dimension(8));

        assert!(res.apply_quasi_inverse(&mut [w.as_slice_mut()], b, &[v.as_slice()]));
        assert!(w.is_zero());
    }
}
