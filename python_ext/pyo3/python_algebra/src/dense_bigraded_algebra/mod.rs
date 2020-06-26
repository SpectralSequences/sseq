use pyo3::{
    prelude::*,
    PyObjectProtocol,
    types::PyDict
};

use python_utils::{
    self,
    py_repr, 
    rc_wrapper_type, 
    // wrapper_type,
    get_from_kwargs
};

use fp::vector::{FpVector as FpVectorRust, FpVectorT};
use python_fp::prime::new_valid_prime;
use python_fp::vector::FpVector;
use python_fp::basis::Basis;
use python_fp::matrix::Matrix;

use algebra::dense_bigraded_algebra::DenseBigradedAlgebra as DenseBigradedAlgebraRust;
rc_wrapper_type!(DenseBigradedAlgebra, DenseBigradedAlgebraRust);


py_repr!(DenseBigradedAlgebra, "FreedDenseBigradedAlgebra", {
    Ok(format!(
        "DenseBigradedAlgebra(p={})",
        inner.prime(),
    ))
});

impl DenseBigradedAlgebra {
    fn check_prime(&self, p : u32) -> PyResult<()> {
        if p != *self.inner_unchkd().prime() {
            Err(python_utils::exception!(ValueError,
                "Prime {} does not match DenseBigradedAlgebra prime {}.", 
                p, *self.inner_unchkd().prime()
            ))
        } else {
            Ok(())
        }
    }

    fn check_degree(&self, x : i32, y : i32) -> PyResult<()> {
        let max_x = self.inner_unchkd().max_x();
        let max_y = self.inner_unchkd().max_y();
        if x >= max_x {
            return Err(python_utils::exception!(IndexError,
                "Cannot find dimension in bidegree ({x}, {y}) because x is too large: maximum x is {max_x}.", 
                x=x, y=y, max_x=max_x
            ));
        }
        if y >= max_y {
            return Err(python_utils::exception!(IndexError,
                "Cannot find dimension in bidegree ({x}, {y}) because y is too large: maximum y is {max_y}.", 
                x=x, y=y, max_y=max_y
            ))
        } 
        Ok(())
    }

    fn check_dimension(&self, x : i32, y : i32, vec : &FpVector) -> PyResult<()> {
        let what_the_dimension_should_be = self.inner_unchkd().dimension(x, y);
        let the_dimension = vec.get_dimension()?;
        if the_dimension <= what_the_dimension_should_be {
            Ok(())
        } else {
            Err(python_utils::exception!(IndexError,
                "Dimension of vector is {} but the dimension of the algebra in bidegree ({}, {}) is {}.",
                the_dimension,
                x, y,
                what_the_dimension_should_be
            ))
        }
    }

    pub fn check_index(&self, x : i32, y : i32, idx : usize) -> PyResult<()> {
        let dimension = self.inner_unchkd().dimension(x, y);
        if idx < dimension {
            Ok(())
        } else {
            Err(python_utils::exception!(IndexError,
                "Index {} is larger than dimension {} of the algebra in bidegree ({}, {}).",
                idx,
                dimension,
                x, y
            ))
        }    
    }    
}

