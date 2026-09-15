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
    collections::HashMap,
    fmt::Display,
    io,
    sync::{Arc, Mutex, mpsc},
};

use algebra::{
    Algebra, combinatorics,
    milnor_algebra::{MilnorAlgebra, PPart, PPartEntry},
    module::{
        FreeModule, GeneratorData, Module, ZeroModule,
        homomorphism::{FreeModuleHomomorphism, FullModuleHomomorphism, ModuleHomomorphism},
    },
};
use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fp::{
    matrix::{AugmentedMatrix, Matrix},
    prime::{TWO, ValidPrime},
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::{Either, Itertools};
use once::OnceBiVec;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use crate::{
    chain_complex::{AugmentedChainComplex, ChainComplex, FiniteChainComplex},
    save::{SaveDirectory, SaveKind},
    utils::{LogWriter, parallel::ParallelGuard},
};

/// See [`resolution::SenderData`](../resolution/struct.SenderData.html). This differs by not having
/// the `new` field. What a computed bidegree still has to register.
///
/// `modules[s]` and `differentials[s]` are append-only in increasing degree, so registration has to
/// happen in `t` order within a row even when the computations that produced it did not. Carrying
/// it as a value lets the scheduler apply it in graph order; nothing waits on a lock to do so.
pub(crate) struct PendingRegistration {
    b: Bidegree,
    num_new_gens: usize,
    /// One row per new generator: its differential, in the target's full basis.
    rows: Vec<FpVector>,
    /// Column count the rows were built against, needed by the save format.
    target_dim: usize,
    /// False when the differential was just READ from a save file and must not be rewritten.
    write_save: bool,
    /// Whether to extend the chain map by zero. The main path does; the save-load path never did,
    /// and this keeps that difference rather than quietly changing it.
    extend_chain_map: bool,
}

struct SenderData {
    b: Bidegree,
    retry: bool,
    /// What the worker computed and the scheduler still has to register, if anything.
    pending: Option<PendingRegistration>,
    sender: mpsc::Sender<Self>,
}

impl SenderData {
    pub(crate) fn send(
        b: Bidegree,
        pending: Option<PendingRegistration>,
        sender: mpsc::Sender<Self>,
    ) {
        sender
            .send(Self {
                b,
                retry: false,
                pending,
                sender: sender.clone(),
            })
            .unwrap()
    }

    pub(crate) fn send_retry(b: Bidegree, sender: mpsc::Sender<Self>) {
        tracing::info!(%b, "retrying");
        sender
            .send(Self {
                b,
                retry: true,
                pending: None,
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

    /// The test "does this element have this signature" compiled into a `(mask, value)` pair to
    /// match against the packed p-part.
    ///
    /// The per-entry test is `ppart[i] & ((1 << profile[i]) - 1) == signature[i]`. Because each
    /// entry occupies a fixed field of the packed word, the low `profile[i]` bits of entry `i` are
    /// a fixed bit range of that word, so the whole conjunction is a single `&` and `==`. Entries
    /// past the end of the p-part read as zero, which the packing already gives us for free.
    ///
    /// Returns `None` when an entry is too large to be one, since then no element matches.
    fn packed_signature(&self, signature: &[PPartEntry]) -> Option<(u64, u64)> {
        let mut mask = 0;
        let mut value = 0;
        for (i, (&profile, &entry)) in self.profile.iter().zip(signature).enumerate() {
            // An entry wider than its field would shift into the next one, so it cannot simply be
            // packed and compared.
            if entry > PPart::max_entry(i) {
                return None;
            }
            // A profile wider than the field constrains the whole field.
            let width = std::cmp::min(profile as u32, PPart::width(i));
            mask |= ((1u64 << width) - 1) << PPart::shift(i);
            value |= (entry as u64) << PPart::shift(i);
        }
        Some((mask, value))
    }

    fn zero_signature(&self) -> Vec<PPartEntry> {
        vec![0; self.profile.len()]
    }

    /// The smallest POSITIVE degree of an operation carrying the zero signature.
    ///
    /// An operation has the zero signature when every p-part entry is zero modulo its field
    /// width, so the smallest non-trivial one is `min_i 2^{w_i} (2^i - 1)`; for `A(k)` that is
    /// `2^{k + 1}`, i.e. 2, 4, 8, 16, 32 for `A(0)` through `A(4)`.
    ///
    /// This bounds how far ahead of its own row a bidegree may be computed. The image is built at
    /// the zero signature, so a source generator whose operation degree is below this floor
    /// contributes no basis element to it, and excluding such generators cannot change the answer.
    ///
    /// It is a property of the SUBALGEBRA, hence of the bidegree. A single global bound is wrong:
    /// one large enough to help an `A(3)` bidegree exceeds the floor of a nearby `A(0)` or `A(1)`
    /// one, and excluding generators that do contribute inflates the generator count without bound.
    fn zero_signature_floor(&self) -> i32 {
        self.profile
            .iter()
            .enumerate()
            .map(|(i, &entry)| {
                let width = std::cmp::min(entry as u32, PPart::width(i));
                ((1i64 << width) * ((1i64 << (i + 1)) - 1)) as i32
            })
            .min()
            .unwrap_or(1)
    }

    /// Give a list of basis elements in degree `degree` that has signature `signature`.
    ///
    /// Only basis elements coming from generators of degree strictly less than `max_gen_degree`
    /// are considered; `None` imposes no restriction. Because generators are laid out in
    /// increasing degree, a restricted result is a prefix of the unrestricted one; see
    /// [`Resolution::step_resolution_with_subalgebra`] for why we restrict.
    ///
    /// This requires passing the algebra for borrow checker reasons.
    fn signature_mask<'a>(
        &'a self,
        algebra: &'a MilnorAlgebra,
        module: &'a FreeModule<MilnorAlgebra>,
        degree: i32,
        signature: &'a [PPartEntry],
        max_gen_degree: Option<i32>,
    ) -> impl Iterator<Item = usize> + 'a {
        // Every element is tested against the same signature, so compile it once.
        let Some((mask, value)) = self.packed_signature(signature) else {
            return Either::Right(std::iter::empty());
        };

        let matching = module
            .iter_gen_offsets([degree])
            .take_while(move |gen_data| max_gen_degree.is_none_or(|bound| gen_data.gen_deg < bound))
            .flat_map(move |gen_data| {
                let GeneratorData {
                    gen_deg,
                    start: [offset],
                    ..
                } = gen_data;
                algebra
                    .ppart_table(degree - gen_deg)
                    .iter()
                    .enumerate()
                    .filter_map(move |(n, op)| (op.bits() & mask == value).then_some(offset + n))
            });

        Either::Left(matching)
    }

    /// Get the matrix of a free module homomorphism when restricted to the subquotient given by
    /// the signature.
    ///
    /// Only generators of the target of degree strictly less than `target_max_gen_degree` are used
    /// (see [`Self::signature_mask`]).
    fn signature_matrix(
        &self,
        hom: &FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>,
        degree: i32,
        signature: &[PPartEntry],
        target_max_gen_degree: i32,
        // Generators of the SOURCE at or above this degree are excluded; `None` reads them all. The
        // image is built at the zero signature, so passing the zero-signature floor here lets a
        // bidegree be computed before its own row predecessor has registered.
        source_max_gen_degree: Option<i32>,
    ) -> Matrix {
        let p = hom.prime();
        let source = hom.source();
        let target = hom.target();
        let algebra = target.algebra();
        let target_degree = degree - hom.degree_shift();

        let target_mask: Vec<usize> = self
            .signature_mask(
                &algebra,
                &target,
                target_degree,
                signature,
                Some(target_max_gen_degree),
            )
            .collect();

        let source_mask: Vec<usize> = self
            .signature_mask(&algebra, &source, degree, signature, source_max_gen_degree)
            .collect();

        let mut scratch = FpVector::new(
            p,
            target.dimension_from_gens_below(target_degree, target_max_gen_degree),
        );
        let mut result = Matrix::new(p, source_mask.len(), target_mask.len());

        for (mut row, &masked_index) in std::iter::zip(result.iter_mut(), &source_mask) {
            scratch.set_to_zero();
            hom.apply_to_basis_element_restricted(scratch.as_slice_mut(), 1, degree, masked_index);

            row.add_masked(scratch.as_slice(), 1, &target_mask);
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
            b.t() >= coeff * (b.s() + 1) + subalgebra.top_degree()
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
        // The packed p-part has no entry past `PPart::MAX_LEN`, so a longer profile cannot be
        // matched against one. This is the only place a profile is built from outside data, and
        // the bound has to hold before narrowing, which truncates where `usize` is 32 bits.
        let len = data.read_u64::<LittleEndian>()?;
        if len > PPart::MAX_LEN as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("profile length {len} exceeds {}", PPart::MAX_LEN),
            ));
        }
        let len = len as usize;
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
            write!(out, "B({})", self.profile.iter().join(","))
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
    modules: OnceBiVec<Arc<FreeModule<MilnorAlgebra>>>,
    zero_module: Arc<FreeModule<MilnorAlgebra>>,
    differentials: OnceBiVec<Arc<FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>>>,
    target: Arc<FiniteChainComplex<M>>,
    chain_maps: OnceBiVec<Arc<FreeModuleHomomorphism<M>>>,
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
            modules: OnceBiVec::new(0),
            differentials: OnceBiVec::new(0),
            chain_maps: OnceBiVec::new(0),
            target,
            max_degree,
            save_dir,
        })
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

        self.modules.extend(max_s, |i| {
            Arc::new(FreeModule::new(
                Arc::clone(&self.algebra()),
                format!("F{i}"),
                min_degree,
            ))
        });

        self.differentials.extend(0, |_| {
            Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[0]),
                Arc::clone(&self.zero_module),
                0,
            ))
        });

        self.differentials.extend(max_s, |i| {
            Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[i]),
                Arc::clone(&self.modules[i - 1]),
                0,
            ))
        });

        self.chain_maps.extend(max_s, |i| {
            Arc::new(FreeModuleHomomorphism::new(
                Arc::clone(&self.modules[i]),
                self.target.module(i),
                0,
            ))
        });
    }

    #[tracing::instrument(skip_all, fields(throughput))]
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

    #[tracing::instrument(skip(self), fields(%b, %subalgebra, num_new_gens, density))]
    fn step_resolution_with_subalgebra(
        &self,
        b: Bidegree,
        subalgebra: MilnorSubalgebra,
    ) -> anyhow::Result<PendingRegistration> {
        // Takes the count rather than reading it back from the module: registration now happens in
        // a later phase, so `number_of_gens_in_bidegree` would still be zero here. `density` is
        // dropped for the same reason -- it reads the registered differential -- and is reported by
        // the registration phase instead.
        let end = |num_new_gens: usize| {
            tracing::Span::current().record("num_new_gens", num_new_gens);
        };

        let p = self.prime();
        let mut scratch = FpVector::new(p, 0);

        let target = &*self.modules[b.s() - 1];
        let algebra = target.algebra();

        // We compute this bidegree treating the target `C_{b.s() - 1}` as if it had no generators
        // of degree `>= b.t()`, and `C_{b.s() - 2}` as if it had none of degree `>= b.t() - 1`.
        // By minimality this loses no information (the differentials we care about land in the
        // radical, hence in strictly lower-degree generators), and it makes the computation depend
        // only on data that is frozen once `(b.s() - 1, b.t() - 1)` and `(b.s(), b.t() - 1)` have
        // been committed. This is what lets [`Self::compute_through_stem`] compute `(b.s(), b.t())`
        // concurrently with `(b.s() - 1, b.t())`, which is adding those degree-`b.t()` generators.
        let target_bound = b.t();
        let next_bound = b.t() - 1;

        let zero_sig = subalgebra.zero_signature();
        let target_dim = target.dimension_from_gens_below(b.t(), target_bound);
        let target_mask: Vec<usize> = subalgebra
            .signature_mask(&algebra, target, b.t(), &zero_sig, Some(target_bound))
            .collect();
        let target_masked_dim = target_mask.len();

        let next = &self.modules[b.s() - 2];
        next.compute_basis(b.t());
        let next_dim = next.dimension_from_gens_below(b.t(), next_bound);

        let mut f = if let Some(dir) = self.save_dir().write() {
            let mut f = self
                .save_file(SaveKind::NassauQi, b - Bidegree::s_t(1, 0))
                .create_file(dir.to_owned(), true);
            f.write_u64::<LittleEndian>(next_dim as u64)?;
            f.write_u64::<LittleEndian>(target_masked_dim as u64)?;
            subalgebra.to_bytes(&mut f)?;
            Some(f)
        } else {
            None
        };

        let guard = tracing::info_span!("step", signature = ?zero_sig).entered();
        let next_mask: Vec<usize> = subalgebra
            .signature_mask(&algebra, next, b.t(), &zero_sig, Some(next_bound))
            .collect();
        let next_masked_dim = next_mask.len();

        let full_matrix = {
            let _guard = ParallelGuard::new();
            self.differentials[b.s() - 1].get_partial_matrix_restricted(
                b.t(),
                &target_mask,
                next_dim,
            )
        };
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

        // The quasi-inverse is always computed on the restricted (degree `< b.t()`) target basis,
        // so from the point of view of a later `apply_quasi_inverse` it was computed with
        // "incomplete information": the differentials on the degree-`b.t()` generators of the
        // target were not available. We flag this unconditionally so the lift is corrected using
        // those differentials once they are known.
        if let Some(f) = &mut f {
            f.write_u64::<LittleEndian>(Magic::Fix as u64)?;
        }

        // Compute image
        let mut n = subalgebra.signature_matrix(
            &self.differentials[b.s()],
            b.t(),
            &zero_sig,
            target_bound,
            Some(b.t() - (subalgebra.zero_signature_floor() - 1)),
        );
        n.row_reduce();
        let next_row = n.rows();

        let num_new_gens = n.extend_image(0, n.columns(), &kernel, 0).len();

        if b.t() < b.s() {
            assert_eq!(num_new_gens, 0, "Adding generators at {b}");
        }

        // NOT registered here: `add_generators` appends to `modules[b.s()]` in increasing degree,
        // and this bidegree may have been computed before its row predecessor. The count travels
        // back to the scheduler in `PendingRegistration` instead. Nothing between here and the end
        // of the signature loop reads `modules[b.s()]` -- the loop works on rows `s-1` and `s-2`.
        let mut xs = vec![FpVector::new(p, target_dim); num_new_gens];
        let mut dxs = vec![FpVector::new(p, next_dim); num_new_gens];

        for ((x, x_masked), dx) in xs
            .iter_mut()
            .zip_eq(n.iter().skip(next_row))
            .zip_eq(&mut dxs)
        {
            x.as_slice_mut().add_unmasked(x_masked, 1, &target_mask);
            for (i, _) in x_masked.iter_nonzero() {
                dx.as_slice_mut().add(full_matrix.row(i), 1);
            }
        }

        // Now add correction terms
        let mut target_mask: Vec<usize> = Vec::new();
        let mut next_mask: Vec<usize> = Vec::new();

        drop(guard);

        for signature in subalgebra.iter_signatures(b.t()) {
            let _guard = tracing::info_span!("step", ?signature).entered();
            target_mask.clear();
            next_mask.clear();
            target_mask.extend(subalgebra.signature_mask(
                &algebra,
                target,
                b.t(),
                &signature,
                Some(target_bound),
            ));
            next_mask.extend(subalgebra.signature_mask(
                &algebra,
                next,
                b.t(),
                &signature,
                Some(next_bound),
            ));

            let full_matrix = {
                let _guard = ParallelGuard::new();
                self.differentials[b.s() - 1].get_partial_matrix_restricted(
                    b.t(),
                    &target_mask,
                    next_dim,
                )
            };

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
                        scratch.as_slice_mut().add(preimage.row(row), 1);
                    }
                    row += 1;
                }
                for (i, _) in scratch.iter_nonzero() {
                    x.add_basis_element(target_mask[i], 1);
                    dx.as_slice_mut().add(full_matrix.row(i), 1);
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
        end(num_new_gens);

        if let Some(f) = &mut f {
            f.write_u64::<LittleEndian>(Magic::End as u64)?;
        }

        Ok(PendingRegistration {
            b,
            num_new_gens,
            rows: xs,
            target_dim,
            write_save: true,
            extend_chain_map: true,
        })
    }

    /// Step resolution for s = 0
    #[tracing::instrument(skip(self))]
    fn step0(&self, t: i32) {
        self.zero_module.extend_by_zero(t);

        let source_module = &self.modules[0];
        let target_module = self.target.module(0);

        let chain_map = &self.chain_maps[0];
        let d = &self.differentials[0];

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
            {
                let _guard = ParallelGuard::new();
                chain_map.get_matrix(matrix.segment(0, 0), t);
            }
            matrix.segment(1, 1).add_identity();

            matrix.row_reduce();

            let num_new_gens = matrix.extend_to_surjection(0, target_dim, 0).len();

            self.add_generators(Bidegree::s_t(0, t), num_new_gens);

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

        let source_module = &self.modules[1];
        let target_module = &self.modules[0];
        let cc_module = self.target.module(0);

        let source_dim = source_module.dimension(t);
        let target_dim = target_module.dimension(t);

        let mut matrix =
            AugmentedMatrix::<2>::new(p, target_dim, [cc_module.dimension(t), target_dim]);
        {
            let _guard = ParallelGuard::new();
            self.chain_maps[0].get_matrix(matrix.segment(0, 0), t);
        }
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
        {
            let _guard = ParallelGuard::new();
            self.differentials[1].get_matrix(matrix.segment(0, 0), t);
        }
        matrix.segment(1, 1).add_identity();
        matrix.row_reduce();

        let num_new_gens = matrix.extend_image(0, target_dim, &desired_image, 0).len();

        self.add_generators(Bidegree::s_t(1, t), num_new_gens);

        self.differentials[1].add_generators_from_matrix_rows(
            t,
            matrix
                .segment(0, 0)
                .row_slice(source_dim, source_dim + num_new_gens),
        );

        self.write_differential(Bidegree::s_t(1, t), num_new_gens, target_dim)?;
        Ok(())
    }

    /// Compute `b`, returning what still has to be registered.
    ///
    /// `None` means the bidegree registered itself: rows 0 and 1 keep the strict schedule, so their
    /// row order is already guaranteed and there is nothing for the scheduler to sequence.
    fn step_resolution_with_result(
        &self,
        b: Bidegree,
    ) -> anyhow::Result<Option<PendingRegistration>> {
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
            return Ok(None);
        }

        if let Some(dir) = self.save_dir.read()
            && let Some(mut f) = self
                .save_file(SaveKind::NassauDifferential, b)
                .open_file(dir.clone())
        {
            tracing::info!(%b, "Loading differential");

            let num_new_gens = f.read_u64::<LittleEndian>()? as usize;
            // This need not be equal to `target_res_dimension`. If we saved a big resolution
            // and now only want to load up to a small stem, then `target_res_dimension` will
            // be smaller. If we have previously saved a small resolution up to a stem and now
            // want to resolve further, it will be bigger.
            let saved_target_res_dimension = f.read_u64::<LittleEndian>()? as usize;

            let mut d_targets = Vec::with_capacity(num_new_gens);

            for _ in 0..num_new_gens {
                d_targets.push(FpVector::from_bytes(p, saved_target_res_dimension, &mut f)?);
            }

            return Ok(Some(PendingRegistration {
                b,
                num_new_gens,
                rows: d_targets,
                target_dim: saved_target_res_dimension,
                // Read from a save file; rewriting it would be pointless work.
                write_save: false,
                extend_chain_map: false,
            }));
        }

        if b.s() == 1 {
            self.step1(b.t())?;
            set_data();
            return Ok(None);
        }

        let pending = self.step_resolution_with_subalgebra(
            b,
            MilnorSubalgebra::optimal_for(b - Bidegree::s_t(0, self.max_degree)),
        )?;
        Ok(Some(pending))
    }

    /// Apply a computed bidegree's registration.
    ///
    /// Everything here appends per degree -- `modules[s]`, the differential's outputs, the chain
    /// map, and the per-degree subspace caches -- so it must run in `t` order within a row. The
    /// scheduler guarantees that by ordering the `Register` nodes; nothing here waits on a lock.
    fn register(&self, pending: PendingRegistration) -> anyhow::Result<()> {
        let PendingRegistration {
            b,
            num_new_gens,
            rows,
            target_dim,
            write_save,
            extend_chain_map,
        } = pending;

        self.add_generators(b, num_new_gens);
        self.differentials[b.s()].add_generators_from_rows(b.t(), rows);

        if write_save {
            self.write_differential(b, num_new_gens, target_dim)?;
        }
        if extend_chain_map {
            self.chain_maps[b.s()].extend_by_zero(b.t());
        }

        // `density` used to be recorded as a span field on the compute span; it reads the
        // registered differential, so it belongs here now and is emitted as an event instead.
        tracing::info!(
            %b,
            num_new_gens,
            density = self.differentials[b.s()].differential_density(b.t()) * 100.0,
            "registered"
        );

        let d = &self.differentials[b.s()];
        let c = &self.chain_maps[b.s()];
        d.set_kernel(b.t(), None);
        d.set_image(b.t(), None);
        d.set_quasi_inverse(b.t(), None);
        c.set_kernel(b.t(), None);
        c.set_image(b.t(), None);
        c.set_quasi_inverse(b.t(), None);
        Ok(())
    }

    /// [`Self::step_resolution_with_result`], panicking rather than returning the error.
    fn step_resolution(&self, b: Bidegree) -> Option<PendingRegistration> {
        self.step_resolution_with_result(b)
            .unwrap_or_else(|e| panic!("Error computing bidegree {b}: {e}"))
    }

    /// This function resolves up till a fixed stem instead of a fixed t.
    ///
    /// The dependency graph is built explicitly, with each bidegree split into a `Compute` node and
    /// a `Register` node; see `depgraph` for why. `Compute` runs on a worker and returns what has
    /// to be registered; `Register` is applied by this function, in graph order, so appends to
    /// `modules[s]` and `differentials[s]` stay in increasing degree without any worker ever
    /// blocking on its row predecessor.
    ///
    /// `Compute(s, t)` requires `(s - 1, t - 1)` registered rather than `(s - 1, t)` -- the relaxed
    /// diagonal, see `step_resolution_with_subalgebra` for why that suffices -- which lets `(s, t)`
    /// run concurrently with `(s - 1, t)`. Rows 0 and 1 keep the strict schedule: `step0` and
    /// `step1` read their targets through full matrices, so they wait for `(s - 1, t)`.
    #[tracing::instrument(skip(self), fields(self = self.name, %max))]
    pub fn compute_through_stem(&self, max: Bidegree) {
        use depgraph::{Node, Phase};

        let _lock = self.lock.lock();

        self.extend_through_degree(max.s());
        self.algebra().compute_basis(max.t());

        let min_degree = self.min_degree();
        let max_s = max.s();
        let max_n = max.n();

        // How far back in its own row `(s, t)` actually reads.
        //
        // The image is built at the ZERO signature, and no operation below the subalgebra's
        // zero-signature floor carries it, so generators within that many degrees contribute
        // nothing to the image and need not be registered yet. `signature_matrix` is passed the
        // matching bound, so the two agree by construction.
        //
        // Rows 0 and 1 keep the strict schedule: `step0` and `step1` read their targets through
        // full matrices rather than the signature-masked image, so this reasoning does not apply to
        // them.
        //
        // The floor is per bidegree, via the same subalgebra the computation will use. A single
        // global value would be wrong in both directions: too small to help `A(3)`, and large
        // enough to exclude generators that a nearby `A(0)` or `A(1)` bidegree genuinely needs.
        let same_row_dep = |s: i32, t: i32| -> i32 {
            if s <= 1 {
                return t - 1;
            }
            let b = Bidegree::s_t(s, t);
            let subalgebra = MilnorSubalgebra::optimal_for(b - Bidegree::s_t(0, self.max_degree));
            t - subalgebra.zero_signature_floor()
        };

        // A predecessor outside the region imposes no edge, which is what makes the base of each
        // row a source; that replaces seeding a `progress` array to `min_degree - 1` so the
        // comparison happened to hold.
        let mut graph = depgraph::Graph::new(min_degree, max_s, max_n, same_row_dep);

        let tracing_span = tracing::Span::current();
        maybe_rayon::in_place_scope(|scope| {
            let _tracing_guard = tracing_span.enter();

            let (sender, receiver) = mpsc::channel();

            let spawn_compute = |b: Bidegree, sender: mpsc::Sender<SenderData>| {
                if self.has_computed_bidegree(b) {
                    // Already present, so there is nothing to compute and nothing to register. It
                    // still travels the normal completion path so its successors are released in
                    // the one place that does that.
                    SenderData::send(b, None, sender);
                } else {
                    let tracing_span = tracing_span.clone();
                    scope.spawn(move |_| {
                        let _tracing_guard = tracing_span.enter();
                        if crate::utils::parallel::is_in_parallel() {
                            SenderData::send_retry(b, sender);
                            return;
                        }
                        let pending = self.step_resolution(b);
                        SenderData::send(b, pending, sender);
                    });
                }
            };

            // A computed bidegree's registration, waiting for its `Register` node to come up.
            let mut payloads: HashMap<Bidegree, Option<PendingRegistration>> = HashMap::new();
            let mut in_flight = 0usize;

            loop {
                // Dispatch everything the graph has freed. Running a `Register` can free more, so
                // this drains rather than taking one pass.
                while let Some(node) = graph.pop_ready() {
                    let b = node.bidegree();
                    match node.phase {
                        Phase::Compute => {
                            in_flight += 1;
                            spawn_compute(b, sender.clone());
                        }
                        Phase::Register => {
                            if let Some(pending) = payloads.remove(&b).flatten() {
                                self.register(pending).unwrap_or_else(|e| {
                                    panic!("Error registering bidegree {b}: {e}")
                                });
                            }
                            graph.complete(node);
                        }
                    }
                }

                if in_flight == 0 {
                    break;
                }

                let Ok(SenderData {
                    b,
                    retry,
                    pending,
                    sender,
                }) = receiver.recv()
                else {
                    break;
                };
                if retry {
                    // Bounced off a worker already inside a parallel section. Still in flight; hand
                    // it back out without touching the graph.
                    spawn_compute(b, sender);
                    continue;
                }
                in_flight -= 1;
                payloads.insert(b, pending);
                graph.complete(Node::compute(b));
            }

            // A stalled graph is an edge bug, not a slow run.
            let stuck = graph.undispatched();
            assert!(
                stuck.is_empty(),
                "dependency graph stalled: {} nodes never dispatched, first {:?}",
                stuck.len(),
                &stuck[..stuck.len().min(5)]
            );
        });
    }
}

