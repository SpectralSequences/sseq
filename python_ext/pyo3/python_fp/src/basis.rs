use pyo3::prelude::*;
use pyo3::{PyObjectProtocol, PySequenceProtocol };

use fp::vector::{FpVector as FpVectorRust, FpVectorT};
use fp::matrix::Basis as BasisRust;


use python_utils;
use crate::prime::new_valid_prime;
use crate::vector::FpVector;
use crate::matrix::Matrix;

python_utils::wrapper_type!(Basis, BasisRust);

python_utils::py_repr!(Basis, "FreedBasis", {
    Ok(format!(
        "F{}Basis {}",
        inner.prime(),
        inner.matrix
    ))
});

impl Basis {
    fn check_dimension(&self, v : &FpVector) -> PyResult<()> {
        if v.inner()?.dimension() != self.inner()?.matrix.rows() {
            Err(python_utils::exception!(ValueError, "Wrong dimension"))
        } else {
            Ok(())
        }
    }
}


#[pymethods]
impl Basis {
    #[new]
    pub fn new(p: u32, dimension : usize) -> PyResult<Self> {
        Ok(Basis::box_and_wrap(BasisRust::new(new_valid_prime(p)?, dimension)))
    }

    #[getter]
    pub fn get_prime(&self) -> PyResult<u32> {
        Ok(*self.inner()?.prime())
    }

    #[getter]
    pub fn get_matrix(&mut self) -> PyResult<Matrix> {
        Ok(Matrix::wrap_immutable(&self.inner_mut()?.matrix, self.owner()))
    }

    // #[getter]
    // pub fn get_inverse(&mut self) -> PyResult<Matrix> {
    //     Ok(Matrix::wrap_immutable(&*self.inner_mut()?.inverse.segment(1, 1), self.owner()))
    // }

    pub fn apply(&self, result : &mut FpVector, v : &FpVector) -> PyResult<()> {
        self.check_dimension(v)?;
        self.check_dimension(result)?;
        self.inner()?.apply(result.inner_mut()?, v.inner()?);
        Ok(())
    }

    pub fn apply_inverse(&mut self, result : &mut FpVector, v : &FpVector) -> PyResult<()> {
        if v.inner()?.dimension() != self.inner()?.matrix.rows() {
            return Err(python_utils::exception!(ValueError, "Wrong dimension"));
        }
        if result.inner()?.dimension() != self.inner()?.matrix.rows() {
            return Err(python_utils::exception!(ValueError, "Wrong dimension"));
        }
        self.inner_mut()?.apply_inverse(result.inner_mut()?, v.inner()?);
        Ok(())
    }

    pub fn replace_entry(&mut self, row : usize, v : &FpVector) -> PyResult<()> {
        let slf = self.inner_mut()?;
        let v = v.inner()?;
        if v.dimension() != slf.matrix.rows() {
            return Err(python_utils::exception!(ValueError, "Wrong dimension"));
        }
        let mut temp_vec = FpVectorRust::new(slf.prime(), slf.matrix.rows());
        slf.apply_inverse(&mut temp_vec, v);
        if temp_vec.entry(row) == 0 {
            return Err(python_utils::exception!(ValueError, "Singular matrix!"));
        }
        slf.replace_entry(row, v);
        Ok(())
    }

}
