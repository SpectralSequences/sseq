use crate::fp_vector::FpVector;
use crate::algebra::Algebra;
use crate::module::Module;
use serde_json::value::Value;
use std::collections::HashMap;


pub struct FiniteDimensionalModule<'a> {
    algebra : &'a Algebra,
    name : String,
    min_degree : i32,
    max_basis_degree : i32,
    graded_dimension : Vec<usize>,
    // This goes input_degree --> output_degree --> operation --> input_index --> Vector
    actions : Vec<Vec<Vec<Vec<FpVector>>>>,
}

impl<'a> Module for FiniteDimensionalModule<'a> {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_algebra(&self) -> &Algebra {
        self.algebra
    }
    
    fn get_min_degree(&self) -> i32 {
        self.min_degree
    }
    
    fn compute_basis(&mut self, _degree : i32){ }

    fn get_dimension(&self, degree : i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        if degree >= self.max_basis_degree {
            return 0;
        }
        let degree_idx = (degree - self.min_degree) as usize;
        return self.graded_dimension[degree_idx];
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        return format!("x_{{{},{}}}", degree, idx);
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        assert!(op_index < self.get_algebra().get_dimension(op_degree, mod_degree));
        assert!(mod_index < self.get_dimension(mod_degree));
        let output_dimension = self.get_dimension(mod_degree + op_degree);    
        if output_dimension == 0 {
            return;
        }          
        let output = self.get_action(op_degree, op_index, mod_degree, mod_index);
        result.add(output, coeff);
    }
}

impl<'a> FiniteDimensionalModule<'a> {
    pub fn new(algebra : &'a Algebra, name : String, min_degree : i32, max_basis_degree : i32, graded_dimension : Vec<usize>) -> Self {
        assert!(max_basis_degree >= min_degree);
        println!("min_degree : {}, max_degree : {}, graded_dimension : {:?}", min_degree, max_basis_degree, graded_dimension);
        let actions = FiniteDimensionalModule::allocate_actions(algebra, min_degree, (max_basis_degree - min_degree) as usize, &graded_dimension);
        FiniteDimensionalModule {
            algebra,
            name,
            min_degree,
            max_basis_degree,
            graded_dimension,
            actions
        }
    }

    pub fn from_json(algebra : &'a Algebra, algebra_name: &str, json : &mut Value) -> Self {
        let gens = json["gens"].take();
        let (min_degree, graded_dimension, gen_to_idx) = Self::module_gens_from_json(&gens);
        let name = json["name"].as_str().unwrap().to_string();
        let mut actions_value = json[algebra_name.to_owned() + "_actions"].take();
        let actions = actions_value.as_array_mut().unwrap();
        let mut result = Self::new(algebra, name, min_degree, min_degree + graded_dimension.len() as i32, graded_dimension);
        for action in actions.iter_mut() {
            let op = action["op"].take();
            let (degree, idx) = algebra.json_to_basis(op);

            let input_name = action["input"].as_str().unwrap();
            let (input_degree, input_idx) = gen_to_idx[&input_name.to_string()];
            let output_vec = result.get_action_mut(degree, idx, input_degree, input_idx);
            let outputs = action["output"].as_array().unwrap();
            for basis_elt in outputs {
                let output_name = basis_elt["gen"].as_str().unwrap();
                let output_idx = gen_to_idx[&output_name.to_string()].1;
                let output_coeff = basis_elt["coeff"].as_u64().unwrap() as u32;
                output_vec.set_entry(output_idx, output_coeff);
            }
        }
        return result;
    }
    
    fn module_gens_from_json(gens : &Value) -> (i32, Vec<usize>, HashMap<&String, (i32, usize)>) {
        let gens = gens.as_object().unwrap();
        assert!(gens.len() > 0);
        let mut min_degree = 10000;
        let mut max_degree = -10000;
        for (_name, degree_value) in gens.iter() {
            let degree = degree_value.as_i64().unwrap();
            if degree < min_degree {
                min_degree = degree;
            }
            if degree + 1 > max_degree {
                max_degree = degree + 1;
            }
        }
        let mut gen_to_idx = HashMap::new();
        let mut graded_dimension = vec!(0; (max_degree - min_degree) as usize);
        for (name, degree_value) in gens.iter() {
            let degree = degree_value.as_i64().unwrap();
            let degree_idx = (degree - min_degree) as usize;
            gen_to_idx.insert(name, (degree as i32, graded_dimension[degree_idx]));
            graded_dimension[degree_idx] += 1;
        }
        return (min_degree as i32, graded_dimension, gen_to_idx);
    }

