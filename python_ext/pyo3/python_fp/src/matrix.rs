use pyo3::{
    prelude::*,
    PyObjectProtocol, 
    PySequenceProtocol,
    types::{ PyTuple, PyList }
};

// use fp::vector::{FpVector as FpVectorRust, FpVectorT};
use fp::matrix::{ 
    Matrix as MatrixRust, 
    Subspace as SubspaceRust, 
    QuasiInverse as QuasiInverseRust 
};

use python_utils::{ 
    self,
    py_repr, 
    wrapper_type
};
use crate::prime::new_valid_prime;
use crate::vector::FpVector;

wrapper_type!(PivotVecWrapper, Vec<isize>);
#[pyproto]
impl PySequenceProtocol for PivotVecWrapper {
    fn __len__(self) -> PyResult<usize> {
        Ok(self.inner()?.len())
    }

    fn __getitem__(self, index : isize) -> PyResult<isize> {
        self.check_not_null()?;
        python_utils::check_index(self.inner_unchkd().len(), index, "length", "pivot vector")?;
        Ok(self.inner_unchkd()[index as usize])
    }
}


#[pymethods]
impl PivotVecWrapper {
    #[staticmethod]
    pub fn from_py(pivots : PyObject) -> PyResult<PivotVecWrapper> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let vec : Vec<isize> = pivots.extract(py)?;
        Ok(PivotVecWrapper::box_and_wrap(vec))
    }

    pub fn to_py(&self) -> PyResult<PyObject> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        // Is this clone necessary? Does into_py need to own the result?
        let vec = self.inner()?.clone();
        Ok(vec.into_py(py))        
    }
}

py_repr!(PivotVecWrapper, "FreedPivotVector", {
    Ok(format!(
        "PivotVector {:?}",
        inner
    ))
});

wrapper_type!(Matrix, MatrixRust);

py_repr!(Matrix, "FreedMatrix", {
    Ok(format!(
        "F{}Matrix {}",
        inner.prime(),
        inner
    ))
});


impl Matrix { 
    fn handle_index(&self, index : isize) -> PyResult<usize> {
        python_utils::handle_index(self.inner_unchkd().rows(), index, "the number of rows", "matrix")
    }
}

#[pymethods]
impl Matrix {
    #[new]
    pub fn new(p : u32, rows : usize, columns : usize) -> PyResult<Self> {
        Ok(Self::box_and_wrap(MatrixRust::new(new_valid_prime(p)?, rows, columns)))
    }

    pub fn initialize_pivots(&mut self) -> PyResult<()> {
        self.inner_mut()?.initialize_pivots();
        Ok(())
    }

    pub fn row(&mut self, i : isize) -> PyResult<FpVector> {
        Ok(FpVector::wrap(&mut self.inner_mut()?[self.handle_index(i)?], self.owner()))
    }

    #[staticmethod]
    pub fn from_python_matrix(p : u32, l : PyObject) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let vec : Vec<Vec<u32>> = l.extract(py)?;
        Ok(Self::box_and_wrap(MatrixRust::from_vec(new_valid_prime(p)?, &vec)))
    }

    // TODO:
    pub fn to_python_matrix(&self) -> PyResult<PyObject> { 
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(PyObject::from(PyList::new(py, self.inner()?.iter().map(|r| PyList::new(py, r.iter())))))
    }
    
    // pub fn from_rows(p : u32, vectors : Vec<FpVector>, columns : usize) -> Self {}
    // pub fn from_vec(p : ValidPrime, input : &[Vec<u32>]) -> Matrix  {}
    // pub fn to_vec(&self) -> Vec<Vec<u32>> {}
    // pub fn augmented_from_vec(p : ValidPrime, input : &[Vec<u32>]) -> (usize, Matrix) {}

    fn add_identity(&mut self, size : usize, row : usize, column : usize) -> PyResult<()> {
        self.check_not_null()?;
        let rows = self.rows()?;
        let cols = self.columns()?;
        if row + size > rows {
            return Err(python_utils::exception!(IndexError,
                "Matrix has only {} rows but needs at least {} rows for desired operation.",
                rows, row + size
            ));
        }
        if column + size > cols {
            return Err(python_utils::exception!(IndexError,
                "Matrix has only {} columns but needs at least {} columns for desired operation.",
                cols, column + size
            ));
        }        
        self.inner_mut()?.add_identity(size, row, column);
        Ok(())
    }

    pub fn prime(&self) -> PyResult<u32> {
        Ok(*self.inner()?.prime())
    }

    pub fn rows(&self) -> PyResult<usize> {
        Ok(self.inner()?.rows())
    }

    pub fn columns(&self) -> PyResult<usize> {
        Ok(self.inner()?.columns())
    }

    pub fn set_slice(&mut self, row_start : usize, row_end : usize, col_start : usize, col_end : usize) -> PyResult<()> {
        self.inner_mut()?.set_slice(row_start, row_end, col_start, col_end);
        Ok(())
    }

    pub fn clear_slice(&mut self) -> PyResult<()> {
        self.inner_mut()?.clear_slice();
        Ok(())
    }

    pub fn set_row_slice(&mut self, row_start: usize, row_end: usize) -> PyResult<()> {
        self.inner_mut()?.set_row_slice(row_start, row_end);
        Ok(())
    }

    pub fn clear_row_slice(&mut self) -> PyResult<()> {
        self.inner_mut()?.clear_row_slice();
        Ok(())
    }

    // pub fn into_slice(mut self) -> Self {}
    // pub fn into_vec(self) -> Vec<FpVector> {}


    pub fn swap_rows(&mut self, i : usize, j : usize) -> PyResult<()> {
        self.inner_mut()?.swap_rows(i, j);
        Ok(())
    }

    pub fn row_reduce(&mut self) -> PyResult<()> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let self_inner = self.inner_mut()?;
        py.allow_threads(move || -> PyResult<()> {
            self_inner.row_reduce();
            Ok(())
        })
    }

    // TODO: What are the right method signatures for these? Do we need a type PermutationWrapper?
    pub fn find_pivots_permutation(&mut self, _permutation : PyObject) -> PyResult<PyObject> {
        Err(python_utils::not_implemented_error!())
    }

    pub fn row_reduce_permutation(&mut self, _permutation : PyObject) -> PyResult<PyObject> {
        Err(python_utils::not_implemented_error!())
    }
}

