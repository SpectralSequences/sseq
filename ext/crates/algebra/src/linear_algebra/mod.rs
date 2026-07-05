//! The linear algebra of *solving* over a graded DVR.
//!
//! This layer sits beside [`algebra`](crate::algebra) and [`module`](crate::module). The coefficient
//! *ring* — scalars, their arithmetic, and the representation of finite free modules over it
//! (vectors and slices) — is [`Ring`](crate::algebra::Ring); that is enough to define an algebra and
//! act on its modules. This module adds the harder capability a free *resolution* needs: computing
//! images, kernels, and quasi-inverses, which requires the ring to be a graded DVR (so that graded
//! Nakayama holds and minimal generators can be read off mod the maximal ideal).
//!
//! The central trait is [`Solvable`]: a [`Ring`](crate::algebra::Ring) over which one can solve
//! linear systems. For [`Field`] the vector types are exactly `fp`'s `FpVector`/`FpSlice`/`FpSliceMut`
//! and every operation forwards to `fp`, so the classical path is unchanged and bit-identical. A
//! future $\mathbb{F}_2[\tau]$ impl provides the graded-local solving (per-weight `fp` blocks plus a
//! $\tau$-divisibility sweep) behind the same interface.

use fp::{
    matrix::{AugmentedMatrix, QuasiInverse, Subspace},
    vector::{FpSlice, FpSliceMut, FpVector},
};
#[allow(unused_imports)]
use maybe_rayon::prelude::*;

use crate::algebra::{Algebra, BaseRingOf, Field, Ring};

/// An owned vector over the base ring of the algebra `A`.
pub type VectorOf<A> = <BaseRingOf<A> as Ring>::Vector;

/// An immutable slice of a vector over the base ring of the algebra `A`. For every classical algebra
/// this is `fp`'s `FpSlice`.
pub type BaseSliceOf<'a, A> = <BaseRingOf<A> as Ring>::Slice<'a>;

/// A mutable slice of a vector over the base ring of the algebra `A`. For every classical algebra
/// this is `fp`'s `FpSliceMut`.
pub type BaseSliceMutOf<'a, A> = <BaseRingOf<A> as Ring>::SliceMut<'a>;

/// A submodule (image or kernel) over the base ring of the algebra `A`. For every classical algebra
/// this is `fp`'s `Subspace`.
pub type SubmoduleOf<A> = <BaseRingOf<A> as Solvable>::Submodule;

/// A quasi-inverse of a map over the base ring of the algebra `A`. For every classical algebra this
/// is `fp`'s `QuasiInverse`.
pub type QuasiInverseOf<A> = <BaseRingOf<A> as Solvable>::QuasiInverse;

/// The data — beyond the step matrix itself — needed to construct the next stage of a resolution:
/// how to make the chain map a chain map, and which cycles the new generators must hit.
///
/// At homological degree 0 there is no previous stage, so both submodule fields are `None` and the
/// target dimension is 0.
pub struct NextStageInput<'a, R: Solvable> {
    /// The previous chain map's quasi-inverse, used to set `dX(x) = f^{-1}(dC(f(x)))`. `None` at
    /// homological degree 0, or when no new augmentation generators are added.
    pub previous_chain_map_quasi_inverse: Option<&'a R::QuasiInverse>,
    /// The previous kernel, whose cycles the new generators must hit. `None` at homological degree 0.
    pub previous_kernel: Option<&'a R::Submodule>,
    /// The dimension of `C_{s-1,t}`, the codomain of the target complex's differential (the length of
    /// the vector the `apply_complex_differential` callback writes into).
    pub complex_differential_target_dim: usize,
}

/// The generators produced by one resolution step: the new differential and chain-map rows to store,
/// plus the quasi-inverses of the two maps. Returned by
/// [`Solvable::construct_next_stage`].
pub struct NextStage<R: Solvable> {
    /// The number of new generators added (surjecting onto the cokernel plus hitting the old kernel).
    pub num_new_gens: usize,
    /// The chain map on each new generator (its image in `C_{s,t}`).
    pub chain_map_rows: Vec<R::Vector>,
    /// The differential on each new generator (its image in `X_{s-1,t}`).
    pub differential_rows: Vec<R::Vector>,
    /// A quasi-inverse of the chain map (the augmentation).
    pub chain_map_quasi_inverse: R::QuasiInverse,
    /// A quasi-inverse of the differential.
    pub differential_quasi_inverse: R::QuasiInverse,
}

