use bivec::BiVec;

use fp::vector::{FpVector, FpVectorT};
use crate::algebra::{Algebra, AlgebraAny};
use crate::module::{Module, ZeroModule, ModuleFailedRelationError, BoundedModule};

use serde_json::value::Value;
use serde_json::json;

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::str::FromStr;

use nom::{
    IResult,
    branch::alt,
    bytes::complete::{is_not, take},
    character::complete::{space0, space1, digit1},
    combinator::map,
    multi::separated_list,
    sequence::tuple,
};

pub struct FiniteDimensionalModule {
    algebra : Arc<AlgebraAny>,
    name : String,
    graded_dimension : BiVec<usize>,
    gen_names : BiVec<Vec<String>>,
    // This goes input_degree --> output_degree --> operation --> input_index --> Vector
    actions : BiVec<BiVec<Vec<Vec<FpVector>>>>,
}

impl PartialEq for FiniteDimensionalModule {
    fn eq(&self, other : &Self) -> bool {
        self.graded_dimension == other.graded_dimension &&
            self.actions == other.actions
    }
}

impl Eq for FiniteDimensionalModule {}

impl FiniteDimensionalModule {
    pub fn test_equal(&self, other : &Self) -> Result<(), String> {
        if self.graded_dimension != other.graded_dimension {
            if self.graded_dimension.min_degree() != other.graded_dimension.min_degree() {
                return Err(format!("Min degrees disagree. left.min_degree() = {} but right.min_degree() = {}.", self.graded_dimension.min_degree(), other.graded_dimension.min_degree()));
            }
            if self.graded_dimension.len() != other.graded_dimension.len() {
                return Err(format!("Graded dimension lengths disagree. left.len() = {} but right.len() = {}.", self.graded_dimension.len(), other.graded_dimension.len()));
            }
            let mut disagreements = vec![];
            for i in self.graded_dimension.min_degree() .. self.graded_dimension.len() {
                if self.graded_dimension[i] != other.graded_dimension[i] {
                    disagreements.push(i);
                }
            }
            return Err(format!("Graded dimensions disagree in positions {:?}. Left has graded dimensions:\n    {:?}\nRight has graded dimension:\n    {:?}\n",
                disagreements,
                self.graded_dimension,
                other.graded_dimension
            ))
        }
        if self.actions != other.actions {
            // actions goes input_degree --> output_degree --> operation --> input_index --> Vector
            let mut disagreements = vec![];
            for input_degree in self.actions.min_degree() .. self.actions.len(){
                for output_degree in self.actions[input_degree].min_degree() .. self.actions[input_degree].len(){
                    for operation in 0 .. self.actions[input_degree][output_degree].len() {
                        for input_index in 0 .. self.actions[input_degree][output_degree][operation].len() {
                            let self_action = &self.actions[input_degree][output_degree][operation][input_index];
                            let other_action = &other.actions[input_degree][output_degree][operation][input_index];
                            if self_action != other_action {
                                disagreements.push((input_degree, output_degree, operation, input_index, self_action, other_action));
                            }
                        }
                    }
                }
            }
            
            let mut err_string = "Actions disagree.\n".to_string();
            for x in disagreements {
                err_string.push_str(&format!("  {} * {} disagreement.\n    Left: {}\n    Right: {}\n",
                    self.algebra.basis_element_to_string(x.1 - x.0, x.2),
                    self.basis_element_to_string(x.0, x.3),
                    self.element_to_string(x.1, &x.4),
                    self.element_to_string(x.1, &x.5)
                ))
            }
            return Err(err_string);
        }
        Ok(())
    }
}

