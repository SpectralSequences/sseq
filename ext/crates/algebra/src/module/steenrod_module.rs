use crate::algebra::SteenrodAlgebra;

pub type SteenrodModule = Box<dyn Module<Algebra = SteenrodAlgebra>>;

#[cfg(feature = "json")]
mod json {
    use super::*;

    use crate::module::{FDModule, FPModule, RealProjectiveSpace};
    use anyhow::anyhow;
    use std::sync::Arc;

    pub fn from_json(
        algebra: Arc<SteenrodAlgebra>,
        json: &serde_json::Value,
    ) -> anyhow::Result<SteenrodModule> {
        match json["type"].as_str() {
            Some("real projective space") => {
                Ok(Box::new(RealProjectiveSpace::from_json(algebra, json)?))
            }
            Some("finite dimensional module") => Ok(Box::new(FDModule::from_json(algebra, json)?)),
            Some("finitely presented module") => Ok(Box::new(FPModule::from_json(algebra, json)?)),
            Some(x) => Err(anyhow!("Unknown module type: {}", x)),
            None => Err(anyhow!("Missing module type")),
        }
    }
}

#[cfg(feature = "json")]
pub use json::*;

use super::Module;
