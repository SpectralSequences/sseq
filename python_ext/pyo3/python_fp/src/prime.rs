use pyo3::prelude::*;
use fp::prime::ValidPrime;

pub fn new_valid_prime(p: u32) -> PyResult<ValidPrime> {
    let result = ValidPrime::try_new(p)
        .ok_or(python_utils::exception!(ValueError,   
                "First argument {} is not a valid prime.", p
        ))?;
    fp::vector::initialize_limb_bit_index_table(result);
    Ok(result)
}
