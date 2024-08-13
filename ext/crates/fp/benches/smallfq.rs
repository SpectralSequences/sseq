use criterion::{criterion_group, criterion_main, Criterion};
use fp::{
    field::{Field, SmallFq},
    prime::{Prime, ValidPrime, P2, P3, P5, P7},
    PRIMES,
};
use pprof::criterion::{Output, PProfProfiler};

fn add_all_pairs<P: Prime>(c: &mut Criterion, f_q: SmallFq<P>) {
    let zero = f_q.zero();
    let one = f_q.one();
    let a = f_q.a();

    let mut elements = vec![zero, one];
    for _ in 1..f_q.q() {
        let prev = elements.last().unwrap();
        elements.push(*prev * a);
    }

    c.bench_function(&format!("add_f_{}", f_q.q()), |b| {
        b.iter(|| {
            for &i in elements.iter() {
                for &j in elements.iter() {
                    let _ = i + j;
                }
            }
        })
    });
}

fn mul_all_pairs<P: Prime>(c: &mut Criterion, f_q: SmallFq<P>) {
    let zero = f_q.zero();
    let one = f_q.one();
    let a = f_q.a();

    let mut elements = vec![zero, one];
    for _ in 1..f_q.q() {
        let prev = elements.last().unwrap();
        elements.push(*prev * a);
    }

    c.bench_function(&format!("mul_f_{}", f_q.q()), |b| {
        b.iter(|| {
            for &i in elements.iter() {
                for &j in elements.iter() {
                    let _ = i * j;
                }
            }
        })
    });
}

fn bench_add(c: &mut Criterion) {
    const MAX_SIZE: u32 = 1000;
    for d in 2.. {
        if P2.pow(d) < MAX_SIZE {
            add_all_pairs(c, SmallFq::new(P2, d));
        } else {
            break;
        }
        if P3.pow(d) < MAX_SIZE {
            add_all_pairs(c, SmallFq::new(P3, d));
        }
        if P5.pow(d) < MAX_SIZE {
            add_all_pairs(c, SmallFq::new(P5, d));
        }
        if P7.pow(d) < MAX_SIZE {
            add_all_pairs(c, SmallFq::new(P7, d));
        }
        for p in PRIMES.iter().skip(4) {
            if p.pow(d) < MAX_SIZE {
                add_all_pairs(c, SmallFq::new(ValidPrime::new(*p), d));
            } else {
                break;
            }
        }
    }
}

fn bench_mul(c: &mut Criterion) {
    const MAX_SIZE: u32 = 1000;
    for d in 2.. {
        if P2.pow(d) < MAX_SIZE {
            mul_all_pairs(c, SmallFq::new(P2, d));
        } else {
            break;
        }
        if P3.pow(d) < MAX_SIZE {
            mul_all_pairs(c, SmallFq::new(P3, d));
        }
        if P5.pow(d) < MAX_SIZE {
            mul_all_pairs(c, SmallFq::new(P5, d));
        }
        if P7.pow(d) < MAX_SIZE {
            mul_all_pairs(c, SmallFq::new(P7, d));
        }
        for p in PRIMES.iter().skip(4) {
            if p.pow(d) < MAX_SIZE {
                mul_all_pairs(c, SmallFq::new(ValidPrime::new(*p), d));
            } else {
                break;
            }
        }
    }
}

criterion_group! {
    name = add;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_add, bench_mul
}

criterion_main!(add);
