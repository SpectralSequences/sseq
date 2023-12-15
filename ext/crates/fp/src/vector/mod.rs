#[cfg(not(feature = "odd-primes"))]
pub mod vector_2;
#[cfg(not(feature = "odd-primes"))]
pub use vector_2::*;

#[cfg(feature = "odd-primes")]
pub mod vector_generic;
#[cfg(feature = "odd-primes")]
pub use vector_generic::*;

pub(crate) mod inner;

mod impl_fpvectorp;
mod impl_slicemutp;
mod impl_slicep;
mod iter;

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use itertools::Itertools;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::{inner::FqVectorP, *};
    use crate::{
        field::{limb::LimbMethods, Fp},
        limb,
        prime::{Prime, ValidPrime, P2},
    };

    pub struct VectorDiffEntry {
        pub index: usize,
        pub left: u32,
        pub right: u32,
    }

    impl FpVector {
        pub fn diff_list(&self, other: &[u32]) -> Vec<VectorDiffEntry> {
            assert!(self.len() == other.len());
            let mut result = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for index in 0..self.len() {
                let left = self.entry(index);
                let right = other[index];
                if left != right {
                    result.push(VectorDiffEntry { index, left, right });
                }
            }
            result
        }

        pub fn diff_vec(&self, other: &Self) -> Vec<VectorDiffEntry> {
            assert!(self.len() == other.len());
            let mut result = Vec::new();
            for index in 0..self.len() {
                let left = self.entry(index);
                let right = other.entry(index);
                if left != right {
                    result.push(VectorDiffEntry { index, left, right });
                }
            }
            result
        }

        pub fn format_diff(diff: Vec<VectorDiffEntry>) -> String {
            let data_formatter =
                diff.iter()
                    .format_with("\n ", |VectorDiffEntry { index, left, right }, f| {
                        f(&format_args!("  At index {index}: {left}!={right}"))
                    });
            format!("{data_formatter}")
        }

        pub fn assert_list_eq(&self, other: &[u32]) {
            let diff = self.diff_list(other);
            if diff.is_empty() {
                return;
            }
            panic!(
                "assert {} == {:?}\n{}",
                self,
                other,
                Self::format_diff(diff)
            );
        }

        pub fn assert_vec_eq(&self, other: &Self) {
            let diff = self.diff_vec(other);
            if diff.is_empty() {
                return;
            }
            panic!(
                "assert {} == {:?}\n{}",
                self,
                other,
                Self::format_diff(diff)
            );
        }
    }

    fn random_vector(p: u32, dimension: usize) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        (0..dimension).map(|_| rng.gen_range(0..p)).collect()
    }

    /// An arbitrary `ValidPrime` in the range `2..(1 << 24)`, plus the largest prime that we support.
    fn arb_prime() -> impl Strategy<Value = ValidPrime> {
        static TEST_PRIMES: OnceLock<Vec<ValidPrime>> = OnceLock::new();
        let test_primes = TEST_PRIMES.get_or_init(|| {
            // Sieve of erathosthenes
            const MAX: usize = 1 << 24;
            let mut is_prime = Vec::new();
            is_prime.resize_with(MAX, || true);
            is_prime[0] = false;
            is_prime[1] = false;
            for i in 2..MAX {
                if is_prime[i] {
                    for j in ((2 * i)..MAX).step_by(i) {
                        is_prime[j] = false;
                    }
                }
            }
            (0..MAX)
                .filter(|&i| is_prime[i])
                .map(|p| ValidPrime::new_unchecked(p as u32))
                .chain(std::iter::once(ValidPrime::new_unchecked(2147483647)))
                .collect()
        });
        (0..test_primes.len()).prop_map(|i| test_primes[i])
    }

    /// An arbitrary (prime, dimension) pair
    fn arb_prime_dim() -> impl Strategy<Value = (ValidPrime, usize)> {
        arb_prime().prop_flat_map(|p| (Just(p), 0usize..=10_000))
    }

    /// The start and end positions of an arbitrary slice of a vector of length `dimension`
    fn arb_slice(dimension: usize) -> impl Strategy<Value = (usize, usize)> {
        (0..=dimension).prop_flat_map(move |first| (Just(first), first..=dimension))
    }

    prop_compose! {
        /// An arbitrary pair of slices of a vector of length `dimension` _that have the same length_
        fn arb_slice_pair(dimension: usize)
            (len in 0..=dimension)
            (len in Just(len), first in 0..=dimension - len, second in 0..=dimension - len)
            -> [(usize, usize); 2]
        {
            [(first, first + len), (second, second + len)]
        }
    }

    /// An arbitrary vector of length `dimension` containing values in the range `0..p`. The tests
    /// take in a `Vec<u32>` instead of an `FpVector` directly because they will usually apply some
    /// operation on both the `FpVector` and the original `Vec<u32>` and then compare the results.
    fn arb_vec_u32(p: ValidPrime, dimension: usize) -> impl Strategy<Value = Vec<u32>> {
        proptest::collection::vec(0..p.as_u32(), dimension)
    }

    /// A pair of a prime `p` and a vector containing values in the range `0..p`. In other
    /// words, a vector over Fp.
    fn arb_vec() -> impl Strategy<Value = (ValidPrime, Vec<u32>)> {
        arb_prime_dim().prop_flat_map(|(p, dim)| (Just(p), arb_vec_u32(p, dim)))
    }

    /// An Fp vector together with valid slice indices
    fn arb_vec_and_slice() -> impl Strategy<Value = (ValidPrime, Vec<u32>, (usize, usize))> {
        arb_prime_dim().prop_flat_map(|(p, dim)| (Just(p), arb_vec_u32(p, dim), arb_slice(dim)))
    }

    /// A pair of Fp vectors of the same length over the same prime
    fn arb_vec_pair() -> impl Strategy<Value = (ValidPrime, Vec<u32>, Vec<u32>)> {
        arb_prime_dim()
            .prop_flat_map(|(p, dim)| (Just(p), arb_vec_u32(p, dim), arb_vec_u32(p, dim)))
    }

    /// A pair of Fp vectors of the same length over the same prime, together with valid slice
    /// indices
    fn arb_vec_pair_and_slice(
    ) -> impl Strategy<Value = (ValidPrime, Vec<u32>, Vec<u32>, (usize, usize))> {
        arb_prime_dim().prop_flat_map(|(p, dim)| {
            (
                Just(p),
                arb_vec_u32(p, dim),
                arb_vec_u32(p, dim),
                arb_slice(dim),
            )
        })
    }

    /// A pair of Fp vectors of the same length over the same prime, together with a mask (in
    /// the sense of [`FpVector::add_masked`] and [`FpVector::add_unmasked`])
    fn arb_vec_pair_and_mask() -> impl Strategy<Value = (ValidPrime, Vec<u32>, Vec<u32>, Vec<usize>)>
    {
        arb_prime()
            .prop_flat_map(|p| (Just(p), arb_slice(10_000)))
            .prop_flat_map(|(p, (dim_small, dim_large))| {
                (
                    Just(p),
                    arb_vec_u32(p, dim_small),
                    arb_vec_u32(p, dim_large),
                    proptest::collection::vec(0..dim_large, dim_small),
                )
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 1024,
            max_shrink_time: 30_000,
            max_shrink_iters: 1_000_000,
            .. ProptestConfig::default()
        })]

        #[test]
        fn test_bit_length(p in arb_prime()) {
            prop_assert!(Fp(p).bit_length() <= 63);
        }

        #[test]
        fn test_incompatible_primes((p1, p2) in (arb_prime(), arb_prime())) {
            prop_assume!(p1 != p2);

            macro_rules! assert_panic {
                ($function:ident $(, $($args:expr),*)?) => {
                    let panic = std::panic::catch_unwind(|| {
                        FpVector::new(p1, 10).$function(&FpVector::new(p2, 10) $(, $($args),*)?)
                    });
                    prop_assert!(panic.is_err());
                };
            }

            assert_panic!(assign);
            assert_panic!(assign_partial);
            assert_panic!(add, 1);
            assert_panic!(add_offset, 1, 5);
            assert_panic!(add_truncate, 1);
            assert_panic!(sign_rule);
            assert_panic!(add_carry, 1, &mut []);
        }

        #[test]
        fn test_serialize((p, v_arr) in arb_vec()) {
            use std::io::{Seek, Cursor};

            let v = FpVector::from_slice(p, &v_arr);

            let mut cursor = Cursor::new(Vec::<u8>::new());
            v.to_bytes(&mut cursor).unwrap();
            cursor.rewind().unwrap();

            let w = FpVector::from_bytes(v.prime(), v.len(), &mut cursor).unwrap();
            v.assert_vec_eq(&w);
        }

        #[test]
        fn test_add((p, mut v_arr, w_arr) in arb_vec_pair()) {
            let mut v = FpVector::from_slice(p, &v_arr);
            let w = FpVector::from_slice(p, &w_arr);

            v.add(&w, 1);

            for (v_element, w_element) in v_arr.iter_mut().zip(w_arr.iter()) {
                *v_element = (*v_element + *w_element) % p;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_scale((p, mut v_arr, c) in arb_prime_dim().prop_flat_map(|(p, dim)| {
            (Just(p), arb_vec_u32(p, dim), 0..p.as_u32())
        })) {
            let mut v = FpVector::from_slice(p, &v_arr);
            v.scale(c);
            for entry in &mut v_arr {
                *entry = p.product(*entry, c);
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_scale_slice((p, mut v_arr, (slice_start, slice_end), c) in
            arb_prime_dim().prop_flat_map(|(p, dim)| {
                (Just(p), arb_vec_u32(p, dim), arb_slice(dim), 0..p.as_u32())
            })
        ) {
            let mut v = FpVector::from_slice(p, &v_arr);
            v.slice_mut(slice_start, slice_end).scale(c);

            for entry in &mut v_arr[slice_start..slice_end] {
                *entry = p.product(*entry, c);
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_entry((p, v_arr) in arb_vec()) {
            let v = FpVector::from_slice(p, &v_arr);

            let mut diffs = Vec::new();
            for (i, val) in v.iter().enumerate() {
                if v.entry(i) != val {
                    diffs.push((i, val, v.entry(i)));
                }
            }
            prop_assert_eq!(diffs, []);
        }

        #[test]
        fn test_entry_slice((p, v_arr, (slice_start, slice_end)) in arb_vec_and_slice()) {
            let v = FpVector::from_slice(p, &v_arr);
            let v = v.slice(slice_start, slice_end);
            println!(
                "slice_start: {slice_start}, slice_end: {slice_end}, slice: {v}"
                );

            let mut diffs = Vec::new();
            for i in 0..v.len() {
                if v.entry(i) != v_arr[i + slice_start] {
                    diffs.push((i, v_arr[i + slice_start], v.entry(i)));
                }
            }
            prop_assert_eq!(diffs, []);
        }

        #[test]
        fn test_set_entry((p, v_arr) in arb_vec()) {
            let mut v = FpVector::new(p, v_arr.len());

            for (i, &val) in v_arr.iter().enumerate() {
                v.set_entry(i, val);
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_set_entry_slice((p, v_arr, (slice_start, slice_end)) in arb_vec_and_slice()) {
            let dim = v_arr.len();
            let mut v = FpVector::new(p, dim);
            let mut v = v.slice_mut(slice_start, slice_end);

            let v_slice = &v_arr[slice_start..slice_end];
            for (i, &val) in v_slice.iter().enumerate() {
                v.set_entry(i, val);
            }
            let v = v.as_slice();

            let mut diffs = Vec::new();
            for (i, &val) in v_slice.iter().enumerate() {
                if v.entry(i) != val {
                    diffs.push((i, val, v.entry(i)));
                }
            }
            prop_assert_eq!(diffs, []);
        }

        #[test]
        fn test_set_to_zero_slice((p, mut v_arr, (slice_start, slice_end)) in arb_vec_and_slice()) {
            println!("slice_start : {slice_start}, slice_end : {slice_end}");
            let mut v = FpVector::from_slice(p, &v_arr);

            v.slice_mut(slice_start, slice_end).set_to_zero();
            prop_assert!(v.slice(slice_start, slice_end).is_zero());

            for entry in &mut v_arr[slice_start..slice_end] {
                *entry = 0;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_add_slice_to_slice((p, mut v_arr, w_arr, (slice_start, slice_end)) in arb_vec_pair_and_slice()) {
            let mut v = FpVector::from_slice(p, &v_arr);
            let w = FpVector::from_slice(p, &w_arr);

            v.slice_mut(slice_start, slice_end)
                .add(w.slice(slice_start, slice_end), 1);

            for i in slice_start..slice_end {
                v_arr[i] = (v_arr[i] + w_arr[i]) % p;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_assign((p, v_arr, w_arr) in arb_vec_pair()) {
            let mut v = FpVector::from_slice(p, &v_arr);
            let w = FpVector::from_slice(p, &w_arr);

            v.assign(&w);
            v.assert_vec_eq(&w);
        }

        #[test]
        fn test_assign_partial((p, v_arr, w_arr) in arb_vec_pair()) {
            let dim = v_arr.len();
            let mut v = FpVector::from_slice(p, &v_arr);
            let w = FpVector::from_slice(p, &w_arr[0..(dim / 2)]);

            v.assign_partial(&w);
            prop_assert!(v.slice(dim / 2, dim).is_zero());
            prop_assert_eq!(v.len(), dim);
            v.slice(0, dim / 2).to_owned().assert_vec_eq(&w);
        }

        #[test]
        fn test_assign_slice_to_slice((p, mut v_arr, w_arr, (slice_start, slice_end)) in arb_vec_pair_and_slice()) {
            let mut v = FpVector::from_slice(p, &v_arr);
            let w = FpVector::from_slice(p, &w_arr);

            v.slice_mut(slice_start, slice_end)
                .assign(w.slice(slice_start, slice_end));
            v_arr[slice_start..slice_end].clone_from_slice(&w_arr[slice_start..slice_end]);
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_add_shift((p, mut v_arr, w_arr, [(slice1_start, slice1_end), (slice2_start, slice2_end)])
            in arb_prime_dim().prop_flat_map(|(p, dim)| {
                (
                    Just(p),
                    arb_vec_u32(p, dim),
                    arb_vec_u32(p, dim),
                    arb_slice_pair(dim),
                )
            })
        ) {
            let mut v = FpVector::from_slice(p, &v_arr);
            let w = FpVector::from_slice(p, &w_arr);

            v.slice_mut(slice1_start, slice1_end)
                .add(w.slice(slice2_start, slice2_end), 1);

            for (v_element, w_element) in v_arr[slice1_start..slice1_end]
                .iter_mut()
                .zip(w_arr[slice2_start..slice2_end].iter())
            {
                *v_element = (*v_element + *w_element) % p;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_add_masked((p, mut v_small, v_big, mask) in arb_vec_pair_and_mask()) {
            let mut v = FpVector::from_slice(p, &v_small);
            let w = FpVector::from_slice(p, &v_big);

            v.as_slice_mut().add_masked(w.as_slice(), 1, &mask);

            for (i, x) in v_small.iter_mut().enumerate() {
                *x += v_big[mask[i]];
                *x %= p;
            }

            v.assert_list_eq(&v_small);
        }

        #[test]
        fn test_add_unmasked((p, v_small, mut v_big, mask) in arb_vec_pair_and_mask()) {
            let mut v = FpVector::from_slice(p, &v_big);
            let w = FpVector::from_slice(p, &v_small);

            v.as_slice_mut().add_unmasked(w.as_slice(), 1, &mask);
            for (i, &x) in v_small.iter().enumerate() {
                v_big[mask[i]] += x;
                v_big[mask[i]] %= p;
            }
            v.assert_list_eq(&v_big);
        }

        #[test]
        fn test_iterator_slice((p, v_arr, (slice_start, slice_end)) in arb_vec_and_slice()) {
            let v = FpVector::from_slice(p, &v_arr);
            let v = v.slice(slice_start, slice_end);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                prop_assert_eq!(v.entry(i), x);
                counter += 1;
            }
            prop_assert_eq!(counter, v.len());
        }

        #[test]
        fn test_iterator_skip((p, v_arr, num_skip) in arb_prime_dim().prop_flat_map(|(p, dim)| {
            (Just(p), arb_vec_u32(p, dim), 0..=dim)
        })) {
            let v = FpVector::from_slice(p, &v_arr);

            let mut w = v.iter();
            w.skip_n(num_skip);
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                prop_assert_eq!(v.entry(i + num_skip), x);
                counter += 1;
            }

            prop_assert_eq!(counter, v.len() - num_skip);
        }

        #[test]
        fn test_iterator((p, v_arr) in arb_vec()) {
            let v = FpVector::from_slice(p, &v_arr);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                prop_assert_eq!(v.entry(i), x);
                counter += 1;
            }
            prop_assert_eq!(counter, v.len());
        }

        #[test]
        fn test_iter_nonzero_empty((p, dimension) in arb_prime_dim()) {
            let v = FpVector::new(p, dimension);
            prop_assert_eq!(v.iter_nonzero().next(), None);
        }

        #[test]
        fn test_iter_nonzero((p, v_arr, (slice_start, slice_end)) in arb_vec_and_slice()) {
            use std::fmt::Write;

            let v = FpVector::from_slice(p, &v_arr);

            println!("v: {v}");
            println!("v_arr: {v_arr:?}");
            let result: Vec<_> = v.slice(slice_start, slice_end).iter_nonzero().collect();
            let comparison_result: Vec<_> = v_arr[slice_start..slice_end]
                .iter()
                .copied()
                .enumerate()
                .filter(|&(_, x)| x != 0)
                .collect();

            let mut i = 0;
            let mut j = 0;
            let mut diffs_str = String::new();
            while i < result.len() && j < comparison_result.len() {
                if result[i] != comparison_result[j] {
                    if result[i].0 < comparison_result[j].0 {
                        let _ = write!(
                            diffs_str,
                            "\n({:?}) present in result, missing from comparison_result",
                            result[i]
                        );
                        i += 1;
                    } else {
                        let _ = write!(
                            diffs_str,
                            "\n({:?}) present in comparison_result, missing from result",
                            comparison_result[j]
                        );
                        j += 1;
                    }
                } else {
                    i += 1;
                    j += 1;
                }
            }
            // for i in 0 .. std::cmp::min(result.len(), comparison_result.len()) {
            //     println!("res : {:?}, comp : {:?}", result[i], comparison_result[i]);
            // }
            prop_assert!(diffs_str.is_empty(), "{}", diffs_str);
        }
    }

    #[rstest]
    #[trace]
    fn test_add_carry(#[values(2)] p: u32, #[values(10, 20, 70, 100, 1000)] dim: usize) {
        use std::fmt::Write;

        let p = ValidPrime::new(p);
        const E_MAX: usize = 4;
        let pto_the_e_max = (p * p * p * p) * p;
        let mut v = Vec::with_capacity(E_MAX + 1);
        let mut w = Vec::with_capacity(E_MAX + 1);
        for _ in 0..=E_MAX {
            v.push(FpVector::new(p, dim));
            w.push(FpVector::new(p, dim));
        }
        let v_arr = random_vector(pto_the_e_max, dim);
        let w_arr = random_vector(pto_the_e_max, dim);
        for i in 0..dim {
            let mut ev = v_arr[i];
            let mut ew = w_arr[i];
            for e in 0..=E_MAX {
                v[e].set_entry(i, ev % p);
                w[e].set_entry(i, ew % p);
                ev /= p;
                ew /= p;
            }
        }

        println!("in  : {v_arr:?}");
        for (e, val) in v.iter().enumerate() {
            println!("in {e}: {val}");
        }
        println!();

        println!("in  : {w_arr:?}");
        for (e, val) in w.iter().enumerate() {
            println!("in {e}: {val}");
        }
        println!();

        for e in 0..=E_MAX {
            let (first, rest) = v[e..].split_at_mut(1);
            first[0].add_carry(&w[e], 1, rest);
        }

        let mut vec_result = vec![0; dim];
        for (i, entry) in vec_result.iter_mut().enumerate() {
            for e in (0..=E_MAX).rev() {
                *entry *= p;
                *entry += v[e].entry(i);
            }
        }

        for (e, val) in v.iter().enumerate() {
            println!("out{e}: {val}");
        }
        println!();

        let mut comparison_result = vec![0; dim];
        for i in 0..dim {
            comparison_result[i] = (v_arr[i] + w_arr[i]) % pto_the_e_max;
        }
        println!("out : {comparison_result:?}");

        let mut diffs = Vec::new();
        let mut diffs_str = String::new();
        for i in 0..dim {
            if vec_result[i] != comparison_result[i] {
                diffs.push((i, comparison_result[i], vec_result[i]));
                let _ = write!(
                    diffs_str,
                    "\nIn position {} expected {} got {}. v[i] = {}, w[i] = {}.",
                    i, comparison_result[i], vec_result[i], v_arr[i], w_arr[i]
                );
            }
        }
        assert!(diffs.is_empty(), "{}", diffs_str);
    }

    #[test]
    fn test_sign_rule_limb() {
        assert!(limb::sign_rule(1, 0b10) == 1);
        assert!(limb::sign_rule(0b10, 1) == 0);
        assert!(limb::sign_rule(0x84012c02, 0x6b920241) == 1);
        assert!(limb::sign_rule(0x6b920241, 0x84012c02) == 0);
    }

    #[test]
    #[ignore]
    fn test_sign_rule() {
        let mut in1 = FqVectorP::<Fp<P2>>::new(P2, 128);
        let mut in2 = FqVectorP::<Fp<P2>>::new(P2, 128);
        let tests = [
            (
                0x181e20846a820820,
                0x2122a1a08c1a0069,
                0xe30140608100e540,
                0xd2180e4350008004,
                false,
                false,
            ),
            (
                0x2090400020017044,
                0xa04e0802080000e1,
                0x18298a0a85080089,
                0x050020311030411a,
                false,
                false,
            ),
            (
                0x082080022408d510,
                0x538a000802078210,
                0x2355308c4a920002,
                0x00058130800000a2,
                true,
                true,
            ),
            (
                0x33a0824922050704,
                0x00400520a0800404,
                0x00090836000a980b,
                0x4801d005064b9840,
                false,
                false,
            ),
            (
                0x290c14040154a01b,
                0x38014102810a0245,
                0x0093281a620a1060,
                0x029014cd0684080a,
                true,
                true,
            ),
            (
                0x240255b490b0e040,
                0x0815414130548881,
                0x8ad4880a00000416,
                0xb660a4b84cab002c,
                true,
                true,
            ),
            (
                0x010c000060840540,
                0x8008001480104028,
                0x8842938396233a31,
                0x5e20400311059a41,
                true,
                true,
            ),
            (
                0x02012141008e5081,
                0x2829060241920a00,
                0xe0208a1a47102310,
                0x051240010e6c4008,
                false,
                false,
            ),
            (
                0x200812011081880f,
                0x100661c082625864,
                0x48840c76c03a2380,
                0x861088274000060a,
                false,
                false,
            ),
            (
                0x84000f5490449008,
                0x00891820f4623401,
                0x107490a964b802a4,
                0x40024487008800b0,
                false,
                false,
            ),
            (
                0x080448a2db282c41,
                0x2c100011e00097dd,
                0x0131024124844028,
                0x8329600202440002,
                false,
                false,
            ),
            (
                0x441c60a208c2e206,
                0x00a4210b50049281,
                0x0842020160091158,
                0x48131424846a6868,
                true,
                true,
            ),
            (
                0xc2743ad490a21411,
                0x0150221280868050,
                0x1082402043040888,
                0xdc070000021128a0,
                true,
                true,
            ),
            (
                0x0614030849072140,
                0x0e7a710422002540,
                0x300904418240c422,
                0x80850ccad8a10200,
                false,
                true,
            ),
            (
                0x90080028402bc624,
                0x215002cf204840a0,
                0x6373f01012001042,
                0x420b111008350859,
                false,
                true,
            ),
            (
                0x4220c41100513301,
                0x332c050498c21102,
                0x0c0c206c8a008044,
                0xc0024840461484d0,
                true,
                false,
            ),
            (
                0x0353a04b08000010,
                0x3e00045295202851,
                0x60040810a42a1284,
                0x001d680860800080,
                true,
                false,
            ),
            (
                0x084801c0c2100581,
                0x1820090035001080,
                0x3111121b0522185c,
                0x01404209002c080c,
                true,
                false,
            ),
            (
                0x414800000823a20e,
                0x008074081080a214,
                0x1a12852095d040c0,
                0x8119003425575408,
                false,
                true,
            ),
            (
                0x210c730112098440,
                0x01c0b106111483d0,
                0x920004486810020c,
                0xb614405084c30004,
                true,
                true,
            ),
            (
                0x60210168b8802094,
                0x2a10021a4b08420c,
                0x1554000102241028,
                0x04048d0000349000,
                true,
                true,
            ),
            (
                0x81200240041188c8,
                0x148008c1c6220818,
                0x0082a92c10000010,
                0x0050500800100084,
                true,
                false,
            ),
            (
                0x4593105c94090408,
                0x820029daa0026830,
                0x1864242101429200,
                0x1822060103290348,
                true,
                false,
            ),
            (
                0x551a0002870e6000,
                0x0040a00040353a00,
                0x200409c110101589,
                0x28870e620a488442,
                true,
                false,
            ),
            (
                0x8a0200806440124b,
                0x9c6000904e824800,
                0x5150404003022c84,
                0x2014452420012031,
                true,
                false,
            ),
            (
                0x840216c970c02c10,
                0x16490c8222011000,
                0x4a6040120034800b,
                0x09008001d4166827,
                false,
                true,
            ),
            (
                0x042040900809589c,
                0x4102064021804040,
                0x98903b221480a523,
                0x964840081847130e,
                false,
                false,
            ),
            (
                0xa005ed201240a002,
                0x580903106014a842,
                0x16680288c4321521,
                0x2030400608021010,
                true,
                true,
            ),
            (
                0x405008860b020123,
                0x2100052200602aee,
                0xb809422040018014,
                0x0a21a20090041001,
                true,
                true,
            ),
            (
                0x3108541538030498,
                0x014302a04a20a081,
                0x0080806005804804,
                0xdc00700020cc405c,
                true,
                true,
            ),
            (
                0x6020490087030a00,
                0x008a11c320049998,
                0x069512591824a091,
                0x4a300a0808002006,
                true,
                true,
            ),
            (
                0x206e90b404108a02,
                0x4a0408221400b022,
                0x0580040201607498,
                0x0131d21d80080b08,
                false,
                false,
            ),
            (
                0x84811204041e00bd,
                0x011410092c824801,
                0x0162802203216100,
                0xd8200844514c8040,
                false,
                false,
            ),
            (
                0x0020000005800845,
                0x4c19021081244589,
                0x56026e803008012a,
                0x916081a350103000,
                true,
                true,
            ),
            (
                0x407050c08808e102,
                0x1102095040020904,
                0x000187005245184c,
                0x28104485228804e3,
                true,
                true,
            ),
            (
                0x6d20550000808446,
                0x4008211019808425,
                0x804e20c004212381,
                0x02305c0542603848,
                false,
                false,
            ),
            (
                0x8010400016110202,
                0x5a40a22409e0220c,
                0x04e20103604a3980,
                0x80181142f20a9103,
                false,
                true,
            ),
            (
                0x002c12089073280e,
                0x80c8680090b66020,
                0xd8c12d02488850a0,
                0x010217794101901c,
                false,
                true,
            ),
            (
                0x290c01102e12800c,
                0x4c881498c852154e,
                0x86c0142101a810b2,
                0x31420a2623a40091,
                false,
                true,
            ),
            (
                0xe08400012018c888,
                0x020204c23b0a1010,
                0x0301230249420426,
                0x01340a3084204282,
                false,
                true,
            ),
            (
                0x4038ea62022e8480,
                0x4098130044062cf8,
                0x2400009810006028,
                0xb200606800900100,
                true,
                true,
            ),
            (
                0x502000190002d410,
                0x0438100a01024d00,
                0x2217c2025085020a,
                0xa302e11110002008,
                false,
                false,
            ),
            (
                0x4200400240411212,
                0xb816804201c00229,
                0x94401924308a01c8,
                0x41203911e0009114,
                true,
                true,
            ),
            (
                0x00181012e8048110,
                0xa040200b8c000504,
                0xe2c08424148b3621,
                0x04a6473461be288b,
                false,
                false,
            ),
            (
                0x118930450a104281,
                0x601aa1629118e100,
                0x0072c190b1208908,
                0x8125461c400018cd,
                false,
                true,
            ),
            (
                0x6420649001148862,
                0xb8140a29851b311c,
                0x93c9180820881088,
                0x014040400a000040,
                true,
                true,
            ),
            (
                0x080622a043c60190,
                0x2103c10f04000312,
                0x1120404098087809,
                0x00000090f8918000,
                false,
                false,
            ),
            (
                0xc19e4204800b0b88,
                0x008040504c102020,
                0x3000844216406441,
                0x4e450203006dc014,
                false,
                false,
            ),
            (
                0xc0204c082c200c01,
                0x13046c600e0044c1,
                0x01cb111600005240,
                0x8012028130c18800,
                false,
                false,
            ),
            (
                0x80e1850014a56020,
                0x20055110c8011012,
                0x240422904200918e,
                0x10d02c21213442a0,
                true,
                true,
            ),
        ];
        let mut diffs = Vec::new();
        for &(in1_limb1, in1_limb2, in2_limb1, in2_limb2, res1, res2) in tests.iter() {
            in1.limbs_mut()[1] = in1_limb1;
            in1.limbs_mut()[0] = in1_limb2;
            in2.limbs_mut()[1] = in2_limb1;
            in2.limbs_mut()[0] = in2_limb2;
            let test_res1 = in1.sign_rule(&in2);
            let test_res2 = in2.sign_rule(&in1);
            let res = (res1, res2);
            let test_res = (test_res1, test_res2);
            let tuple = (in1_limb1, in1_limb2, in2_limb1, in2_limb2);
            let popcnts = (
                in1_limb1.count_ones() % 2,
                in1_limb2.count_ones() % 2,
                in2_limb1.count_ones() % 2,
                in2_limb2.count_ones() % 2,
            );
            if res != test_res {
                diffs.push((tuple, popcnts, res, test_res))
            }
        }
        if !diffs.is_empty() {
            let formatter = diffs
                .iter()
                .format_with("\n", |(tuple, popcnts, res, test_res), f| {
                    f(&format_args!(
                        "   Inputs: {tuple:x?}\n      expected {res:?}, got {test_res:?}. \
                         popcnts: {popcnts:?}"
                    ))
                });
            panic!("\nFailed test cases:\n {formatter}");
        }
    }
}
