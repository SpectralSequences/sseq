//! This module implements [Nassau's algorithm](https://arxiv.org/abs/1910.04063).
//!
//! The main export is the [`Resolution`] object, which resolves a bounded chain complex over a
//! [`MilnorAlgebra`] using Nassau's algorithm. It aims to provide an API similar to
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
    io,
    sync::{Arc, Mutex, mpsc},
};

use algebra::{
    Algebra, combinatorics,
    milnor_algebra::{
        MilnorAlgebra, MilnorBasisElement, MilnorProfile, MilnorSubalgebra, PPart, PPartEntry,
    },
    module::{
        FreeModule, GeneratorData, Module, ZeroModule,
        homomorphism::{FreeModuleHomomorphism, FullModuleHomomorphism, ModuleHomomorphism},
    },
};
use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fp::{
    matrix::{AugmentedMatrix, Matrix, Subspace},
    prime::{Prime, ValidPrime, iter::BitflagIterator},
    vector::{FpSlice, FpSliceMut, FpVector},
};
use itertools::{Either, Itertools};
use once::OnceBiVec;
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use crate::{
    chain_complex::{
        AugmentedChainComplex, BoundedChainComplex, ChainComplex, FiniteChainComplex,
        FreeChainComplex,
    },
    save::{SaveDirectory, SaveKind},
    utils::{LogWriter, parallel::ParallelGuard},
};

/// See [`resolution::SenderData`](../resolution/struct.SenderData.html). This differs by not having the `new` field.
struct SenderData {
    b: Bidegree,
    retry: bool,
    sender: mpsc::Sender<Self>,
}

impl SenderData {
    pub(crate) fn send(b: Bidegree, sender: mpsc::Sender<Self>) {
        sender
            .send(Self {
                b,
                retry: false,
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
                sender: sender.clone(),
            })
            .unwrap()
    }
}

const MAX_NEW_GENS: usize = 10;

/// Candidate subalgebras whose top degree exceeds this are never applicable in practice.
const MAX_TOP_DEGREE: i32 = 10_000;

/// The parts of Nassau's algorithm that depend only on the subalgebra `B`.
trait NassauSubalgebra: Sized {
    /// The candidates for the subalgebra `B` of [Nassau's
    /// algorithm](https://arxiv.org/abs/1910.04063) that lie in `ambient`, in increasing order of
    /// size, starting with the trivial subalgebra.
    ///
    /// At the polynomial shape at `p = 2` these are the profiles of [`SubalgebraIterator`]. At the
    /// exterior shape they are the `A(n)` and the $E(Q_0, \dots, Q_n)$, which share the slope of
    /// $A(n)$ but apply slightly sooner. At the polynomial shape at odd primes there are none
    /// besides the trivial one, since we know no vanishing line there.
    fn candidates(ambient: &MilnorAlgebra) -> Vec<Self>;

    /// The slope of the vanishing line of `Ext_B`: `Ext_B^{s, t}` vanishes for
    /// `t >= slope * (s + 1) + top_degree` (Theorem 3.1).
    ///
    /// This is the slope of the steepest polynomial generator of the May spectral sequence: `v_k`
    /// for each `Q_k`, and for each `ξ_i^{p^j}` either `h_{i, j}` at `p = 2` or `b_{i, j}` at odd
    /// primes. The exterior `h_{i, j}` at odd primes are bounded by the top degree.
    fn vanishing_slope(&self) -> i64;

    /// Whether `b` is in the vanishing region of `self`, where the algorithm applies.
    fn is_applicable(&self, b: Bidegree) -> bool;

    /// Give a list of basis elements in degree `degree` that has signature `signature`, i.e. whose
    /// component in `self` is `signature`.
    ///
    /// Only basis elements coming from generators of degree strictly less than `max_gen_degree` are
    /// considered; `None` imposes no restriction. Because generators are laid out in increasing
    /// degree, a restricted result is a prefix of the unrestricted one; see
    /// [`Resolution::step_resolution_with_subalgebra`] for why we restrict.
    fn signature_mask<'a>(
        &'a self,
        algebra: &'a MilnorAlgebra,
        module: &'a FreeModule<MilnorAlgebra>,
        degree: i32,
        signature: &'a MilnorBasisElement,
        max_gen_degree: Option<i32>,
    ) -> impl Iterator<Item = usize> + 'a;

    /// Get the matrix of a free module homomorphism when restricted to the subquotient given by the
    /// signature.
    ///
    /// Only generators of the target of degree strictly less than `target_max_gen_degree` are used
    /// (see [`NassauSubalgebra::signature_mask`]).
    fn signature_matrix(
        &self,
        hom: &FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>,
        degree: i32,
        signature: &MilnorBasisElement,
        target_max_gen_degree: i32,
    ) -> Matrix;

    /// The nonzero signatures of `self` with a representative in degree at most `degree`, which are
    /// its basis elements, in degree order.
    ///
    /// Products raise the signature of their right factor, and the degree order is a linear
    /// extension of that order (Lemma 2.4).
    fn signatures(&self, degree: i32) -> impl Iterator<Item = MilnorBasisElement> + '_;

    /// Write the profile of `self`, which is all [`NassauSubalgebra::from_bytes`] needs to
    /// recover it.
    ///
    /// The q-part is only written at the exterior shape, so that the format at the polynomial shape
    /// does not depend on it.
    fn to_bytes(&self, buffer: &mut impl io::Write) -> io::Result<()>;

    /// Read a subalgebra of `ambient` written by [`NassauSubalgebra::to_bytes`].
    fn from_bytes(ambient: &MilnorAlgebra, data: &mut impl io::Read) -> io::Result<Self>;

    /// Write a signature of `self`: its p-part, one `u16` per entry of the profile, then at the
    /// exterior shape its q-part.
    fn signature_to_bytes(
        &self,
        signature: &MilnorBasisElement,
        buffer: &mut impl io::Write,
    ) -> io::Result<()>;

    /// Read a signature of `self` written by [`NassauSubalgebra::signature_to_bytes`].
    fn signature_from_bytes(&self, data: &mut impl io::Read) -> io::Result<MilnorBasisElement>;
}

