use algebra::module::Module;
use ext::chain_complex::ChainComplex;
use ext::utils::query_module;

fn main() -> anyhow::Result<()> {
    let res = query_module(None, false)?;

    for s in (0..res.next_homological_degree()).rev() {
        let module = res.module(s);
        for t in res.min_degree() + s as i32..=module.max_computed_degree() {
            print!("{}, ", module.dimension(t));
        }
        println!();
    }
    Ok(())
}
