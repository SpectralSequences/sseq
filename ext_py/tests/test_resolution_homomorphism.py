"""Tests for the `ResolutionHomomorphism` pyclass (`ext::resolution_homomorphism`).

A `ResolutionHomomorphism` is a lifted chain map between two free resolutions —
the resolution-level realisation of a map of `Ext` modules. Only the
*standard*-backend Standard->Standard instantiation is bound (both source and
target are `Resolution(..., "standard")`); a Nassau-backed resolution is rejected
with a clean `ValueError`, because Nassau resolves over the concrete
`MilnorAlgebra` whose maps the bound homomorphism pyclasses cannot represent
(mirroring the standard-only `Resolution.module`/`chain_complex`).

The canonical known value: `from_class` with shift `(0, 0)` and class `[1]` is the
identity chain map lifting the identity on the augmentation, so its `s`-th map
sends generator `idx` to the corresponding basis element (a single `1` at
`operation_generator_to_index(0, 0, t, idx)`). This is exactly the invariant
`ext/tests/extend_identity.rs` asserts for `from_module_homomorphism(identity)`.

All bad degree/index/bidegree inputs are pre-checked and raise
`ValueError`/`IndexError` rather than panicking across the FFI boundary.
"""

import pytest

import ext
from ext import algebra, fp, sseq

Bidegree = sseq.Bidegree
BidegreeGenerator = sseq.BidegreeGenerator
FpVector = fp.FpVector


def s2_rect(max_st=8):
    """A standard-backend resolution of S_2 computed through the full bidegree
    rectangle (max_st, max_st), so an extend over the same range is fully
    resolved."""
    r = ext.Resolution("S_2", "standard")
    r.compute_through_bidegree(Bidegree.s_t(max_st, max_st))
    return r


def identity_hom(r, max_st=8):
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend(Bidegree.s_t(max_st, max_st))
    return hom


# --- construction & accessors ---------------------------------------------


def test_new_accessors_roundtrip():
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(1, 1))
    assert hom.name == "f"
    assert hom.prime == 2
    assert hom.shift.s == 1
    assert hom.shift.t == 1
    # source()/target() share the underlying resolution.
    assert hom.source.prime == 2
    assert hom.target.prime == 2
    assert hom.source.graded_dimension_string() == r.graded_dimension_string()
    # A freshly constructed hom defines no maps yet.
    assert hom.next_homological_degree == 1  # = shift.s
    assert hom.save_dir is None
    assert hom.algebra.prime == 2


# --- known value: from_class([1]) at (0,0) is the identity chain map ------


def test_from_class_identity_matches_basis():
    r = s2_rect(8)
    hom = identity_hom(r, 8)
    assert hom.name == "id"
    assert hom.shift.s == 0 and hom.shift.t == 0

    for s in range(0, 9):
        m = hom.get_map(s)
        src = r.module(s)
        for t in range(0, 9):
            for idx in range(src.number_of_gens_in_degree(t)):
                out = m.output(t, idx)
                j = src.operation_generator_to_index(0, 0, t, idx)
                # Identity: a single 1 at the generator's own basis index.
                for i in range(len(out)):
                    assert out[i] == (1 if i == j else 0), (
                        f"identity mismatch at s={s} t={t} idx={idx} i={i}"
                    )


def test_get_map_is_free_to_free_homomorphism():
    r = s2_rect(4)
    hom = identity_hom(r, 4)
    m = hom.get_map(1)
    # source = r.module(1), target = r.module(0) (output_s = input_s - shift.s = 1).
    assert m.degree_shift == 0
    assert m.prime == 2


# --- extend_all -----------------------------------------------------------


def test_extend_all_then_get_map():
    r = s2_rect(6)
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend_all()
    out = hom.get_map(0).output(0, 0)
    assert out[0] == 1


def test_extend_through_stem():
    r = ext.Resolution("S_2", "standard")
    r.compute_through_stem(Bidegree.n_s(8, 4))
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend_through_stem(Bidegree.n_s(4, 4))
    # The s=0 map is the identity on the unit.
    assert hom.get_map(0).output(0, 0)[0] == 1