/// An immutable slice of a vector over the ring `R`.
///
/// Slices are cheap `Copy` views (like `fp`'s `FpSlice`). This trait exposes the read-only vector
/// operations the module and algebra code performs.
pub trait BaseSlice<'a, R: Ring>: Copy {
    /// The number of entries in the slice.
    fn len(self) -> usize;

    /// Whether the slice is empty.
    fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The entry at `index`.
    fn entry(self, index: usize) -> R::Element;

    /// Whether every entry is zero.
    fn is_zero(self) -> bool;

    /// Restrict to the entries in `start..end`.
    fn restrict(self, start: usize, end: usize) -> Self;

    /// Iterate over `(index, coefficient)` for each nonzero entry.
    fn iter_nonzero(self) -> impl Iterator<Item = (usize, R::Element)> + 'a;
}

/// A mutable slice of a vector over the ring `R`.
///
/// This exposes the in-place vector operations (axpy, setting entries, zeroing) the module and
/// algebra code performs while assembling a result.
pub trait BaseSliceMut<'a, R: Ring> {
    /// Reborrow as a shorter-lived mutable slice (so the original stays usable in a loop). This is
    /// the base-ring analogue of `FpSliceMut::copy`.
    fn reborrow(&mut self) -> R::SliceMut<'_>;

    /// Borrow as an immutable slice.
    fn as_slice(&self) -> R::Slice<'_>;

    /// Add `coeff` to the entry at `index`.
    fn add_basis_element(&mut self, index: usize, coeff: R::Element);

    /// Add `coeff * other` into this slice.
    fn add(&mut self, other: R::Slice<'_>, coeff: R::Element);

    /// Set the entry at `index` to `value`.
    fn set_entry(&mut self, index: usize, value: R::Element);

    /// Set every entry to zero.
    fn set_to_zero(&mut self);

    /// Restrict to the entries in `start..end`.
    fn slice_mut(&mut self, start: usize, end: usize) -> R::SliceMut<'_>;
}

/// The linear algebra of solving over a graded DVR `R`.
///
/// Extends [`Ring`](crate::algebra::Ring) — which already provides the ring's scalars and its vector
/// representation — with the operations a free resolution needs: computing the image, kernel, and
/// quasi-inverse of an `R`-linear map. These are meaningful precisely because `R` is a graded DVR
/// (graded-local, so graded Nakayama holds). For [`Field`] everything forwards to `fp`.
pub trait Solvable: Ring {
    /// A submodule of a finite free module over the ring (the image or kernel of a map). For
    /// [`Field`] this is `fp`'s [`Subspace`].
    type Submodule: Send + Sync;

    /// A quasi-inverse of an `R`-linear map: a right inverse when restricted to its image. For
    /// [`Field`] this is `fp`'s [`QuasiInverse`].
    type QuasiInverse: Send + Sync;

    /// Compute `(image, kernel, quasi_inverse)` of the `R`-linear map
    /// `R^source_dim -> R^target_dim` whose matrix is assembled by calling `fill_row(i, row)` for
    /// each source basis index `i`.
    ///
    /// This is the base-ring-generic form of
    /// [`ModuleHomomorphism::auxiliary_data`](crate::module::homomorphism::ModuleHomomorphism::auxiliary_data):
    /// the map is presented one row at a time, so the ring owns its matrix representation — a field
    /// uses a single dense `fp` matrix, while a graded-local ring can store per-weight blocks.
    fn image_kernel_quasi_inverse(
        self,
        source_dim: usize,
        target_dim: usize,
        fill_row: impl FnMut(usize, Self::SliceMut<'_>),
    ) -> (Self::Submodule, Self::Submodule, Self::QuasiInverse);

    /// The number of generators (rank) of a submodule.
    fn submodule_dimension(self, submodule: &Self::Submodule) -> usize;

    /// Iterate over a basis of the submodule.
    fn submodule_iter<'a>(
        self,
        submodule: &'a Self::Submodule,
    ) -> impl Iterator<Item = Self::Slice<'a>> + 'a;

    /// The dimension of the domain of the map the quasi-inverse inverts.
    fn quasi_inverse_source_dimension(self, quasi_inverse: &Self::QuasiInverse) -> usize;

