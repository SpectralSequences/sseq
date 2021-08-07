use std::sync::Arc;

use crate::algebra::Algebra;
use crate::module::Module;
use bivec::BiVec;
use fp::vector::{Slice, SliceMut};
use once::{OnceBiVec, OnceVec};

#[cfg(feature = "json")]
use {
    crate::algebra::JsonAlgebra,
    serde_json::{json, Value},
};

#[derive(Clone, Debug)]
pub struct OperationGeneratorPair {
    pub operation_degree: i32,
    pub operation_index: usize,
    pub generator_degree: i32,
    pub generator_index: usize,
}

/// A free module.
///
/// A free module is uniquely determined by its list of generators. The generators are listed in
/// increasing degrees, and the index in this list is the internal index.
pub struct FreeModule<A: Algebra> {
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

impl<A: Algebra> std::fmt::Display for FreeModule<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<A: Algebra> FreeModule<A> {
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

impl<A: Algebra> Module for FreeModule<A> {
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

    fn dimension(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        assert!(
            degree < self.basis_element_to_opgen.len(),
            "Free Module {} not computed through degree {}",
            self,
            degree
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
        return format!(
            "{}{}",
            op_str, self.gen_names[opgen.generator_degree][opgen.generator_index]
        );
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
        // assert!(op_index < self.algebra().dimension(op_degree, mod_degree));
        // assert!(self.dimension(op_degree + mod_degree) <= result.dimension());
        let operation_generator = self.index_to_op_gen(mod_degree, mod_index);
        let module_operation_degree = operation_generator.operation_degree;
        let module_operation_index = operation_generator.operation_index;
        let generator_degree = operation_generator.generator_degree;
        let generator_index = operation_generator.generator_index;

        // Now all of the output elements are going to be of the form s * x. Find where such things go in the output vector.
        let num_ops = self
            .algebra()
            .dimension(module_operation_degree + op_degree, generator_degree);
        let output_block_min = self.operation_generator_to_index(
            module_operation_degree + op_degree,
            0,
            generator_degree,
            generator_index,
        );
        let output_block_max = output_block_min + num_ops;

        // Now we multiply s * r and write the result to the appropriate position.
        self.algebra().multiply_basis_elements(
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
        let input_dim = self.dimension(input_degree);
        let output_dim = self.dimension(input_degree + op_degree);
        let algebra = self.algebra();

        let input_table = &self.generator_to_index[input_degree];
        let output_table = &self.generator_to_index[input_degree + op_degree];
        for (i, &idx) in input_table.iter().enumerate() {
            let end_idx = input_table.get(i + 1).copied().unwrap_or(input_dim);
            if end_idx == idx {
                // The algebra is empty in this degree
                continue;
            }
            let opgen = self.index_to_op_gen(input_degree, idx);
            algebra.multiply_basis_element_by_element(
                result.slice_mut(
                    output_table[i],
                    output_table.get(i + 1).copied().unwrap_or(output_dim),
                ),
                coeff,
                op_degree,
                op_index,
                opgen.operation_degree,
                input.slice(idx, end_idx),
                opgen.generator_degree,
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

impl<A: Algebra> FreeModule<A> {
    pub fn gen_names(&self) -> &OnceBiVec<Vec<String>> {
        &self.gen_names
    }

    pub fn max_table_degree(&self) -> i32 {
        self.generator_to_index.max_degree()
    }

    pub fn number_of_gens_in_degree(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        self.num_gens[degree]
    }

    pub fn extend_table_entries(&self, max_degree: i32) {
        self.basis_element_to_opgen.extend(max_degree, |degree| {
            let new_row = OnceVec::new();
            self.generator_to_index.push_checked(OnceVec::new(), degree);

            let mut offset = 0;
            for (gen_deg, &num_gens) in self.num_gens.iter_enum() {
                let op_deg = degree - gen_deg;
                let num_ops = self.algebra().dimension(op_deg, gen_deg);
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

    pub fn add_generators(&self, degree: i32, num_gens: usize, names: Option<Vec<String>>) {
        // We need to acquire the lock because changing num_gens modifies the behaviour of
        // extend_table_entries, and the two cannot happen concurrently.
        let _lock = self.basis_element_to_opgen.lock();
        assert!(degree >= self.min_degree);

        // println!("add_gens == degree : {}, num_gens : {}", degree, num_gens);
        // self.ensure_next_table_entry(degree);
        let gen_names = names.unwrap_or_else(|| {
            (0..num_gens)
                .map(|i| format!("x_{{{},{}}}", degree, i))
                .collect()
        });

        self.gen_names.push_checked(gen_names, degree);
        self.num_gens.push_checked(num_gens, degree);

        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[degree];
        // After adding generators in degree `t`, we now know when the generators for degree `t +
        // 1` starts.
        self.gen_deg_idx_to_internal_idx
            .push_checked(internal_gen_idx + num_gens, degree + 1);

        let gen_deg = degree;
        for total_degree in degree..self.basis_element_to_opgen.len() {
            let op_deg = total_degree - gen_deg;
            let mut offset = self.basis_element_to_opgen[total_degree].len();
            let num_ops = self.algebra().dimension(op_deg, gen_deg);
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

    pub fn operation_generator_pair_to_idx(&self, op_gen: &OperationGeneratorPair) -> usize {
        self.operation_generator_to_index(
            op_gen.operation_degree,
            op_gen.operation_index,
            op_gen.generator_degree,
            op_gen.generator_index,
        )
    }

    pub fn index_to_op_gen(&self, degree: i32, index: usize) -> &OperationGeneratorPair {
        assert!(degree >= self.min_degree);
        &self.basis_element_to_opgen[degree][index]
    }

    pub fn extend_by_zero(&self, degree: i32) {
        self.algebra.compute_basis(degree - self.min_degree);
        self.extend_table_entries(degree);
        for i in self.num_gens.len()..=degree {
            self.add_generators(i, 0, None)
        }
    }

    // Used by Yoneda. Gets nonempty dimensions.
    pub fn get_degrees_with_gens(&self, max_degree: i32) -> Vec<i32> {
        assert!(max_degree < self.gen_deg_idx_to_internal_idx.len() - 1);
        let mut result = Vec::new();
        for i in self.gen_deg_idx_to_internal_idx.min_degree()..max_degree {
            if self.gen_deg_idx_to_internal_idx[i + 1] > self.gen_deg_idx_to_internal_idx[i] {
                result.push(i);
            }
        }
        result
    }

    pub fn get_max_generator_degree(&self) -> i32 {
        let mut max = self.min_degree;
        // Ideally, we should use rev() here. However, the iterator involves a
        // flatten().take(), and Flatten doesn't implement ExactSizeIterator (since the sum
        // of lengths can overflow) and Take<T> doesn't implement DoubleEndedIterator
        // unless T implements ExactSizeIterator.
        for (i, num_gens) in self.num_gens.iter_enum() {
            if *num_gens > 0 {
                max = i;
            }
        }
        max
    }

    /// A version of element_to_string that names the generator as x_(t - s, s, idx). The input s
    /// only affects how the output is displayed.
    pub fn element_to_string_pretty(&self, s: u32, t: i32, vec: Slice) -> String {
        let mut first = true;

        let mut result = String::new();
        for (i, c) in vec.iter_nonzero() {
            if !first {
                result.push_str(" + ");
            }
            first = false;

            if c != 1 {
                result.push_str(&*format!("{} ", c));
            }
            let opgen = self.index_to_op_gen(t, i);
            let op_str = self
                .algebra()
                .basis_element_to_string(opgen.operation_degree, opgen.operation_index);
            if op_str != "1" {
                result.push_str(&*op_str);
                result.push(' ');
            }
            result.push_str(&*format!(
                "x_({},{},{})",
                opgen.generator_degree - s as i32 + 1,
                s - 1,
                opgen.generator_index
            ));
        }
        result
    }
}

#[cfg(feature = "json")]
impl<A: JsonAlgebra> FreeModule<A> {
    pub fn element_to_json(&self, degree: i32, elt: Slice) -> Value {
        let mut result = Vec::new();
        let algebra = self.algebra();
        for (i, v) in elt.iter_nonzero() {
            let opgen = self.index_to_op_gen(degree, i);
            result.push(json!({
                "op" : algebra.json_from_basis(opgen.operation_degree, opgen.operation_index),
                "gen" : self.gen_names[opgen.generator_degree][opgen.generator_index],
                "coeff" : v
            }));
        }
        Value::from(result)
    }
}

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<A: Algebra> Save for FreeModule<A> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.num_gens.save(buffer)
    }
}

impl<A: Algebra> Load for FreeModule<A> {
    type AuxData = (Arc<A>, i32);

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self> {
        let algebra = Arc::clone(&data.0);
        let min_degree = data.1;

        let result = FreeModule::new(algebra, "".to_string(), min_degree);

        let num_gens: BiVec<usize> = Load::load(buffer, &(min_degree, ()))?;
        result.algebra().compute_basis(num_gens.len() - min_degree);

        for (degree, num) in num_gens.iter_enum() {
            result.add_generators(degree, *num, None);
        }
        // We extend to one degree beyond the number of generators added, which is needed for
        // resolving to stem. It is always safe to extend more than we "need".
        result.extend_table_entries(num_gens.len());
        Ok(result)
    }
}

/*
#[cfg(not(feature = "cache-multiplication"))]
impl<A: Algebra> FreeModule<A> {
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

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    use crate::algebra::{AdemAlgebra, SteenrodAlgebra};
    use fp::prime::ValidPrime;
    use fp::vector::FpVector;

    #[test]
    fn test_free_mod() {
        let p = ValidPrime::new(2);
        let A = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(
            p,
            *p != 2,
            false,
            false,
        )));
        A.compute_basis(10);
        let M = FreeModule::new(Arc::clone(&A), "".to_string(), 0);
        M.extend_table_entries(10);
        println!("dim 0 : {}", M.dimension(0));
        M.add_generators(0, 1, None);
        println!("dim 0 : {}", M.dimension(0));
        println!("{:?}", M.basis_element_to_opgen);
        M.add_generators(1, 1, None);
        println!("dim 0 : {}", M.dimension(0));
        // for i in 2..10 {
        //     M.add_generators(i, &lock, table, 0, None);
        // }
        let output_deg = 6;
        let output_dim = M.dimension(output_deg);
        for i in 0..9 {
            println!("i : {}", i);
            assert_eq!(M.dimension(i), A.dimension(i, 0) + A.dimension(i - 1, 1));
        }

        for (gen_deg, gen_idx) in &[(0, 0), (1, 0)] {
            let idx = M.operation_generator_to_index(output_deg - *gen_deg, 0, *gen_deg, *gen_idx);
            println!("index : {}", idx);
        }
        let mut result = FpVector::new(p, output_dim);
        // M.act_on_basis(&mut result, 1, op_deg, op_idx, input_deg, input_idx);
        M.act_on_basis(result.as_slice_mut(), 1, 5, 0, 1, 0);
        println!("{}", result);
        println!(
            "result : {}",
            M.element_to_string(output_deg, result.as_slice())
        );
        result.set_to_zero();
        M.act_on_basis(result.as_slice_mut(), 1, 5, 0, 1, 1);
        println!("{}", result);
        println!(
            "result : {}",
            M.element_to_string(output_deg, result.as_slice())
        );
        println!("1, 0 : {}", M.basis_element_to_string(1, 0));
        println!("1, 1 : {}", M.basis_element_to_string(1, 1));
    }
}

// uint FreeModule_element_toJSONString(char *result, FreeModule *this, int degree, Vector *element);
