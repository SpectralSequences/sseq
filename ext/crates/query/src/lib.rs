//! This library gives various functions that are used to query a user. Each function performs the
//! following:
//!
//!  - Read the next command line argument, and try to parse it as an answer. If the parsing fails,
//!    panic.  Otherwise, return the argument
//!
//!  - If there are no command line arguments left, query the user for an input, and parse it as an
//!    answer. If the parsing fails, query the user again.
//!
//! The "normal" usage mode is to not supply any command line arguments and just use the second
//! functionality. However, the first is useful for testing and batch processing.

use std::fmt::Display;
use std::io::{stderr, stdin, Write};
use std::str::FromStr;

use std::cell::RefCell;
use std::env::Args;

thread_local! {
    static ARGV: RefCell<Args> = {
        let mut args = std::env::args();
        args.next();
        RefCell::new(args)
    }
}

pub fn optional<S, T: FromStr>(
    prompt: &str,
    validator: impl Fn(T) -> Result<S, String>,
) -> Option<S>
where
    <T as FromStr>::Err: Display,
{
    inner(prompt, |x| {
        if x.is_empty() {
            Ok(None)
        } else {
            x.parse::<T>()
                .map_err(|err| err.to_string())
                .and_then(|res| validator(res))
                .map(Some)
        }
    })
}

pub fn with_default<S, T: FromStr>(
    prompt: &str,
    default: &str,
    validator: impl Fn(T) -> Result<S, String>,
) -> S
where
    <T as FromStr>::Err: Display,
{
    inner(&format!("{} (default: {})", prompt, default), |x| {
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

pub fn raw<S, T: FromStr>(prompt: &str, validator: impl Fn(T) -> Result<S, String>) -> S
where
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

pub fn inner<S>(prompt: &str, validator: impl for<'a> Fn(&'a str) -> Result<S, String>) -> S {
    let cli: Option<(String, Result<S, String>)> = ARGV.with(|argv| {
        let arg = argv.borrow_mut().next()?;
        let result = validator(&arg);
        Some((arg, result))
    });

    match cli {
        Some((arg, Ok(res))) => {
            eprintln!("{}: {}", prompt, arg);
            return res;
        }
        Some((arg, Err(e))) => {
            panic!("Invalid input {}\n{}.", arg, e);
        }
        None => (),
    }

    loop {
        eprint!("{}: ", prompt);
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
