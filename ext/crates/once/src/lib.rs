//! # Once Crate
//!
//! The `once` crate provides thread-safe data structures that are designed for write-once semantics,
//! making them particularly useful for concurrent programming scenarios where data is computed once
//! and then read many times.
//!
//! ## Key Components
//!
//! - [`OnceVec`]: A thread-safe vector that allows pushing elements in a way that ensures they are
//!   safely visible to other threads.
//! - [`OnceBiVec`]: A bidirectional vector built on top of `OnceVec` that supports negative indices.
//! - [`MultiIndexed`]: A multi-dimensional array that allows efficient storage and retrieval of values
//!   using multi-dimensional coordinates.
//! - [`Grove`] and [`TwoEndedGrove`]: Specialized collections that support the implementation of the
//!   other data structures.
//!
//! ## Concurrency Support
//!
//! This crate is designed with concurrency in mind and uses atomic operations and locks to ensure
//! thread safety. It can be used with the `concurrent` feature to enable parallel operations via
//! the `rayon` crate.
//!
//! ## Testing with Loom
//!
//! The crate supports testing with the `loom` concurrency testing framework. Loom tests can be run with:
//! ```bash
//! RUSTFLAGS="--cfg loom" cargo test --release --features loom -- loom
//! ```

// The loom tests require `--cfg loom` to be passed to `rustc`. However, this is a nonstandard cfg
// flag that we need to explicitly allow.
#![allow(unexpected_cfgs)]
#![deny(clippy::use_self, unsafe_op_in_unsafe_fn)]

pub mod grove;
pub mod multiindexed;
pub mod once;
pub mod write_once;

mod std_or_loom;

pub use grove::{Grove, TwoEndedGrove};
pub use multiindexed::MultiIndexed;
pub use once::{OnceBiVec, OnceVec};

#[cfg(test)]
mod test_utils {
    pub fn num_threads() -> usize {
        std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(2)
    }

    pub fn values_per_thread() -> usize {
        #[cfg(not(miri))]
        return 1000;

        // Miri is slow, so we reduce the number of values per thread to speed up the tests.
        #[cfg(miri)]
        return 10;
    }
}
