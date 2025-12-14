use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use fp::{blas::tile::orders::*, matrix::Matrix, prime::TWO};
use pprof::criterion::{Output, PProfProfiler};
use rand::Rng;

fn muls(c: &mut Criterion) {
    for m in [64, 128, 256, 512, 1024] {
        for k in [64, 128, 256, 512, 1024] {
            for n in [64, 128, 256, 512, 1024] {
                bench_mkn(m, k, n, c);
            }
        }
    }
    bench_mkn(2048, 2048, 2048, c);
    bench_mkn(4096, 4096, 4096, c);
    bench_mkn(8192, 8192, 8192, c);
}

fn bench_mkn(m: usize, k: usize, n: usize, c: &mut Criterion) {
    let mut g = c.benchmark_group(format!("{m}x{k} * {k}x{n}"));
    g.throughput(criterion::Throughput::Elements((2 * m * k * n) as u64));
    g.bench_function("matmul_sequential", |b| {
        b.iter_batched(
            || random_matrix_pair(m, k, n),
            |(a, b)| a.fast_mul_sequential(&b),
            BatchSize::SmallInput,
        );
    });
    let concurrent_muls: [(fn(&Matrix, &Matrix) -> Matrix, &str); _] = [
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 1, CIR>,
            "1_1_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 2, CIR>,
            "1_2_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 4, CIR>,
            "1_4_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 8, CIR>,
            "1_8_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 16, CIR>,
            "1_16_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 1, CIR>,
            "2_1_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 2, CIR>,
            "2_2_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 4, CIR>,
            "2_4_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 8, CIR>,
            "2_8_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 16, CIR>,
            "2_16_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 1, CIR>,
            "4_1_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 2, CIR>,
            "4_2_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 4, CIR>,
            "4_4_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 8, CIR>,
            "4_8_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 16, CIR>,
            "4_16_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 1, CIR>,
            "8_1_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 2, CIR>,
            "8_2_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 4, CIR>,
            "8_4_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 8, CIR>,
            "8_8_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 16, CIR>,
            "8_16_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 1, CIR>,
            "16_1_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 2, CIR>,
            "16_2_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 4, CIR>,
            "16_4_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 8, CIR>,
            "16_8_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 16, CIR>,
            "16_16_cir",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 1, CRI>,
            "1_1_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 2, CRI>,
            "1_2_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 4, CRI>,
            "1_4_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 8, CRI>,
            "1_8_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 16, CRI>,
            "1_16_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 1, CRI>,
            "2_1_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 2, CRI>,
            "2_2_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 4, CRI>,
            "2_4_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 8, CRI>,
            "2_8_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 16, CRI>,
            "2_16_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 1, CRI>,
            "4_1_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 2, CRI>,
            "4_2_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 4, CRI>,
            "4_4_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 8, CRI>,
            "4_8_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 16, CRI>,
            "4_16_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 1, CRI>,
            "8_1_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 2, CRI>,
            "8_2_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 4, CRI>,
            "8_4_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 8, CRI>,
            "8_8_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 16, CRI>,
            "8_16_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 1, CRI>,
            "16_1_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 2, CRI>,
            "16_2_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 4, CRI>,
            "16_4_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 8, CRI>,
            "16_8_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 16, CRI>,
            "16_16_cri",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 1, ICR>,
            "1_1_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 2, ICR>,
            "1_2_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 4, ICR>,
            "1_4_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 8, ICR>,
            "1_8_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 16, ICR>,
            "1_16_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 1, ICR>,
            "2_1_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 2, ICR>,
            "2_2_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 4, ICR>,
            "2_4_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 8, ICR>,
            "2_8_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 16, ICR>,
            "2_16_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 1, ICR>,
            "4_1_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 2, ICR>,
            "4_2_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 4, ICR>,
            "4_4_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 8, ICR>,
            "4_8_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 16, ICR>,
            "4_16_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 1, ICR>,
            "8_1_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 2, ICR>,
            "8_2_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 4, ICR>,
            "8_4_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 8, ICR>,
            "8_8_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 16, ICR>,
            "8_16_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 1, ICR>,
            "16_1_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 2, ICR>,
            "16_2_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 4, ICR>,
            "16_4_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 8, ICR>,
            "16_8_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 16, ICR>,
            "16_16_icr",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 1, IRC>,
            "1_1_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 2, IRC>,
            "1_2_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 4, IRC>,
            "1_4_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 8, IRC>,
            "1_8_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 16, IRC>,
            "1_16_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 1, IRC>,
            "2_1_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 2, IRC>,
            "2_2_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 4, IRC>,
            "2_4_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 8, IRC>,
            "2_8_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 16, IRC>,
            "2_16_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 1, IRC>,
            "4_1_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 2, IRC>,
            "4_2_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 4, IRC>,
            "4_4_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 8, IRC>,
            "4_8_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 16, IRC>,
            "4_16_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 1, IRC>,
            "8_1_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 2, IRC>,
            "8_2_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 4, IRC>,
            "8_4_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 8, IRC>,
            "8_8_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 16, IRC>,
            "8_16_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 1, IRC>,
            "16_1_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 2, IRC>,
            "16_2_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 4, IRC>,
            "16_4_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 8, IRC>,
            "16_8_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 16, IRC>,
            "16_16_irc",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 1, RCI>,
            "1_1_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 2, RCI>,
            "1_2_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 4, RCI>,
            "1_4_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 8, RCI>,
            "1_8_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 16, RCI>,
            "1_16_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 1, RCI>,
            "2_1_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 2, RCI>,
            "2_2_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 4, RCI>,
            "2_4_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 8, RCI>,
            "2_8_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 16, RCI>,
            "2_16_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 1, RCI>,
            "4_1_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 2, RCI>,
            "4_2_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 4, RCI>,
            "4_4_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 8, RCI>,
            "4_8_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 16, RCI>,
            "4_16_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 1, RCI>,
            "8_1_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 2, RCI>,
            "8_2_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 4, RCI>,
            "8_4_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 8, RCI>,
            "8_8_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 16, RCI>,
            "8_16_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 1, RCI>,
            "16_1_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 2, RCI>,
            "16_2_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 4, RCI>,
            "16_4_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 8, RCI>,
            "16_8_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 16, RCI>,
            "16_16_rci",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 1, RIC>,
            "1_1_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 2, RIC>,
            "1_2_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 4, RIC>,
            "1_4_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 8, RIC>,
            "1_8_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<1, 16, RIC>,
            "1_16_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 1, RIC>,
            "2_1_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 2, RIC>,
            "2_2_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 4, RIC>,
            "2_4_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 8, RIC>,
            "2_8_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<2, 16, RIC>,
            "2_16_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 1, RIC>,
            "4_1_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 2, RIC>,
            "4_2_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 4, RIC>,
            "4_4_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 8, RIC>,
            "4_8_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<4, 16, RIC>,
            "4_16_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 1, RIC>,
            "8_1_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 2, RIC>,
            "8_2_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 4, RIC>,
            "8_4_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 8, RIC>,
            "8_8_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<8, 16, RIC>,
            "8_16_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 1, RIC>,
            "16_1_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 2, RIC>,
            "16_2_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 4, RIC>,
            "16_4_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 8, RIC>,
            "16_8_ric",
        ),
        (
            Matrix::fast_mul_concurrent_blocksize_order::<16, 16, RIC>,
            "16_16_ric",
        ),
    ];
    for (mul_fn, label) in concurrent_muls {
        g.bench_function(format!("matmul_concurrent_{label}"), |b| {
            b.iter_batched(
                || random_matrix_pair(m, k, n),
                |(a, b)| mul_fn(&a, &b),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

fn random_matrix_pair(rows: usize, inner: usize, cols: usize) -> (Matrix, Matrix) {
    (random_matrix(rows, inner), random_matrix(inner, cols))
}

fn random_matrix(rows: usize, cols: usize) -> Matrix {
    let mut rng = rand::rng();
    let mut data = Vec::new();
    let data_len = rows * cols.next_multiple_of(64);
    for _ in 0..data_len {
        data.push(rng.random());
    }
    Matrix::from_data(TWO, rows, cols, data)
}

criterion_group! {
    name = mul;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(3))
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = muls
}

criterion_main!(mul);
