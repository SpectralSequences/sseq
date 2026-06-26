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
    """Build the C2 module by hand via an FDModuleBuilder and set its action.

    Returns the (pre-build) builder; read-only query methods stay available on
    it for inspection during construction.
    """
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    # Sq1 is the algebra operation (degree 1, index 0).
    m.set_action(1, 0, 0, 0, [1])
    return m


# --- FDModuleBuilder ------------------------------------------------------


def test_fdmodule_basic_invariants():
    m = make_c2_fdmodule()
    assert isinstance(m.prime, int)
    assert m.prime == 2
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


def test_fdmodule_act_input_target_aliasing_raises_runtimeerror():
    # Sq1 . x0 = x1: input degree 0 (dim 1) and output degree 1 (dim 1) both
    # have length 1, so a single length-1 vector is shape-valid as both the
    # input and the mutable result. Aliasing them must raise RuntimeError
    # (borrow conflict), NOT the generic ValueError.
    m = make_c2_fdmodule()
    v = fp.FpVector(2, 1)
    v[0] = 1
    with pytest.raises(RuntimeError):
        m.act(v, 1, 1, 0, 0, v)
    with pytest.raises(Exception) as excinfo:
        m.act(v, 1, 1, 0, 0, v)
    assert not isinstance(excinfo.value, ValueError)

    # act_by_element with the result aliased as the input.
    op = fp.FpVector(2, 1)
    op[0] = 1
    with pytest.raises(RuntimeError):
        m.act_by_element(v, 1, 1, op, 0, v)


def test_fdmodule_act_wrong_type_is_valueerror():
    m = make_c2_fdmodule()
    inp = fp.FpVector(2, m.dimension(0))
    inp[0] = 1
    res = fp.FpVector(2, m.dimension(1))
    with pytest.raises(ValueError):
        m.act(123, 1, 1, 0, 0, inp)
    with pytest.raises(ValueError):
        m.act(res, 1, 1, 0, 0, 123)


def test_fdmodule_act_distinct_objects_regression():
    m = make_c2_fdmodule()
    inp = fp.FpVector(2, m.dimension(0))
    inp[0] = 1
    res = fp.FpVector(2, m.dimension(1))
    m.act(res, 1, 1, 0, 0, inp)
    assert res[0] == 1


def test_fdmodule_action_getter_and_string():
    m = make_c2_fdmodule()
    assert list(m.action(1, 0, 0, 0)) == [1]
    # FDModuleBuilder auto-names basis elements `x{degree}_{index}`.
    assert m.basis_element_to_string(0, 0) == "x0_0"
    assert m.string_to_basis_element("x1_0") == (1, 0)
    assert m.string_to_basis_element("nope") is None


def test_fdmodule_set_action_invalid_raises():
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
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


def test_fdmodule_build():
    m = make_c2_fdmodule()
    sm = m.build()
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime == m.prime
    assert sm.dimension(0) == m.dimension(0)
    assert sm.dimension(1) == m.dimension(1)
    # The algebra accessor returns a SteenrodAlgebra at the same prime.
    assert sm.algebra().prime == 2


# --- FDModuleBuilder algebra-argument acceptance --------------------------


@pytest.mark.parametrize(
    "make_alg, expected_type",
    [
        # The union SteenrodAlgebra, in both bases.
        (lambda: algebra.SteenrodAlgebra.adem(2), algebra.AlgebraType.Adem),
        (lambda: algebra.SteenrodAlgebra.milnor(2), algebra.AlgebraType.Milnor),
        # The concrete variant pyclasses: these used to be rejected (the
        # constructor only accepted SteenrodAlgebra), and are reconstructed into
        # the matching SteenrodAlgebra variant.
        (lambda: algebra.AdemAlgebra(2, False), algebra.AlgebraType.Adem),
        (lambda: algebra.MilnorAlgebra(2, False), algebra.AlgebraType.Milnor),
    ],
)
def test_fdmodule_accepts_all_algebra_types(make_alg, expected_type):
    alg = make_alg()
    m = algebra.FDModuleBuilder(alg, "C2", [1, 1])
    # Building C2 with Sq1 x0 = x1 must work regardless of how the algebra was
    # supplied: the reconstructed algebra has identical prime/basis indexing.
    m.set_action(1, 0, 0, 0, [1])
    assert m.prime == 2
    # The builder's algebra is the matching SteenrodAlgebra variant.
    assert m.algebra().algebra_type() == expected_type
    sm = m.build()
    res = fp.FpVector(2, sm.dimension(1))
    sm.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1


