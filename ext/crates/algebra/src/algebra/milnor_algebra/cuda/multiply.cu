// The batched Milnor multiply at p = 2: the anti-diagonal walk of `antidiagonal.rs` on the device.
//
// Every (product, term) pair is one ITEM: multiply P(R) by the basis element P(S) and XOR each
// term P(T) of the product into the output, at the column its index in the Milnor basis gives.
//
// Constants arrive as NVRTC -D options from `mod.rs`; none has a default here, so a forgotten one
// is a compile error rather than a wrong value.

#ifndef PPART_MAX_LEN
#error "PPART_MAX_LEN must be provided as an NVRTC -D option"
#endif
#ifndef PP_SHIFTS
#error "PP_SHIFTS must be provided as an NVRTC -D option"
#endif
#ifndef PP_MASKS
#error "PP_MASKS must be provided as an NVRTC -D option"
#endif
#ifndef NT
#error "NT (threads per block) must be provided as an NVRTC -D option"
#endif

typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long long u64;

// Mirrors `COL_MAP_DROP`.
#define COL_MAP_DROP 0xffffffffu

// Where entry i of a packed p-part sits, straight from `PPart`'s layout.
__device__ static const u32 PP_SHIFT[PPART_MAX_LEN] = PP_SHIFTS;
__device__ static const u32 PP_MASK[PPART_MAX_LEN] = PP_MASKS;

// The index of P(t) in the Milnor basis of its degree: `SeqnoTables::rank`. `g` has row width
// `width`, entry (e, h) at g[e * width + h]; `xi` are the degrees of the xi_i.
__device__ __forceinline__ u32 seqno(const u32 *__restrict__ g, const u32 *__restrict__ xi, u64 t,
                                     u32 width) {
    u32 cur_d = 0;
    for (u32 h = 0; h < PPART_MAX_LEN; ++h) cur_d += (u32)((t >> PP_SHIFT[h]) & PP_MASK[h]) * xi[h];
    u32 rank = 0;
    for (u32 h = PPART_MAX_LEN - 1; h >= 1; --h) {
        u32 r = (u32)((t >> PP_SHIFT[h]) & PP_MASK[h]);
        if (r != 0) {
            u32 below = cur_d - r * xi[h];
            rank += g[(u64)cur_d * width + h] - g[(u64)below * width + h];
            cur_d = below;
        }
    }
    return rank;
}

// XOR the term P(t) into its row: `output_bit` in `batch.rs`.
__device__ __forceinline__ void emit(const u32 *__restrict__ g, const u32 *__restrict__ xi,
                                     u32 *out, const u32 *__restrict__ col_map, u32 col_map_len,
                                     u32 use_col_map, u64 t, u32 out_offset, u32 row_base,
                                     u32 width, u32 num_limbs, u64 out_len) {
    u64 bit = (u64)out_offset + seqno(g, xi, t, width);
    if (use_col_map) {
        if (bit >= (u64)col_map_len) return;
        u32 mapped = col_map[bit];
        if (mapped == COL_MAP_DROP) return;
        bit = mapped;
    }
    // Both bounds are needed: the first keeps the write inside this row, the second inside the
    // buffer.
    u64 limb = bit / 32;
    if (limb >= (u64)num_limbs) return;
    u64 word = (u64)row_base + limb;
    if (word >= out_len) return;
    atomicXor(&out[word], 1u << (u32)(bit % 32));
}

// THE WALK. Interior cells x_ij (i, j >= 1) are filled one anti-diagonal d = i + j at a time, from
// the last (max_diag) down to 2, rows ascending within a diagonal. On reaching diagonal d the other
// entries of row d and column d are fixed, so the slacks x_d0 = rl[d] and x_0d = sl[d] are known:
// the diagonal starts from their OR, and each cell only takes values disjoint from the bits used so
// far on its diagonal. Every completed matrix contributes.
//
// One trip of the loop is one step: ENTER a cell (value 0), or one BACKTRACK attempt (the next
// disjoint value of the current cell, or give it up and step back one cell). After an enter or a
// successful increment the trip also ADVANCES: to the next cell, or, the diagonal being complete,
// check the slacks of d - 1 and start diagonal d - 1, emit (d == 2), or backtrack.
//
// The state is per thread, u16 in shared memory with stride NT so the lanes of a warp hit
// consecutive halfwords: val and bef (the bits used on the cell's diagonal before it) per interior
// cell, N(N-1)/2 of them, and rl, sl (what is left of R's rows and S's columns, indexed from 1).
//
// Accepted terms are queued per warp and emitted 32 at a time, so the atomics of a warp issue
// together rather than one lane at a time.

extern __shared__ u16 smem[];

#define QCAP 64

