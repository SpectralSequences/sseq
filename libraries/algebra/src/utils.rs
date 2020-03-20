
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
