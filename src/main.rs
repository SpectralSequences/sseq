extern crate rust_ext;

use rust_ext::Config;
use rust_ext::run;

const BOLD_ANSI_CODE : &str = "\x1b[1m";

#[allow(unreachable_code)]
fn main() {
    let args : Vec<_> = std::env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        std::process::exit(1);
    });

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
            module_name : String::from(module_name),
            max_degree,
            algebra_name : String::from("adem")
        };
        let m = Config {
            module_name : String::from(module_name),
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
