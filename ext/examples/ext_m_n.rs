use algebra::{module::Module, Algebra};
use ext::chain_complex::ChainComplex;
use hom_cochain_complex::HomCochainComplex;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    eprintln!("This script computes Ext(M, N)");
    let res = ext::utils::query_module_only("Module M", None, false)?;
    let module_spec = query::raw("Module N", ext::utils::parse_module_name);
    #[cfg(not(feature = "nassau"))]
    let module = algebra::module::steenrod_module::from_json(res.algebra(), &module_spec)?;

    #[cfg(feature = "nassau")]
    let module = algebra::module::FDModule::from_json(res.algebra(), &module_spec)?;

    let max_n: i32 = query::raw("Max n", str::parse);
    let max_s: u32 = query::raw("Max s", str::parse);

    res.compute_through_stem(max_s + 1, max_n + module.max_degree().unwrap());
    res.algebra()
        .compute_basis(max_n + module.max_degree().unwrap() + max_s as i32 + 2);

    let hom_cc = HomCochainComplex::new(Arc::new(res), Arc::new(module));
    hom_cc.compute_through_stem(max_s, max_n);

    // FreeChainComplex::graded_dimension_string
    let mut result = String::new();
    for s in (0..=max_s).rev() {
        for n in hom_cc.min_degree()..=max_n {
            result.push(ext::utils::unicode_num(
                hom_cc.homology_dimension(s, n + s as i32),
            ));
            result.push(' ');
        }
        result.push('\n');
        // If it is empty so far, don't print anything
        if result.trim_start().is_empty() {
            result.clear()
        }
    }
    print!("{result}");

    Ok(())
}

mod hom_cochain_complex {
    use algebra::module::homomorphism::{HomPullback, ModuleHomomorphism};
    use algebra::module::{HomModule, Module};
    use ext::chain_complex::FreeChainComplex;
    use fp::matrix::Subquotient;
    use once::OnceVec;

    use std::sync::Arc;

    pub struct HomCochainComplex<CC: FreeChainComplex, M: Module<Algebra = CC::Algebra>> {
        source: Arc<CC>,
        target: Arc<M>,
        modules: OnceVec<Arc<HomModule<M>>>,
        differentials: OnceVec<Arc<HomPullback<M>>>,
    }

    impl<CC: FreeChainComplex, M: Module<Algebra = CC::Algebra>> HomCochainComplex<CC, M> {
        pub fn new(source: Arc<CC>, target: Arc<M>) -> Self {
            Self {
                source,
                target,
                modules: OnceVec::new(),
                differentials: OnceVec::new(),
            }
        }

        pub fn min_degree(&self) -> i32 {
            self.modules[0usize].min_degree()
        }

        pub fn compute_through_stem(&self, max_s: u32, max_n: i32) {
            self.modules.extend(max_s as usize + 1, |s| {
                Arc::new(HomModule::new(
                    self.source.module(s as u32),
                    Arc::clone(&self.target),
                ))
            });
            self.differentials.extend(max_s as usize, |s| {
                Arc::new(HomPullback::new(
                    Arc::clone(&self.modules[s]),
                    Arc::clone(&self.modules[s + 1]),
                    self.source.differential(s as u32 + 1),
                ))
            });
            for (s, module) in self.modules.iter().enumerate() {
                module.compute_basis(max_n + s as i32 + 1);
            }
            for (s, d) in self.differentials.iter().enumerate() {
                d.compute_auxiliary_data_through_degree(max_n + s as i32 + 1);
            }
        }

        pub fn homology_dimension(&self, s: u32, t: i32) -> usize {
            if s == 0 {
                self.differentials[s].kernel(t).unwrap().dimension()
            } else {
                Subquotient::from_parts(
                    self.differentials[s].kernel(t).cloned().unwrap(),
                    self.differentials[s - 1].image(t).cloned().unwrap(),
                )
                .dimension()
            }
        }
    }
}