impl NassauSubalgebra for MilnorSubalgebra {
    fn candidates(ambient: &MilnorAlgebra) -> Vec<Self> {
        let mut profiles = Vec::new();
        if ambient.has_exterior() {
            for n in 0..combinatorics::MAX_TAU as u32 {
                let q_part = u32::MAX >> (u32::BITS - 1 - n);
                profiles.push((vec![], q_part));
                if n as usize <= combinatorics::MAX_XI {
                    profiles.push(((1..=n).rev().collect(), q_part));
                }
            }
        } else if ambient.prime() == 2 {
            profiles.extend(
                SubalgebraIterator::new()
                    .take_while(|profile| profile.len() <= combinatorics::MAX_XI)
                    .map(|profile| (profile.into_iter().map(PPartEntry::from).collect(), 0)),
            );
        }

        let profile = |(p_part, q_part)| MilnorProfile {
            truncated: true,
            q_part,
            p_part,
        };
        std::iter::once(Self::trivial(ambient))
            .chain(
                profiles
                    .into_iter()
                    .map_while(|b| Self::new(ambient, profile(b)))
                    .take_while(|b| b.top_degree() <= MAX_TOP_DEGREE)
                    .filter(|b| b.is_subalgebra_of(ambient)),
            )
            .collect()
    }

    fn vanishing_slope(&self) -> i64 {
        let algebra = self.algebra();
        let p = algebra.prime().as_i32() as i64;
        let q = algebra.q() as i64;
        let profile = self.profile();
        let tau_degrees = combinatorics::tau_degrees(algebra.prime());
        let xi_degrees = combinatorics::xi_degrees(algebra.prime());

        let exterior = if algebra.has_exterior() {
            profile.q_part
        } else {
            0
        };
        let polynomial = profile.p_part.iter().enumerate().flat_map(|(i, &e)| {
            (0..e).map(move |j| {
                let h = q * xi_degrees[i] as i64 * p.pow(j);
                if p == 2 { h } else { h * p / 2 }
            })
        });
        BitflagIterator::set_bit_iterator(exterior as u64)
            .map(|k| tau_degrees[k] as i64)
            .chain(polynomial)
            .max()
            .unwrap_or(0)
    }

    fn is_applicable(&self, b: Bidegree) -> bool {
        b.t() as i64 >= self.vanishing_slope() * (b.s() as i64 + 1) + self.top_degree() as i64
    }

