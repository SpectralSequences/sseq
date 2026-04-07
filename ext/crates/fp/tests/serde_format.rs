//! Tests for the serde impls on `FpVector`, `Matrix`, `Subspace`, and `QuasiInverse`.
//!
//! Two kinds of tests:
//!
//! 1. **Round-trip** (`*_roundtrip`). Serialize a value to JSON, deserialize it back, and assert
//!    equality. Driven by `proptest` using the existing `Arbitrary` impls on `FqVector`, `Matrix`,
//!    and `Subspace`. These verify that the serde impls are mutually consistent across a broad
//!    spread of primes, dimensions, and limb counts.
//!
//! 2. **Golden format** (`*_json_format`). Serialize a known small value to JSON and compare
//!    against an expected string pinned via `expect-test`. These verify that the on-the-wire format
//!    doesn't drift silently — any future change to the serde representation must be explicitly
//!    acknowledged by running `UPDATE_EXPECT=1 cargo test -p fp --test serde_format`. Includes F_2
//!    vectors spanning multiple limbs to catch mistakes in the multi-limb encoding used by
//!    `FqVector::limbs` / `Matrix::data`.

use expect_test::expect;
use fp::{
    field::Fp,
    matrix::{
        Matrix, QuasiInverse, Subspace,
        arbitrary::{MatrixArbParams, SubspaceArbParams},
    },
    prime::ValidPrime,
    vector::{FpVector, FqVector, arbitrary::FqVectorArbParams},
};
use proptest::prelude::*;

fn p(n: u32) -> ValidPrime {
    ValidPrime::new(n)
}

// ---------- Proptest strategies ----------

/// Arbitrary `FpVector` with length up to 300 (covers 0–5 limbs at F_2).
fn arb_fpvector() -> impl Strategy<Value = FpVector> {
    any_with::<FqVector<Fp<ValidPrime>>>(FqVectorArbParams {
        fq: None,
        len: (0..=300usize).boxed(),
    })
    .prop_map(|v| v.into())
}

/// Arbitrary small `Matrix` (dimensions capped so the generated JSON stays manageable).
fn arb_matrix() -> impl Strategy<Value = Matrix> {
    any_with::<Matrix>(MatrixArbParams {
        p: None,
        rows: (0..=20usize).boxed(),
        columns: (0..=20usize).boxed(),
    })
}

/// Arbitrary small `Subspace`.
fn arb_subspace() -> impl Strategy<Value = Subspace> {
    any_with::<Subspace>(SubspaceArbParams {
        p: None,
        dim: (0..=20usize).boxed(),
    })
}

/// Arbitrary `QuasiInverse` whose `image` field is either `None` or an arbitrary `Vec<isize>`.
///
/// We don't enforce semantic consistency between `image` and `preimage` because this test only
/// exercises serde, and the serialization treats both fields as opaque data.
fn arb_quasi_inverse() -> impl Strategy<Value = QuasiInverse> {
    let image = proptest::option::of(proptest::collection::vec(-1isize..100, 0..20usize));
    (arb_matrix(), image).prop_map(|(m, image)| QuasiInverse::new(image, m))
}

// ---------- Round-trip tests ----------

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn fpvector_roundtrip(v in arb_fpvector()) {
        let s = serde_json::to_string(&v).unwrap();
        let v2: FpVector = serde_json::from_str(&s).unwrap();
        prop_assert_eq!(v, v2);
    }

    #[test]
    fn matrix_roundtrip(m in arb_matrix()) {
        let s = serde_json::to_string(&m).unwrap();
        let m2: Matrix = serde_json::from_str(&s).unwrap();
        prop_assert_eq!(m, m2);
    }

    #[test]
    fn subspace_roundtrip(s in arb_subspace()) {
        let json = serde_json::to_string(&s).unwrap();
        let s2: Subspace = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, s2);
    }

    #[test]
    fn quasi_inverse_roundtrip(qi in arb_quasi_inverse()) {
        let s = serde_json::to_string(&qi).unwrap();
        let qi2: QuasiInverse = serde_json::from_str(&s).unwrap();
        prop_assert_eq!(qi, qi2);
    }
}

// ---------- Golden format tests ----------
//
// To update after an intentional format change:
//
//     UPDATE_EXPECT=1 cargo test -p fp --test serde_format

#[test]
fn fpvector_p2_single_limb_json_format() {
    let v = FpVector::from_slice(p(2), &[1, 0, 1, 1, 0]);
    let s = serde_json::to_string(&v).unwrap();
    expect![[r#"{"fq":{"p":2},"len":5,"limbs":[13]}"#]].assert_eq(&s);
}

/// F_2 vector spanning exactly two limbs (entries 0..64 in limb 0, entries 64..128 in limb 1).
#[test]
fn fpvector_p2_two_limbs_json_format() {
    let mut entries = vec![0u32; 128];
    entries[0] = 1;
    entries[1] = 1;
    entries[63] = 1;
    entries[64] = 1;
    entries[127] = 1;
    let v = FpVector::from_slice(p(2), &entries);
    let s = serde_json::to_string(&v).unwrap();
    expect![[r#"{"fq":{"p":2},"len":128,"limbs":[9223372036854775811,9223372036854775809]}"#]]
        .assert_eq(&s);
}

/// F_2 vector straddling three limbs (130 entries, with a bit set in every limb).
#[test]
fn fpvector_p2_three_limbs_json_format() {
    let mut entries = vec![0u32; 130];
    entries[5] = 1;
    entries[70] = 1;
    entries[129] = 1;
    let v = FpVector::from_slice(p(2), &entries);
    let s = serde_json::to_string(&v).unwrap();
    expect![[r#"{"fq":{"p":2},"len":130,"limbs":[32,64,2]}"#]].assert_eq(&s);
}

#[test]
fn fpvector_p3_json_format() {
    let v = FpVector::from_slice(p(3), &[1, 2, 0, 2, 1]);
    let s = serde_json::to_string(&v).unwrap();
    expect![[r#"{"fq":{"p":3},"len":5,"limbs":[5137]}"#]].assert_eq(&s);
}

#[test]
fn fpvector_p5_json_format() {
    let v = FpVector::from_slice(p(5), &[4, 2, 0, 3]);
    let s = serde_json::to_string(&v).unwrap();
    expect![[r#"{"fq":{"p":5},"len":4,"limbs":[98372]}"#]].assert_eq(&s);
}

#[test]
fn matrix_p2_json_format() {
    let m = Matrix::from_vec(p(2), &[vec![1, 0, 1], vec![0, 1, 1]]);
    let s = serde_json::to_string(&m).unwrap();
    expect![[
        r#"{"fp":{"p":2},"rows":2,"physical_rows":2,"columns":3,"data":[5,6],"stride":1,"pivots":[]}"#
    ]]
    .assert_eq(&s);
}

#[test]
fn quasi_inverse_p2_json_format() {
    let preimage = Matrix::from_vec(p(2), &[vec![1, 0, 1], vec![0, 1, 1]]);
    let qi = QuasiInverse::new(Some(vec![0, 1, -1]), preimage);
    let s = serde_json::to_string(&qi).unwrap();
    expect![[
        r#"{"image":[0,1,-1],"preimage":{"fp":{"p":2},"rows":2,"physical_rows":2,"columns":3,"data":[5,6],"stride":1,"pivots":[]}}"#
    ]]
    .assert_eq(&s);
}
