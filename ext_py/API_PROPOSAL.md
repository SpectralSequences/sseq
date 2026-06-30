# Python API Proposal for `ext`

This document specifies the Python API for `ext`, the PyO3 bindings to the `ext` workspace.

## Guiding principle

Two rules drive every decision below:

1. **Completeness.** Every `pub` item in the four bound crates — `fp`, `algebra`, `sseq`, and
   `ext` — gets a Python binding. The Rust public surface *is* the Python surface.
2. **Lightness.** Every binding is 1–3 lines of glue: unwrap the Python arguments into Rust
   types, call the one underlying Rust function, wrap the result back up for Python. No logic
   lives in the binding layer. Multi-step workflows (resolving then charting, building a Massey
   product, …) are *not* bindings — they are Python scripts that compose the bound primitives, just
   as the current `examples/*.rs` compose the Rust API. Orchestration lives in Python `examples/`,
   the per-`pub`-item glue lives in Rust.

The bindings are therefore *mechanical*. The only real design work is (a) choosing the concrete
monomorphizations of the generic Rust types, and (b) fixing a handful of wrapping conventions so
the glue stays uniform. Everything after that is enumeration.

---

## 1. Monomorphization strategy

The Rust crates are generic over the prime `P`, the algebra `A`, the module `M`, the chain
complex `CC`, and a const `U: bool` (stable vs. unstable). Python has no monomorphization, and the
project's standing preference is to **reduce generics and use dynamic dispatch where performance is
not on the critical path** (the heavy lifting is already inside `fp`/`algebra`). So the bindings
commit to one concrete instantiation of each generic parameter and bind *that*:

| Rust generic parameter | Bound instantiation | Rationale |
|---|---|---|
| Prime `P: Prime` | `ValidPrime` (runtime prime) | One vector/matrix type covers all primes; `P2`/`P3`… are an internal perf detail. |
| Algebra `A: Algebra` | `SteenrodAlgebra` (the `enum_dispatch` union) | Already a runtime union of Adem/Milnor; the natural dynamic type. |
| Module `M: Module` | `SteenrodModule = Box<dyn Module<Algebra = SteenrodAlgebra>>` | The crate already provides this trait object; concrete module types coerce into it. |
| Chain complex `CC` | `CCC = FiniteChainComplex<SteenrodModule>` | The crate's own default alias; what `utils::construct` returns. |
| Sseq `Sseq<N, P>` | `Sseq<2, Adams>` | The only spectral sequence the examples use. |
| Const `U: bool` | bound twice: `false` and `true` | Stable and unstable resolutions become two distinct Python classes. |

Consequences:

- Concrete module types (`FDModuleBuilder`, `FreeModule`, `TensorModule`, …) are bound over
  `SteenrodAlgebra`, and each exposes a conversion into the dynamic `SteenrodModule` accepted
  everywhere downstream (`FreeModule`/`TensorModule` via `.into_steenrod_module()`; the
  finite-dimensional builder via `FDModuleBuilder.build()`).
- The `MuResolution<U, CCC>` / `MuResolutionHomomorphism<U, …>` families bind as `Resolution` /
  `UnstableResolution` and `ResolutionHomomorphism` / `UnstableResolutionHomomorphism`.
- Trait methods (`Algebra`, `Module`, `ChainComplex`, `ModuleHomomorphism`, …) are bound as
  inherent methods on each concrete `#[pyclass]`. There is no Python-visible trait hierarchy;
  Python's duck typing stands in for the Rust traits.

---

## 2. Binding conventions

These are the templates. Every item in §4–§7 is one of these shapes.

### 2.1 Newtype wrapper

Every Rust type is wrapped in a tuple struct `#[pyclass]`. Add `From` impls in both directions so
argument-unwrapping and return-wrapping are a single `.into()`.

```rust
#[pyclass]
pub struct MilnorAlgebra(::algebra::MilnorAlgebra);

impl From<::algebra::MilnorAlgebra> for MilnorAlgebra {
    fn from(x: ::algebra::MilnorAlgebra) -> Self { MilnorAlgebra(x) }
}
```

### 2.2 Shared / immutable objects: `frozen` + `Arc`

Resolutions, algebras, modules and homomorphisms are shared by `Arc` in Rust and are interior-mutable
(they compute into `OnceVec`s). Bind them as `#[pyclass(frozen)]` holding an `Arc`, so `&self`
methods suffice and Python can hold many references cheaply. This matches the existing `Resolution`
and `SecondaryResolution` bindings.

```rust
#[pyclass(frozen)]
#[derive(Clone)]
pub struct Resolution(Arc<ext::resolution::Resolution<ext::CCC>>);
```

`Clone` on the wrapper is an `Arc::clone`, so passing a resolution into another constructor is free.

### 2.3 Method body: unwrap → call → wrap

