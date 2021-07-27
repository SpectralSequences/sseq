// use criterion::Criterion;
// use fp::prime::{Binomial, ValidPrime};

// fn binomial_3(bench: &mut Criterion) {
//     bench.iter(|| {
//         for y in 1..100 {
//             for x in 0..y {
//                 u32::binomial_odd(ValidPrime::new(3), y, x);
//             }
//         }
//     });
// }

// fn multinomial_7(bench: &mut Criterion) {
//     bench.iter(|| {
//         for w in 1..20 {
//             for x in 1..20 {
//                 for y in 1..20 {
//                     for z in 1..20 {
//                         u32::multinomial_odd(ValidPrime::new(7), &mut [w, x, y, z]);
//                     }
//                 }
//             }
//         }
//     });
// }

// criterion_group!(main, binomial_3, multinomial_7);
