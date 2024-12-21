//! This module implements [Nassau's algorithm](https://arxiv.org/abs/1910.04063).
//!
//! The main export is the [`Resolution`] object, which is a resolution of the sphere at the prime 2
//! using Nassau's algorithm. It aims to provide an API similar to
//! [`resolution::Resolution`](crate::resolution::Resolution). From an API point of view, the main
//! difference between the two is that our `Resolution` is a chain complex over [`MilnorAlgebra`]
//! over [`SteenrodAlgebra`](algebra::SteenrodAlgebra).
//!
//! To make use of this resolution in the example scripts, enable the `nassau` feature. This will
//! cause [`utils::query_module`](crate::utils::query_module) to return the `Resolution` from this
//! module instead of [`resolution`](crate::resolution). There is no formal polymorphism involved;
//! the feature changes the return type of the function. While this is an incorrect use of features,
//! we find that this the easiest way to make all scripts support both types of resolutions.

use std::{
    fmt::Display,
    io,
    sync::{mpsc, Arc, Mutex},
};

use algebra::{
    combinatorics,
    milnor_algebra::{MilnorAlgebra, PPartEntry},
    module::{
        homomorphism::{FreeModuleHomomorphism, FullModuleHomomorphism, ModuleHomomorphism},
        FreeModule, GeneratorData, Module, ZeroModule,
    },
    Algebra,
};
use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fp::{
    matrix::{AugmentedMatrix, Matrix},
    prime::{ValidPrime, TWO},
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::Itertools;
use once::OnceVec;
use sseq::coordinates::Bidegree;

use crate::{
    chain_complex::{AugmentedChainComplex, ChainComplex, FiniteChainComplex, FreeChainComplex},
    save::{SaveDirectory, SaveKind},
    utils::LogWriter,
};

/// See [`resolution::SenderData`](../resolution/struct.SenderData.html). This differs by not having the `new` field.
struct SenderData {
    b: Bidegree,
    sender: mpsc::Sender<SenderData>,
}

impl SenderData {
    pub(crate) fn send(b: Bidegree, sender: mpsc::Sender<Self>) {
        sender
            .send(Self {
                b,
                sender: sender.clone(),
            })
            .unwrap()
    }
}

const MAX_NEW_GENS: usize = 10;

/// A Milnor subalgebra to be used in [Nassau's algorithm](https://arxiv.org/abs/1910.04063). This
/// is equipped with an ordering of the signature as in Lemma 2.4 of the paper.
///
/// To simplify implementation, we pick the ordering so that the (reverse) lexicographic ordering
/// in Lemma 2.4 is just the (reverse) lexicographic ordering of the P parts. This corresponds to
/// the ordering of $\mathcal{P}$ where $P^s_t < P^{s'}_t$ if $s < s'$).
#[derive(Clone)]
struct MilnorSubalgebra {
    profile: Vec<u8>,
}

impl MilnorSubalgebra {
    /// This should be used when you want an entry of the profile to be infinity
    #[allow(dead_code)]
    const INFINITY: u8 = (std::mem::size_of::<PPartEntry>() * 4 - 1) as u8;

    fn new(profile: Vec<u8>) -> Self {
        Self { profile }
    }

    /// The algebra with trivial profile, corresponding to the trivial algebra.
    fn zero_algebra() -> Self {
        Self { profile: vec![] }
    }

    /// Computes the signature of an element
    fn has_signature(&self, ppart: &[PPartEntry], signature: &[PPartEntry]) -> bool {
        for (i, (&profile, &signature)) in self.profile.iter().zip(signature).enumerate() {
            let ppart = ppart.get(i).copied().unwrap_or(0);
            if ppart & ((1 << profile) - 1) != signature {
                return false;
            }
        }
        true
    }

    fn zero_signature(&self) -> Vec<PPartEntry> {
        vec![0; self.profile.len()]
    }

    /// Give a list of basis elements in degree `degree` that has signature `signature`.
    ///
    /// This requires passing the algebra for borrow checker reasons.
    fn signature_mask<'a>(
        &'a self,
        algebra: &'a MilnorAlgebra,
        module: &'a FreeModule<MilnorAlgebra>,
        degree: i32,
        signature: &'a [PPartEntry],
    ) -> impl Iterator<Item = usize> + 'a {
        module.iter_gen_offsets([degree]).flat_map(
            move |GeneratorData {
                      gen_deg,
                      start: [offset],
                      end: _,
                  }| {
                algebra
                    .ppart_table(degree - gen_deg)
                    .iter()
                    .enumerate()
                    .filter_map(move |(n, op)| {
                        if self.has_signature(op, signature) {
                            Some(offset + n)
                        } else {
                            None
                        }
                    })
            },
        )
    }

    /// Get the matrix of a free module homomorphism when restricted to the subquotient given by
    /// the signature.
    fn signature_matrix(
        &self,
        hom: &FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>,
        degree: i32,
        signature: &[PPartEntry],
    ) -> Matrix {
        let p = hom.prime();
        let source = hom.source();
        let target = hom.target();
        let algebra = target.algebra();
        let target_degree = degree - hom.degree_shift();

        let target_mask: Vec<usize> = self
            .signature_mask(&algebra, &target, degree - hom.degree_shift(), signature)
            .collect();

        let source_mask: Vec<usize> = self
            .signature_mask(&algebra, &source, degree, signature)
            .collect();

        let mut scratch = FpVector::new(p, target.dimension(target_degree));
        let mut result = Matrix::new(p, source_mask.len(), target_mask.len());

        for (row, &masked_index) in std::iter::zip(result.iter_mut(), &source_mask) {
            scratch.set_to_zero();
            hom.apply_to_basis_element(scratch.as_slice_mut(), 1, degree, masked_index);

            row.as_slice_mut()
                .add_masked(scratch.as_slice(), 1, &target_mask);
        }
        result
    }

    /// Iterate through all signatures of this algebra that contain elements of degree at most
    /// `degree` (inclusive). This skips the initial zero signature.
    fn iter_signatures(&self, degree: i32) -> impl Iterator<Item = Vec<PPartEntry>> + '_ {
        SignatureIterator::new(self, degree)
    }

    fn top_degree(&self) -> i32 {
        self.profile
            .iter()
            .map(|&entry| (1 << entry) - 1)
            .enumerate()
            .map(|(idx, entry)| ((1 << (idx + 1)) - 1) * entry)
            .sum()
    }

    fn optimal_for(b: Bidegree) -> Self {
        let b_is_in_vanishing_region = |subalgebra: &Self| {
            let coeff = (1 << subalgebra.profile.len()) - 1;
            b.t() >= coeff * (b.s() as i32 + 1) + subalgebra.top_degree()
        };
        SubalgebraIterator::new()
            .take_while(b_is_in_vanishing_region)
            .last()
            .unwrap_or(Self::zero_algebra())
    }

    fn to_bytes(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        buffer.write_u64::<LittleEndian>(self.profile.len() as u64)?;
        buffer.write_all(&self.profile)?;

        let len = self.profile.len();
        let zeros = [0; 8];
        let padding = len - ((len / 8) * 8);
        buffer.write_all(&zeros[0..padding])
    }

    fn from_bytes(data: &mut impl io::Read) -> io::Result<Self> {
        let len = data.read_u64::<LittleEndian>()? as usize;
        let mut profile = vec![0; len];

        data.read_exact(&mut profile)?;

        let padding = len - ((len / 8) * 8);
        if padding > 0 {
            let mut buf: [u8; 8] = [0; 8];
            data.read_exact(&mut buf[0..padding])?;
            assert_eq!(buf, [0; 8]);
        }
        Ok(Self { profile })
    }

    fn signature_to_bytes(signature: &[PPartEntry], buffer: &mut impl io::Write) -> io::Result<()> {
        if cfg!(target_endian = "little") && std::mem::size_of::<PPartEntry>() == 2 {
            unsafe {
                let buf: &[u8] = std::slice::from_raw_parts(
                    signature.as_ptr() as *const u8,
                    signature.len() * 2,
                );
                buffer.write_all(buf).unwrap();
            }
        } else {
            for &entry in signature {
                buffer.write_u16::<LittleEndian>(entry as u16)?;
            }
        }

        let len = signature.len();
        let zeros = [0; 8];
        let padding = len - ((len / 4) * 4);

        if padding > 0 {
            buffer.write_all(&zeros[0..padding * 2])?;
        }
        Ok(())
    }

    fn signature_from_bytes(&self, data: &mut impl io::Read) -> io::Result<Vec<PPartEntry>> {
        let len = self.profile.len();
        let mut signature: Vec<PPartEntry> = vec![0; len];

        if cfg!(target_endian = "little") && std::mem::size_of::<PPartEntry>() == 2 {
            unsafe {
                let buf: &mut [u8] =
                    std::slice::from_raw_parts_mut(signature.as_mut_ptr() as *mut u8, len * 2);
                data.read_exact(buf).unwrap();
            }
        } else {
            for entry in &mut signature {
                *entry = data.read_u16::<LittleEndian>()? as PPartEntry;
            }
        }

        let padding = len - ((len / 4) * 4);
        if padding > 0 {
            let mut buffer: [u8; 8] = [0; 8];
            data.read_exact(&mut buffer[0..padding * 2])?;
            assert_eq!(buffer, [0; 8]);
        }
        Ok(signature)
    }
}