```rust
#[pymethods]
impl Resolution {
    fn module(&self, s: i32) -> FreeModule {
        self.0.module(s).into()          // call Rust, wrap result
    }
    fn compute_through_stem(&self, max: Bidegree) {
        self.0.compute_through_stem(max.0)   // unwrap arg, call Rust
    }
}
```

### 2.4 Errors: `anyhow`/`Result` → `PyRuntimeError`

Anything returning `anyhow::Result` or another `Result` maps the error through `to_string`, exactly
as the current `query_module` binding does:

```rust
fn parse_module_name(name: &str) -> PyResult<PyObject> {
    ext::utils::parse_module_name(name)
        .map(json_to_py)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}
```

A `panic!`-ing Rust API (e.g. `Module::dimension` out of range) is left to PyO3's panic-to-exception
machinery; we do not add guard code in the binding.

### 2.5 Enums

C-like Rust enums (`AlgebraType`, `Orientation`) bind directly as `#[pyclass]` enums with a `From`
to the Rust enum, as `AlgebraType` already does.

### 2.6 `serde_json::Value` ↔ Python

Rust APIs that speak `serde_json::Value` (module JSON, `to_json`, `parse_module_name`) convert to and
from native Python `dict`/`list` via a single `json_to_py` / `py_to_json` helper pair (using
`pythonize`, or a hand-rolled walk). At the call sites this is one `.into()`-style call, keeping the
binding thin while giving Python users plain dicts rather than opaque handles.

### 2.7 Slices and lifetimes: owning handle + range

`FpSlice<'a>` / `FpSliceMut<'a>` / `MatrixSliceMut<'a>` are borrowed, lifetime-bearing views and so
cannot be stored in a `#[pyclass]` directly. They are bound by **owning the parent and remembering
the range**: the slice pyclass holds a `Py<FpVector>` (a reference-counted handle to the parent
vector, keeping it alive) together with the `(start, end)` it covers, and reconstructs a real Rust
slice on demand at each call:

```rust
#[pyclass]
pub struct FpSlice { parent: Py<FpVector>, start: usize, end: usize }

#[pymethods]
impl FpSlice {
    fn entry(&self, py: Python, i: usize) -> u32 {
        self.parent.borrow(py).0.slice(self.start, self.end).entry(i)   // rebuild slice, call
    }
}
```

`FpSliceMut` is the same, calling `borrow_mut(py)` and `.slice_mut(..)`; Python's borrow rules give
the exclusive access the mutable slice needs (a `RuntimeError` if aliased, which is the correct
behaviour). `MatrixSliceMut` holds a `Py<Matrix>` plus a row/column rectangle. This keeps the
binding thin (rebuild-then-call is one line) and **zero-copy**, and the slice types stay faithful
members of the public surface rather than being collapsed to owned copies. Methods that *return* a
slice return one of these handle pyclasses pointing at the parent the caller already passed in;
methods that *take* a slice accept either the slice pyclass or a plain `FpVector` (whose
`.as_slice()` is used).

### 2.8 Iterators

Rust methods returning `impl Iterator` bind as a method that `.collect()`s into a Python `list` (the
simple, eager choice), unless the iterator is large/lazy by nature (`iter_nonzero_stem`,
`iter_degrees`), in which case we wrap it in a small `#[pyclass]` implementing `__iter__`/`__next__`.
The examples only ever `for`-loop these, so either is observationally identical.

### 2.9 Module layout

The Python package mirrors the crates, reusing the submodule names already present in `src/`:

```python
import ext
from ext import fp, algebra, sseq
# ext-crate items (Resolution, query_module, …) live at the top level of ext
```

`fp`, `algebra`, `sseq` are PyO3 submodules; the `ext` crate's own surface sits directly in
`ext`. Below, each section header notes its Python home.

---

## 3. Naming map

Rust → Python name conventions, applied uniformly:

- snake_case methods are kept as-is (already Python style).
- Constructors `T::new(..)` → `T(..)` (`#[new]`); other associated fns → `@staticmethod`
  (`Bidegree::s_t` → `Bidegree.s_t`), matching the existing `Bidegree` binding.
- Trait methods are flattened onto the concrete pyclass.
- Type aliases keep the short name (`FDModule`, `FPModule`, `CCC` → `ChainComplex`).
- `Mu*<U=false>` → base name; `Mu*<U=true>` → `Unstable*`.

---

## 4. `fp` crate  →  `ext.fp`

The numeric foundation. `u32` ↔ Python `int`; `Vec<u32>` ↔ `list[int]`.

### 4.1 `fp::prime`

