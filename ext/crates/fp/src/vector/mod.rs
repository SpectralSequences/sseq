pub mod inner;

mod fp_wrapper;
mod impl_fqslice;
mod impl_fqslicemut;
mod impl_fqvector;
mod iter;

pub use fp_wrapper::*;
#[cfg(feature = "proptest")]
pub use impl_fqvector::arbitrary;
pub use inner::*;

#[cfg(test)]
pub(super) mod tests {
    use itertools::Itertools;
    use proptest::prelude::*;

    use super::{arbitrary::MAX_LEN as MAX_TEST_VEC_LEN, inner::FqVector};
    use crate::{
        field::{element::FieldElement, fp::F2, Field},
        limb,
    };

    pub struct VectorDiffEntry<F: Field> {
        pub index: usize,
        pub left: FieldElement<F>,
        pub right: FieldElement<F>,
    }

    impl<F: Field> FqVector<F> {
        pub fn diff_list(&self, other: &[FieldElement<F>]) -> Vec<VectorDiffEntry<F>> {
            assert!(self.len() == other.len());
            let mut result = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for index in 0..self.len() {
                let left = self.entry(index);
                let right = other[index].clone();
                if left != right {
                    result.push(VectorDiffEntry { index, left, right });
                }
            }
            result
        }

        pub fn diff_vec(&self, other: &Self) -> Vec<VectorDiffEntry<F>> {
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

        pub fn format_diff(diff: Vec<VectorDiffEntry<F>>) -> String {
            let data_formatter =
                diff.iter()
                    .format_with("\n ", |VectorDiffEntry { index, left, right }, f| {
                        f(&format_args!("  At index {index}: {left}!={right}"))
                    });
            format!("{data_formatter}")
        }

        pub fn assert_list_eq(&self, other: &[FieldElement<F>]) {
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

    /// An arbitrary (field, dimension) pair
    fn arb_field_dim<F: Field>() -> impl Strategy<Value = (F, usize)> {
        (any::<F>(), 0..=MAX_TEST_VEC_LEN)
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

    /// An arbitrary vector of length `dimension` containing values in the field `fq`. The tests
    /// take in a `Vec` instead of an `FqVector` directly because they will usually apply some
    /// operation on both the `FqVector` and the original `Vec` and then compare the results.
    fn arb_element_vec<F: Field>(
        fq: F,
        dimension: usize,
    ) -> impl Strategy<Value = Vec<FieldElement<F>>> {
        proptest::collection::vec(fq.arb_element(), dimension)
    }

    /// A pair of a field `fq` and a vector containing values in that field. In other words, a
    /// vector over `fq`.
    pub fn arb_vec<F: Field>() -> impl Strategy<Value = (F, Vec<FieldElement<F>>)> {
        arb_field_dim().prop_flat_map(|(fq, dim)| (Just(fq), arb_element_vec(fq, dim)))
    }

    /// An Fq vector together with valid slice indices
    fn arb_vec_and_slice<F: Field>(
    ) -> impl Strategy<Value = (F, Vec<FieldElement<F>>, (usize, usize))> {
        arb_field_dim()
            .prop_flat_map(|(fq, dim)| (Just(fq), arb_element_vec(fq, dim), arb_slice(dim)))
    }

    /// A pair of vectors of the same length over the same field
    fn arb_vec_pair<F: Field>(
    ) -> impl Strategy<Value = (F, Vec<FieldElement<F>>, Vec<FieldElement<F>>)> {
        arb_field_dim().prop_flat_map(|(fq, dim)| {
            (Just(fq), arb_element_vec(fq, dim), arb_element_vec(fq, dim))
        })
    }

    /// A pair of vectors of the same length over the same field, together with valid slice indices
    fn arb_vec_pair_and_slice<F: Field>() -> impl Strategy<
        Value = (
            F,
            Vec<FieldElement<F>>,
            Vec<FieldElement<F>>,
            (usize, usize),
        ),
    > {
        arb_field_dim().prop_flat_map(|(fq, dim)| {
            (
                Just(fq),
                arb_element_vec(fq, dim),
                arb_element_vec(fq, dim),
                arb_slice(dim),
            )
        })
    }

    /// A pair of vectors of the same length over the same field, together with a mask (in the sense
    /// of [`FqVector::add_masked`] and [`FqVector::add_unmasked`])
    fn arb_vec_pair_and_mask<F: Field>(
    ) -> impl Strategy<Value = (F, Vec<FieldElement<F>>, Vec<FieldElement<F>>, Vec<usize>)> {
        any::<F>()
            .prop_flat_map(|fq| (Just(fq), arb_slice(MAX_TEST_VEC_LEN)))
            .prop_flat_map(|(fq, (dim_small, dim_large))| {
                (
                    Just(fq),
                    arb_element_vec(fq, dim_small),
                    arb_element_vec(fq, dim_large),
                    proptest::collection::vec(0..dim_large, dim_small),
                )
            })
    }

    macro_rules! vector_tests {
        ($field:ty) => {
    proptest! {
        #![proptest_config(ProptestConfig {
            max_shrink_time: 30_000,
            max_shrink_iters: 1_000_000,
            .. ProptestConfig::default()
        })]

        // These "incompatible_fields" tests would lend themselves nicely to a macro, but it's
        // currently impossible to define the necessary macro inside another. See
        // https://github.com/rust-lang/rust/issues/35853.
        #[test]
        fn test_incompatible_fields_assign((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).assign(&FqVector::new(fq2, 10))
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_incompatible_fields_assign_partial((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).assign_partial(&FqVector::new(fq2, 10))
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_incompatible_fields_add((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).add(&FqVector::new(fq2, 10), fq2.one())
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_incompatible_fields_add_offset((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).add_offset(&FqVector::new(fq2, 10), fq2.one(), 5)
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_incompatible_fields_add_truncate((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).add_truncate(&FqVector::new(fq2, 10), fq2.one())
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_incompatible_fields_sign_rule((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).sign_rule(&FqVector::new(fq2, 10))
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_incompatible_fields_add_carry((fq1, fq2) in (any::<$field>(), any::<$field>())) {
            if fq1 != fq2 {
                let panic = std::panic::catch_unwind(|| {
                    FqVector::new(fq1, 10).add_carry::<FqVector<_>>(&FqVector::new(fq2, 10), fq2.one(), &mut [])
                });
                prop_assert!(panic.is_err());
            }
        }

        #[test]
        fn test_serialize((fq, v_arr) in arb_vec::<$field>()) {
            use std::io::{Seek, Cursor};

            let v = FqVector::from_slice(fq, &v_arr);

            let mut cursor = Cursor::new(Vec::<u8>::new());
            v.to_bytes(&mut cursor).unwrap();
            cursor.rewind().unwrap();

            let w = FqVector::from_bytes(v.fq(), v.len(), &mut cursor).unwrap();
            v.assert_vec_eq(&w);
        }

        #[test]
        fn test_add((fq, mut v_arr, w_arr) in arb_vec_pair::<$field>()) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            let w = FqVector::from_slice(fq, &w_arr);

            v.add(&w, fq.one());

            for (v_element, w_element) in v_arr.iter_mut().zip(w_arr.iter()) {
                *v_element += *w_element;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_scale((fq, mut v_arr, c) in arb_field_dim::<$field>().prop_flat_map(|(fq, dim)| {
            (Just(fq), arb_element_vec(fq, dim), fq.arb_element())
        })) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            v.scale(c);
            for entry in &mut v_arr {
                *entry *= c;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_scale_slice((fq, mut v_arr, (slice_start, slice_end), c) in
            arb_field_dim::<$field>().prop_flat_map(|(fq, dim)| {
                (Just(fq), arb_element_vec(fq, dim), arb_slice(dim), fq.arb_element())
            })
        ) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            v.slice_mut(slice_start, slice_end).scale(c);

            for entry in &mut v_arr[slice_start..slice_end] {
                *entry *= c;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_entry((fq, v_arr) in arb_vec::<$field>()) {
            let v = FqVector::from_slice(fq, &v_arr);

            let mut diffs = Vec::new();
            for (i, val) in v.iter().enumerate() {
                if v.entry(i) != val {
                    diffs.push((i, val, v.entry(i)));
                }
            }
            prop_assert_eq!(diffs, []);
        }

        #[test]
        fn test_entry_slice((fq, v_arr, (slice_start, slice_end)) in arb_vec_and_slice::<$field>()) {
            let v = FqVector::from_slice(fq, &v_arr);
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
        fn test_set_entry((fq, v_arr) in arb_vec::<$field>()) {
            let mut v = FqVector::new(fq, v_arr.len());

            for (i, &val) in v_arr.iter().enumerate() {
                v.set_entry(i, val);
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_set_entry_slice((fq, v_arr, (slice_start, slice_end)) in arb_vec_and_slice::<$field>()) {
            let dim = v_arr.len();
            let mut v = FqVector::new(fq, dim);
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
        fn test_set_to_zero_slice((fq, mut v_arr, (slice_start, slice_end)) in arb_vec_and_slice::<$field>()) {
            println!("slice_start : {slice_start}, slice_end : {slice_end}");
            let mut v = FqVector::from_slice(fq, &v_arr);

            v.slice_mut(slice_start, slice_end).set_to_zero();
            prop_assert!(v.slice(slice_start, slice_end).is_zero());

            for entry in &mut v_arr[slice_start..slice_end] {
                *entry = fq.zero();
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_add_slice_to_slice((fq, mut v_arr, w_arr, (slice_start, slice_end)) in arb_vec_pair_and_slice::<$field>()) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            let w = FqVector::from_slice(fq, &w_arr);

            v.slice_mut(slice_start, slice_end)
                .add(w.slice(slice_start, slice_end), fq.one());

            for i in slice_start..slice_end {
                v_arr[i] += w_arr[i];
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_assign((fq, v_arr, w_arr) in arb_vec_pair::<$field>()) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            let w = FqVector::from_slice(fq, &w_arr);

            v.assign(&w);
            v.assert_vec_eq(&w);
        }

        #[test]
        fn test_assign_partial((fq, v_arr, w_arr) in arb_vec_pair::<$field>()) {
            let dim = v_arr.len();
            let mut v = FqVector::from_slice(fq, &v_arr);
            let w = FqVector::from_slice(fq, &w_arr[0..(dim / 2)]);

            v.assign_partial(&w);
            prop_assert!(v.slice(dim / 2, dim).is_zero());
            prop_assert_eq!(v.len(), dim);
            v.slice(0, dim / 2).to_owned().assert_vec_eq(&w);
        }

        #[test]
        fn test_assign_slice_to_slice((fq, mut v_arr, w_arr, (slice_start, slice_end)) in arb_vec_pair_and_slice::<$field>()) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            let w = FqVector::from_slice(fq, &w_arr);

            v.slice_mut(slice_start, slice_end)
                .assign(w.slice(slice_start, slice_end));
            v_arr[slice_start..slice_end].clone_from_slice(&w_arr[slice_start..slice_end]);
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_add_shift((fq, mut v_arr, w_arr, [(slice1_start, slice1_end), (slice2_start, slice2_end)])
            in arb_field_dim::<$field>().prop_flat_map(|(fq, dim)| {
                (
                    Just(fq),
                    arb_element_vec(fq, dim),
                    arb_element_vec(fq, dim),
                    arb_slice_pair(dim),
                )
            })
        ) {
            let mut v = FqVector::from_slice(fq, &v_arr);
            let w = FqVector::from_slice(fq, &w_arr);

            v.slice_mut(slice1_start, slice1_end)
                .add(w.slice(slice2_start, slice2_end), fq.one());

            for (v_element, w_element) in v_arr[slice1_start..slice1_end]
                .iter_mut()
                .zip(w_arr[slice2_start..slice2_end].iter())
            {
                *v_element += *w_element;
            }
            v.assert_list_eq(&v_arr);
        }

        #[test]
        fn test_add_masked((fq, mut v_small, v_big, mask) in arb_vec_pair_and_mask::<$field>()) {
            let mut v = FqVector::from_slice(fq, &v_small);
            let w = FqVector::from_slice(fq, &v_big);

            v.as_slice_mut().add_masked(w.as_slice(), fq.one(), &mask);

            for (i, x) in v_small.iter_mut().enumerate() {
                *x += v_big[mask[i]];
            }

            v.assert_list_eq(&v_small);
        }

        #[test]
        fn test_add_unmasked((fq, v_small, mut v_big, mask) in arb_vec_pair_and_mask::<$field>()) {
            let mut v = FqVector::from_slice(fq, &v_big);
            let w = FqVector::from_slice(fq, &v_small);

            v.as_slice_mut().add_unmasked(w.as_slice(), fq.one(), &mask);
            for (i, &x) in v_small.iter().enumerate() {
                v_big[mask[i]] += x;
            }
            v.assert_list_eq(&v_big);
        }

        #[test]
        fn test_iterator_slice((p, v_arr, (slice_start, slice_end)) in arb_vec_and_slice::<$field>()) {
            let v = FqVector::from_slice(p, &v_arr);
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
        fn test_iterator_skip((p, v_arr, num_skip) in arb_field_dim::<$field>().prop_flat_map(|(p, dim)| {
            (Just(p), arb_element_vec(p, dim), 0..=dim)
        })) {
            let v = FqVector::from_slice(p, &v_arr);

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
        fn test_iterator((p, v_arr) in arb_vec::<$field>()) {
            let v = FqVector::from_slice(p, &v_arr);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                prop_assert_eq!(v.entry(i), x);
                counter += 1;
            }
            prop_assert_eq!(counter, v.len());
        }

        #[test]
        fn test_iter_nonzero_empty((p, dimension) in arb_field_dim::<$field>()) {
            let v = FqVector::new(p, dimension);
            prop_assert_eq!(v.iter_nonzero().next(), None);
        }

        #[test]
        fn test_iter_nonzero((fq, v_arr, (slice_start, slice_end)) in arb_vec_and_slice::<$field>()) {
            use std::fmt::Write;

            let v = FqVector::from_slice(fq, &v_arr);

            println!("v: {v}");
            println!("v_arr: {v_arr:?}");
            let result: Vec<_> = v.slice(slice_start, slice_end).iter_nonzero().collect();
            let comparison_result: Vec<_> = v_arr[slice_start..slice_end]
                .iter()
                .copied()
                .enumerate()
                .filter(|&(_, x)| x != fq.zero())
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
};
    }

    /// For a given field type generic over a prime type, and a given prime type, run our tests for
    /// vectors over that field.
    macro_rules! test_prime {
        ($fq:tt, $p:tt) => {
            paste::paste! {
                mod [<$p:lower>] {
                    use super::*;
                    use crate::prime::$p;

                    vector_tests!($fq<$p>);
                }
            }
        };
    }

    // For a given field type generic over a prime type, run our tests for it over P2, P3, P5, P7,
    // as well as ValidPrime. Note that this macro makes assumptions about the path to the field
    // type within the crate.
    macro_rules! test_field {
        ($fq:tt) => {
            paste::paste! {
                mod [<$fq:lower>] {
                    use super::*;
                    use crate::field::[<$fq:lower>]::$fq;

                    test_prime!($fq, ValidPrime);
                    test_prime!($fq, P2);
                    cfg_if::cfg_if! { if #[cfg(feature = "odd-primes")] {
                        test_prime!($fq, P3);
                        test_prime!($fq, P5);
                        test_prime!($fq, P7);
                    }}
                }
            }
        };
    }

    test_field!(Fp);
    test_field!(SmallFq);

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
        let mut in1 = FqVector::new(F2, 128);
        let mut in2 = FqVector::new(F2, 128);
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