impl Display for MilnorSubalgebra {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if self.profile.is_empty() {
            write!(out, "F_2")
        } else if self.profile.len() as u8 == self.profile[0] {
            write!(out, "A({})", self.profile.len() - 1)
        } else {
            write!(out, "Algebra with profile {:?}", self.profile)
        }
    }
}

/// An iterator that iterates through a sequence of [`MilnorSubalgebra`] of increasing size. This
/// is used by [`MilnorSubalgebra::optimal_for`] to find the largest subalgebra in this sequence
/// that is applicable to a bidegree.
struct SubalgebraIterator {
    current: MilnorSubalgebra,
}

impl SubalgebraIterator {
    fn new() -> Self {
        Self {
            current: MilnorSubalgebra::new(vec![]),
        }
    }
}

impl Iterator for SubalgebraIterator {
    type Item = MilnorSubalgebra;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.profile.is_empty()
            || self.current.profile[0] == self.current.profile.len() as u8
        {
            // We are at F_2 or at A(n) where n = self.current.profile.len() - 1.
            self.current.profile.push(1);
            Some(self.current.clone())
        } else {
            // We find the first entry that can be incremented and increment it
            if let Some((_, entry)) = self
                .current
                .profile
                .iter_mut()
                .rev()
                .enumerate()
                .find(|(idx, entry)| **entry == *idx as u8)
            {
                *entry += 1;
            }
            Some(self.current.clone())
        }
    }
}