# --- extend_step_raw ------------------------------------------------------


def test_extend_step_raw_seed_then_extend_all():
    # Seed the identity map at (0,0) with extra_images=[ [1] ] (sending the
    # unit generator to itself), then fill in the rest by exactness.
    r = s2_rect(6)
    hom = ext.ResolutionHomomorphism("id", r, r, Bidegree.s_t(0, 0))
    rng = hom.extend_step_raw(Bidegree.s_t(0, 0), [FpVector(2, 1)])
    # Returns the (start, end) half-open range of touched degrees.
    assert isinstance(rng, tuple) and len(rng) == 2
    assert rng[0] <= rng[1]
    hom.extend_all()
    assert hom.get_map(0).output(0, 0)[0] == 0  # FpVector(2,1) is the zero seed


def test_extend_step_raw_extra_images_none_runs():
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism("id", r, r, Bidegree.s_t(0, 0))
    rng = hom.extend_step_raw(Bidegree.s_t(0, 0))
    assert isinstance(rng, tuple) and len(rng) == 2


def test_extend_step_raw_uncomputed_bidegree_raises_value_error():
    # Guard: an input bidegree outside the computed range raises ValueError,
    # never a panic across FFI.
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism("id", r, r, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        hom.extend_step_raw(Bidegree.s_t(50, 500), [FpVector(2, 1)])
    # Negative bidegree is a ValueError too.
    with pytest.raises(ValueError):
        hom.extend_step_raw(Bidegree.s_t(-1, 0))
    # Input below the shift's homological degree is rejected.
    shifted = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(1, 1))
    with pytest.raises(ValueError):
        shifted.extend_step_raw(Bidegree.s_t(0, 0))


# --- act ------------------------------------------------------------------


def test_act_identity_picks_out_generator():
    # For the identity resolution homomorphism, acting on a target generator g
    # at (s, t) writes the indicator of that generator into the result vector of
    # length = number of source generators at (s, t) (shift = 0).
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    # h_0 lives at (n, s) = (0, 1), i.e. (s, t) = (1, 1), with one generator.
    b = Bidegree.s_t(1, 1)
    n_src = r.number_of_gens_in_bidegree(b)
    assert n_src == 1
    result = FpVector(2, n_src)
    hom.act(result, 1, BidegreeGenerator.s_t(1, 1, 0))
    # Identity acts as the identity on Ext: the generator maps to itself.
    assert result[0] == 1


def test_act_guards():
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    result = FpVector(2, 1)
    # Negative generator -> ValueError.
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(-1, 0, 0))
    # Wrong result length -> ValueError.
    bad = FpVector(2, 5)
    with pytest.raises(ValueError):
        hom.act(bad, 1, BidegreeGenerator.s_t(1, 1, 0))
    # Generator index out of range at the target bidegree -> IndexError.
    with pytest.raises(IndexError):
        hom.act(result, 1, BidegreeGenerator.s_t(1, 1, 99))


def test_act_prime_mismatch_errors():
    # result over a different prime than the homomorphism -> ValueError (checked
    # before the length/generator guards).
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    wrong_prime = FpVector(3, 1)
    with pytest.raises(ValueError):
        hom.act(wrong_prime, 1, BidegreeGenerator.s_t(1, 1, 0))


def test_act_map_undefined_at_s_errors():
    # src_s = g.s + shift.s beyond where the chain map is defined -> ValueError.
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    result = FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(99, 99, 0))


def test_act_map_not_extended_far_enough_errors():
    # The map at s exists but is not extended through src_t -> ValueError.
    # Resolve the source over the full (6, 6) rectangle but extend the chain map
    # only through t = 4, so get_map(2).next_degree == 5 and src_t = 5 is out
    # of range.
    r = s2_rect(6)
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    hom.extend(Bidegree.s_t(6, 4))
    result = FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(2, 5, 0))