impl Module for FiniteDimensionalModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn algebra(&self) -> Arc<AlgebraAny> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.graded_dimension.min_degree()
    }
    
    fn compute_basis(&self, _degree : i32){ }

    fn dimension(&self, degree : i32) -> usize {
        if degree < self.graded_dimension.min_degree() {
            return 0;
        }
        if degree > self.graded_dimension.max_degree() {
            return 0;
        }        
        self.graded_dimension[degree]
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        self.gen_names[degree][idx].clone()
    }

    fn is_unit(&self) -> bool {
        self.min_degree() == 0 && self.graded_dimension.len() == 1 && self.graded_dimension[0] == 1
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        assert!(op_index < self.algebra().dimension(op_degree, mod_degree));
        assert!(mod_index < self.dimension(mod_degree));
        let output_dimension = self.dimension(mod_degree + op_degree);    
        if output_dimension == 0 {
            return;
        }
        if op_degree == 0 {
            // We assume our algebras are connected so just add input to output.
            result.add_basis_element(mod_index, coeff);
            return;
        }
        let output = self.action(op_degree, op_index, mod_degree, mod_index);
        result.shift_add(output, coeff);
    }

    fn borrow_output(&self) -> bool { true }
    fn act_on_basis_borrow(&self, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) -> &FpVector {
        self.action(op_degree, op_index, mod_degree, mod_index)
    }
}

impl BoundedModule for FiniteDimensionalModule {
    fn max_degree(&self) -> i32 {
        self.graded_dimension.max_degree()
    }
}

impl ZeroModule for FiniteDimensionalModule {
    fn zero_module(algebra : Arc<AlgebraAny>, min_degree : i32) -> Self {
        Self::new(algebra, "zero".to_string(), BiVec::new(min_degree))
    }
}

impl FiniteDimensionalModule {
    pub fn new(algebra : Arc<AlgebraAny>, name : String, graded_dimension : BiVec<usize>) -> Self {
        let min_degree = graded_dimension.min_degree();
        let max_degree = graded_dimension.len();
        let degree_difference = max_degree - min_degree;
        algebra.compute_basis(degree_difference);
        let mut gen_names = BiVec::with_capacity(min_degree, max_degree);
        for (i, dim) in graded_dimension.iter_enum() {
            let mut names = Vec::with_capacity(*dim);
            for j in 0 .. *dim {
                names.push(format!("x{}_{}", i, j));
            }
            gen_names.push(names);
        }
        let actions = FiniteDimensionalModule::allocate_actions(&algebra, &graded_dimension);
        FiniteDimensionalModule {
            algebra,
            name,
            gen_names,
            graded_dimension,
            actions
        }
    }

    pub fn set_basis_element_name(&mut self, degree : i32, idx : usize, name : String) {
        self.gen_names[degree][idx] = name;
    }
    
    fn module_gens_from_json(gens : Value) -> (BiVec<usize>, BiVec<Vec<String>>, HashMap<String, (i32, usize)>) {
        let gens = gens.as_object().unwrap();

        let degrees = gens.iter().map(|(_, x)| x.as_i64().unwrap() as i32).collect::<Vec<_>>();

        let min_degree = degrees.iter().copied().min().unwrap_or(0);
        let max_degree = degrees.iter().copied().max().unwrap_or(-1) + 1;

        let mut gen_to_idx = HashMap::new();
        let mut graded_dimension = BiVec::with_capacity(min_degree, max_degree);
        let mut gen_names = BiVec::with_capacity(min_degree, max_degree);

        for _ in min_degree..max_degree {
            graded_dimension.push(0);
            gen_names.push(vec![]);
        }

        for (name, degree) in gens {
            let degree = degree.as_i64().unwrap() as i32;
            gen_names[degree].push(name.clone());
            gen_to_idx.insert(name.clone(), (degree, graded_dimension[degree]));
            graded_dimension[degree] += 1;
        }
        (graded_dimension, gen_names, gen_to_idx)
    }

