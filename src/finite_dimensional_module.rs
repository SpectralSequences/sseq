use crate::fp_vector::FpVector;
use crate::algebra::Algebra;
use crate::module::Module;

struct FiniteDimensionalModule<'a> {
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
    
    fn compute_basis(&mut self, degree : i32){ }

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
                for i in 0 .. number_of_operations {
                    let mut in_idx_vec : Vec<FpVector> = Vec::with_capacity(number_of_inputs);
                    for j in 0 .. number_of_inputs {
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
        let output_degree_idx = (input_degree + operation_degree) as usize;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree_idx][output_degree_idx][operation_idx][input_idx];
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
        let output_degree_idx = (input_degree + operation_degree) as usize;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector = &mut self.actions[input_degree_idx][output_degree_idx][operation_idx][input_idx];
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
        let output_degree_idx = (input_degree + operation_degree) as usize;
        return &self.actions[input_degree_idx][output_degree_idx][operation_idx][input_idx];
    }
}