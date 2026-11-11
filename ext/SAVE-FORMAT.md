# Save format

A save directory is one [Zarr v3](https://zarr-specs.readthedocs.io/en/latest/v3/core/v3.0.html)
store. Integers are little-endian, and an `FpVector` is serialized as its limbs (`u64`s).

## Layout

```text
save_dir/
  zarr.json                          # root group: identity attributes (below)
  {kind}/                            # shard-tier array of one kind
  qi/n{n}_s{s}/{res_qi,nassau_qi}/   # stream-tier group of one quasi-inverse
  products/{name}/                   # a named ResolutionHomomorphism
    chain_map/, secondary_composite/, secondary_intermediate/, secondary_homotopy/
  homotopies/{left}__{right}/        # a ChainHomotopy between two named maps
    chain_homotopy/, secondary_composite/, secondary_intermediate/, secondary_homotopy/
```

The root group's attributes identify the complex: `algebra_magic`, `prime` and `algebra_prefix`
bind the algebra, `module_spec` binds the module (the JSON it was built from), and `complex_name`
is a label. Opening a store for a different algebra or module is an error.

Each file is written to a temporary `{key}.{pid}.{n}.tmp` beside it and renamed into place, so an
interrupted write leaves the old file intact. Nothing reads the temporary files, and one left by a
crash can be deleted.

## Shard tier

Each kind is one `vlen-bytes` array with one element per bidegree, indexed `[n - N_MIN, s]`
(`N_MIN` is the most negative stem; stems can be negative), or per generator, indexed
`[n - N_MIN, s, idx]`. The shape grows as needed to cover every element. Elements are grouped
into shards with a CRC32C each. An empty element means nothing is saved there.

| kind | per | payload |
|---|---|---|
| `differential` | bidegree | `bitcode` of `{num_gens, target_res_dimension, target_cc_dimension, differentials, chain_maps}` |
| `kernel` | bidegree | `bitcode` of the `Subspace`, in reduced row echelon form |
| `augmentation_qi` | bidegree | `bitcode` of the quasi-inverse of the augmentation |
| `nassau_differential` | bidegree | `num_new_gens: u64, target_dim: u64`, then `d` of each new generator |
| `chain_map`, `chain_homotopy` | bidegree | the value on each generator of the source |
| `secondary_composite` | generator | `SecondaryComposite::to_bytes` |
| `secondary_intermediate` | generator | `len: u64`, then the intermediate vector |
| `secondary_homotopy` | bidegree | the value on each generator |

A `kernel` is deleted once the next filtration is computed.

## Stream tier

Quasi-inverses are too large to hold whole, so each is a zstd-compressed group read and written a
chunk at a time. Its `finished` attribute is set last; a group without it is treated as missing.
Every chunk is stored, even one equal to the fill value, so a finished group missing a chunk is
corrupt.

`res_qi` (the quasi-inverse of a standard resolution's differential): attributes `source_dim`,
`target_dim` and `image_dim`; `pivots`, an `i64` array of length `target_dim`; and `rows`, a `u8`
array of shape `[image_dim, 8 * limbs]` holding the lift of each pivot.

`nassau_qi` (the quasi-inverse in Nassau's algorithm): attributes `target_dim`, `zero_mask_dim`
(the masked dimension under the zero signature), `subalgebra_profile` and `num_commands`, and a
`vlen-bytes` array `commands` with one command per element. The commands drive a state machine
whose state is the current signature, starting at zero:

- a pivot column `col >= 0` as `i64`, then the lift's byte length as `u32`, the lift and the
  image. The pivot and the image are in the original basis, the lift in the masked basis of the
  current signature.
- `-2`, then the new signature as one `u16` per profile entry.
- `-3`: apply the differential fix. When resolving through a stem, a quasi-inverse is computed
  before the generators of its source bidegree, so it misses them. They have zero signature, so
  the fix comes at the end of the zero-signature commands.
