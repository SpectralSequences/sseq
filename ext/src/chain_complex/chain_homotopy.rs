use crate::chain_complex::{ChainComplex, FreeChainComplex};
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::Module;
use fp::vector::{FpVector, SliceMut};
use once::OnceVec;
use std::sync::Arc;
use std::sync::Mutex;

/// A chain homotopy from $f to g$, or equivalently a null-homotopy of $h = f - g$. A chain map is
/// a priori a collection of free module homomorphisms. However, instead of providing
/// FreeModuleHomomorphism objects, the user is expected to give a function that computes the value
/// of $h$ on each generator.
pub struct ChainHomotopy<S: FreeChainComplex, T: ChainComplex, F: Fn(u32, i32, usize, SliceMut)> {
    source: Arc<S>,
    target: Arc<T>,
    /// The $s$ shift of the original chain map $f - g$.
    shift_s: u32,
    /// The $t$ shift of the original chain map $f - g$.
    shift_t: i32,
    /// A function that given (s, t, idx, result), adds (f - g)(x_{s, t, i}), to `result`.
    map: F,
    lock: Mutex<()>,
    /// Homotopies, indexed by the filtration of the target of f - g.
    homotopies: OnceVec<FreeModuleHomomorphism<T::Module>>,
}

impl<
        S: FreeChainComplex,
        T: ChainComplex<Algebra = S::Algebra>,
        F: Fn(u32, i32, usize, SliceMut),
    > ChainHomotopy<S, T, F>
{
    pub fn new(source: Arc<S>, target: Arc<T>, shift_s: u32, shift_t: i32, map: F) -> Self {
        Self {
            source,
            target,
            shift_s,
            shift_t,
            map,
            lock: Mutex::new(()),
            homotopies: OnceVec::new(),
        }
    }

    /// Lift maps so that the chain *homotopy* is defined on `(max_source_s, max_source_t)`.
    pub fn extend(&self, max_source_s: u32, max_source_t: i32) {
        let _lock = self.lock.lock();

        let p = self.source.prime();

        // The bidegree of the target of f - g
        let max_target_s = max_source_s - self.shift_s;
        let max_target_t = max_source_t - self.shift_t;

        let mut scratch = FpVector::new(p, 0);

        self.homotopies.extend(max_target_s as usize, |s| {
            let s = s as u32;
            FreeModuleHomomorphism::new(
                self.source.module(s + self.shift_s),
                self.target.module(s + 1),
                self.shift_t,
            )
        });

        for target_s in 0..=max_target_s {
            for target_t in self.homotopies[target_s as usize].next_degree() - self.shift_t
                ..=max_target_t - (max_target_s - target_s) as i32
            {
                let source_s = target_s + self.shift_s;
                let source_t = target_t + self.shift_t;

                let num_gens = self
                    .source
                    .module(source_s)
                    .number_of_gens_in_degree(source_t);

                let target_dim = self.target.module(target_s + 1).dimension(target_t);
                let mut outputs = vec![FpVector::new(p, target_dim); num_gens];

                scratch.set_scratch_vector_size(self.target.module(target_s).dimension(target_t));

                for (i, row) in outputs.iter_mut().enumerate() {
                    (self.map)(source_s, source_t, i, scratch.as_slice_mut());

                    if target_s > 0 {
                        self.homotopies[target_s as usize - 1].apply(
                            scratch.as_slice_mut(),
                            *p - 1,
                            source_t,
                            self.source
                                .differential(source_s)
                                .output(source_t, i)
                                .as_slice(),
                        );
                    }

                    self.target.differential(target_s + 1).apply_quasi_inverse(
                        row.as_slice_mut(),
                        target_t,
                        scratch.as_slice(),
                    );
                    scratch.set_to_zero();
                }
                self.homotopies[target_s as usize].add_generators_from_rows(source_t, outputs);
            }
        }
    }

    pub fn homotopy(&self, source_s: u32) -> &FreeModuleHomomorphism<T::Module> {
        &self.homotopies[(source_s - self.shift_s) as usize]
    }
}
