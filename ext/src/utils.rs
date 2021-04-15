use crate::chain_complex::{FiniteChainComplex, FreeChainComplex};
use crate::resolution::Resolution;
use crate::CCC;
use algebra::module::FiniteModule;
use algebra::SteenrodAlgebra;
use saveload::Load;
use serde_json::{json, Value};

#[cfg(feature = "yoneda")]
use crate::chain_complex::ChainComplex;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Config {
    pub module_paths: Vec<PathBuf>,
    pub module_file_name: String,
    pub algebra_name: String,
}

pub fn get_config() -> Config {
    let mut args = pico_args::Arguments::from_env();
    if args.contains("--help") {
        println!(
            "{} [--algebra algebra_name] [module_name] [max_degree]",
            std::env::current_exe()
                .unwrap()
                .file_stem()
                .unwrap()
                .to_string_lossy()
        );
        std::process::exit(1);
    }

    let mut static_modules_path = std::env::current_exe().unwrap();
    static_modules_path.pop();
    static_modules_path.pop();
    static_modules_path.pop();
    static_modules_path.pop();
    static_modules_path.push("steenrod_modules");
    let current_dir = std::env::current_dir().unwrap();
    let mut relative_dir = std::env::current_dir().unwrap();
    relative_dir.push("steenrod_modules");

    Config {
        module_paths: vec![current_dir, relative_dir, static_modules_path],
        algebra_name: args
            .opt_value_from_str("--algebra")
            .unwrap()
            .unwrap_or_else(|| "adem".into()),
        module_file_name: args
            .opt_free_from_str()
            .unwrap()
            .unwrap_or_else(|| "S_2".into()),
    }
}

pub fn construct(config: &Config) -> error::Result<Resolution<CCC>> {
    let mut json = load_module_from_file(config)?;
    construct_from_json(&mut json, &config.algebra_name)
}

pub fn construct_from_json(json: &mut Value, algebra_name: &str) -> error::Result<Resolution<CCC>> {
    let algebra = Arc::new(SteenrodAlgebra::from_json(json, algebra_name)?);
    let module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), json)?);
    #[allow(unused_mut)] // This is only mut with Yoneda enabled
    let mut chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));
    #[allow(unused_mut)] // This is only mut with Yoneda enabled
    let mut resolution = Resolution::new(Arc::clone(&chain_complex));

    let cofiber = &json["cofiber"];
    #[cfg(feature = "yoneda")]
    if !cofiber.is_null() {
        use crate::chain_complex::ChainMap;
        use crate::yoneda::yoneda_representative;
        use algebra::module::homomorphism::FreeModuleHomomorphism;
        use algebra::module::{BoundedModule, Module};

        let s = cofiber["s"].as_u64().unwrap() as u32;
        let t = cofiber["t"].as_i64().unwrap() as i32;
        let idx = cofiber["idx"].as_u64().unwrap() as usize;

        resolution.resolve_through_bidegree(s, t + module.max_degree());

        let map = FreeModuleHomomorphism::new(resolution.module(s), Arc::clone(&module), t);
        let mut new_output = fp::matrix::Matrix::new(
            module.prime(),
            resolution.module(s).number_of_gens_in_degree(t),
            1,
        );
        new_output[idx].set_entry(0, 1);

        map.add_generators_from_matrix_rows(t, new_output.as_slice_mut());
        map.extend_by_zero(module.max_degree() + t);

        let cm = ChainMap {
            s_shift: s,
            chain_maps: vec![map],
        };
        let yoneda = yoneda_representative(Arc::new(resolution), cm);
        let mut yoneda = FiniteChainComplex::from(yoneda);
        yoneda.pop();

        chain_complex = Arc::new(yoneda);
        resolution = Resolution::new(Arc::clone(&chain_complex));
    }

    #[cfg(not(feature = "yoneda"))]
    if !cofiber.is_null() {
        panic!("cofiber not supported. Compile with yoneda feature enabled");
    }
    Ok(resolution)
}

pub fn load_module_from_file(config: &Config) -> error::Result<Value> {
    for path in &config.module_paths {
        let mut path = path.clone();
        path.push(&config.module_file_name);
        path.set_extension("json");
        if let Ok(s) = std::fs::read_to_string(path) {
            return Ok(serde_json::from_str(&s)?);
        }
    }

    error::from_string(format!(
        "Module file '{}' not found on path",
        config.module_file_name
    ))
}

