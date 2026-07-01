"""Tests for the `ChainHomotopy` pyclass (`ext::chain_complex::ChainHomotopy`).

A `ChainHomotopy` is built from two `ResolutionHomomorphism`s `left: S -> T` and
`right: T -> U` whose middle resolution is shared (the *same* Python `Resolution`
object passed to both, so the underlying `Arc`s are pointer-equal). It is the
primitive used to assemble (triple) Massey products — see `examples/massey.rs`,
which is the canonical construction this mirrors: two `from_class` lifts sharing
the unit resolution, extended over a stem, then `ChainHomotopy.extend`-ed.

Only the standard backend is reachable (the input `ResolutionHomomorphism`s are
standard-only). Every degree/index input is pre-checked and raises
`ValueError`/`IndexError` rather than panicking across the FFI boundary:
mismatched middle resolution, negative bidegrees, extending beyond the resolved
range or beyond the maps' extended range, and out-of-range `homotopy(s)`.

The `s`-th homotopy map `h_s` goes `C_s -> C_{s + 1 - shift.s}` and raises the
internal degree by `shift.t` (`degree_shift`). These structural invariants are
asserted (a concrete Massey-product *value* is not derivable without the full
multi-step workflow, which lives in Python `examples/`, not the binding layer).
"""

import pytest

import ext
from ext import sseq

Bidegree = sseq.Bidegree


def sphere(max_st=8):
    r = ext.Resolution("S_2", "standard")
    r.compute_through_bidegree(Bidegree.s_t(max_st, max_st))
    return r


def h0_mult(r, name, max_st=6):
    """Multiplication by h0 = (s=1, t=1) as a ResolutionHomomorphism r -> r,
    extended over the (max_st, max_st) rectangle."""
    hom = ext.ResolutionHomomorphism.from_class(name, r, r, Bidegree.s_t(1, 1), [1])
    hom.extend(Bidegree.s_t(max_st, max_st))
    return hom


def homotopy(max_st=8, ext_deg=6):
    r = sphere(max_st)
    left = h0_mult(r, "a", ext_deg)
    right = h0_mult(r, "b", ext_deg)
    return ext.ChainHomotopy(left, right)


# --- construction & accessors ----------------------------------------------


def test_construction_and_accessors():
    ch = homotopy()
    assert ch.prime == 2
    # shift = left.shift + right.shift = (1,1) + (1,1) = (2,2).
    assert ch.shift.s == 2
    assert ch.shift.t == 2
    # left()/right() share the underlying ResolutionHomomorphism Arcs.
    assert isinstance(ch.left, ext.ResolutionHomomorphism)
    assert isinstance(ch.right, ext.ResolutionHomomorphism)
    assert ch.left.name == "a"
    assert ch.right.name == "b"


def test_construction_requires_shared_middle():
    # left.target and right.source must be the SAME resolution object.
    r1 = sphere(4)
    r2 = sphere(4)
    left = ext.ResolutionHomomorphism.from_class("a", r1, r2, Bidegree.s_t(1, 1), [1])
    # right.source is r2's *handle*, but obtained from a different Python object.
    r3 = sphere(4)
    right = ext.ResolutionHomomorphism.from_class("b", r3, r2, Bidegree.s_t(1, 1), [1])
    with pytest.raises(ValueError):
        ext.ChainHomotopy(left, right)


# --- extend + homotopy shapes ----------------------------------------------


def test_extend_and_homotopy_shapes():
    ch = homotopy(max_st=8, ext_deg=6)
    ch.extend(Bidegree.s_t(4, 4))
    # The homotopy table starts at shift.s - 1 = 1; homotopy(0) is undefined.
    with pytest.raises(IndexError):
        ch.homotopy(0)
    shift = ch.shift  # (2, 2): shift.s == 2, shift.t == 2
    # Upstream `ChainHomotopy::initialize_homotopies` builds each h_s as
    # FreeModuleHomomorphism::new(left.source.module(s),
    #                             right.target.module(s + 1 - shift.s),
    #                             shift.t)
    # (ext/src/chain_complex/chain_homotopy.rs L122-130). So h_s is exactly the
    # map C_s -> C_{s + 1 - shift.s} with degree_shift == shift.t. Pin that down
    # concretely against the two input resolutions' modules.
    src_res = ch.left.source  # the resolution S (= C)
    tgt_res = ch.right.target  # the resolution U (= D)
    for s in range(1, 5):
        h = ch.homotopy(s)
        assert h.source.prime == 2
        assert h.target.prime == 2
        # degree_shift == shift.t (raises internal degree by 2).
        assert h.degree_shift == shift.t
        # h.source is left.source.module(s); h.target is
        # right.target.module(s + 1 - shift.s). Identify each module by its
        # generator/dimension profile over a range of internal degrees.
        expected_src = src_res.module(s)
        expected_tgt = tgt_res.module(s + 1 - shift.s)
        # Non-vacuous: source (C_s) and target (C_{s-1}) are genuinely different
        # modules (different homological degree), so this comparison pins down
        # the C_s -> C_{s + 1 - shift.s} relationship rather than trivially
        # matching the same module against itself.
        differ = False
        for t in range(0, 9):
            assert h.source.number_of_gens_in_degree(
                t
            ) == expected_src.number_of_gens_in_degree(t)
            assert h.source.dimension(t) == expected_src.dimension(t)
            assert h.target.number_of_gens_in_degree(
                t
            ) == expected_tgt.number_of_gens_in_degree(t)
            assert h.target.dimension(t) == expected_tgt.dimension(t)
            if expected_src.number_of_gens_in_degree(
                t
            ) != expected_tgt.number_of_gens_in_degree(t):
                differ = True
        assert differ, f"source/target modules must differ at s={s}"


