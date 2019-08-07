extern crate rust_ext;
#[macro_use]
extern crate clap;

use rust_ext::Config;
use rust_ext::run;
use clap::App;

const BOLD_ANSI_CODE : &str = "\x1b[1m";

#[allow(unreachable_code)]
fn main() {
        rust_ext::test_no_config();
        std::process::exit(1);
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let config = Config {
        module_path : format!("{}/{}.json", matches.value_of("directory").unwrap(), matches.value_of("module").unwrap()),
        algebra_name : matches.value_of("algebra").unwrap().to_string(),
        max_degree : value_t!(matches, "degree", i32).unwrap_or_else(|e| panic!("Invalid degree: {}", e))
    };

    if matches.is_present("test") {
        rust_ext::test(&config);
        std::process::exit(1);
    }

    match run(&config) {
        Ok(string) => println!("{}{}", BOLD_ANSI_CODE, string),
        Err(e) => { eprintln!("Application error: {}", e); std::process::exit(1); }
    }
}