    fn allocate_actions(algebra : &Arc<AlgebraAny>, graded_dimension : &BiVec<usize>) -> BiVec<BiVec<Vec<Vec<FpVector>>>> {
        let min_degree = graded_dimension.min_degree();
        let max_degree = graded_dimension.len();
        let mut result : BiVec<BiVec<Vec<Vec<FpVector>>>> = BiVec::with_capacity(min_degree, max_degree);
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
        for input_degree in min_degree .. max_degree {
            if graded_dimension[input_degree] == 0 {
                result.push(BiVec::new(input_degree));
                continue;
            }
            let mut outputs_vec : BiVec<Vec<Vec<FpVector>>> = BiVec::with_capacity(input_degree, max_degree);
            // We assume our algebra is connected, so we can manually fill in the first entry.
            let number_of_inputs = graded_dimension[input_degree];
            let mut ops_vec : Vec<Vec<FpVector>> = vec![Vec::with_capacity(number_of_inputs)];
            for i in 0 .. number_of_inputs {
                let mut result = FpVector::new(algebra.prime(), number_of_inputs);
                result.set_entry(i, 1);
                ops_vec[0].push(result);
            }
            outputs_vec.push(ops_vec);

            for output_degree in input_degree + 1 .. max_degree {
                if graded_dimension[output_degree] == 0 {
                    outputs_vec.push(Vec::with_capacity(0));
                    continue;
                }
                let op_deg = output_degree - input_degree;
                let number_of_operations = algebra.dimension(op_deg, min_degree + input_degree);
                let number_of_inputs = graded_dimension[input_degree];
                let number_of_outputs = graded_dimension[output_degree];
                let mut ops_vec : Vec<Vec<FpVector>> = Vec::with_capacity(number_of_operations);
                for _ in 0 .. number_of_operations {
                    let mut in_idx_vec : Vec<FpVector> = Vec::with_capacity(number_of_inputs);
                    for _ in 0 .. number_of_inputs {
                        in_idx_vec.push(FpVector::new(algebra.prime(), number_of_outputs));
                    }
                    assert!(in_idx_vec.len() == number_of_inputs);
                    ops_vec.push(in_idx_vec);
                }
                assert!(ops_vec.len() == number_of_operations);
                outputs_vec.push(ops_vec);
            }
            assert!(outputs_vec.len() == max_degree);
            result.push(outputs_vec);
        }
        assert!(result.len() == max_degree);
        result
    }

