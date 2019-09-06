use std::error::Error;
use std::fmt::Display;
use std::io::{stdin, stdout, Write};
use std::rc::Rc;
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;

use bivec::BiVec;
use crate::fp_vector::{FpVector,FpVectorT};
use crate::algebra::{Algebra, AlgebraAny};
use crate::milnor_algebra::MilnorAlgebra;
use crate::adem_algebra::AdemAlgebra;
use crate::module::Module;
use crate::free_module::FreeModule;
use crate::finitely_presented_module::FinitelyPresentedModule as FPModule;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;
use crate::steenrod_evaluator::evaluate_module;

fn query<S : Display, T : FromStr, F>(prompt : &str, validator : F) -> S 
    where F: Fn(T) -> Result<S, String>,
        <T as FromStr>::Err: Display  {
    loop {
        print!("{} : ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        let result = 
            trimmed.parse::<T>()
                   .map_err(|err| format!("{}", err))
                   .and_then(|res| validator(res));
        match result {
            Ok(res) => {
                return res;
            }, 
            Err(e) => {
                println!("Invalid input: {}. Try again", e);
            }
        }
    }
}

fn query_with_default<S : Display, T : FromStr + Display, F>(prompt : &str, default : S, validator : F) -> S
    where F: Fn(T) -> Result<S, String>,
        <T as std::str::FromStr>::Err: std::fmt::Display {
    query_with_default_no_default_indicated(&format!("{} (default : {})", prompt, default), default, validator)
}

fn query_with_default_no_default_indicated<S : Display, T : FromStr, F>(prompt : &str, default : S, validator : F) -> S 
    where F: Fn(T) -> Result<S, String>,
        <T as std::str::FromStr>::Err: std::fmt::Display  {
    loop {
        print!("{} : ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        if trimmed.len() == 0 {
            return default;
        }
        let result = 
            trimmed.parse::<T>()
                   .map_err(|err| format!("{}", err))
                   .and_then(|res| validator(res));
        match result {
            Ok(res) => {
                return res;
            }, 
            Err(e) => {
                println!("Invalid input: {}. Try again", e);
            }
        }
    }
}

fn query_yes_no(prompt : &str) -> bool {
    query(prompt, 
        |response : String| if response.starts_with("y") || response.starts_with("n") {
            Ok(response.starts_with("y"))
        } else {
            Err(format!("unrecognized response '{}'. Should be '(y)es' or '(n)o'", response))
        }
    )
}

pub fn get_gens(min_degree : i32) -> Result<BiVec<Vec<String>>, Box<dyn Error>>{
    // Query for generators
    println!("Input generators. Press return to finish.");
    stdout().flush()?;

    let mut gens : BiVec<Vec<_>> = BiVec::new(min_degree);
    let finished_degree = i32::max_value();
    loop {
        let gen_deg = query_with_default_no_default_indicated("Generator degree", finished_degree, |x : i32| Ok(x));
        if gen_deg == finished_degree {
            println!("This is the list of generators and degrees:");
            for (i, deg_i_gens) in gens.iter_enum() {
                for gen in deg_i_gens.iter(){
                    print!("({}, {}) ", i, gen);
                }
            }
            print!("\n");
            if query_yes_no("Is it okay?") {
                break;
            } else {
                if query_yes_no("Start over?") {
                    gens = BiVec::new(min_degree);
                }
                continue;
            }
        }
        while gens.len() <= gen_deg {
            gens.push(Vec::new());
        }
        let gen_name = query_with_default("Generator name", format!("x{}{}",gen_deg, gens[gen_deg].len()), 
            |x : String| {
                match x.chars().next() {
                    Some(a) => if !a.is_alphabetic() {
                        return Err("variable name must start with a letter".to_string())
                    },
                    None => return Err("Variable name cannot be empty".to_string())
                };
                for c in x.chars() {
                    if !c.is_alphanumeric() && c != '_' {
                        return Err(format!("Variable name cannot contain {}. Should be alphanumeric and '_'", c));
                    }
                }
                return Ok(x);
            }
        );
        gens[gen_deg].push(gen_name);
    }
    Ok(gens)
}

pub fn gens_to_json(gens : &BiVec<Vec<String>>) -> serde_json::Value {
    let mut gens_json = json!({});
    for (i, deg_i_gens) in gens.iter_enum() {
        for gen in deg_i_gens {
            gens_json[gen] = json!(i);
        }
    }
    return gens_json;
}

pub fn get_expression_to_vector<F>(
    prompt : &str, 
    output_vec : &mut FpVector, 
    string_to_basis_element : F
) 
where
    F: for<'a> Fn(&'a str) -> Option<usize>
{
    'outer : loop {
        let result = query(prompt, |res : String| Ok(res));
        if result == "0" {
            output_vec.set_to_zero();
            break;
        }
        for term in result.split("+") {
            let term = term.trim();
            let parts : Vec<&str> = term.splitn(2,  " ").collect();
            if parts.len() == 1 {
                match string_to_basis_element(&parts[0]) {
                    Some(i) => output_vec.add_basis_element(i, 1),
                    None => { println!("Invalid value. Try again"); continue 'outer }
                };
            } else {
                let mut rest = &parts[1];
                let coef = match parts[0].parse::<u32>() {
                    Ok(c) => c,
                    _ => { rest = &term; 1 }
                };
                let gen_idx = match string_to_basis_element(rest) {
                    Some(i) => i,
                    None => { println!("Invalid value. Try again"); continue 'outer }
                };
                output_vec.add_basis_element(gen_idx, coef);
            }
        }
        return;        
    }
}

pub fn interactive_module_define() -> Result<String, Box<dyn Error>>{
    let output_path = query("Output file name", |result : String|
        if result.is_empty() {
            Err("Output file name cannot be empty".to_string())
        } else {
            Ok(result)
        }
    );

    let module_type = query_with_default_no_default_indicated(
        "Input module type (default 'finite dimensional module'):\n (0) - finite dimensional module \n (1) - finitely presented module\n", 
        0,
        |x : u32| match x {
            0 | 1 => Ok(x),
            _ => Err(format!("Invalid type '{}'. Type must be '0' or '1'", x))
        }
    );

    let name = query("Module name (use latex between $'s)", |name : String| Ok(name));
    // Query for prime
    let p = query_with_default("p", 2, 
        |p : u32| if crate::combinatorics::is_valid_prime(p) {Ok(p)} else {Err("invalid prime".to_string())});
    let generic = p != 2;
    let mut output_path_buf = PathBuf::from(output_path);
    output_path_buf.set_extension("json");
    let file_name = output_path_buf.file_stem().unwrap();    
    let mut output_json = json!({
        "file_name" : file_name.to_str(),
        "name" : name,
        "p" : p,
        "generic" : generic,
    });

    println!("module_type : {}", module_type);
    match module_type {
        0 => {output_json = interactive_module_define_fdmodule(output_json, p, generic)?},
        1 => {output_json = interactive_module_define_fpmodule(output_json, p, generic)?},
        _ => unreachable!()
    }
    std::fs::write(&output_path_buf, output_json.to_string())?;
    println!("Wrote module to file {:?}. Run again with {:?} as argument to resolve.", file_name, file_name);
    Ok("".to_string())
}


pub fn interactive_module_define_fdmodule(mut output_json : Value, p : u32, generic : bool) -> Result<Value, Box<dyn Error>>{
    output_json["type"] = Value::from("finite dimensional module");
    let adem_algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, generic, false)));
    let milnor_algebra = Rc::new(AlgebraAny::from(MilnorAlgebra::new(p)));
    let min_degree = 0i32;
    let gens = get_gens(min_degree)?;
    let gens_json = gens_to_json(&gens);    
    let max_degree = (gens.len() + 1) as i32 + min_degree;
    
    adem_algebra.compute_basis(max_degree);
    milnor_algebra.compute_basis(max_degree);
    
    let mut graded_dim = BiVec::with_capacity(min_degree, max_degree);
    for i in gens.iter().map(Vec::len) {
        graded_dim.push(i);
    }

    let mut adem_module = FDModule::new(Rc::clone(&adem_algebra), "".to_string(), graded_dim.clone());
    let mut milnor_module = FDModule::new(Rc::clone(&milnor_algebra), "".to_string(), graded_dim);

    for (i, deg_i_gens) in gens.iter_enum() {
        for (j, gen) in deg_i_gens.iter().enumerate() {
            adem_module.set_basis_element_name(i as i32, j, gen.to_string());
            milnor_module.set_basis_element_name(i as i32, j, gen.to_string());
        }
    }

    println!("Input actions. Write the value of the action in the form 'a x0 + b x1 + ...' where a, b are non-negative integers and x0, x1 are names of the generators. The coefficient can be omitted if it is 1");

    let len = gens.len();
    for input_deg in (0 .. len as i32).rev() {
        for output_deg in (input_deg + 1) .. len as i32 {
            let op_deg = output_deg - input_deg;
            let input_deg_idx = input_deg;
            let output_deg_idx = output_deg;
            if gens[output_deg_idx].len() == 0 {
                continue;
            }
            let adem_gens = adem_algebra.generators(op_deg);
            if adem_gens.len() > 0 {
                let mut output_vec = FpVector::new(p, gens[output_deg_idx].len());
                let adem_op_idx = adem_gens[0];
                let milnor_op_idx = milnor_algebra.generators(op_deg)[0];                
                let callback = |string : &str| gens[output_deg_idx].iter().position(|d| d == string);
                for input_idx in 0 .. gens[input_deg_idx].len() {                        
                    get_expression_to_vector(
                        &format!("{} {}", adem_algebra.basis_element_to_string(op_deg, adem_op_idx), gens[input_deg_idx][input_idx]),
                        &mut output_vec,
                        callback
                    );
                    adem_module.set_action_vector(op_deg, adem_op_idx, input_deg, input_idx, &output_vec);
                    milnor_module.set_action_vector(op_deg, milnor_op_idx, input_deg, input_idx, &output_vec);
                }
            }
            adem_module.extend_actions(input_deg, output_deg);
            milnor_module.extend_actions(input_deg, output_deg);
            adem_module.check_validity(input_deg, output_deg)?;
        }
    }

    let adem_actions = adem_module.actions_to_json();
    let milnor_actions = milnor_module.actions_to_json();
    
    output_json["gens"] = gens_json;
    output_json["adem_actions"] = adem_actions;
    output_json["milnor_actions"] = milnor_actions;
    Ok(output_json)
}

