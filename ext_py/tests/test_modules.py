import pytest

from ext import algebra, fp


# The C2 module: a generator x0 in degree 0 and x1 in degree 1 with Sq1 x0 = x1.
C2_JSON = {
    "p": 2,
    "type": "finite dimensional module",
    "gens": {"x0": 0, "x1": 1},
    "actions": ["Sq1 x0 = x1"],
}


def milnor(p=2):
    return algebra.SteenrodAlgebra.milnor(p)


def make_c2_fdmodule():
    """Build the C2 module by hand as an FDModule and set its single action."""
    m = algebra.FDModule(milnor(2), "C2", [1, 1])
    # Sq1 is the algebra operation (degree 1, index 0).
    m.set_action(1, 0, 0, 0, [1])
    return m


# --- FDModule -------------------------------------------------------------


def test_fdmodule_basic_invariants():
    m = make_c2_fdmodule()
    assert isinstance(m.prime(), int)
    assert m.prime() == 2
    assert m.min_degree() == 0
    assert m.dimension(0) == 1
    assert m.dimension(1) == 1
    assert m.dimension(2) == 0
    assert m.dimension(-1) == 0
    assert m.max_degree() == 1
    assert m.total_dimension() == 2


def test_fdmodule_act_on_basis_known_value():
    m = make_c2_fdmodule()
    # Sq1 . x0 = x1: result lands in degree 1, which has dimension 1.
    res = fp.FpVector(2, m.dimension(1))
    m.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1
    assert sum(res) == 1


def test_fdmodule_act_with_element_input():
    m = make_c2_fdmodule()
    # act with an input vector equal to x0 (degree 0).
    inp = fp.FpVector(2, m.dimension(0))
    inp[0] = 1
    res = fp.FpVector(2, m.dimension(1))
    m.act(res, 1, 1, 0, 0, inp)
    assert res[0] == 1


def test_fdmodule_act_by_element():
    m = make_c2_fdmodule()
    # op = Sq1 as an algebra element in degree 1 (dimension 1).
    op = fp.FpVector(2, 1)
    op[0] = 1
    inp = fp.FpVector(2, m.dimension(0))
    inp[0] = 1
    res = fp.FpVector(2, m.dimension(1))
    m.act_by_element(res, 1, 1, op, 0, inp)
    assert res[0] == 1


def test_fdmodule_action_getter_and_string():
    m = make_c2_fdmodule()
    assert list(m.action(1, 0, 0, 0)) == [1]
    # FDModule auto-names basis elements `x{degree}_{index}`.
    assert m.basis_element_to_string(0, 0) == "x0_0"
    assert m.string_to_basis_element("x1_0") == (1, 0)
    assert m.string_to_basis_element("nope") is None


def test_fdmodule_set_action_invalid_raises():
    m = algebra.FDModule(milnor(2), "C2", [1, 1])
    # Output degree 2 is empty -> length mismatch raises (not panic).
    with pytest.raises((ValueError, IndexError)):
        m.set_action(2, 0, 0, 0, [1])
    # Out-of-range module index raises.
    with pytest.raises((ValueError, IndexError)):
        m.set_action(1, 0, 0, 9, [1])


def test_fdmodule_act_out_of_range_raises():
    m = make_c2_fdmodule()
    res = fp.FpVector(2, m.dimension(1))
    # Module index out of range.
    with pytest.raises((ValueError, IndexError)):
        m.act_on_basis(res, 1, 1, 0, 0, 9)
    # Negative operation degree.
    with pytest.raises((ValueError, IndexError)):
        m.act_on_basis(res, 1, -1, 0, 0, 0)


def test_fdmodule_into_steenrod_module():
    m = make_c2_fdmodule()
    sm = m.into_steenrod_module()
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime() == m.prime()
    assert sm.dimension(0) == m.dimension(0)
    assert sm.dimension(1) == m.dimension(1)
    # The algebra accessor returns a SteenrodAlgebra at the same prime.
    assert sm.algebra().prime() == 2


# --- steenrod_module_from_json --------------------------------------------


def test_steenrod_module_from_json_c2():
    sm = algebra.steenrod_module_from_json(milnor(2), C2_JSON)
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime() == 2
    assert sm.min_degree() == 0
    assert sm.dimension(0) == 1
    assert sm.dimension(1) == 1
    assert sm.dimension(2) == 0
    assert sm.basis_element_to_string(0, 0) == "x0"


def test_steenrod_module_from_json_action_known_value():
    sm = algebra.steenrod_module_from_json(milnor(2), C2_JSON)
    # Sq1 . x0 = x1.
    res = fp.FpVector(2, sm.dimension(1))
    sm.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1


def test_steenrod_module_from_json_bad_spec_raises():
    with pytest.raises(ValueError):
        algebra.steenrod_module_from_json(milnor(2), {"p": 2})  # missing type
    with pytest.raises(ValueError):
        algebra.steenrod_module_from_json(milnor(2), {"p": 2, "type": "bogus"})


# --- FreeModule -----------------------------------------------------------


def make_free(gen_degrees=(0,)):
    # `FreeModule` is query-only (no Python mutators). Obtain a populated
    # FreeModule via the remaining path: build an FPModule whose generators
    # live in the requested degrees, then take its `generators()` FreeModule.
    b = algebra.FPModuleBuilder(milnor(2), "F", 0)
    for d in gen_degrees:
        b.add_generators(d, [f"x{d}"])
    m = b.build().generators()
    m.compute_basis(6)
    return m


