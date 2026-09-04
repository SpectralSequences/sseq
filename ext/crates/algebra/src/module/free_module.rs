use core::range::Range;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use fp::vector::{FpSlice, FpSliceMut};
use once::{OnceBiVec, OnceVec};

use crate::{
    algebra::MuAlgebra,
    module::{Module, ZeroModule},
};

/// One generator's run of consecutive basis elements within a single degree.
#[derive(Clone, Copy)]
struct GeneratorBlock {
    ordinal: usize,
    generator_degree: i32,
    generator_index: usize,
    start: usize,
}

impl GeneratorBlock {
    /// The pair for the basis element at `index`, which must lie in this block.
    fn op_gen(&self, degree: i32, index: usize) -> OperationGeneratorPair {
        OperationGeneratorPair {
            generator_degree: self.generator_degree,
            generator_index: self.generator_index,
            operation_degree: degree - self.generator_degree,
            operation_index: index - self.start,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationGeneratorPair {
    pub operation_degree: i32,
    pub operation_index: usize,
    pub generator_degree: i32,
    pub generator_index: usize,
}

pub type FreeModule<A> = MuFreeModule<false, A>;
pub type UnstableFreeModule<A> = MuFreeModule<true, A>;

/// A free module.
///
/// A free module is uniquely determined by its list of generators. The generators are listed in
/// increasing degrees, and the index in this list is the internal index.
pub struct MuFreeModule<const U: bool, A: MuAlgebra<U>> {
    algebra: Arc<A>,
    name: String,
    min_degree: i32,
    gen_names: OnceBiVec<Vec<String>>,
    /// degree -> internal index of first generator in degree
    gen_deg_idx_to_internal_idx: OnceBiVec<usize>,
    /// internal index -> the degree of that generator.
    ///
    /// The inverse of `gen_deg_idx_to_internal_idx`, materialised so that
    /// [`Self::index_to_op_gen`] can invert it by indexing rather than by searching. One `i32` per
    /// generator, and there are far fewer generators than basis elements.
    internal_idx_to_gen_deg: OnceVec<i32>,
    num_gens: OnceBiVec<usize>,
    /// degree -> number of basis elements.
    ///
    /// This is all that is kept of the per-basis-element table: the records themselves are derived
    /// by [`Self::index_to_op_gen`].
    dimensions: OnceBiVec<AtomicUsize>,
    /// degree -> internal_gen_idx -> the offset of the generator in degree
    generator_to_index: OnceBiVec<OnceVec<usize>>,
}

impl<const U: bool, A: MuAlgebra<U>> std::fmt::Display for MuFreeModule<U, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<const U: bool, A: MuAlgebra<U>> MuFreeModule<U, A> {
    /// A free module with no generators yet; add them with [`Self::add_generators`].
    pub fn new(algebra: Arc<A>, name: String, min_degree: i32) -> Self {
        let gen_deg_idx_to_internal_idx = OnceBiVec::new(min_degree);
        gen_deg_idx_to_internal_idx.push(0);
        Self {
            algebra,
            name,
            min_degree,
            gen_names: OnceBiVec::new(min_degree),
            gen_deg_idx_to_internal_idx,
            internal_idx_to_gen_deg: OnceVec::new(),
            num_gens: OnceBiVec::new(min_degree),
            dimensions: OnceBiVec::new(min_degree),
            generator_to_index: OnceBiVec::new(min_degree),
        }
    }
}

impl<const U: bool, A: MuAlgebra<U>> Module for MuFreeModule<U, A> {
    type Algebra = A;

    fn algebra(&self) -> Arc<A> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn max_computed_degree(&self) -> i32 {
        self.num_gens.max_degree()
    }

    fn max_generator_degree(&self) -> Option<i32> {
        for i in (0..self.num_gens.len()).rev() {
            if self.num_gens[i] > 0 {
                return Some(i);
            }
        }
        Some(self.min_degree)
    }

    /// Lay out the basis of every degree through `max_degree`.
    ///
    /// Only the per-generator block offsets are stored, plus the dimension they add up to. The
    /// per-basis-element records are derived on demand by [`Self::index_to_op_gen`].
    fn compute_basis(&self, max_degree: i32) {
        let algebra = self.algebra();
        // Only the OFFSETS are stored (one per generator); the per-basis-element records are
        // derived on demand in `index_to_op_gen`.
        self.dimensions.extend(max_degree, |degree| {
            self.generator_to_index.push_checked(OnceVec::new(), degree);

            let mut offset = 0;
            for (gen_deg, &num_gens) in &self.num_gens {
                let op_deg = degree - gen_deg;
                let num_ops = algebra.dimension_unstable(op_deg, gen_deg);
                for _ in 0..num_gens {
                    self.generator_to_index[degree].push(offset);
                    offset += num_ops;
                }
            }
            AtomicUsize::new(offset)
        });
    }

    /// The number of basis elements in `degree`.
    ///
    /// Acquiring this is what lets a caller then use [`Self::index_to_op_gen`]: it is the edge
    /// that orders the lookup against a concurrent [`Self::add_generators`].
    fn dimension(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        assert!(
            degree < self.dimensions.len(),
            "Free Module {self} not computed through degree {degree}"
        );
        self.dimensions[degree].load(Ordering::Acquire)
    }

    /// The name of a basis element, as the operation applied to the generator it acts on.
    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let opgen = self.index_to_op_gen(degree, idx);
        let mut op_str = self
            .algebra()
            .basis_element_to_string(opgen.operation_degree, opgen.operation_index);
        if &*op_str == "1" {
            op_str = "".to_string();
        } else {
            op_str.push(' ');
        }
        format!(
            "{}{}",
            op_str, self.gen_names[opgen.generator_degree][opgen.generator_index]
        )
    }

    fn act_on_basis(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        let OperationGeneratorPair {
            operation_degree: module_operation_degree,
            operation_index: module_operation_index,
            generator_degree,
            generator_index,
        } = self.index_to_op_gen(mod_degree, mod_index);

        // Now all of the output elements are going to be of the form s * x. Find where such things go in the output vector.
        let num_ops = self
            .algebra()
            .dimension(module_operation_degree + op_degree);
        let output_block_min = self.operation_generator_to_index(
            module_operation_degree + op_degree,
            0,
            generator_degree,
            generator_index,
        );
        let output_block_max = output_block_min + num_ops;

        // Now we multiply s * r and write the result to the appropriate position.
        self.algebra().multiply_basis_elements_unstable(
            result.slice_mut(output_block_min, output_block_max),
            coeff,
            op_degree,
            op_index,
            module_operation_degree,
            module_operation_index,
            generator_degree,
        );
    }

    fn act(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: FpSlice,
    ) {
        for GeneratorData {
            gen_deg,
            start: [input_start, output_start],
            end: [input_end, output_end],
        } in self.iter_gen_offsets([input_degree, input_degree + op_degree])
        {
            if input_start >= input.len() {
                break;
            }
            let input_slice = input.restrict(input_start, input_end);
            self.algebra.multiply_basis_element_by_element_unstable(
                result.slice_mut(output_start, output_end),
                coeff,
                op_degree,
                op_index,
                input_degree - gen_deg,
                input_slice,
                gen_deg,
            );
        }
    }

    // Will need specialization
    /*    #[cfg(not(feature = "cache-multiplication"))]
    fn act(&self, result : FpSliceMut, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : FpSlice){
        if *self.prime() == 2 {
            if let SteenrodAlgebra::MilnorAlgebra(m) = &*self.algebra() {
                self.custom_milnor_act(m, result, coeff, op_degree, op_index, input_degree, input);
            } else {
                self.standard_act(result, coeff, op_degree, op_index, input_degree, input);
            }
        } else {
                self.standard_act(result, coeff, op_degree, op_index, input_degree, input);
        }
    }*/
}

impl<const U: bool, A: MuAlgebra<U>> ZeroModule for MuFreeModule<U, A> {
    fn zero_module(algebra: Arc<A>, min_degree: i32) -> Self {
        let m = Self::new(algebra, String::from("0"), min_degree);
        m.add_generators(0, 0, None);
        m
    }
}

impl<const U: bool, A: MuAlgebra<U>> MuFreeModule<U, A> {
    pub fn gen_names(&self) -> &OnceBiVec<Vec<String>> {
        &self.gen_names
    }

    pub fn number_of_gens_in_degree(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        self.num_gens[degree]
    }

    /// Add `num_gens` generators in `degree`, extending every degree already computed.
    ///
    /// Generators must be added in consecutive degrees. Each already-computed total degree gains a
    /// block per new generator, and its dimension is republished to cover them.
    pub fn add_generators(&self, degree: i32, num_gens: usize, names: Option<Vec<String>>) {
        // We need to acquire the lock because changing num_gens modifies the behaviour of
        // extend_table_entries, and the two cannot happen concurrently.
        let _lock = self.dimensions.lock();
        assert!(degree >= self.min_degree);

        // println!("add_gens == degree : {}, num_gens : {}", degree, num_gens);
        // self.ensure_next_table_entry(degree);
        let gen_names = names.unwrap_or_else(|| {
            (0..num_gens)
                .map(|i| format!("x_{{{degree},{i}}}"))
                .collect()
        });

        self.gen_names.push_checked(gen_names, degree);
        self.num_gens.push_checked(num_gens, degree);
        for _ in 0..num_gens {
            self.internal_idx_to_gen_deg.push(degree);
        }

        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[degree];
        // After adding generators in degree `t`, we now know when the generators for degree `t +
        // 1` starts.
        self.gen_deg_idx_to_internal_idx
            .push_checked(internal_gen_idx + num_gens, degree + 1);

        let algebra = self.algebra();
        let gen_deg = degree;
        for total_degree in degree..self.dimensions.len() {
            let op_deg = total_degree - gen_deg;
            let mut offset = self.dimensions[total_degree].load(Ordering::Acquire);
            let num_ops = algebra.dimension_unstable(op_deg, gen_deg);
            for _ in 0..num_gens {
                self.generator_to_index[total_degree].push(offset);
                offset += num_ops;
            }
            // Release, and it must be: this store PUBLISHES the `generator_to_index` pushes
            // above. A reader that observes the new dimension with an acquire load is then
            // guaranteed to see the offsets backing it. `_lock` only excludes other writers --
            // readers do not take it, so it does nothing for this pairing.
            self.dimensions[total_degree].store(offset, Ordering::Release);
        }
    }

    /// Given a generator `(gen_deg, gen_idx)`, find the first index in degree `degree` with
    /// elements from the generator.
    pub fn internal_generator_offset(&self, degree: i32, internal_gen_idx: usize) -> usize {
        self.generator_to_index[degree][internal_gen_idx]
    }

    /// Iterate the degrees and indices of each generator up to degree `degree`.
    pub fn iter_gens(&self, degree: i32) -> impl Iterator<Item = (i32, usize)> + '_ {
        self.num_gens
            .iter()
            .take((degree - self.min_degree + 1) as usize)
            .flat_map(|(t, &n)| (0..n).map(move |k| (t, k)))
    }

    /// Iterate the degrees and offsets of each generator up to degree `degree`.
    pub fn iter_gen_offsets<const N: usize>(
        &self,
        degree: [i32; N],
    ) -> impl Iterator<Item = GeneratorData<N>> + '_ {
        OffsetIterator {
            module: self,
            degree,
            offset: [0; N],
            gen_deg: self
                .iter_gens(degree.into_iter().min().unwrap())
                .map(|(t, _)| t),
        }
    }

