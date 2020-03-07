use pyo3::prelude::*;
use pyo3::{PyObjectProtocol, PySequenceProtocol };
use pyo3::exceptions;

use fp::vector::{FpVector as FpVectorRust, FpVectorT};


use python_utils;
use crate::prime::new_valid_prime;

python_utils::wrapper_type!(FpVector, FpVectorRust);

python_utils::py_repr!(FpVector, "FreedVector", {
    Ok(format!(
        "F{}Vector {}",
        inner.prime(),
        inner
    ))
});


impl FpVector {
    fn reduce_coefficient(&self, c : i32) -> u32 {
        python_utils::reduce_coefficient(*self.inner_unchkd().prime(), c)
    }

    fn handle_index(&self, index : isize) -> PyResult<usize> {
        python_utils::handle_index(self.inner_unchkd().dimension(), index, "dimension", "vector")
    }

    fn check_index(&self, index : isize) -> PyResult<()> {
        python_utils::check_index(self.inner_unchkd().dimension(), index, "dimension", "vector")
    }

    pub fn check_primes_match(&self, other : &FpVector, extra_messsage : &str) -> PyResult<()> {
        let p = self.inner_unchkd().prime();
        let q = other.inner_unchkd().prime(); 
        if p != q {
            return Err(exceptions::ValueError::py_err(
                format!("Primes {} and {} are not equal{}.", p, q, extra_messsage)
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_dimensions_match(&self, other : &FpVector, message : &str) -> PyResult<()> {
        if self.inner_unchkd().dimension() != other.inner_unchkd().dimension() {
            return Err(exceptions::ValueError::py_err(
                format!("{}", message)
            ))
        } else {
            Ok(())
        }
    }
}

#[pymethods]
impl FpVector {
    #[new]
    pub fn new(p: u32, dimension : usize) -> PyResult<Self> {
        Ok(FpVector::box_and_wrap(FpVectorRust::new(new_valid_prime(p)?, dimension)))
    }

    pub fn prime(&self) -> PyResult<u32> {
        Ok(*self.inner()?.prime())
    }

    pub fn dimension(&self) -> PyResult<usize> {
        Ok(self.inner()?.dimension())
    }

    pub fn offset(&self) -> PyResult<usize> {
        Ok(self.inner()?.offset())
    }

    pub fn min_index(&self) -> PyResult<usize> {
        Ok(self.inner()?.min_index())
    }

    pub fn slice(&self) -> PyResult<(usize, usize)> {
        Ok(self.inner()?.slice())
    }

    pub fn set_slice(&mut self, slice_start : usize, slice_end : usize) -> PyResult<()> {
        // TODO: needs error handling
        self.inner_mut()?.set_slice(slice_start, slice_end);
        Ok(())
    }
    
    pub fn restore_slice(&mut self, slice : (usize, usize)) -> PyResult<()> {
        // TODO: needs error handling
        self.inner_mut()?.restore_slice(slice);
        Ok(())
    }

    pub fn clear_slice(&mut self) -> PyResult<()> {
        self.inner_mut()?.clear_slice();
        Ok(())
    }

    pub fn into_slice(&mut self) -> PyResult<()>{
        self.inner_mut()?.into_slice();
        Ok(())
    }

    //min_limb, max_limb, limbs, limbs_mut, limb_mask,  

    pub fn set_to_zero_pure(&mut self) -> PyResult<()> {
        self.inner_mut()?.set_to_zero_pure();
        Ok(())
    }
    
    pub fn set_to_zero(&mut self) -> PyResult<()> {
        self.inner_mut()?.set_to_zero();
        Ok(())
    }

    pub fn assign(&mut self, other : &FpVector) -> PyResult<()> {
        self.check_not_null()?;
        other.check_not_null()?;
        if self.inner == other.inner {
            return Ok(());
        }
        self.check_primes_match(other, "")?;
        self.check_dimensions_match(other, "Cannot assign vectors when dimensions do not match.")?;
        self.inner_mut()?.shift_assign(other.inner()?);
        Ok(())
    }

    pub fn assign_unchecked(&mut self, other : &FpVector) {
        self.inner_mut_unchkd().assign(other.inner_unchkd());
    }

    pub fn shift_assign_unchecked(&mut self, other : &FpVector) {
        self.inner_mut_unchkd().shift_assign(other.inner_unchkd());
    }    

    pub fn is_zero_pure(&self) -> PyResult<bool> {
        Ok(self.inner()?.is_zero_pure())
    }

    pub fn is_zero(&self) -> PyResult<bool> {
        Ok(self.inner()?.is_zero())
    }

    pub fn entry(&self, index : isize) -> PyResult<u32> {
        self.check_not_null()?;
        let index = self.handle_index(index)?;
        Ok(self.inner_unchkd().entry(index))
    }

    pub fn entry_unchecked(&self, index : usize) -> u32 {
        self.inner_unchkd().entry(index)
    }

    pub fn set_entry(&mut self, index : isize, value : i32) -> PyResult<()> {
        self.check_not_null()?;
        let index = self.handle_index(index)?;
        let value = self.reduce_coefficient(value);
        self.inner_mut_unchkd().set_entry(index, value);
        Ok(())
    }

    pub fn set_entry_unchecked(&mut self, index : usize, value : u32) {
        self.inner_mut_unchkd().set_entry(index, value);
    }

    pub fn add_basis_element(&mut self, index : isize, c : i32)  -> PyResult<()> {
        self.check_not_null()?;
        let index = self.handle_index(index)?;
        let c = self.reduce_coefficient(c);
        self.inner_mut_unchkd().add_basis_element(index, c);
        Ok(())
    }

    pub fn add_basis_element_unchecked(&mut self, index : usize, c : u32) {
        self.inner_mut_unchkd().add_basis_element(index, c);
    }

    // unpack?

    pub fn to_list(&self) -> PyResult<PyObject> {
        let inner = self.inner_mut()?;
        let mut vec = vec![0; inner.dimension()];
        inner.unpack(&mut vec);
        let gil = Python::acquire_gil();
        let py = gil.python();
        Ok(vec.into_py(py))
    }

    #[staticmethod]
    pub fn from_list(p : u32, l : PyObject) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let mut vec : Vec<u32> = l.extract(py)?;
        for i in 0..vec.len() {
            vec[i] = ((vec[i] % p) + p) % p;
        }
        Ok(FpVector::box_and_wrap(FpVectorRust::from_vec(new_valid_prime(p)?, &vec)))
    }

    pub fn add(&mut self, other : &FpVector, c : i32) -> PyResult<()> {
        self.check_not_null()?;
        other.check_not_null()?;
        if self.inner == other.inner {
            self.scale(c + 1)?;
            return Ok(());
        }        
        self.check_primes_match(other, "")?;
        self.check_dimensions_match(other, "Cannot add vectors when dimensions do not match.")?;
        let c =  self.reduce_coefficient(c);
        self.inner_mut_unchkd().shift_add(other.inner()?, c);
        Ok(())
    }

    pub fn add_unchecked(&mut self, other : &FpVector, c : u32) {
        self.inner_mut_unchkd().add(other.inner_unchkd(), c);
    }

    pub fn shift_add_unchecked(&mut self, other : &FpVector, c : u32) {
        self.inner_mut_unchkd().shift_add(other.inner_unchkd(), c);
    }

    pub fn scale(&mut self, c : i32) -> PyResult<()> {
        let c = self.reduce_coefficient(c);
        self.inner_mut()?.scale(c);
        Ok(())
    }

    pub fn scale_unchecked(&mut self, c : u32) {
        self.inner_mut_unchkd().scale(c);
    }

    #[staticmethod]
    pub fn number_of_limbs(p : u32, dimension : usize) -> PyResult<usize> {
        Ok(FpVectorRust::number_of_limbs(new_valid_prime(p)?, dimension))
    }

    #[staticmethod]
    pub fn padded_dimension(p : u32, dimension : usize) -> PyResult<usize> {
        Ok(FpVectorRust::padded_dimension(new_valid_prime(p)?, dimension))
    }

    pub fn set_scratch_vector_size(&mut self, dimension : usize) -> PyResult<()> {
        self.inner_mut()?.set_scratch_vector_size(dimension);
        Ok(())  
    }
}

#[pyproto]
impl PySequenceProtocol for FpVector {
    fn __len__(self) -> PyResult<usize> {
        Ok(self.inner()?.dimension())
    }

    fn __getitem__(self, index : isize) -> PyResult<u32> {
        self.check_not_null()?;
        self.check_index(index)?;
        Ok(self.inner_unchkd().entry(index as usize))
    }

    fn __setitem__(mut self, index : isize, value : i32) -> PyResult<()> {
        self.check_not_null()?;
        self.check_index(index)?;
        self.inner_mut_unchkd().set_entry(index as usize, self.reduce_coefficient(value));
        Ok(())
    }
}



// fn set_entry(&mut self, index : isize, value : i32) -> PyResult<()> {
// self.check_not_null()?;
// let index = self.handle_index(index)?;
// let value = self.reduce_coefficient(value);
// self.inner_mut_unchkd().set_entry(index, value);
// Ok(())