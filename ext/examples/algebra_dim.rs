use algebra::{Algebra, MilnorAlgebra};

fn main() -> anyhow::Result<()> {
    ext::utils::init_logging();

    let algebra = MilnorAlgebra::new(fp::prime::TWO, false);
    algebra.compute_basis(125);
    for n in 0..=125 {
        println!("dim A_{n} = {}", algebra.dimension(n));
    }
    Ok(())
}
