use crate::algebra::Algebra;
use crate::module::{BoundedModule, Module, ZeroModule};
use bivec::BiVec;
use fp::vector::{FpVector, SliceMut};
use std::sync::Arc;

#[cfg(feature = "json")]
use {
    crate::algebra::{GeneratedAlgebra, JsonAlgebra},
    crate::module::ModuleFailedRelationError,
    error::GenericError,
    nom::{
        branch::alt,
        bytes::complete::{is_not, take},
        character::complete::{char, digit1, space0, space1},
        combinator::map,
        multi::separated_list0,
        sequence::delimited,
        sequence::tuple,
        IResult,
    },
    rustc_hash::FxHashMap as HashMap,
    serde::Deserialize,
    serde_json::{json, value::Value},
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
        self.graded_dimension == other.graded_dimension && self.actions == other.actions
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
            if self.graded_dimension.len() != other.graded_dimension.len() {
                return Err(format!(
                    "Graded dimension lengths disagree. left.len() = {} but right.len() = {}.",
                    self.graded_dimension.len(),
                    other.graded_dimension.len()
                ));
            }
            let mut disagreements = vec![];
            for i in self.graded_dimension.min_degree()..self.graded_dimension.len() {
                if self.graded_dimension[i] != other.graded_dimension[i] {
                    disagreements.push(i);
                }
            }
            return Err(format!("Graded dimensions disagree in positions {:?}. Left has graded dimensions:\n    {:?}\nRight has graded dimension:\n    {:?}\n",
                disagreements,
                self.graded_dimension,
                other.graded_dimension
            ));
        }
        if self.actions != other.actions {
            // actions goes input_degree --> output_degree --> operation --> input_index --> Vector
            let mut disagreements = vec![];
            for input_degree in self.actions.min_degree()..self.actions.len() {
                for output_degree in
                    self.actions[input_degree].min_degree()..self.actions[input_degree].len()
                {
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

            let mut err_string = "Actions disagree.\n".to_string();
            for x in disagreements {
                err_string.push_str(&format!(
                    "  {} * {} disagreement.\n    Left: {}\n    Right: {}\n",
                    self.algebra.basis_element_to_string(x.1 - x.0, x.2),
                    self.basis_element_to_string(x.0, x.3),
                    self.element_to_string(x.1, x.4.as_slice()),
                    self.element_to_string(x.1, x.5.as_slice())
                ))
            }
            return Err(err_string);
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
        i32::max_value()
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

    fn is_unit(&self) -> bool {
        self.min_degree() == 0 && self.graded_dimension.len() == 1 && self.graded_dimension[0] == 1
    }

    fn act_on_basis(
        &self,
        mut result: SliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
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
        result.add(output.as_slice(), coeff);
    }

    fn borrow_output(&self) -> bool {
        true
    }

    fn act_on_basis_borrow(
        &self,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> &FpVector {
        self.action(op_degree, op_index, mod_degree, mod_index)
    }
}

impl<A: Algebra> BoundedModule for FiniteDimensionalModule<A> {
    fn max_degree(&self) -> i32 {
        self.graded_dimension.max_degree()
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
                names.push(format!("x{}_{}", i, j));
            }
            gen_names.push(names);
        }
        let actions = FiniteDimensionalModule::allocate_actions(&algebra, &graded_dimension);
        FiniteDimensionalModule {
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
                let number_of_operations = algebra.dimension(op_deg, min_degree + input_degree);
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
        let old_max_degree = self.max_degree();
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
                    let number_of_operations = algebra.dimension(op_deg, min_degree + input_degree);
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
                    return Some((i as i32, j));
                }
            }
        }
        None
    }

    pub fn set_action_vector(
        &mut self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
        output: &FpVector,
    ) {
        assert!(operation_idx < self.algebra.dimension(operation_degree, input_degree));
        assert!(input_idx < self.dimension(input_degree));
        let output_degree = input_degree + operation_degree;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector =
            &mut self.actions[input_degree][output_degree][operation_idx][input_idx];
        output_vector.assign(output);
    }

    pub fn set_action(
        &mut self,
        operation_degree: i32,
        operation_idx: usize,
        input_degree: i32,
        input_idx: usize,
        output: Vec<u32>,
    ) {
        assert!(operation_idx < self.algebra.dimension(operation_degree, input_degree));
        assert!(input_idx < self.dimension(input_degree));
        let output_degree = input_degree + operation_degree;
        // (in_deg) -> (out_deg) -> (op_index) -> (in_index) -> Vector
        let output_vector =
            &mut self.actions[input_degree][output_degree][operation_idx][input_idx];
        output_vector.copy_from_slice(&output);
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

#[cfg(feature = "json")]
impl<A: JsonAlgebra + GeneratedAlgebra> FiniteDimensionalModule<A> {
    fn module_gens_from_json(
        gens: &Value,
    ) -> (
        BiVec<usize>,
        BiVec<Vec<String>>,
        HashMap<String, (i32, usize)>,
    ) {
        let gens = gens.as_object().unwrap();

        let degrees = gens
            .iter()
            .map(|(_, x)| x.as_i64().unwrap() as i32)
            .collect::<Vec<_>>();

        let min_degree = degrees.iter().copied().min().unwrap_or(0);
        let max_degree = degrees.iter().copied().max().unwrap_or(-1) + 1;

        let mut gen_to_idx = HashMap::default();
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

    pub fn from_json(algebra: Arc<A>, json: &Value) -> error::Result<Self> {
        let (graded_dimension, gen_names, gen_to_idx) = Self::module_gens_from_json(&json["gens"]);
        let name = json["name"].as_str().unwrap_or("").to_string();

        let mut result = Self::new(Arc::clone(&algebra), name, graded_dimension.clone());
        for (i, dim) in graded_dimension.iter_enum() {
            for j in 0..*dim {
                result.set_basis_element_name(i, j, gen_names[i][j].clone());
            }
        }

        if let Ok(actions) = Vec::<String>::deserialize(&json["actions"]) {
            for action in actions {
                result.parse_action(&gen_to_idx, &action, false)?;
            }
            for input_degree in (result.min_degree()..=result.max_degree()).rev() {
                for output_degree in input_degree + 1..=result.max_degree() {
                    result.extend_actions(input_degree, output_degree);
                    result.check_validity(input_degree, output_degree)?;
                }
            }
        } else {
            #[derive(Deserialize)]
            struct OutputStruct {
                gen: String,
                coeff: u32,
            }
            #[derive(Deserialize)]
            struct ActionStruct {
                op: Value,
                input: String,
                output: Vec<OutputStruct>,
            }

            let actions_value = &json[format!("{}_actions", algebra.prefix())];
            let actions: Vec<ActionStruct> = <_>::deserialize(actions_value)?;
            for action in actions {
                let (degree, idx) = algebra.json_to_basis(&action.op)?;
                let input = action.input;
                let (input_degree, input_idx) = *gen_to_idx
                    .get(&input)
                    .ok_or_else(|| GenericError::new(format!("Invalid generator: {}", input)))?;
                let output_vec = result.action_mut(degree, idx, input_degree, input_idx);
                for basis_elt in action.output {
                    let gen = basis_elt.gen;
                    let (_, output_idx) = *gen_to_idx.get(&gen).ok_or_else(move || {
                        GenericError::new(format!("Invalid generator: {}", gen))
                    })?;
                    output_vec.add_basis_element(output_idx, basis_elt.coeff);
                }
            }
        }
        Ok(result)
    }

    pub fn to_json(&self, json: &mut Value) {
        json["name"] = Value::String(self.to_string());
        json["type"] = Value::from("finite dimensional module");
        json["gens"] = json!({});
        for (i, deg_i_gens) in self.gen_names.iter_enum() {
            for gen in deg_i_gens {
                json["gens"][gen] = Value::from(i);
            }
        }

        json["actions"] = self.actions_to_json();
    }

    pub fn parse_action(
        &mut self,
        gen_to_idx: &HashMap<String, (i32, usize)>,
        entry_: &str,
        overwrite: bool,
    ) -> error::Result {
        let algebra = self.algebra();
        let mut lhs = tuple((
            |e| algebra.string_to_generator(e),
            is_not("="),
            take(1usize),
        ));

        let (entry, ((op_deg, op_idx), gen, _)) =
            lhs(entry_).map_err(|_err| GenericError::new(format!("Invalid action: {}", entry_)))?;

        let (input_deg, input_idx) = *gen_to_idx
            .get(gen.trim())
            .ok_or_else(|| GenericError::new(format!("Invalid generator: {}", gen.trim())))?;

        let row = self.action_mut(op_deg, op_idx, input_deg, input_idx);

        if overwrite {
            row.set_to_zero();
        }

        if let IResult::<_, _>::Ok(("", _)) = delimited(space0, char('0'), space0)(entry) {
            return Ok(());
        }

        // Need explicit type here
        let (_, values) =
            <IResult<_, _>>::unwrap(separated_list0(take(1usize), is_not("+"))(entry));

        for value in values {
            let (_, (coef, gen)) = Self::take_element(value)
                .map_err(|_| GenericError::new(format!("Invalid action: {}", entry_)))?;

            let (deg, idx) = *gen_to_idx
                .get(gen)
                .ok_or_else(|| GenericError::new(format!("Invalid generator: {}", gen)))?;
            if deg != input_deg + op_deg {
                return Err(GenericError::new(format!("Invalid action: {}", entry_)).into());
            }

            row.add_basis_element(idx, coef);
        }
        Ok(())
    }

    pub fn parse_element(&self, entry: &str, degree: i32, mut result: SliceMut) -> error::Result {
        if let IResult::<_, _>::Ok(("", _)) = delimited(space0, char('0'), space0)(entry) {
            return Ok(());
        }
        for elt in entry.split('+') {
            if let Ok(("", (coef, gen))) = Self::take_element(elt.trim_end()) {
                if let Some(idx) = self.gen_names[degree].iter().position(|x| x == gen) {
                    result.add_basis_element(idx, coef);
                } else {
                    return Err(GenericError::new(format!("Invalid generator: {}", elt)).into());
                }
            } else {
                return Err(GenericError::new(format!("Invalid term: {}", elt)).into());
            }
        }
        Ok(())
    }

    fn take_element(i: &str) -> IResult<&str, (u32, &str)> {
        // coefficient, name
        let coef_gen = map(
            tuple((space0, digit1, space1, is_not(" "))),
            |(_, coef, _, gen): (_, &str, _, &str)| (coef.parse().unwrap(), gen),
        );
        let o_gen = map(tuple((space0, is_not(" "))), |(_, gen)| (1, gen));
        alt((coef_gen, o_gen))(i)
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
                        relation_string.push_str(&format!(
                            "{} * {} * {}  +  ",
                            *coef,
                            &algebra.basis_element_to_string(*deg_1, *idx_1),
                            &algebra.basis_element_to_string(*deg_2, *idx_2)
                        ));
                    }
                    for _ in 0..5 {
                        relation_string.pop();
                    }

                    let value_string =
                        self.element_to_string(output_deg as i32, output_vec.as_slice());
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

        let mut output_vec = FpVector::new(p, self.dimension(output_deg));
        let mut tmp_output = FpVector::new(p, self.dimension(output_deg));
        let generators = algebra.generators(op_deg);
        for idx in 0..self.dimension(input_deg) {
            for op_idx in 0..algebra.dimension(op_deg, -1) {
                if !generators.contains(&op_idx) {
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
            "p": *self.prime(),
            "algebra": self.algebra().prefix(),
            "min_degree": self.min_degree(),
            "graded_dimension": self.graded_dimension,
            "actions": self.minimal_actions_to_json(),
        })
    }

    pub fn actions_to_json(&self) -> Value {
        let algebra = self.algebra();
        let min_degree = self.min_degree();
        let max_degree = self.graded_dimension.len() as i32;
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

    pub fn gens_to_json(&self) -> Value {
        let mut gens = json!({});
        for (i, names) in self.gen_names.iter_enum() {
            for name in names {
                gens[name] = Value::from(i);
            }
        }
        gens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{AdemAlgebra, SteenrodAlgebra};
    use bivec::BiVec;

    #[test]
    fn test_module_check_validity() {
        let p = fp::prime::ValidPrime::new(2);
        let adem_algebra = Arc::new(SteenrodAlgebra::from(AdemAlgebra::new(
            p,
            *p != 2,
            false,
            false,
        )));
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
        adem_module.set_action_vector(1, 0, 0, 0, &FpVector::from_slice(p, &[1, 1]));
        adem_module.set_action_vector(1, 0, 1, 0, &FpVector::from_slice(p, &[1]));
        adem_module.set_action_vector(1, 0, 1, 1, &FpVector::from_slice(p, &[1]));
        adem_module.set_action_vector(2, 0, 0, 0, &FpVector::from_slice(p, &[1]));
        adem_module.check_validity(0, 2).unwrap();
    }
}