| Rust item | Python binding |
|---|---|
| `ValidPrime::new(p) -> Self` (panicking) | `ValidPrime(p)` |
| `ValidPrime::new_unchecked(p)` | `ValidPrime.new_unchecked(p)` (staticmethod) |
| `ValidPrime` as int-like | `__int__`, `__index__`, `__eq__`, `__hash__`, `__repr__` via `as_u32`/`Display` |
| `Prime::{as_i32, as_u32, as_usize}` | folded into `__int__` (bind on `ValidPrime`) |
| `Prime::sum(a, b)` / `product` / `inverse` / `pow` / `pow_mod` | methods on `ValidPrime` |
| `power_mod(p, b, e)` | `fp.power_mod(p, b, e)` |
| `log2(n)`, `logp(p, n)`, `factor_pk(p, n)`, `inverse(p, k)`, `minus_one_to_the_n(p, i)`, `is_prime(p)` | module-level `fp.*` functions |
| `Binomial::{binomial, multinomial, binomial_odd_is_zero, …}` | module-level `fp.binomial(p, n, k)`, `fp.multinomial(p, list)`, … (take prime + ints) |
| `TWO` constant | `fp.TWO` |
| `PRIMES`, `NUM_PRIMES`, `PRIME_TO_INDEX_MAP`, `MAX_MULTINOMIAL_LEN`, `ODD_PRIMES` | module attributes |

`Prime` trait itself and the static `P2/P3/P5/P7` types are **not** bound — they are the
monomorphization detail §1 collapses into `ValidPrime`. (Internal; see §8.)

### 4.2 `fp::field`

| Rust item | Python binding |
|---|---|
| `Fp::new(p)` | `Fp(p)` |
| `Fp::{characteristic, degree, zero, one, element}` | methods on `Fp` |
| `SmallFq::new(p, degree)` | `SmallFq(p, degree)` |
| `SmallFq::{p, degree, a, q, zero, one}` | methods on `SmallFq` |
| `FieldElement::{inv, frobenius, field}` + arithmetic | `FieldElement` pyclass with `__add__`/`__sub__`/`__mul__`/`__truediv__`/`__neg__`, `inv`, `frobenius`; `__int__` via `Deref` |
| `F2` (and `F3/F5/F7` under `odd-primes`) | `fp.F2`, … |

The `Field` trait is flattened onto `Fp` and `SmallFq`.

### 4.3 `fp::vector`

`FpVector` is the owned vector; `FpSlice`/`FpSliceMut` are bound as handle+range pyclasses (§2.7).

| Rust item | Python binding |
|---|---|
| `FpVector::new(p, len)` | `FpVector(p, len)` |
| `new_with_capacity`, `from_slice(p, &[u32])`, `from_bytes` | `@staticmethod` constructors; `from_slice(p, list)` |
| `prime, len, is_empty, entry, density, is_zero, first_nonzero` | query methods; `len` also `__len__`, `entry` also `__getitem__` |
| `set_entry, scale, set_to_zero, add_basis_element, copy_from_slice, assign, extend_len, set_scratch_vector_size` | mutators; `set_entry` also `__setitem__` |
| `add(other, c)`, `add_offset`, `add_truncate`, `add_carry` | methods; `add(other, 1)` backs `__add__`/`__iadd__` |
| `sign_rule(other)` | method |
| `iter` / `iter_nonzero` | `__iter__` / `iter_nonzero()` |
| `to_bytes` / `update_from_bytes` | `to_bytes() -> bytes` / `update_from_bytes(bytes)` |
| `padded_len`, `num_limbs` | `@staticmethod` |
| `FpVectorIterator`, `FpVectorNonZeroIterator` | small `#[pyclass]` iterator wrappers |
| `FpSlice` — `prime, len, is_empty, entry, is_zero, first_nonzero, restrict, iter, iter_nonzero, to_owned` | `FpSlice` pyclass = `Py<FpVector>` + `(start, end)` (§2.7); `slice(v, a, b)` / `v.slice(a, b)` build it |
| `FpSliceMut` — `prime, set_entry, set_to_zero, scale, add_basis_element, add, add_offset, add_masked, add_unmasked, add_tensor, assign, slice_mut, as_slice` | `FpSliceMut` pyclass = `Py<FpVector>` + range (§2.7), `borrow_mut` on each call |

### 4.4 `fp::matrix`