def test_freemodule_basic_invariants():
    m = make_free()
    assert m.prime() == 2
    assert m.min_degree() == 0
    assert m.number_of_gens_in_degree(0) == 1
    # dimension(t) tracks the algebra dimension for a single degree-0 generator.
    assert m.dimension(0) == 1
    assert m.dimension(1) == 1
    assert m.dimension(2) == 1
    assert m.dimension(3) == 2


def test_freemodule_index_to_op_gen():
    m = make_free()
    opgen = m.index_to_op_gen(1, 0)
    assert isinstance(opgen, algebra.OperationGeneratorPair)
    assert opgen.generator_degree == 0
    assert opgen.generator_index == 0
    assert opgen.operation_degree == 1
    assert opgen.operation_index == 0


def test_freemodule_index_to_op_gen_out_of_range():
    m = make_free()
    with pytest.raises((ValueError, IndexError)):
        m.index_to_op_gen(1, 9)


def test_freemodule_operation_generator_to_index():
    m = make_free()
    # Sq1 (op degree 1, index 0) applied to gen (0,0) lands at index 0 in deg 1.
    idx = m.operation_generator_to_index(1, 0, 0, 0)
    assert idx == 0


def test_freemodule_offsets_and_iter_gens():
    m = make_free()
    assert m.generator_offset(1, 0, 0) == 0
    assert m.iter_gens(3) == [(0, 0)]
    names = m.gen_names()
    assert len(names) >= 1


def test_freemodule_act_on_basis():
    m = make_free()
    # Sq1 . (gen in degree 0) = the Sq1-generator basis element in degree 1.
    res = fp.FpVector(2, m.dimension(1))
    m.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1


def test_freemodule_into_steenrod_module():
    m = make_free()
    sm = m.into_steenrod_module()
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime() == m.prime()
    assert sm.dimension(1) == m.dimension(1)


def test_freemodule_total_dimension_unbounded_raises():
    m = make_free()
    # FreeModule is unbounded above -> total_dimension raises, never panics.
    with pytest.raises(ValueError):
        m.total_dimension()


def test_freemodule_number_of_gens_in_degree_above_range_returns_zero():
    # Fresh module: no generators added anywhere yet -> 0, never panics.
    fresh = algebra.FreeModule(milnor(2), "F", 0)
    assert fresh.number_of_gens_in_degree(0) == 0
    assert fresh.number_of_gens_in_degree(5) == 0
    assert fresh.number_of_gens_in_degree(-1) == 0
    # After adding degree-0 generators, degrees above the populated range
    # still read 0 rather than panicking.
    m = make_free()
    assert m.number_of_gens_in_degree(0) == 1
    assert m.number_of_gens_in_degree(1) == 0
    assert m.number_of_gens_in_degree(99) == 0


def test_freemodule_generator_offset_out_of_range_raises():
    m = make_free()
    # gen_degree above the populated range raises IndexError, not panic.
    with pytest.raises(IndexError):
        m.generator_offset(4, 3, 0)
    # gen_index out of range in a populated degree raises IndexError.
    with pytest.raises(IndexError):
        m.generator_offset(1, 0, 5)


def test_freemodule_operation_generator_to_index_out_of_range_raises():
    m = make_free()
    with pytest.raises(IndexError):
        m.operation_generator_to_index(0, 0, 3, 0)
    with pytest.raises(IndexError):
        m.operation_generator_to_index(0, 0, 0, 5)


def test_freemodule_has_no_python_mutators():
    # FreeModule is query-only: the consecutiveness guard that used to live on
    # FreeModule.add_generators (former test_freemodule_add_generators_
    # consecutive_only) now lives on FPModuleBuilder.add_generators, tested in
    # test_fp_module.py. Confirm the mutators are gone here.
    m = algebra.FreeModule(milnor(2), "F", 0)
    assert not hasattr(m, "add_generators")
    assert not hasattr(m, "extend_by_zero")


def test_freemodule_iter_gens_below_min_degree_empty():
    m = make_free(gen_degrees=(0, 1))
    # Below min_degree must be empty, not "all generators".
    assert m.iter_gens(-1) == []
    assert len(m.iter_gens(1)) == 2


def test_fdmodule_set_action_out_of_range_output_degree_raises():
    m = algebra.FDModule(milnor(2), "C2", [1, 1])
    # op_degree 5 lands in output degree 5 (above max_degree 1); an empty
    # output used to slip past the length check and panic.
    with pytest.raises(ValueError):
        m.set_action(5, 0, 0, 0, [])
    # Valid set_action still works.
    m.set_action(1, 0, 0, 0, [1])
    assert list(m.action(1, 0, 0, 0)) == [1]


def test_fdmodule_into_steenrod_module_is_snapshot():
    m = algebra.FDModule(milnor(2), "C2", [1, 1])
    sm = m.into_steenrod_module()
    # Mutating the FDModule after boxing does not affect the boxed snapshot.
    m.set_action(1, 0, 0, 0, [1])
    res = fp.FpVector(2, sm.dimension(1))
    sm.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 0


# --- invalid construction -------------------------------------------------


def test_module_from_json_prime_mismatch():
    # Passing a p=3 algebra to a p=2 spec must error, not panic.
    with pytest.raises((ValueError, RuntimeError)):
        algebra.steenrod_module_from_json(milnor(3), C2_JSON)
