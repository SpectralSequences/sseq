import pytest

from ext import algebra


# --- tau_degrees / xi_degrees ------------------------------------------------


def test_xi_degrees_known_values():
    # xi_i has degree (p^(i+1) - 1) / (p - 1) (divided by q).
    assert algebra.xi_degrees(2)[:5] == [1, 3, 7, 15, 31]
    assert algebra.xi_degrees(3)[:4] == [1, 4, 13, 40]


def test_tau_degrees_known_values():
    # Nonsense at p = 2 (documented), but matches the upstream table.
    assert algebra.tau_degrees(2)[:4] == [1, 3, 7, 15]
    assert algebra.tau_degrees(3)[:4] == [1, 5, 17, 53]


def test_degree_tables_reject_bad_primes():
    for bad in (0, 1, 4, 6):
        with pytest.raises(ValueError):
            algebra.xi_degrees(bad)
        with pytest.raises(ValueError):
            algebra.tau_degrees(bad)


def test_degree_tables_reject_prime_above_table_bound():
    # 257 is prime but exceeds the largest precomputed prime (251); must raise
    # ValueError rather than indexing out of bounds and panicking.
    with pytest.raises(ValueError):
        algebra.xi_degrees(257)
    with pytest.raises(ValueError):
        algebra.tau_degrees(257)


# --- adem_relation_coefficient ----------------------------------------------


def test_adem_relation_coefficient_known_values():
    # Sq^2 Sq^2 = Sq^3 Sq^1 at p = 2: the j = 1 term has coefficient 1.
    assert algebra.adem_relation_coefficient(2, 2, 2, 1, 0, 0) == 1
    assert algebra.adem_relation_coefficient(2, 2, 2, 0, 0, 0) == 0


def test_adem_relation_coefficient_is_reduced_mod_p():
    for _ in range(1):
        c = algebra.adem_relation_coefficient(3, 5, 4, 1, 0, 0)
        assert 0 <= c < 3


def test_adem_relation_coefficient_rejects_bad_primes():
    with pytest.raises(ValueError):
        algebra.adem_relation_coefficient(4, 2, 2, 1, 0, 0)
    with pytest.raises(ValueError):
        algebra.adem_relation_coefficient(257, 1, 1, 0, 0, 0)


def test_adem_relation_coefficient_rejects_oversized_args():
    # Pre-fix, absurdly large args drove the internal i32 degree arithmetic
    # (e.g. (y - j) * (p - 1) + e1 - 1) to overflow: a silent wrap in release
    # and only a panic in debug. They are now rejected with ValueError, and
    # normal args still return the known value.
    huge = 2_000_000
    with pytest.raises(ValueError):
        algebra.adem_relation_coefficient(2, huge, 2, 1, 0, 0)
    with pytest.raises(ValueError):
        algebra.adem_relation_coefficient(2, 2, 2, huge, 0, 0)
    assert algebra.adem_relation_coefficient(2, 2, 2, 1, 0, 0) == 1


# --- inadmissible_pairs ------------------------------------------------------


def test_inadmissible_pairs_known_values():
    # Sq^1 Sq^1 is the only inadmissible pair in degree 2 at p = 2.
    assert algebra.inadmissible_pairs(2, False, 2) == [(1, 0, 1)]
    # Sq^1 Sq^2 in degree 3.
    assert algebra.inadmissible_pairs(2, False, 3) == [(1, 0, 2)]


def test_inadmissible_pairs_rejects_bad_prime():
    with pytest.raises(ValueError):
        algebra.inadmissible_pairs(4, False, 2)


def test_inadmissible_pairs_rejects_negative_degree():
    # A negative degree is malformed input for this combinatorics function, so
    # it now raises ValueError specifically (not IndexError): the function used
    # to cast it to a huge u32 upstream.
    with pytest.raises(ValueError):
        algebra.inadmissible_pairs(2, False, -1)


def test_inadmissible_pairs_rejects_oversized_degree():
    # Pre-fix, a huge degree made upstream push a multi-GB Vec (an uncatchable
    # OOM abort). It is now rejected with ValueError before allocating, while a
    # normal small degree still returns the correct pairs.
    with pytest.raises(ValueError):
        algebra.inadmissible_pairs(2, False, 100_001)
    assert algebra.inadmissible_pairs(2, False, 2) == [(1, 0, 1)]


# --- module_gens_from_json ---------------------------------------------------


def test_module_gens_from_json_joker():
    gens = {"x0": 0, "x1": 1, "x2": 2, "x3": 3, "x4": 4}
    dims, names = algebra.module_gens_from_json(gens)
    assert dims == {0: 1, 1: 1, 2: 1, 3: 1, 4: 1}
    assert names == {
        0: ["x0"],
        1: ["x1"],
        2: ["x2"],
        3: ["x3"],
        4: ["x4"],
    }


def test_module_gens_from_json_multiple_in_one_degree():
    gens = {"a": 0, "b": 0, "c": 1}
    dims, names = algebra.module_gens_from_json(gens)
    assert dims == {0: 2, 1: 1}
    assert sorted(names[0]) == ["a", "b"]
    assert names[1] == ["c"]


def test_module_gens_from_json_rejects_non_object():
    with pytest.raises(ValueError):
        algebra.module_gens_from_json([1, 2, 3])


def test_module_gens_from_json_rejects_non_integer_degree():
    with pytest.raises(ValueError):
        algebra.module_gens_from_json({"x0": "not an int"})


def test_module_gens_from_json_rejects_huge_degree():
    # Pre-fix, a degree near i32::MAX made upstream's
    # BiVec::with_capacity(min, max + 1) attempt a ~4-billion-entry allocation
    # (an uncatchable OOM abort) and `max + 1` overflow i32. Now ValueError.
    with pytest.raises(ValueError):
        algebra.module_gens_from_json({"x": 2147483647})


def test_module_gens_from_json_rejects_huge_span():
    # Each degree's magnitude is huge; upstream would allocate the full span.
    with pytest.raises(ValueError):
        algebra.module_gens_from_json({"a": -2000000000, "b": 2000000000})


def test_module_gens_from_json_rejects_just_over_cap():
    # Just over the per-degree cap (1_000_000) -> ValueError.
    with pytest.raises(ValueError):
        algebra.module_gens_from_json({"x": 1000001})


def test_module_gens_from_json_legitimate_small_spec_still_works():
    # A realistic small spec is unaffected by the guards.
    dims, names = algebra.module_gens_from_json({"x0": 0, "x1": 7, "x2": 7})
    # dims spans the full [min, max] degree range (empty degrees map to 0),
    # matching upstream's BiVec semantics.
    assert dims == {0: 1, 1: 0, 2: 0, 3: 0, 4: 0, 5: 0, 6: 0, 7: 2}
    assert names[0] == ["x0"]
    assert sorted(names[7]) == ["x1", "x2"]


def test_dualpairs_indexer_not_bound():
    # DualpairsIndexer does not exist upstream; it must not have been invented.
    assert not hasattr(algebra, "DualpairsIndexer")
