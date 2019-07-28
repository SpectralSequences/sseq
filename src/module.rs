use crate::fp_vector::FpVector;
use crate::algebra::Algebra;

// enum Module_Type {

// }

pub trait Module {
    fn get_prime(&self) -> u32 {
        self.get_algebra().get_prime()
    }
    fn get_algebra(&self) -> &Algebra;
    fn get_name(&self) -> &str;
    // fn get_type() -> Module_Type;
    // int min_degree;
    // int max_degree; // Rename to max_allocated_degree?
    // int max_computed_degree;
// Methods:
    fn get_min_degree(&self) -> i32;
    // fn get_max_degree(&self) -> i32;
    fn compute_basis(&mut self, _degree : i32) {}
    fn get_dimension(&self, degree : i32) -> usize;
    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize);
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String;

    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
        assert!(input.get_dimension() == self.get_dimension(input_degree));
        let p = self.get_algebra().get_prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act_on_basis(result, (coeff * v) % p, op_degree, op_index, input_degree, i);
        }
    }

    fn element_to_string(&self, degree : i32, element : &FpVector) -> String {
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

pub struct ZeroModule<'a> {algebra : &'a Algebra, name : String }

impl<'a> ZeroModule<'a> {
    pub fn new(algebra : &'a Algebra) -> Self {
        let name = format!("Zero Module over {}", algebra.get_name());
        ZeroModule {
            algebra,
            name
        }
    }
}

impl<'a> Module for ZeroModule<'a> {
    fn get_algebra(&self) -> &Algebra {
        self.algebra
    }
    
    fn get_name(&self) -> &str{
        return &self.name;
    }

    fn get_dimension(&self, _degree : i32) -> usize {
        0
    }

    fn get_min_degree(&self) -> i32 {
        0
    }

    // Since the dimension is 0, the input of this function is an element of the basis which is the empty set.
    fn act_on_basis(&self, _result : &mut FpVector, _coeff : u32, _op_degree : i32, _op_index : usize, _mod_degree : i32, _mod_index : usize){
        assert!(false);
    }

    fn basis_element_to_string(&self, _degree : i32, _index : usize) -> String{
        assert!(false);
        "".to_string()
    }

}
