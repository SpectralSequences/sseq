use pyo3::prelude::*;
use fp::prime::ValidPrime;
use pyo3::exceptions::ValueError;

pub fn new_valid_prime(p: u32) -> PyResult<ValidPrime> {
    let result = ValidPrime::try_new(p)
        .ok_or(
            PyErr::new::<ValueError, _>(
                format!("First argument {} is not a valid prime.", p)
            )
        )?;
    fp::vector::initialize_limb_bit_index_table(result);
    Ok(result)
}
