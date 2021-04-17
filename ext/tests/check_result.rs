use expect_test::{expect_file, ExpectFile};
use ext::chain_complex::{ChainComplex, FreeChainComplex};
use ext::utils::Config;
use ext::utils::{construct, construct_s_2};
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
    let config = Config {
        module_file_name: module_name.to_string(),
        algebra_name: String::from("adem"),
    };

    let a = construct(&config).unwrap();
    a.compute_through_bidegree(max_degree as u32, max_degree);
    result.assert_eq(&a.graded_dimension_string());

    #[cfg(feature = "concurrent")]
    {
        let bucket = std::sync::Arc::new(TokenBucket::new(2));
        let b = construct(&config).unwrap();
        b.compute_through_bidegree_concurrent(max_degree as u32, max_degree, &bucket);
        result.assert_eq(&b.graded_dimension_string());
    }
}

#[test]
fn check_non_rectangular() {
    let resolution = construct_s_2::<&str>("adem", None);

    resolution.compute_through_bidegree(6, 6);
    resolution.compute_through_bidegree(2, 20);

    expect_file!["benchmarks/S_2_L"].assert_eq(&resolution.graded_dimension_string());
}

#[cfg(feature = "concurrent")]
#[test]
fn check_non_rectangular_concurrent() {
    let resolution = construct_s_2::<&str>("adem", None);

    let bucket = std::sync::Arc::new(TokenBucket::new(2));
    resolution.compute_through_bidegree_concurrent(6, 6, &bucket);
    resolution.compute_through_bidegree_concurrent(2, 20, &bucket);

    expect_file!["benchmarks/S_2_L"].assert_eq(&resolution.graded_dimension_string());
}
