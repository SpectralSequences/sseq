# Save file format

This file documents how we save data pertaining to a resolution. We let `/`
denote the base directory containing the data of a single resolution.

All integers are stored in little-endian.

## Design philosophy

We have the following goals:

- Modularity: Each piece of data pertinent to each bidegree is contained in
   its own file. This gives the possibility of sharding large workloads and
   only loading the relevant data for each computation.

- Zero-copy deserialization: For large data structures (e.g. quasi-inverses),
   it should be possible to `mmap` the save file and use the pointer directly.
   In particular, the data should be 64 bit-aligned. This use case ties in with
   modularity, since each such data structure should be contained in exactly
   one file in order to do RAII properly.

   Of course, we only get true zero-copy deserializaion on little-endian 64-bit
   machines.

- Incrementality: We should be able to save data incrementally in case the
   program crashes, and resume old computations to push them further without
   too much additional cost.

- Robustness: It should be easy to detect and correct for data corruption,
   possibly due to the program crashing halfway through a write. Here "correct"
   would largely mean deleting the corrupted file and recomputing the data, but
   modularity helps with minimizing the amount we need to recompute.

- Space and time efficiency: We should be able to compress the saved data to
   limit disk usage. If we run this over a cluster, we would be reading files
   over the network, and space efficiency would be even more important. On the
   other hand, decompression should not be too slow either. This is necessarily
   at odds with zero-copy deserialization, and the user should be able to
   choose whether or not to compress.

## Overview

In general, we have many "kinds" of data associated to a resolution, such as
the differential itself and the quasi-inverses. Each kind of data has a name,
and the data for the bidegree `(s, t)` is contained in
`/{name}s/{s}_{t}_{name}`.

(In some cases, we want to store the data by bidegree *and* generator. Then the
file name is `/{name}s/{s}_{t}_{idx}_{name}`.)

We support reading compressed files. At the moment, we only support zstd, and
the compressed file should be saved in `/{name}s/{s}_{t}_{name}.zst`. When
seeking saved data, we always try the uncompressed version first, as it tends
to be faster.

(when dealing with compressed files, the `_{name}` suffix makes it easier to
wildcard match uncompressed files via `/{name}s/*_{name}`)

In general, we always write uncompressed data files, and the compression can be
performed with other programs separately. This should not pose storage
problems, since the raw data has to be small enough to be held in memory prior
to being written.

### File headers

In addition to a name, each kind of data has a 4-byte magic number to ensure we
do not mix them up. Further, since it is easy to forget to specify which
algebra to use, we have a 4-byte magic number for each algebra. The two least
significant bytes are given as follows:

```text
adem algebra: 0x0000
unstable adem algebra: 0x0001
milnor algebra without profile: 0x8000
milnor algebra without profile: 0x8000
milnor algebra with profile: 0x8001
```

The two most significant bytes are given by the prime the algebra is over.

Each data file starts with a 16-byte header of the form

```text
struct {
    magic: u32,
    algebra_magic: u32,
    s: u32,
    last: union {
        t: i32,
        (t, idx): (i16, u16)
    }
}
```

Here if the data is for a bidegree, then the last field is the topological
degree. Otherwise, it contains both the topological degree and the index of the
generator. Note that since we use little endian ordering, the last field is
`t + (idx << 16)`.

Apart from the prime, all the other data is also present in the file name, and
the header serves as a sanity check that we did not mess up our files. Note
that it is important that the length of the header is a multiple of 64 bits, so
that the remaining data is 64 bit-aligned.

### File footer

At the end of the file, an adler32 checksum of the contents (including the
header) is appended as a u32.

## Data types

### Differentials

This has magic `0xD1FF0000`.

This contains the differential in the resolution as well as the augmentation
map. There is not much reason to put these two together apart from the fact
that they are both quite small and are useful for most use cases.

For each bidegree, we store the outputs of the maps on the free module
generators. In particular, this data file records the number of generators in
each bidegree.

Let `x_1, ..., x_n` be the generators and `d, ε` be the differential and
augmentation map. The format of the save file is then as follows:

