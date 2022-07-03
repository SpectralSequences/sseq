//! A module containing various utility functions related to user interaction in some way.
use crate::chain_complex::{
    AugmentedChainComplex, BoundedChainComplex, ChainComplex, FiniteChainComplex,
};

use crate::resolution::{Resolution, UnstableResolution};
use crate::CCC;
use algebra::module::{steenrod_module, FDModule, Module, SteenrodModule};
use algebra::{AlgebraType, MilnorAlgebra, SteenrodAlgebra};

use anyhow::{anyhow, Context};
use serde_json::Value;

use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;
use std::sync::Arc;

// We build docs with --all-features so the docs are at the feature = "nassau" version
#[cfg(not(feature = "nassau"))]
pub type QueryModuleResolution = Resolution<CCC>;

/// The type returned by [`query_module`]. The value of this type depends on whether
/// [`nassau`](crate::nassau) is enabled. In any case, it is an augmented free chain complex over
/// either [`SteenrodAlgebra`] or [`MilnorAlgebra`](algebra::MilnorAlgebra) and supports the
/// `compute_through_stem` function.
#[cfg(feature = "nassau")]
pub type QueryModuleResolution = crate::nassau::Resolution<FDModule<MilnorAlgebra>>;

const STATIC_MODULES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ext/steenrod_modules");

/// A config object is an object that specifies how a Steenrod module should be constructed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// The json specification of the module
    module: Value,
    /// The basis for the Steenrod algebra
    algebra: AlgebraType,
}

/// Given a module specification string, load a json description of the module as described
/// [here](../index.html#module-specification).
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
        if let Some(spec_shift) = module.get_mut("shift") {
            *spec_shift = Value::from(spec_shift.as_i64().unwrap() + shift);
        } else {
            module["shift"] = Value::from(shift);
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
            None => AlgebraType::Milnor,
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
///
/// This dispatches to either [`construct_nassau`] or [`construct_standard`] depending on whether
/// the `nassau` feature is enabled.
pub fn construct<T, E>(
    module_spec: T,
    save_dir: Option<PathBuf>,
) -> anyhow::Result<QueryModuleResolution>
where
    anyhow::Error: From<E>,
    T: TryInto<Config, Error = E>,
{
    #[cfg(feature = "nassau")]
    {
        construct_nassau(module_spec, save_dir)
    }

    #[cfg(not(feature = "nassau"))]
    {
        construct_standard(module_spec, save_dir)
    }
}

/// See [`construct`]
pub fn construct_nassau<T, E>(
    module_spec: T,
    save_dir: Option<PathBuf>,
) -> anyhow::Result<crate::nassau::Resolution<FDModule<MilnorAlgebra>>>
where
    anyhow::Error: From<E>,
    T: TryInto<Config, Error = E>,
{
    let Config {
        module: json,
        algebra,
    } = module_spec.try_into()?;

    if algebra == AlgebraType::Adem {
        return Err(anyhow!("Nassau's algorithm requires Milnor's basis"));
    }
    if !json["profile"].is_null() {
        return Err(anyhow!(
            "Nassau's algorithm does not support non-trivial profile"
        ));
    }
    if json["p"].as_i64() != Some(2) {
        return Err(anyhow!("Nassau's algorithm does not support odd primes"));
    }
    if json["type"].as_str() != Some("finite dimensional module") {
        return Err(anyhow!(
            "Nassau's algorithm only supports finite dimensional modules"
        ));
    }

    let algebra = Arc::new(MilnorAlgebra::new(fp::prime::TWO, false));
    let module = Arc::new(FDModule::from_json(Arc::clone(&algebra), &json)?);

    if !json["confiber"].is_null() {
        return Err(anyhow!("Nassau's algorithm does not support cofiber"));
    }
    crate::nassau::Resolution::new_with_save(module, save_dir)
}

/// See [`construct`]
pub fn construct_standard<const U: bool, T, E>(
    module_spec: T,
    save_dir: Option<PathBuf>,
) -> anyhow::Result<crate::resolution::MuResolution<U, CCC>>
where
    anyhow::Error: From<E>,
    T: TryInto<Config, Error = E>,
    SteenrodAlgebra: algebra::MuAlgebra<U>,
{
    let Config {
        module: json,
        algebra,
    } = module_spec.try_into()?;

    let algebra = Arc::new(SteenrodAlgebra::from_json(&json, algebra, U)?);
    let module = Arc::new(steenrod_module::from_json(Arc::clone(&algebra), &json)?);
    let mut chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));

    let cofiber = &json["cofiber"];
    if !cofiber.is_null() {
        assert!(!U, "Cofiber not supported for unstable resolution");
        use crate::chain_complex::ChainMap;
        use crate::yoneda::yoneda_representative;
        use algebra::module::homomorphism::FreeModuleHomomorphism;

        let shift = json["shift"].as_i64().unwrap_or(0) as i32;

        let s = cofiber["s"].as_u64().unwrap() as u32;
        let t = cofiber["t"].as_i64().unwrap() as i32 + shift;
        let idx = cofiber["idx"].as_u64().unwrap() as usize;

        let max_degree = module
            .max_degree()
            .expect("Can only take cofiber when module is bounded");

        let resolution = Resolution::new(Arc::clone(&chain_complex));
        resolution.compute_through_stem(s, t + max_degree - s as i32);

        let map = FreeModuleHomomorphism::new(resolution.module(s), Arc::clone(&module), t);
        let mut new_output = fp::matrix::Matrix::new(
            module.prime(),
            resolution.module(s).number_of_gens_in_degree(t),
            1,
        );
        new_output[idx].set_entry(0, 1);

        map.add_generators_from_matrix_rows(t, new_output.as_slice_mut());
        map.extend_by_zero(max_degree + t);

        let cm = ChainMap {
            s_shift: s,
            chain_maps: vec![map],
        };
        let yoneda = yoneda_representative(Arc::new(resolution), cm);
        let mut yoneda = FiniteChainComplex::from(yoneda);
        yoneda.pop();

        chain_complex = Arc::new(yoneda.map(|m| Box::new(m.clone()) as SteenrodModule));
    }

    crate::resolution::MuResolution::new_with_save(chain_complex, save_dir)
}

