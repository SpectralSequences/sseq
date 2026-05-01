# sseq_ext — Python bindings for `ext`

This crate exposes a minimal Python API over [`ext`](../ext/), the Rust library
for computing Ext over the Steenrod algebra.

The current scope mirrors a representative handful of the `ext/examples/*.rs`
example binaries; see `examples/` for the corresponding Python translations:

| Rust example                | Python translation        |
| --------------------------- | ------------------------- |
| `ext/examples/resolve.rs`   | `examples/resolve.py`     |
| `ext/examples/num_gens.rs`  | `examples/num_gens.py`    |
| `ext/examples/chart.rs`     | `examples/chart.py`       |
| `ext/examples/secondary.rs` | `examples/secondary.py`   |
| `ext/examples/massey.rs`    | `examples/massey.py`      |

Other examples (`steenrod.rs`, `sq0.rs`, `mahowald_invariant.rs`,
`bruner.rs`, `define_module.rs`, `secondary_massey.rs`, …) define their own
chain complexes inline or use APIs not yet bound. They will require
additional bindings.

## Install

This crate is built with [maturin](https://www.maturin.rs/). Using
[`uv`](https://docs.astral.sh/uv/) and Python 3.14:

```sh
cd python_ext
uv sync                        # creates a .venv with maturin and pytest
uv run maturin develop --release   # build & install the extension
```

After that you can run the example translations:

```sh
uv run python examples/resolve.py S_2 30 15
uv run python examples/num_gens.py S_2 30 7
uv run python examples/chart.py S_2 30 7 > chart.svg
uv run python examples/secondary.py S_2 30 7
uv run python examples/massey.py
```

## Design notes

- The interactive `query` crate is **not** bound. Examples use Python idioms
  (`sys.argv`, `argparse`, `input()`) instead.
- Built with the default `odd-primes` feature plus `concurrent`. The `nassau`
  feature is **not** enabled, so `QueryModuleResolution` is `Resolution<CCC>`.
- We only bind the concrete instantiation needed by `ext::utils::construct`,
  i.e. `Resolution<CCC>` (for stable resolutions) and the matching
  `ResolutionHomomorphism`, `ChainHomotopy`, `SecondaryResolution`. Generic
  instantiations over other chain complexes are not exposed.
- All long-lived Rust objects (resolutions, modules, homomorphisms) are
  wrapped in `Arc<…>` and exposed as opaque Python handles.
