use std::{fmt::Write as _, sync::Arc};

use anyhow::{anyhow, Context};
use bivec::BiVec;
use fp::vector::{FpSliceMut, FpVector};
use serde::Deserialize;
use serde_json::{json, value::Value};

use crate::{
    algebra::{Algebra, GeneratedAlgebra},
    module::{Module, ModuleFailedRelationError, ZeroModule},
};

pub struct FiniteDimensionalModule<A: Algebra> {
    algebra: Arc<A>,
    pub name: String,
    graded_dimension: BiVec<usize>,
    gen_names: BiVec<Vec<String>>,
    // This goes input_degree --> output_degree --> operation --> input_index --> Vector
    actions: BiVec<BiVec<Vec<Vec<FpVector>>>>,
}

impl<A: Algebra> std::fmt::Display for FiniteDimensionalModule<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<A: Algebra> Clone for FiniteDimensionalModule<A> {
    fn clone(&self) -> Self {
        Self {
            algebra: Arc::clone(&self.algebra),
            name: self.name.clone(),
            graded_dimension: self.graded_dimension.clone(),
            gen_names: self.gen_names.clone(),
            actions: self.actions.clone(),
        }
    }
}

impl<A: Algebra> PartialEq for FiniteDimensionalModule<A> {
    fn eq(&self, other: &Self) -> bool {
        self.test_equal(other).is_ok()
    }
}

impl<A: Algebra> Eq for FiniteDimensionalModule<A> {}

impl<A: Algebra> FiniteDimensionalModule<A> {
    pub fn test_equal(&self, other: &Self) -> Result<(), String> {
        if self.graded_dimension != other.graded_dimension {
            if self.graded_dimension.min_degree() != other.graded_dimension.min_degree() {
                return Err(format!(
                    "Min degrees disagree. left.min_degree() = {} but right.min_degree() = {}.",
                    self.graded_dimension.min_degree(),
                    other.graded_dimension.min_degree()
                ));
            }
            let mut disagreements = vec![];
            for i in self.graded_dimension.min_degree()
                ..std::cmp::max(self.graded_dimension.len(), other.graded_dimension.len())
            {
                if self.graded_dimension.get(i).copied().unwrap_or(0)
                    != other.graded_dimension.get(i).copied().unwrap_or(0)
                {
                    disagreements.push(i);
                }
            }
            if !disagreements.is_empty() {
                return Err(format!(
                    "Graded dimensions disagree in positions {:?}. Left has graded \
                        dimensions:\n    {:?}\nRight has graded dimension:\n    {:?}\n",
                    disagreements, self.graded_dimension, other.graded_dimension
                ));
            }
        }
        let max_degree = std::cmp::min(self.actions.len(), other.actions.len());
        if self.actions != other.actions {
            // actions goes input_degree --> output_degree --> operation --> input_index --> Vector
            let mut disagreements = vec![];
            for input_degree in self.actions.min_degree()..max_degree {
                for output_degree in self.actions[input_degree].min_degree()..max_degree {
                    for operation in 0..self.actions[input_degree][output_degree].len() {
                        for input_index in
                            0..self.actions[input_degree][output_degree][operation].len()
                        {
                            let self_action =
                                &self.actions[input_degree][output_degree][operation][input_index];
                            let other_action =
                                &other.actions[input_degree][output_degree][operation][input_index];
                            if self_action != other_action {
                                disagreements.push((
                                    input_degree,
                                    output_degree,
                                    operation,
                                    input_index,
                                    self_action,
                                    other_action,
                                ));
                            }
                        }
                    }
                }
            }

            if !disagreements.is_empty() {
                let mut err_string = "Actions disagree.\n".to_string();
                for x in disagreements {
                    let _ = write!(
                        err_string,
                        "  {} * {} disagreement.\n    Left: {}\n    Right: {}\n",
                        self.algebra.basis_element_to_string(x.1 - x.0, x.2),
                        self.basis_element_to_string(x.0, x.3),
                        self.element_to_string(x.1, x.4.as_slice()),
                        self.element_to_string(x.1, x.5.as_slice())
                    );
                }
                return Err(err_string);
            }
        }
        Ok(())
    }
}

impl<A: Algebra> Module for FiniteDimensionalModule<A> {
    type Algebra = A;

