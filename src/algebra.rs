use crate::fp_vector::FpVector;

pub trait Algebra {
    fn get_prime(&self) -> u32;
    fn get_max_degree(&self) -> i32; 
    fn get_name(&self) -> &str;
    // FiltrationOneProductList *product_list; // This determines which indecomposibles have lines drawn for them.
// Methods:
    fn compute_basis(&self, degree : i32);
    fn get_dimension(&self, degree : i32, excess : i32) -> usize;
    fn multiply_basis_elements(&self, result : &mut FpVector, coeff : u32, r_degree : i32, r_idx : usize, s_degree: i32, s_idx : usize, excess : i32);
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String;

    fn element_to_string(&self, degree : i32, element : FpVector) -> String {
        let mut result = String::new();
        let mut zero = true;
        for (idx, value) in element.iter().enumerate() {
            if value == 0 {
                continue;
            }
            zero = false;
            if value != 1 {
                result.push_str(&format!("{} * ", value));
            }
            let b = self.basis_element_to_string(degree, idx);
            result.push_str(&format!("{} + ", b));
        }
        if zero {
            result.push_str("0");
        } else {
            // Remove trailing " + "
            result.pop();
            result.pop();
            result.pop();
        }
        return result;
    }    
}