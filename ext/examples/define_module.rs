use rustc_hash::FxHashMap as HashMap;
use std::io::{stderr, Write};
use std::sync::Arc;

use serde_json::json;
use serde_json::Value;

use algebra::module::{FDModule, FPModule, FreeModule, Module};
use algebra::steenrod_evaluator::evaluate_module;
use algebra::{AdemAlgebra, Algebra, GeneratedAlgebra, MilnorAlgebra, SteenrodAlgebra};
use bivec::BiVec;
use fp::prime::ValidPrime;
use fp::vector::FpVector;

pub fn get_gens() -> error::Result<BiVec<Vec<String>>> {
    // Query for generators
    eprintln!("Input generators. Press return to finish.");
    stderr().flush()?;

    let mut gens: BiVec<Vec<_>> = BiVec::new(0);
    loop {
        let gen_deg: Option<i32> = query::optional("Generator degree", str::parse);
        if gen_deg.is_none() {
            eprintln!("This is the list of generators and degrees:");
            for (i, deg_i_gens) in gens.iter_enum() {
                for gen in deg_i_gens.iter() {
                    eprint!("({}, {}) ", i, gen);
                }
            }
            eprintln!();
            if query::yes_no("Is it okay?") {
                break;
            } else {
                if query::yes_no("Start over?") {
                    gens = BiVec::new(0);
                }
                continue;
            }
        }

        let gen_deg = gen_deg.unwrap();

        gens.extend_negative(gen_deg, Vec::new());
        gens.extend_with(gen_deg, |_| Vec::new());

        let gen_name = query::with_default(
            "Generator name",
            &format!("x{}{}", gen_deg, gens[gen_deg].len()).replace('-', "_"),
            |x| {
                match x.chars().next() {
                    Some(a) => {
                        if !a.is_alphabetic() {
                            return Err("variable name must start with a letter".to_string());
                        }
                    }
                    None => return Err("Variable name cannot be empty".to_string()),
                };
                for c in x.chars() {
                    if !c.is_alphanumeric() && c != '_' {
                        return Err(format!(
                            "Variable name cannot contain {}. Should be alphanumeric and '_'",
                            c
                        ));
                    }
                }
                Ok(x.to_string())
            },
        );
        gens[gen_deg].push(gen_name);
    }
    Ok(gens)
}

pub fn gens_to_json(gens: &BiVec<Vec<String>>) -> serde_json::Value {
    let mut gens_json = json!({});
    for (i, deg_i_gens) in gens.iter_enum() {
        for gen in deg_i_gens {
            gens_json[gen] = json!(i);
        }
    }
    gens_json
}

pub fn get_expression_to_vector<F>(
    prompt: &str,
    output_vec: &mut FpVector,
    string_to_basis_element: F,
) where
    F: for<'a> Fn(&'a str) -> Option<usize>,
{
    'outer: loop {
        let result: String = query::raw(prompt, str::parse);
        if result == "0" {
            output_vec.set_to_zero();
            break;
        }
        for term in result.split('+') {
            let term = term.trim();
            let parts: Vec<&str> = term.splitn(2, ' ').collect();
            if parts.len() == 1 {
                match string_to_basis_element(parts[0]) {
                    Some(i) => output_vec.add_basis_element(i, 1),
                    None => {
                        eprintln!("Invalid value. Try again");
                        continue 'outer;
                    }
                };
            } else {
                let mut rest = &parts[1];
                let coef = match parts[0].parse::<u32>() {
                    Ok(c) => c,
                    _ => {
                        rest = &term;
                        1
                    }
                };
                let gen_idx = match string_to_basis_element(rest) {
                    Some(i) => i,
                    None => {
                        eprintln!("Invalid value. Try again");
                        continue 'outer;
                    }
                };
                output_vec.add_basis_element(gen_idx, coef);
            }
        }
        return;
    }
}

pub fn interactive_module_define_fdmodule(
    output_json: &mut Value,
    p: ValidPrime,
    generic: bool,
    name: String,
) -> error::Result {
    let algebra = Arc::new(SteenrodAlgebra::AdemAlgebra(AdemAlgebra::new(
        p, generic, false, false,
    )));

    let gens = get_gens()?;
    let min_degree = gens.min_degree();
    let max_degree = gens.len();

    algebra.compute_basis(max_degree - min_degree);

    let mut graded_dim = BiVec::with_capacity(min_degree, max_degree);
    for i in gens.iter().map(Vec::len) {
        graded_dim.push(i);
    }

    let mut module = FDModule::new(Arc::clone(&algebra), name, graded_dim);

    for (i, deg_i_gens) in gens.iter_enum() {
        for (j, gen) in deg_i_gens.iter().enumerate() {
            module.set_basis_element_name(i as i32, j, gen.to_string());
        }
    }

    eprintln!("Input actions. Write the value of the action in the form 'a x0 + b x1 + ...' where a, b are non-negative integers and x0, x1 are names of the generators. The coefficient can be omitted if it is 1");

    let len = gens.len();
    for input_deg in gens.range().rev() {
        for output_deg in (input_deg + 1)..len as i32 {
            let op_deg = output_deg - input_deg;
            let input_deg_idx = input_deg;
            let output_deg_idx = output_deg;
            if gens[output_deg_idx].is_empty() {
                continue;
            }
            for op_idx in algebra.generators(op_deg) {
                let mut output_vec = FpVector::new(p, gens[output_deg_idx].len());
                let callback = |string: &str| gens[output_deg_idx].iter().position(|d| d == string);
                for input_idx in 0..gens[input_deg_idx].len() {
                    get_expression_to_vector(
                        &format!(
                            "{} {}",
                            algebra.basis_element_to_string(op_deg, op_idx),
                            gens[input_deg_idx][input_idx]
                        ),
                        &mut output_vec,
                        callback,
                    );
                    module.set_action_vector(op_deg, op_idx, input_deg, input_idx, &output_vec);
                    output_vec.set_to_zero();
                }
            }
            module.extend_actions(input_deg, output_deg);
            module.check_validity(input_deg, output_deg)?;
        }
    }

    algebra.to_json(output_json);
    module.to_json(output_json);
    Ok(())
}

