use criterion::{black_box, criterion_group, criterion_main, Criterion};
use once::OnceVec;

fn push(n: usize) {
    let v = OnceVec::new();
    for i in 0..n {
        v.push(i);
    }
    assert_eq!(v.len(), n);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("push 100000", |b| b.iter(|| push(black_box(100000))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
