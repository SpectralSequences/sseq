"""Tests for `Resolution` (Nassau vs standard dispatch) and `SecondaryResolution`.

`Resolution(spec, algorithm)` selects an algorithm at runtime: ``None``/``"auto"``
(try Nassau, fall back to the general algorithm), ``"nassau"`` (force Nassau, error
if ineligible), or ``"standard"`` (force the general algorithm).

Expected values are not hardcoded: the resolution of ``S_2`` through a fixed small
range must be identical regardless of algorithm, so the auto/nassau results are
validated against the standard result via `graded_dimension_string()` agreement.

`SecondaryResolution` is only supported over the *standard* backend; a Nassau-backed
`Resolution` is rejected with a clean `ValueError` (Nassau stores quasi-inverses on
disk and needs a save directory, which the binding does not provide). See the
maintainer decision rejecting Nassau-backed secondary resolutions.

Ranges are kept small (`Bidegree.n_s(8, 4)`) so the suite stays fast (~0.05s total
on the dev machine).
"""

import pytest

import ext
from ext import sseq

# Small target bidegree: resolving S_2 through this stem is ~10ms per algorithm.
SMALL = sseq.Bidegree.n_s(8, 4)


def resolve(algorithm, max=SMALL):
    r = ext.Resolution("S_2", algorithm)
    r.compute_through_stem(max)
    return r


# --- algorithm dispatch ----------------------------------------------------


def test_auto_resolves_and_dimension_string_nonempty():
    s = resolve(None).graded_dimension_string()
    assert isinstance(s, str)
    assert len(s) > 0


def test_auto_equals_standard():
    # auto picks Nassau for the sphere; both algorithms resolve the same object, so
    # the graded dimensions over the same range must agree exactly.
    assert resolve("auto").graded_dimension_string() == resolve("standard").graded_dimension_string()


def test_nassau_equals_standard():
    assert resolve("nassau").graded_dimension_string() == resolve("standard").graded_dimension_string()


def test_standard_resolves():
    assert len(resolve("standard").graded_dimension_string()) > 0


def test_nassau_resolves():
    assert len(resolve("nassau").graded_dimension_string()) > 0


# --- error taxonomy --------------------------------------------------------


def test_bogus_algorithm_raises_valueerror():
    with pytest.raises(ValueError):
        ext.Resolution("S_2", "bogus")


def test_nassau_on_ineligible_spec_raises_valueerror():
    # Odd-prime sphere is ineligible for Nassau (requires p = 2). This is a
    # bad-argument condition, so it must be a clean ValueError, not a panic.
    with pytest.raises(ValueError):
        ext.Resolution("S_3", "nassau")


# --- compute_through_stem guard --------------------------------------------


def test_negative_s_bidegree_raises_valueerror():
    # Pre-fix this panicked across the FFI boundary (negative s over-allocates /
    # overflows in the resolve loop). The guard now raises ValueError first.
    r = ext.Resolution("S_2", "standard")
    with pytest.raises(ValueError):
        r.compute_through_stem(sseq.Bidegree.n_s(0, -1))


# --- SecondaryResolution ---------------------------------------------------


def test_secondary_over_standard_backend():
    r = resolve("standard")
    sec = ext.SecondaryResolution(r)
    sec.extend_all()
    assert isinstance(sec.underlying(), ext.Resolution)


def test_secondary_over_nassau_backend_raises_valueerror():
    # Nassau-backed secondary resolution is rejected up front (it would otherwise
    # panic in extend_all because Nassau quasi-inverses live on disk only).
    r = ext.Resolution("S_2", "nassau")
    with pytest.raises(ValueError):
        ext.SecondaryResolution(r)
