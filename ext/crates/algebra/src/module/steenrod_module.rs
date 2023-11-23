use crate::algebra::SteenrodAlgebra;

pub type SteenrodModule = Box<dyn Module<Algebra = SteenrodAlgebra>>;

mod json {
    use super::*;

    use crate::module::{FDModule, FPModule, RealProjectiveSpace, SuspensionModule};
    use anyhow::anyhow;
    use std::sync::Arc;

    pub fn from_json(
        algebra: Arc<SteenrodAlgebra>,
        json: &serde_json::Value,
    ) -> anyhow::Result<SteenrodModule> {
        fn box_new(
            m: impl Module<Algebra = SteenrodAlgebra>,
            json: &serde_json::Value,
        ) -> SteenrodModule {
            if let Some(shift) = json["shift"].as_i64() {
                Box::new(SuspensionModule::new(Arc::new(m), shift as i32))
            } else {
                Box::new(m)
            }
        }

        match json["type"].as_str() {
            Some("real projective space") => Ok(box_new(
                RealProjectiveSpace::from_json(algebra, json)?,
                json,
            )),
            Some("finite dimensional module") => {
                Ok(box_new(FDModule::from_json(algebra, json)?, json))
            }
            Some("finitely presented module") => {
                Ok(box_new(FPModule::from_json(algebra, json)?, json))
            }
            Some(x) => Err(anyhow!("Unknown module type: {}", x)),
            None => Err(anyhow!("Missing module type")),
        }
    }
}

pub use json::*;

use super::Module;
