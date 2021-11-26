use std::sync::{Arc, Mutex};

use crate::chain_complex::{AugmentedChainComplex, ChainComplex};
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use fp::matrix::{AugmentedMatrix, Subspace};
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use once::OnceVec;

use dashmap::DashMap;

#[cfg(feature = "concurrent")]
use crossbeam_channel::{unbounded, Receiver};

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

/// This is the maximum number of new generators we expect in each bidegree. This affects how much
/// space we allocate when we are extending our resolutions. Having more than this many new
/// generators will result in a slowdown but not an error. It is relatively cheap to increment this
/// number if needs be, but up to the 140th stem we only see at most 8 new generators.
const MAX_NEW_GENS: usize = 10;

/// A resolution of a chain complex.
pub struct Resolution<CC: ChainComplex> {
    lock: Mutex<()>,
    complex: Arc<CC>,
    modules: OnceVec<Arc<FreeModule<<CC::Module as Module>::Algebra>>>,
    zero_module: Arc<FreeModule<<CC::Module as Module>::Algebra>>,
    chain_maps: OnceVec<Arc<FreeModuleHomomorphism<CC::Module>>>,
    differentials:
        OnceVec<Arc<FreeModuleHomomorphism<FreeModule<<CC::Module as Module>::Algebra>>>>,

    ///  For each *internal* degree, store the kernel of the most recently calculated chain map as
    ///  returned by `generate_old_kernel_and_compute_new_kernel`, to be used if we run
    ///  compute_through_degree again.
    kernels: DashMap<(u32, i32), Subspace>,
}

impl<CC: ChainComplex> Resolution<CC> {
    pub fn new(complex: Arc<CC>) -> Self {
        let algebra = complex.algebra();
        let min_degree = complex.min_degree();
        let zero_module = Arc::new(FreeModule::new(algebra, "F_{-1}".to_string(), min_degree));

        Self {
            complex,
            zero_module,
            lock: Mutex::new(()),

            chain_maps: OnceVec::new(),
            modules: OnceVec::new(),
            differentials: OnceVec::new(),
            kernels: DashMap::new(),
        }
    }

