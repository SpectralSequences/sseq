use std::sync::Arc;

use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::construct,
    yoneda::try_yoneda_representative_element,
};
use sseq::coordinates::Bidegree;

#[test]
fn try_yoneda_representative_element_validates_input() {
    let resolution = Arc::new(construct(("S_2", "milnor"), None).unwrap());
    resolution.compute_through_bidegree(Bidegree::s_t(4, 4));

    // VALID case: the bottom class (0, 0) is a computed bidegree with exactly one
    // generator, so a class vector of length 1 is well-formed.
    let bottom = Bidegree::s_t(0, 0);
    assert!(resolution.has_computed_bidegree(bottom));
    assert_eq!(resolution.number_of_gens_in_bidegree(bottom), 1);
    assert!(try_yoneda_representative_element(Arc::clone(&resolution), bottom, &[1]).is_ok());

    // VALID case: the h_0 class lives at (s, t) = (1, 1), a computed nonzero bidegree.
    let h0 = Bidegree::s_t(1, 1);
    assert!(resolution.has_computed_bidegree(h0));
    let h0_gens = resolution.number_of_gens_in_bidegree(h0);
    assert_eq!(h0_gens, 1);
    let class: Vec<u32> = vec![1; h0_gens];
    assert!(try_yoneda_representative_element(Arc::clone(&resolution), h0, &class).is_ok());

    // INVALID case: an uncomputed bidegree (far beyond the resolved range) must return
    // Err instead of panicking.
    let uncomputed = Bidegree::s_t(100, 100);
    assert!(!resolution.has_computed_bidegree(uncomputed));
    assert!(
        try_yoneda_representative_element(Arc::clone(&resolution), uncomputed, &[]).is_err(),
        "uncomputed bidegree should error, not panic"
    );

    // INVALID case: a class vector whose length does not match the generator count at a
    // computed bidegree must return Err instead of panicking.
    let wrong_len = vec![0; resolution.number_of_gens_in_bidegree(bottom) + 3];
    assert!(
        try_yoneda_representative_element(Arc::clone(&resolution), bottom, &wrong_len).is_err(),
        "class of wrong length should error, not panic"
    );
}