| Rust item | Python binding |
|---|---|
| `Matrix::new(p, rows, cols)` | `Matrix(p, rows, cols)` |
| `from_rows, from_row, from_vec, from_data, from_bytes, identity, augmented_from_vec` | `@staticmethod` constructors |
| `prime, rows, columns, pivots, is_zero, to_vec, to_bytes` | queries / conversions |
| `row(i)` → `FpSlice`, `row_mut(i)` → `FpSliceMut`, `slice_mut(...)` → `MatrixSliceMut` | handle+range pyclasses per §2.7 (zero-copy) |
| `set_to_zero, assign, swap_rows, initialize_pivots, extend_column_dimension, extend_column_capacity, add_row, safe_row_op, trim, rotate_down` | mutators |
| `row_reduce` → `int` (rank), `find_pivots_permutation` | methods |
| `compute_kernel, compute_image, compute_quasi_inverse` | return `Subspace`/`QuasiInverse` |
| `apply(result, coeff, input)` | `apply(result_slice, coeff, input_slice)` — takes `FpSliceMut`/`FpSlice` or `FpVector` |
| `MatrixSliceMut` — `prime, rows, columns, row, row_mut, row_slice, iter, iter_mut, add_identity, add_masked` | `MatrixSliceMut` pyclass = `Py<Matrix>` + rectangle (§2.7) |
| `naive_mul` and `Mul` | `__matmul__` / `__mul__` |
| `iter` / `iter_mut` | `__iter__` yielding owned rows |
| `Subspace` — `new, from_matrix, entire_space, dimension, ambient_dimension, contains, contains_space, add_vector, reduce, sum, iter, iter_all_vectors, set_to_zero, set_to_entire, to_bytes, from_bytes` | full `Subspace` pyclass |
| `Subquotient` — `new, new_full, dimension, ambient_dimension, gens, zeros, reduce, quotient, add_gen, clear_gens, …` | full `Subquotient` pyclass |
| `QuasiInverse` — `new, image_dimension, source_dimension, target_dimension, to_bytes, from_bytes` + `apply` | `QuasiInverse` pyclass |
| `AffineSubspace` — `new, offset, linear_part, contains, contains_space, sum` | pyclass |
| `AugmentedMatrix<N>` | bind `N=2` and `N=3` as `AugmentedMatrix2` / `AugmentedMatrix3` (const-generic → two classes), with `segment`, `row_segment`, `compute_kernel/image/quasi_inverse` (N=3), `into_matrix`, `add_identity` |

The BLAS variants (`fast_mul_*`, `blas`) fold behind `__mul__` and are not individually exposed.

---

## 5. `algebra` crate  →  `ext.algebra`

### 5.1 The `Algebra` surface

The `Algebra` trait (and `GeneratedAlgebra`, `Bialgebra`, `UnstableAlgebra`) is flattened onto each
concrete algebra pyclass. The bound method set, per algebra:

`prime, compute_basis, dimension, basis_element_to_string, basis_element_from_string,
element_to_string, multiply_basis_elements, multiply_basis_element_by_element,
multiply_element_by_basis_element, multiply_element_by_element, default_filtration_one_products,
generators, generator_to_string, decompose_basis_element, generating_relations, coproduct, decompose`.

The `multiply_*` family's `result: FpSliceMut` / input `FpSlice` arguments accept the slice
handle pyclasses (or a plain `FpVector`) per §2.7; the binding rebuilds the Rust slice and calls
through.

### 5.2 Concrete algebras

| Rust item | Python binding |
|---|---|
| `SteenrodAlgebra` (enum union) | primary `SteenrodAlgebra` pyclass |
| `SteenrodAlgebra::from_json(value, ty, unstable)` | `SteenrodAlgebra.from_json(dict, ty, unstable)` (staticmethod) |
| convenience constructors | `SteenrodAlgebra.adem(p, unstable=False)`, `SteenrodAlgebra.milnor(p, unstable=False)` |
| `MilnorAlgebra::new(p, unstable)` | `MilnorAlgebra(p, unstable_enabled=False)` (already drafted) |
| `MilnorAlgebra::{new_with_profile, generic, q, profile, basis_element_from_index, basis_element_to_index, try_basis_element_to_index, ppart_table, beps_pn, multiply}` | methods |
| `MilnorProfile`, `MilnorBasisElement` | small pyclasses (fields + `compute_degree`) |
| `AdemAlgebra::new(p, unstable)` | `AdemAlgebra(p, unstable_enabled=False)` |
| `AdemAlgebra::{generic, q, basis_element_from_index, basis_element_to_index, beps_pn}` | methods |
| `AdemBasisElement`, `PorBockstein` | pyclass / enum |
| `Field::new(p)` | `algebra.Field(p)` (the trivial 1-dim algebra; distinct from `fp.Fp`) |
| `AlgebraType` enum | already bound; keep `MILNOR`/`ADEM` |
| `module_gens_from_json(value)` | `algebra.module_gens_from_json(dict)` → `(graded_dims, names)` (the returned closure is dropped; not bindable, see §8) |
| `combinatorics::{adem_relation_coefficient, inadmissible_pairs, tau_degrees, xi_degrees}` | module-level functions |
| `combinatorics::DualpairsIndexer` | pyclass |

`PPartAllocation`/`PPartMultiplier` are perf-workspace types — internal (§8).

### 5.3 Modules (`algebra::module`)  →  `algebra`

All concrete module types are bound over `SteenrodAlgebra`, each with the flattened `Module` method
set (`algebra, min_degree, max_computed_degree, dimension, compute_basis, act_on_basis, act,
act_by_element, basis_element_to_string, element_to_string, is_unit, prime, max_degree,
total_dimension`) plus `.into_steenrod_module()`.

