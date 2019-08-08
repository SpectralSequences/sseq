use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra, AlgebraAny};
use crate::module::{Module, OptionModule, ModuleFailedRelationError};

use serde_json::value::Value;
use serde_json::json;

use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;


pub struct FiniteDimensionalModule {
    algebra : Rc<AlgebraAny>,
    name : String,
    min_degree : i32,
    graded_dimension : Vec<usize>,
    gen_names : Vec<Vec<String>>,
    // This goes input_degree --> output_degree --> operation --> input_index --> Vector
    actions : Vec<Vec<Vec<Vec<FpVector>>>>,
}

impl Module for FiniteDimensionalModule {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_algebra(&self) -> Rc<AlgebraAny> {
        Rc::clone(&self.algebra)
    }

    fn get_min_degree(&self) -> i32 {
        self.min_degree
    }
    
    fn compute_basis(&self, _degree : i32){ }

    fn get_dimension(&self, degree : i32) -> usize {
        if degree < self.min_degree {
            return 0;
        }
        let degree_idx = (degree - self.min_degree) as usize;
        if degree_idx >= self.graded_dimension.len() {
            return 0;
        }        
        return self.graded_dimension[degree_idx];
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        return self.gen_names[degree_idx][idx].clone();
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        assert!(op_index < self.get_algebra().get_dimension(op_degree, mod_degree));
        assert!(mod_index < self.get_dimension(mod_degree));
        let output_dimension = self.get_dimension(mod_degree + op_degree);    
        if output_dimension == 0 {
            return;
        }
        if op_degree == 0 {
            // We assume our algebras are connected so just add input to output.
            result.add_basis_element(mod_index, coeff);
            return;
        }
        let output = self.get_action(op_degree, op_index, mod_degree, mod_index);
        result.add(output, coeff);
    }
}

impl FiniteDimensionalModule {
    pub fn new(algebra : Rc<AlgebraAny>, name : String, min_degree : i32, graded_dimension : Vec<usize>) -> Self {
        algebra.compute_basis(min_degree + graded_dimension.len() as i32);
        let mut gen_names = Vec::with_capacity(graded_dimension.len());
        for i in 0..graded_dimension.len() {
            let mut names = Vec::with_capacity(graded_dimension[i]);
            for j in 0..graded_dimension[i]{
                names.push(format!("x{}{}", min_degree + i as i32, j));
            }
            gen_names.push(names);
        }
        let actions = FiniteDimensionalModule::allocate_actions(&algebra, min_degree, &graded_dimension);
        FiniteDimensionalModule {
            algebra,
            name,
            min_degree,
            gen_names,
            graded_dimension,
            actions
        }
    }