    /// This function prepares the Resolution object to perform computations up to the
    /// specified s degree. It does *not* perform any computations by itself. It simply lengthens
    /// the `OnceVec`s `modules`, `chain_maps`, etc. to the right length.
    fn extend_through_degree(&self, max_s: u32) {
        let min_degree = self.min_degree();

        for i in self.modules.len() as u32..=max_s {
            self.modules.push(Arc::new(FreeModule::new(
                Arc::clone(&self.algebra()),
                format!("F{}", i),
                min_degree,
            )));
            self.chain_maps.push(Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[i]),
                Arc::clone(&self.complex.module(i)),
                0,
            )));
        }

        if self.differentials.is_empty() {
            self.differentials
                .push(Arc::new(FreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[0u32]),
                    Arc::clone(&self.zero_module),
                    0,
                )));
        }

        for i in self.differentials.len() as u32..=max_s {
            self.differentials
                .push(Arc::new(FreeModuleHomomorphism::new(
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
    fn get_kernel(&self, s: u32, t: i32) -> Subspace {
        if let Some((_, v)) = self.kernels.remove(&(s, t)) {
            return v;
        }

        if s == 0 {
            self.zero_module.extend_by_zero(t);
        }

        let p = self.prime();

        let complex = self.complex();
        complex.compute_through_bidegree(s, t);

        let current_differential = self.differential(s);
        let current_chain_map = self.chain_map(s);

        let source = self.module(s);
        let target_cc = complex.module(s);
        let target_res = current_differential.target(); // This is self.module(s - 1) unless s = 0.

        source.extend_table_entries(t);
        target_res.extend_table_entries(t);

        let source_dimension = source.dimension(t);
        let target_cc_dimension = target_cc.dimension(t);
        let target_res_dimension = target_res.dimension(t);

        let mut matrix = AugmentedMatrix::<3>::new(
            p,
            source_dimension,
            [target_cc_dimension, target_res_dimension, source_dimension],
        );

        current_chain_map.get_matrix(&mut matrix.segment(0, 0), t);
        current_differential.get_matrix(&mut matrix.segment(1, 1), t);
        matrix.segment(2, 2).add_identity();
        matrix.row_reduce();

        matrix.compute_kernel()
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
    fn step_resolution(&self, s: u32, t: i32) {
        if s == 0 {
            self.zero_module.extend_by_zero(t);
        }

        let p = self.prime();

        //                           current_chain_map
        //                X_{s, t} --------------------> C_{s, t}
        //                   |                               |
        //                   | current_differential          |
        //                   v                               v
        // old_kernel <= X_{s-1, t} -------------------> C_{s-1, t}

        let complex = self.complex();
        complex.compute_through_bidegree(s, t);

        let current_differential = self.differential(s);
        let current_chain_map = self.chain_map(s);
        let complex_cur_differential = complex.differential(s);

        match current_differential.next_degree().cmp(&t) {
            std::cmp::Ordering::Greater => {
                // Already computed this degree.
                return;
            }
            std::cmp::Ordering::Less => {
                // Haven't computed far enough yet
                panic!("We're not ready to compute bidegree ({s}, {t}) yet.");
            }
            std::cmp::Ordering::Equal => (),
        };

        let source = self.module(s);
        let target_cc = complex.module(s);
        let target_res = current_differential.target(); // This is self.module(s - 1) unless s = 0.

        source.extend_table_entries(t);

        // The Homomorphism matrix has size source_dimension x target_dimension, but we are going to augment it with an
        // identity matrix so that gives a matrix with dimensions source_dimension x (target_dimension + source_dimension).
        // Later we're going to write into this same matrix an isomorphism source/image + new vectors --> kernel
        // This has size target_dimension x (2*target_dimension).
        // This latter matrix may be used to find a preimage of an element under the differential.
        let source_dimension = source.dimension(t);
        let target_cc_dimension = target_cc.dimension(t);
        target_res.extend_table_entries(t);
        let target_res_dimension = target_res.dimension(t);

        let mut matrix = AugmentedMatrix::<3>::new_with_capacity(
            p,
            source_dimension,
            &[target_cc_dimension, target_res_dimension, source_dimension],
            source_dimension + MAX_NEW_GENS,
            MAX_NEW_GENS,
        );
        // Get the map (d, f) : X_{s, t} -> X_{s-1, t} (+) C_{s, t} into matrix

        current_chain_map.get_matrix(&mut matrix.segment(0, 0), t);
        current_differential.get_matrix(&mut matrix.segment(1, 1), t);
        matrix.segment(2, 2).add_identity();

        matrix.row_reduce();

        if !self.has_computed_bidegree(s + 1, t) {
            self.kernels.insert((s, t), matrix.compute_kernel());
        }

        // Now add generators to surject onto C_{s, t}.
        // (For now we are just adding the eventual images of the new generators into matrix, we will update
        // X_{s,t} and f later).
        // We record which pivots exactly we added so that we can walk over the added genrators in a moment and
        // work out what dX should to to each of them.
        let cc_new_gens = matrix.extend_to_surjection(0, target_cc_dimension, MAX_NEW_GENS);

        let mut res_new_gens = Vec::new();

        if s > 0 {
            if !cc_new_gens.is_empty() {
                // Now we need to make sure that we have a chain homomorphism. Each generator x we just added to
                // X_{s,t} has a nontrivial image f(x) \in C_{s,t}. We need to set d(x) so that f(dX(x)) = dC(f(x)).
                // So we set dX(x) = f^{-1}(dC(f(x)))
                let prev_chain_map = self.chain_map(s - 1);
                let quasi_inverse = prev_chain_map.quasi_inverse(t).unwrap();

                let dfx_dim = complex_cur_differential.target().dimension(t);
                let mut dfx = FpVector::new(self.prime(), dfx_dim);

                for (i, &column) in cc_new_gens.iter().enumerate() {
                    complex_cur_differential.apply_to_basis_element(
                        dfx.as_slice_mut(),
                        1,
                        t,
                        column,
                    );
                    quasi_inverse.apply(
                        matrix.row_segment(source_dimension + i, 1, 1),
                        1,
                        dfx.as_slice(),
                    );
                    dfx.set_to_zero();
                }
            }

            // Now we add new generators to hit any cycles in old_kernel that we don't want in our homology.
            //
            // At this point the matrix is not quite row reduced and the pivots are not correct.
            // However, extend_image only needs the sign of the pivots within the column range,
            // which are still correct. The point is that the rows we added all have pivot columns
            // in the first segment.
            res_new_gens = matrix.inner.extend_image(
                matrix.start[1],
                matrix.end[1],
                &self.get_kernel(s - 1, t),
                MAX_NEW_GENS,
            );
        }
        let num_new_gens = cc_new_gens.len() + res_new_gens.len();
        source.add_generators(t, num_new_gens, None);

        let new_rows = source_dimension + num_new_gens;

        current_chain_map.add_generators_from_matrix_rows(
            t,
            matrix.segment(0, 0).row_slice(source_dimension, new_rows),
        );
        current_differential.add_generators_from_matrix_rows(
            t,
            matrix.segment(1, 1).row_slice(source_dimension, new_rows),
        );

        if num_new_gens > 0 {
            // Fix up the augmentation
            let columns = matrix.columns();
            matrix.extend_column_dimension(columns + num_new_gens);

            for i in source_dimension..new_rows {
                matrix.inner[i].set_entry(matrix.start[2] + i, 1);
            }

            // We are now supposed to row reduce the matrix. However, running the full row
            // reduction algorithm is wasteful, since we have only added a few rows and the rest is
            // intact.
            //
            // The new resolution rows are all zero in the existing pivot columns. Indeed,
            // the resolution generators are mapped to generators of the kernel, which are zero in
            // pivot columns of the kernel matrix. But the old image is a subspace of the kernel,
            // so its pivot columns are a subset of the pivot columns of the kernel matrix.
            //
            // So we clear the new cc rows using the old rows.
            for k in source_dimension..source_dimension + cc_new_gens.len() {
                for column in matrix.start[1]..matrix.end[1] {
                    let row = matrix.pivots()[column];
                    if row < 0 {
                        continue;
                    }
                    let row = row as usize;
                    unsafe {
                        matrix.row_op(k, row, column, p);
                    }
                }
            }

            // Now use the new resolution rows to reduce the old rows and the cc rows.
            let first_res_row = source_dimension + cc_new_gens.len();
            for (source_row, &pivot_col) in res_new_gens.iter().enumerate() {
                for target_row in 0..first_res_row {
                    unsafe {
                        matrix.row_op(target_row, source_row + first_res_row, pivot_col, p);
                    }
                }
            }

            // We are now almost in RREF, except we need to permute the rows.
            let mut new_gens = cc_new_gens.into_iter().chain(res_new_gens).enumerate();
            let (mut next_new_row, mut next_new_col) = new_gens.next().unwrap();
            let mut next_old_row = 0;

            for old_col in 0..matrix.columns() {
                if old_col == next_new_col {
                    matrix[next_old_row..=source_dimension + next_new_row].rotate_right(1);
                    matrix.pivots_mut()[old_col] = next_old_row as isize;
                    match new_gens.next() {
                        Some((x, y)) => {
                            next_new_row = x;
                            next_new_col = y;
                        }
                        None => {
                            for entry in &mut matrix.pivots_mut()[old_col + 1..] {
                                if *entry >= 0 {
                                    *entry += next_new_row as isize + 1;
                                }
                            }
                            break;
                        }
                    }
                    next_old_row += 1;
                } else if matrix.pivots()[old_col] >= 0 {
                    matrix.pivots_mut()[old_col] += next_new_row as isize;
                    next_old_row += 1;
                }
            }
        }

        let (cm_qi, res_qi) = matrix.compute_quasi_inverses();

        current_chain_map.set_quasi_inverse(t, Some(cm_qi));
        current_chain_map.set_kernel(t, None);
        current_chain_map.set_image(t, None);
        current_differential.set_quasi_inverse(t, Some(res_qi));
        current_differential.set_kernel(t, None);
        current_differential.set_image(t, None);
    }

    pub fn cocycle_string(&self, hom_deg: u32, int_deg: i32, idx: usize) -> String {
        let d = self.differential(hom_deg);
        let target = d.target();
        let result_vector = d.output(int_deg, idx);

        target.element_to_string_pretty(hom_deg, int_deg, result_vector.as_slice())
    }

    pub fn complex(&self) -> Arc<CC> {
        Arc::clone(&self.complex)
    }

    pub fn number_of_gens_in_bidegree(&self, s: u32, t: i32) -> usize {
        self.module(s).number_of_gens_in_degree(t)
    }

    pub fn prime(&self) -> ValidPrime {
        self.complex.prime()
    }

    #[cfg(feature = "concurrent")]
    pub fn compute_through_bidegree_concurrent_with_callback(
        &self,
        max_s: u32,
        max_t: i32,
        bucket: &TokenBucket,
        mut cb: impl FnMut(u32, i32),
    ) {
        let min_degree = self.min_degree();
        let _lock = self.lock.lock();

        self.complex().compute_through_bidegree(max_s, max_t);
        self.extend_through_degree(max_s);
        self.algebra().compute_basis(max_t - min_degree);

        crossbeam_utils::thread::scope(|s| {
            let (pp_sender, pp_receiver) = unbounded();
            let mut last_receiver: Option<Receiver<()>> = None;
            for t in min_degree..=max_t {
                let (sender, receiver) = unbounded();

                let pp_sender = pp_sender.clone();
                s.builder()
                    .name(format!("t = {}", t))
                    .spawn(move |_| {
                        let mut token = bucket.take_token();
                        for s in 0..=max_s {
                            token = bucket.recv_or_release(token, &last_receiver);
                            if !self.has_computed_bidegree(s, t) {
                                self.step_resolution(s, t);

                                pp_sender.send((s, t)).unwrap();
                            }
                            sender.send(()).unwrap();
                        }
                    })
                    .unwrap();
                last_receiver = Some(receiver);
            }
            // We drop this pp_sender, so that when all previous threads end, no pp_sender's are
            // present, so pp_receiver terminates.
            drop(pp_sender);

            for (s, t) in pp_receiver {
                cb(s, t);
            }
        })
        .unwrap();
    }

    pub fn compute_through_bidegree_with_callback(
        &self,
        max_s: u32,
        max_t: i32,
        mut cb: impl FnMut(u32, i32),
    ) {
        let min_degree = self.min_degree();
        let _lock = self.lock.lock();

        self.complex().compute_through_bidegree(max_s, max_t);
        self.extend_through_degree(max_s);
        self.algebra().compute_basis(max_t - min_degree);

        for t in min_degree..=max_t {
            for s in 0..=max_s {
                if self.has_computed_bidegree(s, t) {
                    continue;
                }
                self.step_resolution(s, t);
                cb(s, t);
            }
        }
    }

    /// This function resolves up till a fixed stem instead of a fixed t. It is an error to
    /// attempt to resolve further after this is called, and will result in a deadlock.
    pub fn compute_through_stem(&self, max_s: u32, max_n: i32) {
        let min_degree = self.min_degree();
        let _lock = self.lock.lock();
        let max_t = max_s as i32 + max_n;
        self.complex().compute_through_bidegree(max_s, max_t);
        self.extend_through_degree(max_s);
        self.algebra().compute_basis(max_t - min_degree);

        for t in min_degree..=max_t {
            let start_s = std::cmp::max(0, t - max_n) as u32;
            for s in start_s..=max_s {
                if self.has_computed_bidegree(s, t) {
                    continue;
                }
                self.step_resolution(s, t);
            }
        }
    }

    /// Computes the filtration one product. This returns None if the source or target is out of
    /// range.
    pub fn filtration_one_product(
        &self,
        op_deg: i32,
        op_idx: usize,
        target_s: u32,
        target_t: i32,
    ) -> Option<Vec<Vec<u32>>> {
        let source_t = target_t - op_deg;
        let source_s = target_s.overflowing_sub(1).0;
        if target_s == 0
            || target_s >= self.next_homological_degree()
            || source_t - (source_s as i32) < self.min_degree()
        {
            return None;
        }

        let source = self.module(target_s - 1);
        let target = self.module(target_s);

        if target_t > target.max_computed_degree() {
            return None;
        }

        let source_dim = source.number_of_gens_in_degree(source_t);
        let target_dim = target.number_of_gens_in_degree(target_t);

        let d = self.differential(target_s);

        let mut products = vec![Vec::with_capacity(target_dim); source_dim];
        for i in 0..target_dim {
            let dx = d.output(target_t, i);

            for (j, row) in products.iter_mut().enumerate() {
                let idx = source.operation_generator_to_index(op_deg, op_idx, source_t, j);
                row.push(dx.entry(idx));
            }
        }

        Some(products)
    }

    /// A concurrent version of [`Resolution::compute_through_stem`]
    #[cfg(feature = "concurrent")]
    pub fn compute_through_stem_concurrent(&self, max_s: u32, max_n: i32, bucket: &TokenBucket) {
        let min_degree = self.min_degree();
        let _lock = self.lock.lock();
        let max_t = max_s as i32 + max_n;

        self.complex().compute_through_bidegree(max_s, max_t);
        self.extend_through_degree(max_s);
        self.algebra().compute_basis(max_t - min_degree);

        crossbeam_utils::thread::scope(|s| {
            let mut last_receiver: Option<Receiver<()>> = None;
            for t in min_degree..=max_t {
                let (sender, receiver) = unbounded();
                s.builder()
                    .name(format!("t = {}", t))
                    .spawn(move |_| {
                        let mut token = bucket.take_token();
                        let start_s = std::cmp::max(0, t - max_n - 1) as u32;
                        for s in start_s..=max_s {
                            token = bucket.recv_or_release(token, &last_receiver);
                            if !self.has_computed_bidegree(s, t) {
                                if s as i32 + max_n + 1 == t {
                                    // We compute the kernel now and save it. If we don't call
                                    // this, then the step_resolution stage will compute it, but we
                                    // may be unnecessarily idling in that case.
                                    self.kernels.insert((s, t), self.get_kernel(s, t));
                                    // The next t cannot be computed yet
                                    continue;
                                } else {
                                    self.step_resolution(s, t);
                                }
                            }
                            // In the last round the receiver would have been dropped
                            sender.send(()).ok();
                        }
                    })
                    .unwrap();
                last_receiver = Some(receiver);
            }
        })
        .unwrap();
    }
}

impl<CC: ChainComplex> ChainComplex for Resolution<CC> {
    type Algebra = CC::Algebra;
    type Module = FreeModule<Self::Algebra>;
    type Homomorphism = FreeModuleHomomorphism<FreeModule<Self::Algebra>>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.complex().algebra()
    }

    fn module(&self, s: u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s as usize])
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn min_degree(&self) -> i32 {
        self.complex().min_degree()
    }

    fn has_computed_bidegree(&self, s: u32, t: i32) -> bool {
        self.differentials.len() > s as usize && self.differential(s).next_degree() > t
    }

    fn set_homology_basis(&self, _s: u32, _t: i32, _homology_basis: Vec<usize>) {
        unimplemented!()
    }

    fn homology_basis(&self, _s: u32, _t: i32) -> &Vec<usize> {
        unimplemented!()
    }

    fn homology_dimension(&self, s: u32, t: i32) -> usize {
        self.number_of_gens_in_bidegree(s, t)
    }

    fn max_homology_degree(&self, _s: u32) -> i32 {
        unimplemented!()
    }

    fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn compute_through_bidegree(&self, s: u32, t: i32) {
        self.compute_through_bidegree_with_callback(s, t, |_, _| ())
    }

    #[cfg(feature = "concurrent")]
    fn compute_through_bidegree_concurrent(&self, max_s: u32, max_t: i32, bucket: &TokenBucket) {
        self.compute_through_bidegree_concurrent_with_callback(max_s, max_t, bucket, |_, _| ())
    }

    fn next_homological_degree(&self) -> u32 {
        self.modules.len() as u32
    }
}

impl<CC: ChainComplex> AugmentedChainComplex for Resolution<CC> {
    type TargetComplex = CC;
    type ChainMap = FreeModuleHomomorphism<CC::Module>;

    fn target(&self) -> Arc<Self::TargetComplex> {
        self.complex()
    }

    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s])
    }
}

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<CC: ChainComplex> Save for Resolution<CC> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        // This is no longer used but we keep it around for backwards compatibility
        let max_algebra_dim = self.module(0).max_computed_degree() - self.min_degree();
        max_algebra_dim.save(buffer)?;

        self.modules.save(buffer)?;
        self.kernels.save(buffer)?;
        self.differentials.save(buffer)?;
        self.chain_maps.save(buffer)?;
        Ok(())
    }
}

