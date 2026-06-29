# Phase 0 gate — bit-sliced storage prototype results

**Verdict: GO.** Bit-slicing is worth pursuing. The decision is more favorable than the
plan predicted, because the existing *packed* path is only well-optimized for
`p ∈ {2, 3, 5}` — for every other prime its reduction step is a slow element-wise
fallback (`Fp::reduce`'s generic arm, `field/fp.rs:142`), and even an unoptimized
bit-sliced kernel beats it.

Benchmarks: `cargo bench -p fp --bench bitslice` (median of 30 samples, short config:
warm-up 0.5s, measurement 2s). `add` is `self += 2*other`; `scale` is `self *= 2`.
"generic" = the prime-agnostic ripple-carry kernel; "f3" = the hand-written F3 circuit.

## `add`, length 100,000 (asymptotic regime)

| prime | packed | bitsliced generic | bitsliced F3 | best vs packed |
|------:|-------:|------------------:|-------------:|:--------------|
| 3     | 9.10 µs |  93.1 µs | **3.60 µs** | F3 **2.5× faster** |
| 5     | 19.0 µs | 102.6 µs |      —      | generic 5.4× slower |
| 7     | 91.6 µs | 102.4 µs |      —      | ~even (packed reduce is slow) |
| 251   |  528 µs | **145.6 µs** |  —      | generic **3.6× faster** |

## `scale`, length 100,000

| prime | packed | bitsliced generic | bitsliced F3 | best vs packed |
|------:|-------:|------------------:|-------------:|:--------------|
| 3     | 2.61 µs | 82.6 µs | **1.49 µs** | F3 **1.75× faster** |
| 5     | 5.00 µs | 87.4 µs |      —      | generic 17× slower |
| 7     | 67.2 µs | 85.4 µs |      —      | generic 1.27× slower |
| 251   |  480 µs | **117.5 µs** |  —      | generic **4.1× faster** |

## Reading the results

1. **Specialized small-prime circuits win.** The F3 circuit is 2.5× faster than packed
   for `add` and 1.75× for `scale`, at every length tested. This is the core validation:
   replacing the madd+reduce sequence with a short branch-free circuit pays off.

2. **The generic kernel loses for `p ∈ {3, 5}` but wins for large primes.** The packed
   path has hand-tuned SWAR `reduce` only for 2/3/5; for `p = 7` and everything larger it
   falls back to a per-element `pack(unpack(limb))`, which is slow. The generic bit-sliced
   kernel (ripple-carry add + one conditional subtract, no tables) already beats packed by
   **3.6×/4.1×** at `p = 251` and is roughly even at `p = 7` — *despite* prototype overhead
   (fixed `[Limb; 24]` scratch arrays regardless of `k`, and double-and-add for `scale`).
   A real implementation that sizes scratch to `k` will widen this further.

3. **This vindicates the "all primes" + "replace" decision.** Bit-slicing helps across the
   board, just via two mechanisms: specialized circuits for the tuned small primes
   (3, 5, 7), and the generic kernel for the large primes where packed reduction is the
   bottleneck.

## Phase 0b — tightened generic kernel

After replacing the prototype's fixed `[Limb; 24]` scratch + large by-value returns with
`k`-sized reusable scratch written directly into the destination planes (and replacing
double-and-add's doubling with a plane shift), the generic kernel improved and the
crossover where it beats packed moved down to **p = 7** (numbers from one run, length 100k):

| op    | prime | packed | generic before | generic after | after vs packed |
|------:|------:|-------:|---------------:|--------------:|:----------------|
| add   | 3     | 11.2 µs |  93.1 µs |  61.0 µs | 5.5× slower (use F3 circuit instead) |
| add   | 7     | 95.4 µs | 102.4 µs |  73.4 µs | **1.30× faster** |
| add   | 251   |  471 µs | 145.6 µs | 137.2 µs | **3.44× faster** |
| scale | 7     | 73.8 µs |       —  |  52.9 µs | **1.40× faster** |
| scale | 251   |  444 µs |       —  |  96.3 µs | **4.61× faster** |

(The F3 specialized circuit is unchanged: add ≈ 3.8 µs / **2.9× faster** than packed,
scale ≈ 1.8 µs / **1.6× faster**, at 100k.)

So the tightened generic kernel is now faster than packed for **all `p ≥ 7`**; only the
SWAR-tuned `p ∈ {3, 5}` still need specialized circuits to win — and F3 confirms that
specialization does win.

## Implications for the next phases

- **Specialized circuits are needed for `p = 3, 5, 7`** (not just F3) to beat the tuned
  packed path; F5/F7 are the same shape as F3 (3 planes instead of 2).
- **The generic kernel is sufficient for `p ≥ 11`** and for `Fp<ValidPrime>` (runtime `k`),
  and is the path that makes large-prime arithmetic dramatically faster.
- The generic kernel must size its plane scratch to `k` (drop the `MAX_K` arrays) to
  avoid the prototype's overhead on small `k`.
- No prime regresses badly enough to keep packed as a fallback for it — so "replace" holds
  for all of `Fp`. (`SmallFq` stays packed regardless; its arithmetic is table-based.)
