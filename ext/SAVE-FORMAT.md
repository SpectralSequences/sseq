This documents the save file structure for resolutions. Everything is stored in
little endian.

Each save file starts with a header 32-byte header, of the form

```
[MAGIC: u32][prime: u32][s: u32][t: i32]
```

The MAGIC specifies the data contained in this file, and the rest specify the prime and the bidegree this pertains to.

# Differentials
The `MAGIC` is `0xD1FF0000`. The header is followed by
```
[number of generators: u64][length of each vector: u64][length of augmentation target: u64]
[image of first generator]
[image of second generator]
...
[image of last generator]
[augmentation image of first generator]
...
[augmentation image of last generator]
```

The images are stored as the limbs of the corresponding `FpVector`.

# Quasi-inverses
The `MAGIC` of every quasi-inverse is of the form `0x0100XXXX`, where the last four digits depend on what the quasi-inverse is associated to. For quasi-inverses to the differential, it it is `0x0100D1FF`, while for the augmentation it is `0x0100A000`. The header is followed by
```
[source dimension: u64][target dimension: u64][image dimension: u64]
[pivot columns of the image: [i64; target_dimension]]
[lift of first basis element of the image]
[lift of second basis element of the image]
...
[lift of last basis element of the image]
```

# Subspaces
The kernel of the differential has `MAGIC` given by `0x0000D1FF`
```
[rows: u64][ambient_dimension: u64]
[first basis element of subspace]
[second basis element of subspace]
...
[last basis element of subspace]
[pivot columns: [i64; ambient_dimension]]
```