    pub fn set_basis_element_name(&mut self, degree : i32, idx : usize, name : String) {
        assert!(degree >= self.min_degree);
        let degree_idx = (degree - self.min_degree) as usize;
        self.gen_names[degree_idx][idx] = name;
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

    fn allocate_actions(algebra : &Rc<AlgebraAny>, min_degree : i32, graded_dimension : &Vec<usize>) -> Vec<Vec<Vec<Vec<FpVector>>>> {
        let basis_degree_range = graded_dimension.len();
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
        output : &FpVector
    ){
        assert!(operation_idx < self.algebra.get_dimension(operation_degree, input_degree));
        assert!(input_idx < self.get_dimension(input_degree));      
        let input_degree_idx = (input_degree - self.min_degree) as usize;
        let output_degree_idx = (input_degree + operation_degree - self.min_degree) as usize;
        let in_out_diff = output_degree_idx - input_degree_idx - 1;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree_idx][in_out_diff][operation_idx][input_idx];
        output_vector.assign(output);
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

    pub fn get_action(
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

    pub fn from_json(algebra : Rc<AlgebraAny>, algebra_name: &str, json : &mut Value) -> Self {
        let gens = json["gens"].take();
        let (min_degree, graded_dimension, gen_to_idx) = Self::module_gens_from_json(&gens);
        let name = json["name"].as_str().unwrap().to_string();
        let mut actions_value = json[algebra_name.to_owned() + "_actions"].take();
        let actions = actions_value.as_array_mut().unwrap();
        let mut result = Self::new(Rc::clone(&algebra), name, min_degree, graded_dimension);
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

    pub fn check_validity(&self, input_deg : i32, output_deg : i32) -> Result<(),Box<dyn Error>>{
        assert!(output_deg > input_deg);
        let p = self.get_prime();
        let algebra = self.get_algebra();
        let op_deg = output_deg - input_deg;
        let mut output_vec = FpVector::new(p, self.get_dimension(output_deg), 0);
        let mut tmp_output = FpVector::new(p, self.get_dimension(output_deg), 0);  
        for idx in 0..self.get_dimension(input_deg) {      
            for op_idx in 0..algebra.get_dimension(op_deg, -1) {
                let relations = algebra.get_relations_to_check(op_deg);
                for relation in relations {
                    for (coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                        let intermediate_dim = self.get_dimension(input_deg + *deg_2);
                        if intermediate_dim > tmp_output.get_dimension() {
                            tmp_output = FpVector::new(p, intermediate_dim, 0);
                        }
                        tmp_output.set_slice(0, intermediate_dim);
                        self.act_on_basis(&mut tmp_output, 1, *deg_2, *idx_2, input_deg, idx);
                        self.act(&mut output_vec, *coef, *deg_1, *idx_1, *deg_2 + input_deg, &tmp_output); 
                        tmp_output.clear_slice();
                        tmp_output.set_to_zero();                       
                    }
                    if !output_vec.is_zero() {
                        let mut relation_string = String::new();
                        for (coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                            relation_string.push_str(&format!("{} * {} * {}  +  ", 
                                *coef, 
                                &algebra.basis_element_to_string(*deg_1, *idx_1), 
                                &algebra.basis_element_to_string(*deg_2, *idx_2))
                            );
                        }
                        relation_string.pop(); relation_string.pop(); relation_string.pop();
                        relation_string.pop(); relation_string.pop();

                        let value_string = self.element_to_string(output_deg as i32, &output_vec);
                        return Err(Box::new(ModuleFailedRelationError {relation : relation_string, value : value_string}));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn extend_actions(&mut self, input_deg : i32, output_deg : i32){
        let p = self.get_prime();
        let algebra = self.get_algebra();
        let op_deg = output_deg - input_deg;
        let mut output_vec = FpVector::new(p, self.get_dimension(output_deg), 0);
        let mut tmp_output = FpVector::new(p, self.get_dimension(output_deg), 0);
        let generators = algebra.get_generators(op_deg);  
        for idx in 0 .. self.get_dimension(input_deg) {      
            for op_idx in 0 .. algebra.get_dimension(op_deg, -1) {
                if !generators.contains(&op_idx) {
                    let decomposition = algebra.decompose_basis_element(op_deg, op_idx);
                    for (coef, (deg_1, idx_1), (deg_2, idx_2)) in decomposition {
                        let intermediate_dim = self.get_dimension(input_deg + deg_2);
                        if intermediate_dim > tmp_output.get_dimension() {
                            tmp_output = FpVector::new(p, intermediate_dim, 0);
                        }
                        tmp_output.set_slice(0, intermediate_dim);                        
                        self.act_on_basis(&mut tmp_output, 1, deg_2, idx_2, input_deg, idx);
                        self.act(&mut output_vec, coef, deg_1, idx_1, deg_2 + input_deg, &tmp_output);
                        tmp_output.clear_slice();
                        tmp_output.set_to_zero();
                    }
                    self.set_action_vector(op_deg, op_idx, input_deg, idx, &output_vec);
                }
                output_vec.set_to_zero();
            }
        }
    }

    pub fn actions_to_json(&self) -> Value {
        let p = self.get_prime();
        let algebra = self.get_algebra();
        let min_degree = self.get_min_degree();
        let max_degree = min_degree + self.graded_dimension.len() as i32;
        let mut actions = Vec::new();
        for input_degree in min_degree..max_degree {
            for output_degree in (input_degree + 1) .. max_degree {
                let op_degree = output_degree - input_degree;
                for input_idx in 0..self.get_dimension(input_degree){
                    for op_idx in 0..algebra.get_dimension(op_degree, -1) {
                        let vec = self.get_action(op_degree, op_idx, input_degree, input_idx);
                        let mut current_terms = Vec::new();
                        for (i, v) in vec.iter().enumerate() {
                            if v == 0 {
                                continue;
                            }
                            current_terms.push(json!({"gen" : self.basis_element_to_string(output_degree, i), "coeff" : v}));
                        }
                        let current_action = json!({
                            "op" : algebra.json_from_basis(op_degree, op_idx),
                            "input" : self.basis_element_to_string(input_degree, input_idx),
                            "output" : current_terms
                        });
                        actions.push(current_action);
                    }
                }
            }
        }
        json!(actions)
    }
}

pub type OptionFDModule = OptionModule<FiniteDimensionalModule>;