```text
struct {
    num_gens: u64,
    len_of_d(x_i): u64,
    len_of_ε(x_i): u64,
    d(x_i): [[u64; num_limbs(len_of_d(x_i))]; num_gens]
    ε(x_i): [[u64; num_limbs(len_of_d(x_i))]; num_gens]
}
```

The length is the vector space dimension of the target, while the vectors
themselves are stored as the limbs of the FpVectors. This will be the case for
all other data types.

Note that the length of the value of the differential may be shorter than the
dimension of the target if we use `compute_through_stem`, since we do not yet
know how many new generators will be added.

### Subspaces

After computing the differential in bidegree `(s, t)`, we save the kernel of
the differrential, which will be used when we compute `(s + 1, t)`. We then
delete the saved file after we are done with computing `(s + 1, t)`. This has
magic `0x0000D1FF`.

In general, given any subspace, we can choose a canonical basis by taking a
matrix of spanning (row) vectors and putting it in reduced row echelon form. We
will always use this basis when dealing with subspaces of a vector space.

Having found such a basis, we have a vector of pivots, whose length is the
dimension of the ambient space. The kth entry of this vector is index of the
basis element whose pivot column is `k`, and `-1` if there is no such basis
element. For example, if the basis is

```text
[1, 1, 0, 1, 0, 0]
[0, 0, 1, 0, 0, 0]
[0, 0, 0, 0, 1, 0]
[0, 0, 0, 0, 0, 1]
```

then the pivot is

```text
[0, -1, 1, -1, 2, 3]
```

The format of a subspace is as follows:

```text
struct {
    subspace_dimension: u64,
    total_space_dimension: u64,
    basis_vectors: [[u64; num_limbs(total_space_dimension)]; subspace_dimension]
    pivots: [i64; total_space_dimension]
}
```

As usual, the basis elements of a subspace are chosen to be in row reduced
echelon form.

### Quasi-inverses

We also store the quasi-inverses to the differential and the augmentation map.
The associated magics are `0x0100D1FF` and `0x0100A000` respectively.

The format of a quasi-inverse is as follows:

```text
struct {
    source_dimension: u64,
    target_dimension: u64,
    image_dimension: u64,
    pivots_of_image: [i64; target_dimension],
    lift_of_basis_elements: [[u64; num_limbs(source_dimension)]; image_dimension]
}
```

Note that it is not necessary to know what the image is, as long as the user of
the quasi-inverse guarantees their vector is in the image. This is due to the
fact that our basis is in *reduced* row ecehlon form.

### Nassau's algorithm

We save data for Nassau's algorithm differently.

The differential is the same except we don't store the augmentation data, since
we always resolve the sphere.

The quasi-inverse is stored in a custom format. One can interpret the saved
data as bytecode for a state machine that computes the quasi-inverse, whose
state is the signature we are currently working on.

The file starts with a header, which is given by

```text
struct {
    target_dimension: u64,
    target_masked_dimension: u64,
    subalgebra_profile_length: u64,
    subalgebra_profile: [u8; subalgebra_profile_length],
}
```

where the masked dimension is the mask under the zero signature. This (and
target_dimension) is needed because we might have computed the quasi-inverse
using incomplete information if resolving up to a stem.

The header is followed by a series of commands. The starting state of the state
machine is the zero signature. Each command of this state machine starts with a
u64, which is to be interpreted as follows:

- (-1) indicates the end of the program. There should be no bytes after this
   program (apart from checksums).

- (-2) indicates a change of signature. We should read in a
   `[u16; subalgebra_profile_length]` which will be the new signature.

- (-3) instructs the machine to perform a "differential fix" --- when
   resolving up to a stem, at the boundary, we compute the quasi-inverse before
   computing the source itself. This means the quasi-inverse is missing the
   parts that come from the new generator.

   Since a generator has zero signature, this only affects zero signature part,
   and this command is encountered at the end of the instructions for the zero
   signature. Note that data has to be collected for the fix before this
   command is encountered. The machine must detect beforehand whether the fix
   is needed. This instruction merely indicates *when* the fix is to be
   performed.

- Any other number is a pivot column. The upcoming data gives an element that
   hits this pivot column and the image of this element under the differential.
   The pivot column and the image are expressed in terms of the original basis,
   while the lift is expressed in terms of the masked basis under the current
   signature. The latter measure is done in order to save space.