def test_act_target_s_out_of_range_errors():
    # g.s >= target.next_homological_degree. With shift >= 0 this guard is
    # shadowed by the src_s ("map undefined") guard (src_s = g.s + shift.s >=
    # g.s), so it cannot fire first from the bound API, but the call still
    # raises ValueError.
    r = s2_rect(6)
    hom = identity_hom(r, 6)
    assert hom.target.next_homological_degree == 7
    result = FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(7, 7, 0))


def test_act_degree_overflow_errors():
    # g.s + shift.s / g.t + shift.t overflowing i32 is caught (checked_add) and
    # raised as ValueError rather than wrapping. BidegreeGenerator imposes no
    # upper bound, so i32::MAX is constructible from Python.
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(1, 1))
    result = FpVector(2, 1)
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(2147483647, 0, 0))
    with pytest.raises(ValueError):
        hom.act(result, 1, BidegreeGenerator.s_t(0, 2147483647, 0))


# --- backend rejection ----------------------------------------------------


def test_rejects_nassau_backend():
    standard = s2_rect(4)
    nassau = ext.Resolution("S_2", "nassau")
    nassau.compute_through_stem(Bidegree.n_s(8, 4))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", nassau, standard, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", standard, nassau, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class(
            "f", nassau, nassau, Bidegree.s_t(0, 0), [1]
        )


# --- panic guards ---------------------------------------------------------


def test_new_prime_mismatch_errors():
    # source over p=2, target over p=3 -> ValueError (no computation required;
    # the prime check fires first).
    s2 = s2_rect(4)
    s3 = ext.Resolution("S_3", "standard")
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", s2, s3, Bidegree.s_t(0, 0))


def test_from_class_prime_mismatch_errors():
    s2 = s2_rect(4)
    s3 = ext.Resolution("S_3", "standard")
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class("f", s2, s3, Bidegree.s_t(0, 0), [1])


def test_new_negative_shift_errors():
    # A negative homological-degree shift (shift.s < 0) is nonsensical and
    # rejected, but a negative internal-degree shift (shift.t < 0) is legitimate
    # (e.g. a map out of a stunted projective space RP_{-k}) and is allowed.
    r = s2_rect(4)
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(-1, 0))
    # shift.t < 0 is allowed.
    hom = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(0, -1))
    assert hom.shift.t == -1


def rp_minus_k(k, max_st):
    """A standard-backend resolution of the stunted projective space RP_{-k}^inf
    (min_degree = -k), computed through the (max_st, max_st) rectangle."""
    spec = ({"p": 2, "type": "real projective space", "min": -k}, algebra.AlgebraType.Milnor)
    r = ext.Resolution.construct(spec, None, "standard")
    r.compute_through_bidegree(Bidegree.s_t(max_st, max_st))
    return r


def test_from_class_negative_t_shift_rp():
    # A map OUT OF RP_{-k} into S_2 uses the shift (s=0, t=-k); the binding must
    # allow the negative internal degree once the source is resolved there.
    k = 3
    rp = rp_minus_k(k, 6)
    s2 = s2_rect(6)
    hom = ext.ResolutionHomomorphism.from_class(
        "bottom_cell", rp, s2, Bidegree.s_t(0, -k), [1]
    )
    hom.extend_all()
    assert hom.shift.s == 0
    assert hom.shift.t == -k


def test_from_class_negative_s_shift_still_rejected():
    # Guard: a negative homological-degree shift is still rejected.
    r = s2_rect(4)
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class("f", r, r, Bidegree.s_t(-1, 0), [1])


def test_from_class_guards():
    r = s2_rect(4)
    # Wrong class length (source has 1 generator at (0,0)).
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1, 1])
    # Class at an uncomputed bidegree.
    with pytest.raises(ValueError):
        ext.ResolutionHomomorphism.from_class(
            "id", r, r, Bidegree.s_t(100, 100), []
        )