    fn signature_mask<'a>(
        &'a self,
        algebra: &'a MilnorAlgebra,
        module: &'a FreeModule<MilnorAlgebra>,
        degree: i32,
        signature: &'a MilnorBasisElement,
        max_gen_degree: Option<i32>,
    ) -> impl Iterator<Item = usize> + 'a {
        let gens = module
            .iter_gen_offsets([degree])
            .take_while(move |gen_data| max_gen_degree.is_none_or(|bound| gen_data.gen_deg < bound))
            .map(
                move |GeneratorData {
                          gen_deg,
                          start: [offset],
                          ..
                      }| (degree - gen_deg, offset),
            );

        if let Some(mask) = self.packed_component_mask() {
            // At this shape the p-part table is the basis.
            let value = signature.p_part.bits();
            Either::Left(gens.flat_map(move |(op_deg, offset)| {
                algebra
                    .ppart_table(op_deg)
                    .iter()
                    .enumerate()
                    .filter_map(move |(n, op)| (op.bits() & mask == value).then_some(offset + n))
            }))
        } else {
            Either::Right(gens.flat_map(move |(op_deg, offset)| {
                (0..algebra.dimension(op_deg))
                    .filter(move |&n| {
                        self.has_component(&algebra.basis_element_from_index(op_deg, n), signature)
                    })
                    .map(move |n| offset + n)
            }))
        }
    }

    fn signature_matrix(
        &self,
        hom: &FreeModuleHomomorphism<FreeModule<MilnorAlgebra>>,
        degree: i32,
        signature: &MilnorBasisElement,
        target_max_gen_degree: i32,
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
            .signature_mask(&algebra, &source, degree, signature, None)
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

    fn signatures(&self, degree: i32) -> impl Iterator<Item = MilnorBasisElement> + '_ {
        let algebra = self.algebra();
        algebra.compute_basis(std::cmp::min(degree, self.top_degree()));
        (1..=std::cmp::min(degree, self.top_degree())).flat_map(move |t| {
            (0..algebra.dimension(t)).map(move |idx| algebra.basis_element_from_index(t, idx))
        })
    }

    fn to_bytes(&self, buffer: &mut impl io::Write) -> io::Result<()> {
        let profile = self.profile();
        let len = profile.p_part.len();
        buffer.write_u64::<LittleEndian>(len as u64)?;
        for &entry in &profile.p_part {
            buffer.write_u8(entry as u8)?;
        }

        let zeros = [0; 8];
        let padding = len - ((len / 8) * 8);
        buffer.write_all(&zeros[0..padding])?;

        if self.algebra().has_exterior() {
            buffer.write_u64::<LittleEndian>(profile.q_part as u64)?;
        }
        Ok(())
    }

    fn from_bytes(ambient: &MilnorAlgebra, data: &mut impl io::Read) -> io::Result<Self> {
        // The packed p-part has no entry past `PPart::MAX_LEN`, so a longer profile cannot be
        // matched against one. This is the only place a profile is built from outside data, and the
        // bound has to hold before narrowing, which truncates where `usize` is 32 bits.
        let len = data.read_u64::<LittleEndian>()?;
        if len > PPart::MAX_LEN as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("profile length {len} exceeds {}", PPart::MAX_LEN),
            ));
        }
        let len = len as usize;
        let mut p_part = vec![0; len];

        data.read_exact(&mut p_part)?;

        let padding = len - ((len / 8) * 8);
        if padding > 0 {
            let mut buf: [u8; 8] = [0; 8];
            data.read_exact(&mut buf[0..padding])?;
            assert_eq!(buf, [0; 8]);
        }

        let q_part = if ambient.has_exterior() {
            data.read_u64::<LittleEndian>()? as u32
        } else {
            0
        };
        let profile = MilnorProfile {
            truncated: true,
            q_part,
            p_part: p_part.into_iter().map(PPartEntry::from).collect(),
        };
        Self::new(ambient, profile)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "not a finite subalgebra"))
    }

    fn signature_to_bytes(
        &self,
        signature: &MilnorBasisElement,
        buffer: &mut impl io::Write,
    ) -> io::Result<()> {
        let len = self.profile().p_part.len();
        for i in 0..len {
            buffer.write_u16::<LittleEndian>(signature.p_part.get(i) as u16)?;
        }

        let zeros = [0; 8];
        let padding = len - ((len / 4) * 4);
        if padding > 0 {
            buffer.write_all(&zeros[0..padding * 2])?;
        }

        if self.algebra().has_exterior() {
            buffer.write_u64::<LittleEndian>(signature.q_part as u64)?;
        }
        Ok(())
    }

    fn signature_from_bytes(&self, data: &mut impl io::Read) -> io::Result<MilnorBasisElement> {
        let len = self.profile().p_part.len();
        let mut p_part = vec![0; len];
        for entry in &mut p_part {
            *entry = data.read_u16::<LittleEndian>()? as PPartEntry;
        }

        let padding = len - ((len / 4) * 4);
        if padding > 0 {
            let mut buffer: [u8; 8] = [0; 8];
            data.read_exact(&mut buffer[0..padding * 2])?;
            assert_eq!(buffer, [0; 8]);
        }

        let q_part = if self.algebra().has_exterior() {
            data.read_u64::<LittleEndian>()? as u32
        } else {
            0
        };
        let mut signature = MilnorBasisElement {
            q_part,
            p_part: PPart::try_from_slice(&p_part).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "signature entry too large")
            })?,
            degree: 0,
        };
        self.algebra().compute_degree(&mut signature);
        Ok(signature)
    }
}

/// The signature of the elements of `B` itself, which is the unit.
fn zero_signature() -> MilnorBasisElement {
    MilnorBasisElement::default()
}

/// An iterator through the p-part profiles of an increasing sequence of subalgebras at the
/// polynomial shape at `p = 2`, from `A(0)` up through each `A(n)`. See
/// [`NassauSubalgebra::candidates`].
struct SubalgebraIterator {
    current: Vec<u8>,
}

impl SubalgebraIterator {
    fn new() -> Self {
        Self { current: vec![] }
    }
}

impl Iterator for SubalgebraIterator {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_empty() || self.current[0] == self.current.len() as u8 {
            // We are at F_2 or at A(n) where n = self.current.len() - 1.
            self.current.push(1);
        } else if let Some((_, entry)) = self
            .current
            .iter_mut()
            .rev()
            .enumerate()
            .find(|(idx, entry)| **entry == *idx as u8)
        {
            // We find the first entry that can be incremented and increment it
            *entry += 1;
        }
        Some(self.current.clone())
    }
}

/// Some magic constants used in the save file
enum Magic {
    End = -1,
    Signature = -2,
    Fix = -3,
}

