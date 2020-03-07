use pyo3::prelude::*;
use pyo3::{PyObjectProtocol, PySequenceProtocol };
use pyo3::exceptions;

// use fp::vector::{FpVector as FpVectorRust, FpVectorT};
use fp::matrix::{ Matrix as MatrixRust, Subspace as SubspaceRust, QuasiInverse as QuasiInverseRust };

use python_utils as util;
use python_utils::{ py_repr, wrapper_type};
use crate::prime::new_valid_prime;
use crate::vector::FpVector;

wrapper_type!(PivotVecWrapper, Vec<isize>);

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
        util::handle_index(self.inner_unchkd().rows(), index, "the number of rows", "matrix")
    }
}

#[pymethods]
impl Matrix {
    #[new]
    fn new(p : u32, rows : usize, columns : usize) -> PyResult<Self> {
        Ok(Self::box_and_wrap(MatrixRust::new(new_valid_prime(p)?, rows, columns)))
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
    // pub fn to_python_matrix(&self) -> PyResult<PyObject> { }
    
    // pub fn from_rows(p : u32, vectors : Vec<FpVector>, columns : usize) -> Self {}
    // pub fn from_vec(p : ValidPrime, input : &[Vec<u32>]) -> Matrix  {}
    // pub fn to_vec(&self) -> Vec<Vec<u32>> {}
    // pub fn augmented_from_vec(p : ValidPrime, input : &[Vec<u32>]) -> (usize, Matrix) {}

    fn set_identity(&mut self, size : usize, row : usize, column : usize) -> PyResult<()> {
        self.inner_mut()?.set_identity(size, row, column);
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

    pub fn row_reduce_into_vec(&mut self, pivots : &mut PivotVecWrapper) -> PyResult<()> {
        self.inner_mut()?.row_reduce(pivots.inner_mut()?);
        Ok(())
    }

    pub fn row_reduce(&mut self) -> PyResult<PyObject> {
        let inner = self.inner_mut()?;
        let mut vec = vec![0; inner.columns()];
        inner.row_reduce(&mut vec);        
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(vec.into_py(py))
    }    

    pub fn row_reduce_offset_into_vec(&mut self, pivots : &mut PivotVecWrapper, offset : usize) -> PyResult<()> {
        self.inner_mut()?.row_reduce_offset(pivots.inner_mut()?, offset);
        Ok(())
    }

    // TODO: What are the right method signatures for these? Do we need a type PermutationWrapper?
    pub fn find_pivots_permutation(&mut self, _permutation : PyObject) -> PyResult<PyObject> {
        Err(exceptions::NotImplementedError::py_err("Not implemented."))
    }

    pub fn row_reduce_permutation(&mut self, _permutation : PyObject) -> PyResult<PyObject> {
        Err(exceptions::NotImplementedError::py_err("Not implemented."))
    }
}

#[pyproto]
impl PySequenceProtocol for Matrix {
    fn __len__(self) -> PyResult<usize> {
        Ok(self.inner()?.rows())
    }

    fn __getitem__(self, index : isize) -> PyResult<FpVector> {
        Ok(FpVector::wrap(&mut self.inner_mut()?[self.handle_index(index)?], self.owner()))
    }
}


wrapper_type!(Subspace, SubspaceRust);


py_repr!(Subspace, "FreedSubspace", {
    Ok(format!(
        "FSubspace (TODO: a repr)",
        // self.inner.prime(),
        // self.inner
    ))
});

#[pymethods]
impl Subspace {
    #[new]
    pub fn new(p : u32, rows : usize, columns : usize) -> PyResult<Self> { 
        Ok(Self::box_and_wrap(SubspaceRust::new(new_valid_prime(p)?, rows, columns)))
    }

    #[staticmethod]
    pub fn subquotient(space : Option<&Subspace>, subspace : Option<&Subspace>, ambient_dimension : usize) -> PyResult<Vec<usize>> { 
        Ok(SubspaceRust::subquotient(
            space.map_or::<PyResult<Option<&SubspaceRust>>,_>(Ok(None), |s| Ok(Some(s.inner()?)))?, 
            subspace.map_or::<PyResult<Option<&SubspaceRust>>,_>(Ok(None), |s| Ok(Some(s.inner()?)))?, 
            ambient_dimension
        ))
    }

    #[staticmethod]
    pub fn entire_space(p : u32, dim : usize) -> PyResult<Self> { 
        let prime = new_valid_prime(p)?;
        Ok(Self::box_and_wrap(SubspaceRust::entire_space(prime, dim)))
    }

    pub fn add_vector(&mut self, row : &FpVector) -> PyResult<()> { 
        self.inner_mut()?.add_vector(row.inner()?);
        Ok(())
    }

    // TODO: Another place where we could wrap Vec<usize>...
    pub fn add_basis_elements(&mut self, _rows : PyObject) -> PyResult<()> { 
        return Err(exceptions::NotImplementedError::py_err(""));
        // let gil = Python::acquire_gil();
        // let py = gil.python();
        // let rows : Vec<usize> = rows.extract(py)?;
        // drop(gil);
        // // Why it no work?
        // self.inner_mut()?.add_basis_elements(rows.iter());
        // Ok(())
    }

    pub fn reduce(&self, vector : &mut FpVector) -> PyResult<()> { 
        self.inner()?.reduce(vector.inner_mut()?);
        Ok(())
    }

    pub fn shift_reduce(&self, vector : &mut FpVector) -> PyResult<()> {
        self.inner()?.reduce(vector.inner_mut()?);
        Ok(())
    }

    pub fn row_reduce(&mut self) -> PyResult<()> {  
        self.inner_mut()?.row_reduce();
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
        Err(exceptions::NotImplementedError::py_err("basis not yet implemented."))
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

    pub fn find_first_row_in_block(&self, pivots : &PivotVecWrapper, first_column_in_block : usize) -> PyResult<usize> {
        Ok(self.inner()?.find_first_row_in_block(pivots.inner()?, first_column_in_block))
    }

    pub fn compute_kernel(&mut self, column_to_pivot_row : &PivotVecWrapper, first_source_column : usize) -> PyResult<Subspace> { 
        Ok(Subspace::box_and_wrap(self.inner_mut()?.compute_kernel(column_to_pivot_row.inner()?, first_source_column)))
    }

    pub fn compute_quasi_inverse(&mut self, pivots : &PivotVecWrapper, last_target_col : usize, first_source_column : usize) -> PyResult<QuasiInverse> {  
        Ok(QuasiInverse::box_and_wrap(self.inner_mut()?.compute_quasi_inverse(pivots.inner()?, last_target_col, first_source_column)))
    }

    pub fn compute_quasi_inverses(
            &mut self, 
            pivots : &PivotVecWrapper,
            first_res_col : usize, 
            last_res_column : usize,  
            first_source_column : usize
    ) -> PyResult<(QuasiInverse, QuasiInverse)> {
        let (qi1,qi2) = self.inner_mut()?.compute_quasi_inverses(pivots.inner()?, first_res_col, last_res_column, first_source_column);
        Ok((QuasiInverse::box_and_wrap(qi1), QuasiInverse::box_and_wrap(qi2)))
    }

    pub fn get_image(&mut self, image_rows : usize, target_dimension : usize, pivots : &PivotVecWrapper) -> PyResult<Subspace> {
        Ok(Subspace::box_and_wrap(self.inner_mut()?.get_image(image_rows, target_dimension, pivots.inner()?)))
    }

   
    pub fn extend_to_surjection(&mut self,
        first_empty_row : usize,
        start_column : usize, end_column : usize,
        current_pivots : &PivotVecWrapper
    ) -> PyResult<Vec<usize>> { 
        Ok(self.inner_mut()?.extend_to_surjection(first_empty_row, start_column, end_column, current_pivots.inner()?))
    }

    pub fn extend_image_to_desired_image(&mut self,
        first_empty_row : usize,
        start_column : usize, end_column : usize,
        current_pivots : &PivotVecWrapper, desired_image : &Subspace
    ) -> PyResult<Vec<usize>> { 
        Ok(self.inner_mut()?.extend_image_to_desired_image(
            first_empty_row, start_column, end_column, current_pivots.inner()?, desired_image.inner()?
        ))
    }

    // TODO: how to create Python optional arguments?

    // pub fn extend_image(&mut self,
    //     first_empty_row : usize,
    //     start_column : usize, end_column : usize,
    //     current_pivots : &PivotVecWrapper, desired_image : &Option<Subspace>
    // ) -> PyResult<Vec<usize>> {  
    //     Ok(self.inner_mut()?.extend_image(
    //         first_empty_row, start_column, end_column, current_pivots.inner()?, 
    //         desired_image.map_or::<PyResult<Option<&SubspaceRust>>,_>(Ok(None), |s| Ok(Some(s.inner()?)))?
    //     ))           
    // }

    pub fn apply(&self, result : &mut FpVector, coeff : u32, input : &FpVector) -> PyResult<()> {
        self.inner()?.apply(result.inner_mut()?, coeff, input.inner()?);
        Ok(())
    }
}