/// See [`MilnorSubalgebra::iter_signatures`].
struct SignatureIterator<'a> {
    subalgebra: &'a MilnorSubalgebra,
    current: Vec<PPartEntry>,
    signature_degree: i32,
    degree: i32,
}

impl<'a> SignatureIterator<'a> {
    fn new(subalgebra: &'a MilnorSubalgebra, degree: i32) -> Self {
        Self {
            current: vec![0; subalgebra.profile.len()],
            degree,
            subalgebra,
            signature_degree: 0,
        }
    }
}

impl Iterator for SignatureIterator<'_> {
    type Item = Vec<PPartEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        let xi_degrees = combinatorics::xi_degrees(TWO);
        let len = self.current.len();
        for (i, current) in self.current.iter_mut().enumerate() {
            *current += 1;
            self.signature_degree += xi_degrees[i];

            if self.signature_degree > self.degree || *current == 1 << self.subalgebra.profile[i] {
                self.signature_degree -= xi_degrees[i] * *current as i32;
                *current = 0;
                if i + 1 == len {
                    return None;
                }
            } else {
                return Some(self.current.clone());
            }
        }
        // This only happens when the profile is trivial
        assert!(self.current.is_empty());
        None
    }
}

/// Some magic constants used in the save file
enum Magic {
    End = -1,
    Signature = -2,
    Fix = -3,
}

/// A resolution of `S_2` using Nassau's algorithm.
///
/// This aims to have an API similar to that of
/// [`resolution::Resolution`](crate::resolution::Resolution). From an API point of view, the main
/// difference between the two is that this is a chain complex over [`MilnorAlgebra`] over
/// [`SteenrodAlgebra`](algebra::SteenrodAlgebra).
pub struct Resolution<M: ZeroModule<Algebra = MilnorAlgebra>> {
    lock: Mutex<()>,
    name: String,
    max_degree: i32,
    modules: OnceVec<Arc<FreeModule<MilnorAlgebra>>>,
    zero_module: Arc<FreeModule<MilnorAlgebra>>,
    differentials: OnceVec<Arc<FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>>>,
    target: Arc<FiniteChainComplex<M>>,
    chain_maps: OnceVec<Arc<FreeModuleHomomorphism<M>>>,
    save_dir: SaveDirectory,
}