/// Given the name of a module file (without the `.json` extension), find a json file with this
/// name, and return the parsed json object. The search path for this json file is described
/// [here](../index.html#module-specification).
pub fn load_module_json(name: &str) -> anyhow::Result<Value> {
    let current_dir = std::env::current_dir().context("Failed to read current directory")?;
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

/// Given an `n: usize`, return a UTF-8 character that best depicts this number. If `n < 9`, then
/// this is a UTF-8 when `n` many dots. If `n = 9`, then this is the number `9`. Otherwise, it is
/// `*`.
pub fn unicode_num(n: usize) -> char {
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

/// Query the user for a module and its save directory. See
/// [here](../index.html#module-specification) for details on the propmt format.
///
/// # Arguments
/// - `prompt`: The prompt used to query the user for the module. This is `"Module"` when invoked
///   through [`query_module`], but the user may want to use something more specific, e.g. `"Source
///   module"`.
/// - `algebra`: The Steenrod algebra basis allowed. Some applications only support using one of
///   the two basis, and specifying this parameter forbids the user from specifying the other
///   basis.
/// - `load_quasi_inverse`: Whether or not the quasi-inverses of the resolution should be stored.
///   Note that if there is a save directory, then quasi-inverses will never be stored in memory;
///   they must be accessed via `apply_quasi_inverse`.
///
/// # Returns
/// A [`QueryModuleResolution`]. Note that this type depends on whether the `nassau` feature is
/// enabled.
pub fn query_module_only(
    prompt: &str,
    algebra: Option<AlgebraType>,
    load_quasi_inverse: bool,
) -> anyhow::Result<QueryModuleResolution> {
    let (name, module): (String, Config) = query::with_default(prompt, "S_2", |s| {
        Result::<_, anyhow::Error>::Ok((
            s.to_owned(),
            match algebra {
                Some(algebra) => (s, algebra).try_into()?,
                None => s.try_into()?,
            },
        ))
    });

    let save_dir = query::optional(&format!("{prompt} save directory"), |x| {
        core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
    });

    let mut resolution =
        construct(module, save_dir).context("Failed to load module from save file")?;

    let load_quasi_inverse = load_quasi_inverse && resolution.save_dir().is_none();

    #[cfg(not(feature = "nassau"))]
    {
        resolution.load_quasi_inverse = load_quasi_inverse;
    }

    #[cfg(feature = "nassau")]
    assert!(
        !load_quasi_inverse,
        "Quasi inverse loading not support with Nassau. Please use a save directory instead"
    );

    resolution.set_name(name);

    Ok(resolution)
}

/// Query the user for a module and a bidegree, and return a resolution resolved up to said
/// bidegree. This is mainly a wrapper around [`query_module_only`] that also asks for the bidegree
/// to resolve up to as well. The prompt of [`query_module_only`] is always set to `"Module"` when
/// invoked through this function.
pub fn query_module(
    algebra: Option<AlgebraType>,
    load_quasi_inverse: bool,
) -> anyhow::Result<QueryModuleResolution> {
    let resolution = query_module_only("Module", algebra, load_quasi_inverse)?;

    let max_n: i32 = query::with_default("Max n", "30", str::parse);
    let mut max_s: u32 = query::with_default("Max s", "7", str::parse);

    if let Some(s) = secondary_job() {
        if s <= max_s {
            max_s = std::cmp::min(s + 1, max_s);
        } else {
            return Err(anyhow!("SECONDARY_JOB is larger than max_s"));
        }
    }

    resolution.compute_through_stem(max_s, max_n);

    Ok(resolution)
}

pub fn query_unstable_module_only() -> anyhow::Result<SteenrodModule> {
    let spec: Config = query::raw("Module", |x| x.try_into());
    let algebra = Arc::new(SteenrodAlgebra::from_json(
        &spec.module,
        spec.algebra,
        true,
    )?);
    steenrod_module::from_json(algebra, &spec.module)
}

pub fn query_unstable_module(load_quasi_inverse: bool) -> anyhow::Result<UnstableResolution<CCC>> {
    let module = Arc::new(query_unstable_module_only()?);
    let cc = Arc::new(FiniteChainComplex::ccdz(module));

    let save_dir = query::optional("Module save directory", |x| {
        core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
    });

    let mut resolution = UnstableResolution::new_with_save(cc, save_dir)?;
    resolution.load_quasi_inverse = load_quasi_inverse && resolution.save_dir().is_none();

    Ok(resolution)
}

/// Prints an element in the bidegree `(n, s)` to stdout. For example, `[0, 2, 1]` will be printed
/// as `2 x_(n, s, 1) + x_(n, s, 2)`.
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

/// Given a resolution, return a resolution of the unit, together with a boolean indicating whether
/// this is the original resolution was already a resolution of the unit. If the boolean is true,
/// then the original resolution is returned.
pub fn get_unit(
    resolution: Arc<QueryModuleResolution>,
) -> anyhow::Result<(bool, Arc<QueryModuleResolution>)> {
    let is_unit = resolution.target().max_s() == 1 && resolution.target().module(0).is_unit();

    let unit = if is_unit {
        Arc::clone(&resolution)
    } else {
        let save_dir = query::optional("Unit save directory", |x| {
            core::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
        });

        let algebra = resolution.algebra();
        let module = FDModule::new(
            algebra,
            String::from("unit"),
            bivec::BiVec::from_vec(0, vec![1]),
        );

        #[cfg(feature = "nassau")]
        {
            Arc::new(crate::nassau::Resolution::new_with_save(
                Arc::new(module),
                save_dir,
            )?)
        }

        #[cfg(not(feature = "nassau"))]
        {
            let cc = FiniteChainComplex::ccdz(Arc::new(Box::new(module) as SteenrodModule));
            Arc::new(Resolution::new_with_save(Arc::new(cc), save_dir)?)
        }
    };

    Ok((is_unit, unit))
}

/// Given a function `f(s, t)`, compute it for every `s` in `[min_s, max_s]` and every `t` in
/// `[min_t, max_t(s)]`.  Further, we only compute `f(s, t)` when `f(s - 1, t')` has been computed
/// for all `t' < t`.
///
/// The function `f` should return a range starting from t and ending at the largest `T` such that
/// `f(s, t')` has already been computed for every `t' < T`.
///
/// While `iter_s_t` could have had kept track of that data, it is usually the case that `f` would
/// compute something and write it to a `OnceBiVec`, and
/// [`OnceBiVec::push_ooo`](once::OnceBiVec::push_ooo) would return this range for us.
///
/// This uses [`rayon`] under the hood, and `f` should feel free to use further rayon parallelism.
///
/// # Arguments:
///  - `max_s`: This is exclusive
///  - `max_t`: This is exclusive
#[cfg(feature = "concurrent")]
pub fn iter_s_t(
    f: &(impl Fn(u32, i32) -> std::ops::Range<i32> + Sync),
    min_s: u32,
    min_t: i32,
    max_s: u32,
    max_t: &(impl Fn(u32) -> i32 + Sync),
) {
    use rayon::prelude::*;

    rayon::scope(|scope| {
        // Rust does not support recursive closures, so we have to pass everything along as
        // arguments.
        fn run<'a>(
            scope: &rayon::Scope<'a>,
            f: &'a (impl Fn(u32, i32) -> std::ops::Range<i32> + Sync + 'a),
            max_s: u32,
            max_t: &'a (impl Fn(u32) -> i32 + Sync + 'a),
            s: u32,
            t: i32,
        ) {
            let mut ret = f(s, t);
            if s + 1 < max_s {
                ret.start += 1;
                ret.end = std::cmp::min(ret.end + 1, max_t(s + 1));

                if !ret.is_empty() {
                    // We spawn a new scope to avoid recursion, which may blow the stack
                    scope.spawn(move |scope| {
                        ret.into_par_iter()
                            .for_each(|t| run(scope, f, max_s, max_t, s + 1, t));
                    });
                }
            }
        }

        rayon::join(
            || {
                (min_t..max_t(min_s))
                    .into_par_iter()
                    .for_each(|t| run(&scope, f, max_s, max_t, min_s, t))
            },
            || {
                (min_s + 1..max_s)
                    .into_par_iter()
                    .for_each(|s| run(&scope, f, max_s, max_t, s, min_t))
            },
        );
    });
}

