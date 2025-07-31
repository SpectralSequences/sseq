use std::{
    io::{stderr, Write},
    sync::Arc,
};

use algebra::{
    module::FDModule, steenrod_evaluator::SteenrodEvaluator, AdemAlgebra, Algebra, GeneratedAlgebra,
};
use anyhow::anyhow;
use bivec::BiVec;
use fp::{
    prime::{Prime, ValidPrime},
    vector::FpVector,
};
use rustc_hash::FxHashMap as HashMap;
use serde_json::{json, Value};

pub fn get_gens() -> anyhow::Result<BiVec<Vec<String>>> {
    ext::utils::init_logging()?;

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
                    eprint!("({i}, {gen}) ");
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
            &format!("x{gen_deg}{}", gens[gen_deg].len()).replace('-', "_"),
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
                            "Variable name cannot contain {c}. Should be alphanumeric and '_'"
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

pub fn interactive_module_define_fdmodule(
    output_json: &mut Value,
    p: ValidPrime,
) -> anyhow::Result<()> {
    output_json["p"] = Value::from(p.as_u32());
    let algebra = Arc::new(AdemAlgebra::new(p, false));

    let gens = get_gens()?;
    let min_degree = gens.min_degree();
    let max_degree = gens.len();

    algebra.compute_basis(max_degree - min_degree);

    let mut graded_dim = BiVec::with_capacity(min_degree, max_degree);
    for i in gens.iter().map(Vec::len) {
        graded_dim.push(i);
    }

    let mut module = FDModule::new(Arc::clone(&algebra), String::new(), graded_dim);

    for (i, deg_i_gens) in gens.iter_enum() {
        for (j, gen) in deg_i_gens.iter().enumerate() {
            module.set_basis_element_name(i, j, gen.to_string());
        }
    }

    eprintln!(
        "Input actions. Write the value of the action in the form 'a x0 + b x1 + ...' where a, b \
         are non-negative integers and x0, x1 are names of the generators. The coefficient can be \
         omitted if it is 1"
    );

    let len = gens.len();
    for input_deg in gens.range().rev() {
        for output_deg in (input_deg + 1)..len {
            let op_deg = output_deg - input_deg;
            if gens[output_deg].is_empty() {
                continue;
            }
            for op_idx in algebra.generators(op_deg) {
                for input_idx in 0..gens[input_deg].len() {
                    let output = query::raw(
                        &format!(
                            "{} {}",
                            algebra.basis_element_to_string(op_deg, op_idx),
                            gens[input_deg][input_idx]
                        ),
                        |expr| {
                            let mut result = vec![0; gens[output_deg].len()];
                            if expr == "0" {
                                return Ok(result);
                            }
                            for term in expr.split('+') {
                                let term = term.trim();
                                let (coef, gen) = match term.split_once(' ') {
                                    Some((coef, gen)) => (str::parse::<u32>(coef)?, gen),
                                    None => (1, term),
                                };

                                if let Some(gen_idx) =
                                    gens[output_deg].iter().position(|d| d == gen)
                                {
                                    result[gen_idx] += coef;
                                } else {
                                    return Err(anyhow!(
                                        "No generator {gen} in degree {output_deg}"
                                    ));
                                }
                            }

                            Ok(result)
                        },
                    );

                    module.set_action(op_deg, op_idx, input_deg, input_idx, &output);
                }
            }
            module.extend_actions(input_deg, output_deg);
            module.check_validity(input_deg, output_deg)?;
        }
    }

    module.to_json(output_json);
    Ok(())
}

/// Given a string representation of an element in an algebra together with a generator, multiply
/// each term on the right with the generator.
fn replace(algebra_elt: &str, gen: &str) -> String {
    algebra_elt.replace('+', &format!("{gen} +")) + " " + gen
}

pub fn interactive_module_define_fpmodule(
    output_json: &mut Value,
    p: ValidPrime,
) -> anyhow::Result<()> {
    let gens = get_gens()?;
    let min_degree = gens.min_degree();
    let max_degree = gens.len();

    let ev = SteenrodEvaluator::new(p);

    let mut graded_dim = BiVec::with_capacity(min_degree, max_degree);
    for i in gens.iter().map(Vec::len) {
        graded_dim.push(i);
    }

    eprintln!("Input relations");
    match p.as_u32() {
        2 => eprintln!("Write relations in the form 'Sq6 * Sq2 * x + Sq7 * y'"),
        _ => eprintln!(
            "Write relations in the form 'Q5 * P(5) * x + 2 * P(1, 3) * Q2 * y', where P(...) and \
             Qi are Milnor basis elements."
        ),
    }

    let mut degree_lookup = HashMap::default();
    for (i, deg_i_gens) in gens.iter_enum() {
        for gen in deg_i_gens.iter() {
            degree_lookup.insert(gen.clone(), i);
        }
    }

    let mut adem_relations = Vec::new();
    let mut milnor_relations = Vec::new();

    loop {
        let relation = query::raw("Enter relation", |rel| {
            let result = ev.evaluate_module_adem(rel)?;

            if result.is_empty() {
                return Ok(result);
            }

            // Check that the generators exist and the terms all have the same degree
            let mut deg = None;
            for (gen, (op_deg, _)) in result.iter() {
                let cur_deg = op_deg
                    + degree_lookup
                        .get(gen)
                        .ok_or_else(|| anyhow!("Unknown generator: {gen}"))?;
                if deg.is_none() {
                    deg = Some(cur_deg);
                } else if deg != Some(cur_deg) {
                    return Err(anyhow!(
                        "Relation terms have different degrees: {} and {cur_deg}",
                        deg.unwrap()
                    ));
                }
            }

            Ok(result)
        });

        if relation.is_empty() {
            break;
        }

        let mut adem_relation = String::new();
        let mut milnor_relation = String::new();

        let mut milnor_op = FpVector::new(p, 0);
        for (gen, (op_deg, adem_op)) in relation.iter() {
            if adem_op.is_zero() {
                continue;
            }
            if !adem_relation.is_empty() {
                adem_relation += " + ";
                milnor_relation += " + ";
            }
            milnor_op.set_scratch_vector_size(adem_op.len());
            ev.adem_to_milnor(&mut milnor_op, 1, *op_deg, adem_op);

            adem_relation += &replace(&ev.adem.element_to_string(*op_deg, adem_op.as_slice()), gen);
            milnor_relation += &replace(
                &ev.milnor.element_to_string(*op_deg, milnor_op.as_slice()),
                gen,
            );
        }
        if !adem_relation.is_empty() {
            adem_relations.push(Value::String(adem_relation));
            milnor_relations.push(Value::String(milnor_relation));
        }
    }

    output_json["p"] = Value::from(p.as_u32());
    output_json["type"] = Value::String("finitely presented module".to_owned());
    for (i, deg_i_gens) in gens.iter_enum() {
        for gen in deg_i_gens {
            output_json["gens"][gen] = Value::from(i);
        }
    }
    output_json["adem_relations"] = Value::Array(adem_relations);
    output_json["milnor_relations"] = Value::Array(milnor_relations);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let module_type = query::with_default(
        "Input module type (default 'finite dimensional module'):\n (fd) - finite dimensional \
         module \n (fp) - finitely presented module\n",
        "fd",
        |x| match x {
            "fd" | "fp" => Ok(x.to_string()),
            _ => Err(format!("Invalid type '{x}'. Type must be 'fd' or 'fp'")),
        },
    );

    let p: ValidPrime = query::with_default("p", "2", str::parse);
    let mut output_json = json!({});

    eprintln!("module_type: {module_type}");
    match &*module_type {
        "fd" => interactive_module_define_fdmodule(&mut output_json, p)?,
        "fp" => interactive_module_define_fpmodule(&mut output_json, p)?,
        _ => unreachable!(),
    }
    println!("{output_json}");
    Ok(())
}
