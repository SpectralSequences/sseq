extern crate rust_ext;

use std::error::Error;

#[allow(unreachable_code)]
#[allow(non_snake_case)]
#[allow(unused_mut)]
fn main() {
    let args : Vec<_> = std::env::args().collect();
    let config = rust_ext::Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        std::process::exit(1);
    });

    match rust_ext::run(config) {
        Ok(string) => println!("{}", string),
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

        match (run(a), run(m)) {
            (Err(e), _)    => panic!("Failed to read file: {}", e),
            (_, Err(e))    => panic!("Failed to read file: {}", e),
            (Ok(x), Ok(y)) => assert_eq!(x, y)
        }
    }
}