#[pyproto]
impl PySequenceProtocol for Matrix {
    fn __len__(self) -> PyResult<usize> {
        Ok(self.inner()?.rows())
    }

    fn __getitem__(self, index : isize) -> PyResult<FpVector> {
        if self.is_mutable(){
            Ok(FpVector::wrap(&mut self.inner_mut()?[self.handle_index(index)?], self.owner()))
        } else {
            Ok(FpVector::wrap_immutable(&self.inner()?[self.handle_index(index)?], self.owner()))
        }
    }
}


wrapper_type!(Subspace, SubspaceRust);


py_repr!(Subspace, "FreedSubspace", {
    Ok(format!(
        "F{}Subspace {}",
        inner.prime(),
        inner.matrix
    ))
});

#[pymethods]
impl Subspace {
    #[new]
    pub fn new(p : u32, rows : usize, columns : usize) -> PyResult<Self> { 
        Ok(Self::box_and_wrap(SubspaceRust::new(new_valid_prime(p)?, rows, columns)))
    }

    pub fn matrix(&self) -> PyResult<Matrix> {
        Ok(Matrix::wrap_immutable(&self.inner()?.matrix, self.owner()))
    }

    // #[staticmethod]
    // pub fn subquotient(space : Option<&Subspace>, subspace : Option<&Subspace>, ambient_dimension : usize) -> PyResult<Vec<usize>> { 
    //     Ok(SubspaceRust::subquotient(
    //         space.map_or::<PyResult<Option<&SubspaceRust>>,_>(Ok(None), |s| Ok(Some(s.inner()?)))?, 
    //         subspace.map_or::<PyResult<Option<&SubspaceRust>>,_>(Ok(None), |s| Ok(Some(s.inner()?)))?, 
    //         ambient_dimension
    //     ))
    // }

    #[staticmethod]
    pub fn entire_space(p : u32, dim : usize) -> PyResult<Self> { 
        let prime = new_valid_prime(p)?;
        Ok(Self::box_and_wrap(SubspaceRust::entire_space(prime, dim)))
    }


    pub fn reduce(&self, vector : &mut FpVector) -> PyResult<()> { 
        self.inner()?.reduce(vector.inner_mut()?);
        Ok(())
    }

    pub fn shift_reduce(&self, vector : &mut FpVector) -> PyResult<()> {
        self.inner()?.reduce(vector.inner_mut()?);
        Ok(())
    }

    pub fn contains(&self, vector : &FpVector) -> PyResult<bool> { 
        Ok(self.inner()?.contains(vector.inner()?))
    }

    pub fn dimension(&self) -> PyResult<usize> { 
        Ok(self.inner()?.dimension())
    }

    // TODO: basis is supposed to return a read only view.
    // Do we copy? Make a read only vector type? Return a mutable view?
    pub fn basis(&self) -> PyResult<PyObject> { 
        Err(python_utils::not_implemented_error!())
    }

    pub fn add_vector(&mut self, row : &FpVector) -> PyResult<()> { 
        self.inner_mut()?.add_vector(row.inner()?);
        Ok(())
    }

    // TODO: Another place where we could wrap Vec<usize>...
    pub fn add_basis_elements(&mut self, _rows : PyObject) -> PyResult<()> { 
        return Err(python_utils::not_implemented_error!());
        // let gil = Python::acquire_gil();
        // let py = gil.python();
        // let rows : Vec<usize> = rows.extract(py)?;
        // drop(gil);
        // // Why it no work?
        // self.inner_mut()?.add_basis_elements(rows.iter());
        // Ok(())
    }


