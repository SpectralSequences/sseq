"""Tests for the secondary (``Mod_{Cλ²}``) family:
``SecondaryResolutionHomomorphism`` and ``SecondaryChainHomotopy``
(``ext::secondary``).

These lift a ``ResolutionHomomorphism`` / ``ChainHomotopy`` to a map respecting
the secondary (``d₂``) structure, used to compute secondary products and Massey
products (see ``examples/secondary_product.rs`` / ``examples/secondary_massey.rs``,
the canonical constructions mirrored here).

Only the standard backend is reachable: a ``SecondaryResolutionHomomorphism`` is
built from two ``SecondaryResolution``s (standard-only — Nassau is rejected at
their construction) and a ``ResolutionHomomorphism`` (also standard-only).

Construction pre-checks every upstream ``Arc::ptr_eq`` ``assert!`` and raises
``ValueError`` rather than panicking across the FFI boundary; ``extend_all``
checks the source/target secondary resolutions are extended far enough.

A concrete secondary-product *value* is not derivable without the full
multi-step workflow (which lives in Python ``examples/``), so structural
invariants are asserted: shift relationships, source/target identities, name
bracketing, and that ``extend_all`` succeeds on fully-extended prerequisites.
"""

import pytest

import ext
from ext import sseq

Bidegree = sseq.Bidegree


def sphere(max_st=8):
    r = ext.Resolution("S_2", "standard")
    r.compute_through_bidegree(Bidegree.s_t(max_st, max_st))
    return r


def h0(r, name, ext_deg=6):
    """Multiplication by h0 = (n=0, s=1) = (s=1, t=1) as a ResolutionHomomorphism
    r -> r, extended over the (ext_deg, ext_deg) rectangle."""
    hom = ext.ResolutionHomomorphism.from_class(name, r, r, Bidegree.s_t(1, 1), [1])
    hom.extend(Bidegree.s_t(ext_deg, ext_deg))
    return hom


# --- SecondaryResolutionHomomorphism: construction & accessors -------------


def test_secondary_res_hom_construction_and_accessors():
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    hom = h0(r, "h0")
    sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, hom)
    assert sec.prime() == 2
    # shift = underlying.shift + (1, 0) = (1, 1) + (1, 0) = (2, 1).
    assert sec.shift().s == 2
    assert sec.shift().t == 1
    # name is bracketed to mark it as the secondary lift.
    assert sec.name() == "[h0]"
    # underlying() shares the same ResolutionHomomorphism (same name).
    assert isinstance(sec.underlying(), ext.ResolutionHomomorphism)
    assert sec.underlying().name() == "h0"
    # source()/target() are the underlying resolutions.
    assert isinstance(sec.source(), ext.Resolution)
    assert sec.source().prime() == 2
    assert sec.target().prime() == 2
    assert sec.save_dir() is None


def test_secondary_res_hom_extend_all_succeeds_when_extended():
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    res_lift.extend_all()
    hom = h0(r, "h0")
    sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, hom)
    # Fully extended prerequisites -> extend_all succeeds (no panic). h0 is a
    # permanent cycle supporting no d2, so the lift-validity assert does not fire.
    sec.extend_all()


# --- SecondaryResolutionHomomorphism: guards -------------------------------


def test_secondary_res_hom_rejects_mismatched_underlying_source():
    # Source secondary resolution over r1, but underlying hom over r2.
    r1 = sphere(6)
    r2 = sphere(6)
    res_lift1 = ext.SecondaryResolution(r1)
    hom = h0(r2, "h0")
    with pytest.raises(ValueError):
        ext.SecondaryResolutionHomomorphism(res_lift1, res_lift1, hom)


def test_secondary_res_hom_rejects_mismatched_underlying_target():
    r1 = sphere(6)
    r2 = sphere(6)
    res_lift1 = ext.SecondaryResolution(r1)
    res_lift2 = ext.SecondaryResolution(r2)
    # underlying maps r1 -> r1, but target secondary resolution is over r2.
    hom = h0(r1, "h0")
    with pytest.raises(ValueError):
        ext.SecondaryResolutionHomomorphism(res_lift1, res_lift2, hom)


def test_secondary_res_hom_extend_all_rejects_unextended_secondary():
    # The underlying hom is extended (so the touched range is non-empty), but the
    # source/target SecondaryResolution was never extend_all-ed: extend_all must
    # raise a clean ValueError, not index an empty OnceBiVec / panic.
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)  # NOT extended
    hom = h0(r, "h0")
    sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, hom)
    with pytest.raises(ValueError):
        sec.extend_all()


# --- SecondaryChainHomotopy: construction & accessors ----------------------


def make_secondary_chain_homotopy(r, res_lift):
    left = h0(r, "a")
    right = h0(r, "b")
    left_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, left)
    right_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, right)
    ch = ext.ChainHomotopy(left, right)
    return ext.SecondaryChainHomotopy(left_sec, right_sec, ch)