| Rust item | Python binding |
|---|---|
| `SteenrodModule = Box<dyn Module<Algebra=SteenrodAlgebra>>` | `SteenrodModule` pyclass (the dynamic module §1) |
| `from_json(algebra, value)` (steenrod_module) | `algebra.steenrod_module_from_json(algebra, dict)` |
| `FDModule::new(algebra, name, graded_dims)` | `FDModuleBuilder(algebra, name, graded_dims, min_degree=0)` (call `.build()` for the `SteenrodModule`) |
| `FDModule::{set_basis_element_name, add_generator, set_action, action, extend_actions, check_validity, parse_action, string_to_basis_element, from_json, to_json, test_equal}` | methods (`to_json` → dict; `set_action` takes `FpVector`) |
| `FreeModule::new(algebra, name, min_degree)` | `FreeModule(algebra, name, min_degree)` |
| `FreeModule::{add_generators, number_of_gens_in_degree, gen_names, generator_offset, internal_generator_offset, operation_generator_to_index, index_to_op_gen, extend_by_zero, iter_gens}` | methods |
| `OperationGeneratorPair` | small pyclass (4 int fields) |
| `FinitelyPresentedModule::new` (`FPModule`) | `FPModule(algebra, name, min_degree)` |
| `FPModule::{generators, add_generators, add_relations, gen_idx_to_fp_idx, fp_idx_to_gen_idx, from_json}` | methods |
| `TensorModule::new(left, right)` | `TensorModule(left, right)` (`+ seek_module_num, offset`) |
| `SuspensionModule::new(inner, shift)` | `SuspensionModule(inner, shift)` |
| `QuotientModule::new(module, truncation)` | `QuotientModule(module, truncation)` (`+ quotient*, reduce, old_basis_to_new`) |
| `HomModule::new(source, target)` | `HomModule(source, target)` (`+ source, target`) |
| `RealProjectiveSpace::new(algebra, min, max, clear_bottom)` | `RealProjectiveSpace(algebra, min, max, clear_bottom)` (`+ from_json/to_json`) |
| `ZeroModule` | `ZeroModule(algebra, min_degree)` |
| `BlockStructure`, `GeneratorBasisEltPair` | pyclasses |
| `FDModule::from_tensor_module` (used by `tensor.py`) | provide as `FDModuleBuilder.from_module(steenrod_module)` — thin wrapper over the existing `From`/bounded-module conversion (not yet bound) |

### 5.4 Module homomorphisms (`algebra::module::homomorphism`)  →  `algebra`

`ModuleHomomorphism` flattened method set on each: `source, target, degree_shift, apply,
apply_to_basis_element, kernel, image, quasi_inverse, compute_auxiliary_data_through_degree,
get_partial_matrix, apply_quasi_inverse, min_degree, prime`.

| Rust item | Python binding |
|---|---|
| `FreeModuleHomomorphism::new(source, target, degree_shift)` | `FreeModuleHomomorphism(source, target, degree_shift)` |
| `FreeModuleHomomorphism::{output, next_degree, extend_by_zero, add_generators_from_rows, add_generators_from_matrix_rows, apply_to_generator, hom_k, set_image/kernel/quasi_inverse, differential_density}` | methods |
| `FullModuleHomomorphism::{new, from_matrices, from, replace_source, replace_target}` | `FullModuleHomomorphism` pyclass |
| `QuotientHomomorphism`, `QuotientHomomorphismSource` | pyclasses |
| `GenericZeroHomomorphism::new` | pyclass |
| `HomPullback::new` | pyclass |
| `ZeroHomomorphism`/`IdentityHomomorphism` traits | bound as `@staticmethod` `zero(s, t, shift)` / `identity(s)` constructors on the relevant pyclasses |

The `UnstableFreeModuleHomomorphism` (`U=true`) binds as `UnstableFreeModuleHomomorphism`.

### 5.5 Steenrod evaluator / parser  →  `algebra`

| Rust item | Python binding |
|---|---|
| `SteenrodEvaluator::new(p)` | `SteenrodEvaluator(p)` |
| `evaluate_algebra_adem(str)` → `(i32, FpVector)` | `evaluate_algebra_adem(s) -> (deg, FpVector)` |
| `evaluate_algebra_milnor(str)` | likewise |
| `evaluate_module_adem(str)` → `BTreeMap<String,(i32,FpVector)>` | returns `dict[str, (int, FpVector)]` |
| `adem_to_milnor(result, coeff, deg, input)` / `milnor_to_adem` | `adem_to_milnor(deg, vec) -> FpVector` (own output) |
| `adem_element_to_string` / `milnor_element_to_string` | via the algebra's `element_to_string` |
| `parse_algebra(str)`, `parse_module(str)` | `algebra.parse_algebra`, `parse_module` returning the parse-tree pyclasses |
| `AlgebraNode, AlgebraBasisElt, ModuleNode, BocksteinOrSq` | enum/pyclass bindings (needed for `parse_*` returns) |

`PairAlgebra` trait + `pair_algebra` element type: bound only as far as `SecondaryResolution` needs
(see §7.4); the associated `Element` type is opaque (`#[pyclass]` with `element_is_zero`,
`to/from_bytes`). Marked low priority.

