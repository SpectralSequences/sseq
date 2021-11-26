use crate::chain_complex::{ChainComplex, FiniteChainComplex, FreeChainComplex};
use crate::resolution::Resolution;
use crate::CCC;
use algebra::module::{FiniteModule, Module};
use algebra::{Algebra, AlgebraType, SteenrodAlgebra};
use fp::prime::ValidPrime;

use anyhow::{anyhow, Context};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde_json::Value;

use std::convert::{TryFrom, TryInto};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

const STATIC_MODULES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ext/steenrod_modules");

/// A config object is an object that specifies how a Steenrod module should be constructed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// The json specification of the module
    pub module: Value,
    /// The basis for the Steenrod algebra
    pub algebra: AlgebraType,
}

pub fn parse_module_name(module_name: &str) -> anyhow::Result<Value> {
    let mut args = module_name.split('[');
    let module_file = args.next().unwrap();
    let mut module = load_module_json(module_file)
        .with_context(|| format!("Failed to load module file {}", module_file))?;
    if let Some(shift) = args.next() {
        let shift: i64 = match shift.strip_suffix(']') {
            None => return Err(anyhow!("Unterminated shift [")),
            Some(x) => x
                .parse()
                .with_context(|| format!("Cannot parse shift value ({}) as an integer", x))?,
        };
        if let Some(gens) = module["gens"].as_object_mut() {
            for entry in gens.into_iter() {
                *entry.1 = (entry.1.as_i64().unwrap() + shift).into()
            }
        }
    }
    Ok(module)
}

impl TryFrom<&str> for Config {
    type Error = anyhow::Error;

    fn try_from(spec: &str) -> Result<Self, Self::Error> {
        let mut args = spec.split('@');
        let module_name = args.next().unwrap();
        let algebra = match args.next() {
            Some(x) => x
                .parse()
                .with_context(|| format!("Invalid algebra type: {}", x))?,
            None => AlgebraType::Adem,
        };

        Ok(Config {
            module: parse_module_name(module_name)
                .with_context(|| format!("Failed to load module: {}", module_name))?,
            algebra,
        })
    }
}

impl<T, E> TryFrom<(&str, T)> for Config
where
    anyhow::Error: From<E>,
    T: TryInto<AlgebraType, Error = E>,
{
    type Error = anyhow::Error;

    fn try_from(mut spec: (&str, T)) -> Result<Self, Self::Error> {
        let algebra = spec.1.try_into()?;
        if spec.0.contains('@') {
            if spec.0.ends_with(&*algebra.to_string()) {
                spec.0 = &spec.0[0..spec.0.len() - algebra.to_string().len() - 1];
            } else {
                return Err(anyhow!("Invalid algebra supplied. Must be {}", algebra));
            }
        }
        Ok(Config {
            module: parse_module_name(spec.0)?,
            algebra,
        })
    }
}

impl<T: TryInto<AlgebraType>> TryFrom<(Value, T)> for Config {
    type Error = T::Error;

    fn try_from(spec: (Value, T)) -> Result<Self, Self::Error> {
        Ok(Config {
            module: spec.0,
            algebra: spec.1.try_into()?,
        })
    }
}

/// This constructs a resolution resolving a module according to the specifications
///
/// # Arguments
///  - `module_spec`: A specification for the module. This is any object that implements
///     [`TryInto<Config>`] (with appropriate error bounds). In practice, we can supply
///     - A [`Config`] object itself
///     - `(json, algebra)`: The first argument is a [`serde_json::Value`] that specifies the
///       module; the second argument is either a string (`"milnor"` or `"adem"`) or an
///       [`algebra::AlgebraType`] object.
///     - `(module_name, algebra)`: The first argument is the name of the module and the second is
///       as above. Modules are searched in the current directory, `$CWD/steenrod_modules` and
///       `ext/steenrod_modules`. The modules can be shifted by appending e.g. `S_2[2]`.
///     - `module_spec`, a single `&str` of the form `module_name@algebra`, where `module_name` and
///       `algebra` are as above.
///  - `save_file`: The save file for the module. If it points to an invalid save file, an error is
///    returned.
pub fn construct<T, E>(module_spec: T, save_dir: Option<PathBuf>) -> anyhow::Result<Resolution<CCC>>
where
    anyhow::Error: From<E>,
    T: TryInto<Config, Error = E>,
{
    let Config {
        module: json,
        algebra,
    } = module_spec.try_into()?;

    let algebra = Arc::new(SteenrodAlgebra::from_json(&json, algebra)?);
    let module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), &json)?);
    let mut chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));

    let cofiber = &json["cofiber"];
    if !cofiber.is_null() {
        use crate::chain_complex::ChainMap;
        use crate::yoneda::yoneda_representative;
        use algebra::module::homomorphism::FreeModuleHomomorphism;
        use algebra::module::BoundedModule;

        let s = cofiber["s"].as_u64().unwrap() as u32;
        let t = cofiber["t"].as_i64().unwrap() as i32;
        let idx = cofiber["idx"].as_u64().unwrap() as usize;

        let resolution = Resolution::new(Arc::clone(&chain_complex));
        resolution.compute_through_bidegree(s, t + module.max_degree());

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
    }

    Resolution::new_with_save(chain_complex, save_dir)
}