    pub fn row_reduce(&mut self) -> PyResult<()> {  
        let self_inner = self.inner_mut()?;
        python_utils::release_gil!(self_inner.row_reduce());
        Ok(())
    }    

    pub fn set_to_zero(&mut self) -> PyResult<()> {
        Ok(self.inner_mut()?.set_to_zero())
    }

    pub fn set_to_entire(&mut self) -> PyResult<()> {
        Ok(self.inner_mut()?.set_to_entire())
    }
}

wrapper_type!(QuasiInverse, QuasiInverseRust);

py_repr!(QuasiInverse, "FreedQuasiInverse", {
    Ok(format!(
        "F{}QuasiInverse {:?}",
        inner.prime(),
        inner
    ))
});

#[pymethods]
impl QuasiInverse {
    pub fn prime(&self) -> PyResult<u32> { 
        Ok(*self.inner()?.prime())
    }

    pub fn apply(&self, target : &mut FpVector, coeff : u32, input : &FpVector) -> PyResult<()> {
        self.inner()?.apply(target.inner_mut()?, coeff, input.inner_mut()?);
        Ok(())
    }
}

#[pymethods]
impl Matrix {
    pub fn set_to_zero(&mut self) -> PyResult<()> { 
        self.inner_mut()?.set_to_zero();
        Ok(())
    }

    pub fn pivots(&self) -> PyResult<PivotVecWrapper> {
        Ok(PivotVecWrapper::wrap_immutable(self.inner()?.pivots(), self.owner()))
    }

    pub fn find_first_row_in_block(&self, first_column_in_block : usize) -> PyResult<usize> {
        Ok(self.inner()?.find_first_row_in_block(first_column_in_block))
    }

    pub fn compute_kernel(&mut self, first_source_column : usize) -> PyResult<Subspace> { 
        Ok(Subspace::box_and_wrap(self.inner_mut()?.compute_kernel(first_source_column)))
    }

    pub fn compute_quasi_inverse(&mut self,  last_target_col : usize, first_source_column : usize) -> PyResult<QuasiInverse> {  
        Ok(QuasiInverse::box_and_wrap(self.inner_mut()?.compute_quasi_inverse(last_target_col, first_source_column)))
    }

    pub fn compute_quasi_inverses(
            &mut self, 
            first_res_col : usize, 
            last_res_column : usize,  
            first_source_column : usize
    ) -> PyResult<(QuasiInverse, QuasiInverse)> {
        let (qi1,qi2) = self.inner_mut()?.compute_quasi_inverses(first_res_col, last_res_column, first_source_column);
        Ok((QuasiInverse::box_and_wrap(qi1), QuasiInverse::box_and_wrap(qi2)))
    }

    pub fn get_image(&mut self, image_rows : usize, target_dimension : usize, pivots : &PivotVecWrapper) -> PyResult<Subspace> {
        let self_inner = self.inner_mut()?;
        let pivots_inner = pivots.inner()?;
        Ok(Subspace::box_and_wrap(python_utils::release_gil!(
            self_inner.get_image(image_rows, target_dimension, pivots_inner)
        )))
    }

   
    pub fn extend_to_surjection(&mut self,
        first_empty_row : usize,
        start_column : usize, end_column : usize,
    ) -> PyResult<Vec<usize>> { 
        let self_inner = self.inner_mut()?;
        Ok(python_utils::release_gil!(
            self_inner.extend_to_surjection(first_empty_row, start_column, end_column)
        ))
    }

    pub fn extend_image_to_desired_image(&mut self,
        first_empty_row : usize,
        start_column : usize, end_column : usize,
        desired_image : &Subspace
    ) -> PyResult<Vec<usize>> { 
        let self_inner = self.inner_mut()?;
        let desired_image_inner = desired_image.inner()?;
        Ok(python_utils::release_gil!( 
            self_inner.extend_image_to_desired_image(
                first_empty_row, start_column, end_column, desired_image_inner
            )
        ))
    }

    #[args(pyargs="*")]
    pub fn extend_image(&mut self,
        first_empty_row : usize,
        start_column : usize, end_column : usize,
        pyargs : &PyTuple
    ) -> PyResult<Vec<usize>> {  
        python_utils::check_number_of_positional_arguments!("extend_image", 5, 6, 5 + pyargs.len())?;
        let self_inner = self.inner_mut()?;
        let desired_image : Option<&SubspaceRust> = 
            if pyargs.is_empty() {
                None
            } else {
                Some(
                    pyargs.get_item(0)
                          .extract::<&Subspace>()?
                          .inner()?
                )
            };

        Ok(python_utils::release_gil!(
            self_inner.extend_image(
                first_empty_row, start_column, end_column,  
                desired_image
            )
        ))
    }

    pub fn apply(&self, result : &mut FpVector, coeff : u32, input : &FpVector) -> PyResult<()> {
        self.inner()?.apply(result.inner_mut()?, coeff, input.inner()?);
        Ok(())
    }
}
