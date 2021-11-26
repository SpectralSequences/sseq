use std::sync::{Arc, Mutex};

use crate::chain_complex::{AugmentedChainComplex, ChainComplex};
use crate::utils;
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use fp::matrix::{AugmentedMatrix, QuasiInverse, Subspace};
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use once::OnceVec;

use std::fs::File;
use std::path::PathBuf;

use anyhow::Context;
use dashmap::DashMap;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[cfg(feature = "concurrent")]
use crossbeam_channel::{unbounded, Receiver};

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

use std::io::{BufReader, BufWriter, Read, Write};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SaveData {
    Kernel,
    Differential,
    ResQi,
    AugmentationQi,
}

impl SaveData {
    pub fn magic(self) -> u32 {
        match self {
            Self::Kernel => 0x0000D1FF,
            Self::Differential => 0xD1FF0000,
            Self::ResQi => 0x0100D1FF,
            Self::AugmentationQi => 0x0100A000,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Differential => "differential",
            Self::ResQi => "res_qi",
            Self::AugmentationQi => "augmentation_qi",
        }
    }

    pub fn resolution_data() -> impl Iterator<Item = SaveData> {
        use SaveData::*;
        static KINDS: [SaveData; 4] = [Kernel, Differential, ResQi, AugmentationQi];
        KINDS.iter().copied()
    }
}

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
    save_dir: Option<PathBuf>,
    pub should_save: bool,
    pub load_quasi_inverse: bool,
}

impl<CC: ChainComplex> Resolution<CC> {
    pub fn new(complex: Arc<CC>) -> Self {
        // It doesn't error if the save file is None
        Self::new_with_save(complex, None).unwrap()
    }

