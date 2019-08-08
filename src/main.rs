extern crate rust_ext;
#[macro_use]
extern crate clap;

use rust_ext::Config;
use rust_ext::run;
use rust_ext::run_interactive;
use clap::App;

const BOLD_ANSI_CODE : &str = "\x1b[1m";

#[allow(unreachable_code)]
fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let mut static_modules_path = std::env::current_exe().unwrap();
    static_modules_path.pop(); static_modules_path.pop(); static_modules_path.pop();
    static_modules_path.push("static/modules");
    let current_dir = std::env::current_dir().unwrap();
    let config = Config {
        module_paths : vec![current_dir, static_modules_path],
        module_file_name : matches.value_of("module").unwrap().to_string(),
        algebra_name : matches.value_of("algebra").unwrap().to_string(),
        max_degree : value_t!(matches, "degree", i32).unwrap_or_else(|e| panic!("Invalid degree: {}", e))
    };

    if matches.is_present("test") {
        rust_ext::test(&config);
        std::process::exit(1);
    }

    if matches.is_present("interactive") {
        match run_interactive() {
            Ok(string) => println!("{}{}", BOLD_ANSI_CODE, string),
            Err(e) => { eprintln!("Application error: {}", e); std::process::exit(1); }
        }
        std::process::exit(1);
    }

    match run(&config) {
        Ok(string) => println!("{}{}", BOLD_ANSI_CODE, string),
        Err(e) => { eprintln!("Application error: {}", e); std::process::exit(1); }
    }
}