def test_get_map_out_of_range_errors():
    r = s2_rect(4)
    hom = identity_hom(r, 4)
    with pytest.raises(IndexError):
        hom.get_map(-1)
    with pytest.raises(IndexError):
        hom.get_map(1000)


def test_extend_guards():
    r = s2_rect(4)
    hom = ext.ResolutionHomomorphism.from_class("id", r, r, Bidegree.s_t(0, 0), [1])
    # Negative target.
    with pytest.raises(ValueError):
        hom.extend(Bidegree.s_t(-1, 0))
    # Beyond the resolved range (source only computed through (4, 4)).
    with pytest.raises(ValueError):
        hom.extend(Bidegree.s_t(100, 100))
    with pytest.raises(ValueError):
        hom.extend_through_stem(Bidegree.n_s(100, 4))


def test_extend_all_unresolved_errors():
    r = ext.Resolution("S_2", "standard")
    hom = ext.ResolutionHomomorphism("f", r, r, Bidegree.s_t(0, 0))
    with pytest.raises(ValueError):
        hom.extend_all()


# --- DoubleChainComplex target (the sq0 use case) -------------------------


def test_double_target_sq0_action():
    """A ResolutionHomomorphism into a DoubleChainComplex realises Sq^0 on
    Ext(S_2). Mirrors examples/sq0.py: seed the (0,0) step with [1], extend by
    exactness, and read the action off get_map(s).output(...)."""
    r = s2_rect(8)
    doubled = ext.DoubleChainComplex(r)
    doubled.compute_through_bidegree(
        Bidegree.s_t(r.next_homological_degree - 1, 0)
    )

    hom = ext.ResolutionHomomorphism("Sq^0", r, doubled, Bidegree.zero())
    # target() hands back the DoubleChainComplex (shared Arc), not a Resolution.
    assert isinstance(hom.target, ext.DoubleChainComplex)
    assert hom.name == "Sq^0"
    assert hom.prime == 2

    hom.extend_step_raw(Bidegree.zero(), [FpVector.from_slice(r.prime, [1])])
    hom.extend_all()

    # Sq^0 fixes the generators on the 0- and 1-lines of Ext(S_2): for a
    # generator x_(n, s) the action lands on the doubled bidegree (s, 2t).
    results = {}
    for b in r.iter_nonzero_stem():
        doubled_b = Bidegree.s_t(b.s, 2 * b.t)
        if not r.has_computed_bidegree(doubled_b):
            continue
        source_num_gens = r.number_of_gens_in_bidegree(doubled_b)
        module = r.module(b.s)
        offset = module.generator_offset(b.t, b.t, 0)
        m = hom.get_map(b.s)
        for i in range(r.number_of_gens_in_bidegree(b)):
            g = BidegreeGenerator(b, i)
            row = [m.output(doubled_b.t, j).entry(offset + i) for j in range(source_num_gens)]
            results[(b.n, b.s, i)] = row

    # Known values (cf. ext/examples/sq0.rs): Sq^0 is the identity on the
    # bottom cells and the h-towers it reaches in this range.
    assert results[(0, 0, 0)] == [1]
    assert results[(1, 1, 0)] == [1]
    assert results[(3, 1, 0)] == [1]


def test_double_target_extend_step_only_for_resolution():
    """extend_step (the augmentation-lifting variant) needs an AugmentedChainComplex
    target, so it is rejected for a DoubleChainComplex target."""
    r = s2_rect(4)
    doubled = ext.DoubleChainComplex(r)
    doubled.compute_through_bidegree(Bidegree.s_t(r.next_homological_degree - 1, 0))
    hom = ext.ResolutionHomomorphism("Sq^0", r, doubled, Bidegree.zero())
    with pytest.raises(ValueError):
        hom.extend_step(Bidegree.zero(), None)


def test_target_must_be_resolution_or_double():
    r = s2_rect(2)
    with pytest.raises(TypeError):
        ext.ResolutionHomomorphism("f", r, 42, Bidegree.zero())