    fn algebra(&self) -> Arc<Self::Algebra> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.graded_dimension.min_degree()
    }

    fn max_computed_degree(&self) -> i32 {
        i32::MAX
    }

    fn compute_basis(&self, _degree: i32) {}

    fn dimension(&self, degree: i32) -> usize {
        if degree < self.graded_dimension.min_degree() {
            return 0;
        }
        if degree > self.graded_dimension.max_degree() {
            return 0;
        }
        self.graded_dimension[degree]
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        self.gen_names[degree][idx].clone()
    }

    fn act_on_basis(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        assert!(op_index < self.algebra().dimension(op_degree));
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
        result.add(output.as_slice(), coeff);
    }

    fn max_degree(&self) -> Option<i32> {
        Some(self.graded_dimension.max_degree())
    }
}

impl<A: Algebra> ZeroModule for FiniteDimensionalModule<A> {
    fn zero_module(algebra: Arc<A>, min_degree: i32) -> Self {
        Self::new(algebra, "zero".to_string(), BiVec::new(min_degree))
    }
}

impl<A: Algebra> FiniteDimensionalModule<A> {
    pub fn new(algebra: Arc<A>, name: String, graded_dimension: BiVec<usize>) -> Self {
        let min_degree = graded_dimension.min_degree();
        let max_degree = graded_dimension.len();
        let degree_difference = max_degree - min_degree;
        algebra.compute_basis(degree_difference);
        let mut gen_names = BiVec::with_capacity(min_degree, max_degree);
        for (i, dim) in graded_dimension.iter_enum() {
            let mut names = Vec::with_capacity(*dim);
            for j in 0..*dim {
                names.push(format!("x{i}_{j}"));
            }
            gen_names.push(names);
        }
        let actions = Self::allocate_actions(&algebra, &graded_dimension);
        Self {
            algebra,
            name,
            graded_dimension,
            gen_names,
            actions,
        }
    }

    pub fn set_basis_element_name(&mut self, degree: i32, idx: usize, name: String) {
        self.gen_names[degree][idx] = name;
    }

    fn allocate_actions(
        algebra: &Arc<A>,
        graded_dimension: &BiVec<usize>,
    ) -> BiVec<BiVec<Vec<Vec<FpVector>>>> {
        let min_degree = graded_dimension.min_degree();
        let max_degree = graded_dimension.len();
        let mut result: BiVec<BiVec<Vec<Vec<FpVector>>>> =
            BiVec::with_capacity(min_degree, max_degree);

        for input_degree in min_degree..max_degree {
            let mut outputs_vec: BiVec<Vec<Vec<FpVector>>> =
                BiVec::with_capacity(input_degree, max_degree);
            // We assume our algebra is connected, so we can manually fill in the first entry.
            let number_of_inputs = graded_dimension[input_degree];
            let mut ops_vec: Vec<Vec<FpVector>> = vec![Vec::with_capacity(number_of_inputs)];
            for i in 0..number_of_inputs {
                let mut result = FpVector::new(algebra.prime(), number_of_inputs);
                result.set_entry(i, 1);
                ops_vec[0].push(result);
            }
            outputs_vec.push(ops_vec);

            for output_degree in input_degree + 1..max_degree {
                let op_deg = output_degree - input_degree;
                let number_of_operations = algebra.dimension(op_deg);
                let number_of_inputs = graded_dimension[input_degree];
                let number_of_outputs = graded_dimension[output_degree];

                outputs_vec.push(vec![
                    vec![
                        FpVector::new(algebra.prime(), number_of_outputs);
                        number_of_inputs
                    ];
                    number_of_operations
                ]);
            }
            assert!(outputs_vec.len() == max_degree);
            result.push(outputs_vec);
        }
        assert!(result.len() == max_degree);
        result
    }

