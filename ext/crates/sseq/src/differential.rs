use fp::{
    matrix::{Matrix, Subspace},
    prime::ValidPrime,
    vector::{prelude::*, FpVector, Slice, SliceMut},
};

pub struct Differential {
    pub matrix: Matrix,
    first_empty_row: usize,
    source_dim: usize,
    target_dim: usize,
    error: bool,
}

impl Differential {
    pub fn new(p: ValidPrime, source_dim: usize, target_dim: usize) -> Self {
        // Leave more rows to make room for inconsistent differentials
        Differential {
            matrix: Matrix::new(p, source_dim + target_dim + 1, source_dim + target_dim),
            source_dim,
            target_dim,
            error: false,
            first_empty_row: 0,
        }
    }

    pub fn set_to_zero(&mut self) {
        self.matrix.set_to_zero();
        self.first_empty_row = 0;
        self.error = false;
    }

    /// Add a differential
    ///
    /// # Return
    /// Whether a new differential was indeed added. If false, the differential is already
    /// existent.
    ///
    /// # Arguments
    ///  - `source`: The source of the differential
    ///  - `target`: The target of the differential. If `None`, the differential is zero. This
    ///     should be reduced by the known images of earlier differentials.
    pub fn add(&mut self, source: Slice, target: Option<Slice>) -> bool {
        let source_dim = self.source_dim;
        let target_dim = self.target_dim;
        let next_row = &mut self.matrix[self.first_empty_row];
        next_row.slice_mut(0, source_dim).add(source, 1);

        // The last row is always empty
        if let Some(t) = target {
            next_row
                .slice_mut(source_dim, source_dim + target_dim)
                .assign(t);
        };
        if next_row.is_zero() {
            return false;
        }
        self.matrix.row_reduce();

        if self.matrix[self.first_empty_row].is_zero() {
            false
        } else {
            self.first_empty_row += 1;
            true
        }
    }

    /// An iterator of differentials in the form `(source, target)`
    pub fn get_source_target_pairs(&self) -> Vec<(FpVector, FpVector)> {
        let source_dim = self.source_dim;
        let target_dim = self.target_dim;
        self.matrix
            .iter()
            .filter(|d| !d.is_zero())
            .map(move |d| {
                (
                    d.slice(0, source_dim).into_owned(),
                    d.slice(source_dim, source_dim + target_dim).into_owned(),
                )
            })
            .collect()
    }

    /// Given a subspace of the target space, project the target vectors to the complement.
    pub fn reduce_target(&mut self, zeros: &Subspace) {
        assert_eq!(zeros.matrix.columns(), self.target_dim);

        for row in self.matrix.iter_mut() {
            zeros.reduce(row.slice_mut(self.source_dim, self.source_dim + self.target_dim));
        }

        self.matrix.row_reduce();

        self.error = false;
        for i in 0..self.target_dim {
            if self.matrix.pivots()[self.source_dim + i] >= 0 {
                self.error = true;
                break;
            }
        }
    }

    /// This evaluates the differential on `source`, adding the result to `target`. This assumes
    /// all unspecified differentials are zero. More precisely, it assumes every non-pivot column
    /// of the differential matrix has zero differential. This may or may not be actually true
    /// (e.g. if we only know d(a + b) = c, it might be that d(a) = c and d(b) = 0, or vice versa,
    /// or neither. Here we assume d(a) = c and d(b) = 0.
    pub fn evaluate(&self, source: Slice, mut target: SliceMut) {
        for (i, c) in source.iter_nonzero() {
            let row = self.matrix.pivots()[i];
            if row < 0 {
                continue;
            }
            let row = row as usize;

            target.add(
                self.matrix[row].slice(self.source_dim, self.source_dim + self.target_dim),
                c,
            );
        }
    }

    pub fn prime(&self) -> ValidPrime {
        self.matrix.prime()
    }

    /// Whether the current set of differentials is inconsistent. This should be called only after
    /// `reduce_target` is called.
    pub fn inconsistent(&self) -> bool {
        self.error
    }

    /// Find the differential that hits `value`, and write the result to `result`.
    ///
    /// This computes the quasi-inverse from scratch and allocates two matrices, and should not be
    /// used in a hot path.
    pub fn quasi_inverse(&self, result: SliceMut, value: Slice) {
        let mut matrix = Matrix::new(
            self.matrix.prime(),
            self.source_dim,
            self.source_dim + self.target_dim,
        );
        // Transpose the source and target columns
        for (target, source) in matrix.iter_mut().zip(self.matrix.iter()) {
            target
                .slice_mut(0, self.target_dim)
                .assign(source.slice(self.source_dim, self.source_dim + self.target_dim));
            target
                .slice_mut(self.target_dim, self.target_dim + self.source_dim)
                .assign(source.slice(0, self.source_dim));
        }
        matrix.row_reduce();
        let qi = matrix.compute_quasi_inverse(self.target_dim, self.target_dim);
        qi.apply(result, 1, value);
    }
}
