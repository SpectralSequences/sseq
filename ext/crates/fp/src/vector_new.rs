use crate::prime::ValidPrime;
use crate::vector_inner::{
    entries_per_64_bits, FpVectorIterator, FpVectorNonZeroIterator, FpVectorP, SliceMutP, SliceP,
};
use itertools::Itertools;

macro_rules! dispatch_vector_inner {
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident(&mut self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(ref mut x), $other::_2(ref y)) => x.$method(y, $($arg),*),
                (Self::_3(ref mut x), $other::_3(ref y)) => x.$method(y, $($arg),*),
                (Self::_5(ref mut x), $other::_5(ref y)) => x.$method(y, $($arg),*),
                (Self::_7(ref mut x), $other::_7(ref y)) => x.$method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self, other: $other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, other: $other, $($arg: $ty),* ) $(-> $ret)* {
            match (self, other) {
                (Self::_2(ref mut x), $other::_2(y)) => x.$method(y, $($arg),*),
                (Self::_3(ref mut x), $other::_3(y)) => x.$method(y, $($arg),*),
                (Self::_5(ref mut x), $other::_5(y)) => x.$method(y, $($arg),*),
                (Self::_7(ref mut x), $other::_7(y)) => x.$method(y, $($arg),*),
                (l, r) => {
                    panic!("Applying {} to vectors over different primes ({} and {})", stringify!($method), l.prime(), r.prime());
                }
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(ref mut x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(ref mut x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(ref mut x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(ref mut x) => $ret::_7(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {
            match self {
                Self::_2(ref x) => $ret::_2(x.$method($($arg),*)),
                Self::_3(ref x) => $ret::_3(x.$method($($arg),*)),
                Self::_5(ref x) => $ret::_5(x.$method($($arg),*)),
                Self::_7(ref x) => $ret::_7(x.$method($($arg),*)),
            }
        }
    };
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(ref mut x) => x.$method($($arg),*),
                Self::_3(ref mut x) => x.$method($($arg),*),
                Self::_5(ref mut x) => x.$method($($arg),*),
                Self::_7(ref mut x) => x.$method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(ref x) => x.$method($($arg),*),
                Self::_3(ref x) => x.$method($($arg),*),
                Self::_5(ref x) => x.$method($($arg),*),
                Self::_7(ref x) => x.$method($($arg),*),
            }
        }
    };
    ($vis:vis fn $method:ident(self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {
        #[allow(unused_parens)]
        $vis fn $method(self, $($arg: $ty),* ) $(-> $ret)* {
            match self {
                Self::_2(x) => x.$method($($arg),*),
                Self::_3(x) => x.$method($($arg),*),
                Self::_5(x) => x.$method($($arg),*),
                Self::_7(x) => x.$method($($arg),*),
            }
        }
    }
}

macro_rules! dispatch_vector {
    () => {};
    ($vis:vis fn $method:ident $tt:tt $(-> $ret:tt)?; $($tail:tt)*) => {
        dispatch_vector_inner! {
            $vis fn $method $tt $(-> $ret)*
        }
        dispatch_vector!{$($tail)*}
    }
}

macro_rules! match_p {
    ($p:ident, $($val:tt)*) => {
        match *$p {
            2 => Self::_2($($val)*),
            3 => Self::_3($($val)*),
            5 => Self::_5($($val)*),
            7 => Self::_7($($val)*),
            _ => panic!("Prime not supported: {}", *$p)
        }
    }
}

#[derive(Eq, PartialEq, Clone)]
pub enum FpVector {
    _2(FpVectorP<2>),
    _3(FpVectorP<3>),
    _5(FpVectorP<5>),
    _7(FpVectorP<7>),
}

#[derive(Copy, Clone)]
pub enum Slice<'a> {
    _2(SliceP<'a, 2>),
    _3(SliceP<'a, 3>),
    _5(SliceP<'a, 5>),
    _7(SliceP<'a, 7>),
}

pub enum SliceMut<'a> {
    _2(SliceMutP<'a, 2>),
    _3(SliceMutP<'a, 3>),
    _5(SliceMutP<'a, 5>),
    _7(SliceMutP<'a, 7>),
}

impl FpVector {
    pub fn new(p: ValidPrime, dim: usize) -> FpVector {
        match_p!(p, FpVectorP::new(dim))
    }

    pub fn from_slice(p: ValidPrime, slice: &[u32]) -> Self {
        match_p!(p, FpVectorP::from(&slice))
    }

    fn from_limbs(p: ValidPrime, dim: usize, limbs: Vec<u64>) -> Self {
        match_p!(p, FpVectorP::from_limbs(dim, limbs))
    }

    dispatch_vector! {
        pub fn prime(&self) -> u32;
        pub fn dimension(&self) -> usize;
        pub fn scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn entry(&self, index: usize) -> u32;
        pub fn set_entry(&mut self, index: usize, value: u32);
        pub fn assign(&mut self, other: &Self);
        pub fn add(&mut self, other: &Self, c: u32);
        pub fn slice(&self, start: usize, end: usize) -> (dispatch Slice);
        pub fn as_slice(&self) -> (dispatch Slice);
        pub fn slice_mut(&mut self, start: usize, end: usize) -> (dispatch SliceMut);
        pub fn as_slice_mut(&mut self) -> (dispatch SliceMut);
        pub fn is_zero(&self) -> bool;
        pub fn iter(&self) -> FpVectorIterator;
        pub fn iter_nonzero(&self) -> FpVectorNonZeroIterator;

        fn limbs(&self) -> (&[u64]);
    }
}

impl<'a> Slice<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn dimension(&self) -> usize;
        pub fn entry(&self, index: usize) -> u32;
        pub fn iter(self) -> (FpVectorIterator<'a>);
        pub fn iter_nonzero(self) -> (FpVectorNonZeroIterator<'a>);
        pub fn is_zero(&self) -> bool;
    }
}

impl<'a> SliceMut<'a> {
    dispatch_vector! {
        pub fn prime(&self) -> ValidPrime;
        pub fn scale(&mut self, c: u32);
        pub fn set_to_zero(&mut self);
        pub fn add(&mut self, other: Slice, c: u32);
        pub fn assign(&mut self, other: Slice);
        pub fn set_entry(&mut self, index: usize, value: u32);
        pub fn as_slice(&self) -> (dispatch Slice);
    }
}

impl std::fmt::Display for FpVector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.as_slice().fmt(f)
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "[{}]", self.iter().join(", "))?;
        Ok(())
    }
}

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl Save for FpVector {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.dimension().save(buffer)?;
        for limb in self.limbs() {
            limb.save(buffer)?;
        }
        Ok(())
    }
}

impl Load for FpVector {
    type AuxData = ValidPrime;

    fn load(buffer: &mut impl Read, p: &ValidPrime) -> io::Result<Self> {
        let p = *p;

        let dimension = usize::load(buffer, &())?;

        if dimension == 0 {
            return Ok(FpVector::new(p, 0));
        }

        let entries_per_64_bits = entries_per_64_bits(p);
        let num_limbs = (dimension - 1) / entries_per_64_bits + 1;
        let mut limbs: Vec<u64> = Vec::with_capacity(num_limbs);

        for _ in 0..num_limbs {
            limbs.push(u64::load(buffer, &())?);
        }

        Ok(FpVector::from_limbs(p, dimension, limbs))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::vector_inner::*;
    use rand::Rng;
    use rstest::rstest;

    pub struct VectorDiffEntry {
        pub index: usize,
        pub left: u32,
        pub right: u32,
    }

    impl FpVector {
        pub fn diff_list(&self, other: &[u32]) -> Vec<VectorDiffEntry> {
            assert!(self.dimension() == other.len());
            let mut result = Vec::new();
            #[allow(clippy::needless_range_loop)]
            for index in 0..self.dimension() {
                let left = self.entry(index);
                let right = other[index];
                if left != right {
                    result.push(VectorDiffEntry { index, left, right });
                }
            }
            result
        }

        pub fn diff_vec(&self, other: &FpVector) -> Vec<VectorDiffEntry> {
            assert!(self.dimension() == other.dimension());
            let mut result = Vec::new();
            for index in 0..self.dimension() {
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
                        f(&format_args!("  At index {}: {}!={}", index, left, right))
                    });
            format!("{}", data_formatter)
        }

        pub fn assert_list_eq(&self, other: &[u32]) {
            let diff = self.diff_list(other);
            if diff.is_empty() {
                return;
            }
            println!("assert {} == {:?}", self, other);
            println!("{}", FpVector::format_diff(diff));
        }

        pub fn assert_vec_eq(&self, other: &FpVector) {
            let diff = self.diff_vec(other);
            if diff.is_empty() {
                return;
            }
            println!("assert {} == {}", self, other);
            println!("{}", FpVector::format_diff(diff));
        }
    }

    fn random_vector(p: u32, dimension: usize) -> Vec<u32> {
        let mut result = Vec::with_capacity(dimension);
        let mut rng = rand::thread_rng();
        for _ in 0..dimension {
            result.push(rng.gen::<u32>() % p);
        }
        result
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]
    fn test_add(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);
            let mut v = FpVector::from_slice(p_, &v_arr);
            let w = FpVector::from_slice(p_, &w_arr);

            v.add(&w, 1);
            for i in 0..dim {
                v_arr[i] = (v_arr[i] + w_arr[i]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_scale(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let mut v_arr = random_vector(p, dim);
            let mut rng = rand::thread_rng();
            let c = rng.gen::<u32>() % p;

            let mut v = FpVector::from_slice(p_, &v_arr);
            v.scale(c);
            for entry in &mut v_arr {
                *entry = (*entry * c) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_entry(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for &dim in &dim_list {
            let v_arr = random_vector(p, dim);
            let v = FpVector::from_slice(p_, &v_arr);

            let mut diffs = Vec::new();
            for (i, val) in v.iter().enumerate() {
                if v.entry(i) != val {
                    diffs.push((i, val, v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))] //
    fn test_entry_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            let v_arr = random_vector(p, dim);
            let v = FpVector::from_slice(p_, &v_arr);
            let v = v.slice(slice_start, slice_end);
            println!(
                "slice_start: {}, slice_end: {}, slice: {}",
                slice_start, slice_end, v
            );

            let mut diffs = Vec::new();
            for i in 0..v.dimension() {
                if v.entry(i) != v_arr[i + slice_start] {
                    diffs.push((i, v_arr[i + slice_start], v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_set_entry(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for &dim in &dim_list {
            let mut v = FpVector::new(p_, dim);
            let v_arr = random_vector(p, dim);
            for (i, &val) in v_arr.iter().enumerate() {
                v.set_entry(i, val);
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))] //
    fn test_set_entry_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            let mut v = FpVector::new(p_, dim);
            let mut v = v.slice_mut(slice_start, slice_end);

            let slice_dim = v.as_slice().dimension();
            let v_arr = random_vector(p, slice_dim);
            for (i, &val) in v_arr.iter().enumerate() {
                v.set_entry(i, val);
            }
            let v = v.as_slice();

            // println!("slice_start: {}, slice_end: {}, slice: {}", slice_start, slice_end, v);
            let mut diffs = Vec::new();
            for (i, &val) in v_arr.iter().enumerate() {
                if v.entry(i) != val {
                    diffs.push((i, val, v.entry(i)));
                }
            }
            assert_eq!(diffs, []);
        }
    }

    // Tests set_to_zero for a slice and also is_zero.
    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_set_to_zero_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            println!("slice_start : {}, slice_end : {}", slice_start, slice_end);
            let mut v_arr = random_vector(p, dim);
            v_arr[0] = 1; // make sure that v isn't zero
            let mut v = FpVector::from_slice(p_, &v_arr);

            v.slice_mut(slice_start, slice_end).set_to_zero();
            assert!(v.slice(slice_start, slice_end).is_zero());

            assert!(!v.is_zero()); // The first entry is 1, so it's not zero.
            for entry in &mut v_arr[slice_start..slice_end] {
                *entry = 0;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]//
    fn test_add_slice_to_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            let mut v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);

            let mut v = FpVector::from_slice(p_, &v_arr);
            let w = FpVector::from_slice(p_, &w_arr);

            v.slice_mut(slice_start, slice_end)
                .add(w.slice(slice_start, slice_end), 1);

            for i in slice_start..slice_end {
                v_arr[i] = (v_arr[i] + w_arr[i]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    // Tests assign and Eq
    #[rstest(p, case(2), case(3), case(5), case(7))] //
    fn test_assign(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        for &dim in &[10, 20, 70, 100, 1000] {
            println!("p: {}, dim: {}", p, dim);
            let v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);

            let mut v = FpVector::from_slice(p_, &v_arr);
            let w = FpVector::from_slice(p_, &w_arr);

            v.assign(&w);
            v.assert_vec_eq(&w);
        }
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]//
    fn test_assign_slice_to_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;

            let mut v_arr = random_vector(p, dim);
            let mut w_arr = random_vector(p, dim);

            v_arr[0] = 1; // Ensure v != w.
            w_arr[0] = 0; // Ensure v != w.

            let mut v = FpVector::from_slice(p_, &v_arr);
            let w = FpVector::from_slice(p_, &w_arr);

            v.slice_mut(slice_start, slice_end)
                .assign(w.slice(slice_start, slice_end));
            v_arr[slice_start..slice_end].clone_from_slice(&w_arr[slice_start..slice_end]);
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]
    fn test_add_shift_right(p: u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            let mut v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);

            let mut v = FpVector::from_slice(p_, &v_arr);
            let w = FpVector::from_slice(p_, &w_arr);

            v.slice_mut(slice_start + 2, slice_end + 2)
                .add(w.slice(slice_start, slice_end), 1);

            println!("v : {}", v);
            for i in slice_start + 2..slice_end + 2 {
                v_arr[i] = (v_arr[i] + w_arr[i - 2]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]
    fn test_add_shift_left(p: u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [10, 20, 70, 100, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            let mut v_arr = random_vector(p, dim);
            let w_arr = random_vector(p, dim);

            let mut v = FpVector::from_slice(p_, &v_arr);
            let w = FpVector::from_slice(p_, &w_arr);

            v.slice_mut(slice_start - 2, slice_end - 2)
                .add(w.slice(slice_start, slice_end), 1);
            for i in slice_start - 2..slice_end - 2 {
                v_arr[i] = (v_arr[i] + w_arr[i + 2]) % p;
            }
            v.assert_list_eq(&v_arr);
        }
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]
    fn test_iterator_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let ep = entries_per_64_bits(p_);
        for &dim in &[5, 10, ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1] {
            let v_arr = random_vector(p, dim);
            let v = FpVector::from_slice(p_, &v_arr);
            let v = v.slice(3, dim - 1);

            println!("v: {:?}", v_arr);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                println!("i: {}, dim : {}", i, dim);
                assert_eq!(v.entry(i), x);
                counter += 1;
            }
            assert_eq!(counter, v.dimension());
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_iterator_skip(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let ep = entries_per_64_bits(p_);
        let dim = 5 * ep;
        for &num_skip in &[ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1, 6 * ep] {
            let v_arr = random_vector(p, dim);
            let v = FpVector::from_slice(p_, &v_arr);

            let mut w = v.iter();
            w.skip_n(num_skip);
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                assert_eq!(v.entry(i + num_skip), x);
                counter += 1;
            }
            if num_skip == 6 * ep {
                assert_eq!(counter, 0);
            } else {
                assert_eq!(counter, v.dimension() - num_skip);
            }
        }
    }

    #[rstest(p, case(2), case(3), case(5), case(7))]
    fn test_iterator(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let ep = entries_per_64_bits(p_);
        for &dim in &[0, 5, 10, ep, ep - 1, ep + 1, 3 * ep, 3 * ep - 1, 3 * ep + 1] {
            let v_arr = random_vector(p, dim);
            let v = FpVector::from_slice(p_, &v_arr);

            let w = v.iter();
            let mut counter = 0;
            for (i, x) in w.enumerate() {
                assert_eq!(v.entry(i), x);
                counter += 1;
            }
            assert_eq!(counter, v.dimension());
        }
    }

    #[rstest(p, case(2))] //, case(3), case(5))]//, case(7))]
    fn test_iter_nonzero_empty(p: u32) {
        let p_ = ValidPrime::new(p);
        let v = FpVector::new(p_, 0);
        for (_idx, _v) in v.iter_nonzero() {
            panic!();
        }
    }

    #[rstest(p, case(2))] //, case(7))]
    fn test_iter_nonzero_slice(p: u32) {
        let p_ = ValidPrime::new(p);
        initialize_limb_bit_index_table(p_);
        let mut v = FpVector::new(p_, 5);
        v.set_entry(0, 1);
        v.set_entry(1, 1);
        v.set_entry(2, 1);
        for (i, _) in v.slice(0, 1).iter_nonzero() {
            assert!(i == 0);
        }
    }

    #[rstest(p, case(2), case(3), case(5))] //, case(7))]
    fn test_iter_nonzero(p: u32) {
        let p_ = ValidPrime::new(p);
        println!("p : {}", p);
        initialize_limb_bit_index_table(p_);
        let dim_list = [20, 66, 100, 270, 1000];
        for (i, &dim) in dim_list.iter().enumerate() {
            let slice_start = [5, 10, 20, 30, 290][i];
            let slice_end = (dim + slice_start) / 2;
            let v_arr = random_vector(p, dim);
            let v = FpVector::from_slice(p_, &v_arr);

            println!("v: {}", v);
            println!("v_arr: {:?}", v_arr);
            let result: Vec<_> = v.slice(slice_start, slice_end).iter_nonzero().collect();
            let comparison_result: Vec<_> = (&v_arr[slice_start..slice_end])
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
                        diffs_str.push_str(&format!(
                            "\n({:?}) present in result, missing from comparison_result",
                            result[i]
                        ));
                        i += 1;
                    } else {
                        diffs_str.push_str(&format!(
                            "\n({:?}) present in comparison_result, missing from result",
                            comparison_result[j]
                        ));
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
            assert!(diffs_str.is_empty(), "{}", diffs_str);
        }
    }
}
