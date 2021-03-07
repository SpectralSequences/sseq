use bencher::{Bencher, benchmark_group};

use fp::prime;

fn binomial_3(bench: &mut Bencher) {
    bench.iter(|| {
        for y in 1 .. 100 {
            for x in 0 .. y {
                prime::binomial_odd(prime::ValidPrime::new(3), y, x);
            }
        }
    });
}

fn multinomial_7(bench: &mut Bencher) {
    bench.iter(|| {
        for w in 1 .. 20 {
            for x in 1 .. 20 {
                for y in 1 .. 20 {
                    for z in 1 .. 20 {
                        prime::multinomial_odd(prime::ValidPrime::new(7), &mut [w, x, y, z]);
                    }
                }
            }
        }
    });
}


benchmark_group!(main, binomial_3, multinomial_7);