#[cfg(feature = "logging")]
mod logging {
    use std::io::Write;
    use std::time::Instant;

    /// If the `logging` feature is enabled, this can be used to time how long an operation takes.
    /// If the `logging` features is disabled, this is a no-op.
    ///
    /// # Example
    /// ```
    /// # use logging::Timer;
    /// let timer = Timer::start();
    /// // slow_function();
    /// timer.end(format_args!("Ran slow_function"));
    /// ```
    pub struct Timer(Instant);

    impl Timer {
        pub fn start() -> Self {
            Self(Instant::now())
        }

        pub fn end(self, msg: std::fmt::Arguments) {
            let duration = self.0.elapsed();
            eprintln!(
                "[{:>6}.{:>06} s] {msg}",
                duration.as_secs(),
                duration.subsec_micros(),
            );
        }
    }

    pub struct LogWriter<T> {
        writer: T,
        bytes: u64,
        timer: Timer,
    }

    impl<T: Write> Write for LogWriter<T> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let written = self.writer.write(buf)?;
            self.bytes += written as u64;
            Ok(written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.writer.flush()
        }
    }

    impl<T> LogWriter<T> {
        pub fn new(writer: T) -> Self {
            LogWriter {
                writer,
                bytes: 0,
                timer: Timer::start(),
            }
        }
    }

    impl<T: Write> LogWriter<T> {
        pub fn finalize(mut self, msg: std::fmt::Arguments) {
            self.writer.flush().unwrap();
            let duration = self.timer.0.elapsed();
            let mib = self.bytes as f64 / (1024 * 1024) as f64;
            let mib_per_second = mib / duration.as_secs_f64();
            self.timer
                .end(format_args!("[{mib_per_second:>9.3} MiB/s] {msg}"));
        }
    }
}