    pub fn add_generator(&mut self, degree: i32, name: String) {
        let old_max_degree = self.max_degree().unwrap();
        let algebra = self.algebra();

        self.graded_dimension.extend_with(degree, |_| 0);
        self.graded_dimension[degree] += 1;

        self.gen_names.extend_with(degree, |_| Vec::new());
        self.gen_names[degree].push(name);

        let min_degree = self.graded_dimension.min_degree();
        let max_degree = self.graded_dimension.len();

        // Now allocate actions
        if old_max_degree < degree {
            self.actions.reserve((degree - old_max_degree) as usize);
            for input_degree in min_degree..max_degree {
                if input_degree <= old_max_degree {
                    self.actions[input_degree].reserve((degree - old_max_degree) as usize);
                } else {
                    self.actions
                        .push(BiVec::with_capacity(input_degree, max_degree));

                    // We assume our algebra is connected, so we can manually fill in the first entry.
                    let number_of_inputs = self.dimension(input_degree);
                    let mut ops_vec: Vec<Vec<FpVector>> =
                        vec![Vec::with_capacity(number_of_inputs)];
                    for i in 0..number_of_inputs {
                        let mut result = FpVector::new(algebra.prime(), number_of_inputs);
                        result.set_entry(i, 1);
                        ops_vec[0].push(result);
                    }
                    self.actions[input_degree].push(ops_vec);
                }

                for output_degree in std::cmp::max(input_degree + 1, old_max_degree + 1)..max_degree
                {
                    // This code is copied from allocate_actions
                    let op_deg = output_degree - input_degree;
                    let number_of_operations = algebra.dimension(op_deg);
                    let number_of_inputs = self.dimension(input_degree);
                    let number_of_outputs = self.dimension(output_degree);

                    self.actions[input_degree].push(vec![
                        vec![
                            FpVector::new(
                                algebra.prime(),
                                number_of_outputs
                            );
                            number_of_inputs
                        ];
                        number_of_operations
                    ]);
                }
            }
        } else {
            let new_dim = self.dimension(degree);

            // input_degree = degree
            for output_degree in min_degree..max_degree {
                let number_of_outputs = self.dimension(output_degree);
                // iterate over operations
                for v in &mut self.actions[degree][output_degree] {
                    v.push(FpVector::new(algebra.prime(), number_of_outputs));
                }
            }
            // output_degree = degree
            for input_degree in min_degree..max_degree {
                // iterate over operations
                for v in &mut self.actions[input_degree][degree] {
                    // Iterate over input index
                    for w in v {
                        w.extend_len(new_dim);
                    }
                }
            }

            // input_degree = output_degree = degree. We already extend everything to the right
            // dimension. We just need to set the identity action
            self.actions[degree][degree][0][new_dim - 1].set_entry(new_dim - 1, 1);
        }
    }

    pub fn string_to_basis_element(&self, string: &str) -> Option<(i32, usize)> {
        for (i, v) in self.gen_names.iter_enum() {
            for (j, n) in v.iter().enumerate() {
                if n == string {
                    return Some((i, j));
                }
            }
        }
        None
    }

    pub fn set_action(
        &mut self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
        output: &[u32],
    ) {
        assert!(operation_idx < self.algebra.dimension(operation_degree));
        assert!(input_idx < self.dimension(input_degree));
        let output_degree = input_degree + operation_degree;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector =
            &mut self.actions[input_degree][output_degree][operation_idx][input_idx];
        output_vector.copy_from_slice(output);
    }

    /// This function will panic if you call it with input such that `module.dimension(input_degree +
    /// operation_degree) = 0`.
    pub fn action(
        &self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
    ) -> &FpVector {
        let output_degree = input_degree + operation_degree;
        &self.actions[input_degree][output_degree][operation_idx][input_idx]
    }

    /// This function will panic if you call it with input such that `module.dimension(input_degree +
    /// operation_degree) = 0`.
    pub fn action_mut(
        &mut self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
    ) -> &mut FpVector {
        let output_degree = input_degree + operation_degree;
        &mut self.actions[input_degree][output_degree][operation_idx][input_idx]
    }
}

impl<M: Module> From<&M> for FiniteDimensionalModule<M::Algebra> {
    /// This should really by try_from but orphan rules prohibit this
    fn from(module: &M) -> Self {
        let min_degree = module.min_degree();
        let max_degree = module
            .max_degree()
            .expect("Can only convert to fininte dimensional module if bounded");
        module.compute_basis(max_degree);

        let mut graded_dimension = BiVec::with_capacity(min_degree, max_degree + 1);
        for t in min_degree..=max_degree {
            graded_dimension.push(module.dimension(t));
        }
        let mut result = Self::new(module.algebra(), module.to_string(), graded_dimension);
        for t in min_degree..=max_degree {
            for idx in 0..result.dimension(t) {
                result.set_basis_element_name(t, idx, module.basis_element_to_string(t, idx));
            }
        }

        let algebra = module.algebra();
        for input_degree in min_degree..=max_degree {
            for output_degree in (input_degree + 1)..=max_degree {
                let output_dimension = result.dimension(output_degree);
                if output_dimension == 0 {
                    continue;
                }
                let op_degree = output_degree - input_degree;

                for input_idx in 0..result.dimension(input_degree) {
                    for op_idx in 0..algebra.dimension(op_degree) {
                        let output_vec: &mut FpVector =
                            result.action_mut(op_degree, op_idx, input_degree, input_idx);
                        module.act_on_basis(
                            output_vec.as_slice_mut(),
                            1,
                            op_degree,
                            op_idx,
                            input_degree,
                            input_idx,
                        );
                    }
                }
            }
        }
        result
    }
}