#[pymethods]
impl DenseBigradedAlgebra {
    #[new]
    #[args(min_x=0, min_y=0)]
    fn new(p : u32, dimensions : PyObject, min_x : i32, min_y : i32) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let dims : Vec<Vec<usize>> = dimensions.extract(py)?;
        let algebra = DenseBigradedAlgebraRust::from_dimension_vec(new_valid_prime(p)?, min_x, min_y, &dims);
        let mut result = Self::box_and_wrap(algebra);
        result.freeze().unwrap_or_else(|_err| unreachable!());
        Ok(result)
    }


    #[getter]
    pub fn get_prime(&self) -> PyResult<u32> {
        Ok(*self.inner()?.prime())
    }

    pub fn dimension(&self, x : i32, y : i32) -> PyResult<usize> {
        self.check_not_null()?;
        self.check_degree(x, y)?;
        Ok(self.inner_unchkd().dimension(x, y))
    }

    pub fn basis(&self, x : i32, y : i32) -> PyResult<Basis> {
        self.check_not_null()?;
        self.check_degree(x, y)?;
        let result = self.inner_unchkd().data[x][y].read().unwrap().basis.clone();
        Ok(Basis::box_and_wrap(result))
    }

    pub fn names(&self, x : i32, y : i32) -> PyResult<Vec<String>> {
        self.check_not_null()?;
        self.check_degree(x, y)?;        
        Ok(self.inner_unchkd().data[x][y].read().unwrap()
            .names.iter()
            .map(|n| n.clone().unwrap_or("".to_string()))
            .collect()
        )
    }

    pub fn indecomposables(&self, x : i32, y : i32) -> PyResult<Vec<usize>> {
        self.check_not_null()?;
        self.check_degree(x, y)?;          
        Ok(self.inner_unchkd().data[x][y].read().unwrap()
            .decomposables.matrix.pivots()
            .iter().enumerate()
            .filter_map(|(idx, &r)| if r < 0 { Some(idx) } else { None })
            .collect())
    }

    pub fn indecomposable_decompositions(&self, x : i32, y : i32) -> PyResult<Vec<(FpVector, Vec<(String, i32)>)>> {
        self.check_not_null()?;
        self.check_degree(x, y)?;          
        let data = self.inner_unchkd().data[x][y].read().unwrap();
        let result : Result<_,()> = data.indecomposable_decompositions.iter().map(|(vec,mono)| 
            Ok((FpVector::box_and_wrap(vec.clone()),self.inner_unchkd().monomial_to_string_pairs(mono)?))
        ).collect();
        result.map_err(|e| python_utils::exception!(ValueError, "Invalid indecomposable in monomial"))
    }

    pub fn set_product(&self, 
        left_x : i32, left_y : i32, left_idx : usize, 
        right_x : i32, right_y : i32, right_idx : usize, 
        output : FpVector
    ) -> PyResult<()> {
        self.check_not_null()?;
        self.check_degree(left_x + right_x, left_y + right_y)?;
        self.check_index(left_x, left_y, left_idx)?;
        self.check_index(right_x, right_y, right_idx)?;
        self.check_dimension(left_x + right_x, left_y + right_y, &output)?;
        let out = output.inner()?;
        self.check_prime(*out.prime())?;
        self.inner_unchkd().set_product(left_x, left_y, left_idx, right_x, right_y, right_idx, out.clone());
        Ok(())
    }    

    pub fn set_basis(&self, x : i32, y : i32, basis : Matrix) -> PyResult<()> {
        self.check_not_null()?;
        self.check_degree(x, y)?;          
        self.inner_unchkd().set_basis(x, y, basis.inner()?);
        Ok(())
    }

    pub fn set_name(&self, x : i32, y : i32, idx : usize, name : String) -> PyResult<()> {
        self.check_not_null()?;
        self.check_degree(x, y)?;
        self.check_index(x, y, idx)?;
        self.inner_unchkd().set_name(x, y, idx, name);
        Ok(())
    }

    pub fn multiply_element_by_element(&self, 
        result : &mut FpVector, coeff : u32, 
        left_degree : (i32, i32), left_vec : &FpVector, 
        right_degree : (i32, i32), right_vec : &FpVector, 
    ) -> PyResult<()> {
        let (left_x, left_y) = left_degree;
        let (right_x, right_y) = right_degree;
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        self.check_not_null()?;
        self.check_degree(out_x, out_y)?;
        self.check_dimension(left_x, left_y, left_vec)?;
        self.check_dimension(right_x, right_y, right_vec)?;
        self.check_dimension(out_x, out_y, result)?;
        let p = self.inner_unchkd().prime();
        let mut sv_left = FpVectorRust::new(p, 0);
        let mut sv_right = FpVectorRust::new(p, 0);
        let mut sv_out = FpVectorRust::new(p, 0);        
        self.inner_unchkd().multiply_element_by_element(
            result.inner_mut()?, coeff, 
            left_x, left_y, left_vec.inner()?,
            right_x, right_y, right_vec.inner()?,
            &mut sv_left, &mut sv_right, &mut sv_out,
        ).map_err(|_| python_utils::exception!(RuntimeError, "Missing product."))?;
        Ok(())
    }

    pub fn multiply_element_by_basis_element(&self, 
        result : &mut FpVector, coeff : u32, 
        left_degree : (i32, i32), left_vec : &FpVector, 
        right_degree : (i32, i32), right_idx : usize, 
    ) -> PyResult<()> {
        let (left_x, left_y) = left_degree;
        let (right_x, right_y) = right_degree;
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        self.check_not_null()?;
        self.check_degree(out_x, out_y)?;
        self.check_dimension(left_x, left_y, left_vec)?;
        self.check_index(right_x, right_y, right_idx)?;
        self.check_dimension(out_x, out_y, result)?;
        let p = self.inner_unchkd().prime();
        let mut sv_left = FpVectorRust::new(p, 0);
        let mut sv_right = FpVectorRust::new(p, 0);
        let mut sv_out = FpVectorRust::new(p, 0);        
        self.inner_unchkd().multiply_element_by_basis_element(
            result.inner_mut()?, coeff, 
            left_x, left_y, left_vec.inner()?,
            right_x, right_y, right_idx,
            &mut sv_left, &mut sv_right, &mut sv_out,
        ).map_err(|_| python_utils::exception!(RuntimeError, "Missing product."))?;
        Ok(())
    }


    pub fn multiply_basis_element_by_element(&self, 
        result : &mut FpVector, coeff : u32, 
        left_degree : (i32, i32), left_idx : usize, 
        right_degree : (i32, i32), right_vec : &FpVector, 
    ) -> PyResult<()> {
        let (left_x, left_y) = left_degree;
        let (right_x, right_y) = right_degree;
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        self.check_not_null()?;
        self.check_degree(out_x, out_y)?;
        self.check_index(left_x, left_y, left_idx)?;
        self.check_dimension(right_x, right_y, right_vec)?;
        self.check_dimension(out_x, out_y, result)?;
        let p = self.inner_unchkd().prime();
        let mut sv_left = FpVectorRust::new(p, 0);
        let mut sv_right = FpVectorRust::new(p, 0);
        let mut sv_out = FpVectorRust::new(p, 0);        
        self.inner_unchkd().multiply_basis_element_by_element(
            result.inner_mut()?, coeff, 
            left_x, left_y, left_idx,
            right_x, right_y, right_vec.inner()?,
            &mut sv_left, &mut sv_right, &mut sv_out,
        ).map_err(|_| python_utils::exception!(RuntimeError, "Missing product."))?;
        Ok(())
    }    
   
    pub fn multiply_basis_element_by_basis_element(&self, 
        result : &mut FpVector, coeff : u32, 
        left_degree : (i32, i32), left_idx : usize, 
        right_degree : (i32, i32), right_idx : usize, 
    ) -> PyResult<()> {
        let (left_x, left_y) = left_degree;
        let (right_x, right_y) = right_degree;
        let out_x = left_x + right_x;
        let out_y = left_y + right_y;
        self.check_not_null()?;
        self.check_degree(out_x, out_y)?;
        self.check_index(left_x, left_y, left_idx)?;
        self.check_index(right_x, right_y, right_idx)?;
        self.check_dimension(out_x, out_y, result)?;
        let p = self.inner_unchkd().prime();
        let mut sv_left = FpVectorRust::new(p, 0);
        let mut sv_right = FpVectorRust::new(p, 0);
        let mut sv_out = FpVectorRust::new(p, 0);        
        self.inner_unchkd().multiply_basis_element_by_basis_element(
            result.inner_mut()?, coeff, 
            left_x, left_y, left_idx,
            right_x, right_y, right_idx,
            &mut sv_left, &mut sv_right, &mut sv_out,
        ).map_err(|_| python_utils::exception!(RuntimeError, "Missing product."))?;
        Ok(())
    }

    pub fn compute_indecomposables_in_bidegree(&self, x : i32, y : i32) -> PyResult<()> {
        self.check_not_null()?;
        self.check_degree(x, y)?;
        let p = self.inner_unchkd().prime();
        let mut sv = FpVectorRust::new(p, 0);
        self.inner_unchkd().compute_indecomposables_in_bidegree(x, y, &mut sv);
        Ok(())
    }

    pub fn compute_all_indecomposables(&self) -> PyResult<()> {
        self.check_not_null()?;
        let inner = self.inner_unchkd();
        let p = inner.prime();
        let mut sv = FpVectorRust::new(p, 0);
        for x in inner.min_x .. inner.max_x() {
            for y in inner.min_y .. inner.max_y() {
                drop(inner.compute_indecomposables_in_bidegree(x, y, &mut sv));
                    // .map_err(|_| python_utils::exception!(RuntimeError, "Missing product."))?;
            }
        }
        Ok(())
    }    

}