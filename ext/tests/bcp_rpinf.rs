use ext::{
    chain_complex::{ChainComplex, FreeChainComplex},
    utils::construct,
};
use sseq::coordinates::Bidegree;

/// At p=2, BZ/2 = RP^∞, so their Ext groups should agree.
#[test]
fn bcp_p2_equals_rp_inf() {
    let max = Bidegree::s_t(30, 30);

    let bcp = construct(("BCp2", "milnor"), None).unwrap();
    let rp = construct(("RP_inf", "milnor"), None).unwrap();

    bcp.compute_through_bidegree(max);
    rp.compute_through_bidegree(max);

    assert_eq!(bcp.graded_dimension_string(), rp.graded_dimension_string());
}
