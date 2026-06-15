# sseq_ext — Python bindings for `ext`

This crate exposes a minimal Python API over [`ext`](../ext/), the Rust library
for computing Ext over the Steenrod algebra.

The current scope mirrors a representative handful of the `ext/examples/*.rs`
example binaries; see `examples/` for the corresponding Python translations:

| Rust example                       | Python translation              |
| ---------------------------------- | ------------------------------- |
| `ext/examples/algebra_dim.rs`      | `examples/algebra_dim.py`       |
| `ext/examples/resolve.rs`          | `examples/resolve.py`           |
| `ext/examples/resolve_through_stem.rs` | `examples/resolve_through_stem.py` |
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

This crate is built with [maturin](https://www.maturin.rs/). It produces an
`abi3` extension compatible with CPython >= 3.10. Using
[`uv`](https://docs.astral.sh/uv/):

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

## WebAssembly / Pyodide build

The crate can also be cross-compiled to a WebAssembly wheel that loads in
[Pyodide](https://pyodide.org/) (CPython in the browser). This produces a
`wasm32-unknown-emscripten` `abi3` extension module.

Unlike the native build, the wasm build drops the `concurrent` (rayon/threads)
feature — default Pyodide has no threads — and the dev-only `test-hooks`
feature.

### Requirements

- The emscripten SDK at the **exact** version the target Pyodide release was
  built with. Mismatched versions produce a wheel that fails to load.
- `pyodide-build` (installed into the venv) and the matching Pyodide
  cross-build environment.

The versions are pinned in `build-pyodide.sh`:

| Pyodide   | CPython | Emscripten |
| --------- | ------- | ---------- |
| `314.0.0` | 3.14.2  | `5.0.3`    |

### Build

```sh
uv pip install pyodide-build        # one-time, into the .venv
source .venv/bin/activate
./build-pyodide.sh
```

The script clones/activates a local emscripten SDK under `.emsdk/`
(gitignored), installs the Pyodide cross-build environment, and runs
`pyodide build`. The resulting wheel lands in `dist/`:

```
dist/sseq_ext-0.1.0-cp310-abi3-emscripten_5_0_3_wasm32.whl
```

To build manually, the key steps are:

```sh
source .emsdk/emsdk_env.sh
pyodide xbuildenv install 314.0.0
MATURIN_PEP517_ARGS="--no-default-features --features pyo3/extension-module" \
    pyodide build
```

### Loading in Pyodide

The wheel can be installed with `micropip` (e.g. serve it over HTTP):

```python
import micropip
await micropip.install("https://.../sseq_ext-0.1.0-cp310-abi3-emscripten_5_0_3_wasm32.whl")
import sseq_ext
```

The test suite covers:

- `tests/test_examples.py` — end-to-end smoke tests for each of the example
  scripts (with exact-output assertions where the computation is small and
  deterministic).
- `tests/test_views.py` — basic correctness of the `FpVector` view system
  (slicing, owned/view/view-mut transitions, composition).
- `tests/test_coordinates.py` — `Bidegree` / `BidegreeGenerator` /
  `BidegreeElement` behaviour.
- `tests/test_api_fixes.py` — negative indexing, getter consistency
  (`prime`/`name`), typed exceptions in place of panics, and coverage of the
  `Subspace` and coordinate types.
- `tests/test_view_safety.py` — exhaustive safety tests:
  - Slice arithmetic & out-of-bounds handling (including out-of-range and
    reversed `AugmentedMatrix` segment keys, and 3-segment matrices).
  - Read-only enforcement (writes through a `View` raise).
  - Lifetime / GC (parent kept alive by view; cleaned up when both go).
  - Mutation visibility between parent and view.
  - Aliasing semantics for overlapping `ViewMut`s.
  - Re-entrancy: a Rust-side test hook (gated behind the `test-hooks` cargo
    feature, on by default for dev builds) holds `borrow_mut` on a `Matrix`
    and tries to write through a view; the runtime borrow check fires with
    `BufferError`. These tests are skipped if the extension was built
    without the feature.
  - Random-op stress tests for owned vectors, matrix row views, and
    overlapping slices, cross-checked against a Python-side snapshot.

Release wheels can exclude the test-only hooks by building with
`--no-default-features --features pyo3/extension-module`.

## Design notes

### Scope

- The interactive `query` crate is **not** bound. Examples use Python idioms
  (`sys.argv`, `argparse`, `input()`) instead.
- Built with the default `odd-primes` feature. The native build also enables
  `concurrent` (rayon threads) by default; it is exposed as a `sseq_ext` cargo
  feature so it can be dropped for the threadless WebAssembly/Pyodide build
  (`--no-default-features`). The `nassau` feature is **not** enabled, so
  `QueryModuleResolution` is `Resolution<CCC>`.
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
| `am.segment_const[seg]`             | matrix-like read-only view of one segment   |
| `am.segment_mut[seg]`               | matrix-like mutable view of one segment     |
| `am.segment_mut[seg].add_identity()`| write identity into a (square) segment      |

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