    /// The dimension of the codomain of the map the quasi-inverse inverts.
    fn quasi_inverse_target_dimension(self, quasi_inverse: &Self::QuasiInverse) -> usize;

    /// Add `coeff * quasi_inverse(input)` to `result`.
    ///
    /// `input` must lie in the image of the map the quasi-inverse was computed for; the result is a
    /// preimage. This is the base-ring-generic form of
    /// [`ModuleHomomorphism::apply_quasi_inverse`](crate::module::homomorphism::ModuleHomomorphism::apply_quasi_inverse).
    fn apply_quasi_inverse(
        self,
        quasi_inverse: &Self::QuasiInverse,
        result: Self::SliceMut<'_>,
        coeff: Self::Element,
        input: Self::Slice<'_>,
    );

    /// The augmented matrix `[f | d | I]` of a resolution step, expressing the step's chain map `f`
    /// and differential `d` (out of the free module `X_{s,t}`) as a plain vector-space map over the
    /// ring, with an identity block for tracking preimages. For `Field` this is `fp`'s
    /// `AugmentedMatrix<3>`.
    type Matrix;

    /// **Block 2** — realize the differential as a vector-space map. Assemble and row-reduce the
    /// augmented matrix `[f | d | I]`: `fill_chain_map(i, row)` / `fill_differential(i, row)` write
    /// row `i` of the `f` / `d` blocks (via the module action), for `i in 0..source_dim`. The
    /// callbacks are `Sync` so the assembly can be parallelized.
    fn differential_matrix(
        self,
        source_dim: usize,
        chain_map_target_dim: usize,
        differential_target_dim: usize,
        max_new_gens: usize,
        fill_chain_map: impl Fn(usize, Self::SliceMut<'_>) + Sync,
        fill_differential: impl Fn(usize, Self::SliceMut<'_>) + Sync,
    ) -> Self::Matrix;

    /// **Block 3** — the kernel of the reduced step matrix (the cycles in `X_{s,t}`).
    fn step_kernel(self, matrix: &Self::Matrix) -> Self::Submodule;

    /// **Block 4** — construct the next stage: add generators to `X_{s,t}` that surject onto the
    /// cokernel of `f` and hit any cycles of the previous kernel not already in the image, compute
    /// their differential (making `f` a chain map, via `apply_complex_differential` and the previous
    /// chain map's quasi-inverse), and produce quasi-inverses of `f` and `d`.
    ///
    /// The source dimension and chain-map target dimension are read off the matrix; the rest of the
    /// data is bundled in [`NextStageInput`]. `apply_complex_differential` writes `dC(f(x))` for the
    /// new generator at column `x` into the provided slice. Consumes the matrix (its quasi-inverses
    /// take ownership).
    fn construct_next_stage(
        self,
        matrix: Self::Matrix,
        input: NextStageInput<'_, Self>,
        apply_complex_differential: impl FnMut(usize, Self::SliceMut<'_>),
        max_new_gens: usize,
    ) -> NextStage<Self>;
}

impl<'a> BaseSlice<'a, Field> for FpSlice<'a> {
    fn len(self) -> usize {
        FpSlice::len(&self)
    }

    fn entry(self, index: usize) -> u32 {
        FpSlice::entry(&self, index)
    }

    fn is_zero(self) -> bool {
        FpSlice::is_zero(&self)
    }

    fn restrict(self, start: usize, end: usize) -> Self {
        FpSlice::restrict(self, start, end)
    }

    fn iter_nonzero(self) -> impl Iterator<Item = (usize, u32)> + 'a {
        FpSlice::iter_nonzero(self)
    }
}

impl<'a> BaseSliceMut<'a, Field> for FpSliceMut<'a> {
    fn reborrow(&mut self) -> FpSliceMut<'_> {
        self.copy()
    }

    fn as_slice(&self) -> FpSlice<'_> {
        FpSliceMut::as_slice(self)
    }

    fn add_basis_element(&mut self, index: usize, coeff: u32) {
        FpSliceMut::add_basis_element(self, index, coeff);
    }

    fn add(&mut self, other: FpSlice<'_>, coeff: u32) {
        FpSliceMut::add(self, other, coeff);
    }

    fn set_entry(&mut self, index: usize, value: u32) {
        FpSliceMut::set_entry(self, index, value);
    }

    fn set_to_zero(&mut self) {
        FpSliceMut::set_to_zero(self);
    }

    fn slice_mut(&mut self, start: usize, end: usize) -> FpSliceMut<'_> {
        FpSliceMut::slice_mut(self, start, end)
    }
}

