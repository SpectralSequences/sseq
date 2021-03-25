use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use saveload::Save;
use serde_json::json;

fn main() -> error::Result<()> {
    // Define a module via a json file.
    let json = json!({
        "type" : "finite dimensional module",
        "p": 2,
        "generic": false,
        "gens": {"x0": 0},
        "actions": []
    });

    // Construct the bundle object from the json. The bundle consists of data used to build the
    // resolution. Most of the we only need the resolution property, which is wrapped in an
    // Arc<RwLock>.
    let resolution = ext::utils::construct_from_json(json, "milnor")?;

    // Now resolve through the desired bidegree
    resolution.resolve_through_bidegree(6, 70);

    // Pretty print the resolution to stdout
    println!("{}", resolution.graded_dimension_string());

    // Finally, save the resolution to resolution.save if it doesn't already exist.
    if !Path::new("resolution.save").exists() {
        let file = File::create("resolution.save")?;
        let mut file = BufWriter::new(file);
        resolution.save(&mut file)?;
    }

    Ok(())
}