    /// Given a generator `(gen_deg, gen_idx)`, find the first index in degree `degree` with
    /// elements from the generator.
    pub fn generator_offset(&self, degree: i32, gen_deg: i32, gen_idx: usize) -> usize {
        assert!(gen_deg >= self.min_degree);
        assert!(gen_idx < self.num_gens[gen_deg]);
        self.internal_generator_offset(degree, self.gen_deg_idx_to_internal_idx[gen_deg] + gen_idx)
    }

    pub fn operation_generator_to_index(
        &self,
        op_deg: i32,
        op_idx: usize,
        gen_deg: i32,
        gen_idx: usize,
    ) -> usize {
        assert!(op_deg >= 0);
        assert!(gen_deg >= self.min_degree);
        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[gen_deg] + gen_idx;
        assert!(internal_gen_idx <= self.gen_deg_idx_to_internal_idx[gen_deg + 1]);
        self.generator_to_index[op_deg + gen_deg][internal_gen_idx] + op_idx
    }

    /// The `(operation, generator)` pair for basis element `index` of `degree`.
    ///
    /// The basis of a degree is consecutive blocks, one per generator, in `(gen_deg, gen_idx)`
    /// order, and `generator_to_index` holds each block's start offset. The pair is therefore
    /// recoverable from that offset array plus `internal_idx_to_gen_deg`, both indexed per
    /// generator, rather than from a record stored per basis element.
    ///
    /// Blocks may be empty: a generator admitting no operations in this degree shares its offset
    /// with the next one. The search must therefore take the LAST block whose offset is at most
    /// `index`, since taking the first would return a zero-width block.
    ///
    /// `index` must be one the caller has seen [`Module::dimension`] cover. That is what orders
    /// this lookup against a concurrent `add_generators`: the offsets for a degree are pushed
    /// before the dimension that counts them is published, so a caller that acquired the dimension
    /// also sees the offsets. The assertion below does not establish that edge and is not what
    /// makes the lookup sound; it is a bounds check, and it is worth its cost in release because
    /// the alternative for an out-of-range `index` is a subtraction that wraps and a panic from
    /// somewhere further in.
    pub fn index_to_op_gen(&self, degree: i32, index: usize) -> OperationGeneratorPair {
        assert!(degree >= self.min_degree);
        assert!(
            index < self.dimension(degree),
            "index {index} out of range in degree {degree}"
        );
        let block = self.block_containing(degree, index);
        block.op_gen(degree, index)
    }

