use algebra::module::{FDModule, TensorModule};
use algebra::AdemAlgebra;
use ext::utils::parse_module_name;
use fp::prime::ValidPrime;

use anyhow::anyhow;
use serde_json::json;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let left = query::with_default("Left module", "S_2", parse_module_name);
    let p = left["p"].as_u64().unwrap();

    let right = query::with_default("Right module", "S_2", |name| {
        let module = parse_module_name(name)?;
        if module["p"].as_u64() == Some(p) {
            Ok(module)
        } else {
            Err(anyhow!("Two modules must be over the same prime"))
        }
    });

    let p = ValidPrime::new(p as u32);
    let algebra = Arc::new(AdemAlgebra::new(p, *p != 2, false));
    let left_module = FDModule::from_json(Arc::clone(&algebra), &left)?;
    let right_module = FDModule::from_json(Arc::clone(&algebra), &right)?;

    let mut tensor = FDModule::from(&TensorModule::new(
        Arc::new(left_module),
        Arc::new(right_module),
    ));
    tensor.name = String::new();

    let mut output = json!({
        "p": *p,
    });
    tensor.to_json(&mut output);

    println!("{}", output);
    Ok(())
}