    pub fn new_with_save(complex: Arc<CC>, mut save_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let algebra = complex.algebra();
        let min_degree = complex.min_degree();
        let zero_module = Arc::new(FreeModule::new(algebra, "F_{-1}".to_string(), min_degree));

        if let Some(p) = save_dir.as_mut() {
            use std::io::{Error, ErrorKind};

            if !p.exists() {
                std::fs::create_dir_all(&p)
                    .context("Failed to create save directory for resolution")?;
            } else if !p.is_dir() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("{p:?} is not a directory"),
                )
                .into());
            }
            for subdir in SaveData::resolution_data() {
                p.push(format!("{}s", subdir.name()));
                if !p.exists() {
                    std::fs::create_dir_all(&p)
                        .with_context(|| format!("Failed to create directory {p:?}"))?;
                } else if !p.is_dir() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("{p:?} is not a directory"),
                    )
                    .into());
                }
                p.pop();
            }
        }

        Ok(Self {
            complex,
            zero_module,
            should_save: save_dir.is_some(),
            save_dir,
            lock: Mutex::new(()),

            chain_maps: OnceVec::new(),
            modules: OnceVec::new(),
            differentials: OnceVec::new(),
            kernels: DashMap::new(),
            load_quasi_inverse: true,
        })
    }

    /// This panics if there is no save dir
    fn get_save_path(&self, kind: SaveData, s: u32, t: i32) -> PathBuf {
        let name = kind.name();
        let mut p = self.save_dir.clone().unwrap();
        p.push(format!("{name}s/{s}_{t}_{name}"));
        p
    }

    // When compiling to wasm we don't mutate path
    #[allow(unused_mut)]
    fn search_file(mut path: PathBuf) -> Option<Box<dyn Read>> {
        // We should try in decreasing order of access speed.
        match File::open(&path) {
            Ok(f) => return Some(Box::new(BufReader::new(f))),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    panic!("Error when opening {path:?}");
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            path.set_extension("zst");
            match File::open(&path) {
                Ok(f) => return Some(Box::new(zstd::stream::Decoder::new(f).unwrap())),
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        panic!("Error when opening {path:?}");
                    }
                }
            }
        }

        None
    }

    /// This panics if there is no save dir
    fn open_save_file(&self, kind: SaveData, s: u32, t: i32) -> Option<Box<dyn Read>> {
        let mut f = Self::search_file(self.get_save_path(kind, s, t))?;
        utils::validate_header(kind.magic(), self.prime(), s, t, &mut f).unwrap();
        Some(f)
    }

    fn create_save_file(&self, kind: SaveData, s: u32, t: i32) -> impl Write {
        let p = self.get_save_path(kind, s, t);

        let f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)
            .with_context(|| format!("Failed to create save file {p:?}"))
            .unwrap();
        let mut f = BufWriter::new(f);
        utils::write_header(kind.magic(), self.prime(), s, t, &mut f).unwrap();
        f
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

        if self.save_dir.is_some() {
            if let Some(mut f) = self.open_save_file(SaveData::Kernel, s, t) {
                return Subspace::from_bytes(p, &mut f)
                    .with_context(|| format!("Failed to read kernel at ({s}, {t})"))
                    .unwrap();
            }
        }

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

        let kernel = matrix.compute_kernel();

        if self.should_save && self.save_dir.is_some() {
            let mut f = self.create_save_file(SaveData::Kernel, s, t);
            kernel
                .to_bytes(&mut f)
                .with_context(|| format!("Failed to write kernel at ({s}, {t})"))
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

        if self.save_dir.is_some() {
            if let Some(mut f) = self.open_save_file(SaveData::Differential, s, t) {
                let num_new_gens = f.read_u64::<LittleEndian>().unwrap() as usize;
                // It can be smaller than target_res_dimension if we resolved through stem
                let saved_target_res_dimension = f.read_u64::<LittleEndian>().unwrap() as usize;
                assert!(
                    target_res_dimension >= saved_target_res_dimension,
                    "Malformed data: mismatched resolution target dimension"
                );
                assert_eq!(
                    target_cc_dimension,
                    f.read_u64::<LittleEndian>().unwrap() as usize,
                    "Malformed data: mismatched augmentation target dimension"
                );

                source.add_generators(t, num_new_gens, None);

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
                current_differential.add_generators_from_rows(t, d_targets);
                current_chain_map.add_generators_from_rows(t, a_targets);

                // res qi
                if self.load_quasi_inverse {
                    if let Some(mut f) = self.open_save_file(SaveData::ResQi, s, t) {
                        let res_qi = QuasiInverse::from_bytes(p, &mut f).unwrap();

                        assert!(
                            target_res_dimension >= res_qi.target_dimension(),
                            "Malformed data: mismatched resolution target dimension in qi at ({s}, {t})"
                            );
                        assert_eq!(
                            res_qi.source_dimension(),
                            source_dimension + num_new_gens,
                            "Malformed data: mismatched source dimension in resolution qi at ({s}, {t})"
                            );

                        current_differential.set_quasi_inverse(t, Some(res_qi));
                    } else {
                        current_differential.set_quasi_inverse(t, None);
                    }

                    if let Some(mut f) = self.open_save_file(SaveData::AugmentationQi, s, t) {
                        let cm_qi = QuasiInverse::from_bytes(p, &mut f).unwrap();

                        assert_eq!(
                        cm_qi.target_dimension(),
                        target_cc_dimension,
                        "Malformed data: mismatched augmentation target dimension in qi at ({s}, {t})"
                    );
                        assert_eq!(
                        cm_qi.source_dimension(),
                        source_dimension + num_new_gens,
                        "Malformed data: mismatched source dimension in augmentation qi at ({s}, {t})"
                    );

                        current_chain_map.set_quasi_inverse(t, Some(cm_qi));
                    } else {
                        current_chain_map.set_quasi_inverse(t, None);
                    }
                } else {
                    current_differential.set_quasi_inverse(t, None);
                    current_chain_map.set_quasi_inverse(t, None);
                }
                current_differential.set_kernel(t, None);
                current_differential.set_image(t, None);

                current_chain_map.set_kernel(t, None);
                current_chain_map.set_image(t, None);
                return;
            }
        }

        let rows = source_dimension + target_cc_dimension + target_res_dimension;

        let mut matrix = AugmentedMatrix::<3>::new_with_capacity(
            p,
            source_dimension,
            &[target_cc_dimension, target_res_dimension, source_dimension],
            rows,
            rows,
        );
        // Get the map (d, f) : X_{s, t} -> X_{s-1, t} (+) C_{s, t} into matrix

        current_chain_map.get_matrix(&mut matrix.segment(0, 0), t);
        current_differential.get_matrix(&mut matrix.segment(1, 1), t);
        matrix.segment(2, 2).add_identity();

        matrix.row_reduce();

        if !self.has_computed_bidegree(s + 1, t) {
            let kernel = matrix.compute_kernel();
            if self.should_save && self.save_dir.is_some() {
                let mut f = self.create_save_file(SaveData::Kernel, s, t);

                kernel
                    .to_bytes(&mut f)
                    .with_context(|| format!("Failed to write kernel at ({s}, {t})"))
                    .unwrap();
            }

            self.kernels.insert((s, t), kernel);
        }

        // Now add generators to surject onto C_{s, t}.
        // (For now we are just adding the eventual images of the new generators into matrix, we will update
        // X_{s,t} and f later).
        // We record which pivots exactly we added so that we can walk over the added genrators in a moment and
        // work out what dX should to to each of them.
        let cc_new_gens = matrix.extend_to_surjection(0, target_cc_dimension, rows);

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
                rows,
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
        let (cm_qi, res_qi) =
            if (self.should_save && self.save_dir.is_some()) || self.load_quasi_inverse {
                let (c, r) = matrix.compute_quasi_inverses();
                (Some(c), Some(r))
            } else {
                (None, None)
            };

        if self.should_save && self.save_dir.is_some() {
            // Write differentials
            let mut f = self.create_save_file(SaveData::Differential, s, t);

            f.write_u64::<LittleEndian>(num_new_gens as u64).unwrap();
            f.write_u64::<LittleEndian>(target_res_dimension as u64)
                .unwrap();
            f.write_u64::<LittleEndian>(target_cc_dimension as u64)
                .unwrap();

            for n in 0..num_new_gens {
                current_differential.output(t, n).to_bytes(&mut f).unwrap();
            }
            for n in 0..num_new_gens {
                current_chain_map.output(t, n).to_bytes(&mut f).unwrap();
            }
            drop(f);

            // Write resolution qi
            let mut f = self.create_save_file(SaveData::ResQi, s, t);
            res_qi.as_ref().unwrap().to_bytes(&mut f).unwrap();
            drop(f);

            // Write augmentation qi
            let mut f = self.create_save_file(SaveData::AugmentationQi, s, t);
            cm_qi.as_ref().unwrap().to_bytes(&mut f).unwrap();
            drop(f);

            // Delete kernel
            if self.should_save && s > 0 {
                let ker_path = self.get_save_path(SaveData::Kernel, s - 1, t);
                if ker_path.exists() {
                    std::fs::remove_file(ker_path).unwrap();
                }
            }
        }

        current_chain_map.set_quasi_inverse(t, cm_qi);
        current_chain_map.set_kernel(t, None);
        current_chain_map.set_image(t, None);
        current_differential.set_quasi_inverse(t, res_qi);
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
                                    // This is the bidegree just beyond max_n. We are not computing
                                    // this bidegree, but if we have to compute the next s, we have
                                    // to compute the kernel of this bidegree.
                                    //
                                    // We can wait until the next step to compute the kernel, but
                                    // we are already ready to do so, so let's do it now while we
                                    // have time.
                                    if !self.has_computed_bidegree(s + 1, t)
                                        && (self.save_dir.is_none()
                                            || !self
                                                .get_save_path(SaveData::Differential, s + 1, t)
                                                .exists())
                                    {
                                        self.kernels.insert((s, t), self.get_kernel(s, t));
                                    }
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