/// A resolution of a bounded finite chain complex over a [`MilnorAlgebra`] using Nassau's
/// algorithm.
///
/// This aims to have an API similar to that of
/// [`resolution::Resolution`](crate::resolution::Resolution). From an API point of view, the main
/// difference between the two is that this is a chain complex over [`MilnorAlgebra`] over
/// [`SteenrodAlgebra`](algebra::SteenrodAlgebra).
///
/// The algebra can have either [shape](algebra::milnor_algebra::MilnorShape), any prime and any
/// profile. The exterior shape at `p = 2` is $A^{\mathbb{C}}/\tau$, and at odd primes the
/// classical algebra. Both need the `odd-primes` feature, without which basis elements compare by
/// their p-part alone.
pub struct Resolution<M: ZeroModule<Algebra = MilnorAlgebra>> {
    lock: Mutex<()>,
    name: String,
    /// The top degree of the target. A subalgebra applies to the target above its own vanishing
    /// line shifted by this.
    max_degree: i32,
    /// See [`NassauSubalgebra::candidates`].
    subalgebras: Vec<MilnorSubalgebra>,
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
        Self::new_with_complex(Arc::new(FiniteChainComplex::ccdz(module)), save_dir)
    }

    /// A resolution of the chain complex `target`.
    ///
    /// Save files hold the differentials alone, so bidegrees where the target is nonzero are
    /// recomputed rather than loaded.
    pub fn new_with_complex(
        target: Arc<FiniteChainComplex<M>>,
        save_dir: impl Into<SaveDirectory>,
    ) -> anyhow::Result<Self> {
        let save_dir = save_dir.into();
        let max_degree = (0..target.max_s())
            .map(|s| target.module(s).max_degree())
            .try_fold(0, |acc, d| Some(std::cmp::max(acc, d?)))
            .ok_or_else(|| anyhow!("Nassau's algorithm requires a bounded target"))?;

        if let Some(p) = save_dir.write() {
            for subdir in SaveKind::nassau_data() {
                subdir.create_dir(p)?;
            }
        }

        Ok(Self {
            lock: Mutex::new(()),
            zero_module: Arc::new(FreeModule::new(target.algebra(), "F_{-1}".to_string(), 0)),
            subalgebras: MilnorSubalgebra::candidates(&target.algebra()),
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
        subalgebra: &MilnorSubalgebra,
        signature: &MilnorBasisElement,
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
        if signature.q_part != 0 || !signature.p_part.is_empty() {
            f.write_u64::<LittleEndian>(Magic::Signature as u64)?;
            subalgebra.signature_to_bytes(signature, f)?;
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
            for (i, c) in preimage.iter_nonzero() {
                scratch.as_slice_mut().add(full_matrix.row(i), c);
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
        subalgebra: &MilnorSubalgebra,
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

        let zero_sig = zero_signature();
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
            subalgebra,
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
        let mut n =
            subalgebra.signature_matrix(&self.differentials[b.s()], b.t(), &zero_sig, target_bound);
        n.row_reduce();
        let next_row = n.rows();

        let num_new_gens = n.extend_image(0, n.columns(), &kernel, 0).len();

        if b.t() < b.s() {
            assert_eq!(num_new_gens, 0, "Adding generators at {b}");
        }

        self.add_generators(b, num_new_gens);

        let mut xs = vec![FpVector::new(p, target_dim); num_new_gens];
        let mut dxs = vec![FpVector::new(p, next_dim); num_new_gens];

        for ((x, x_masked), dx) in xs
            .iter_mut()
            .zip_eq(n.iter().skip(next_row))
            .zip_eq(&mut dxs)
        {
            x.as_slice_mut().add_unmasked(x_masked, 1, &target_mask);
            for (i, c) in x_masked.iter_nonzero() {
                dx.as_slice_mut().add(full_matrix.row(i), c);
            }
        }

        // Now add correction terms
        let mut target_mask: Vec<usize> = Vec::new();
        let mut next_mask: Vec<usize> = Vec::new();

        drop(guard);

        for signature in subalgebra.signatures(b.t()) {
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
                // Subtract a preimage of this signature's component of `dx`.
                scratch.set_scratch_vector_size(target_mask.len());
                let mut row = 0;
                for (i, &v) in next_mask.iter().enumerate() {
                    if pivots[i] < 0 {
                        continue;
                    }
                    let c = dx.entry(v);
                    if c != 0 {
                        scratch.as_slice_mut().add(preimage.row(row), p - c);
                    }
                    row += 1;
                }
                for (i, c) in scratch.iter_nonzero() {
                    x.add_basis_element(target_mask[i], c);
                    dx.as_slice_mut().add(full_matrix.row(i), c);
                }
            }
            Self::write_qi(
                &mut f,
                &mut scratch,
                subalgebra,
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

    /// Whether `b` needs an ordinary step, which takes the target into account.
    ///
    /// The target can only matter where `C_s` or `C_{s - 1}` is nonzero, and `C_s` vanishes from
    /// `max_s` on. Elsewhere the kernel to hit is that of the differential alone, which Nassau's
    /// algorithm computes.
    fn is_ordinary(&self, b: Bidegree) -> bool {
        b.s() <= 1 || (b.s() <= self.target.max_s() && b.t() <= self.max_degree)
    }

    /// The ordinary minimal resolution step, as in [`crate::resolution::Resolution`], except
    /// that the kernel of `(s - 1, t)` is recomputed rather than stored.
    fn step_ordinary(&self, b: Bidegree) -> anyhow::Result<()> {
        let p = self.prime();
        let t = b.t();
        if b.s() == 0 {
            self.zero_module.extend_by_zero(t);
        }
        self.target.compute_through_bidegree(b);

        let source = &self.modules[b.s()];
        let target_cc = self.target.module(b.s());
        let differential = &self.differentials[b.s()];
        let chain_map = &self.chain_maps[b.s()];
        let target_res = differential.target();

        target_cc.compute_basis(t);
        target_res.compute_basis(t);
        let source_dim = source.dimension(t);
        let target_cc_dim = target_cc.dimension(t);
        let target_res_dim = target_res.dimension(t);

        let mut matrix = AugmentedMatrix::<3>::new_with_capacity(
            p,
            source_dim,
            &[target_cc_dim, target_res_dim, source_dim],
            source_dim + MAX_NEW_GENS,
            0,
        );
        {
            let _guard = ParallelGuard::new();
            chain_map.get_matrix(matrix.segment(0, 0), t);
            differential.get_matrix(matrix.segment(1, 1), t);
        }
        matrix.segment(2, 2).add_identity();
        matrix.row_reduce();

        let cc_new_gens = matrix.extend_to_surjection(0, target_cc_dim, 0);
        let mut num_new_gens = cc_new_gens.len();

        if b.s() > 0 {
            // Make the new generators a chain map: d(x) = f⁻¹(d(f(x))). This only reads the
            // quasi-inverse of the previous chain map where its target is nonzero, which is where
            // it is kept.
            let complex_differential = (!cc_new_gens.is_empty())
                .then(|| self.target.differential(b.s()))
                .filter(|d| d.target().dimension(t) > 0);
            if let Some(complex_differential) = complex_differential {
                let quasi_inverse = self.chain_maps[b.s() - 1].quasi_inverse(t).unwrap();
                let mut dfx = FpVector::new(p, complex_differential.target().dimension(t));
                for (i, &column) in cc_new_gens.iter().enumerate() {
                    dfx.set_to_zero();
                    complex_differential.apply_to_basis_element(dfx.as_slice_mut(), 1, t, column);
                    quasi_inverse.apply(
                        matrix.row_segment_mut(source_dim + i, 1, 1),
                        1,
                        dfx.as_slice(),
                    );
                }
            }

            let desired_image = self.kernel(b - Bidegree::s_t(1, 0));
            num_new_gens += matrix
                .inner
                .extend_image(matrix.start[1], matrix.end[1], &desired_image, 0)
                .len();
        }

        self.add_generators(b, num_new_gens);
        let new_rows = source_dim..source_dim + num_new_gens;
        chain_map.add_generators_from_matrix_rows(
            t,
            matrix.segment(0, 0).row_slice(new_rows.start, new_rows.end),
        );
        differential.add_generators_from_matrix_rows(
            t,
            matrix.segment(1, 1).row_slice(new_rows.start, new_rows.end),
        );

        // The chain map's auxiliary data is kept where its target is nonzero, for the step above
        // and for lifting maps of resolutions, and always at `s = 0` as the augmentation.
        if b.s() == 0 || target_cc_dim > 0 {
            chain_map.compute_auxiliary_data_through_degree(t);
        } else {
            chain_map.set_kernel(t, None);
            chain_map.set_image(t, None);
            chain_map.set_quasi_inverse(t, None);
        }
        differential.set_kernel(t, None);
        differential.set_image(t, None);
        differential.set_quasi_inverse(t, None);

        if b.s() > 0 && target_cc_dim == 0 {
            self.write_differential(b, num_new_gens, target_res_dim)?;
        }
        Ok(())
    }

    /// The kernel of `(d, f): F_{s, t} → F_{s - 1, t} ⊕ C_{s, t}`.
    fn kernel(&self, b: Bidegree) -> Subspace {
        let t = b.t();
        let source = &self.modules[b.s()];
        // At the stem boundary, `b` itself may have been skipped. Its degree `t` generators map to
        // nonzero elements, so they are not in the kernel the step above has to hit.
        source.compute_basis(t);
        if b.s() == 0 {
            self.zero_module.extend_by_zero(t);
        } else {
            self.modules[b.s() - 1].compute_basis(t);
        }
        self.target.compute_through_bidegree(b);

        let mut matrix = AugmentedMatrix::<3>::new(
            self.prime(),
            source.dimension(t),
            [
                self.target.module(b.s()).dimension(t),
                self.differentials[b.s()].target().dimension(t),
                source.dimension(t),
            ],
        );
        {
            let _guard = ParallelGuard::new();
            self.chain_maps[b.s()].get_matrix(matrix.segment(0, 0), t);
            self.differentials[b.s()].get_matrix(matrix.segment(1, 1), t);
        }
        matrix.segment(2, 2).add_identity();
        matrix.row_reduce();
        matrix.compute_kernel()
    }

    /// The largest candidate subalgebra that applies at `b`, which is not ordinary.
    fn subalgebra_for(&self, b: Bidegree) -> &MilnorSubalgebra {
        let shifted = b - Bidegree::s_t(0, self.max_degree);
        self.subalgebras
            .iter()
            .filter(|subalgebra| subalgebra.is_applicable(shifted))
            .max_by_key(|subalgebra| subalgebra.dimension())
            .unwrap_or(&self.subalgebras[0])
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
        self.target.compute_through_bidegree(b);

        // A save file holds only the differential, which determines the step when the chain map
        // is zero.
        if b.s() > 0
            && self.target.module(b.s()).dimension(b.t()) == 0
            && let Some(dir) = self.save_dir.read()
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

            self.add_generators(b, num_new_gens);

            let mut d_targets = Vec::with_capacity(num_new_gens);

            for _ in 0..num_new_gens {
                d_targets.push(FpVector::from_bytes(p, saved_target_res_dimension, &mut f)?);
            }

            self.differentials[b.s()].add_generators_from_rows(b.t(), d_targets);
            self.chain_maps[b.s()].extend_by_zero(b.t());

            set_data();

            return Ok(());
        }

        if self.is_ordinary(b) {
            return self.step_ordinary(b);
        }

        self.step_resolution_with_subalgebra(b, self.subalgebra_for(b))?;
        self.chain_maps[b.s()].extend_by_zero(b.t());

        set_data();
        Ok(())
    }

    fn step_resolution(&self, b: Bidegree) {
        self.step_resolution_with_result(b)
            .unwrap_or_else(|e| panic!("Error computing bidegree {b}: {e}"));
    }

    /// This function resolves up till a fixed stem instead of a fixed t.
    #[tracing::instrument(skip(self), fields(self = self.name, %max))]
    pub fn compute_through_stem(&self, max: Bidegree) {
        let _lock = self.lock.lock();

        self.extend_through_degree(max.s());
        self.algebra().compute_basis(max.t());

        let min_degree = self.min_degree();
        let max_s = max.s();
        let max_n = max.n();

        let in_region = |s: i32, t: i32| -> bool {
            (0..=max_s).contains(&s) && t >= min_degree && t - s <= max_n
        };

        let is_ordinary = |s: i32, t: i32| self.is_ordinary(Bidegree::s_t(s, t));

        // `(s, t)` may be computed once its same-row predecessor `(s, t - 1)` and its diagonal
        // predecessor are committed. The diagonal predecessor of an ordinary step is `(s - 1, t)`,
        // whose kernel it hits, and otherwise `(s - 1, t - 1)` (the relaxed graph); `s == 0` has
        // none. `progress[s]` is the largest committed `t` in row `s`, so it doubles as a
        // "predecessor committed" test.
        let ready = |s: i32, t: i32, progress: &[i32]| -> bool {
            in_region(s, t)
                && progress[s as usize] >= t - 1
                && match s {
                    0 => true,
                    // At the stem edge `(s - 1, t)` lies outside the computed region, so we treat
                    // it as satisfied.
                    _ if is_ordinary(s, t) => {
                        t - (s - 1) > max_n || progress[(s - 1) as usize] >= t
                    }
                    _ => progress[(s - 1) as usize] >= t - 1,
                }
        };

        let tracing_span = tracing::Span::current();
        maybe_rayon::in_place_scope(|scope| {
            let _tracing_guard = tracing_span.enter();

            let mut progress: Vec<i32> = vec![min_degree - 1; max_s as usize + 1];

            let (sender, receiver) = mpsc::channel();

            let spawn_bidegree = |b: Bidegree, sender: mpsc::Sender<SenderData>| {
                if self.has_computed_bidegree(b) {
                    SenderData::send(b, sender);
                } else {
                    let tracing_span = tracing_span.clone();
                    scope.spawn(move |_| {
                        let _tracing_guard = tracing_span.enter();
                        if crate::utils::parallel::is_in_parallel() {
                            SenderData::send_retry(b, sender);
                            return;
                        }
                        self.step_resolution(b);
                        SenderData::send(b, sender);
                    });
                }
            };

            // Seed the base of every row that has no in-region predecessor. An ordinary
            // `(s, min_degree)` with `s > 0` has `(s - 1, min_degree)`, so it is spawned instead.
            for s in 0..=max_s {
                if s == 0 || !is_ordinary(s, min_degree) {
                    spawn_bidegree(Bidegree::s_t(s, min_degree), sender.clone());
                }
            }
            drop(sender);

            while let Ok(SenderData { b, retry, sender }) = receiver.recv() {
                if retry {
                    spawn_bidegree(b, sender);
                    continue;
                }
                assert!(progress[b.s() as usize] == b.t() - 1);
                progress[b.s() as usize] = b.t();

                // Completing `b` can only make ready the bidegrees it is a predecessor of: its
                // same-row successor `(s, t + 1)`, and `(s + 1, t)` if that is ordinary or
                // `(s + 1, t + 1)` if that is not. `ready` requires *both* predecessors, so of the
                // two completions that could spawn a given bidegree, only the later one does.
                let same_row = b + Bidegree::s_t(0, 1);
                let above = b + Bidegree::s_t(1, 0);
                let diagonal = b + Bidegree::s_t(1, 1);
                let successors = [
                    Some(same_row),
                    is_ordinary(above.s(), above.t()).then_some(above),
                    (!is_ordinary(diagonal.s(), diagonal.t())).then_some(diagonal),
                ];

                for cand in successors.into_iter().flatten() {
                    if ready(cand.s(), cand.t(), &progress) {
                        spawn_bidegree(cand, sender.clone());
                    }
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
        self.zero_module.prime()
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
                self.step_resolution(b);
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
        let source = &self.modules[b.s()];
        let target = &self.modules[b.s() - 1];
        let algebra = target.algebra();
        let subalgebra = &MilnorSubalgebra::from_bytes(&algebra, &mut f).unwrap();

        let mut inputs: Vec<FpVector> = inputs.iter().map(|x| x.into().to_owned()).collect();
        let mut mask: Vec<usize> = Vec::with_capacity(zero_mask_dim + 8);
        mask.extend(subalgebra.signature_mask(&algebra, source, b.t(), &zero_signature(), None));

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
                .signature_mask(&algebra, target, b.t(), &zero_signature(), None)
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
                        let c = dx_matrix.row_segment(i, 1, 1).entry(k);
                        if c != 0 {
                            dx_matrix.row_segment_mut(i, 0, 0).add_basis_element(j, c);
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
                            output.into().add_unmasked(
                                dx_matrix.row_segment(i, 2, 2),
                                entry,
                                &mask,
                            );
                            input
                                .as_slice_mut()
                                .add(dx_matrix.row_segment(i, 1, 1), p - entry);
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
                        output
                            .into()
                            .add_unmasked(scratch0.as_slice(), entry, &mask);
                        // If we resume a resolve_through_stem, input may be longer than scratch1.
                        input
                            .slice_mut(0, scratch1.len())
                            .add(scratch1.as_slice(), p - entry);
                    }
                }

                // Row reduce the differentials
                if !target_zero_mask.is_empty() {
                    for i in 0..dx_matrix.rows() {
                        let entry = dx_matrix.row_segment(i, 1, 1).entry(col);
                        if entry != 0 {
                            dx_matrix
                                .row_segment_mut(i, 2, 2)
                                .slice_mut(0, zero_mask_dim)
                                .add(scratch0.as_slice(), p - entry);
                            dx_matrix
                                .row_segment_mut(i, 1, 1)
                                .slice_mut(0, target_dim)
                                .add(scratch1.as_slice(), p - entry);
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
    use algebra::module::FDModule;
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

        let max = Bidegree::n_s(20, 7);

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

    /// The trivial module over `algebra`, as a chain complex.
    fn sphere(algebra: MilnorAlgebra) -> Arc<FiniteChainComplex<FDModule<MilnorAlgebra>>> {
        let module = FDModule::new(
            Arc::new(algebra),
            "k".to_string(),
            bivec::BiVec::from_vec(0, vec![1]),
        );
        Arc::new(FiniteChainComplex::ccdz(Arc::new(module)))
    }

    /// The exterior shape at `p = 2`, which is $A^{\mathbb{C}}/\tau$.
    #[cfg(feature = "odd-primes")]
    fn c_tau() -> MilnorAlgebra {
        use algebra::milnor_algebra::{Exterior, MilnorAlgebraInner};

        MilnorAlgebraInner::<Exterior>::new(fp::prime::TWO, false).into()
    }

    /// Resolve `target` through `max` with this and the standard algorithm, and check that the
    /// ranks agree everywhere and that a nontrivial subalgebra was used somewhere.
    fn assert_matches_standard(
        target: Arc<FiniteChainComplex<FDModule<MilnorAlgebra>>>,
        save_dir: Option<std::path::PathBuf>,
        max: Bidegree,
    ) -> Resolution<FDModule<MilnorAlgebra>> {
        let standard = crate::resolution::Resolution::new(Arc::clone(&target));
        standard.compute_through_stem(max);

        let nassau = Resolution::new_with_complex(target, save_dir).unwrap();
        nassau.compute_through_stem(max);

        for b in standard.iter_stem() {
            assert_eq!(
                nassau.number_of_gens_in_bidegree(b),
                standard.number_of_gens_in_bidegree(b),
                "rank mismatch at {b}"
            );
        }
        assert!(
            nassau
                .iter_stem()
                .any(|b| !nassau.is_ordinary(b) && nassau.subalgebra_for(b).dimension() > 1),
            "no subalgebra was used"
        );
        nassau
    }

    /// The sphere at `p = 2`, through the concurrent scheduler.
    #[test]
    fn test_classical() {
        assert_matches_standard(
            sphere(MilnorAlgebra::new(fp::prime::TWO, false)),
            None,
            Bidegree::n_s(20, 10),
        );
    }

    /// Cη and Cν. The top cell of Cν is high enough that ignoring it picks subalgebras below their
    /// vanishing line.
    #[test]
    fn test_finite_modules() {
        for (top, action) in [(2, "Sq2 x0 = x2"), (4, "Sq4 x0 = x4")] {
            let spec = serde_json::json!({
                "type": "finite dimensional module",
                "p": 2,
                "gens": { "x0": 0, format!("x{top}"): top },
                "actions": [action],
            });
            let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
            let module = Arc::new(FDModule::from_json(algebra, &spec).unwrap());
            assert_matches_standard(
                Arc::new(FiniteChainComplex::ccdz(module)),
                None,
                Bidegree::n_s(18, 10),
            );
        }
    }

    /// The cofiber of h₀², a three-term chain complex.
    fn h0_squared_cofiber() -> Arc<FiniteChainComplex<FDModule<MilnorAlgebra>>> {
        use algebra::module::homomorphism::FreeModuleHomomorphism;

        use crate::{chain_complex::ChainMap, yoneda::yoneda_representative};

        let k = sphere(MilnorAlgebra::new(fp::prime::TWO, false));
        let class = BidegreeGenerator::s_t(2, 2, 0);
        let resolution = crate::resolution::Resolution::new(Arc::clone(&k));
        resolution.compute_through_stem(class.degree());

        let map = FreeModuleHomomorphism::new(resolution.module(class.s()), k.module(0), class.t());
        let mut matrix = Matrix::new(fp::prime::TWO, 1, 1);
        matrix.row_mut(0).set_entry(0, 1);
        map.add_generators_from_matrix_rows(class.t(), matrix.as_slice_mut());
        map.extend_by_zero(class.t());
        let yoneda = yoneda_representative(
            Arc::new(resolution),
            ChainMap {
                s_shift: class.s(),
                chain_maps: vec![map],
            },
        );
        let mut cofiber = FiniteChainComplex::from(yoneda);
        cofiber.pop();
        assert!(cofiber.max_s() >= 2);
        Arc::new(cofiber)
    }

    /// The cofiber of h₀², which needs ordinary steps beyond `s = 1`.
    #[test]
    fn test_yoneda_cofiber() {
        assert_matches_standard(h0_squared_cofiber(), None, Bidegree::n_s(16, 12));
    }

    /// tmf, which is the sphere over the finite ambient $A(2)$.
    #[test]
    fn test_tmf() {
        let a2 = MilnorAlgebra::new_with_profile(
            fp::prime::TWO,
            MilnorProfile {
                truncated: true,
                q_part: !0,
                p_part: vec![3, 2, 1],
            },
            false,
        );
        let res = assert_matches_standard(sphere(a2), None, Bidegree::n_s(30, 14));
        let gens = |n, s| res.number_of_gens_in_bidegree(Bidegree::n_s(n, s));
        assert_eq!(gens(3, 1), 1, "h_2");
        assert_eq!(gens(7, 1), 0, "no h_3 over A(2)");
    }

    /// $C\tau$, which is the sphere over $A^{\mathbb{C}}/\tau$.
    #[cfg(feature = "odd-primes")]
    #[test]
    fn test_c_tau() {
        assert_matches_standard(sphere(c_tau()), None, Bidegree::n_s(20, 12));
    }

    /// The sphere at odd primes, far enough out that `b_{1, 0}` sets the slope of $A(1)$ at 5.
    #[cfg(feature = "odd-primes")]
    #[test]
    fn test_odd_primes() {
        for (p, n) in [(3, 60), (5, 120)] {
            let algebra = MilnorAlgebra::new(ValidPrime::new(p), false);
            assert_matches_standard(sphere(algebra), None, Bidegree::n_s(n, 6));
        }
    }

    /// The saved quasi-inverses lift boundaries, and a second resolution loads the saved
    /// differentials, at every shape and for a chain complex. Lifting `d(x)` for the generators
    /// `x` exercises the `Magic::Fix` path.
    #[test]
    fn test_save_files() {
        #[cfg_attr(not(feature = "odd-primes"), expect(unused_mut))]
        let mut targets = vec![
            (
                sphere(MilnorAlgebra::new(fp::prime::TWO, false)),
                Bidegree::n_s(16, 8),
            ),
            (h0_squared_cofiber(), Bidegree::n_s(16, 8)),
        ];
        #[cfg(feature = "odd-primes")]
        targets.extend([
            (sphere(c_tau()), Bidegree::n_s(16, 8)),
            (
                sphere(MilnorAlgebra::new(ValidPrime::new(3), false)),
                Bidegree::n_s(40, 5),
            ),
        ]);

        for (target, max) in targets {
            let dir = tempfile::TempDir::new().unwrap();
            let res =
                assert_matches_standard(Arc::clone(&target), Some(dir.path().to_owned()), max);
            let p = res.prime();

            let mut lifted = 0;
            for b in res.iter_stem() {
                let next = b + Bidegree::s_t(1, 0);
                if b.s() == 0 || !res.has_computed_bidegree(next) || res.is_ordinary(next) {
                    continue;
                }
                let d = res.differential(b.s());
                let target_dim = res.module(b.s() - 1).dimension(b.t());
                let inputs: Vec<FpVector> = (0..res.number_of_gens_in_bidegree(b))
                    .map(|i| {
                        // Outputs only span the target generators that existed when computed.
                        let output = d.output(b.t(), i);
                        let mut v = FpVector::new(p, target_dim);
                        v.slice_mut(0, output.len()).add(output.as_slice(), 1);
                        v
                    })
                    .collect();
                let mut results =
                    vec![FpVector::new(p, res.module(b.s()).dimension(b.t())); inputs.len()];
                assert!(res.apply_quasi_inverse(&mut results, b, &inputs));
                for (input, result) in inputs.iter().zip(&results) {
                    let mut image = FpVector::new(p, target_dim);
                    d.apply(image.as_slice_mut(), 1, b.t(), result.as_slice());
                    assert_eq!(&image, input, "lift fails at {b}");
                }
                lifted += inputs.len();
            }
            assert!(lifted > 0, "nothing was lifted");

            let loaded = Resolution::new_with_complex(target, Some(dir.path().to_owned())).unwrap();
            loaded.compute_through_stem(max);
            for b in res.iter_stem() {
                assert_eq!(
                    loaded.number_of_gens_in_bidegree(b),
                    res.number_of_gens_in_bidegree(b),
                    "loaded rank mismatch at {b}"
                );
            }
        }
    }

    /// The signatures of a subalgebra are its basis elements, by degree.
    #[test]
    fn test_signatures() {
        let ambient = MilnorAlgebra::new(fp::prime::TWO, false);
        let profile = MilnorProfile {
            truncated: true,
            q_part: 0,
            p_part: vec![2, 1],
        };
        let subalgebra = MilnorSubalgebra::new(&ambient, profile).unwrap();
        let signatures: Vec<_> = subalgebra.signatures(6).collect();
        let mut p_parts: Vec<Vec<PPartEntry>> = signatures
            .iter()
            .map(|s| s.p_part.iter().collect())
            .collect();
        p_parts.sort();
        assert_eq!(
            p_parts,
            [
                vec![0, 1],
                vec![1],
                vec![1, 1],
                vec![2],
                vec![2, 1],
                vec![3],
                vec![3, 1]
            ]
        );
        assert!(signatures.is_sorted_by_key(|s| s.degree));
        assert_eq!(subalgebra.signatures(3).count(), 4);
        assert_eq!(subalgebra.signatures(0).count(), 0);
    }

    /// The candidates grow, lie in the ambient, and start with the trivial subalgebra.
    #[test]
    fn test_candidates() {
        let ambient = MilnorAlgebra::new(fp::prime::TWO, false);
        let candidates = MilnorSubalgebra::candidates(&ambient);
        assert_eq!(candidates[0].to_string(), "F_2");
        assert!(
            candidates
                .windows(2)
                .all(|w| w[0].dimension() < w[1].dimension())
        );

        let a2 = MilnorAlgebra::new_with_profile(
            fp::prime::TWO,
            MilnorProfile {
                truncated: true,
                q_part: !0,
                p_part: vec![3, 2, 1],
            },
            false,
        );
        let capped = MilnorSubalgebra::candidates(&a2);
        assert_eq!(capped.last().unwrap().to_string(), "A(2)");
        assert!(capped.iter().all(|b| b.is_subalgebra_of(&a2)));
    }
}
