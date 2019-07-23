use crate::fp_vector::FpVector;

pub trait Algebra {
    fn get_prime(&self) -> u32;
    fn get_max_degree(&self) -> i32; 
    fn get_name(&self) -> String;
    // FiltrationOneProductList *product_list; // This determines which indecomposibles have lines drawn for them.
// Methods:
    fn compute_basis(&self, degree : i32);
    fn get_dimension(&self, degree : i32, excess : i32) -> usize;
    fn multiply_basis_elements(&self, result : FpVector, coeff : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32);
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> &str;
}