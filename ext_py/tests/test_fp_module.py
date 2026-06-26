import pytest

from ext import algebra, fp


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def a_mod_sq1(alg):
    """A/(Sq1) on one generator x0: x0 in degree 0, relation Sq1 x0 in degree 1."""
    b = algebra.FPModuleBuilder(alg, "A/(Sq1)", 0)
    b.add_generators(0, ["x0"])
    # Relations must start at min_degree; degree 0 has none.
    b.add_relations(0, [])
    # Sq1 x0 lives in degree 1 where the generators are 1-dimensional.
    v = fp.FpVector(2, 1)
    v.set_entry(0, 1)
    b.add_relations(1, [v])
    return b.build()


# --- FPModule construction / invariants -----------------------------------


def test_fp_module_construct_and_dimensions():
    alg = milnor(2)
    m = a_mod_sq1(alg)
    assert isinstance(m.prime, int)
    assert m.prime == 2
    assert m.min_degree() == 0
    # x0 survives, Sq1 x0 killed, Sq2 x0 survives.
    assert m.dimension(0) == 1
    assert m.dimension(1) == 0
    assert m.dimension(2) == 1
    assert m.dimension(-1) == 0
    assert repr(m).startswith("FPModule(")


def test_fp_module_generators_is_free_module():
    alg = milnor(2)
    m = a_mod_sq1(alg)
    gens = m.generators()
    assert isinstance(gens, algebra.FreeModule)
    assert gens.number_of_gens_in_degree(0) == 1
    assert gens.dimension(1) == 1


def test_fp_module_gen_fp_idx_round_trip():
    alg = milnor(2)
    m = a_mod_sq1(alg)
    # Degree 0: generator 0 survives -> fp index 0, round trip.
    assert m.gen_idx_to_fp_idx(0, 0) == 0
    assert m.fp_idx_to_gen_idx(0, 0) == 0
    # Degree 1: the generator is killed -> -1, and no fp basis element.
    assert m.gen_idx_to_fp_idx(1, 0) == -1
    with pytest.raises(IndexError):
        m.fp_idx_to_gen_idx(1, 0)
    # Degree 2: generator 0 survives -> fp index 0, round trip.
    assert m.gen_idx_to_fp_idx(2, 0) == 0
    assert m.fp_idx_to_gen_idx(2, 0) == 0


def test_fp_module_into_steenrod_module_round_trip():
    alg = milnor(2)
    m = a_mod_sq1(alg)
    boxed = m.into_steenrod_module()
    assert isinstance(boxed, algebra.SteenrodModule)
    assert boxed.prime == m.prime
    assert boxed.dimension(0) == m.dimension(0)
    assert boxed.dimension(2) == m.dimension(2)


def test_fp_module_is_immutable():
    # The built FPModule is immutable: it exposes no mutators at all.
    alg = milnor(2)
    m = a_mod_sq1(alg)
    assert not hasattr(m, "add_generators")
    assert not hasattr(m, "add_relations")
    # And it cannot be constructed directly from Python (no __new__).
    with pytest.raises(TypeError):
        algebra.FPModule(alg, "M", 0)


# --- FreeModule is query-only ----------------------------------------------


def test_free_module_has_no_mutators():
    alg = milnor(2)
    f = algebra.FreeModule(alg, "F", 0)
    assert not hasattr(f, "add_generators")
    assert not hasattr(f, "extend_by_zero")
    assert not hasattr(algebra.FreeModule, "add_generators")
    assert not hasattr(algebra.FreeModule, "extend_by_zero")


# --- FPModuleBuilder -------------------------------------------------------


def test_fp_module_builder_build_and_mutation_after_build_raises():
    alg = milnor(2)
    b = algebra.FPModuleBuilder(alg, "M", 0)
    assert b.prime == 2
    assert b.min_degree() == 0
    b.add_generators(0, ["x0"])
    m = b.build()
    assert isinstance(m, algebra.FPModule)
    # After build(), mutating the builder raises RuntimeError (never panics).
    with pytest.raises(RuntimeError):
        b.add_generators(1, ["y"])
    with pytest.raises(RuntimeError):
        b.add_relations(0, [])


# --- invalid inputs --------------------------------------------------------


def test_fp_module_bad_prime_raises():
    with pytest.raises(ValueError):
        algebra.SteenrodAlgebra.milnor(4)