impl<A: GeneratedAlgebra> FiniteDimensionalModule<A> {
    pub fn from_json(algebra: Arc<A>, json: &Value) -> anyhow::Result<Self> {
        let (graded_dimension, gen_names, gen_to_idx) = crate::module_gens_from_json(&json["gens"]);
        let name = json["name"].as_str().unwrap_or("").to_string();

        let mut result = Self::new(Arc::clone(&algebra), name, graded_dimension.clone());
        for (i, dim) in graded_dimension.iter_enum() {
            for j in 0..*dim {
                result.set_basis_element_name(i, j, gen_names[i][j].clone());
            }
        }

        let actions = Vec::<String>::deserialize(&json["actions"]).unwrap();
        for action in actions {
            result
                .parse_action(&gen_to_idx, &action, false)
                .with_context(|| format!("Failed to parse action: {action}"))?;
        }
        for input_degree in (result.min_degree()..=result.max_degree().unwrap()).rev() {
            for output_degree in input_degree + 1..=result.max_degree().unwrap() {
                result.extend_actions(input_degree, output_degree);
                result.check_validity(input_degree, output_degree)?;
            }
        }
        Ok(result)
    }

    pub fn to_json(&self, json: &mut Value) {
        if !self.name.is_empty() {
            json["name"] = Value::String(self.name.clone());
        }
        json["type"] = Value::from("finite dimensional module");
        json["gens"] = json!({});
        for (i, deg_i_gens) in self.gen_names.iter_enum() {
            for g in deg_i_gens {
                json["gens"][g] = Value::from(i);
            }
        }

        json["actions"] = self.actions_to_json();
    }