impl<M: ZeroModule<Algebra = MilnorAlgebra>> Resolution<M> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn new(module: Arc<M>) -> Self {
        Self::new_with_save(module, None).unwrap()
    }

    pub fn new_with_save(
        module: Arc<M>,
        save_dir: impl Into<SaveDirectory>,
    ) -> anyhow::Result<Self> {
        let save_dir = save_dir.into();
        let max_degree = module
            .max_degree()
            .ok_or_else(|| anyhow!("Nassau's algorithm requires bounded module"))?;
        let target = Arc::new(FiniteChainComplex::ccdz(module));

        if let Some(p) = save_dir.write() {
            for subdir in SaveKind::nassau_data() {
                subdir.create_dir(p)?;
            }
        }

        Ok(Self {
            lock: Mutex::new(()),
            zero_module: Arc::new(FreeModule::new(target.algebra(), "F_{-1}".to_string(), 0)),
            name: String::new(),
            modules: OnceVec::new(),
            differentials: OnceVec::new(),
            chain_maps: OnceVec::new(),
            target,
            max_degree,
            save_dir,
        })
    }

    /// This function prepares the Resolution object to perform computations up to the
    /// specified s degree. It does *not* perform any computations by itself. It simply lengthens
    /// the `OnceVec`s `modules`, `chain_maps`, etc. to the right length.
    fn extend_through_degree(&self, max_s: u32) {
        let min_degree = self.min_degree();

        self.modules.extend(max_s as usize, |i| {
            Arc::new(FreeModule::new(
                Arc::clone(&self.algebra()),
                format!("F{i}"),
                min_degree,
            ))
        });

        self.differentials.extend(0, |_| {
            Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[0u32]),
                Arc::clone(&self.zero_module),
                0,
            ))
        });

        self.differentials.extend(max_s as usize, |i| {
            Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[i]),
                Arc::clone(&self.modules[i - 1]),
                0,
            ))
        });

        self.chain_maps.extend(max_s as usize, |i| {
            Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[i]),
                self.target.module(i as u32),
                0,
            ))
        });
    }

    #[tracing::instrument(skip_all, fields(signature = ?signature, throughput))]
    fn write_qi(
        f: &mut Option<impl io::Write>,
        scratch: &mut FpVector,
        signature: &[PPartEntry],
        next_mask: &[usize],
        full_matrix: &Matrix,
        masked_matrix: &AugmentedMatrix<2>,
    ) -> io::Result<()> {
        let f = match f {
            Some(f) => f,
            None => return Ok(()),
        };

        let mut own_f = LogWriter::new(f);
        let f = &mut own_f;

        let pivots = &masked_matrix.pivots()[0..masked_matrix.end[0]];
        if !pivots.iter().any(|&x| x >= 0) {
            return Ok(());
        }

        // Write signature if non-zero.
        if signature.iter().any(|&x| x > 0) {
            f.write_u64::<LittleEndian>(Magic::Signature as u64)?;
            MilnorSubalgebra::signature_to_bytes(signature, f)?;
        }

        // Write quasi-inverses
        for (col, &row) in pivots.iter().enumerate() {
            if row < 0 {
                continue;
            }
            f.write_u64::<LittleEndian>(next_mask[col] as u64)?;
            let preimage = masked_matrix.row_segment(row as usize, 1, 1);
            scratch.set_scratch_vector_size(preimage.len());
            scratch.as_slice_mut().assign(preimage);
            scratch.to_bytes(f)?;

            scratch.set_scratch_vector_size(full_matrix.columns());
            for (i, _) in preimage.iter_nonzero() {
                scratch.as_slice_mut().add(full_matrix.row(i), 1);
            }
            scratch.to_bytes(f)?;
        }

        tracing::Span::current().record(
            "throughput",
            tracing::field::display(own_f.into_throughput()),
        );
        Ok(())
    }

    fn write_differential(
        &self,
        b: Bidegree,
        num_new_gens: usize,
        target_dim: usize,
    ) -> anyhow::Result<()> {
        if let Some(dir) = self.save_dir.write() {
            let mut f = self
                .save_file(SaveKind::NassauDifferential, b)
                .create_file(dir.clone(), false);
            f.write_u64::<LittleEndian>(num_new_gens as u64)?;
            f.write_u64::<LittleEndian>(target_dim as u64)?;

            for n in 0..num_new_gens {
                self.differential(b.s()).output(b.t(), n).to_bytes(&mut f)?;
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(b = %b, subalgebra = %subalgebra, num_new_gens, density))]
    fn step_resolution_with_subalgebra(
        &self,
        b: Bidegree,
        subalgebra: MilnorSubalgebra,
    ) -> anyhow::Result<()> {
        let end = || {
            tracing::Span::current().record("num_new_gens", self.number_of_gens_in_bidegree(b));
            tracing::Span::current().record(
                "density",
                self.differentials[b.s()].differential_density(b.t()) * 100.0,
            );
        };

        let p = self.prime();
        let mut scratch = FpVector::new(p, 0);

        let source = &*self.modules[b.s()];
        let target = &*self.modules[b.s() - 1];
        let algebra = target.algebra();

        let zero_sig = subalgebra.zero_signature();
        let target_dim = target.dimension(b.t());
        let target_mask: Vec<usize> = subalgebra
            .signature_mask(&algebra, target, b.t(), &zero_sig)
            .collect();
        let target_masked_dim = target_mask.len();

        let next = &self.modules[b.s() - 2];
        next.compute_basis(b.t());

        let mut f = if let Some(dir) = self.save_dir().write() {
            let mut f = self
                .save_file(SaveKind::NassauQi, b - Bidegree::s_t(1, 0))
                .create_file(dir.to_owned(), true);
            f.write_u64::<LittleEndian>(next.dimension(b.t()) as u64)?;
            f.write_u64::<LittleEndian>(target_masked_dim as u64)?;
            subalgebra.to_bytes(&mut f)?;
            Some(f)
        } else {
            None
        };

        let next_mask: Vec<usize> = subalgebra
            .signature_mask(&algebra, &self.modules[b.s() - 2], b.t(), &zero_sig)
            .collect();
        let next_masked_dim = next_mask.len();

        let full_matrix = self.differentials[b.s() - 1].get_partial_matrix(b.t(), &target_mask);
        let mut masked_matrix =
            AugmentedMatrix::new(p, target_masked_dim, [next_masked_dim, target_masked_dim]);

        masked_matrix
            .segment(0, 0)
            .add_masked(&full_matrix, &next_mask);
        masked_matrix.segment(1, 1).add_identity();
        masked_matrix.row_reduce();
        let kernel = masked_matrix.compute_kernel();

        Self::write_qi(
            &mut f,
            &mut scratch,
            &zero_sig,
            &next_mask,
            &full_matrix,
            &masked_matrix,
        )?;

        if let Some(f) = &mut f {
            if target.max_computed_degree() < b.t() {
                f.write_u64::<LittleEndian>(Magic::Fix as u64)?;
            }
        }

        // Compute image
        let mut n = subalgebra.signature_matrix(&self.differentials[b.s()], b.t(), &zero_sig);
        n.row_reduce();
        let next_row = n.rows();

        let num_new_gens = n.extend_image(0, n.columns(), &kernel, 0).len();

        if b.t() < b.s() as i32 {
            assert_eq!(num_new_gens, 0, "Adding generators at {b}");
        }

        source.add_generators(b.t(), num_new_gens, None);

        let mut xs = vec![FpVector::new(p, target_dim); num_new_gens];
        let mut dxs = vec![FpVector::new(p, next.dimension(b.t())); num_new_gens];

        for ((x, x_masked), dx) in xs.iter_mut().zip_eq(&n[next_row..]).zip_eq(&mut dxs) {
            x.as_slice_mut()
                .add_unmasked(x_masked.as_slice(), 1, &target_mask);
            for (i, _) in x_masked.iter_nonzero() {
                dx.add(&full_matrix[i], 1);
            }
        }

        // Now add correction terms
        let mut target_mask: Vec<usize> = Vec::new();
        let mut next_mask: Vec<usize> = Vec::new();

        for signature in subalgebra.iter_signatures(b.t()) {
            target_mask.clear();
            next_mask.clear();
            target_mask.extend(subalgebra.signature_mask(&algebra, target, b.t(), &signature));
            next_mask.extend(subalgebra.signature_mask(&algebra, next, b.t(), &signature));

            let full_matrix = self
                .differential(b.s() - 1)
                .get_partial_matrix(b.t(), &target_mask);

            let mut masked_matrix =
                AugmentedMatrix::new(p, target_mask.len(), [next_mask.len(), target_mask.len()]);
            masked_matrix
                .segment(0, 0)
                .add_masked(&full_matrix, &next_mask);
            masked_matrix.segment(1, 1).add_identity();
            masked_matrix.row_reduce();

            let qi = masked_matrix.compute_quasi_inverse();
            let pivots = qi.pivots().unwrap();
            let preimage = qi.preimage();

            for (x, dx) in xs.iter_mut().zip(&mut dxs) {
                scratch.set_scratch_vector_size(target_mask.len());
                let mut row = 0;
                for (i, &v) in next_mask.iter().enumerate() {
                    if pivots[i] < 0 {
                        continue;
                    }
                    if dx.entry(v) != 0 {
                        scratch.add(&preimage[row], 1);
                    }
                    row += 1;
                }
                for (i, _) in scratch.iter_nonzero() {
                    x.add_basis_element(target_mask[i], 1);
                    dx.add(&full_matrix[i], 1);
                }
            }
            Self::write_qi(
                &mut f,
                &mut scratch,
                &signature,
                &next_mask,
                &full_matrix,
                &masked_matrix,
            )?;
        }
        for dx in &dxs {
            assert!(dx.is_zero(), "dx non-zero at {b}");
        }
        self.differential(b.s()).add_generators_from_rows(b.t(), xs);

        end();

        if let Some(f) = &mut f {
            f.write_u64::<LittleEndian>(Magic::End as u64)?;
        }

        self.write_differential(b, num_new_gens, target_dim)?;
        Ok(())
    }

    /// Step resolution for s = 0
    #[tracing::instrument(skip(self))]
    fn step0(&self, t: i32) {
        self.zero_module.extend_by_zero(t);

        let source_module = &self.modules[0usize];
        let target_module = self.target.module(0);

        let chain_map = &self.chain_maps[0usize];
        let d = &self.differentials[0usize];

        let source_dim = source_module.dimension(t);
        let target_dim = target_module.dimension(t);

        source_module.compute_basis(t);
        target_module.compute_basis(t);

        if target_dim == 0 {
            source_module.extend_by_zero(t);
            chain_map.extend_by_zero(t);
        } else {
            let mut matrix = AugmentedMatrix::<2>::new_with_capacity(
                self.prime(),
                source_dim,
                &[target_dim, source_dim],
                source_dim + target_dim,
                0,
            );
            chain_map.get_matrix(matrix.segment(0, 0), t);
            matrix.segment(1, 1).add_identity();

            matrix.row_reduce();

            let num_new_gens = matrix.extend_to_surjection(0, target_dim, 0).len();
            source_module.add_generators(t, num_new_gens, None);

            chain_map.add_generators_from_matrix_rows(
                t,
                matrix
                    .segment(0, 0)
                    .row_slice(source_dim, source_dim + num_new_gens),
            );
        }
        chain_map.compute_auxiliary_data_through_degree(t);

        d.set_kernel(t, None);
        d.set_image(t, None);
        d.set_quasi_inverse(t, None);
        d.extend_by_zero(t);
    }

    /// Step resolution for s = 1
    #[tracing::instrument(skip(self))]
    fn step1(&self, t: i32) -> anyhow::Result<()> {
        let p = self.prime();

        let source_module = &self.modules[1usize];
        let target_module = &self.modules[0usize];
        let cc_module = self.target.module(0);

        let source_dim = source_module.dimension(t);
        let target_dim = target_module.dimension(t);

        let mut matrix =
            AugmentedMatrix::<2>::new(p, target_dim, [cc_module.dimension(t), target_dim]);
        self.chain_maps[0usize].get_matrix(matrix.segment(0, 0), t);
        matrix.segment(1, 1).add_identity();
        matrix.row_reduce();
        let desired_image = matrix.compute_kernel();

        let mut matrix = AugmentedMatrix::<2>::new_with_capacity(
            p,
            source_dim,
            &[target_dim, source_dim],
            source_dim + MAX_NEW_GENS,
            0,
        );
        self.differentials[1usize].get_matrix(matrix.segment(0, 0), t);
        matrix.segment(1, 1).add_identity();
        matrix.row_reduce();

        let num_new_gens = matrix.extend_image(0, target_dim, &desired_image, 0).len();

        source_module.add_generators(t, num_new_gens, None);

        self.differentials[1usize].add_generators_from_matrix_rows(
            t,
            matrix
                .segment(0, 0)
                .row_slice(source_dim, source_dim + num_new_gens),
        );

        self.write_differential(Bidegree::s_t(1, t), num_new_gens, target_dim)?;
        Ok(())
    }

    fn step_resolution_with_result(&self, b: Bidegree) -> anyhow::Result<()> {
        let p = self.prime();
        let set_data = || {
            let d = &self.differentials[b.s()];
            let c = &self.chain_maps[b.s()];

            d.set_kernel(b.t(), None);
            d.set_image(b.t(), None);
            d.set_quasi_inverse(b.t(), None);

            c.set_kernel(b.t(), None);
            c.set_image(b.t(), None);
            c.set_quasi_inverse(b.t(), None);
        };
        self.modules[b.s()].compute_basis(b.t());
        if b.s() > 0 {
            self.modules[b.s() - 1].compute_basis(b.t());
        }

        if b.s() == 0 {
            self.step0(b.t());
            return Ok(());
        }

        if let Some(dir) = self.save_dir.read() {
            if let Some(mut f) = self
                .save_file(SaveKind::NassauDifferential, b)
                .open_file(dir.clone())
            {
                tracing::info!("Loading differential at {b}");

                let num_new_gens = f.read_u64::<LittleEndian>()? as usize;
                // This need not be equal to `target_res_dimension`. If we saved a big resolution
                // and now only want to load up to a small stem, then `target_res_dimension` will
                // be smaller. If we have previously saved a small resolution up to a stem and now
                // want to resolve further, it will be bigger.
                let saved_target_res_dimension = f.read_u64::<LittleEndian>()? as usize;

                self.modules[b.s()].add_generators(b.t(), num_new_gens, None);

                let mut d_targets = Vec::with_capacity(num_new_gens);

                for _ in 0..num_new_gens {
                    d_targets.push(FpVector::from_bytes(p, saved_target_res_dimension, &mut f)?);
                }

                self.differentials[b.s()].add_generators_from_rows(b.t(), d_targets);

                set_data();

                return Ok(());
            }
        }

        if b.s() == 1 {
            self.step1(b.t())?;
            set_data();
            return Ok(());
        }

        self.step_resolution_with_subalgebra(
            b,
            MilnorSubalgebra::optimal_for(b - Bidegree::s_t(0, self.max_degree)),
        )?;
        self.chain_maps[b.s()].extend_by_zero(b.t());

        set_data();
        Ok(())
    }

    fn step_resolution(&self, b: Bidegree) {
        self.step_resolution_with_result(b)
            .unwrap_or_else(|e| panic!("Error computing bidegree {b}: {e}"));
    }

    /// This function resolves up till a fixed stem instead of a fixed t.
    #[tracing::instrument(skip(self), fields(self = self.name, max = %max))]
    pub fn compute_through_stem(&self, max: Bidegree) {
        let _lock = self.lock.lock();

        self.extend_through_degree(max.s());
        self.algebra().compute_basis(max.t());

        let tracing_span = tracing::Span::current();
        maybe_rayon::in_place_scope(|scope| {
            let _tracing_guard = tracing_span.enter();

            // This algorithm is not optimal, as we compute (s, t) only after computing (s - 1, t)
            // and (s, t - 1). In theory, it suffices to wait for (s, t - 1) and (s - 1, t - 1),
            // but having the dimensions of the modules change halfway through the computation is
            // annoying to do correctly. It seems more prudent to improve parallelism elsewhere.

            // Things that we have finished computing.
            let mut progress: Vec<i32> = vec![-1; max.s() as usize + 1];
            // We will kickstart the process by pretending we have computed (0, - 1). So
            // we must pretend we have only computed up to (0, - 2);
            progress[0] = -2;

            let (sender, receiver) = mpsc::channel();
            SenderData::send(Bidegree::s_t(0, -1), sender);

            let f = |b, sender| {
                if self.has_computed_bidegree(b) {
                    SenderData::send(b, sender);
                } else {
                    let tracing_span = tracing_span.clone();
                    scope.spawn(move |_| {
                        let _tracing_guard = tracing_span.enter();
                        self.step_resolution(b);
                        SenderData::send(b, sender);
                    });
                }
            };

            while let Ok(SenderData { b, sender }) = receiver.recv() {
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
                    SenderData::send(b + Bidegree::s_t(0, 1), sender);
                }
            }
        });
    }
}

