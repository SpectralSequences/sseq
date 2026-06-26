"""Tests for the `ChainComplex` pyclass (`CCC = FiniteChainComplex<SteenrodModule>`).

A `ChainComplex` is obtained either from a single module via
`ChainComplex.ccdz(module)` (the one-term "concentrated, zero differential"
complex) or from a standard `Resolution` via `resolution.chain_complex()` (the
complex the resolution resolves, sharing the same `Arc`).

Only the `ChainComplex` trait surface is bound on the pyclass. The
`FreeChainComplex` methods (`to_sseq`, `graded_dimension_string`,
`number_of_gens_in_bidegree`, ...) are *not* implemented for `CCC` (its modules
are arbitrary `SteenrodModule`s, not free modules); `to_sseq` is exposed on
`Resolution` instead, whose modules are free.

`iter_stem` is exposed faithfully as a lazy iterator. For a `FiniteChainComplex`
it is *infinite* (every homological degree past the top resolves to the zero
module, and `FDModule.max_computed_degree` is unbounded), so it is only ever
sliced here with `itertools.islice`, never materialised with `list()`.
"""

import itertools

import pytest

import ext
from ext import algebra, sseq


def c2_module():
    """The C2 module (cells in degrees 0, 1; `Sq1 x0 = x1`) as a SteenrodModule."""
    alg = algebra.SteenrodAlgebra.milnor(2)
    b = algebra.FDModuleBuilder(alg, "C2", [1, 1], 0)
    b.set_action(1, 0, 0, 0, [1])
    return b.build()


def point(alg, min_degree=0):
    """A single-generator FDModule in `min_degree` over `alg`."""
    return algebra.FDModuleBuilder(alg, "pt", [1], min_degree).build()


def ccdz_c2():
    cc = ext.ChainComplex.ccdz(c2_module())
    cc.compute_through_bidegree(sseq.Bidegree.s_t(0, 1))
    return cc


# --- construction + basic invariants ---------------------------------------


def test_ccdz_basic_invariants():
    cc = ccdz_c2()
    assert cc.prime == 2
    assert cc.min_degree() == 0
    assert cc.algebra().prime == 2
    # next_homological_degree is i32::MAX for a FiniteChainComplex.
    assert cc.next_homological_degree() == 2147483647
    assert cc.save_dir() is None


def test_ccdz_modules_and_differential():
    cc = ccdz_c2()
    # C_0 is the module itself.
    m0 = cc.module(0)
    assert isinstance(m0, algebra.SteenrodModule)
    assert m0.dimension(0) == 1
    assert m0.dimension(1) == 1
    assert m0.dimension(2) == 0
    # C_s = 0 for s >= 1.
    assert cc.module(1).dimension(0) == 0
    assert cc.zero_module().dimension(0) == 0
    # differential is the (zero) boundary; shares the algebra.
    d0 = cc.differential(0)
    assert isinstance(d0, algebra.FullModuleHomomorphism)
    assert d0.prime == 2


def test_ccdz_has_computed_bidegree():
    cc = ccdz_c2()
    assert cc.has_computed_bidegree(sseq.Bidegree.s_t(0, 0)) is True


