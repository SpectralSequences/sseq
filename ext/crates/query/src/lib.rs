use std::fmt::Display;
use std::io::{stderr, stdin, Write};
use std::str::FromStr;

pub fn optional<S: Display, T: FromStr, F>(prompt: &str, validator: F) -> Option<S>
where
    F: Fn(T) -> Result<S, String>,
    <T as FromStr>::Err: Display,
{
    inner(prompt, |x| {
        if x.is_empty() {
            Ok(None)
        } else {
            x.parse::<T>()
                .map_err(|err| err.to_string())
                .and_then(|res| validator(res))
                .map(|x| Some(x))
        }
    })
}

pub fn with_default<S: Display, T: FromStr, F>(prompt: &str, default: &str, validator: F) -> S
where
    F: Fn(T) -> Result<S, String>,
    <T as FromStr>::Err: Display,
{
    inner(prompt, |x| {
        if x.is_empty() {
            default
                .parse::<T>()
                .map_err(|err| err.to_string())
                .and_then(|res| validator(res))
        } else {
            x.parse::<T>()
                .map_err(|err| err.to_string())
                .and_then(|res| validator(res))
        }
    })
}

pub fn raw<S: Display, T: FromStr, F>(prompt: &str, validator: F) -> S
where
    F: Fn(T) -> Result<S, String>,
    <T as FromStr>::Err: Display,
{
    inner(prompt, |x| {
        x.parse::<T>()
            .map_err(|err| err.to_string())
            .and_then(|res| validator(res))
    })
}

pub fn yes_no(prompt: &str) -> bool {
    with_default(prompt, "y", |response: String| {
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

pub fn inner<S, F>(prompt: &str, validator: F) -> S
where
    F: for<'a> Fn(&'a str) -> Result<S, String>,
{
    loop {
        eprint!("{} : ", prompt);
        stderr().flush().unwrap();
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .unwrap_or_else(|_| panic!("Error reading for prompt: {}", prompt));
        let trimmed = input.trim();
        match validator(trimmed) {
            Ok(res) => {
                return res;
            }
            Err(e) => {
                eprintln!("Invalid input: {}. Try again", e);
            }
        }
    }
}
