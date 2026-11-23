//! Secondary actions on a module whose zero secondary lift does not exist.

use std::sync::Arc;

use ext::{
    secondary::{SecondaryLift, SecondaryResolution},
    utils::construct_standard,
};
use sseq::coordinates::Bidegree;

/// Whether the secondary lift of the minimal resolution of `json` exists through `max`.
fn lifts(json: serde_json::Value, max: Bidegree) -> bool {
    let resolution =
        construct_standard::<false, _, _>((json, algebra::AlgebraType::Milnor), None).unwrap();
    resolution.compute_through_stem(max);
    let lift = SecondaryResolution::new(Arc::new(resolution));
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lift.extend_all())).is_ok()
}

/// `RP_14^31`, which is `Σ^{-96} RP_110^127` by James periodicity. Its zero secondary lift fails
/// at `(26, 3)`. The secondary actions in `data/` are a solution of the bar equation; on the
/// minimal resolution their comparison correction is not a coboundary, so they lift only because
/// it is applied.
#[test]
fn rp_14_31() {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("data/RP14_31_secondary.json")).unwrap();
    let mut zero = json.clone();
    zero["secondary_actions"].take();
    let max = Bidegree::n_s(30, 4);
    assert!(!lifts(zero, max));
    assert!(lifts(json, max));
}
