use pyo3::prelude::*;
use pyo3::{
    PyObjectProtocol, 
    PySequenceProtocol
};

use fp::vector::{
    FpVector as FpVectorRust, 
    FpVectorT,
    // FpVectorIteratorNonzero as FpVectorIteratorNonzeroRust
};


use python_utils;
use crate::prime::new_valid_prime;

python_utils::wrapper_type!(FpVector, FpVectorRust);
// python_utils::wrapper_type!(FpVectorIteratorNonzero, FpVectorIteratorNonzeroRust<'static>);

python_utils::py_repr!(FpVector, "FreedVector", {
    Ok(format!(
        "F{}Vector {}",
        inner.prime(),
        inner
    ))
});


impl FpVector {
    pub fn reduce_coefficient(&self, c : i32) -> u32 {
        python_utils::reduce_coefficient(*self.inner_unchkd().prime(), c)
    }

    pub fn handle_index(&self, index : isize) -> PyResult<usize> {
        python_utils::handle_index(self.inner_unchkd().dimension(), index, "dimension", "vector")
    }

    pub fn check_index(&self, index : isize) -> PyResult<()> {
        python_utils::check_index(self.inner_unchkd().dimension(), index, "dimension", "vector")
    }

    pub fn check_primes_match(&self, other : &FpVector, extra_messsage : &str) -> PyResult<()> {
        let p = self.inner_unchkd().prime();
        let q = other.inner_unchkd().prime(); 
        if p != q {
            return Err(python_utils::exception!(ValueError,
                "Primes {} and {} are not equal{}.", p, q, extra_messsage
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_dimensions_match(&self, other : &FpVector, message : &str) -> PyResult<()> {
        if self.inner_unchkd().dimension() != other.inner_unchkd().dimension() {
            return Err(python_utils::exception!(ValueError,
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

    #[getter]
    pub fn get_prime(&self) -> PyResult<u32> {
        Ok(*self.inner()?.prime())
    }

    #[getter]
    pub fn get_dimension(&self) -> PyResult<usize> {
        Ok(self.inner()?.dimension())
    }

    #[getter]
    pub fn get_offset(&self) -> PyResult<usize> {
        Ok(self.inner()?.offset())
    }

    #[getter]
    pub fn get_min_index(&self) -> PyResult<usize> {
        Ok(self.inner()?.min_index())
    }

    pub fn slice(&self) -> PyResult<(usize, usize)> {
        Ok(self.inner()?.slice())
    }

    pub fn set_slice(&mut self, slice_start : usize, slice_end : usize) -> PyResult<()> {
        if !self.inner()?.is_set_slice_valid(slice_start, slice_end) {
            return Err(python_utils::exception!(IndexError,
                "Slice out of bound!"
            ));
        }
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
        if self.equal(other) {
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
        self.inner_mut()?.set_entry(index, value);
        Ok(())
    }

    pub fn set_entry_unchecked(&mut self, index : usize, value : u32) {
        self.inner_mut_unchkd().set_entry(index, value);
    }

    #[args(c=1)]
    pub fn add_basis_element(&mut self, index : isize, c : i32)  -> PyResult<()> {
        self.check_not_null()?;
        let index = self.handle_index(index)?;
        let c = self.reduce_coefficient(c);
        self.inner_mut()?.add_basis_element(index, c);
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

    // pub fn iter_nonzero(&self) -> PyResult<FpVectorIteratorNonzero> {
    //     let iter = self.inner()?.iter_nonzero();
    //     let mut iter_static : FpVectorIteratorNonzeroRust<'static> = unsafe { std::mem::transmute(iter) };
    //     Ok(FpVectorIteratorNonzero::wrap(&mut iter_static, self.owner()))
    // }
}

// #[pyproto]
// impl PyIterProtocol for FpVectorIteratorNonzero {
//     fn __iter__(slf: PyRef<Self>) -> PyResult<FpVectorIteratorNonzero> {
//         Ok(slf.into())
//     }
//     fn __next__(mut slf: PyRefMut<Self>) -> PyResult<Option<(usize, u32)>> {
//         // Ok(self.inner()?.next())
//     }
// }

fn vec_from_pyobj(p : u32, l : PyObject) -> PyResult<Vec<u32>> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    let vec : Vec<i32> = l.extract(py)?;
    let mut result : Vec<u32> = Vec::with_capacity(vec.len());
    for i in 0..vec.len() {
        result.push(python_utils::reduce_coefficient(p, vec[i]));
    }
    Ok(result)
}

#[pymethods]
impl FpVector {
    #[staticmethod]
    pub fn from_list(p : u32, l : PyObject) -> PyResult<Self> {
        let vec = vec_from_pyobj(p, l)?;
        Ok(Self::box_and_wrap(FpVectorRust::from_vec(new_valid_prime(p)?, &vec)))
    }

    
    pub fn pack(&self, l : PyObject) -> PyResult<()> {
        let inner = self.inner_mut()?;
        let vec = vec_from_pyobj(*inner.prime(), l)?;
        if vec.len() != inner.dimension() {
            return Err(python_utils::exception!(ValueError, 
                "Input list has length {} vector has dimension {}.", vec.len(), inner.dimension()
            ))
        }
        inner.pack(&vec);
        Ok(())
    }

    #[args(c=1)]
    pub fn add(&mut self, other : &Self, c : i32) -> PyResult<()> {
        self.check_not_null()?;
        other.check_not_null()?;
        if self.equal(other) {
            self.scale(c + 1)?;
            return Ok(());
        }
        self.check_primes_match(other, "")?;
        self.check_dimensions_match(other, "Cannot add vectors when dimensions do not match.")?;
        let c =  self.reduce_coefficient(c);
        self.inner_mut()?.add(other.inner()?, c);
        Ok(())
    }

    #[args(coeff = 1, offset = 0)]
    fn add_tensor(&mut self, left : &FpVector, right : &FpVector, coeff : u32, offset : usize) -> PyResult<()> {
        self.check_not_null()?;
        left.check_not_null()?;
        right.check_not_null()?;
        self.check_primes_match(left, "")?;
        self.check_primes_match(right, "")?;
        let left_dim = left.inner()?.dimension();
        let right_dim = right.inner()?.dimension();
        let slf = self.inner_mut()?;
        if left_dim * right_dim + offset > slf.dimension() {
            Err(python_utils::exception!(IndexError,
                "Target needs to be at least {} dimensional to fit tensor but dimension is only {}", 
                left_dim * right_dim + offset, slf.dimension()
            ))
        } else {
            Ok(self.inner_mut()?.add_tensor(offset, coeff, left.inner()?, right.inner()?))
        }
    }

    pub fn add_unchecked(&mut self, other : &FpVector, c : u32) {
        self.inner_mut_unchkd().add(other.inner_unchkd(), c);
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
        self.inner_mut()?.set_entry(index as usize, self.reduce_coefficient(value));
        Ok(())
    }
}



// fn set_entry(&mut self, index : isize, value : i32) -> PyResult<()> {
// self.check_not_null()?;
// let index = self.handle_index(index)?;
// let value = self.reduce_coefficient(value);
// self.inner_mut()?.set_entry(index, value);
// Ok(())