---

## 6. `sseq` crate  →  `ext.sseq`

### 6.1 `sseq::coordinates`

Bind the `N=2` aliases as first-class; the general `MultiDegree<N>` is not exposed (the bidegree case
is the only one used).

| Rust item | Python binding |
|---|---|
| `Bidegree::{s_t, n_s, x_y}` | already drafted: `Bidegree.s_t`, `Bidegree.n_s`, `Bidegree.x_y` |
| `Bidegree::{n, s, t, x, y, coords}` | properties / methods |
| `Bidegree` `Add`/`Sub`/`Display`/`Eq`/`Hash` | `__add__`, `__sub__`, `__str__`, `__eq__`, `__hash__` |
| `BidegreeElement::new(degree, vec)` | `BidegreeElement(degree, vec)` |
| `BidegreeElement::{degree, n, s, t, x, y, vec, into_vec, to_basis_string}` + `Display` | methods (`vec` → owned `FpVector`) |
| `BidegreeGenerator::{new, s_t, n_s}` | `BidegreeGenerator(degree, idx)`, `.s_t`, `.n_s` |
| `BidegreeGenerator::{degree, idx, n, s, t, x, y, into_element}` + `Display` | methods |
| `BidegreeRange::{new, s, t, restrict}` | `BidegreeRange` pyclass (mainly an argument carrier) |
| `iter_s_t(f, min, max)` | `sseq.iter_s_t(callback, min, max)` |

The `ordered` submodule (`ByStem`, `ByInternalDegree`, `OrderedMultiDegree`, …) is sorting
machinery — internal (§8).

### 6.2 `Sseq` (bound as `Sseq<2, Adams>`)

| Rust item | Python binding |
|---|---|
| `Sseq::new(p)` | `Sseq(p)` |
| `set_dimension, dimension, get_dimension, clear` | methods |
| `min, max, defined, iter_degrees` | methods (`iter_degrees` → iterator) |
| `add_permanent_class, permanent_classes` | methods (`permanent_classes` → `Subspace`) |
| `add_differential(r, source, target_vec)` | method |
| `differentials, differentials_hitting` | return `Differential` / iterator |
| `page_data, invalid, update, update_degree, complete, inconsistent` | methods |
| `multiply(elem, product)`, `leibniz(...)` | methods |
| `write_to_graph(backend, *, page, differentials=False, products=[], header=None)` | the charting entry point used by `chart.py` |
| `Adams`, `SseqProfile`, `Product<2>` | `Adams` marker exposed as default; `Product` pyclass (`b`, `left`, `matrices`) |
| `Differential::{new, add, set_to_zero, prime, inconsistent, get_source_target_pairs, evaluate, quasi_inverse}` | `Differential` pyclass (slice args per §2.7) |

### 6.3 `sseq::charting`

| Rust item | Python binding |
|---|---|
| `SvgBackend::new(writer)` | `SvgBackend(file)` — accepts any Python file-like / `sys.stdout`; the binding wraps it in a Rust `io::Write` adapter |
| `SvgBackend::legend(writer)` | `SvgBackend.legend(file)` (staticmethod) |
| `TikzBackend::new(writer)` | `TikzBackend(file)` |
| `Orientation` enum | `sseq.Orientation.{Left, Right, Above, Below}` |
| `Backend` trait methods (`header, line, text, node, structline, init, structline_matrix`) | flattened onto `SvgBackend`/`TikzBackend` (so charts can be driven manually from Python too) |

Note: `write_to_graph` is generic over `T: Backend`; the binding accepts the concrete
`SvgBackend`/`TikzBackend` pyclasses (an enum-dispatch over the two bound backends), keeping the call
monomorphic.

---

## 7. `ext` crate  →  `ext` (top level)

### 7.1 `ext::chain_complex`

`ChainComplex` + `FreeChainComplex` + `AugmentedChainComplex` + `BoundedChainComplex` flattened onto
each concrete complex pyclass:

`algebra, module, differential, min_degree, zero_module, has_computed_bidegree,
compute_through_bidegree, next_homological_degree, prime, iter_stem, save_dir` and (free)
`graded_dimension_string, to_sseq, filtration_one_products, filtration_one_product,
number_of_gens_in_bidegree, iter_nonzero_stem, boundary_string` and (augmented) `target, chain_map`
and (bounded) `max_s, euler_characteristic`.

| Rust item | Python binding |
|---|---|
| `CCC = FiniteChainComplex<SteenrodModule>` | `ChainComplex` pyclass |
| `FiniteChainComplex::{new, ccdz, pop, map}` | constructors / methods (`map` over modules → not bound; see §8) |
| `FiniteAugmentedChainComplex::{augment, map}` | `augment` bound; constructed mainly via `utils` |
| `ChainHomotopy::{new, extend, extend_all, shift, left, right, homotopy, prime}` | `ChainHomotopy` pyclass (used to build Massey products — §7.6) |
| `StemIterator` | iterator pyclass backing `iter_stem` |