    /// The generator block of `degree` that contains basis element `index`.
    fn block_containing(&self, degree: i32, index: usize) -> GeneratorBlock {
        let offsets = &self.generator_to_index[degree];

        // The last block starting at or before `index`: `partition_point` gives the first block
        // starting strictly after it, and every block starts at or after its predecessor. Taking
        // the last is what skips the empty blocks, which share their offset with the block after.
        let ordinal = offsets.partition_point(|&offset| offset <= index) - 1;
        let generator_degree = self.internal_idx_to_gen_deg[ordinal];

        GeneratorBlock {
            ordinal,
            generator_degree,
            generator_index: ordinal - self.gen_deg_idx_to_internal_idx[generator_degree],
            start: offsets[ordinal],
        }
    }

    /// A cursor over the basis of `degree`, for callers that look up many indices in ascending
    /// order.
    ///
    /// The cursor answers only indices below the dimension of `degree` as of this call.
    pub fn opgen_cursor(&self, degree: i32) -> OpGenCursor<'_, U, A> {
        OpGenCursor {
            module: self,
            degree,
            dimension: self.dimension(degree),
            // An empty range, so the first lookup takes the slow path whatever it asks for.
            range: (0..0).into(),
            block: GeneratorBlock {
                ordinal: 0,
                generator_degree: self.min_degree,
                generator_index: 0,
                start: 0,
            },
        }
    }

    pub fn extend_by_zero(&self, degree: i32) {
        self.algebra.compute_basis(degree - self.min_degree);
        self.compute_basis(degree);
        for i in self.num_gens.len()..=degree {
            self.add_generators(i, 0, None)
        }
    }

    /// Given a vector that represents an element in degree `degree`, slice it to the part that
    /// represents the terms that correspond to the specified generator.
    pub fn slice_vector<'a>(
        &self,
        degree: i32,
        gen_degree: i32,
        gen_index: usize,
        v: FpSlice<'a>,
    ) -> FpSlice<'a> {
        let start = self.generator_offset(degree, gen_degree, gen_index);
        let len = self
            .algebra()
            .dimension_unstable(degree - gen_degree, gen_degree);
        v.restrict(
            std::cmp::min(v.len(), start),
            std::cmp::min(v.len(), start + len),
        )
    }

    /// Given an element in a degree, iterate through the slices corresponding to each generator.
    /// Each item of the iterator is `(gen_degree, gen_index, op_degree, slice)`. This skips slices
    /// that are zero length.
    pub fn iter_slices<'a>(
        &'a self,
        degree: i32,
        slice: FpSlice<'a>,
    ) -> impl Iterator<Item = (i32, usize, i32, FpSlice<'a>)> + 'a {
        (self.min_degree..=degree)
            .flat_map(|t| (0..self.num_gens.get(t).copied().unwrap_or(0)).map(move |n| (t, n)))
            .map(move |(t, n)| (t, n, degree - t, self.slice_vector(degree, t, n, slice)))
            .filter(|(_, _, _, v)| !v.is_empty())
    }
}