impl<M: ZeroModule<Algebra = MilnorAlgebra>> ChainComplex for Resolution<M> {
    type Algebra = MilnorAlgebra;
    type Homomorphism = FreeModuleHomomorphism<FreeModule<Self::Algebra>>;
    type Module = FreeModule<Self::Algebra>;

    fn prime(&self) -> ValidPrime {
        TWO
    }

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.zero_module.algebra()
    }

    fn module(&self, s: u32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s as usize])
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn min_degree(&self) -> i32 {
        0
    }

    fn has_computed_bidegree(&self, b: Bidegree) -> bool {
        self.differentials.len() > b.s() as usize && self.differential(b.s()).next_degree() > b.t()
    }

    fn differential(&self, s: u32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    #[tracing::instrument(skip(self), fields(self = self.name, max = %max))]
    fn compute_through_bidegree(&self, max: Bidegree) {
        let _lock = self.lock.lock();

        self.extend_through_degree(max.s());
        self.algebra().compute_basis(max.t());

        for t in 0..=max.t() {
            for s in 0..=max.s() {
                let b = Bidegree::s_t(s, t);
                if self.has_computed_bidegree(b) {
                    continue;
                }
                self.step_resolution(b);
            }
        }
    }

    fn next_homological_degree(&self) -> u32 {
        self.modules.len() as u32
    }

    fn save_dir(&self) -> &SaveDirectory {
        &self.save_dir
    }

    fn apply_quasi_inverse<T, S>(&self, results: &mut [T], b: Bidegree, inputs: &[S]) -> bool
    where
        for<'a> &'a mut T: Into<FpSliceMut<'a>>,
        for<'a> &'a S: Into<FpSlice<'a>>,
    {
        let mut f = if let Some(dir) = self.save_dir.read() {
            if let Some(f) = self.save_file(SaveKind::NassauQi, b).open_file(dir.clone()) {
                f
            } else {
                return false;
            }
        } else {
            return false;
        };

        let p = self.prime();

        let target_dim = f.read_u64::<LittleEndian>().unwrap() as usize;
        let zero_mask_dim = f.read_u64::<LittleEndian>().unwrap() as usize;
        let subalgebra = MilnorSubalgebra::from_bytes(&mut f).unwrap();
        let source = &self.modules[b.s()];
        let target = &self.modules[b.s() - 1];
        let algebra = target.algebra();

        let mut inputs: Vec<FpVector> = inputs.iter().map(|x| x.into().to_owned()).collect();
        let mut mask: Vec<usize> = Vec::with_capacity(zero_mask_dim + 8);
        mask.extend(subalgebra.signature_mask(
            &algebra,
            source,
            b.t(),
            &subalgebra.zero_signature(),
        ));

        let mut scratch0 = FpVector::new(p, zero_mask_dim);
        let mut scratch1 = FpVector::new(p, target_dim);

        // If the quasi-inverse was computed using incomplete information, we need to figure out
        // what the differentials in this bidegree hit and use them to lift. these variables are
        // trivial if there is no such problem.
        //
        // target_zero_mask is the signature mask of the target under the zero signature.
        //
        // dx_matrix is an AugmentedMatrix::<3>.
        //
        // Each row of this matrix is of the form [r; dx; x], where x is an element of the source
        // of signature zero, expressed in the masked basis, and dx is the value of the
        // differential on x. Then r is the entries of dx that have zero signature, which we
        // include so that the rref of the matix is nice. In practice, we keep r empty until the
        // very end, and then populate it manually.
        //
        // At the beginning the x's will be the new generators in this bidegree. As we read in the
        // quasi-inverses for the zero signature, we keep on reducing this so that dx is zero in
        // the pivot columns of the quasi-inverse. We can then use (the rref of) this matrix to
        // lift remaining elements with zero signature.
        let (mut target_zero_mask, mut dx_matrix) = if zero_mask_dim != mask.len() {
            let num_new_gens = source.number_of_gens_in_degree(b.t());
            assert_eq!(mask.len(), zero_mask_dim + num_new_gens);

            let target_zero_mask: Vec<usize> = subalgebra
                .signature_mask(&algebra, target, b.t(), &subalgebra.zero_signature())
                .collect();
            let mut matrix = AugmentedMatrix::<3>::new(
                p,
                num_new_gens,
                [target_zero_mask.len(), target.dimension(b.t()), mask.len()],
            );

            for i in 0..num_new_gens {
                let dx = self.differentials[b.s()].output(b.t(), i);
                matrix
                    .row_segment_mut(i, 1, 1)
                    .slice_mut(0, dx.len())
                    .add(dx.as_slice(), 1);
                matrix
                    .row_segment_mut(i, 2, 2)
                    .add_basis_element(zero_mask_dim + i, 1);
            }

            (target_zero_mask, matrix)
        } else {
            (Vec::new(), AugmentedMatrix::<3>::new(p, 0, [0, 0, 0]))
        };

        loop {
            let col = f.read_u64::<LittleEndian>().unwrap() as usize;
            if col == Magic::End as usize {
                break;
            } else if col == Magic::Signature as usize {
                let signature = subalgebra.signature_from_bytes(&mut f).unwrap();

                mask.clear();
                mask.extend(subalgebra.signature_mask(&algebra, source, b.t(), &signature));
                scratch0.set_scratch_vector_size(mask.len());
            } else if col == Magic::Fix as usize {
                // We need to fix the differential problem
                //
                // First manually add_masked the second segment to the first, which we use for
                // row reduction. We do this manually for borrow checker reasons.
                for (j, &k) in target_zero_mask.iter().enumerate() {
                    for i in 0..dx_matrix.rows() {
                        if dx_matrix.row_segment(i, 1, 1).entry(k) != 0 {
                            dx_matrix.row_segment_mut(i, 0, 0).add_basis_element(j, 1);
                        }
                    }
                }
                dx_matrix.row_reduce();

                // Now reduce by these elements
                for i in 0..dx_matrix.rows() {
                    let masked_col = dx_matrix[i].first_nonzero().unwrap().0;
                    assert_eq!(dx_matrix.pivots()[masked_col], i as isize);
                    let col = target_zero_mask[masked_col];

                    for (input, output) in inputs.iter_mut().zip(results.iter_mut()) {
                        let entry = input.entry(col);
                        if entry != 0 {
                            output
                                .into()
                                .add_unmasked(dx_matrix.row_segment(i, 2, 2), 1, &mask);
                            input.as_slice_mut().add(dx_matrix.row_segment(i, 1, 1), 1);
                        }
                    }
                }

                // Drop these objects to save a bit of memory
                target_zero_mask = Vec::new();
                dx_matrix = AugmentedMatrix::<3>::new(p, 0, [0, 0, 0]);
            } else {
                scratch0.update_from_bytes(&mut f).unwrap();
                scratch1.update_from_bytes(&mut f).unwrap();
                for (input, output) in inputs.iter_mut().zip(results.iter_mut()) {
                    let entry = input.entry(col);
                    if entry != 0 {
                        output.into().add_unmasked(scratch0.as_slice(), 1, &mask);
                        // If we resume a resolve_through_stem, input may be longer than scratch1.
                        input
                            .slice_mut(0, scratch1.len())
                            .add(scratch1.as_slice(), 1);
                    }
                }

                // Row reduce the differentials
                if !target_zero_mask.is_empty() {
                    for i in 0..dx_matrix.rows() {
                        if dx_matrix.row_segment(i, 1, 1).entry(col) != 0 {
                            dx_matrix
                                .row_segment_mut(i, 2, 2)
                                .slice_mut(0, zero_mask_dim)
                                .add(scratch0.as_slice(), 1);
                            dx_matrix
                                .row_segment_mut(i, 1, 1)
                                .slice_mut(0, target_dim)
                                .add(scratch1.as_slice(), 1);
                        }
                    }
                }
            }
        }
        // Make sure we have finished reading everything
        drop(f);

        for dx in inputs {
            assert!(
                dx.is_zero(),
                "remainder non-zero at {b}\nAlgebra: {subalgebra}\ndx: {}",
                target.element_to_string(b.t(), dx.as_slice())
            );
        }
        true
    }
}

