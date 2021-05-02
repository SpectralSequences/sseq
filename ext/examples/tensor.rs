use algebra::module::{BoundedModule, FDModule, TensorModule};
use algebra::AdemAlgebra;
use ext::utils::parse_module_name;
use fp::prime::ValidPrime;
use serde_json::json;
use std::sync::Arc;

fn main() -> error::Result {
    let left = query::with_default("Left module", "S_2", |name| parse_module_name(name));
    let p = left["p"].as_u64().unwrap();

    let right = query::with_default("Right module", "S_2", |name| {
        let module = parse_module_name(name)?;
        if module["p"].as_u64() == Some(p) {
            Ok(module)
        } else {
            Err(String::from("Two modules must be over the same prime"))
        }
    });

    let p = ValidPrime::new(p as u32);
    let algebra = Arc::new(AdemAlgebra::new(p, *p != 2, false, false));
    let left_module = FDModule::from_json(Arc::clone(&algebra), &left)?;
    let right_module = FDModule::from_json(Arc::clone(&algebra), &right)?;

    let tensor = TensorModule::new(Arc::new(left_module), Arc::new(right_module)).to_fd_module();

    let mut output = json!({
        "p": *p,
        "generic": *p != 2
    });
    tensor.to_json(&mut output);

    println!("{}", output);
    Ok(())
}