    pub fn set_action_vector(
        &mut self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize,
        output : &FpVector
    ){
        assert!(operation_idx < self.algebra.dimension(operation_degree, input_degree));
        assert!(input_idx < self.dimension(input_degree));      
        let output_degree = input_degree + operation_degree;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree][output_degree][operation_idx][input_idx];
        output_vector.assign(output);
    }

    pub fn set_action(
        &mut self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize,
        output : Vec<u32>
    ){
        assert!(operation_idx < self.algebra.dimension(operation_degree, input_degree));
        assert!(input_idx < self.dimension(input_degree));
        let output_degree = input_degree + operation_degree;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree][output_degree][operation_idx][input_idx];
        output_vector.pack(&output);
    }    

    /// This function will panic if you call it with input such that `module.dimension(input_degree +
    /// operation_degree) = 0`.
    pub fn action(
        &self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize
    ) -> &FpVector {
        let output_degree = input_degree + operation_degree;
        &self.actions[input_degree][output_degree][operation_idx][input_idx]
    }

    /// This function will panic if you call it with input such that `module.dimension(input_degree +
    /// operation_degree) = 0`.
    pub fn action_mut(
        &mut self,
        operation_degree : i32, operation_idx : usize,
        input_degree : i32, input_idx : usize
    ) -> &mut FpVector {
        let output_degree = input_degree + operation_degree;
        &mut self.actions[input_degree][output_degree][operation_idx][input_idx]
    }

    pub fn from_json(algebra : Arc<AlgebraAny>, json : &mut Value) -> Self {
        let gens = json["gens"].take();
        let (graded_dimension, gen_names, gen_to_idx) = Self::module_gens_from_json(gens);
        let name = json["name"].as_str().unwrap_or("").to_string();

        let mut result = Self::new(Arc::clone(&algebra), name, graded_dimension.clone());
        for (i, dim) in graded_dimension.iter_enum() {
            for j in 0..*dim {
                result.set_basis_element_name(i, j, gen_names[i][j].clone());
            }
        }

        if let Ok(actions) = serde_json::from_value::<Vec<String>>(json["actions"].take()) {
            for action in actions {
                result.parse_action(&gen_to_idx, &action);
            }
            for input_degree in (result.min_degree() ..= result.max_degree()).rev() {
                for output_degree in input_degree + 1 ..= result.max_degree() {
                    result.extend_actions(input_degree, output_degree);
                    result.check_validity(input_degree, output_degree).unwrap();
                }
            }
        } else {
            let mut actions_value = json[algebra.algebra_type().to_owned() + "_actions"].take();
            let actions = actions_value.as_array_mut().unwrap();
            for action in actions.iter_mut() {
                let op = action["op"].take();
                let (degree, idx) = algebra.json_to_basis(op);
                let input_name = action["input"].as_str().unwrap();
                let (input_degree, input_idx) = gen_to_idx[input_name];
                let output_vec = result.action_mut(degree, idx, input_degree, input_idx);
                let outputs = action["output"].as_array().unwrap();
                for basis_elt in outputs {
                    let output_name = basis_elt["gen"].as_str().unwrap();
                    let output_idx = gen_to_idx[output_name].1;
                    let output_coeff = basis_elt["coeff"].as_u64().unwrap() as u32;
                    output_vec.set_entry(output_idx, output_coeff);
                }
            }
        }
        result
    }

    fn parse_action(&mut self, gen_to_idx: &HashMap<String, (i32, usize)>, entry_: &str) {
        let algebra = self.algebra();
        let lhs = tuple((
            |e| algebra.string_to_basis(e),
            is_not("="),
            take(1usize)
        ));

        let (entry, ((op_deg, op_idx), gen, _)) = lhs(entry_).unwrap();

        let (input_deg, input_idx) = gen_to_idx[gen.trim()];
        let row = self.action_mut(op_deg, op_idx, input_deg, input_idx);

        // Need explicit type here
        let (_, values) = <IResult<_, _>>::unwrap(separated_list(take(1usize), is_not("+"))(entry));

        for value in values {
            let (_, (coef, gen)) = Self::parse_element(value)
                .unwrap_or_else(|_| panic!("Invalid action: {}", entry_));

            let (deg, idx) = gen_to_idx[gen];
            assert!(deg == input_deg + op_deg, "Invalid action: {}", entry_);

            row.add_basis_element(idx, coef);
        }
    }

    fn parse_element(i: &str) -> IResult<&str, (u32, &str)> { // coefficient, name
        let coef_gen = map(tuple((space0, digit1, space1, is_not(" "))), |(_, coef, _, gen)| (FromStr::from_str(coef).unwrap(), gen));
        let o_gen = map(tuple((space0, is_not(" "))), |(_, gen)| (1, gen));
        alt((coef_gen, o_gen))(i)
    }

    pub fn check_validity(&self, input_deg : i32, output_deg : i32) -> Result<(),Box<dyn Error>>{
        assert!(output_deg > input_deg);
        let p = self.prime();
        let algebra = self.algebra();
        let op_deg = output_deg - input_deg;
        let mut output_vec = FpVector::new(p, self.dimension(output_deg));
        let mut tmp_output = FpVector::new(p, self.dimension(output_deg));
        for idx in 0..self.dimension(input_deg) {
            let relations = algebra.relations_to_check(op_deg);
            for relation in relations {
                for &(coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    let intermediate_dim = self.dimension(input_deg + deg_2);
                    if intermediate_dim > tmp_output.dimension() {
                        tmp_output = FpVector::new(p, intermediate_dim);
                    }
                    tmp_output.set_slice(0, intermediate_dim);
                    self.act_on_basis(&mut tmp_output, 1, deg_2, idx_2, input_deg, idx);
                    self.act(&mut output_vec, coef, deg_1, idx_1, deg_2 + input_deg, &tmp_output);
                    tmp_output.clear_slice();
                    tmp_output.set_to_zero_pure();
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
                    for _ in 0 .. 5 {
                        relation_string.pop();
                    }

                    let value_string = self.element_to_string(output_deg as i32, &output_vec);
                    return Err(Box::new(ModuleFailedRelationError {relation : relation_string, value : value_string}));
                }
            }
        }
        Ok(())
    }

    pub fn extend_actions(&mut self, input_deg : i32, output_deg : i32){
        let p = self.prime();
        let algebra = self.algebra();
        let op_deg = output_deg - input_deg;
        if self.dimension(output_deg) == 0 || self.dimension(input_deg) == 0 {
            return;
        }

        let mut output_vec = FpVector::new(p, self.dimension(output_deg));
        let mut tmp_output = FpVector::new(p, self.dimension(output_deg));
        let generators = algebra.generators(op_deg);  
        for idx in 0 .. self.dimension(input_deg) {      
            for op_idx in 0 .. algebra.dimension(op_deg, -1) {
                if !generators.contains(&op_idx) {
                    let decomposition = algebra.decompose_basis_element(op_deg, op_idx);
                    for (coef, (deg_1, idx_1), (deg_2, idx_2)) in decomposition {
                        let intermediate_dim = self.dimension(input_deg + deg_2);
                        if intermediate_dim > tmp_output.dimension() {
                            tmp_output = FpVector::new(p, intermediate_dim);
                        }
                        tmp_output.set_slice(0, intermediate_dim);                        
                        self.act_on_basis(&mut tmp_output, 1, deg_2, idx_2, input_deg, idx);
                        self.act(&mut output_vec, coef, deg_1, idx_1, deg_2 + input_deg, &tmp_output);
                        tmp_output.clear_slice();
                        tmp_output.set_to_zero_pure();
                    }
                    self.set_action_vector(op_deg, op_idx, input_deg, idx, &output_vec);
                }
                output_vec.set_to_zero();
            }
        }
    }

    pub fn minimal_actions_to_json(&self) -> Value {
        let algebra = self.algebra();
        let min_degree = self.min_degree();
        let max_degree = min_degree + self.graded_dimension.len() as i32;
        let mut actions = Vec::new();
        for input_degree in min_degree..max_degree {
            for output_degree in (input_degree + 1) .. max_degree {
                if self.dimension(output_degree) == 0 {
                    continue;
                }
                let op_degree = output_degree - input_degree;
                for op_idx in algebra.generators(op_degree) {
                    for input_idx in 0..self.dimension(input_degree) {
                        let vec = self.action(op_degree, op_idx, input_degree, input_idx);
                        if vec.is_zero() {
                            continue;
                        }
                        actions.push(json!({
                            "op": algebra.json_from_basis(op_degree, op_idx),
                            "input_deg": input_degree,
                            "input_idx": input_idx,
                            "output": vec.iter().collect::<Vec<u32>>()
                        }));
                    }
                }
            }
        }
        json!(actions)
    }

    pub fn to_minimal_json(&self) -> Value {
        json!({
            "p": self.prime(),
            "algebra": self.algebra().algebra_type(),
            "min_degree": self.min_degree(),
            "graded_dimension": self.graded_dimension,
            "actions": self.minimal_actions_to_json(),
        })
    }

    pub fn actions_to_json(&self) -> Value {
        let algebra = self.algebra();
        let min_degree = self.min_degree();
        let max_degree = min_degree + self.graded_dimension.len() as i32;
        let mut actions = Vec::new();
        for input_degree in min_degree..max_degree {
            for output_degree in (input_degree + 1) .. max_degree {
                if self.dimension(output_degree) == 0 {
                    continue;
                }
                let op_degree = output_degree - input_degree;
                for input_idx in 0..self.dimension(input_degree){
                    for op_idx in 0..algebra.dimension(op_degree, -1) {
                        let vec = self.action(op_degree, op_idx, input_degree, input_idx);
                        let mut current_terms = Vec::new();
                        for (i, v) in vec.iter().enumerate() {
                            if v == 0 {
                                continue;
                            }
                            current_terms.push(json!({"gen" : self.basis_element_to_string(output_degree, i), "coeff" : v}));
                        }
                        if current_terms.is_empty() {
                            continue;
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::adem_algebra::AdemAlgebra;
    use bivec::BiVec;

    #[test]
    fn test_module_check_validity(){
        let p = 2;
        let adem_algebra = Arc::new(AlgebraAny::from(AdemAlgebra::new(p, p != 2, false)));
        adem_algebra.compute_basis(10);
        let mut adem_module = FiniteDimensionalModule::new(Arc::clone(&adem_algebra), "".to_string(), BiVec::from_vec(0, vec![1,2,1]));
        adem_module.set_basis_element_name(0, 0, "x0".to_string());
        adem_module.set_basis_element_name(1, 0, "x10".to_string());
        adem_module.set_basis_element_name(1, 1, "x11".to_string());
        adem_module.set_basis_element_name(2, 0, "x2".to_string());
        adem_module.set_action_vector(1, 0, 0, 0, &FpVector::from_vec(2,&[1,1]));
        adem_module.set_action_vector(1, 0, 1, 0, &FpVector::from_vec(2,&[1]));
        adem_module.set_action_vector(1, 0, 1, 1, &FpVector::from_vec(2,&[1]));
        adem_module.set_action_vector(2, 0, 0, 0, &FpVector::from_vec(2,&[1]));
        adem_module.check_validity(0, 2).unwrap();
    }

}
