use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use fp::{
    blas::tile::{LoopOrder, orders::*},
    matrix::Matrix,
    prime::TWO,
};
use pprof::criterion::{Output, PProfProfiler};
use rand::Rng;

/// A simple enum to bridge between runtime variables and type-level loop orderings.
///
/// We use various dispatch functions to convert constants to generics.
#[derive(Debug, Clone, Copy)]
enum Order {
    Cir,
    Cri,
    Icr,
    Irc,
    Rci,
    Ric,
}

type GemmFnDatum = (fn(&Matrix, &Matrix) -> Matrix, String);

fn dispatch_l(ord: Order, m: i32, n: i32) -> GemmFnDatum {
    match ord {
        Order::Cir => dispatch_m::<CIR>(m, n, "cir"),
        Order::Cri => dispatch_m::<CRI>(m, n, "cri"),
        Order::Icr => dispatch_m::<ICR>(m, n, "icr"),
        Order::Irc => dispatch_m::<IRC>(m, n, "irc"),
        Order::Rci => dispatch_m::<RCI>(m, n, "rci"),
        Order::Ric => dispatch_m::<RIC>(m, n, "ric"),
    }
}

fn dispatch_m<L: LoopOrder>(m: i32, n: i32, l_name: &'static str) -> GemmFnDatum {
    match m {
        1 => dispatch_n::<1, L>(n, l_name),
        2 => dispatch_n::<2, L>(n, l_name),
        4 => dispatch_n::<4, L>(n, l_name),
        8 => dispatch_n::<8, L>(n, l_name),
        16 => dispatch_n::<16, L>(n, l_name),
        _ => unimplemented!(),
    }
}

fn dispatch_n<const M: usize, L: LoopOrder>(n: i32, l_name: &'static str) -> GemmFnDatum {
    match n {
        1 => (
            Matrix::fast_mul_concurrent_blocksize_order::<M, 1, L>,
            format!("{M}_{n}_{l_name}"),
        ),
        2 => (
            Matrix::fast_mul_concurrent_blocksize_order::<M, 2, L>,
            format!("{M}_{n}_{l_name}"),
        ),
        4 => (
            Matrix::fast_mul_concurrent_blocksize_order::<M, 4, L>,
            format!("{M}_{n}_{l_name}"),
        ),
        8 => (
            Matrix::fast_mul_concurrent_blocksize_order::<M, 8, L>,
            format!("{M}_{n}_{l_name}"),
        ),
        16 => (
            Matrix::fast_mul_concurrent_blocksize_order::<M, 16, L>,
            format!("{M}_{n}_{l_name}"),
        ),
        _ => unimplemented!(),
    }
}

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
    let ords = [
        Order::Cir,
        Order::Cri,
        Order::Icr,
        Order::Irc,
        Order::Rci,
        Order::Ric,
    ];
    let ms = (0..5).map(|i| 1 << i);
    let ns = (0..5).map(|i| 1 << i);

    let concurrent_muls: Vec<_> = itertools::iproduct!(ords.into_iter(), ms, ns)
        .map(|(ord, m, n)| dispatch_l(ord, m, n))
        .collect();

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
