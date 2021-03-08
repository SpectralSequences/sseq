use algebra::milnor_algebra::{PPartAllocation, PPartMultiplier, PPartEntry};
use bencher::{benchmark_group, benchmark_main, Bencher};
use fp::prime::ValidPrime;

fn ppart_inner<const MOD4: bool>(bench: &mut Bencher, p: u32, r: Vec<PPartEntry>, s: Vec<u32>) {
    let p = ValidPrime::new(p);

    bench.iter(move || {
        let mut m = PPartMultiplier::<MOD4>::new_from_allocation(p, &r, &s, PPartAllocation::default(), 0, 0);

        while let Some(c) = m.next() {
            if MOD4 {
                assert!(c < 4);
            } else {
                assert!(c < *p);
            }
        }
    });
}

fn ppart_2(bench: &mut Bencher) {
    ppart_inner::<false>(bench, 2, vec![60, 30, 8, 2, 1], vec![20, 30, 20, 4, 1, 2]);
    ppart_inner::<false>(bench, 2, vec![35, 12, 20, 14, 1, 3], vec![60, 30, 0, 2, 1]);
}

fn ppart_4(bench: &mut Bencher) {
    ppart_inner::<true>(bench, 2, vec![60, 30, 8, 2, 1], vec![20, 30, 20, 4, 1, 2]);
    ppart_inner::<true>(bench, 2, vec![35, 12, 20, 14, 1, 3], vec![60, 30, 0, 2, 1]);
}

fn ppart_3(bench: &mut Bencher) {
    ppart_inner::<false>(bench, 3, vec![120, 70, 40, 2], vec![60, 35, 21, 6]);
    ppart_inner::<false>(bench, 3, vec![30, 12, 35, 24], vec![100, 80, 16, 2, 3]);
}

benchmark_group!(benches, ppart_2, ppart_4, ppart_3);
benchmark_main!(benches);