def test_initialize_homotopies_allocates_table_without_lifting():
    # `initialize_homotopies(max_source_s)` allocates the homotopy table so that
    # `homotopy(s)` is defined for s in [shift.s - 1, max_source_s) WITHOUT
    # lifting any maps (the maps were never extended here). This is the setup a
    # secondary Massey product uses to install a non-zero bottom homotopy by
    # hand before extending.
    ch = homotopy(max_st=8, ext_deg=6)
    shift = ch.shift  # (2, 2)
    # Nothing allocated yet.
    with pytest.raises(IndexError):
        ch.homotopy(shift.s - 1)
    ch.initialize_homotopies(5)
    # Defined on [shift.s - 1, 5) == [1, 5); each h_s is C_s -> C_{s+1-shift.s}.
    for s in range(shift.s - 1, 5):
        h = ch.homotopy(s)
        assert h.degree_shift == shift.t
        assert h.source.prime == 2
    with pytest.raises(IndexError):
        ch.homotopy(5)
    # Idempotent / no-op when not growing the range.
    ch.initialize_homotopies(3)
    with pytest.raises(IndexError):
        ch.homotopy(5)


def test_initialize_homotopies_beyond_resolved_range_raises_value_error():
    # max_source_s reaching past the resolutions' computed homological degree
    # would make upstream index a not-yet-created module and panic; the binding
    # must reject it with a clean ValueError instead.
    ch = homotopy(max_st=8, ext_deg=6)
    # The resolutions are computed through s = 8 (next homological degree 9).
    with pytest.raises(ValueError):
        ch.initialize_homotopies(20)


def test_extend_all_succeeds_when_maps_fully_extended():
    r = sphere(5)
    left = h0_mult(r, "a", 5)
    right = h0_mult(r, "b", 5)
    ch = ext.ChainHomotopy(left, right)
    ch.extend_all()
    # The homotopy is now defined on [shift.s - 1, ...]; homotopy(1..) work.
    h = ch.homotopy(2)
    assert h.degree_shift == 2
    assert h.source.prime == 2


# --- guards: no panics across the FFI boundary -----------------------------


def test_extend_all_rejects_when_source_outpaces_target():
    # Massey-style zig-zag S -> T -> U sharing the middle resolution T, but with
    # the left source S resolved strictly further than the right target U. Then
    #   n_left  = S.next_homological_degree  (= 9)
    #   n_right = U.next_homological_degree  (= 3)
    #   shift.s = 2
    # so n_left >= n_right + shift.s, the config where upstream extend_all would
    # index right.target.module(n_right) and panic. The binding must reject this
    # with a clean ValueError, NOT panic across the FFI boundary.
    src = sphere(8)  # left.source S, resolved deep
    mid = sphere(8)  # shared middle T (left.target == right.source)
    tgt = sphere(2)  # right.target U, resolved shallow
    left = ext.ResolutionHomomorphism.from_class(
        "a", src, mid, Bidegree.s_t(1, 1), [1]
    )
    right = ext.ResolutionHomomorphism.from_class(
        "b", mid, tgt, Bidegree.s_t(1, 1), [1]
    )
    ch = ext.ChainHomotopy(left, right)
    with pytest.raises(ValueError):
        ch.extend_all()


def test_homotopy_out_of_range_raises_index_error():
    ch = homotopy()
    # Nothing extended yet -> the homotopy table is empty.
    with pytest.raises(IndexError):
        ch.homotopy(0)
    with pytest.raises(IndexError):
        ch.homotopy(2)


def test_extend_negative_bidegree_raises_value_error():
    ch = homotopy()
    with pytest.raises(ValueError):
        ch.extend(Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        ch.extend(Bidegree.s_t(0, -1))


def test_extend_unextended_maps_raises_value_error():
    # Build the homotopy from homs that were NOT extended; extend must reject
    # rather than panic when it would index an undefined chain map.
    r = sphere(6)
    left = ext.ResolutionHomomorphism.from_class("a", r, r, Bidegree.s_t(1, 1), [1])
    right = ext.ResolutionHomomorphism.from_class("b", r, r, Bidegree.s_t(1, 1), [1])
    ch = ext.ChainHomotopy(left, right)
    with pytest.raises(ValueError):
        ch.extend(Bidegree.s_t(4, 4))


def test_extend_beyond_resolved_range_raises_value_error():
    r = sphere(4)
    left = h0_mult(r, "a", 4)
    right = h0_mult(r, "b", 4)
    ch = ext.ChainHomotopy(left, right)
    with pytest.raises(ValueError):
        ch.extend(Bidegree.s_t(20, 20))
