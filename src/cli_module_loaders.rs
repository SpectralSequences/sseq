use std::error::Error;
use std::fmt::Display;
use std::io::{stdin, stdout, Write};
use std::rc::Rc;
use std::path::PathBuf;
use std::str::FromStr;
// use serde_json::value::Value;
use serde_json::json;

use crate::fp_vector::{FpVector,FpVectorT};
use crate::algebra::{Algebra, AlgebraAny};
use crate::milnor_algebra::MilnorAlgebra;
use crate::adem_algebra::AdemAlgebra;
// use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;

fn query<T : FromStr>(prompt : &str) -> T {
    loop {
        print!("{} : ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        if let Ok(res) = input.trim().parse::<T>() {
            return res;
        }
        println!("Invalid input. Try again");
    }
}

fn query_with_default<T : FromStr + Display>(prompt : &str, default : T) -> T {
    loop {
        print!("{} (default {}): ", prompt, default);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        if trimmed.len() == 0 {
            return default;
        }
        if let Ok(res) = trimmed.parse::<T>() {
            return res;
        }
        println!("Invalid input. Try again");
    }
}

fn query_with_default_no_default_indicated<T : FromStr + Display>(prompt : &str, default : T) -> T {
    loop {
        print!("{}: ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin().read_line(&mut input).expect(&format!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        if trimmed.len() == 0 {
            return default;
        }
        if let Ok(res) = trimmed.parse::<T>() {
            return res;
        }
        println!("Invalid input. Try again");
    }
}

pub fn get_gens() -> Result<Vec<Vec<String>>, Box<dyn Error>>{
    // Query for generators
    println!("Input generators. Press return to finish.");
    stdout().flush()?;

    let mut gens = Vec::new();
    let finished_degree = usize::max_value();
    loop {
        let gen_deg = query_with_default_no_default_indicated::<usize>("Generator degree", finished_degree);
        if gen_deg == finished_degree {
            println!("This is the list of generators and degrees:");
            for i in 0..gens.len() {
                for gen in &gens[i] {
                    print!("({}, {}) ", i, gen)
                }
            }
            print!("\n");
            if query::<String>("Is it okay? (yes/no)").starts_with("y") {
                break;
            } else {
                if query::<String>("Reset generator list? (yes/no)").starts_with("y") {
                    gens = Vec::new();
                }
                continue;
            }
        }
        while gens.len() <= gen_deg {
            gens.push(Vec::new());
        }
        let gen_name = query_with_default("Generator name", format!("x{}{}",gen_deg, gens[gen_deg].len()));        
        gens[gen_deg].push(gen_name);
    }
    Ok(gens)
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
        let result = query::<String>(prompt);
        if result == "0" {
            break;
        }
        for term in result.split("+") {
            let term = term.trim();
            let parts : Vec<&str> = term.split(" ").collect();
            if parts.len() == 1 {
                match string_to_basis_element(&parts[0]) {
                    Some(i) => output_vec.add_basis_element(i, 1),
                    None => { println!("Invalid value. Try again"); continue 'outer }
                };
            } else if parts.len() == 2 {
                let gen_idx = match string_to_basis_element(&parts[1]) {
                    Some(i) => i,
                    None => { println!("Invalid value. Try again"); continue 'outer }
                };
                let coef = match parts[0].parse::<u32>() {
                    Ok(c) => c,
                    _ => { println!("Invalid value. Try again"); continue 'outer }
                };
                output_vec.add_basis_element(gen_idx, coef);
            } else {
                println!("Invalid value. Try again"); continue 'outer;
            }
        }
        return;        
    }
}

pub fn interactive_module_define() -> Result<String, Box<dyn Error>>{
    let output_path = query::<String>("Output file name");
    let name = query::<String>("Module name (use latex between $'s)");
    // Query for prime and max_degree
    let mut p;
    loop {
        p = query_with_default("p", 2);
        if crate::combinatorics::is_valid_prime(p) {
            break;
        }
        println!("Invalid input. Try again");
    }
    let generic = p != 2;

    let adem_algebra = Rc::new(AlgebraAny::from(AdemAlgebra::new(p, generic, false)));
    let milnor_algebra = Rc::new(AlgebraAny::from(MilnorAlgebra::new(p)));
    let gens = get_gens()?;
    let mut gens_json = json!({});
    for i in 0..gens.len() {
        for gen in &gens[i] {
            gens_json[gen] = json!(i);
        }
    }
    
    adem_algebra.compute_basis(gens.len() as i32 + 1);
    milnor_algebra.compute_basis(gens.len() as i32 + 1);

    let graded_dim : Vec<usize> = gens.iter().map(Vec::len).collect();

    let mut adem_module = FDModule::new(Rc::clone(&adem_algebra), "".to_string(), 0, graded_dim.clone());
    let mut milnor_module = FDModule::new(Rc::clone(&milnor_algebra), "".to_string(), 0, graded_dim);

    for i in 0..gens.len() {
        for (j, gen) in gens[i].iter().enumerate() {
            adem_module.set_basis_element_name(i as i32, j, gen.to_string());
            milnor_module.set_basis_element_name(i as i32, j, gen.to_string());
        }
    }

    println!("Input actions. Write the value of the action in the form 'a x0 + b x1 + ...' where a, b are non-negative integers and x0, x1 are names of the generators. The coefficient can be omitted if it is 1");

    let len = gens.len();
    for input_deg in (0 .. len as i32).rev() {
        for output_deg in (input_deg + 1) .. len  as i32 {
            let op_deg = output_deg - input_deg;
            let input_deg_idx = input_deg as usize;
            let output_deg_idx = output_deg as usize;
            if gens[output_deg_idx].len() == 0 {
                continue;
            }
            let adem_gens = adem_algebra.get_generators(op_deg);
            if adem_gens.len() > 0 {
                let mut output_vec = FpVector::new(p, gens[output_deg_idx].len(), 0);
                let adem_op_idx = adem_gens[0];
                let milnor_op_idx = milnor_algebra.get_generators(op_deg)[0];                
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
    
    let mut output_path_buf = PathBuf::from(output_path);
    output_path_buf.set_extension("json");
    let file_name = output_path_buf.file_stem().unwrap();
    let output_json = json!({
        "type" : "finite dimensional module",
        "file_name" : file_name.to_str(),
        "name" : name,
        "p" : p,
        "generic" : generic,
        "gens" : gens_json,
        "adem_actions" : adem_actions,
        "milnor_actions" : milnor_actions
    });
    std::fs::write(&output_path_buf, output_json.to_string())?;
    println!("Wrote module to file {:?}. Run again with {:?} as argument to resolve.", file_name, file_name);
    Ok("".to_string())
}