impl<M: ZeroModule<Algebra = MilnorAlgebra>> AugmentedChainComplex for Resolution<M> {
    type ChainMap = FreeModuleHomomorphism<M>;
    type TargetComplex = FiniteChainComplex<M, FullModuleHomomorphism<M, M>>;

    fn target(&self) -> Arc<Self::TargetComplex> {
        Arc::clone(&self.target)
    }

    fn chain_map(&self, s: u32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s])
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn test_restart_stem() {
        let res = crate::utils::construct_nassau("S_2", None).unwrap();
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
    fn test_signature_iterator() {
        let subalgebra = MilnorSubalgebra::new(vec![2, 1]);
        assert_eq!(
            subalgebra.iter_signatures(6).collect::<Vec<_>>(),
            vec![
                vec![1, 0],
                vec![2, 0],
                vec![3, 0],
                vec![0, 1],
                vec![1, 1],
                vec![2, 1],
                vec![3, 1],
            ]
        );

        assert_eq!(
            subalgebra.iter_signatures(5).collect::<Vec<_>>(),
            vec![
                vec![1, 0],
                vec![2, 0],
                vec![3, 0],
                vec![0, 1],
                vec![1, 1],
                vec![2, 1],
            ]
        );
        assert_eq!(
            subalgebra.iter_signatures(4).collect::<Vec<_>>(),
            vec![vec![1, 0], vec![2, 0], vec![3, 0], vec![0, 1], vec![1, 1],]
        );
        assert_eq!(
            subalgebra.iter_signatures(3).collect::<Vec<_>>(),
            vec![vec![1, 0], vec![2, 0], vec![3, 0], vec![0, 1],]
        );
        assert_eq!(
            subalgebra.iter_signatures(2).collect::<Vec<_>>(),
            vec![vec![1, 0], vec![2, 0],]
        );
        assert_eq!(
            subalgebra.iter_signatures(1).collect::<Vec<_>>(),
            vec![vec![1, 0],]
        );
        assert_eq!(
            subalgebra.iter_signatures(0).collect::<Vec<_>>(),
            Vec::<Vec<PPartEntry>>::new()
        );
    }

    #[test]
    fn test_signature_iterator_large() {
        let subalgebra = MilnorSubalgebra::new(vec![
            0,
            MilnorSubalgebra::INFINITY,
            MilnorSubalgebra::INFINITY,
            MilnorSubalgebra::INFINITY,
        ]);
        assert_eq!(
            subalgebra.iter_signatures(7).collect::<Vec<_>>(),
            vec![vec![0, 1, 0, 0], vec![0, 2, 0, 0], vec![0, 0, 1, 0],]
        );
    }
}
