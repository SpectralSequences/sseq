use algebra::module::homomorphism::FreeModuleHomomorphism;
use algebra::module::{FiniteModule, FreeModule, Module};
use algebra::{Algebra, SteenrodAlgebra};
use ext::utils::Config;
use ext::{
    chain_complex::{ChainComplex, FiniteChainComplex},
    resolution::Resolution,
    resolution::SaveData,
    CCC,
};
use fp::matrix::Subspace;
use once::{OnceBiVec, OnceVec};

use saveload::Load;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use byteorder::{LittleEndian, WriteBytesExt};

fn main() -> anyhow::Result<()> {
    let Config {
        module: json,
        algebra,
    } = query::with_default("Module", "S_2", |s| s.try_into());

    #[allow(clippy::redundant_closure)]
    let save_file = query::raw("Old save file", |x| File::open(x));
    let mut buffer = BufReader::new(save_file);

    let save_dir = query::raw("New save directory", |x| {
        std::result::Result::<PathBuf, std::convert::Infallible>::Ok(PathBuf::from(x))
    });

    let _max_algebra_dim = i32::load(&mut buffer, &())?;

    let algebra = Arc::new(SteenrodAlgebra::from_json(&json, algebra)?);
    let module = Arc::new(FiniteModule::from_json(Arc::clone(&algebra), &json)?);
    let chain_complex = Arc::new(FiniteChainComplex::ccdz(Arc::clone(&module)));

    let p = algebra.prime();

    let resolution: Resolution<CCC> =
        Resolution::new_with_save(Arc::clone(&chain_complex), Some(save_dir))?;

    let min_degree = chain_complex.min_degree();
    let modules: OnceVec<Arc<FreeModule<SteenrodAlgebra>>> =
        Load::load(&mut buffer, &(Arc::clone(&algebra), min_degree))?;
    let kernels: OnceBiVec<Option<Subspace>> = Load::load(&mut buffer, &(min_degree, Some(p)))?;

    let max_s = modules.len();
    let len = usize::load(&mut buffer, &())?;

    assert_eq!(len, max_s);

    for (t, kernel) in kernels.iter_enum() {
        if let Some(kernel) = kernel.as_ref() {
            let mut f = resolution.create_save_file(SaveData::Kernel, max_s as u32 - 1, t);
            kernel.to_bytes(&mut f)?;
        }
    }

    let mut differentials = Vec::new();

    let mut d: FreeModuleHomomorphism<FreeModule<SteenrodAlgebra>> = Load::load(
        &mut buffer,
        &(Arc::clone(&modules[0usize]), resolution.zero_module(), 0),
    )?;
    for t in d.quasi_inverses.min_degree()..d.quasi_inverses.len() {
        let qi = d.quasi_inverses[t].take().unwrap();
        let mut f = resolution.create_save_file(SaveData::ResQi, 0, t);
        qi.to_bytes(&mut f)?;
    }
    differentials.push(d);

    for s in 1..max_s as u32 {
        let mut d: FreeModuleHomomorphism<FreeModule<SteenrodAlgebra>> = Load::load(
            &mut buffer,
            &(Arc::clone(&modules[s]), Arc::clone(&modules[s - 1]), 0),
        )?;
        for t in d.quasi_inverses.min_degree()..d.quasi_inverses.len() {
            let qi = d.quasi_inverses[t].take().unwrap();
            let mut f = resolution.create_save_file(SaveData::ResQi, s, t);
            qi.to_bytes(&mut f)?;
        }
        differentials.push(d);
    }

    let len = usize::load(&mut buffer, &())?;
    assert_eq!(len, max_s);

    for (s, d) in differentials.into_iter().enumerate() {
        let mut c: FreeModuleHomomorphism<FiniteModule> = Load::load(
            &mut buffer,
            &(Arc::clone(&modules[s]), chain_complex.module(s as u32), 0),
        )?;
        for t in c.quasi_inverses.min_degree()..c.quasi_inverses.len() {
            let qi = c.quasi_inverses[t].take().unwrap();
            let mut f = resolution.create_save_file(SaveData::AugmentationQi, s as u32, t);
            qi.to_bytes(&mut f)?;

            let num_new_gens = modules[s].number_of_gens_in_degree(t);
            let mut f = resolution.create_save_file(SaveData::Differential, s as u32, t);
            f.write_u64::<LittleEndian>(num_new_gens as u64)?;
            f.write_u64::<LittleEndian>(if s == 0 {
                0
            } else {
                modules[s - 1].dimension(t)
            } as u64)?;
            f.write_u64::<LittleEndian>(chain_complex.module(s as u32).dimension(t) as u64)?;

            for n in 0..num_new_gens {
                d.output(t, n).to_bytes(&mut f)?;
            }
            for n in 0..num_new_gens {
                c.output(t, n).to_bytes(&mut f)?;
            }
        }
    }
    Ok(())
}
