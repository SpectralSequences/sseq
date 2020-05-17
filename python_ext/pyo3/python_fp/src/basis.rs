use pyo3::prelude::*;
use pyo3::{
    PyObjectProtocol, 
    PySequenceProtocol,
    types::{PyTuple, PyList}
};

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
    fn check_dimension(&self, dim : usize) -> PyResult<()> {
        if dim != self.inner()?.dimension() {
            Err(python_utils::exception!(ValueError, "Wrong dimension"))
        } else {
            Ok(())
        }
    }


    fn check_nonsingular(&self) -> PyResult<()> {
        if self.inner()?.is_singular() {
            Err(python_utils::exception!(ValueError, "Matrix is singular!"))
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
    pub fn get_matrix(&self) -> PyResult<Matrix> {
        Ok(Matrix::wrap_immutable(&self.inner()?.matrix, self.owner()))
    }

    #[getter]
    pub fn get_inverse(&self) -> PyResult<Matrix> {
        Ok(Matrix::wrap_immutable(&*self.inner()?.inverse, self.owner()))
    }

    pub fn apply(&self, result : &mut FpVector, v : &FpVector) -> PyResult<()> {
        self.check_dimension(v.inner()?.dimension())?;
        self.check_dimension(result.inner()?.dimension())?;
        self.check_nonsingular()?;
        self.inner()?.apply(result.inner_mut()?, v.inner()?);
        Ok(())
    }

    pub fn apply_inverse(&self, result : &mut FpVector, v : &FpVector) -> PyResult<()> {
        self.check_dimension(v.inner()?.dimension())?;
        self.check_dimension(result.inner()?.dimension())?;
        self.check_nonsingular()?;
        self.inner_mut()?.apply_inverse(result.inner_mut()?, v.inner()?);
        Ok(())
    }

    pub fn set_matrix(&mut self, value : PyObject) -> PyResult<()> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        if let Ok(m) = value.extract::<Matrix>(py) {
            let m_inner = m.inner()?;
            self.check_dimension(m_inner.rows())?;
            self.check_dimension(m_inner.columns())?;
            let slf = self.inner_mut()?;
            for i in 0 .. slf.dimension() {
                slf.matrix[i].assign(&m_inner[i]);
            }
        } else if let Ok(m) = value.extract::<Vec<FpVector>>(py) {
            self.check_dimension(m.len())?;
            for i in 0 .. self.inner()?.dimension() {
                self.check_dimension(m[i].inner()?.dimension())?;
            }
            let slf = self.inner_mut()?;
            for i in 0 .. slf.dimension() {
                slf.matrix[i].assign(m[i].inner()?);
            }
        } else if let Ok(m) = value.extract::<Vec<Vec<u32>>>(py){
            self.check_dimension(m.len())?;
            for i in 0 .. self.inner()?.dimension() {
                self.check_dimension(m[i].len())?;
            }
            let slf = self.inner_mut()?;
            for i in 0 .. slf.dimension() {
                slf.matrix[i].pack(&m[i]);
            }
        } else {
            return Err(python_utils::exception!(TypeError, "value should be a Matrix or a list or tuple of lists or tuples of integers."))
        }
        self.inner_mut()?.calculate_inverse();
        self.check_nonsingular()?;
        Ok(())
    }
    
    // #[args(pyargs="*")]
    // pub fn getitem(self, pyargs) -> PyResult<PyObject> {
        
    // }

    // #[args(pyargs="*")]
    // pub fn setitem(self, pyargs) -> PyResult<PyObject> {
        
    // }

    // pub fn replace_entry(&mut self, row : usize, v : &FpVector) -> PyResult<()> {
    //     let slf = self.inner_mut()?;
    //     let v = v.inner()?;
    //     if v.dimension() != slf.matrix.rows() {
    //         return Err(python_utils::exception!(ValueError, "Wrong dimension"));
    //     }
    //     let mut temp_vec = FpVectorRust::new(slf.prime(), slf.matrix.rows());
    //     slf.apply_inverse(&mut temp_vec, v);
    //     if temp_vec.entry(row) == 0 {
    //         return Err(python_utils::exception!(ValueError, "Singular matrix!"));
    //     }
    //     slf.replace_entry(row, v);
    //     Ok(())
    // }
}

impl Basis { 
    fn handle_index(&self, index : isize) -> PyResult<usize> {
        python_utils::handle_index(self.inner_unchkd().matrix.rows(), index, "the number of rows", "basis")
    }
}

#[pyproto]
impl PySequenceProtocol for Basis {
    fn __len__(self) -> PyResult<usize> {
        Ok(self.inner()?.matrix.rows())
    }

    fn __getitem__(self, index : isize) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        if self.is_mutable(){
            Ok(FpVector::wrap(&mut self.inner_mut()?.matrix[self.handle_index(index)?], self.owner()).into_py(py))
        } else {
            Ok(FpVector::wrap_immutable(&self.inner()?.matrix[self.handle_index(index)?], self.owner()).into_py(py))
        }
    }

    fn __setitem__(mut self, index : isize, value : PyObject) -> PyResult<()>{
        let gil = Python::acquire_gil();
        let py = gil.python();
        if let Ok(vec) = value.extract::<FpVector>(py) {
            self.check_dimension(vec.inner()?.dimension())?;
            self.inner_mut()?.matrix[self.handle_index(index)?].assign(vec.inner()?);
        } else if let Ok(vec) = value.extract::<Vec<u32>>(py) {
            self.check_dimension(vec.len())?;
            self.inner_mut()?.matrix[self.handle_index(index)?].pack(&vec);
        } else {
            return Err(python_utils::exception!(TypeError, "value should be an FpVector or a list or tuple of integers."))
        }
        self.inner_mut()?.calculate_inverse();
        self.check_nonsingular()?;
        Ok(())
    }
}
