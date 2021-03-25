#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::implicit_hasher)]
#![warn(clippy::default_trait_access)]
#![warn(clippy::if_not_else)]
#![warn(clippy::needless_continue)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::explicit_into_iter_loop)]

mod run;

use clap::{load_yaml, value_t, App};
use ext::utils::Config;

const BOLD_ANSI_CODE: &str = "\x1b[1m";

#[allow(unreachable_code)]
fn main() -> error::Result<()> {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let result = match matches.subcommand() {
        ("module", Some(_)) => {
            run::define_module()?
        }
        ("yoneda", Some(_)) => {
            run::yoneda(&get_config(matches))?
        }
        ("secondary", Some(_)) => {
            run::secondary()?
        }
        (_, _) => {
            run::resolve(&get_config(matches))?
        }
    };
    println!("{}{}", BOLD_ANSI_CODE, result);
    Ok(())
}

fn get_config(matches: clap::ArgMatches<'_>) -> Config {
    let mut static_modules_path = std::env::current_exe().unwrap();
    static_modules_path.pop();
    static_modules_path.pop();
    static_modules_path.pop();
    static_modules_path.push("steenrod_modules");
    let current_dir = std::env::current_dir().unwrap();
    Config {
        module_paths: vec![current_dir, static_modules_path],
        module_file_name: matches.value_of("module").unwrap().to_string(),
        algebra_name: matches.value_of("algebra").unwrap().to_string(),
        max_degree: value_t!(matches, "degree", i32)
            .unwrap_or_else(|e| panic!("Invalid degree: {}", e)),
    }
}