    pub fn parse_action(
        &mut self,
        gen_to_idx: impl for<'a> Fn(&'a str) -> anyhow::Result<(i32, usize)>,
        entry: &str,
        overwrite: bool,
    ) -> anyhow::Result<()> {
        let algebra = self.algebra();

        let (lhs, rhs) = entry
            .split_once(" = ")
            .ok_or_else(|| anyhow!("Invalid action: {entry}"))?;

        let (action, g) = lhs
            .rsplit_once(' ')
            .ok_or_else(|| anyhow!("Invalid action: {entry}"))?;

        let (op_deg, op_idx) = algebra
            .basis_element_from_string(action)
            .ok_or_else(|| anyhow!("Invalid algebra element: {action}"))?;

        let (input_deg, input_idx) = gen_to_idx(g.trim())?;

        let row = self.action_mut(op_deg, op_idx, input_deg, input_idx);

        if overwrite {
            row.set_to_zero();
        }

        if rhs == "0" {
            return Ok(());
        }

        for item in rhs.split(" + ") {
            let (coef, g) = match item.split_once(' ') {
                Some((coef, g)) => (
                    str::parse(coef)
                        .map_err(|_| anyhow!("Invalid item on right-hand side: {item}"))?,
                    g,
                ),
                None => (1, item),
            };
            let (deg, idx) = gen_to_idx(g.trim())?;
            if deg != input_deg + op_deg {
                return Err(anyhow!(
                    "Degree of {g} is {deg} but degree of LHS is {}",
                    input_deg + op_deg
                ));
            }
            row.add_basis_element(idx, coef);
        }
        Ok(())
    }

    pub fn check_validity(
        &self,
        input_deg: i32,
        output_deg: i32,
    ) -> Result<(), ModuleFailedRelationError> {
        assert!(output_deg > input_deg);
        let p = self.prime();
        let algebra = self.algebra();
        let op_deg = output_deg - input_deg;
        let mut output_vec = FpVector::new(p, self.dimension(output_deg));
        let mut tmp_output = FpVector::new(p, self.dimension(output_deg));
        for idx in 0..self.dimension(input_deg) {
            let relations = algebra.generating_relations(op_deg);
            for relation in relations {
                for &(coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                    let intermediate_dim = self.dimension(input_deg + deg_2);
                    tmp_output.set_scratch_vector_size(intermediate_dim);
                    self.act_on_basis(tmp_output.as_slice_mut(), 1, deg_2, idx_2, input_deg, idx);
                    self.act(
                        output_vec.as_slice_mut(),
                        coef,
                        deg_1,
                        idx_1,
                        deg_2 + input_deg,
                        tmp_output.as_slice(),
                    );
                }

                if !output_vec.is_zero() {
                    let mut relation_string = String::new();
                    for (coef, (deg_1, idx_1), (deg_2, idx_2)) in &relation {
                        let _ = write!(
                            relation_string,
                            "{} * {} * {}  +  ",
                            *coef,
                            &algebra.basis_element_to_string(*deg_1, *idx_1),
                            &algebra.basis_element_to_string(*deg_2, *idx_2)
                        );
                    }
                    for _ in 0..5 {
                        relation_string.pop();
                    }

                    let value_string = self.element_to_string(output_deg, output_vec.as_slice());
                    return Err(ModuleFailedRelationError {
                        relation: relation_string,
                        value: value_string,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn extend_actions(&mut self, input_deg: i32, output_deg: i32) {
        let p = self.prime();
        let algebra = self.algebra();
        let op_deg = output_deg - input_deg;
        if self.dimension(output_deg) == 0 || self.dimension(input_deg) == 0 {
            return;
        }

        let mut tmp_output = FpVector::new(p, self.dimension(output_deg));
        let generators = algebra.generators(op_deg);
        for idx in 0..self.dimension(input_deg) {
            for op_idx in 0..algebra.dimension(op_deg) {
                if !generators.contains(&op_idx) {
                    let mut output_vec = std::mem::replace(
                        &mut self.actions[input_deg][output_deg][op_idx][idx],
                        FpVector::new(p, 0),
                    );
                    let decomposition = algebra.decompose_basis_element(op_deg, op_idx);
                    for (coef, (deg_1, idx_1), (deg_2, idx_2)) in decomposition {
                        let intermediate_dim = self.dimension(input_deg + deg_2);
                        if intermediate_dim > tmp_output.len() {
                            tmp_output = FpVector::new(p, intermediate_dim);
                        }
                        self.act_on_basis(
                            tmp_output.slice_mut(0, intermediate_dim),
                            1,
                            deg_2,
                            idx_2,
                            input_deg,
                            idx,
                        );
                        self.act(
                            output_vec.as_slice_mut(),
                            coef,
                            deg_1,
                            idx_1,
                            deg_2 + input_deg,
                            tmp_output.slice(0, intermediate_dim),
                        );
                        tmp_output.set_to_zero();
                    }
                    let _ = std::mem::replace(
                        &mut self.actions[input_deg][output_deg][op_idx][idx],
                        output_vec,
                    );
                }
            }
        }
    }

    fn actions_to_json(&self) -> Value {
        let algebra = self.algebra();
        let min_degree = self.min_degree();
        let max_degree = self.graded_dimension.len();
        let mut actions = Vec::new();
        for input_degree in min_degree..max_degree {
            for output_degree in (input_degree + 1)..max_degree {
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
                        actions.push(format!(
                            "{} {} = {}",
                            algebra.generator_to_string(op_degree, op_idx),
                            self.gen_names[input_degree][input_idx],
                            self.element_to_string(output_degree, vec.as_slice())
                        ))
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
    use crate::algebra::AdemAlgebra;

    #[test]
    fn test_module_check_validity() {
        let p = fp::prime::ValidPrime::new(2);
        let adem_algebra = Arc::new(AdemAlgebra::new(p, false));
        adem_algebra.compute_basis(10);
        let mut adem_module = FiniteDimensionalModule::new(
            Arc::clone(&adem_algebra),
            "".to_string(),
            BiVec::from_vec(0, vec![1, 2, 1]),
        );
        adem_module.set_basis_element_name(0, 0, "x0".to_string());
        adem_module.set_basis_element_name(1, 0, "x10".to_string());
        adem_module.set_basis_element_name(1, 1, "x11".to_string());
        adem_module.set_basis_element_name(2, 0, "x2".to_string());
        adem_module.set_action(1, 0, 0, 0, &[1, 1]);
        adem_module.set_action(1, 0, 1, 0, &[1]);
        adem_module.set_action(1, 0, 1, 1, &[1]);
        adem_module.set_action(2, 0, 0, 0, &[1]);
        adem_module.check_validity(0, 2).unwrap();
    }
}
