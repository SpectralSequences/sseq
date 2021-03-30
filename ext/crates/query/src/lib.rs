use std::fmt::Display;
use std::io::{stdin, stdout, Write};
use std::str::FromStr;

pub fn query_optional<S: Display, T: FromStr, F>(prompt: &str, validator: F) -> Option<S>
where
    F: Fn(T) -> Result<S, String>,
    <T as FromStr>::Err: Display,
{
    loop {
        print!("{}: ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .unwrap_or_else(|_| panic!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }
        let result = trimmed
            .parse::<T>()
            .map_err(|err| format!("{}", err))
            .and_then(|res| validator(res));
        match result {
            Ok(res) => {
                return Some(res);
            }
            Err(e) => {
                println!("Invalid input: {}. Try again", e);
            }
        }
    }
}

pub fn query<S: Display, T: FromStr, F>(prompt: &str, validator: F) -> S
where
    F: Fn(T) -> Result<S, String>,
    <T as FromStr>::Err: Display,
{
    loop {
        print!("{} : ", prompt);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .unwrap_or_else(|_| panic!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        let result = trimmed
            .parse::<T>()
            .map_err(|err| format!("{}", err))
            .and_then(|res| validator(res));
        match result {
            Ok(res) => {
                return res;
            }
            Err(e) => {
                println!("Invalid input: {}. Try again", e);
            }
        }
    }
}

pub fn query_with_default<S: Display, T: FromStr, F>(prompt: &str, default: &str, validator: F) -> S
where
    F: Fn(T) -> Result<S, String>,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    loop {
        print!("{} (default: {}): ", prompt, default);
        stdout().flush().unwrap();
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .unwrap_or_else(|_| panic!("Error reading for prompt: {}", prompt));
        let mut trimmed = input.trim();
        if trimmed.is_empty() {
            trimmed = default;
        }
        let result = trimmed
            .parse::<T>()
            .map_err(|err| format!("{}", err))
            .and_then(|res| validator(res));
        match result {
            Ok(res) => {
                return res;
            }
            Err(e) => {
                println!("Invalid input: {}. Try again", e);
            }
        }
    }
}

pub fn query_yes_no(prompt: &str) -> bool {
    query_with_default(prompt, "y", |response: String| {
        if response.starts_with('y') || response.starts_with('n') {
            Ok(response.starts_with('y'))
        } else {
            Err(format!(
                "unrecognized response '{}'. Should be '(y)es' or '(n)o'",
                response
            ))
        }
    })
}