impl<CC: ChainComplex> Load for Resolution<CC> {
    type AuxData = Arc<CC>;

    fn load(buffer: &mut impl Read, cc: &Self::AuxData) -> io::Result<Self> {
        // This is no longer used but we keep it around for backwards compatibility
        let _max_algebra_dim = i32::load(buffer, &())?;

        let mut result = Resolution::new(Arc::clone(cc));

        let algebra = result.algebra();
        let p = result.prime();
        let min_degree = result.min_degree();

        result.modules = Load::load(buffer, &(Arc::clone(&algebra), min_degree))?;
        result.kernels = Load::load(buffer, &(((), ()), p))?;

        let max_s = result.modules.len();
        assert!(max_s > 0, "cannot load uninitialized resolution");

        let len = usize::load(buffer, &())?;
        assert_eq!(len, max_s);

        result.differentials.push(Load::load(
            buffer,
            &(result.module(0), result.zero_module(), 0),
        )?);
        for s in 1..max_s as u32 {
            let d: Arc<FreeModuleHomomorphism<FreeModule<CC::Algebra>>> =
                Load::load(buffer, &(result.module(s), result.module(s - 1), 0))?;
            result.differentials.push(d);
        }

        let len = usize::load(buffer, &())?;
        assert_eq!(len, max_s);

        for s in 0..max_s as u32 {
            let c: Arc<FreeModuleHomomorphism<CC::Module>> =
                Load::load(buffer, &(result.module(s), result.complex().module(s), 0))?;
            result.chain_maps.push(c);
        }

        result
            .zero_module
            .extend_by_zero(result.module(0).max_computed_degree());

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{chain_complex::FreeChainComplex, utils::construct};
    use expect_test::expect;

    #[test]
    fn test_restart_stem() {
        let res = construct("S_2", None).unwrap();
        #[cfg(not(feature = "concurrent"))]
        {
            res.compute_through_stem(8, 14);
            res.compute_through_bidegree(5, 19);
        }

        #[cfg(feature = "concurrent")]
        {
            let bucket = thread_token::TokenBucket::new(core::num::NonZeroUsize::new(2).unwrap());
            res.compute_through_stem_concurrent(8, 14, &bucket);
            res.compute_through_bidegree_concurrent(5, 19, &bucket);
        }

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
}
