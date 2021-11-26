# Save file format

This file documents how we save data pertaining to a resolution. We let `/`
denote the base directory containing the data of a single resolution

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
do not mix them up. Each data file starts with a 16-byte header of the form

```
[MAGIC: u32][prime: u32][s: u32][t: i32]
```

Apart from the prime, all the other data is also present in the file name, and
the header serves as a sanity check that we did not mess up our files. Note
that it is important that the length of the header is a multiple of 64 bits, so
that the remaining data is 64 bit-aligned.

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

```
[number of generators: u64]
[length of d(x_i): u64]
[length of ε(x_i): u64]
[d(x_1)]
[d(x_2)]
...
[d(x_n)]
[ε(x_1)]
[ε(x_2)]
...
[ε(x_n)]
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
```
[1, 1, 0, 1, 0, 0]
[0, 0, 1, 0, 0, 0]
[0, 0, 0, 0, 1, 0]
[0, 0, 0, 0, 0, 1]
```
then the pivot is
```
[0, -1, 1, -1, 2, 3]
```

The format of a subspace is as follows:
```
[dimension of subspace: u64][ambient_dimension: u64]
[first basis element of subspace]
[second basis element of subspace]
...
[last basis element of subspace]
[pivots: [i64; ambient_dimension]]
```
As usual, the basis elements of a subspace are chosen to be in row reduced
echelon form.

### Quasi-inverses
We also store the quasi-inverses to the differential and the augmentation map.
The associated magics are `0x0100D1FF` and `0x0100A000` respectively.

The format of a quasi-inverse is as follows:
```
[source dimension: u64][target dimension: u64][image dimension: u64]
[pivots of the image: [i64; target_dimension]]
[lift of first basis element of the image]
[lift of second basis element of the image]
...
[lift of last basis element of the image]
```

Note that it is not necessary to know what the image is, as long as the user of
the quasi-inverse guarantees their vector is in the image. This is due to the
fact that our basis is in *reduced* row ecehlon form.