pub struct GeneratorData<const N: usize> {
    pub gen_deg: i32,
    pub start: [usize; N],
    pub end: [usize; N],
}

/// A position in one degree's basis, for callers that look up many indices in ascending order.
///
/// [`MuFreeModule::index_to_op_gen`] searches the block offsets on every call. A caller walking a
/// degree's basis crosses a block boundary only once every `num_ops` indices, so remembering the
/// current block answers almost every lookup with a range check instead.
///
/// The cursor borrows the module, which is what makes it sound: it cannot outlive the module whose
/// layout it caches, and `generator_to_index` is append-only, so the block it remembers is never
/// rewritten. It is also pinned to the dimension `degree` had when it was created, which is what
/// bounds the last block -- that block is the only one with no following offset to end it, and a
/// concurrent [`MuFreeModule::add_generators`] would otherwise extend it under the cursor.
///
/// Lookups need not actually ascend. Any index below the pinned dimension is answered correctly; a
/// descending or scattered walk just misses more often and pays for the search.
pub struct OpGenCursor<'a, const U: bool, A: MuAlgebra<U>> {
    module: &'a MuFreeModule<U, A>,
    degree: i32,
    dimension: usize,
    /// The indices `block` covers.
    ///
    /// [`core::range::Range`] rather than [`std::ops::Range`], because the latter is an iterator
    /// and so deliberately not `Copy`, which a field read here would otherwise have to clone.
    range: Range<usize>,
    block: GeneratorBlock,
}

