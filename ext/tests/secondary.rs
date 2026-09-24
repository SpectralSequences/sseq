use std::sync::Arc;

use ext::{
    resolution_homomorphism::ResolutionHomomorphism, secondary::*, utils::construct_standard,
};
use sseq::coordinates::Bidegree;

#[test]
fn boundary_has_boundary() {
    ext::utils::init_logging();

    // Construct resolution of S_2 to stem 15
    let resolution = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
    resolution.compute_through_stem(Bidegree::n_s(15, 9));

    // Lift to secondary resolution
    let res_lift = Arc::new(SecondaryResolution::new(Arc::clone(&resolution)));
    res_lift.extend_all();

    // Construct lift of "h0". This could ready be any element, but we choose "h0" for simplicity.
    let hom = ResolutionHomomorphism::from_class(
        "h0".to_string(),
        Arc::clone(&resolution),
        Arc::clone(&resolution),
        Bidegree::n_s(0, 1),
        &[1],
    );

    // Extend only to stem 14. The class in (14, 3) is a boundary.
    hom.extend_through_stem(Bidegree::n_s(14, 9));

    // Lift to secondary homomorphism
    let hom_lift = SecondaryResolutionHomomorphism::new(
        Arc::clone(&res_lift),
        Arc::clone(&res_lift),
        Arc::new(hom),
    );

    // Crash
    hom_lift.extend_all();
}