fn get_relation(adem_algebra : &AdemAlgebra, milnor_algebra : &MilnorAlgebra, module : &FreeModule, basis_elt_lookup : &HashMap<String, (i32, usize)>) -> Result<(i32, FpVector), String> {
    let relation = query("Relation", |x : String| Ok(x));
    if relation == "" {
        return Err("".to_string());
    }
    return evaluate_module(adem_algebra, milnor_algebra, module, basis_elt_lookup, &relation).map_err(|err| err.to_string());
}

pub fn interactive_module_define_fpmodule(mut output_json : Value, p : u32, generic : bool) -> Result<Value, Box<dyn Error>>{
    output_json["type"] = Value::from("finitely presented module");
    let min_degree = 0i32;
    let gens = get_gens(min_degree)?;
    let gens_json = gens_to_json(&gens);    
    let max_degree = 20;

    let adem_algebra_rc = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, generic, false)));
    let adem_algebra = AdemAlgebra::new(p, generic, false);
    let milnor_algebra = MilnorAlgebra::new(p);
    adem_algebra_rc.compute_basis(max_degree);
    adem_algebra.compute_basis(max_degree);
    milnor_algebra.compute_basis(max_degree);
    
    let mut graded_dim = BiVec::with_capacity(min_degree, max_degree);
    for i in gens.iter().map(Vec::len) {
        graded_dim.push(i);
    }

    let adem_module = FPModule::new(Rc::clone(&adem_algebra_rc), "".to_string(), min_degree);

    
    for (i, deg_i_gens) in gens.iter_enum() {
        adem_module.add_generators(i, deg_i_gens.clone());
    }
    // TODO: make relation parser automatically extend module by zero if necessary...
    adem_module.generators.extend_by_zero(20);

    println!("Input relations");
    match p {
        2 => println!("Write relations in the form 'Sq6 * Sq2 * x + Sq7 * y'"),
        _ => println!("Write relations in the form 'Q5 * P(5) * x + 2 * P(1, 3) * Q2 * y', where P(...) and Qi are Milnor basis elements."),
    }
    println!("There is currently a hard-coded maximum degree of {} for relations (this is the maximum allowed degree of an operator acting on the generators). One can raise this number by editing the max_degree variable in the interactive_module_define_fpmodule function of src/cli_module_loaders.rs. Apologies.", max_degree);

    let mut basis_elt_lookup = HashMap::new();
    for (i, deg_i_gens) in gens.iter_enum() {
        for (j, gen) in deg_i_gens.iter().enumerate() {
            let k = adem_module.generators.operation_generator_to_index(0, 0, i, j);
            basis_elt_lookup.insert(gen.clone(), (i, k));
        }
    }

    let mut relations : BiVec<Vec<FpVector>> = BiVec::new(min_degree);
    loop {
        match get_relation(&adem_algebra, &milnor_algebra, &adem_module.generators, &basis_elt_lookup) {
            Err(x) => {
                if x != "" {
                    println!("Invalid relation: {}. Try again.", x);
                    continue;
                }
                println!("This is the list of relations:");
                for (i, deg_i_relns) in relations.iter_enum() {
                    for r in deg_i_relns {
                        print!("{}, ", adem_module.generators.element_to_string(i, r));
                    }
                }
                print!("\n");
                if query_yes_no("Is it okay?") {
                    break;
                } else {
                    if query_yes_no("Start over?") {
                        relations = BiVec::new(min_degree);
                    }
                    continue;
                }
            },
            Ok((degree, vector)) => {   
                while relations.len() <= degree {
                    relations.push(Vec::new());
                }
                relations[degree].push(vector);
            },
        }
    }

    for (i, relns) in relations.iter_enum() {
        let dim = adem_module.generators.dimension(i);
        let mut matrix = crate::matrix::Matrix::new(p, relns.len(), dim);
        for (j, r) in relns.iter().enumerate() {
            matrix[j].assign(r);
        }
        adem_module.add_relations(i, &mut matrix);
    }
    output_json["gens"] = gens_json;
    output_json["adem_relations"] = adem_module.relations_to_json();

    Ok(output_json)
}
