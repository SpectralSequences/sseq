#![feature(backtrace)]
use std::error::Error as StdError;
use std::backtrace::Backtrace;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    error: Box<dyn StdError + Send + Sync + 'static>,
    backtrace: Backtrace,
}

impl Error {
    pub fn inner(&self) -> &(dyn StdError + Send + Sync + 'static) {
        &*self.error
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.error)?;
        writeln!(f, "{}", self.backtrace)?;
        Ok(())
    }
}

impl<E: StdError + Send + Sync + 'static> From<E> for Error {
    fn from(error: E) -> Error {
        Self {
            error: Box::new(error),
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<Error> for Box<dyn StdError + Send + Sync + 'static> {
    fn from(e: Error) -> Box<dyn StdError + Send + Sync + 'static> {
        e.error
    }
}

impl From<Error> for Box<dyn StdError> {
    fn from(e: Error) -> Box<dyn StdError> {
        e.error
    }
}

#[derive(Debug)]
pub struct GenericError(String);

impl GenericError {
    pub fn new(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl std::error::Error for GenericError {}