/// A function that constructs the resolution of S_2 over the `algebra`. If `save_file` points to
/// an existent file, then it loads the resolution from the save file. It is not an error to supply
/// a non-existent file, but it is an error to supply an invalid save file.
///
/// This is used for various examples and tests as a shorthand.
pub fn construct_s_2<T: AsRef<Path>>(algebra: &str, save_file: Option<T>) -> Resolution<CCC> {
    let mut json = json!({
        "type" : "finite dimensional module",
        "p": 2,
        "gens": {"x0": 0},
        "actions": []
    });
    let mut resolution = construct_from_json(&mut json, algebra).unwrap();
    if let Some(path) = save_file {
        let path: &Path = path.as_ref();
        if path.exists() {
            let f = std::fs::File::open(path).unwrap();
            let mut f = std::io::BufReader::new(f);
            resolution = Resolution::load(&mut f, &resolution.complex()).unwrap();
        }
    }
    resolution
}

#[derive(Debug)]
struct ModuleFileNotFoundError {
    name: String,
}

impl std::fmt::Display for ModuleFileNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module file '{}' not found on path", &self.name)
    }
}

impl Error for ModuleFileNotFoundError {
    fn description(&self) -> &str {
        "Module file not found"
    }
}

const RED_ANSI_CODE: &str = "\x1b[31;1m";
const WHITE_ANSI_CODE: &str = "\x1b[0m";

pub fn ascii_num(n: usize) -> char {
    match n {
        0 => ' ',
        1 => '·',
        2 => ':',
        3 => '∴',
        4 => '⁘',
        5 => '⁙',
        6 => '⠿',
        7 => '⡿',
        8 => '⣿',
        9 => '9',
        _ => '*',
    }
}

pub fn print_resolution_color<C: FreeChainComplex, S: std::hash::BuildHasher>(
    res: &C,
    max_s: u32,
    highlight: &std::collections::HashMap<(u32, i32), u32, S>,
) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    for s in (0..=max_s).rev() {
        for t in s as i32..=res.module(s).max_computed_degree() {
            if matches!(highlight.get(&(s, t)), None | Some(0)) {
                write!(
                    stdout,
                    "{}{}{} ",
                    RED_ANSI_CODE,
                    ascii_num(res.module(s).number_of_gens_in_degree(t)),
                    WHITE_ANSI_CODE
                )
                .unwrap();
            } else {
                write!(
                    stdout,
                    "{} ",
                    ascii_num(res.module(s).number_of_gens_in_degree(t))
                )
                .unwrap();
            }
        }
        writeln!(stdout).unwrap();
    }
}

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

pub trait HashMapTuple<A, B, C> {
    fn get_tuple(&self, a: &A, b: &B) -> Option<&C>;
}

impl<A: Eq + Hash, B: Eq + Hash, C, S: BuildHasher> HashMapTuple<A, B, C>
    for HashMap<(A, B), C, S>
{
    fn get_tuple(&self, a: &A, b: &B) -> Option<&C> {
        let mut hasher = self.hasher().build_hasher();
        a.hash(&mut hasher);
        b.hash(&mut hasher);
        let raw_entry = self.raw_entry();

        raw_entry
            .from_hash(hasher.finish(), |v| &v.0 == a && &v.1 == b)
            .map(|(_, y)| y)
    }
}

/// Iterate through all pairs (s, f, t) such that f = t - s, s <= max_s and t <= max_t
pub fn iter_stems(max_s: u32, max_t: i32) -> impl Iterator<Item = (u32, i32, i32)> {
    (0..=max_t)
        .map(move |f| {
            (0..=std::cmp::min(max_s, (max_t - f) as u32)).map(move |s| (s, f, f + s as i32))
        })
        .flatten()
}

/// Iterate through all pairs (s, f, t) such that f = t - s, s <= max_s and f <= max_f
pub fn iter_stems_f(max_s: u32, max_f: i32) -> impl Iterator<Item = (u32, i32, i32)> {
    (0..=max_f)
        .map(move |f| (0..=max_s).map(move |s| (s, f, f + s as i32)))
        .flatten()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hashmap_tuple() {
        let mut x: HashMap<(u32, u32), bool> = HashMap::new();
        x.insert((5, 3), true);

        assert_eq!(x.get_tuple(&5, &3), Some(&true));
        assert_eq!(x.get_tuple(&3, &5), None);
        assert_eq!(x.get_tuple(&7, &12), None);
    }
}