### 7.2 `ext::resolution`

| Rust item | Python binding |
|---|---|
| `Resolution<CCC>` (= `MuResolution<false, CCC>`) | `Resolution` pyclass (already partly bound) |
| `Resolution::{new, new_with_save, set_name, name}` | `Resolution(chain_complex)`, `Resolution.with_save(cc, dir)`, `name` property |
| `compute_through_bidegree(_with_callback)`, `compute_through_stem(_with_callback)` | methods (callback = Python callable) |
| all `ChainComplex`/`FreeChainComplex`/`AugmentedChainComplex` methods | per §7.1 — incl. `to_sseq`, `graded_dimension_string`, `filtration_one_products`, `module`, `iter_nonzero_stem`, `number_of_gens_in_bidegree` |
| `UnstableResolution<CCC>` (`U=true`) | `UnstableResolution` pyclass, incl. `to_unstable_sseq` (= `to_sseq` on the unstable complex) |

### 7.3 `ext::resolution_homomorphism`

| Rust item | Python binding |
|---|---|
| `ResolutionHomomorphism::{new, from_class, from_module_homomorphism}` | `ResolutionHomomorphism(name, source, target, shift)`, `.from_class(...)`, `.from_module_homomorphism(...)` |
| fields `source, target, shift` | read-only properties |
| `{name, algebra, next_homological_degree, get_map, extend, extend_through_stem, extend_all, extend_step, act, save_dir}` | methods (`act` writes into an `FpVector`) |
| `UnstableResolutionHomomorphism` | bound analogously |

### 7.4 `ext::secondary`

| Rust item | Python binding |
|---|---|
| `SecondaryResolution::new(resolution)` | already drafted: `SecondaryResolution(resolution)` |
| `SecondaryLift` flattened: `underlying, algebra, prime, source, target, shift, max, homotopy, homotopies, intermediates, compute_partial, compute_intermediates, compute_homotopy_step, extend_all, extend_through_stem, compute_through_bidegree, initialize_homotopies, compute_composites` | methods on `SecondaryResolution` |
| `SecondaryResolution::e3_page()` | `e3_page() -> Sseq` |
| `SecondaryResolutionHomomorphism::{new, name, homotopy, hom_k, hom_k_with, product_nullhomotopy}` | pyclass |
| `SecondaryHomotopy::{new, add_composite, act, composite}` | pyclass |
| `SecondaryComposite::{new, finalize, add_composite, act, to_bytes, from_bytes}` | pyclass |
| `SecondaryChainHomotopy` | pyclass |
| `LAMBDA_BIDEGREE` constant | `ext.LAMBDA_BIDEGREE` |

### 7.5 `ext::yoneda`

| Rust item | Python binding |
|---|---|
| `yoneda_representative_element(cc, b, class)` | `ext.yoneda_representative_element(resolution, bidegree, class)` (used by `steenrod.py`) |
| `yoneda_representative(cc, map)` | `ext.yoneda_representative(resolution, chain_map)` |
| `yoneda_representative_with_strategy(cc, map, strategy)` | with a Python `strategy` callable |
| `Yoneda<CC>` alias (a `FiniteAugmentedChainComplex`) | returned as the bound augmented-complex pyclass |

### 7.6 `ext::nassau`

| Rust item | Python binding |
|---|---|
| `nassau::Resolution::{new, new_with_save, name, set_name, compute_through_stem}` + `ChainComplex` | `NassauResolution(module)` pyclass (Milnor-only; feature-gated mirror of `Resolution`) |

### 7.7 `ext::utils`

| Rust item | Python binding |
|---|---|
| `query_module(save_dir, load_quasi_inverse)` | already bound (`query_module(algebra_type, save)` — reconcile signature: real Rust takes `Option<PathBuf>` + bool) |
| `query_module_only(prompt, algebra, load_quasi_inverse)` | already bound |
| `query_unstable_module(load_quasi_inverse)` / `query_unstable_module_only()` | `query_unstable_module`, `query_unstable_module_only` |
| `construct(spec, save_dir)` | `ext.construct(spec, save_dir=None)` (spec = dict or `"S_2"` string) |
| `construct_standard`, `construct_nassau` | `construct_standard`, `construct_nassau` |
| `parse_module_name(name)` | `ext.parse_module_name(name) -> dict` |
| `load_module_json(name)` | `ext.load_module_json(name) -> dict` |
| `get_unit(resolution)` | `ext.get_unit(resolution) -> (is_unit, unit_resolution)` (used by `massey.py`) |
| `unicode_num(n)`, `secondary_job()` | module-level helpers |
| `init_logging()` | already called in `#[pymodule_init]`; also expose `ext.init_logging()` |
| `Config` struct | `ext.Config(module, algebra)` pyclass |
| `LogWriter`, `ext_tracing_subscriber` | internal (§8) |