fn get_relation(
    adem_algebra: &AdemAlgebra,
    milnor_algebra: &MilnorAlgebra,
    module: &FreeModule<SteenrodAlgebra>,
    basis_elt_lookup: &HashMap<String, (i32, usize)>,
) -> Result<(i32, FpVector), String> {
    let relation: String = query::raw("Relation", str::parse);
    if relation.is_empty() {
        return Err("".to_string());
    }
    evaluate_module(
        adem_algebra,
        milnor_algebra,
        module,
        basis_elt_lookup,
        &relation,
    )
    .map_err(|err| err.to_string())
}

pub fn interactive_module_define_fpmodule(
    output_json: &mut Value,
    p: ValidPrime,
    generic: bool,
    name: String,
) -> error::Result {
    output_json["type"] = Value::from("finitely presented module");

    let gens = get_gens()?;
    let min_degree = gens.min_degree();
    let max_degree = 20;

    let steenrod_algebra = Arc::new(SteenrodAlgebra::AdemAlgebra(AdemAlgebra::new(
        p, generic, false, false,
    )));
    let adem_algebra = AdemAlgebra::new(p, generic, false, false);
    let milnor_algebra = MilnorAlgebra::new(p);

    steenrod_algebra.compute_basis(max_degree - min_degree);
    adem_algebra.compute_basis(max_degree - min_degree);
    milnor_algebra.compute_basis(max_degree - min_degree);

    let mut graded_dim = BiVec::with_capacity(min_degree, max_degree);
    for i in gens.iter().map(Vec::len) {
        graded_dim.push(i);
    }

    let mut adem_module = FPModule::new(Arc::clone(&steenrod_algebra), name, min_degree);

    for (i, deg_i_gens) in gens.iter_enum() {
        adem_module.add_generators(i, deg_i_gens.clone());
    }
    // TODO: make relation parser automatically extend module by zero if necessary...
    adem_module.generators().extend_by_zero(20);

    eprintln!("Input relations");
    match *p {
        2 => eprintln!("Write relations in the form 'Sq6 * Sq2 * x + Sq7 * y'"),
        _ => eprintln!("Write relations in the form 'Q5 * P(5) * x + 2 * P(1, 3) * Q2 * y', where P(...) and Qi are Milnor basis elements."),
    }
    eprintln!("There is currently a hard-coded maximum degree of {} for relations (this is the maximum allowed degree of an operator acting on the generators). One can raise this number by editing the max_degree variable in the interactive_module_define_fpmodule function of src/cli_module_loaders.rs. Apologies.", max_degree);

    let mut basis_elt_lookup = HashMap::default();
    for (i, deg_i_gens) in gens.iter_enum() {
        for (j, gen) in deg_i_gens.iter().enumerate() {
            let k = adem_module
                .generators()
                .operation_generator_to_index(0, 0, i, j);
            basis_elt_lookup.insert(gen.clone(), (i, k));
        }
    }

    let mut relations: BiVec<Vec<FpVector>> = BiVec::new(min_degree);
    loop {
        match get_relation(
            &adem_algebra,
            &milnor_algebra,
            &adem_module.generators(),
            &basis_elt_lookup,
        ) {
            Err(x) => {
                if x.is_empty() {
                    eprintln!("Invalid relation: {}. Try again.", x);
                    continue;
                }
                eprintln!("This is the list of relations:");
                for (i, deg_i_relns) in relations.iter_enum() {
                    for r in deg_i_relns {
                        print!(
                            "{}, ",
                            adem_module.generators().element_to_string(i, r.as_slice())
                        );
                    }
                }
                eprintln!();
                if query::yes_no("Is it okay?") {
                    break;
                } else {
                    if query::yes_no("Start over?") {
                        relations = BiVec::new(min_degree);
                    }
                    continue;
                }
            }
            Ok((degree, vector)) => {
                while relations.len() <= degree {
                    relations.push(Vec::new());
                }
                relations[degree].push(vector);
            }
        }
    }

    for (i, relns) in relations.iter_enum() {
        let dim = adem_module.generators().dimension(i);
        let mut matrix = fp::matrix::Matrix::new(p, relns.len(), dim);
        for (j, r) in relns.iter().enumerate() {
            matrix[j].assign(r);
        }
        adem_module.add_relations(i, &mut matrix);
    }
    steenrod_algebra.to_json(output_json);
    adem_module.to_json(output_json);
    Ok(())
}

fn main() -> error::Result {
    let module_type = query::with_default(
        "Input module type (default 'finite dimensional module'):\n (fd) - finite dimensional module \n (fp) - finitely presented module\n",
        "fd",
        |x| match x {
            "fd" | "fp" => Ok(x.to_string()),
            _ => Err(format!("Invalid type '{}'. Type must be 'fd' or 'fp'", x))
        }
    );

    let name: String = query::raw("Module name (use latex between $'s)", str::parse);
    let p: ValidPrime = query::with_default("p", "2", str::parse);
    let generic = *p != 2;
    let mut output_json = json!({});

    eprintln!("module_type: {}", module_type);
    match &*module_type {
        "fd" => interactive_module_define_fdmodule(&mut output_json, p, generic, name)?,
        "fp" => interactive_module_define_fpmodule(&mut output_json, p, generic, name)?,
        _ => unreachable!(),
    }
    println!("{}", output_json);
    Ok(())
}
