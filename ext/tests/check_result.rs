use expect_test::{expect_file, ExpectFile};
use ext::chain_complex::FreeChainComplex;
use ext::utils::construct;
use ext::utils::Config;
#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

#[test]
fn check_result() {
    compare("C2v14", expect_file!["benchmarks/C2v14"], 30);
    compare("C3", expect_file!["benchmarks/C3"], 30);
    compare("tmf2", expect_file!["benchmarks/tmf2"], 30);
    compare("A-mod-Sq1-Sq2-Sq4", expect_file!["benchmarks/tmf2"], 30);
    compare("RP_-4_inf", expect_file!["benchmarks/RP_-4_inf"], 30);
}

fn compare(module_name: &str, result: ExpectFile, max_degree: i32) {
    println!("module: {}", module_name);
    let path = std::path::PathBuf::from("steenrod_modules");
    let a = Config {
        module_paths: vec![path],
        module_file_name: module_name.to_string(),
        max_degree,
        algebra_name: String::from("adem"),
    };

    let a = construct(&a).unwrap();

    #[cfg(not(feature = "concurrent"))]
    {
        a.resolve_through_bidegree(max_degree as u32, max_degree);
    }

    #[cfg(feature = "concurrent")]
    {
        let bucket = std::sync::Arc::new(TokenBucket::new(2));
        a.resolve_through_bidegree_concurrent(max_degree as u32, max_degree, &bucket);
    }

    result.assert_eq(&a.graded_dimension_string(max_degree as u32, max_degree));
}