def test_secondary_chain_homotopy_construction_and_accessors():
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    sec_ch = make_secondary_chain_homotopy(r, res_lift)
    assert sec_ch.prime() == 2
    assert sec_ch.source().prime() == 2
    assert sec_ch.target().prime() == 2
    # underlying() shares the ChainHomotopy.
    assert isinstance(sec_ch.underlying(), ext.ChainHomotopy)
    assert sec_ch.underlying().prime() == 2
    assert sec_ch.save_dir() is None
    # algebra() round-trips.
    assert sec_ch.algebra().prime() == 2


# --- SecondaryChainHomotopy: guards ----------------------------------------


def test_secondary_chain_homotopy_rejects_mismatched_underlying():
    # The ChainHomotopy is built from *different* homomorphism objects than the
    # secondary lifts wrap, so the upstream Arc::ptr_eq assert would fail; the
    # binding must reject with a ValueError, not panic.
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    left = h0(r, "a")
    right = h0(r, "b")
    left_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, left)
    right_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, right)
    # Fresh homomorphisms (different Arcs) for the ChainHomotopy.
    left2 = h0(r, "a2")
    right2 = h0(r, "b2")
    ch = ext.ChainHomotopy(left2, right2)
    with pytest.raises(ValueError):
        ext.SecondaryChainHomotopy(left_sec, right_sec, ch)


def test_secondary_chain_homotopy_rejects_bad_lambda_shift():
    # A left_lambda with the wrong shift must be rejected (upstream assert_eq).
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    left = h0(r, "a")
    right = h0(r, "b")
    left_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, left)
    right_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, right)
    ch = ext.ChainHomotopy(left, right)
    # left_lambda must have shift = left.shift + LAMBDA_BIDEGREE = (1,1)+(1,1)
    # = (2,2); supply a wrong shift (3,3) to trigger the guard.
    bad_lambda = ext.ResolutionHomomorphism.from_class(
        "al", r, r, Bidegree.s_t(3, 3), [1]
    )
    with pytest.raises(ValueError):
        ext.SecondaryChainHomotopy(
            left_sec, right_sec, ch, left_lambda=bad_lambda
        )


def test_secondary_chain_homotopy_rejects_lambda_source_target_mismatch():
    # left_lambda with the CORRECT shift ((1,1)+LAMBDA(1,1) = (2,2)) but built
    # over a *different* resolution than underlying.left(): only the source/
    # target Arc::ptr_eq branch can reject (distinct from the shift-mismatch
    # branch exercised above).
    r = sphere(8)
    r2 = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    left = h0(r, "a")
    right = h0(r, "b")
    left_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, left)
    right_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, right)
    ch = ext.ChainHomotopy(left, right)
    bad_lambda = ext.ResolutionHomomorphism.from_class(
        "al", r2, r2, Bidegree.s_t(2, 2), [1]
    )
    with pytest.raises(ValueError):
        ext.SecondaryChainHomotopy(
            left_sec, right_sec, ch, left_lambda=bad_lambda
        )


def test_secondary_chain_homotopy_rejects_bad_right_lambda():
    # right_lambda with the wrong shift -> ValueError (exercises the right_lambda
    # reject branch, the mirror of the left_lambda guard).
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    left = h0(r, "a")
    right = h0(r, "b")
    left_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, left)
    right_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, right)
    ch = ext.ChainHomotopy(left, right)
    bad_right_lambda = ext.ResolutionHomomorphism.from_class(
        "br", r, r, Bidegree.s_t(3, 3), [1]
    )
    with pytest.raises(ValueError):
        ext.SecondaryChainHomotopy(
            left_sec, right_sec, ch, right_lambda=bad_right_lambda
        )


def test_secondary_chain_homotopy_accepts_valid_lambdas():
    # Exercise the left_lambda AND right_lambda Some(..) *acceptance* paths:
    # lambdas with the correct source/target identity and shift ((2,2))
    # construct cleanly.
    r = sphere(8)
    res_lift = ext.SecondaryResolution(r)
    left = h0(r, "a")
    right = h0(r, "b")
    left_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, left)
    right_sec = ext.SecondaryResolutionHomomorphism(res_lift, res_lift, right)
    ch = ext.ChainHomotopy(left, right)
    left_lambda = ext.ResolutionHomomorphism.from_class(
        "al", r, r, Bidegree.s_t(2, 2), [1]
    )
    right_lambda = ext.ResolutionHomomorphism.from_class(
        "bl", r, r, Bidegree.s_t(2, 2), [1]
    )
    sec_ch = ext.SecondaryChainHomotopy(
        left_sec,
        right_sec,
        ch,
        left_lambda=left_lambda,
        right_lambda=right_lambda,
    )
    assert sec_ch.prime() == 2
    assert isinstance(sec_ch.underlying(), ext.ChainHomotopy)


# --- backend rejection (Nassau) --------------------------------------------


def test_secondary_resolution_rejects_nassau():
    # SecondaryResolution (and hence the whole secondary family) is standard-only.
    rn = ext.Resolution("S_2", "nassau")
    rn.compute_through_bidegree(Bidegree.s_t(4, 4))
    with pytest.raises(ValueError):
        ext.SecondaryResolution(rn)
