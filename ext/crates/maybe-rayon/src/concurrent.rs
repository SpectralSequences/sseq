pub mod prelude {
    use rayon::prelude::*;

    pub use rayon::iter::{IndexedParallelIterator, ParallelIterator};

    pub trait MaybeParallelIterator: ParallelIterator {}

    pub trait MaybeIndexedParallelIterator: IndexedParallelIterator {}

    pub trait MaybeIntoParallelIterator: IntoParallelIterator {
        fn maybe_into_par_iter(self) -> Self::Iter;
    }

    pub trait MaybeIntoParallelRefMutIterator<'data>: IntoParallelRefMutIterator<'data> {
        fn maybe_par_iter_mut(&'data mut self) -> Self::Iter;
    }

    // Implementations

    impl<I: ParallelIterator> MaybeParallelIterator for I {}

    impl<I: IndexedParallelIterator> MaybeIndexedParallelIterator for I {}

    impl<I: IntoParallelIterator> MaybeIntoParallelIterator for I {
        fn maybe_into_par_iter(self) -> Self::Iter {
            self.into_par_iter()
        }
    }

    impl<'data, I: IntoParallelRefMutIterator<'data> + ?Sized>
        MaybeIntoParallelRefMutIterator<'data> for I
    {
        fn maybe_par_iter_mut(&'data mut self) -> Self::Iter {
            self.par_iter_mut()
        }
    }
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