#[cfg(not(feature = "logging"))]
mod logging {
    use std::io::Write;

    pub struct Timer;

    impl Timer {
        pub fn start() -> Self {
            Self {}
        }

        pub fn end(self, _msg: std::fmt::Arguments) {}
    }

    pub struct LogWriter<T>(T);

    impl<T: Write> Write for LogWriter<T> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }

    impl<T> LogWriter<T> {
        pub fn new(writer: T) -> Self {
            LogWriter(writer)
        }

        pub fn finalize(self, _msg: std::fmt::Arguments) {}
    }
}

pub use logging::LogWriter;
pub use logging::Timer;

/// The value of the SECONDARY_JOB environment variable. This is used for distributing the
/// `secondary`. If set, only data with `s = SECONDARY_JOB` will be computed. The minimum value of
/// `s` is the `shift_s` of the [`SecondaryLift`](crate::secondary::SecondaryLift) and the maximum
/// value (inclusive) is the maximum `s` of the resolution.
pub fn secondary_job() -> Option<u32> {
    let val = std::env::var("SECONDARY_JOB").ok()?;
    let parsed: Option<u32> = str::parse(&val).ok();
    if parsed.is_none() {
        eprintln!(
            "Invalid argument for `SECONDARY_JOB`. Expected non-negative integer but found {val}"
        );
    }
    parsed
}
