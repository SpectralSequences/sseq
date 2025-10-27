pub mod prelude {
    pub use rayon::iter::{IndexedParallelIterator, ParallelIterator};
    use rayon::prelude::*;

    pub trait MaybeParallelIterator: ParallelIterator {}

    pub trait MaybeIndexedParallelIterator: IndexedParallelIterator {}

    pub trait IntoMaybeParallelIterator: IntoParallelIterator + Sized {
        fn into_maybe_par_iter(self) -> Self::Iter {
            self.into_par_iter()
        }
    }

    pub type MaybeIterBridge<I> = rayon::iter::IterBridge<I>;

    pub trait MaybeParallelBridge: ParallelBridge {
        fn maybe_par_bridge(self) -> MaybeIterBridge<Self> {
            self.par_bridge()
        }
    }

    pub trait MaybeParallelSliceMut<T: Send>: ParallelSliceMut<T> {
        fn maybe_par_chunks_mut<'data>(
            &'data mut self,
            chunk_size: usize,
        ) -> impl MaybeIndexedParallelIterator<Item = &'data mut [T]>
        where
            T: 'data,
        {
            self.par_chunks_mut(chunk_size)
        }
    }

    // Implementations

    impl<I: ParallelIterator> MaybeParallelIterator for I {}

    impl<I: IndexedParallelIterator> MaybeIndexedParallelIterator for I {}

    impl<I: IntoParallelIterator> IntoMaybeParallelIterator for I {}

    impl<I: ParallelBridge> MaybeParallelBridge for I {}

    impl<T: Send> MaybeParallelSliceMut<T> for [T] {}
}

pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    rayon::join(oper_a, oper_b)
}

pub type Scope<'scope> = rayon::Scope<'scope>;

pub fn scope<'scope, OP, R>(op: OP) -> R
where
    OP: FnOnce(&Scope<'scope>) -> R + Send,
    R: Send,
{
    rayon::scope(op)
}

pub fn in_place_scope<'scope, OP, R>(op: OP) -> R
where
    OP: FnOnce(&Scope<'scope>) -> R,
{
    rayon::in_place_scope(op)
}