pub fn load_module_json(name: &str) -> anyhow::Result<Value> {
    let current_dir = std::env::current_dir().unwrap();
    let relative_dir = current_dir.join("steenrod_modules");

    for path in &[
        current_dir,
        relative_dir,
        PathBuf::from(STATIC_MODULES_PATH),
    ] {
        let mut path = path.clone();
        path.push(name);
        path.set_extension("json");
        if let Ok(s) = std::fs::read_to_string(&path) {
            return serde_json::from_str(&s)
                .with_context(|| format!("Failed to load module json at {:?}", path));
        }
    }
    Err(anyhow!("Module file '{}' not found", name))
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
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    for s in (0..max_s).rev() {
        for t in s as i32..=res.module(s).max_computed_degree() {
            if matches!(highlight.get(&(s, t)), None | Some(0)) {
                write!(
                    stderr,
                    "{}{}{} ",
                    RED_ANSI_CODE,
                    ascii_num(res.module(s).number_of_gens_in_degree(t)),
                    WHITE_ANSI_CODE
                )
                .unwrap();
            } else {
                write!(
                    stderr,
                    "{} ",
                    ascii_num(res.module(s).number_of_gens_in_degree(t))
                )
                .unwrap();
            }
        }
        writeln!(stderr, "\x1b[K").unwrap();
    }
}

pub struct QueryModuleResult {
    pub resolution: Resolution<CCC>,
    #[cfg(feature = "concurrent")]
    pub bucket: thread_token::TokenBucket,
}

pub fn query_module_only(
    prompt: &str,
    algebra: Option<AlgebraType>,
) -> anyhow::Result<Resolution<CCC>> {
    let module: Config = query::with_default(prompt, "S_2", |s| match algebra {
        Some(algebra) => (s, algebra).try_into(),
        None => s.try_into(),
    });

    let save_dir = query::optional(&format!("{prompt} save directory"), |x| {
        core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
    });

    construct(module, save_dir).context("Failed to load module from save file")
}

pub fn query_module(algebra: Option<AlgebraType>) -> anyhow::Result<QueryModuleResult> {
    let resolution = query_module_only("Module", algebra)?;

    #[cfg(feature = "concurrent")]
    let bucket = query_bucket();

    let max_s: u32 = query::with_default("Max s", "7", str::parse);
    let max_n: i32 = query::with_default("Max n", "30", str::parse);

    #[cfg(not(feature = "concurrent"))]
    resolution.compute_through_stem(max_s, max_n);

    #[cfg(feature = "concurrent")]
    resolution.compute_through_stem_concurrent(max_s, max_n, &bucket);

    Ok(QueryModuleResult {
        resolution,
        #[cfg(feature = "concurrent")]
        bucket,
    })
}

#[cfg(feature = "concurrent")]
pub fn query_num_threads() -> core::num::NonZeroUsize {
    use std::env;

    match env::var("EXT_THREADS") {
        Ok(n) => match n.parse::<core::num::NonZeroUsize>() {
            Ok(n) => return n,
            Err(_) => eprintln!("Invalid value of EXT_THREADS variable: {n}"),
        },
        Err(env::VarError::NotUnicode(_)) => eprintln!("Invalid value of EXT_THREADS variable"),
        Err(env::VarError::NotPresent) => (),
    };

    query::with_default("Number of threads", "2", str::parse)
}

#[cfg(feature = "concurrent")]
pub fn query_bucket() -> thread_token::TokenBucket {
    thread_token::TokenBucket::new(query_num_threads())
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

/// Prints an element in the bidegree `(n, s)` to stdout. For example, `[0, 2, 1]` will be printed
/// as `2 x_(n, s, 1) + x_(f, s, 2)`.
pub fn print_element(v: fp::vector::Slice, n: i32, s: u32) {
    let mut first = true;
    for (i, v) in v.iter_nonzero() {
        if !first {
            print!(" + ");
        }
        if v != 1 {
            print!("{} ", v);
        }
        print!("x_({}, {}, {})", n, s, i);
        first = false;
    }
}

pub fn write_header<A: Algebra>(
    magic: u32,
    algebra: &A,
    p: ValidPrime,
    s: u32,
    t: i32,
    buffer: &mut impl Write,
) -> std::io::Result<()> {
    buffer.write_u32::<LittleEndian>(magic)?;
    buffer.write_u16::<LittleEndian>(algebra.magic())?;
    buffer.write_u16::<LittleEndian>(*p as u16)?;
    buffer.write_u32::<LittleEndian>(s)?;
    buffer.write_i32::<LittleEndian>(t)
}

pub fn validate_header<A: Algebra>(
    magic: u32,
    algebra: &A,
    p: ValidPrime,
    s: u32,
    t: i32,
    buffer: &mut impl Read,
) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};

    let data_magic = buffer.read_u32::<LittleEndian>()?;
    if data_magic != magic {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid magic {data_magic:#010x}; expected {magic:#010x}"),
        ));
    }

    let algebra_magic = buffer.read_u16::<LittleEndian>()?;
    if algebra_magic != algebra.magic() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid algebra magic {algebra_magic:#06x}; expected {:#06x}",
                algebra.magic()
            ),
        ));
    }

    let data_p = buffer.read_u16::<LittleEndian>()? as u32;
    if data_p != *p {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid prime {data_p}; expected {p}"),
        ));
    }

    let data_s = buffer.read_u32::<LittleEndian>()?;
    if data_s != s {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid s {data_s}; expected {s}"),
        ));
    }

    let data_t = buffer.read_i32::<LittleEndian>()?;
    if data_t != t {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid s {data_t}; expected {t}"),
        ));
    }

    Ok(())
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