// Bytes of shared memory the walk state of a block takes (16-byte aligned); the queue follows.
// Mirrored by `shared_bytes` in `mod.rs`.
#define STATE_U16(N) ((N) * ((N) - 1) + 2 * ((N) + 1))
#define STATE_BYTES(N) (((STATE_U16(N) * 2u * NT) + 15u) & ~15u)

template <int N>
__device__ __forceinline__ void antidiag_body(
    const u64 *__restrict__ pp, const u32 *__restrict__ term_gei, const u32 *__restrict__ term_prod,
    const u64 *__restrict__ prod_r, const u32 *__restrict__ prod_row_base,
    const u32 *__restrict__ prod_out_offset, const u32 *__restrict__ g, const u32 *__restrict__ xi,
    u32 *out, const u32 *__restrict__ col_map, u32 col_map_len, u32 use_col_map, u32 num_items,
    u32 *counter, u32 width, u32 num_limbs, u64 out_len) {
    constexpr u32 CELLS = N * (N - 1) / 2;
    u16 *base = smem + threadIdx.x;
// Cell (i, j), i + j = d: diagonals 2..d-1 hold 1 + 2 + ... + (d - 2) cells before it.
#define CELL(d, i) (((d) - 2) * ((d) - 1) / 2 + (i) - 1)
#define VAL(c) base[(c) * NT]
#define BEF(c) base[(CELLS + (c)) * NT]
#define RL(k) base[(2 * CELLS + (k)) * NT]
#define SL(k) base[(2 * CELLS + N + 1 + (k)) * NT]
// First and last interior row on diagonal d.
#define LO(d) max(1, (int)(d) - (int)cols)
#define HI(d) min((int)rows, (int)(d) - 1)

    const u32 lane = threadIdx.x & 31u;
    const u32 nwarps = NT >> 5;
    unsigned char *qbase = (unsigned char *)smem + STATE_BYTES(N);
    u64 *wq_t = (u64 *)qbase + (threadIdx.x >> 5) * QCAP;
    u32 *wq_rb = (u32 *)(qbase + nwarps * QCAP * 8u) + (threadIdx.x >> 5) * QCAP;
    u32 *wq_oo = (u32 *)(qbase + nwarps * QCAP * 12u) + (threadIdx.x >> 5) * QCAP;
    volatile u32 *wq_cnt = (u32 *)(qbase + nwarps * QCAP * 16u) + (threadIdx.x >> 5);
    if (lane == 0) *wq_cnt = 0;
    __syncwarp();
#define QUEUE(t_)                                                                                 \
    {                                                                                             \
        u32 s_ = atomicAdd((u32 *)wq_cnt, 1u);                                                    \
        wq_t[s_] = (t_);                                                                          \
        wq_rb[s_] = row_base;                                                                     \
        wq_oo[s_] = out_offset;                                                                   \
    }

    // Persistent: each lane takes its next item from a global counter when it finishes one.
    u32 item = atomicAdd(counter, 1u);
    bool live = false;
    bool back = false; // backtracking (else entering)
    int d = 0, i = 0, max_diag = 0;
    u32 rows = 0, cols = 0, tally = 0;
    u64 t_bits = 0;
    u32 row_base = 0, out_offset = 0;

    for (;;) {
        bool have = live || item < num_items;
        if (!__any_sync(0xffffffffu, have)) break;
        if (!live) {
            if (have) {
                // ---- INIT
                u32 p = term_prod[item];
                u64 rbits = prod_r[p];
                u64 sbits = pp[term_gei[item]];
                row_base = prod_row_base[p];
                out_offset = prod_out_offset[p];
                item = atomicAdd(counter, 1u);
                // Lengths and the product's degree, sum of e_k (2^(k+1) - 1) over both factors.
                rows = 0;
                cols = 0;
                u32 degree = 0;
#pragma unroll
                for (u32 k = 0; k < N; ++k) {
                    u32 rv = (u32)((rbits >> PP_SHIFT[k]) & PP_MASK[k]);
                    u32 sv = (u32)((sbits >> PP_SHIFT[k]) & PP_MASK[k]);
                    RL(k + 1) = (u16)rv;
                    SL(k + 1) = (u16)sv;
                    if (rv) rows = k + 1;
                    if (sv) cols = k + 1;
                    degree += (rv + sv) * ((2u << k) - 1u);
                }
                if (rows == 0 || cols == 0) {
                    QUEUE(rows == 0 ? sbits : rbits);
                } else {
                    int md = min((int)(rows + cols), N);
                    while (md > 1 && (1u << md) - 1u > degree) --md;
                    max_diag = md;
                    u32 a = RL(md), b = SL(md);
                    if ((a & b) == 0) {
                        if (md == 1) {
                            QUEUE((u64)(a | b) << PP_SHIFT[0]);
                        } else {
                            d = md;
                            i = LO(d);
                            tally = a | b;
                            t_bits = 0;
                            back = false;
                            live = true;
                        }
                    }
                }
            }
        } else {
            // ---- STEP
            int j = d - i;
            u32 c = CELL(d, i);
            bool adv = false;
            u32 used = 0;
            if (!back) {
                // ENTER: value 0 never overlaps.
                BEF(c) = (u16)tally;
                VAL(c) = 0;
                used = tally;
                adv = true;
            } else {
                // BACKTRACK: the next value of this cell disjoint from the bits before it, if it
                // still fits what is left of row i and column j; else give the cell up.
                u32 v = VAL(c);
                u32 mask = BEF(c);
                u32 next = ((v | mask) + 1u) & ~mask;
                u32 inc = next - v;
                u32 rli = RL(i), slj = SL(j);
                if ((inc << j) <= rli && inc <= slj) {
                    RL(i) = (u16)(rli - (inc << j));
                    SL(j) = (u16)(slj - inc);
                    VAL(c) = (u16)next;
                    used = mask | next;
                    adv = true;
                    back = false;
                } else {
                    RL(i) = (u16)(rli + (v << j));
                    SL(j) = (u16)(slj + v);
                    if (i > LO(d)) {
                        --i;
                    } else if (d < max_diag) {
                        ++d;
                        i = HI(d);
                    } else {
                        live = false;
                    }
                }
            }
            if (adv) {
                if (i < HI(d)) {
                    // Next cell of this diagonal, entering with the bits used so far.
                    ++i;
                    tally = used;
                } else {
                    // Diagonal d is complete: T_d = used, and the slacks of d - 1 are known.
                    u32 sh = PP_SHIFT[d - 1];
                    t_bits = (t_bits & ~((u64)PP_MASK[d - 1] << sh)) | ((u64)used << sh);
                    u32 a = RL(d - 1), b = SL(d - 1);
                    if ((a & b) == 0) {
                        if (d == 2) {
                            QUEUE(t_bits | ((u64)(a | b) << PP_SHIFT[0]));
                            back = true;
                        } else {
                            --d;
                            i = LO(d);
                            tally = a | b;
                        }
                    } else {
                        back = true;
                    }
                }
            }
        }
        // Each trip queues at most one term per lane, so the queue holds at most 63 + 32 < QCAP
        // after a trip and is drained back below 32 here.
        __syncwarp();
        u32 cnt = *wq_cnt;
        if (cnt >= 32u) {
            emit(g, xi, out, col_map, col_map_len, use_col_map, wq_t[lane], wq_oo[lane],
                 wq_rb[lane], width, num_limbs, out_len);
            u32 rest = cnt - 32u;
            bool mv = lane < rest;
            u64 tt = 0;
            u32 rb = 0, oo = 0;
            if (mv) {
                tt = wq_t[32u + lane];
                rb = wq_rb[32u + lane];
                oo = wq_oo[32u + lane];
            }
            __syncwarp();
            if (mv) {
                wq_t[lane] = tt;
                wq_rb[lane] = rb;
                wq_oo[lane] = oo;
            }
            if (lane == 0) *wq_cnt = rest;
            __syncwarp();
        }
    }
    __syncwarp();
    {
        u32 cnt = *wq_cnt;
        if (lane < cnt)
            emit(g, xi, out, col_map, col_map_len, use_col_map, wq_t[lane], wq_oo[lane],
                 wq_rb[lane], width, num_limbs, out_len);
    }
#undef QUEUE
#undef CELL
#undef VAL
#undef BEF
#undef RL
#undef SL
#undef LO
#undef HI
}