impl Solvable for Field {
    type Matrix = AugmentedMatrix<3>;
    type QuasiInverse = QuasiInverse;
    type Submodule = Subspace;

    fn image_kernel_quasi_inverse(
        self,
        source_dim: usize,
        target_dim: usize,
        mut fill_row: impl FnMut(usize, FpSliceMut<'_>),
    ) -> (Subspace, Subspace, QuasiInverse) {
        let p = Algebra::prime(&self);
        // The augmented matrix `[ M | I ]`: segment 0 holds the map, segment 1 tracks preimages so
        // that after row reduction we can read off the quasi-inverse. This mirrors
        // `ModuleHomomorphism::auxiliary_data` exactly.
        let mut matrix = AugmentedMatrix::<2>::new(p, source_dim, [target_dim, source_dim]);
        {
            let mut segment = matrix.segment(0, 0);
            for i in 0..source_dim {
                fill_row(i, segment.row_mut(i));
            }
        }
        matrix.segment(1, 1).add_identity();

        matrix.row_reduce();

        (
            matrix.compute_image(),
            matrix.compute_kernel(),
            matrix.compute_quasi_inverse(),
        )
    }

    fn submodule_dimension(self, submodule: &Subspace) -> usize {
        submodule.dimension()
    }

    fn submodule_iter<'a>(self, submodule: &'a Subspace) -> impl Iterator<Item = FpSlice<'a>> + 'a {
        submodule.iter()
    }

    fn quasi_inverse_source_dimension(self, quasi_inverse: &QuasiInverse) -> usize {
        quasi_inverse.source_dimension()
    }

    fn quasi_inverse_target_dimension(self, quasi_inverse: &QuasiInverse) -> usize {
        quasi_inverse.target_dimension()
    }

    fn apply_quasi_inverse(
        self,
        quasi_inverse: &QuasiInverse,
        result: FpSliceMut<'_>,
        coeff: u32,
        input: FpSlice<'_>,
    ) {
        quasi_inverse.apply(result, coeff, input);
    }

