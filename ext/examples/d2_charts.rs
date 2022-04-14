use algebra::Algebra;
use chart::{Backend as _, TikzBackend as Backend};
use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    secondary::{SecondaryLift, SecondaryResolution},
};
use std::{fs::File, sync::Arc};

fn main() -> anyhow::Result<()> {
    let resolution = Arc::new(ext::utils::query_module(
        Some(algebra::AlgebraType::Milnor),
        true,
    )?);

    let lift = SecondaryResolution::new(Arc::clone(&resolution));
    lift.extend_all();

    let sseq = lift.e3_page();
    let products: Vec<_> = resolution
        .algebra()
        .default_filtration_one_products()
        .into_iter()
        .map(|(name, op_deg, op_idx)| (name, resolution.filtration_one_products(op_deg, op_idx)))
        .collect();

    let write = |path, page, diff, prod| {
        const EXT: &str = Backend::<File>::EXT;
        let backend = Backend::new(File::create(format!(
            "{}_{}.{}",
            path,
            resolution.name(),
            EXT
        ))?);
        sseq.write_to_graph(backend, page, diff, products.iter().take(prod), |_| Ok(()))?;
        <Result<(), std::io::Error>>::Ok(())
    };

    write("e2", 2, false, 3)?;
    write("e2_d2", 2, true, 3)?;
    write("e3", 3, false, 3)?;

    write("e2_clean", 2, false, 2)?;
    write("e2_d2_clean", 2, true, 2)?;
    write("e3_clean", 3, false, 2)?;

    Ok(())
}