def test_fp_module_builder_add_generators_non_consecutive_raises():
    alg = milnor(2)
    b = algebra.FPModuleBuilder(alg, "M", 0)
    # First expected degree is 0; skipping to 2 raises rather than panics.
    with pytest.raises(ValueError):
        b.add_generators(2, ["x"])
    b.add_generators(0, ["x0"])
    # Re-adding degree 0 raises.
    with pytest.raises(ValueError):
        b.add_generators(0, ["x0b"])
    # Below min_degree raises.
    with pytest.raises(ValueError):
        b.add_generators(-1, [])


def test_fp_module_builder_add_relations_bad_input_raises():
    alg = milnor(2)
    b = algebra.FPModuleBuilder(alg, "M", 0)
    b.add_generators(0, ["x0"])
    # Relations must start at min_degree 0; degree 1 first raises.
    v = fp.FpVector(2, 1)
    with pytest.raises(ValueError):
        b.add_relations(1, [v])
    b.add_relations(0, [])
    # Wrong length in degree 1 (gen dim is 1) raises.
    bad_len = fp.FpVector(2, 3)
    with pytest.raises(ValueError):
        b.add_relations(1, [bad_len])
    # Wrong prime raises.
    bad_p = fp.FpVector(3, 1)
    with pytest.raises(ValueError):
        b.add_relations(1, [bad_p])


# --- from_json -------------------------------------------------------------


A_MOD_SQ1_SQ2 = {
    "p": 2,
    "type": "finitely presented module",
    "gens": {"x0": 0},
    "adem_relations": ["Sq1 x0", "Sq2 x0"],
    "milnor_relations": ["P(1) x0", "P(2) x0"],
}


def test_fp_module_from_json():
    alg = milnor(2)
    m = algebra.FPModule.from_json(alg, A_MOD_SQ1_SQ2)
    assert isinstance(m, algebra.FPModule)
    assert m.prime == 2
    assert m.min_degree() == 0
    assert m.dimension(0) == 1
    # Both Sq1 x0 and Sq2 x0 are killed.
    assert m.dimension(1) == 0
    assert m.dimension(2) == 0
    # The result is immutable: the prior HIGH desync path (calling
    # add_relations on a from_json result) is gone by construction.
    assert not hasattr(m, "add_relations")
    assert not hasattr(m, "add_generators")
    # Round-trips into a SteenrodModule.
    boxed = m.into_steenrod_module()
    assert boxed.dimension(0) == 1


def test_fp_module_from_json_prime_mismatch_raises():
    alg = milnor(3)
    with pytest.raises(ValueError):
        algebra.FPModule.from_json(alg, A_MOD_SQ1_SQ2)


# --- BlockStructure / GeneratorBasisEltPair --------------------------------


def test_block_structure_queries():
    # Degree 0 has blocks of size [2, 1]; degree 1 has block [3].
    bs = algebra.BlockStructure(0, [[2, 1], [3]])
    assert bs.total_dimension() == 6
    assert bs.generator_to_block(0, 0) == (0, 2)
    assert bs.generator_to_block(0, 1) == (2, 3)
    assert bs.generator_to_block(1, 0) == (3, 6)
    assert bs.generator_basis_elt_to_index(0, 1, 0) == 2
    assert bs.generator_basis_elt_to_index(1, 0, 2) == 5
    pair = bs.index_to_generator_basis_elt(5)
    assert isinstance(pair, algebra.GeneratorBasisEltPair)
    assert pair.generator_degree == 1
    assert pair.generator_index == 0
    assert pair.basis_index == 2


def test_block_structure_out_of_range_raises():
    bs = algebra.BlockStructure(0, [[2, 1], [3]])
    with pytest.raises(IndexError):
        bs.generator_to_block(2, 0)
    with pytest.raises(IndexError):
        bs.generator_to_block(0, 5)
    with pytest.raises(IndexError):
        bs.generator_basis_elt_to_index(0, 0, 9)
    with pytest.raises(IndexError):
        bs.index_to_generator_basis_elt(6)


def test_block_structure_add_block():
    bs = algebra.BlockStructure(0, [[2, 1], [3]])
    target = fp.FpVector(2, bs.total_dimension())
    source = fp.FpVector(2, 3)
    source.set_entry(0, 1)
    source.set_entry(2, 1)
    # Add into the block of generator (1, 0), which spans indices [3, 6).
    bs.add_block(target, 1, 1, 0, source)
    assert target.entry(3) == 1
    assert target.entry(5) == 1
    assert target.entry(0) == 0


def test_generator_basis_elt_pair_construct():
    p = algebra.GeneratorBasisEltPair(3, 1, 2)
    assert p.generator_degree == 3
    assert p.generator_index == 1
    assert p.basis_index == 2
    assert "GeneratorBasisEltPair" in repr(p)