def test_fdmodule_accepts_milnor_algebra_with_profile():
    # A profile-restricted MilnorAlgebra is accepted and reconstructed as Milnor.
    alg = algebra.MilnorAlgebra(2, False)
    m = algebra.FDModuleBuilder(alg, "", [1, 1])
    assert m.algebra().algebra_type() == algebra.AlgebraType.Milnor


def test_fdmodule_rejects_non_algebra_argument():
    with pytest.raises(TypeError):
        algebra.FDModuleBuilder("not an algebra", "", [1])
    with pytest.raises(TypeError):
        algebra.FDModuleBuilder(2, "", [1])


def test_fdmodule_to_json():
    # to_json mirrors upstream FiniteDimensionalModule::to_json: name/type/gens/
    # actions, and NOT the prime p (the caller adds that separately).
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    j = m.to_json()
    assert j["name"] == "C2"
    assert j["type"] == "finite dimensional module"
    assert j["gens"] == {"x0_0": 0, "x1_0": 1}
    assert j["actions"] == ["Sq1 x0_0 = x1_0"]
    assert "p" not in j
    # Still available after build() (it is a read-only query).
    m.build()
    assert m.to_json()["name"] == "C2"


# --- from_spec ------------------------------------------------------------


def test_from_spec_c2():
    sm = algebra.SteenrodModule.from_spec(C2_JSON, milnor(2))
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime == 2
    assert sm.min_degree() == 0
    assert sm.dimension(0) == 1
    assert sm.dimension(1) == 1
    assert sm.dimension(2) == 0
    assert sm.basis_element_to_string(0, 0) == "x0"


def test_from_spec_action_known_value():
    sm = algebra.SteenrodModule.from_spec(C2_JSON, milnor(2))
    # Sq1 . x0 = x1.
    res = fp.FpVector(2, sm.dimension(1))
    sm.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1


def test_from_spec_bad_spec_raises():
    with pytest.raises(ValueError):
        algebra.SteenrodModule.from_spec({"p": 2}, milnor(2))  # missing type
    with pytest.raises(ValueError):
        algebra.SteenrodModule.from_spec({"p": 2, "type": "bogus"}, milnor(2))


@pytest.mark.parametrize("name", ["milnor", "adem"])
def test_from_spec_string_algebra(name):
    sm = algebra.SteenrodModule.from_spec(C2_JSON, name)
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime == 2
    assert sm.dimension(0) == 1
    assert sm.dimension(1) == 1


def test_from_spec_string_algebra_case_insensitive():
    sm = algebra.SteenrodModule.from_spec(C2_JSON, "Milnor")
    assert isinstance(sm, algebra.SteenrodModule)
    assert sm.prime == 2
    assert sm.dimension(0) == 1
    assert sm.dimension(1) == 1


def test_from_spec_string_algebra_unknown_raises():
    with pytest.raises(ValueError):
        algebra.SteenrodModule.from_spec(C2_JSON, "foo")


def test_from_spec_string_algebra_missing_prime_raises():
    spec = {k: v for k, v in C2_JSON.items() if k != "p"}
    with pytest.raises(ValueError):
        algebra.SteenrodModule.from_spec(spec, "milnor")


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
    assert m.prime == 2
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
    assert sm.prime == m.prime
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
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    # op_degree 5 lands in output degree 5 (above max_degree 1); an empty
    # output used to slip past the length check and panic.
    with pytest.raises(ValueError):
        m.set_action(5, 0, 0, 0, [])
    # Valid set_action still works.
    m.set_action(1, 0, 0, 0, [1])
    assert list(m.action(1, 0, 0, 0)) == [1]


def test_fdmodule_build_shares_state_and_locks_mutation():
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    # Pre-build mutation works and is reflected in the built module (shared
    # state via Arc).
    m.set_action(1, 0, 0, 0, [1])
    sm = m.build()
    res = fp.FpVector(2, sm.dimension(1))
    sm.act_on_basis(res, 1, 1, 0, 0, 0)
    assert res[0] == 1
    # After build() the builder is locked: the `built` flag is checked first in
    # every mutator, so they raise RuntimeError even with valid arguments.
    with pytest.raises(RuntimeError):
        m.set_action(1, 0, 0, 0, [0])
    with pytest.raises(RuntimeError):
        m.add_generator(0, "x")
    with pytest.raises(RuntimeError):
        m.extend_actions(0, 1)
    with pytest.raises(RuntimeError):
        m.set_basis_element_name(0, 0, "y")


