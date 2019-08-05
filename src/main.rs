extern crate rust_ext;
#[macro_use]
extern crate clap;

use rust_ext::Config;
use rust_ext::run;
use clap::App;

const BOLD_ANSI_CODE : &str = "\x1b[1m";

#[allow(unreachable_code)]
fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let config = Config {
        module_path : format!("{}/{}.json", matches.value_of("directory").unwrap(), matches.value_of("module").unwrap()),
        algebra_name : matches.value_of("algebra").unwrap().to_string(),
        max_degree : value_t!(matches, "degree", i32).unwrap_or_else(|e| panic!("Invalid degree: {}", e))
    };

    match run(&config) {
        Ok(string) => println!("{}{}", BOLD_ANSI_CODE, string),
        Err(e) => { eprintln!("Application error: {}", e); std::process::exit(1); }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn milnor_vs_adem() {
        compare("S_2", 30);
        compare("C2", 30);
        compare("Joker", 30);
        compare("RP4", 30);
        compare("Csigma", 30);
        compare("S_3", 30);
        compare("Calpha", 30);
        compare("C3", 60);
    }

    fn compare(module_name : &str, max_degree : i32) {
        println!("module : {}", module_name);
        let a = Config {
            module_path : format!("static/modules/{}.json", module_name),
            max_degree,
            algebra_name : String::from("adem")
        };
        let m = Config {
            module_path : format!("static/modules/{}.json", module_name),
            max_degree,
            algebra_name : String::from("milnor")
        };

        match (run(&a), run(&m)) {
            (Err(e), _)    => panic!("Failed to read file: {}", e),
            (_, Err(e))    => panic!("Failed to read file: {}", e),
            (Ok(x), Ok(y)) => assert_eq!(x, y)
        }
    }
}
