pub mod prelude {
    pub trait MaybeParallelIterator: Iterator {}

    pub trait MaybeIndexedParallelIterator: Iterator {}

    pub trait IntoMaybeParallelIterator:
        IntoIterator<IntoIter: Send, Item: Send> + Send + Sized
    {
        fn into_maybe_par_iter(self) -> Self::IntoIter {
            self.into_iter()
        }
    }

    pub trait IntoMaybeParallelRefMutIterator<'data> {
        type Iter: MaybeParallelIterator<Item = Self::Item>;
        type Item: Send + 'data;

        // Required method
        fn maybe_par_iter_mut(&'data mut self) -> Self::Iter;
    }

    pub struct MaybeIterBridge<Iter>(Iter);

    pub trait MaybeParallelBridge: Sized {
        fn maybe_par_bridge(self) -> MaybeIterBridge<Self> {
            MaybeIterBridge(self)
        }
    }

    pub trait MaybeParallelSliceMut<T: Send> {
        fn maybe_par_chunks_mut<'data>(
            &'data mut self,
            chunk_size: usize,
        ) -> impl MaybeIndexedParallelIterator<Item = &'data mut [T]>
        where
            T: 'data;
    }

    // Implementations

    impl<I: Iterator> MaybeParallelIterator for I {}

    impl<I: Iterator> MaybeIndexedParallelIterator for I {}

    impl<I: IntoIterator<Item: Send, IntoIter: Send> + Send> IntoMaybeParallelIterator for I {}

    impl<'data, I: 'data + ?Sized> IntoMaybeParallelRefMutIterator<'data> for I
    where
        &'data mut I: IntoMaybeParallelIterator,
    {
        type Item = <&'data mut I as IntoIterator>::Item;
        type Iter = <&'data mut I as IntoIterator>::IntoIter;

        fn maybe_par_iter_mut(&'data mut self) -> Self::Iter {
            self.into_maybe_par_iter()
        }
    }

    impl<Iter: Iterator> Iterator for MaybeIterBridge<Iter> {
        type Item = Iter::Item;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }
    }

    impl<T: Iterator> MaybeParallelBridge for T {}

    impl<T: Send> MaybeParallelSliceMut<T> for [T] {
        fn maybe_par_chunks_mut<'data>(
            &'data mut self,
            chunk_size: usize,
        ) -> impl MaybeIndexedParallelIterator<Item = &'data mut [T]>
        where
            T: 'data,
        {
            self.chunks_mut(chunk_size)
        }
    }
}

#[allow(dead_code)]
pub struct Scope<'scope>(&'scope ());

impl<'scope> Scope<'scope> {
    pub fn spawn<BODY>(&self, body: BODY)
    where
        BODY: FnOnce(&Self) + Send + 'scope,
    {
        body(self)
    }
}

pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    (oper_a(), oper_b())
}

pub fn scope<'scope, OP, R>(op: OP) -> R
where
    OP: FnOnce(&Scope<'scope>) -> R + Send,
    R: Send,
{
    op(&Scope(&()))
}

pub fn in_place_scope<'scope, OP, R>(op: OP) -> R
where
    OP: FnOnce(&Scope<'scope>) -> R,
{
    op(&Scope(&()))
}
