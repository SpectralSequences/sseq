use std::sync::Arc;

use crate::algebra::MuAlgebra;
use crate::module::{Module, ZeroModule};
use fp::vector::{Slice, SliceMut};
use once::{OnceBiVec, OnceVec};

#[derive(Clone, Debug)]
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
    num_gens: OnceBiVec<usize>,
    basis_element_to_opgen: OnceBiVec<OnceVec<OperationGeneratorPair>>,
    /// degree -> internal_gen_idx -> the offset of the generator in degree
    generator_to_index: OnceBiVec<OnceVec<usize>>,
}

impl<const U: bool, A: MuAlgebra<U>> std::fmt::Display for MuFreeModule<U, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<const U: bool, A: MuAlgebra<U>> MuFreeModule<U, A> {
    pub fn new(algebra: Arc<A>, name: String, min_degree: i32) -> Self {
        let gen_deg_idx_to_internal_idx = OnceBiVec::new(min_degree);
        gen_deg_idx_to_internal_idx.push(0);
        Self {
            algebra,
            name,
            min_degree,
            gen_names: OnceBiVec::new(min_degree),
            gen_deg_idx_to_internal_idx,
            num_gens: OnceBiVec::new(min_degree),
            basis_element_to_opgen: OnceBiVec::new(min_degree),
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

    fn compute_basis(&self, max_degree: i32) {
        let algebra = self.algebra();
        self.basis_element_to_opgen.extend(max_degree, |degree| {
            let new_row = OnceVec::new();
            self.generator_to_index.push_checked(OnceVec::new(), degree);

            let mut offset = 0;
            for (gen_deg, &num_gens) in self.num_gens.iter_enum() {
                let op_deg = degree - gen_deg;
                let num_ops = algebra.dimension_unstable(op_deg, gen_deg);
                for gen_idx in 0..num_gens {
                    self.generator_to_index[degree].push(offset);
                    offset += num_ops;
                    for op_idx in 0..num_ops {
                        new_row.push(OperationGeneratorPair {
                            generator_degree: gen_deg,
                            generator_index: gen_idx,
                            operation_degree: op_deg,
                            operation_index: op_idx,
                        });
                    }
                }
            }
            new_row
        });
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        assert!(
            degree < self.basis_element_to_opgen.len(),
            "Free Module {self} not computed through degree {degree}"
        );
        self.basis_element_to_opgen[degree].len()
    }

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
        mut result: SliceMut,
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
        } = *self.index_to_op_gen(mod_degree, mod_index);

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
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: Slice,
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
            let input_slice = input.slice(input_start, input_end);
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
    fn act(&self, result : SliceMut, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : Slice){
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

    pub fn add_generators(&self, degree: i32, num_gens: usize, names: Option<Vec<String>>) {
        // We need to acquire the lock because changing num_gens modifies the behaviour of
        // extend_table_entries, and the two cannot happen concurrently.
        let _lock = self.basis_element_to_opgen.lock();
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

        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[degree];
        // After adding generators in degree `t`, we now know when the generators for degree `t +
        // 1` starts.
        self.gen_deg_idx_to_internal_idx
            .push_checked(internal_gen_idx + num_gens, degree + 1);

        let algebra = self.algebra();
        let gen_deg = degree;
        for total_degree in degree..self.basis_element_to_opgen.len() {
            let op_deg = total_degree - gen_deg;
            let mut offset = self.basis_element_to_opgen[total_degree].len();
            let num_ops = algebra.dimension_unstable(op_deg, gen_deg);
            for gen_idx in 0..num_gens {
                self.generator_to_index[total_degree].push(offset);
                offset += num_ops;
                for op_idx in 0..num_ops {
                    self.basis_element_to_opgen[total_degree].push(OperationGeneratorPair {
                        generator_degree: gen_deg,
                        generator_index: gen_idx,
                        operation_degree: op_deg,
                        operation_index: op_idx,
                    });
                }
            }
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
            .iter_enum()
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

    pub fn index_to_op_gen(&self, degree: i32, index: usize) -> &OperationGeneratorPair {
        assert!(degree >= self.min_degree);
        &self.basis_element_to_opgen[degree][index]
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
        v: Slice<'a>,
    ) -> Slice<'a> {
        let start = self.generator_offset(degree, gen_degree, gen_index);
        let len = self
            .algebra()
            .dimension_unstable(degree - gen_degree, gen_degree);
        v.slice(
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
        slice: Slice<'a>,
    ) -> impl Iterator<Item = (i32, usize, i32, Slice<'a>)> + 'a {
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

/*
#[cfg(not(feature = "cache-multiplication"))]
impl<A: Algebra> MuFreeModule<A> {
    fn standard_act(&self, result : SliceMut, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : Slice) {
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
    fn custom_milnor_act(&self, algebra: &MilnorAlgebra, result : SliceMut, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : Slice) {
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