def test_has_computed_bidegree_negative_raises_valueerror():
    # Pre-fix a negative s wrapped to a huge usize and returned a bool.
    cc = ccdz_c2()
    with pytest.raises(ValueError):
        cc.has_computed_bidegree(sseq.Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        cc.has_computed_bidegree(sseq.Bidegree.s_t(0, -1))


# --- ChainComplex.new input validation -------------------------------------


def test_new_valid_two_term_complex():
    # C_0 <- C_1 with the (zero) differential C_1 -> C_0, all over one algebra.
    alg = algebra.SteenrodAlgebra.milnor(2)
    m0 = point(alg, 0)
    m1 = point(alg, 1)
    d = algebra.FullModuleHomomorphism(m1, m0)  # C_1 -> C_0
    cc = ext.ChainComplex.new([m0, m1], [d])
    assert cc.prime == 2
    assert cc.module(0).dimension(0) == 1
    assert cc.module(1).dimension(1) == 1
    # differential(1): C_1 -> C_0 is defined.
    assert cc.differential(1).prime == 2


def test_new_single_module_no_differentials():
    # The ccdz case: one module, zero differentials, is valid.
    alg = algebra.SteenrodAlgebra.milnor(2)
    cc = ext.ChainComplex.new([point(alg)], [])
    assert cc.prime == 2
    assert cc.module(0).dimension(0) == 1


def test_new_mismatched_lengths_raises_valueerror():
    # Pre-fix this was silently accepted (2 modules, 0 differentials).
    alg = algebra.SteenrodAlgebra.milnor(2)
    with pytest.raises(ValueError):
        ext.ChainComplex.new([point(alg, 0), point(alg, 1)], [])


def test_new_mixed_prime_raises_valueerror():
    # Pre-fix this was silently accepted: prime()==2 but module(1) over p=3.
    a2 = algebra.SteenrodAlgebra.milnor(2)
    a3 = algebra.SteenrodAlgebra.milnor(3)
    with pytest.raises(ValueError):
        ext.ChainComplex.new([point(a2), point(a3)], [])


def test_new_mixed_algebra_raises_valueerror():
    # Same prime but two distinct algebra objects -> incoherent.
    a2a = algebra.SteenrodAlgebra.milnor(2)
    a2b = algebra.SteenrodAlgebra.milnor(2)
    with pytest.raises(ValueError):
        ext.ChainComplex.new([point(a2a), point(a2b)], [])


def test_new_empty_modules_raises_valueerror():
    with pytest.raises(ValueError):
        ext.ChainComplex.new([], [])


# --- out-of-range s is safe (zero module / zero differential) --------------


def test_module_large_s_is_zero_module():
    cc = ccdz_c2()
    m = cc.module(1000)
    assert m.dimension(0) == 0


def test_differential_large_s_is_valid():
    cc = ccdz_c2()
    d = cc.differential(1000)
    assert isinstance(d, algebra.FullModuleHomomorphism)
    assert d.prime == 2


# --- iter_stem (lazy, infinite for a FiniteChainComplex) -------------------


def test_iter_stem_first_bidegrees():
    cc = ccdz_c2()
    first = list(itertools.islice(cc.iter_stem(), 5))
    assert [(b.s, b.t) for b in first] == [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]


def test_iter_stem_is_iterator_protocol():
    cc = ccdz_c2()
    it = cc.iter_stem()
    assert iter(it) is it
    b = next(it)
    assert (b.s, b.t) == (0, 0)


# --- guards: no panics across the FFI boundary -----------------------------


def test_negative_bidegree_compute_raises_valueerror():
    cc = ext.ChainComplex.ccdz(c2_module())
    with pytest.raises(ValueError):
        cc.compute_through_bidegree(sseq.Bidegree.s_t(-1, 0))
    with pytest.raises(ValueError):
        cc.compute_through_bidegree(sseq.Bidegree.s_t(0, -1))


def test_negative_homological_degree_raises_valueerror():
    cc = ccdz_c2()
    with pytest.raises(ValueError):
        cc.module(-1)
    with pytest.raises(ValueError):
        cc.differential(-1)


# --- pop --------------------------------------------------------------------


def test_pop_sole_owner_succeeds():
    cc = ccdz_c2()
    cc.pop()
    # After popping the only module, C_0 becomes the zero module.
    assert cc.module(0).dimension(0) == 0


def test_pop_shared_complex_raises_runtimeerror():
    # A complex obtained from a Resolution shares its Arc, so it cannot be popped.
    r = ext.Resolution("S_2", "standard")
    r.compute_through_stem(sseq.Bidegree.n_s(4, 2))
    cc = r.chain_complex()
    with pytest.raises(RuntimeError):
        cc.pop()


def test_live_stem_iterator_blocks_pop():
    # A live StemIterator holds a shared handle, so pop fails until it is dropped.
    cc = ccdz_c2()
    it = cc.iter_stem()
    next(it)  # keep it alive
    with pytest.raises(RuntimeError):
        cc.pop()
    # Dropping the iterator releases the shared handle; pop then succeeds.
    del it
    cc.pop()
    assert cc.module(0).dimension(0) == 0


# --- Resolution accessors (FreeChainComplex surface lives here) ------------


def test_resolution_chain_complex_accessor():
    r = ext.Resolution("S_2", "standard")
    r.compute_through_stem(sseq.Bidegree.n_s(8, 4))
    cc = r.chain_complex()
    assert isinstance(cc, ext.ChainComplex)
    assert cc.prime == 2
    assert cc.min_degree() == 0
    # The complex resolved is the sphere: C_0 is the unit module (dim 1 in deg 0).
    assert cc.module(0).dimension(0) == 1


def test_resolution_chain_complex_nassau_raises_valueerror():
    r = ext.Resolution("S_2", "nassau")
    with pytest.raises(ValueError):
        r.chain_complex()


def test_resolution_to_sseq_returns_sseq():
    r = ext.Resolution("S_2", "standard")
    r.compute_through_stem(sseq.Bidegree.n_s(8, 4))
    ss = r.to_sseq()
    assert isinstance(ss, sseq.Sseq)
    assert ss.prime == 2


def test_resolution_nassau_to_sseq_returns_sseq():
    r = ext.Resolution("S_2", "nassau")
    r.compute_through_stem(sseq.Bidegree.n_s(4, 2))
    ss = r.to_sseq()
    assert isinstance(ss, sseq.Sseq)
    assert ss.prime == 2


def test_chain_complex_has_no_free_chain_complex_methods():
    # to_sseq / graded_dimension_string / number_of_gens_in_bidegree are
    # FreeChainComplex methods and are not implemented for CCC.
    cc = ccdz_c2()
    assert not hasattr(cc, "to_sseq")
    assert not hasattr(cc, "graded_dimension_string")
    assert not hasattr(cc, "number_of_gens_in_bidegree")


# --- FiniteAugmentedChainComplex -------------------------------------------
#
# An augmented finite chain complex C -> D: a finite complex C plus an
# augmentation chain map (one per module, chain_maps[s]: C_s -> D_s) to a target
# complex D. Constructed here directly from explicit modules/differentials, a
# target ChainComplex, and the augmentation maps. The C-side validation mirrors
# ChainComplex.new; additionally chain_maps must have exactly one map per module
# and share the complex's prime+algebra.


def test_facc_valid_one_module():
    alg = algebra.SteenrodAlgebra.milnor(2)
    c0 = point(alg, 0)
    d0 = point(alg, 0)
    target = ext.ChainComplex.ccdz(d0)
    aug = algebra.FullModuleHomomorphism(c0, d0)  # C_0 -> D_0
    facc = ext.FiniteAugmentedChainComplex([c0], [], target, [aug])
    assert facc.prime == 2
    assert facc.min_degree() == 0
    assert facc.algebra().prime == 2
    assert facc.max_s() == 1
    assert facc.module(0).dimension(0) == 1
    # target() shares the Arc and reports the same prime.
    assert isinstance(facc.target(), ext.ChainComplex)
    assert facc.target().prime == 2
    # chain_map(0) is the augmentation.
    assert isinstance(facc.chain_map(0), algebra.FullModuleHomomorphism)
    assert facc.chain_map(0).prime == 2


def test_facc_valid_two_module():
    alg = algebra.SteenrodAlgebra.milnor(2)
    c0, c1 = point(alg, 0), point(alg, 1)
    d0, d1 = point(alg, 0), point(alg, 1)
    diff = algebra.FullModuleHomomorphism(c1, c0)  # C_1 -> C_0
    target = ext.ChainComplex.new([d0, d1], [algebra.FullModuleHomomorphism(d1, d0)])
    aug0 = algebra.FullModuleHomomorphism(c0, d0)
    aug1 = algebra.FullModuleHomomorphism(c1, d1)
    facc = ext.FiniteAugmentedChainComplex([c0, c1], [diff], target, [aug0, aug1])
    assert facc.max_s() == 2
    assert facc.module(1).dimension(1) == 1
    assert facc.differential(1).prime == 2
    assert facc.chain_map(1).prime == 2


def test_facc_chain_map_out_of_range_raises_index_error():
    alg = algebra.SteenrodAlgebra.milnor(2)
    c0, d0 = point(alg, 0), point(alg, 0)
    target = ext.ChainComplex.ccdz(d0)
    aug = algebra.FullModuleHomomorphism(c0, d0)
    facc = ext.FiniteAugmentedChainComplex([c0], [], target, [aug])
    with pytest.raises(IndexError):
        facc.chain_map(1)
    with pytest.raises(IndexError):
        facc.chain_map(-1)


def test_facc_module_large_s_is_zero_and_negative_raises():
    alg = algebra.SteenrodAlgebra.milnor(2)
    c0, d0 = point(alg, 0), point(alg, 0)
    target = ext.ChainComplex.ccdz(d0)
    facc = ext.FiniteAugmentedChainComplex(
        [c0], [], target, [algebra.FullModuleHomomorphism(c0, d0)]
    )
    # Out-of-range s: zero module, no panic.
    assert facc.module(1000).dimension(0) == 0
    # Negative s: ValueError.
    with pytest.raises(ValueError):
        facc.module(-1)
    with pytest.raises(ValueError):
        facc.differential(-1)


def test_facc_empty_modules_raises_value_error():
    alg = algebra.SteenrodAlgebra.milnor(2)
    target = ext.ChainComplex.ccdz(point(alg, 0))
    with pytest.raises(ValueError):
        ext.FiniteAugmentedChainComplex([], [], target, [])


def test_facc_mismatched_differential_count_raises_value_error():
    alg = algebra.SteenrodAlgebra.milnor(2)
    c0, c1 = point(alg, 0), point(alg, 1)
    d0 = point(alg, 0)
    target = ext.ChainComplex.ccdz(d0)
    # Two modules need exactly one differential; supplying none fails.
    aug0 = algebra.FullModuleHomomorphism(c0, d0)
    aug1 = algebra.FullModuleHomomorphism(c1, d0)
    with pytest.raises(ValueError):
        ext.FiniteAugmentedChainComplex([c0, c1], [], target, [aug0, aug1])


def test_facc_mismatched_chain_map_count_raises_value_error():
    alg = algebra.SteenrodAlgebra.milnor(2)
    c0 = point(alg, 0)
    d0 = point(alg, 0)
    target = ext.ChainComplex.ccdz(d0)
    # One module needs exactly one chain map; supplying two fails.
    aug = algebra.FullModuleHomomorphism(c0, d0)
    with pytest.raises(ValueError):
        ext.FiniteAugmentedChainComplex([c0], [], target, [aug, aug])


def test_facc_mixed_prime_raises_value_error():
    a2 = algebra.SteenrodAlgebra.milnor(2)
    a3 = algebra.SteenrodAlgebra.milnor(3)
    c0 = point(a3, 0)
    d0 = point(a2, 0)
    target = ext.ChainComplex.ccdz(d0)
    aug = algebra.FullModuleHomomorphism(c0, c0)
    with pytest.raises(ValueError):
        ext.FiniteAugmentedChainComplex([c0], [], target, [aug])


def test_facc_mixed_algebra_raises_value_error():
    # Same prime but the target complex is over a *distinct* algebra object than
    # the modules -> incoherent. (A homomorphism cannot itself mix algebras, so
    # the reachable incoherence is module-algebra vs target-algebra.)
    a2a = algebra.SteenrodAlgebra.milnor(2)
    a2b = algebra.SteenrodAlgebra.milnor(2)
    c0 = point(a2a, 0)
    target = ext.ChainComplex.ccdz(point(a2b, 0))  # over a2b
    aug = algebra.FullModuleHomomorphism(c0, c0)  # over a2a
    with pytest.raises(ValueError):
        ext.FiniteAugmentedChainComplex([c0], [], target, [aug])
