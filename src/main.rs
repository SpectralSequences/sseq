use rust_ext::Config;
use rust_ext::run_resolve;
use rust_ext::run_define_module;
use rust_ext::run_test;
use rust_ext::run_yoneda;
use rust_ext::run_steenrod;
use clap::{App, load_yaml, value_t};

const BOLD_ANSI_CODE : &str = "\x1b[1m";

#[allow(unreachable_code)]
fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let result;
    match matches.subcommand() {
        ("module", Some(_sub_m)) => {
            result = run_define_module();
        },
        ("test", Some(_sub_m)) => {
            run_test().unwrap();
            return;
        },
        ("yoneda", Some(_sub_m)) => {
            result = run_yoneda(&get_config(matches));
        },
        ("steenrod", Some(_)) => {
            result = run_steenrod();
        },
        (_, _) => {
            result = run_resolve(&get_config(matches));
        }
    }
    match result {
        Ok(string) => println!("{}{}", BOLD_ANSI_CODE, string),
        Err(e) => { eprintln!("Application error: {}", e); std::process::exit(1); }
    }
}

fn get_config(matches : clap::ArgMatches<'_>) -> Config {
    let mut static_modules_path = std::env::current_exe().unwrap();
    static_modules_path.pop(); static_modules_path.pop(); static_modules_path.pop();
    static_modules_path.push("modules");
    let current_dir = std::env::current_dir().unwrap();
    Config {
        module_paths : vec![current_dir, static_modules_path],
        module_file_name : matches.value_of("module").unwrap().to_string(),
        algebra_name : matches.value_of("algebra").unwrap().to_string(),
        max_degree : value_t!(matches, "degree", i32).unwrap_or_else(|e| panic!("Invalid degree: {}", e))
    }
}
