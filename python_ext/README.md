# sseq_ext — Python bindings for `ext`

This crate exposes a minimal Python API over [`ext`](../ext/), the Rust library
for computing Ext over the Steenrod algebra.

The current scope mirrors a representative handful of the `ext/examples/*.rs`
example binaries; see `examples/` for the corresponding Python translations:

| Rust example                       | Python translation              |
| ---------------------------------- | ------------------------------- |
| `ext/examples/algebra_dim.rs`      | `examples/algebra_dim.py`       |
| `ext/examples/resolve.rs`          | `examples/resolve.py`           |
| `ext/examples/num_gens.rs`         | `examples/num_gens.py`          |
| `ext/examples/resolution_size.rs`  | `examples/resolution_size.py`   |
| `ext/examples/differentials.rs`    | `examples/differentials.py`     |
| `ext/examples/filtration_one.rs`   | `examples/filtration_one.py`    |
| `ext/examples/chart.rs`            | `examples/chart.py`             |
| `ext/examples/secondary.rs`        | `examples/secondary.py`         |
| `ext/examples/massey.rs`           | `examples/massey.py`            |

Other examples (`steenrod.rs`, `sq0.rs`, `mahowald_invariant.rs`,
`bruner.rs`, `define_module.rs`, `secondary_massey.rs`, …) define their own
chain complexes inline or use APIs not yet bound. Adding them is a
straightforward extension of the existing scaffolding.

## Install

This crate is built with [maturin](https://www.maturin.rs/). Using
[`uv`](https://docs.astral.sh/uv/) and Python 3.14:

```sh
cd python_ext
uv sync                # creates a .venv with maturin and pytest, builds the extension
```

After that you can run the example translations:

```sh
uv run python examples/resolve.py S_2 30 15
uv run python examples/num_gens.py S_2 30 7
uv run python examples/chart.py S_2 30 7 --out chart.svg
uv run python examples/secondary.py S_2 30 7
uv run python examples/massey.py S_2 20 10  # interactive
```

To rebuild after editing the Rust sources:

```sh
uv run maturin develop --release
```

To run the tests:

```sh
uv run pytest
```

The test suite covers:

- `tests/test_examples.py` — end-to-end smoke tests for the five example
  scripts.
- `tests/test_views.py` — basic correctness of the `FpVector` view system
  (slicing, owned/view/view-mut transitions, composition).
- `tests/test_view_safety.py` — exhaustive safety tests:
  - Slice arithmetic & out-of-bounds handling.
  - Read-only enforcement (writes through a `View` raise).
  - Lifetime / GC (parent kept alive by view; cleaned up when both go).
  - Mutation visibility between parent and view.
  - Aliasing semantics for overlapping `ViewMut`s.
  - Re-entrancy: a Rust-side test hook holds `borrow_mut` on a `Matrix`
    and tries to write through a view; the runtime borrow check fires
    with `BufferError`.
  - Random-op stress tests for owned vectors, matrix row views, and
    overlapping slices, cross-checked against a Python-side snapshot.

## Design notes

### Scope

- The interactive `query` crate is **not** bound. Examples use Python idioms
  (`sys.argv`, `argparse`, `input()`) instead.
- Built with the default `odd-primes` feature plus `concurrent`. The `nassau`
  feature is **not** enabled, so `QueryModuleResolution` is `Resolution<CCC>`.
- We only bind the concrete instantiations needed by `ext::utils::construct`,
  i.e. `Resolution<CCC>` (for stable resolutions) and the matching
  `ResolutionHomomorphism`, `ChainHomotopy`, `SecondaryResolution`. Generic
  instantiations over other chain complexes are not exposed.
- All long-lived Rust objects (resolutions, modules, homomorphisms) are
  wrapped in `Arc<…>` and exposed as opaque Python handles. Mutable
  resources (matrices, vectors) are wrapped in plain `pyclass` objects with
  pyo3's borrow tracking.

### `FpVector` views

`FpVector` is a tagged union of three internal modes:

1. `Owned` — wraps an actual `fp::vector::FpVector`.
2. `View` — a read-only borrow into another object's storage.
3. `ViewMut` — a mutable borrow.

The user-facing API uses `.const` / `.mut` accessors and standard Python
indexing:

| What you write                      | What you get                                |
| ----------------------------------- | ------------------------------------------- |
| `v.const`                           | read-only view of the whole vector          |
| `v.mut`                             | mutable view of the whole vector            |
| `v.const[a:b]`, `v.mut[a:b]`        | read-only / mutable sub-view                |
| `view[a:b]`                         | sub-view (mutability inherited)             |
| `view[i]`                           | int entry (read; writes via `view[i] = …`)  |
| `m.const[row]`, `m.mut[row]`        | row view of a `Matrix`                      |
| `am.const[row, seg]`                | segment view of an `AugmentedMatrix`        |
| `am.mut[row, (start_seg, end_seg)]` | range of segments                           |

Slicing on a bare *owned* `FpVector` raises with a hint to use
`.const` / `.mut`. This avoids the ambiguity of whether `v[a:b]` should
yield a read-only or mutable view.

Views hold a reference-counted handle (`Py<…>`) to the parent so the parent
remains alive. Each operation on the view re-derives the underlying slice
transiently from the parent under a runtime borrow check (pyo3's
`try_borrow` / `try_borrow_mut`). If the parent is currently borrowed
elsewhere — e.g. you're in the middle of a method that takes `&mut self`
on the parent — the view operation raises `BufferError`.

This lets you write code like

```python
hom.act(matrix.mut[idx, 0], v, gen)
```

which directly mirrors the Rust idiom

```rust
hom.act(matrix.row_mut(idx).slice_mut(start, end), v, gen)
```

without exposing raw `FpSliceMut<'_>` lifetimes to Python.