/// The dependency graph for [`Resolution::compute_through_stem`].
///
/// Each bidegree is TWO nodes, because its two halves have different dependencies:
///
/// * `Compute(s, t)` does the expensive work. It reads rows `s-1` and `s-2` only, so it needs those
///   registered -- but of its OWN row it needs only what the image computation reads.
/// * `Register(s, t)` appends to `modules[s]` and `differentials[s]`, which are append-only in
///   increasing degree, so it needs `Register(s, t-1)`.
///
/// Splitting them is what lets a row compute out of order while still registering in order. The
/// scheduler dispatches `Compute` to workers and runs `Register` itself, so a node is only ever
/// handed out when it can run immediately -- nothing blocks a worker waiting for its predecessor.
///
/// Readiness is an indegree reaching zero rather than a predicate over per-row high-water marks.
/// That matters: with a predicate, "each bidegree is dispatched exactly once" was an EMERGENT
/// property of needing both predecessors, and any relaxation silently broke it into double
/// dispatch. Here a node leaves `blocked` exactly once, by construction.
mod depgraph {
    use sseq::coordinates::Bidegree;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Phase {
        Compute,
        Register,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Node {
        pub phase: Phase,
        pub s: i32,
        pub t: i32,
    }

    impl Node {
        /// The node that computes `b` without registering it.
        pub fn compute(b: Bidegree) -> Self {
            Self {
                phase: Phase::Compute,
                s: b.s(),
                t: b.t(),
            }
        }

        /// The bidegree this node belongs to, discarding its phase.
        pub fn bidegree(&self) -> Bidegree {
            Bidegree::s_t(self.s, self.t)
        }
    }

    /// The dependency graph for [`Resolution::compute_through_stem`].
    ///
    /// Each bidegree is TWO nodes, because its two halves have different dependencies:
    ///
    /// * `Compute(s, t)` does the expensive work. It reads rows `s-1` and `s-2` only, so it needs
    ///   those registered -- but of its OWN row it needs only what the image computation reads.
    /// * `Register(s, t)` appends to `modules[s]` and `differentials[s]`, which are append-only in
    ///   increasing degree, so it needs `Register(s, t-1)`.
    ///
    /// Splitting them is what lets a row compute out of order while still registering in order. The
    /// scheduler dispatches `Compute` to workers and runs `Register` itself, so a node is only ever
    /// handed out when it can run immediately -- nothing blocks a worker waiting for its
    /// predecessor.
    ///
    /// Readiness is an indegree reaching zero rather than a predicate over per-row high-water
    /// marks. That matters: with a predicate, "each bidegree is dispatched exactly once" was an
    /// EMERGENT property of needing both predecessors, and any relaxation silently broke it into
    /// double dispatch. Here a node leaves the blocked set exactly once, by construction.
    ///
    /// # Representation
    ///
    /// Nothing is stored that the precedence rules already determine. Every row of the region is
    /// one contiguous run of `t`, so a node's slot is arithmetic rather than a hash key, and both
    /// the indegrees and the edges follow from the rules below:
    ///
    /// ```text
    /// Register(s, t-1)               -> Register(s, t)      appends are in increasing degree
    /// Compute(s, t)                  -> Register(s, t)
    /// Register(s, same_row[(s, t)])  -> Compute(s, t)       the same-row read, relaxed
    /// Register(0, t)                 -> Compute(1, t)       row 1 reads through a full matrix
    /// Register(s-1, t-1)             -> Compute(s, t)       the relaxed diagonal, s >= 2
    /// ```
    ///
    /// So the only per-node state is an indegree and a dispatched flag, both dense arrays. The
    /// same-row bounds are kept because they are the one input the rules cannot recompute cheaply;
    /// `max_gap` bounds the inverse lookup that finds a `Register`'s same-row consumers.
    pub struct Graph {
        min_degree: i32,
        max_s: i32,
        max_n: i32,
        /// Prefix sums of the row lengths, so `idx` is a single add.
        row_offset: Vec<usize>,
        /// `Compute(s, t)` waits for `Register(s, same_row[idx(s, t)])`.
        same_row: Vec<i32>,
        /// The widest `t - same_row[..]`, which bounds the scan in [`Self::successors`].
        max_gap: i32,
        /// Predecessors not yet complete, indexed by slot.
        blocked: Vec<u32>,
        ready: Vec<u32>,
        dispatched: Vec<bool>,
        /// Reused by [`Self::complete`] so releasing successors never allocates.
        succ_buf: Vec<Node>,
    }

    impl Graph {
        /// Build the whole graph and prime the ready queue.
        ///
        /// `same_row_dep(s, t)` is the earliest degree in row `s` that `Compute(s, t)` reads; a
        /// value outside the region imposes no edge, which is what makes the base of each row a
        /// source.
        pub fn new(
            min_degree: i32,
            max_s: i32,
            max_n: i32,
            same_row_dep: impl Fn(i32, i32) -> i32,
        ) -> Self {
            let mut row_offset = Vec::with_capacity(max_s as usize + 2);
            let mut total = 0usize;
            for s in 0..=max_s {
                row_offset.push(total);
                total += (max_n + s - min_degree + 1).max(0) as usize;
            }
            row_offset.push(total);

            let mut g = Self {
                min_degree,
                max_s,
                max_n,
                row_offset,
                same_row: Vec::new(),
                max_gap: 1,
                blocked: Vec::new(),
                ready: Vec::new(),
                dispatched: vec![false; 2 * total],
                succ_buf: Vec::new(),
            };

            g.same_row = Vec::with_capacity(total);
            for s in 0..=max_s {
                for t in min_degree..=(max_n + s) {
                    let dep = if g.in_region(s, t) {
                        same_row_dep(s, t)
                    } else {
                        t - 1
                    };
                    g.max_gap = g.max_gap.max(t - dep);
                    g.same_row.push(dep);
                }
            }

            g.blocked = (0..2 * total)
                .map(|slot| g.indegree(g.node_at(slot)))
                .collect();
            // Descending, so `pop` hands out ascending slots and two runs are diffable.
            g.ready = (0..2 * total)
                .rev()
                .filter(|&slot| g.blocked[slot] == 0)
                .map(|slot| slot as u32)
                .collect();
            g
        }

        fn in_region(&self, s: i32, t: i32) -> bool {
            (0..=self.max_s).contains(&s) && t >= self.min_degree && t - s <= self.max_n
        }

        fn idx(&self, s: i32, t: i32) -> usize {
            self.row_offset[s as usize] + (t - self.min_degree) as usize
        }

        fn slot(&self, n: Node) -> usize {
            2 * self.idx(n.s, n.t) + usize::from(n.phase == Phase::Register)
        }

        fn node_at(&self, slot: usize) -> Node {
            let phase = if slot % 2 == 0 {
                Phase::Compute
            } else {
                Phase::Register
            };
            let idx = slot / 2;
            // The row is the last one starting at or before `idx`.
            let s = self.row_offset.partition_point(|&o| o <= idx) - 1;
            Node {
                phase,
                s: s as i32,
                t: self.min_degree + (idx - self.row_offset[s]) as i32,
            }
        }

        fn same_row_dep(&self, s: i32, t: i32) -> i32 {
            self.same_row[self.idx(s, t)]
        }

        fn indegree(&self, n: Node) -> u32 {
            let (s, t) = (n.s, n.t);
            match n.phase {
                // Its own compute, plus the row predecessor whose appends must land first.
                Phase::Register => 1 + u32::from(self.in_region(s, t - 1)),
                Phase::Compute => {
                    let mut k = u32::from(self.in_region(s, self.same_row_dep(s, t)));
                    if s == 1 {
                        k += u32::from(self.in_region(0, t));
                    } else if s >= 2 {
                        k += u32::from(self.in_region(s - 1, t - 1));
                    }
                    k
                }
            }
        }

        /// The nodes `n` blocks, derived from the rules rather than stored.
        fn successors(&self, n: Node, out: &mut Vec<Node>) {
            out.clear();
            let (s, t) = (n.s, n.t);
            match n.phase {
                Phase::Compute => out.push(Node {
                    phase: Phase::Register,
                    s,
                    t,
                }),
                Phase::Register => {
                    if self.in_region(s, t + 1) {
                        out.push(Node {
                            phase: Phase::Register,
                            s,
                            t: t + 1,
                        });
                    }
                    // Same-row consumers: every `t'` whose read reaches back exactly to `t`. The
                    // gap is bounded by the widest zero-signature floor in the region, so this is a
                    // short scan and not a stored edge list.
                    for tp in (t + 1)..=(t + self.max_gap) {
                        if self.in_region(s, tp) && self.same_row_dep(s, tp) == t {
                            out.push(Node {
                                phase: Phase::Compute,
                                s,
                                t: tp,
                            });
                        }
                    }
                    if s == 0 {
                        if self.in_region(1, t) {
                            out.push(Node {
                                phase: Phase::Compute,
                                s: 1,
                                t,
                            });
                        }
                    } else if self.in_region(s + 1, t + 1) {
                        out.push(Node {
                            phase: Phase::Compute,
                            s: s + 1,
                            t: t + 1,
                        });
                    }
                }
            }
        }

        /// The next node whose dependencies are all complete, or `None` if there is none right
        /// now. A node is handed out at most once, however many predecessors freed it.
        pub fn pop_ready(&mut self) -> Option<Node> {
            while let Some(slot) = self.ready.pop() {
                let slot = slot as usize;
                if !self.dispatched[slot] {
                    self.dispatched[slot] = true;
                    return Some(self.node_at(slot));
                }
            }
            None
        }

        /// Mark `n` complete, moving anything it was blocking into the ready queue.
        ///
        /// This is the ONLY way a node becomes ready, so "reports completion" and "releases
        /// successors" cannot diverge -- previously that could differ per early return in a
        /// bidegree's body.
        pub fn complete(&mut self, n: Node) {
            let mut buf = std::mem::take(&mut self.succ_buf);
            self.successors(n, &mut buf);
            for &d in &buf {
                let slot = self.slot(d);
                debug_assert!(self.blocked[slot] > 0, "releasing {d:?} twice");
                self.blocked[slot] -= 1;
                if self.blocked[slot] == 0 {
                    self.ready.push(slot as u32);
                }
            }
            self.succ_buf = buf;
        }

        /// Nodes never dispatched. Non-empty at the end means the edges are wrong; report it rather
        /// than exiting quietly with a partial resolution.
        pub fn undispatched(&self) -> Vec<Node> {
            (0..self.dispatched.len())
                .filter(|&slot| !self.dispatched[slot])
                .map(|slot| self.node_at(slot))
                .collect()
        }
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

    fn module(&self, s: i32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s])
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn min_degree(&self) -> i32 {
        0
    }

    fn has_computed_bidegree(&self, b: Bidegree) -> bool {
        self.differentials.len() > b.s() && self.differential(b.s()).next_degree() > b.t()
    }

    fn differential(&self, s: i32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s])
    }

    #[tracing::instrument(skip(self), fields(self = self.name, %max))]
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
                // This walks `t` then `s`, so registering each bidegree as it is computed is
                // already in order. Dropping the returned registration would leave the generators
                // unadded and every later bidegree reading a differential that is not there.
                if let Some(pending) = self.step_resolution(b) {
                    self.register(pending)
                        .unwrap_or_else(|e| panic!("Error registering bidegree {b}: {e}"));
                }
            }
        }
    }

    fn next_homological_degree(&self) -> i32 {
        self.modules.len()
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
            None,
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
                .signature_mask(&algebra, target, b.t(), &subalgebra.zero_signature(), None)
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
                mask.extend(subalgebra.signature_mask(&algebra, source, b.t(), &signature, None));
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
                    let masked_col = dx_matrix.row(i).first_nonzero().unwrap().0;
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

    fn chain_map(&self, s: i32) -> Arc<Self::ChainMap> {
        Arc::clone(&self.chain_maps[s])
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;
    // Pulled in here rather than at file scope: the resolution itself no longer calls a
    // `FreeChainComplex` method, so a top-level import would be unused in a non-test build.
    use crate::chain_complex::FreeChainComplex;

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

    /// An entry too wide for its field would shift into the next one, so it must not be packed
    /// and compared. Nothing has such a signature, so the mask is unsatisfiable.
    #[test]
    fn test_packed_signature_rejects_oversized_entry() {
        let subalgebra = MilnorSubalgebra::new(vec![1, 1]);

        assert!(subalgebra.packed_signature(&[0, 0]).is_some());
        assert!(
            subalgebra
                .packed_signature(&[PPart::max_entry(0), 0])
                .is_some()
        );
        assert!(
            subalgebra
                .packed_signature(&[PPart::max_entry(0) + 1, 0])
                .is_none()
        );
    }

    /// Cross-check the secondary (d2) computation on a *save-backed* Nassau resolution computed
    /// with the relaxed [`Resolution::compute_through_stem`] against the standard resolution. This
    /// exercises the quasi-inverse save files, which under the relaxed schedule are always written
    /// using the "incomplete information" (`Magic::Fix`) path, since a bidegree is computed while
    /// ignoring the same-degree generators of its target.
    #[test]
    fn test_stem_concurrent_secondary() {
        use std::sync::Arc;

        use algebra::pair_algebra::PairAlgebra;

        use crate::{
            chain_complex::FreeChainComplex, resolution::secondary::SecondaryResolution,
            secondary::SecondaryLift, utils::construct_standard,
        };

        /// Render every non-trivial d2 in `lift` as one line, for comparison across resolutions.
        fn d2_chart<CC>(lift: &SecondaryResolution<CC>) -> String
        where
            CC: FreeChainComplex,
            CC::Algebra: PairAlgebra,
        {
            let underlying = lift.underlying();
            let mut out = String::new();
            // Mirror the guarded iteration in `SecondaryResolution::e3_page`.
            for b in underlying.iter_stem() {
                if b.t() > 0 && underlying.has_computed_bidegree(b + Bidegree::n_s(-1, 2)) {
                    let matrix = lift.homotopy(b.s() + 2).homotopies.hom_k(b.t());
                    if matrix.iter().any(|row| !row.is_empty()) {
                        out.push_str(&format!("d2 {b}: {matrix:?}\n"));
                    }
                }
            }
            out
        }

        // Far enough to carry a nonzero d2 (the first is `d2(h4) = h0 h3^2`, out of stem 15) and
        // no further: this runs on every `cargo test`, and the assertion below fails loudly if the
        // range is ever trimmed past the last differential it is meant to compare.
        let max = Bidegree::n_s(16, 5);

        let dir = tempfile::TempDir::new().unwrap();
        let nassau = crate::utils::construct_nassau("S_2", Some(dir.path().to_owned())).unwrap();
        nassau.compute_through_stem(max);
        let nassau_lift = SecondaryResolution::new(Arc::new(nassau));
        nassau_lift.extend_all();

        let standard = construct_standard::<false, _, _>("S_2", None).unwrap();
        standard.compute_through_stem(max);
        let standard_lift = SecondaryResolution::new(Arc::new(standard));
        standard_lift.extend_all();

        let nassau_chart = d2_chart(&nassau_lift);
        // Both charts are built by the same guarded iteration, so an empty pair would compare
        // equal without having compared any d2 at all.
        assert!(
            !nassau_chart.is_empty(),
            "no d2 differentials were compared"
        );

        assert_eq!(
            nassau_chart,
            d2_chart(&standard_lift),
            "secondary d2 chart differs between Nassau (save-backed, relaxed schedule) and \
             standard"
        );
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

    #[test]
    fn test_subalgebra_fmt() {
        let f2 = MilnorSubalgebra::zero_algebra();
        let a2 = MilnorSubalgebra::new(vec![3, 2, 1]);
        let b3321 = MilnorSubalgebra::new(vec![3, 3, 2, 1]);

        assert_eq!(f2.to_string(), "F_2");
        assert_eq!(a2.to_string(), "A(2)");
        assert_eq!(b3321.to_string(), "B(3,3,2,1)");
    }
}