### 7.8 `ext::save`

Save-directory plumbing (`SaveDirectory`, `SaveKind`, …). Bound minimally: a `SaveDirectory` pyclass
constructible from a path string (`SaveDirectory(path)`), accepted anywhere a `save_dir` argument
appears. The rest of `save` is internal serialization detail (§8).

---

## 8. Deliberately not bound

These `pub` items are excluded, with reason — the only carve-outs from rule (1):

- **Monomorphization-collapsed types:** `Prime` trait, `P2/P3/P5/P7`, `Fp<P>` for static `P`,
  `MultiDegree<N>`/`MultiDegreeElement<N>`/`MultiDegreeGenerator<N>` for `N≠2`, the `Mu*` generic
  forms. Their bound `ValidPrime` / `Bidegree` / `Resolution` instantiations stand in for them (§1).
- **Performance workspaces:** `PPartAllocation`, `PPartMultiplier`, BLAS `fast_mul_*`/`blas`. Hidden
  behind the operations that use them.
- **Closures returned from Rust:** the name-lookup closure from `module_gens_from_json`, the
  `strategy`/`callback`/`header` higher-order *returns*. Higher-order *arguments* are bound (Python
  callables); returned Rust closures cannot be wrapped thinly and are dropped.
- **Iteration/ordering helpers:** `coordinates::ordered::*`, `StemIterator` internals,
  `BiVec`/`OnceVec`/`MultiIndexed` (these surface only as Python lists/iterators).
- **Threading & serialization internals:** `SenderData`, `LogWriter`, `ext_tracing_subscriber`, the
  byte-level `save` machinery beyond `SaveDirectory`.

Anything in this list that a user later needs is promoted by exposing the capability upstream, not by
thickening the binding.

---

## 9. Reconciling the examples

The example scripts in `examples/` were drafted against an aspirational API; several names do not
exist in Rust. The bindings above follow Rust, so the examples must be adjusted:

- **`massey.py`** references `MasseyProductComputer` / `compute_massey_product`. No such type exists.
  The real computation (see `examples/massey.rs`) is assembled from `get_unit`,
  `ResolutionHomomorphism::{new, from_class, extend_through_stem, act}`, `ChainHomotopy::{new,
  extend, homotopy}`, and `AugmentedMatrix` row-reduction. All of those primitives are bound (§7.1,
  §7.3, §4.4), so `massey.py` is **rewritten in Python to compose them**, line-for-line mirroring
  `examples/massey.rs`. This is the general policy: multi-step workflows that exceed a single binding
  call live as Python example scripts (like everything in `examples/`), not as thick bindings or new
  upstream helpers. The binding layer stays at 1–3 lines per `pub` item; orchestration is Python.
- **`chart.py`** matches Rust once `write_to_graph` and `SvgBackend` are bound as in §6.2–§6.3
  (`products` is an iterator of `(name, Product)` pairs).
- **`define_module.py`** uses `module.to_json()`, `algebra.generators`, `algebra.basis_element_to_string`,
  `SteenrodEvaluator` — all bound (§5).
- **`tensor.py`** uses `parse_module_name`, `steenrod_module_from_json`, `TensorModule`,
  `FDModuleBuilder.from_module` — all bound (§5.3, §7.7).
- **`secondary.py`** uses `SecondaryResolution`, `compute_partial`, `extend_all`, `homotopy`,
  `underlying`, `iter_nonzero_stem`, `BidegreeGenerator` — all bound (§7.4).

`unstable_chart.py`, `resolve.py`, `steenrod.py`, `algebra_dim.py` map directly onto §7.2, §7.5,
§5.1.

---

## 10. Implementation priority

The bindings are independent and mechanical, so order is driven by example coverage:

1. `fp`: `ValidPrime`, `FpVector`, `Matrix`, `Subspace` (foundation for everything).
2. `algebra`: `SteenrodAlgebra`, `MilnorAlgebra`, `AdemAlgebra`, the `Algebra` method set,
   `SteenrodModule` + `FDModuleBuilder`/`FreeModule`/`TensorModule`, `steenrod_module_from_json`.
3. `sseq`: `Bidegree`, `BidegreeGenerator`, `BidegreeElement`, `Sseq`, `SvgBackend`.
4. `ext` top level: `Resolution`, `construct`/`query_module*`, `ChainComplex` method set,
   `to_sseq`, `filtration_one_products`.
5. `ResolutionHomomorphism`, `ChainHomotopy`, `get_unit`, `yoneda_representative_element`.
6. `SecondaryResolution` and the secondary family; `UnstableResolution`; `NassauResolution`.
7. The Steenrod evaluator/parser, finitely-presented modules, remaining module constructions.
8. The long tail of `Algebra`/`Module`/`Matrix` queries needed only for completeness (rule 1).

Each item is one wrapper of the shape in §2; "done" means the corresponding `pub` Rust item has a
Python name that unwraps, calls, and wraps.
