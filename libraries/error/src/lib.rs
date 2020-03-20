use std::error::Error as StdError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    error: Box<dyn StdError + Send + Sync + 'static>,
    backtrace: backtrace::Backtrace,
}

impl Error {
    pub fn to_string(&self) -> String {
        self.error.to_string()
    }
}

impl<E: StdError + Send + Sync + 'static> From<E> for Error {
    fn from(error: E) -> Error {
        Self {
            error: Box::new(error),
            backtrace: backtrace::Backtrace::new(),
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