    fn differential_matrix(
        self,
        source_dim: usize,
        chain_map_target_dim: usize,
        differential_target_dim: usize,
        max_new_gens: usize,
        fill_chain_map: impl Fn(usize, FpSliceMut<'_>) + Sync,
        fill_differential: impl Fn(usize, FpSliceMut<'_>) + Sync,
    ) -> AugmentedMatrix<3> {
        let p = Algebra::prime(&self);
        let mut matrix = AugmentedMatrix::<3>::new_with_capacity(
            p,
            source_dim,
            &[chain_map_target_dim, differential_target_dim, source_dim],
            source_dim + max_new_gens,
            max_new_gens,
        );
        matrix
            .segment(0, 0)
            .maybe_par_iter_mut()
            .enumerate()
            .for_each(|(i, row)| fill_chain_map(i, row));
        matrix
            .segment(1, 1)
            .maybe_par_iter_mut()
            .enumerate()
            .for_each(|(i, row)| fill_differential(i, row));
        matrix.segment(2, 2).add_identity();
        matrix.row_reduce();
        matrix
    }

    fn step_kernel(self, matrix: &AugmentedMatrix<3>) -> Subspace {
        matrix.compute_kernel()
    }

    fn construct_next_stage(
        self,
        mut matrix: AugmentedMatrix<3>,
        input: NextStageInput<'_, Self>,
        mut apply_complex_differential: impl FnMut(usize, FpSliceMut<'_>),
        max_new_gens: usize,
    ) -> NextStage<Self> {
        let NextStageInput {
            previous_chain_map_quasi_inverse,
            previous_kernel,
            complex_differential_target_dim,
        } = input;

        let p = Algebra::prime(&self);
        // The source dimension and the chain-map (augmentation) target dimension are exactly the row
        // count and the first block's width of the freshly built matrix.
        let source_dimension = matrix.rows();
        let target_cc_dimension = matrix.end[0] - matrix.start[0];
        let differential_block_dim = matrix.end[1] - matrix.start[1];

        // Add generators to surject onto C_{s,t} (the cokernel of f).
        let cc_new_gens = matrix.extend_to_surjection(0, target_cc_dimension, max_new_gens);
        let mut res_new_gens = Vec::new();

        if let Some(previous_kernel) = previous_kernel {
            if !cc_new_gens.is_empty() {
                // Make f a chain map: set dX(x) = f^{-1}(dC(f(x))) for each new generator x.
                let quasi_inverse = previous_chain_map_quasi_inverse
                    .expect("chain homotopy requires the previous chain map's quasi-inverse");
                let mut dfx = FpVector::new(p, complex_differential_target_dim);
                for (i, &column) in cc_new_gens.iter().enumerate() {
                    apply_complex_differential(column, dfx.as_slice_mut());
                    quasi_inverse.apply(
                        matrix.row_segment_mut(source_dimension + i, 1, 1),
                        1,
                        dfx.as_slice(),
                    );
                    dfx.set_to_zero();
                }
            }

            // Add generators to hit any cycles of the old kernel not already in the image.
            res_new_gens = matrix.inner.extend_image(
                matrix.start[1],
                matrix.end[1],
                previous_kernel,
                max_new_gens,
            );
        }

        let num_new_gens = cc_new_gens.len() + res_new_gens.len();
        let new_rows = source_dimension + num_new_gens;

        // Read off the new generators' chain-map and differential rows before the RREF fix-up below
        // mutates them.
        let chain_map_rows: Vec<FpVector> = {
            let mut segment = matrix.segment(0, 0);
            (source_dimension..new_rows)
                .map(|r| {
                    let mut v = FpVector::new(p, target_cc_dimension);
                    v.as_slice_mut().add(segment.row(r), 1);
                    v
                })
                .collect()
        };
        let differential_rows: Vec<FpVector> = {
            let mut segment = matrix.segment(1, 1);
            (source_dimension..new_rows)
                .map(|r| {
                    let mut v = FpVector::new(p, differential_block_dim);
                    v.as_slice_mut().add(segment.row(r), 1);
                    v
                })
                .collect()
        };

        if num_new_gens > 0 {
            // Fix up the augmentation: the new rows are almost in RREF, so patch them up rather than
            // re-running the full reduction.
            let columns = matrix.columns();
            matrix.extend_column_dimension(columns + num_new_gens);

            for i in source_dimension..new_rows {
                matrix.inner.row_mut(i).set_entry(matrix.start[2] + i, 1);
            }

            // Clear the new cc rows using the old rows.
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

            // Use the new resolution rows to reduce the old rows and the cc rows.
            let first_res_row = source_dimension + cc_new_gens.len();
            for (source_row, &pivot_col) in res_new_gens.iter().enumerate() {
                for target_row in 0..first_res_row {
                    unsafe {
                        matrix.row_op(target_row, source_row + first_res_row, pivot_col, p);
                    }
                }
            }

            // Permute the rows into RREF.
            let mut new_gens = cc_new_gens.into_iter().chain(res_new_gens).enumerate();
            let (mut next_new_row, mut next_new_col) = new_gens.next().unwrap();
            let mut next_old_row = 0;

            for old_col in 0..matrix.columns() {
                if old_col == next_new_col {
                    matrix.rotate_down(next_old_row..source_dimension + next_new_row + 1, 1);
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

        let (chain_map_quasi_inverse, differential_quasi_inverse) = matrix.compute_quasi_inverses();

        NextStage {
            num_new_gens,
            chain_map_rows,
            differential_rows,
            chain_map_quasi_inverse,
            differential_quasi_inverse,
        }
    }
}

#[cfg(test)]
mod tests {
    use fp::prime::ValidPrime;

    use super::*;

    #[test]
    fn field_vector_ops() {
        let field = Field::new(ValidPrime::new(2));
        let mut v = field.new_vector(3);
        {
            let mut s = field.as_slice_mut(&mut v);
            s.add_basis_element(0, 1);
            s.add_basis_element(2, 1);
        }
        let s = field.as_slice(&v);
        assert_eq!(s.len(), 3);
        assert_eq!(s.entry(0), 1);
        assert_eq!(s.entry(1), 0);
        assert_eq!(s.entry(2), 1);
        assert!(!s.is_zero());
        let nz: Vec<_> = s.iter_nonzero().collect();
        assert_eq!(nz, vec![(0, 1), (2, 1)]);
    }

    #[test]
    fn field_kernel_quasi_inverse() {
        let field = Field::new(ValidPrime::new(2));
        // Rank-deficient map F_2^2 -> F_2^3 with two equal rows (1,1,0): kernel is 1-dimensional.
        let rows = [[1u32, 1, 0], [1, 1, 0]];
        let (image, kernel, _qi) = field.image_kernel_quasi_inverse(2, 3, |i, mut row| {
            for (j, &val) in rows[i].iter().enumerate() {
                if val != 0 {
                    row.add_basis_element(j, val);
                }
            }
        });
        assert_eq!(image.dimension(), 1);
        assert_eq!(kernel.dimension(), 1);
    }

    #[test]
    fn field_apply_quasi_inverse_is_section() {
        let field = Field::new(ValidPrime::new(2));
        // Full-rank map F_2^2 -> F_2^3, rows (1,1,0) and (0,1,1).
        let rows = [[1u32, 1, 0], [0, 1, 1]];
        let (_image, kernel, qi) = field.image_kernel_quasi_inverse(2, 3, |i, mut row| {
            for (j, &val) in rows[i].iter().enumerate() {
                if val != 0 {
                    row.add_basis_element(j, val);
                }
            }
        });

        assert_eq!(field.submodule_dimension(&kernel), 0);
        assert_eq!(field.quasi_inverse_source_dimension(&qi), 2);
        assert_eq!(field.quasi_inverse_target_dimension(&qi), 3);

        // Apply the quasi-inverse to y = row 0 = (1,1,0), then re-apply the map: recover y.
        let mut preimage = field.new_vector(2);
        let y = FpVector::from_slice(field.prime(), &[1, 1, 0]);
        field.apply_quasi_inverse(&qi, field.as_slice_mut(&mut preimage), 1, y.as_slice());

        let mut reconstructed = field.new_vector(3);
        for (i, c) in field.as_slice(&preimage).iter_nonzero() {
            for (j, &val) in rows[i].iter().enumerate() {
                if val != 0 {
                    field
                        .as_slice_mut(&mut reconstructed)
                        .add_basis_element(j, (c * val) % 2);
                }
            }
        }
        for j in 0..3 {
            assert_eq!(field.as_slice(&reconstructed).entry(j), y.entry(j));
        }
    }

    #[test]
    fn differential_matrix_and_kernel() {
        let field = Field::new(ValidPrime::new(2));
        // X has dimension 2; the differential d: X -> Y (dim 2) has rows (1,0) and (1,0): rank 1, so
        // its kernel is 1-dimensional. There is no chain map (C has dimension 0).
        let d_rows = [[1u32, 0], [1, 0]];
        let matrix = field.differential_matrix(
            2,
            0,
            2,
            0,
            |_, _| {},
            |i, mut row| {
                for (j, &v) in d_rows[i].iter().enumerate() {
                    if v != 0 {
                        row.add_basis_element(j, v);
                    }
                }
            },
        );
        let kernel = field.step_kernel(&matrix);
        assert_eq!(field.submodule_dimension(&kernel), 1);
    }

    #[test]
    fn resolution_step_augmentation() {
        let field = Field::new(ValidPrime::new(2));
        // s = 0 augmentation: X_0 is empty so far, C_0 has dimension 2, X_{-1} is zero. We expect two
        // new generators mapping isomorphically onto C_0, with no differential.
        let matrix = field.differential_matrix(0, 2, 0, 2, |_, _| {}, |_, _| {});
        let next = field.construct_next_stage(
            matrix,
            NextStageInput {
                previous_chain_map_quasi_inverse: None,
                previous_kernel: None,
                complex_differential_target_dim: 0,
            },
            |_, _| {},
            2,
        );

        assert_eq!(next.num_new_gens, 2);
        assert_eq!(next.chain_map_rows.len(), 2);
        // The chain map on the new generators is the identity onto C_0.
        assert_eq!(next.chain_map_rows[0].entry(0), 1);
        assert_eq!(next.chain_map_rows[0].entry(1), 0);
        assert_eq!(next.chain_map_rows[1].entry(0), 0);
        assert_eq!(next.chain_map_rows[1].entry(1), 1);
        // No differential (X_{-1} is zero).
        for row in &next.differential_rows {
            assert_eq!(row.len(), 0);
        }
    }
}