impl<const U: bool, A: MuAlgebra<U>> OpGenCursor<'_, U, A> {
    /// The `(operation, generator)` pair for basis element `index` of this cursor's degree.
    pub fn get(&mut self, index: usize) -> OperationGeneratorPair {
        assert!(
            index < self.dimension,
            "index {index} is past the dimension this cursor was created with"
        );
        if !self.range.contains(&index) {
            self.block = self.module.block_containing(self.degree, index);
            let offsets = &self.module.generator_to_index[self.degree];
            // The final block ends at the pinned dimension. A later `add_generators` may since
            // have pushed an offset there, but it equals that dimension, so either answer bounds
            // this block identically.
            let end = offsets
                .get(self.block.ordinal + 1)
                .copied()
                .unwrap_or(self.dimension);
            self.range = (self.block.start..end).into();
        }
        self.block.op_gen(self.degree, index)
    }

    /// The degree this cursor walks.
    pub fn degree(&self) -> i32 {
        self.degree
    }

    /// The dimension this cursor is pinned to.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

struct OffsetIterator<
    'a,
    const U: bool,
    A: MuAlgebra<U>,
    T: Iterator<Item = i32> + 'a,
    const N: usize,
> {
    module: &'a MuFreeModule<U, A>,
    degree: [i32; N],
    offset: [usize; N],
    gen_deg: T,
}

