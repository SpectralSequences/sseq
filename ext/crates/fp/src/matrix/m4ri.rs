use itertools::Itertools;

use crate::{
    field::{limb::LimbMethods, Fp},
    limb::{Limb, LimbBitIndexPair},
    matrix::Matrix,
    prime::P2,
    simd,
};

#[derive(Debug, Default)]
/// M4RI works as follows --- first row reduce k rows using the naive algorithm. We then construct
/// a table of all 2^k linear combinations of these rows. This can be done in O(2^k) time. We then
/// use this table to reduce the remaining rows, so that each row takes O(`num_columns`) time,
/// which reduces the time taken by a factor of k x density.
///
/// Since we are likely to run into empty rows when doing row reduction, what we do in practice is
/// that we keep reducing rows until we collect k of them. Whenever we find a row, we record it
/// with [`M4riTable::add`]. We only record the row number and pivot column, as the values of these
/// rows will change as we go on due to the desire to land in a RREF.
///
/// Once we have recorded enough rows, we generate the table using [`M4riTable::generate`], and
/// then reduce limbs using [`M4riTable::reduce`]. When we are done we clear it using
/// [`M4riTable::clear`] and proceed to the next k rows.
pub(crate) struct M4riTable {
    /// The indices of new rows in the table
    rows: Vec<usize>,
    /// The list of pivot columns of the rows
    columns: Vec<LimbBitIndexPair>,
    /// The 2^k linear combinations of the k rows, apart from the first one which is identically
    /// zero.
    data: Vec<Limb>,
    /// The smallest non-zero limb in this table. We use this when row reducing to save a few
    /// operations.
    min_limb: usize,
}

impl M4riTable {
    /// Create a table with space for `k` vectors, each with `cols` columns.
    pub fn new(k: usize, cols: usize) -> Self {
        let num_limbs = Fp(P2).number(cols);
        Self {
            rows: Vec::with_capacity(k),
            columns: Vec::with_capacity(k),
            min_limb: 0,
            // There are 2^k rows but the first one is zero which we omit
            data: Vec::with_capacity(((1 << k) - 1) * num_limbs),
        }
    }

    /// Number of rows in the M4riTable
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Whether the table has no rows
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Get the list of pivot rows
    pub fn rows(&self) -> &[usize] {
        &self.rows
    }

    /// Add a row to the table.
    ///
    /// # Arguments
    ///  - `column`: pivot column of the row
    ///  - `row`: index of the row
    pub fn add(&mut self, column: usize, row: usize) {
        self.columns.push(Fp(P2).limb_bit_index_pair(column));
        self.rows.push(row);
    }

    /// Clear the contents of the table
    pub fn clear(&mut self) {
        self.columns.clear();
        self.rows.clear();
        self.data.clear();
    }

    /// Generates the table from the known data
    /// `num` is the number of the vector being added.
    pub fn generate(&mut self, matrix: &Matrix) {
        let num_limbs = matrix[0].limbs().len();
        self.min_limb = usize::MAX;
        for (n, (c, &r)) in self.columns.iter().zip_eq(&self.rows).enumerate() {
            let old_len = self.data.len();
            self.data.extend_from_slice(matrix[r].limbs());
            self.data.extend_from_within(..old_len);
            for i in 1 << n..2 * (1 << n) - 1 {
                simd::add_simd(
                    &mut self.data[i * num_limbs..(i + 1) * num_limbs],
                    matrix[r].limbs(),
                    c.limb,
                );
            }
            self.min_limb = std::cmp::min(self.min_limb, c.limb);
        }
    }

    pub fn reduce_naive(&self, matrix: &mut Matrix, target: usize) {
        for (&row, col) in self.rows.iter().zip_eq(&self.columns) {
            assert!(target != row);
            unsafe {
                let coef = (matrix[target].limbs()[col.limb] >> col.bit_index) & 1;
                if coef != 0 {
                    let (target, source) = matrix.split_borrow(target, row);
                    simd::add_simd(target.limbs_mut(), source.limbs(), col.limb)
                }
            }
        }
    }

    pub fn reduce(&self, v: &mut [Limb]) {
        let num_limbs = v.len();
        let mut index: usize = 0;
        for &col in self.columns.iter().rev() {
            index <<= 1;
            index += ((v[col.limb] >> col.bit_index) & 1) as usize;
        }
        if index != 0 {
            simd::add_simd(v, &self.data[(index - 1) * num_limbs..], self.min_limb);
        }
    }
}