def test_fdmodule_build_lock_is_checked_before_validation():
    # The build-lock (the `built` flag) is checked FIRST, before any argument
    # validation, so even arguments that would otherwise raise ValueError/
    # IndexError raise a clean RuntimeError after build().
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    m.build()
    # op_degree 99 lands in an empty output degree (would be ValueError before
    # build); out-of-range indices (would be IndexError). Both must surface as
    # RuntimeError because the build-lock fires first.
    with pytest.raises(RuntimeError):
        m.set_action(99, 0, 0, 0, [])
    with pytest.raises(RuntimeError):
        m.set_action(1, 0, 0, 9, [1])
    with pytest.raises(RuntimeError):
        m.set_basis_element_name(0, 9, "y")


def test_fdmodule_build_callable_multiple_times():
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    sm1 = m.build()
    sm2 = m.build()
    # Both handles share the same finished module.
    assert isinstance(sm1, algebra.SteenrodModule)
    assert isinstance(sm2, algebra.SteenrodModule)
    assert sm1.dimension(1) == sm2.dimension(1) == 1
    # The builder stays locked regardless of how many handles are alive or
    # dropped: the `built` flag is the primary gate.
    import gc

    del sm1
    del sm2
    gc.collect()
    with pytest.raises(RuntimeError):
        m.set_action(1, 0, 0, 0, [0])


def test_fdmodule_build_result_usable_by_consumers():
    # A TensorModule built from FDModuleBuilder(...).build() still works (the
    # consumer that previously used into_steenrod_module()).
    m = algebra.FDModuleBuilder(milnor(2), "C2", [1, 1])
    m.set_action(1, 0, 0, 0, [1])
    sm = m.build()
    t = algebra.TensorModule(sm, sm)
    t.compute_basis(2)
    assert t.dimension(0) == 1


def test_fdmodulebuilder_present_fdmodule_absent():
    names = dir(algebra)
    assert "FDModuleBuilder" in names
    assert "FDModule" not in names


# --- invalid construction -------------------------------------------------


def test_module_from_json_prime_mismatch():
    # Passing a p=3 algebra to a p=2 spec must error, not panic.
    with pytest.raises((ValueError, RuntimeError)):
        algebra.SteenrodModule.from_spec(C2_JSON, milnor(3))


# --- FDModuleBuilder.from_tensor_module -----------------------------------


def make_c2_tensor_c2():
    """Build TensorModule(C2, C2) from two C2 modules over the same algebra.

    TensorModule requires both factors to share the *same* algebra object
    (checked via Arc::ptr_eq), so both are built from a single `alg`.
    """
    alg = algebra.SteenrodAlgebra.adem(2)
    left = algebra.SteenrodModule.from_spec(C2_JSON, alg)
    right = algebra.SteenrodModule.from_spec(C2_JSON, alg)
    return algebra.TensorModule(left, right)


def test_from_tensor_module_to_json_sensible():
    fd = algebra.FDModuleBuilder.from_tensor_module(make_c2_tensor_c2())
    j = fd.to_json()
    assert j["type"] == "finite dimensional module"
    # C2 (x) C2 has dimensions 1, 2, 1 in degrees 0, 1, 2.
    assert fd.min_degree() == 0
    assert [fd.dimension(t) for t in range(3)] == [1, 2, 1]
    # gens map names to degrees; there should be 4 of them.
    assert isinstance(j["gens"], dict)
    assert len(j["gens"]) == 4
    assert "p" not in j


def test_from_tensor_module_name_roundtrips():
    fd = algebra.FDModuleBuilder.from_tensor_module(make_c2_tensor_c2())
    # name is readable and settable.
    fd.name = ""
    assert fd.name == ""
    assert "name" not in fd.to_json()  # empty name is omitted from json
    fd.name = "C2 (x) C2"
    assert fd.name == "C2 (x) C2"
    assert fd.to_json()["name"] == "C2 (x) C2"


def test_from_tensor_module_name_setter_locked_after_build():
    fd = algebra.FDModuleBuilder.from_tensor_module(make_c2_tensor_c2())
    fd.build()
    with pytest.raises(RuntimeError):
        fd.name = "nope"
