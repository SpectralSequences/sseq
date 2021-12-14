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

use std::cell::RefCell;
use std::env::Args;

thread_local! {
    static ARGV: RefCell<Args> = {
        let mut args = std::env::args();
        args.next();
        RefCell::new(args)
    }
}

pub fn optional<S, E: Display>(
    prompt: &str,
    mut parser: impl for<'a> FnMut(&'a str) -> Result<S, E>,
) -> Option<S> {
    raw(&format!("{} (optional)", prompt), |x| {
        if x.is_empty() {
            Ok(None)
        } else {
            parser(x).map(Some)
        }
    })
}

pub fn with_default<S, E: Display>(
    prompt: &str,
    default: &str,
    mut parser: impl for<'a> FnMut(&'a str) -> Result<S, E>,
) -> S {
    raw(&format!("{} (default: {})", prompt, default), |x| {
        if x.is_empty() {
            parser(default)
        } else {
            parser(x)
        }
    })
}

pub fn yes_no(prompt: &str) -> bool {
    with_default(prompt, "y", |response| {
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

pub fn raw<S, E: Display>(
    prompt: &str,
    mut parser: impl for<'a> FnMut(&'a str) -> Result<S, E>,
) -> S {
    let cli: Option<(String, Result<S, E>)> = ARGV.with(|argv| {
        let arg = argv.borrow_mut().next()?;
        let result = parser(&arg);
        Some((arg, result))
    });

    match cli {
        Some((arg, Ok(res))) => {
            eprintln!("{}: {}", prompt, arg);
            return res;
        }
        Some((arg, Err(e))) => {
            eprintln!("{}: {}", prompt, arg);
            eprintln!("{:#}", e);
            std::process::exit(1);
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
        match parser(trimmed) {
            Ok(res) => {
                return res;
            }
            Err(e) => {
                eprintln!("{:#}\n\nTry again", e);
            }
        }
    }
}

pub fn vector(prompt: &str, len: usize) -> Vec<u32> {
    raw(prompt, |s| {
        let v = s[1..s.len() - 1]
            .split(',')
            .map(|x| x.trim().parse::<u32>().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, String>>()?;
        if v.len() != len {
            return Err(format!(
                "Target has dimension {} but {} coordinates supplied",
                len,
                v.len()
            ));
        }
        Ok(v)
    })
}