impl<'a, const U: bool, A: MuAlgebra<U>, T: Iterator<Item = i32> + 'a, const N: usize> Iterator
    for OffsetIterator<'a, U, A, T, N>
{
    type Item = GeneratorData<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut retval = GeneratorData {
            gen_deg: self.gen_deg.next()?,
            start: [0; N],
            end: [0; N],
        };

        for i in 0..N {
            retval.start[i] = self.offset[i];
            retval.end[i] = retval.start[i]
                + self
                    .module
                    .algebra
                    .dimension_unstable(self.degree[i] - retval.gen_deg, retval.gen_deg);
            self.offset[i] = retval.end[i];
        }
        Some(retval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AdemAlgebra, MilnorAlgebra, algebra::Algebra};

    /// Every basis element round-trips through both directions of the index/opgen correspondence.
    ///
    /// This is the invariant that the derivation in [`MuFreeModule::index_to_op_gen`] has to
    /// uphold, and the one a stored table upheld by construction.
    fn assert_round_trips<const U: bool, A: MuAlgebra<U>>(module: &MuFreeModule<U, A>, max: i32) {
        for degree in module.min_degree()..=max {
            let dim = module.dimension(degree);
            for index in 0..dim {
                let opgen = module.index_to_op_gen(degree, index);
                assert_eq!(
                    opgen.operation_degree + opgen.generator_degree,
                    degree,
                    "degrees of {opgen:?} do not sum to {degree}"
                );
                assert!(
                    opgen.generator_index < module.number_of_gens_in_degree(opgen.generator_degree)
                );
                assert!(
                    opgen.operation_index
                        < module
                            .algebra()
                            .dimension_unstable(opgen.operation_degree, opgen.generator_degree)
                );
                assert_eq!(
                    module.operation_generator_to_index(
                        opgen.operation_degree,
                        opgen.operation_index,
                        opgen.generator_degree,
                        opgen.generator_index,
                    ),
                    index
                );
            }

            // A cursor must answer exactly as the direct lookup does, ascending and descending.
            let mut cursor = module.opgen_cursor(degree);
            assert_eq!(cursor.degree(), degree);
            assert_eq!(cursor.dimension(), dim);
            for index in 0..dim {
                assert_eq!(cursor.get(index), module.index_to_op_gen(degree, index));
            }
            let mut cursor = module.opgen_cursor(degree);
            for index in (0..dim).rev() {
                assert_eq!(cursor.get(index), module.index_to_op_gen(degree, index));
            }
            // Scattered order, so a cursor that assumed monotonicity would be caught.
            let mut cursor = module.opgen_cursor(degree);
            for index in (0..dim).step_by(7).chain((0..dim).step_by(3)) {
                assert_eq!(cursor.get(index), module.index_to_op_gen(degree, index));
            }
        }
    }

    /// Generator degrees with no generators at all, which contribute no block.
    #[test]
    fn round_trip_with_empty_generator_degrees() {
        const NUM_GENS: [usize; 6] = [2, 0, 1, 0, 0, 3];
        const MAX: i32 = 12;

        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        algebra.compute_basis(MAX + 1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(MAX);
        for (degree, num_gens) in NUM_GENS.into_iter().enumerate() {
            module.add_generators(degree as i32, num_gens, None);
        }
        module.compute_basis(MAX);

        assert_round_trips(&module, MAX);
    }

    /// A generator in the top degree admits no operations, so its block is empty and shares an
    /// offset with the block after it. The search has to resolve that tie towards the later block.
    #[test]
    fn round_trip_with_empty_blocks() {
        const MAX: i32 = 6;

        let algebra = Arc::new(AdemAlgebra::new(fp::prime::TWO, false));
        algebra.compute_basis(MAX + 1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(MAX);
        // Generators in every degree up to MAX: the ones in degree MAX itself have an empty block
        // in degree MAX, as do those in degrees where the algebra has no operations to apply.
        for degree in 0..=MAX {
            module.add_generators(degree, 2, None);
        }
        module.compute_basis(MAX);

        assert_round_trips(&module, MAX);
    }

    /// `extend_by_zero` adds zero-generator degrees after the fact, past the generators.
    #[test]
    fn round_trip_after_extend_by_zero() {
        const MAX: i32 = 10;

        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        algebra.compute_basis(MAX + 1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(2);
        module.add_generators(0, 1, None);
        module.add_generators(1, 2, None);
        module.add_generators(2, 1, None);
        module.extend_by_zero(MAX);

        assert_round_trips(&module, MAX);
    }

    /// Generators added after a degree has already been computed extend that degree's blocks.
    #[test]
    fn round_trip_when_generators_are_added_late() {
        const MAX: i32 = 8;

        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        algebra.compute_basis(MAX + 1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(MAX);
        for degree in 0..=3 {
            module.add_generators(degree, 1, None);
            // Re-check every degree after each addition, so a stale offset would be caught at the
            // point it goes stale rather than only at the end.
            assert_round_trips(&module, MAX);
        }
    }

    /// An index past the dimension is rejected in release builds too, where the search would
    /// otherwise underflow and fail somewhere less obvious.
    #[test]
    #[should_panic(expected = "index 5 out of range in degree 0")]
    fn index_past_dimension_panics() {
        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        algebra.compute_basis(1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(0);
        module.add_generators(0, 1, None);
        module.compute_basis(0);

        assert_eq!(module.dimension(0), 1);
        module.index_to_op_gen(0, 5);
    }

    /// A degree with no generators at all has no blocks, so there is nothing for a lookup to find.
    #[test]
    #[should_panic(expected = "index 0 out of range in degree 0")]
    fn lookup_in_empty_degree_panics() {
        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
        algebra.compute_basis(1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(0);
        module.add_generators(0, 0, None);
        module.compute_basis(0);

        assert_eq!(module.dimension(0), 0);
        module.index_to_op_gen(0, 0);
    }

    /// A module over an odd prime, where `dimension_unstable` vanishes in more degrees.
    #[test]
    #[cfg(feature = "odd-primes")]
    fn round_trip_odd_prime() {
        const MAX: i32 = 20;

        let algebra = Arc::new(MilnorAlgebra::new(fp::prime::ValidPrime::new(3), false));
        algebra.compute_basis(MAX + 1);
        let module = FreeModule::new(algebra, "F".to_string(), 0);
        module.compute_basis(MAX);
        for degree in 0..=9 {
            let num_gens = match degree {
                0 | 9 => 1,
                4 => 2,
                _ => 0,
            };
            module.add_generators(degree, num_gens, None);
        }
        module.compute_basis(MAX);

        assert_round_trips(&module, MAX);
    }
}

/*
#[cfg(not(feature = "cache-multiplication"))]
impl<A: Algebra> MuFreeModule<A> {
    fn standard_act(&self, result : FpSliceMut, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : FpSlice) {
        assert!(input.dimension() == self.dimension(input_degree));
        let p = *self.prime();
        for (i, v) in input.iter_nonzer() {
            self.act_on_basis(result, (coeff * v) % p, op_degree, op_index, input_degree, i);
        }
    }

    /// For the Milnor algebra, there is a faster algorithm for computing the action, which I
    /// learnt from Christian Nassau. This is only implemented for p = 2 for now.
    ///
    /// To compute $\mathrm{Sq}(R) \mathrm{Sq}(S)$, we need to iterate over all admissible
    /// matrices, namely the $x_{i, j}$ such that $r_i = \sum x_{i, j} p^j$ and the column sums are
    /// the $s_j$.
    ///
    /// Now if we want to compute $\mathrm{Sq}(R) (\mathrm{Sq}(S^{(1)}) + \cdots)$, we can omit the
    /// 0th row of the matrix and the column sum condition. The such a matrix $x_{i, j}$
    /// contributes to $\mathrm{Sq}(R) \mathrm{Sq}(S^{(k)})$ iff the column sum is at most
    /// $s_{j}^{(k)}$. There are also some bitwise disjointness conditions we have to check to
    /// ensure the coefficient is non-zero.
    fn custom_milnor_act(&self, algebra: &MilnorAlgebra, result : FpSliceMut, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : FpSlice) {
        if coeff % 2 == 0 {
            return;
        }
        if op_degree == 0 {
            *result += input;
        }
        let op = algebra.basis_element_from_index(op_degree, op_index);
        let p_part = &op.p_part;
        let mut matrix = AdmissibleMatrix::new(p_part);

        let output_degree = input_degree + op_degree;
        let mut working_elt = MilnorBasisElement {
            q_part: 0,
            p_part: Vec::with_capacity(matrix.cols - 1),
            degree: 0,
        };

        let terms : Vec<usize> = input.iter_nonzero()
            .map(|(i, _)| i)
            .collect();

        loop {
            'outer: for &i in &terms {
                let elt = self.index_to_op_gen(input_degree, i);
                let basis = algebra.basis_element_from_index(elt.operation_degree, elt.operation_index);

                working_elt.p_part.clear();
                working_elt.p_part.reserve(max(basis.p_part.len(), matrix.masks.len()));

                for j in 0 .. min(basis.p_part.len(), matrix.col_sums.len()) {
                    if matrix.col_sums[j] > basis.p_part[j] {
                        continue 'outer;
                    }
                    if (basis.p_part[j] - matrix.col_sums[j]) & matrix.masks[j] != 0 {
                        continue 'outer;
                    }
                    working_elt.p_part.push((basis.p_part[j] - matrix.col_sums[j]) | matrix.masks[j]); // We are supposed to add the diagonal sum, but that is equal to the mask, and since there are no bit conflicts, this is the same as doing a bitwise or.
                }
                if basis.p_part.len() < matrix.col_sums.len() {
                    for j in basis.p_part.len() ..  matrix.col_sums.len() {
                        if matrix.col_sums[j] > 0 {
                            continue 'outer;
                        }
                    }
                    for j in basis.p_part.len() ..  matrix.masks.len() {
                        working_elt.p_part.push(matrix.masks[j])
                    }
                } else {
                    for j in matrix.col_sums.len() .. min(basis.p_part.len(), matrix.masks.len()) {
                        if basis.p_part[j] & matrix.masks[j] != 0 {
                            continue 'outer;
                        }
                        working_elt.p_part.push(basis.p_part[j] | matrix.masks[j]);
                    }
                    if basis.p_part.len() < matrix.masks.len() {
                        for j in basis.p_part.len() .. matrix.masks.len() {
                            working_elt.p_part.push(matrix.masks[j]);
                        }
                    } else {
                        for j in matrix.masks.len() .. basis.p_part.len() {
                            working_elt.p_part.push(basis.p_part[j])
                        }
                    }
                }
                while let Some(0) = working_elt.p_part.last() {
                    working_elt.p_part.pop();
                }
                working_elt.degree = output_degree - elt.generator_degree;

                let idx = self.operation_generator_to_index(
                    working_elt.degree,
                    algebra.basis_element_to_index(&working_elt),
                    elt.generator_degree,
                    elt.generator_index
                );
                result.add_basis_element(idx, 1);
            }
            if !matrix.next() {
                break;
            }
        }
    }
}

#[cfg(not(feature = "cache-multiplication"))]
struct AdmissibleMatrix {
    cols: usize,
    rows: usize,
    matrix: Vec<u32>,
    totals: Vec<u32>,
    col_sums: Vec<u32>,
    masks: Vec<u32>,
}

#[cfg(not(feature = "cache-multiplication"))]
impl AdmissibleMatrix {
    fn new(ps: &[u32]) -> Self {
        let rows = ps.len();
        let cols = ps.iter().map(|x| 32 - x.leading_zeros()).max().unwrap() as usize;
        let mut matrix = vec![0; rows * cols];
        for (i, &x) in ps.iter().enumerate() {
            matrix[i * cols] = x;
        }

        let mut masks = Vec::with_capacity(rows + cols - 1);
        masks.extend_from_slice(ps);
        masks.resize(rows + cols - 1, 0);

        Self {
            rows,
            cols,
            totals: vec![0; rows], // totals is only used next_matrix. No need to initialize
            col_sums: vec![0; cols - 1],
            matrix,
            masks,
        }
    }

    fn next(&mut self) -> bool {
        let mut p_to_the_j;
        for row in 0 .. self.rows {
            p_to_the_j = 1;
            self.totals[row] = self[row][0];
            'mid: for col in 1 .. self.cols {
                p_to_the_j *= 2;
                // We do a quick check before computing the bitsums.
                if p_to_the_j <= self.totals[row] {
                    // Compute bitsum
                    let mut d = 0;
                    for c in (row + col + 1).saturating_sub(self.rows) .. col {
                        d |= self[row + col - c][c];
                    }
                    // Magic - find next number greater than self[row][col] whose bitwise and with
                    // d is 0.
                    let new_entry = ((self[row][col] | d) + 1) & !d;
                    let inc = new_entry - self[row][col];
                    let sub = inc * p_to_the_j;
                    if self.totals[row] < sub {
                        self.totals[row] += p_to_the_j * self[row][col];
                        continue 'mid;
                    }
                    self[row][0] = self.totals[row] - sub;
                    self.masks[row] = self[row][0];
                    self.col_sums[col - 1] += inc;
                    for j in 1 .. col {
                        self.masks[row + j] &= !self[row][j];
                        self.col_sums[j - 1] -= self[row][j];
                        self[row][j] = 0;
                    }
                    self[row][col] = new_entry;

                    for i in 0 .. row {
                        self[i][0] = self.totals[i];
                        self.masks[i] = self.totals[i];
                        for j in 1 .. self.cols {
                            if i + j > row {
                                self.masks[i + j] &= !self[i][j];
                            }
                            self.col_sums[j - 1] -= self[i][j];
                            self[i][j] = 0;
                        }
                    }
                    self.masks[row + col] = d | new_entry;
                    return true;
                }
                self.totals[row] += p_to_the_j * self[row][col];
            }
        }
        false
    }
}

#[cfg(not(feature = "cache-multiplication"))]
impl std::ops::Index<usize> for AdmissibleMatrix {
    type Output = [u32];

    fn index(&self, row: usize) -> &Self::Output {
        &self.matrix[row * self.cols .. (row + 1) * self.cols]
    }
}

#[cfg(not(feature = "cache-multiplication"))]
impl std::ops::IndexMut<usize> for AdmissibleMatrix {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.matrix[row * self.cols .. (row + 1) * self.cols]
    }
}
*/
