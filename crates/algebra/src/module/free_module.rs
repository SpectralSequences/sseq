use parking_lot::{Mutex, MutexGuard};
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;

use crate::algebra::Algebra;
use crate::module::Module;
use bivec::BiVec;
use fp::vector::{FpVector, FpVectorT};
use once::OnceBiVec;

#[derive(Clone, Debug)]
pub struct OperationGeneratorPair {
    pub operation_degree: i32,
    pub operation_index: usize,
    pub generator_degree: i32,
    pub generator_index: usize,
}

#[derive(Clone)]
pub struct FreeModuleTableEntry {
    pub num_gens: usize,
    pub basis_element_to_opgen: Vec<OperationGeneratorPair>,
    pub generator_to_index: Vec<usize>,
}

pub struct FreeModule<A: Algebra> {
    pub algebra: Arc<A>,
    pub name: String,
    pub min_degree: i32,
    pub gen_names: OnceBiVec<Vec<String>>,
    gen_deg_idx_to_internal_idx: OnceBiVec<usize>,
    pub table: OnceBiVec<FreeModuleTableEntry>,
    next_table_entry : Mutex<Option<FreeModuleTableEntry>>
}

impl<A: Algebra> Module for FreeModule<A> {
    type Algebra = A;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn algebra(&self) -> Arc<A> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.min_degree
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        assert!(
            degree < self.table.len(),
            "Free Module {} not computed through degree {}",
            self.name(),
            degree
        );
        self.table[degree].basis_element_to_opgen.len()
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        let opgen = self.index_to_op_gen(degree, idx);
        let mut op_str = self
            .algebra
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
        result: &mut FpVector,
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
            &mut *result.borrow_slice(output_block_min, output_block_max),
            coeff,
            op_degree,
            op_index,
            module_operation_degree,
            module_operation_index,
            0,
        );
    }

    // Will need specialization
    /*    #[cfg(not(feature = "cache-multiplication"))]
    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
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
    pub fn new(algebra: Arc<A>, name: String, min_degree: i32) -> Self {
        let gen_deg_idx_to_internal_idx = OnceBiVec::new(min_degree);
        gen_deg_idx_to_internal_idx.push(0);
        Self {
            algebra,
            name,
            min_degree,
            gen_names: OnceBiVec::new(min_degree),
            gen_deg_idx_to_internal_idx,
            table: OnceBiVec::new(min_degree),
            next_table_entry : Mutex::new(None)
        }
    }

    pub fn lock(&self) -> MutexGuard<Option<FreeModuleTableEntry>> {
        self.next_table_entry.lock()
    }

    pub fn max_computed_degree(&self) -> i32 {
        self.table.max_degree()
    }

    pub fn number_of_gens_in_degree(&self, degree: i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        self.table[degree].num_gens
    }

    pub fn start_next_table_entry(&self, degree: i32, next_table_entry : &mut Option<FreeModuleTableEntry>) {
        assert!(next_table_entry.is_none());
        let mut basis_element_to_opgen: Vec<OperationGeneratorPair> = Vec::new();
        let mut generator_to_index: Vec<usize> = Vec::new();
        // gen_to_idx goes internal_gen_idx => start of block.
        // so gen_to_idx_size should be (number of possible degrees + 1) * sizeof(uint*) + number of gens * sizeof(uint).
        // The other part of the table goes idx => opgen
        // The size should be (number of basis elements in current degree) * sizeof(FreeModuleOperationGeneratorPair)
        // A basis element in degree n comes from a generator in degree i paired with an operation in degree n - i.
        let mut offset = 0;
        for gen_deg in self.min_degree..degree {
            let num_gens = self.number_of_gens_in_degree(gen_deg);
            let op_deg = degree - gen_deg;
            let num_ops = self.algebra().dimension(op_deg, gen_deg);
            for gen_idx in 0..num_gens {
                generator_to_index.push(offset);
                for op_idx in 0..num_ops {
                    basis_element_to_opgen.push(OperationGeneratorPair {
                        generator_degree: gen_deg,
                        generator_index: gen_idx,
                        operation_degree: op_deg,
                        operation_index: op_idx,
                    })
                }
                offset += num_ops;
            }
        }
        *next_table_entry = 
            Some(FreeModuleTableEntry {
                num_gens: 0,
                basis_element_to_opgen,
                generator_to_index,
            })
    }

    pub fn ensure_next_table_entry(&self, degree : i32, 
        next_table_entry : &mut Option<FreeModuleTableEntry>
    ) {
        if degree < self.table.len() {
            return
        }
        if let None = *next_table_entry {
            self.start_next_table_entry(degree, next_table_entry)
        }
    }

    pub fn finish_next_table_entry(&self, 
        next_table_entry : &mut Option<FreeModuleTableEntry>
    ) {
        self.table.push(next_table_entry.take().unwrap());
    }

    pub fn get_table_allow_unfinished<'b, 'a : 'b>(&'a self, degree : i32,
        next_table_entry : &'b Option<FreeModuleTableEntry>
    ) -> &'b FreeModuleTableEntry {
        assert!(degree <= self.table.len());
        if degree == self.table.len() {
            if let Some(table) = &*next_table_entry  {
                table
            } else {
                panic!()
            }
        } else {
            &self.table[degree]
        }
    }

    pub fn dimension_allow_unfinished(&self, degree : i32, 
        next_table_entry : &Option<FreeModuleTableEntry>
    ) -> usize {
        assert!(degree <= self.table.len());
        if degree == self.table.len() {
            if let Some(table) = &*next_table_entry  {
                table.basis_element_to_opgen.len()
            } else {
                panic!()
            }
        } else {
            self.table[degree].basis_element_to_opgen.len()
        }
    }

    pub fn add_generators(
        &self,
        degree: i32,
        next_table_entry_lock : &mut MutexGuard<Option<FreeModuleTableEntry>>,
        num_gens: usize,
        names: Option<Vec<String>>,
    ) {
        assert!(degree >= self.min_degree);
        assert!(std::ptr::eq(lock_api::MutexGuard::mutex(&next_table_entry_lock), &self.next_table_entry));
        assert_eq!(self.table.len(), degree);
        self.ensure_next_table_entry(degree, next_table_entry_lock);
        let mut gen_names;
        if let Some(names_vec) = names {
            gen_names = names_vec;
        } else {
            gen_names = Vec::with_capacity(num_gens);
            for i in 0..num_gens {
                gen_names.push(format!("x_{{{},{}}}", degree, i));
            }
        }
        self.gen_names.push(gen_names);
        self.add_generators_to_table(degree, next_table_entry_lock, num_gens);
        self.finish_next_table_entry(next_table_entry_lock);
    }

    fn add_generators_to_table(
        &self,
        degree: i32,
        next_table_entry_lock : &mut MutexGuard<Option<FreeModuleTableEntry>>,
        num_gens: usize,
    ) {
        match &mut **next_table_entry_lock {
            None => {panic!("")},
            Some(table) => {
                table.num_gens = num_gens;
                let old_dimension = table.basis_element_to_opgen.len();
                let mut start_of_block = old_dimension;
                let internal_gen_idx = self.gen_deg_idx_to_internal_idx[degree];
                self.gen_deg_idx_to_internal_idx
                    .push(internal_gen_idx + num_gens);
                // let mut gen_to_idx = Vec::with_capacity(num_gens);
                for gen_idx in 0..num_gens {
                    table.basis_element_to_opgen.push(OperationGeneratorPair {
                        generator_degree: degree,
                        generator_index: gen_idx,
                        operation_degree: 0,
                        operation_index: 0,
                    });
                    table.generator_to_index.push(start_of_block);
                    start_of_block += 1;
                }
            }
        }
    }

    pub fn generator_offset(&self, degree: i32, gen_deg: i32, gen_idx: usize) -> usize {
        assert!(gen_deg >= self.min_degree);
        let internal_gen_idx = self.gen_deg_idx_to_internal_idx[gen_deg] + gen_idx;
        assert!(internal_gen_idx <= self.gen_deg_idx_to_internal_idx[gen_deg + 1]);
        self.table[degree].generator_to_index[internal_gen_idx]
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
        self.table[op_deg + gen_deg].generator_to_index[internal_gen_idx] + op_idx
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
        &self.table[degree].basis_element_to_opgen[index]
    }

    pub fn element_to_json(&self, degree: i32, elt: &FpVector) -> Value {
        let mut result = Vec::new();
        let algebra = self.algebra();
        for (i, v) in elt.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let opgen = self.index_to_op_gen(degree, i);
            result.push(json!({
                "op" : algebra.json_from_basis(opgen.operation_degree, opgen.operation_index),
                "gen" : self.gen_names[opgen.generator_degree][opgen.generator_index],
                "coeff" : v
            }));
        }
        Value::from(result)
    }

    pub fn add_generators_immediate(
        &self,
        degree: i32,
        num_gens: usize,
        gen_names: Option<Vec<String>>,
    ) {
        self.add_num_generators(degree, &mut self.lock(), num_gens, gen_names);
    }

    pub fn add_num_generators(
        &self,
        degree: i32,
        next_table_entry_lock : &mut MutexGuard<Option<FreeModuleTableEntry>>,
        num_gens: usize,
        gen_names: Option<Vec<String>>,
    ) {
        self.add_generators(degree, next_table_entry_lock, num_gens, gen_names);
    }

    pub fn extend_by_zero(&self, degree: i32) {
        let mut lock = self.lock();
        for i in self.table.len()..=degree {
            self.add_num_generators(i, &mut lock, 0, None)
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
        for (i, table) in self.table.iter_enum() {
            if table.num_gens > 0 {
                max = i;
            }
        }
        max
    }
}

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<A: Algebra> Save for FreeModule<A> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        let num_gens: Vec<usize> = self.table.iter().map(|t| t.num_gens).collect::<Vec<_>>();
        let num_gens: BiVec<usize> = BiVec::from_vec(self.table.min_degree(), num_gens);
        num_gens.save(buffer)
    }
}

impl<A: Algebra> Load for FreeModule<A> {
    type AuxData = (Arc<A>, i32);

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self> {
        let algebra = Arc::clone(&data.0);
        let min_degree = data.1;

        let result = FreeModule::new(algebra, "".to_string(), min_degree);

        let num_gens: BiVec<usize> = Load::load(buffer, &(min_degree, ()))?;
        let mut lock = result.lock();
        for (degree, num) in num_gens.iter_enum() {
            result.add_num_generators(degree, &mut lock, *num, None);
        }
        drop(lock);
        Ok(result)
    }
}

/*
#[cfg(not(feature = "cache-multiplication"))]
impl<A: Algebra> FreeModule<A> {
    fn standard_act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector) {
        assert!(input.dimension() == self.dimension(input_degree));
        let p = *self.prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
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
    fn custom_milnor_act(&self, algebra: &MilnorAlgebra, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector) {
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

        let terms : Vec<usize> = input.iter().enumerate()
            .filter(|(_, x)| *x != 0)
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

    #[test]
    fn test_free_mod() {
        let p = ValidPrime::new(2);
        let A = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(p, *p != 2, false)));
        A.compute_basis(10);
        let M = FreeModule::new(Arc::clone(&A), "".to_string(), 0);
        let lock = M.lock();
        let table = M.construct_table(0);
        M.add_generators(0, &lock, table, 1, None);
        let table = M.construct_table(1);
        M.add_generators(1, &lock, table, 1, None);
        for i in 2..10 {
            let table = M.construct_table(i);
            M.add_generators(i, &lock, table, 0, None);
        }
        let output_deg = 6;
        let output_dim = M.dimension(output_deg);
        for i in 0..9 {
            assert_eq!(M.dimension(i), A.dimension(i, 0) + A.dimension(i - 1, 1));
        }

        for (gen_deg, gen_idx) in &[(0, 0), (1, 0)] {
            let idx = M.operation_generator_to_index(output_deg - *gen_deg, 0, *gen_deg, *gen_idx);
            println!("index : {}", idx);
        }
        let mut result = FpVector::new(p, output_dim);
        // M.act_on_basis(&mut result, 1, op_deg, op_idx, input_deg, input_idx);
        M.act_on_basis(&mut result, 1, 5, 0, 1, 0);
        println!("{}", result);
        println!("result : {}", M.element_to_string(output_deg, &result));
        result.set_to_zero();
        M.act_on_basis(&mut result, 1, 5, 0, 1, 1);
        println!("{}", result);
        println!("result : {}", M.element_to_string(output_deg, &result));
        println!("1, 0 : {}", M.basis_element_to_string(1, 0));
        println!("1, 1 : {}", M.basis_element_to_string(1, 1));
    }
}

// uint FreeModule_element_toJSONString(char *result, FreeModule *this, int degree, Vector *element);