// One entry point per walk width N: the products of degree in [2^N - 1, 2^(N+1) - 1), or below
// 2^7 - 1 for N = 6. See `width_for_degree`.
#define ANTIDIAG_ENTRY(NN)                                                                        \
    extern "C" __global__ void __launch_bounds__(NT) multiply_antidiag_##NN(                      \
        const u64 *__restrict__ pp, const u32 *__restrict__ term_gei,                              \
        const u32 *__restrict__ term_prod, const u64 *__restrict__ prod_r,                         \
        const u32 *__restrict__ prod_row_base, const u32 *__restrict__ prod_out_offset,            \
        const u32 *__restrict__ g, const u32 *__restrict__ xi, u32 *out,                           \
        const u32 *__restrict__ col_map, u32 col_map_len, u32 use_col_map, u32 num_items,          \
        u32 *counter, u32 width, u32 num_limbs, u64 out_len) {                                     \
        antidiag_body<NN>(pp, term_gei, term_prod, prod_r, prod_row_base, prod_out_offset, g, xi,  \
                          out, col_map, col_map_len, use_col_map, num_items, counter, width,       \
                          num_limbs, out_len);                                                     \
    }

ANTIDIAG_ENTRY(6)
ANTIDIAG_ENTRY(7)
ANTIDIAG_ENTRY(8)
ANTIDIAG_ENTRY(9)
ANTIDIAG_ENTRY(10)