    fn allocate_actions(algebra : &Algebra, min_degree : i32, basis_degree_range : usize, graded_dimension : &Vec<usize>) -> Vec<Vec<Vec<Vec<FpVector>>>> {
        let mut result : Vec<Vec<Vec<Vec<FpVector>>>> = Vec::with_capacity(basis_degree_range);
        // Count number of triples (x, y, op) with |x| + |op| = |y|.
        // The amount of memory we need to allocate is:
        // # of input_degrees  * sizeof(***Vector)
        // + # of nonempty input degrees * # of output degrees * sizeof(**Vector)
        // + Sum over (nonempty in_deg < nonempty out_deg) of (
        //              # of operations in (out_deg - in_deg) * sizeof(*Vector)
        //              # of operations in (out_deg - in_deg) * # of gens in degree in_degree * sizeof(Vector)
        //              # of operations in (out_deg - in_deg) * # of gens in degree in_degree * # of gens in degree out_degree * sizeof(uint)
        // )
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> (out_index) -> value
        //  ****    -> ***       -> **Vector   -> *Vector    -> Vector -> uint
        for input_degree in 0 .. basis_degree_range {
            if graded_dimension[input_degree] == 0 {
                result.push(Vec::with_capacity(0));
                continue;
            }
            let number_of_output_degrees = (basis_degree_range - input_degree - 1) as usize;
            let mut outputs_vec : Vec<Vec<Vec<FpVector>>> = Vec::with_capacity(number_of_output_degrees);
            for output_degree in input_degree + 1 .. basis_degree_range {
                if graded_dimension[output_degree] == 0 {
                    outputs_vec.push(Vec::with_capacity(0));
                    continue;
                }
                let number_of_operations = algebra.get_dimension(min_degree + (output_degree - input_degree) as i32, min_degree + input_degree as i32 ) as usize;
                let number_of_inputs = graded_dimension[input_degree];
                let number_of_outputs = graded_dimension[output_degree];
                let mut ops_vec : Vec<Vec<FpVector>> = Vec::with_capacity(number_of_operations);
                for _ in 0 .. number_of_operations {
                    let mut in_idx_vec : Vec<FpVector> = Vec::with_capacity(number_of_inputs);
                    for _ in 0 .. number_of_inputs {
                        in_idx_vec.push(FpVector::new(algebra.get_prime(), number_of_outputs, 0));
                    }
                    assert!(in_idx_vec.len() == number_of_inputs);
                    ops_vec.push(in_idx_vec);
                }
                assert!(ops_vec.len() == number_of_operations);
                outputs_vec.push(ops_vec);
            }
            assert!(outputs_vec.len() == number_of_output_degrees);
            result.push(outputs_vec);
        }
        assert!(result.len() == basis_degree_range);
        return result;
    }

    pub fn set_action_vector(
        &mut self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize,
        output : FpVector
    ){
        assert!(operation_idx < self.algebra.get_dimension(operation_degree, input_degree));
        assert!(input_idx < self.get_dimension(input_degree));      
        let input_degree_idx = (input_degree - self.min_degree) as usize;
        let output_degree_idx = (input_degree + operation_degree - self.min_degree) as usize;
        let in_out_diff = output_degree_idx - input_degree_idx - 1;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree_idx][in_out_diff][operation_idx][input_idx];
        output_vector.assign(&output);
    }

    pub fn set_action(
        &mut self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize,
        output : Vec<u32>
    ){
        assert!(operation_idx < self.algebra.get_dimension(operation_degree, input_degree));
        assert!(input_idx < self.get_dimension(input_degree));      
        let input_degree_idx = (input_degree - self.min_degree) as usize;
        let output_degree_idx = (input_degree + operation_degree - self.min_degree) as usize;
        let in_out_diff = output_degree_idx - input_degree_idx - 1;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree_idx][in_out_diff][operation_idx][input_idx];
        output_vector.pack(&output);
    }    

    fn get_action(
        &self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize
    ) -> &FpVector {
        assert!(operation_idx < self.algebra.get_dimension(operation_degree, input_degree));
        assert!(input_idx < self.get_dimension(input_degree));              
        let input_degree_idx = (input_degree - self.min_degree) as usize;
        let output_degree_idx = (input_degree + operation_degree - self.min_degree) as usize;
        let in_out_diff = output_degree_idx - input_degree_idx - 1;
        return &self.actions[input_degree_idx][in_out_diff][operation_idx][input_idx];
    }

    fn get_action_mut(
        &mut self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize
    ) -> &mut FpVector {
        assert!(operation_idx < self.algebra.get_dimension(operation_degree, input_degree));
        assert!(input_idx < self.get_dimension(input_degree));              
        let input_degree_idx = (input_degree - self.min_degree) as usize;
        let output_degree_idx = (input_degree + operation_degree - self.min_degree) as usize;
        let in_out_diff = output_degree_idx - input_degree_idx - 1;
        return &mut self.actions[input_degree_idx][in_out_diff][operation_idx][input_idx];
    }    
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fd_mod(){

    }

}
