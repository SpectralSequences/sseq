use std::cmp::Ordering;

/// Partially ordered type with greatest lower bounds, or meets.
pub trait MeetSemilattice: PartialOrd<Self> {
    fn meet(self, rhs: Self) -> Self;
}

pub fn meet<T>(v1: T, v2: T) -> T
where
    T: MeetSemilattice,
{
    T::meet(v1, v2)
}

/// Partially ordered type with least upper bounds, or joins.
pub trait JoinSemilattice: PartialOrd<Self> {
    fn join(self, rhs: Self) -> Self;
}

pub fn join<T>(v1: T, v2: T) -> T
where
    T: JoinSemilattice,
{
    T::join(v1, v2)
}

/// A lattice is simultaneously a `MeetSemilattice` and a `JoinSemilattice`.
pub trait Lattice: MeetSemilattice + JoinSemilattice {}

impl<T: MeetSemilattice + JoinSemilattice> Lattice for T {}

/// Marker trait to enable automatic Meet and Join Semilattice implementations for some types that
/// implement `Ord`. We don't blanket implement for all `T: Ord` because we might want to override
/// the default behavior, e.g. [`WithMax`].
pub trait LatticeFromOrd: Ord {}

macro_rules! impl_lattice_from_ord {
    ($t:ty) => {impl LatticeFromOrd for $t {}};
    ($t:ty, $($tail:ty),+) => {
        impl_lattice_from_ord!($t);
        impl_lattice_from_ord!($($tail),+);
    }
}

impl_lattice_from_ord!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize, bool);

impl<T: LatticeFromOrd> MeetSemilattice for T {
    fn meet(self, rhs: Self) -> Self {
        Self::min(self, rhs)
    }
}

impl<T: LatticeFromOrd> JoinSemilattice for T {
    fn join(self, rhs: Self) -> Self {
        Self::max(self, rhs)
    }
}

/// Adds a disjoint least element to a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WithMin<T> {
    Min,
    From(T),
}

/// Adds a disjoint greatest element to a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WithMax<T> {
    From(T),
    Max,
}

/// Adds disjoint least and greatest elements to a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WithMinMax<T> {
    Min,
    From(T),
    Max,
}

macro_rules! impl_from_option {
    ($t:ident, $m:ident) => {
        impl<T> From<Option<T>> for $t<T> {
            fn from(val: Option<T>) -> Self {
                match val {
                    Some(t) => $t::From(t),
                    None => $t::$m,
                }
            }
        }
    };
}

impl_from_option!(WithMin, Min);
impl_from_option!(WithMax, Max);

macro_rules! impl_option_from {
    ($t:ident) => {
        impl<T> From<$t<T>> for Option<T> {
            fn from(val: $t<T>) -> Option<T> {
                match val {
                    $t::From(t) => Some(t),
                    _ => None,
                }
            }
        }
    };
}

impl_option_from!(WithMin);
impl_option_from!(WithMax);
impl_option_from!(WithMinMax);

macro_rules! impl_from_mm {
    ($t1:ident, $m1:ident, $t2:ident, $m2:ident) => {
        impl<T> From<$t1<$t2<T>>> for WithMinMax<T> {
            fn from(val: $t1<$t2<T>>) -> WithMinMax<T> {
                match val {
                    $t1::$m1 => WithMinMax::$m1,
                    $t1::From($t2::$m2) => WithMinMax::$m2,
                    $t1::From($t2::From(t)) => WithMinMax::From(t),
                }
            }
        }
    };
}

impl_from_mm!(WithMin, Min, WithMax, Max);
impl_from_mm!(WithMax, Max, WithMin, Min);

macro_rules! impl_partialord {
    ($t:ident, $m:ident, $o1:ident, $o2:ident) => {
        impl<T: PartialOrd> PartialOrd for $t<T> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                match (self, other) {
                    ($t::$m, $t::$m) => Some(Ordering::Equal),
                    ($t::$m, _) => Some(Ordering::$o1),
                    (_, $t::$m) => Some(Ordering::$o2),
                    ($t::From(t1), $t::From(t2)) => t1.partial_cmp(t2),
                }
            }
        }
    };
}

impl_partialord!(WithMin, Min, Less, Greater);
impl_partialord!(WithMax, Max, Greater, Less);

impl<T: PartialOrd> PartialOrd for WithMinMax<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (WithMinMax::Min, WithMinMax::Min) => Some(Ordering::Equal),
            (WithMinMax::Min, _) => Some(Ordering::Less),
            (_, WithMinMax::Min) => Some(Ordering::Greater),

            (WithMinMax::Max, WithMinMax::Max) => Some(Ordering::Equal),
            (WithMinMax::Max, _) => Some(Ordering::Greater),
            (_, WithMinMax::Max) => Some(Ordering::Less),

            (WithMinMax::From(t1), WithMinMax::From(t2)) => t1.partial_cmp(t2),
        }
    }
}

macro_rules! impl_ord {
    ($t:ident) => {
        impl<T: Ord> Ord for $t<T> {
            fn cmp(&self, other: &Self) -> Ordering {
                // The only potential `None` value is when comparing two `From` variants. Since the
                // semantics of `Ord` imply that `partial_cmp(a, b) == Some(cmp(a, b))`, the `T:
                // Ord` condition implies that the following unwrap is safe.
                self.partial_cmp(other).unwrap()
            }
        }
    };
}

impl_ord!(WithMin);
impl_ord!(WithMax);
impl_ord!(WithMinMax);

impl<T: MeetSemilattice> MeetSemilattice for WithMin<T> {
    fn meet(self, other: Self) -> Self {
        match (self, other) {
            (WithMin::From(t1), WithMin::From(t2)) => WithMin::From(t1.meet(t2)),
            _ => WithMin::Min,
        }
    }
}

impl<T: MeetSemilattice> MeetSemilattice for WithMax<T> {
    fn meet(self, other: Self) -> Self {
        match (self, other) {
            (WithMax::From(t1), WithMax::From(t2)) => WithMax::From(t1.meet(t2)),
            _ => WithMax::Max,
        }
    }
}

impl<T: MeetSemilattice> MeetSemilattice for WithMinMax<T> {
    fn meet(self, other: Self) -> Self {
        match self {
            WithMinMax::Max => other,
            WithMinMax::From(t1) => match other {
                WithMinMax::Max => WithMinMax::From(t1), // Should be `self`, but it is partially moved.
                WithMinMax::From(t2) => WithMinMax::From(t1.meet(t2)),
                WithMinMax::Min => WithMinMax::Min,
            },
            WithMinMax::Min => WithMinMax::Min,
        }
    }
}

impl<T: JoinSemilattice> JoinSemilattice for WithMin<T> {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (WithMin::From(t1), WithMin::From(t2)) => WithMin::From(t1.join(t2)),
            (WithMin::Min, t) => t,
            (t, WithMin::Min) => t,
        }
    }
}

impl<T: JoinSemilattice> JoinSemilattice for WithMax<T> {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (WithMax::From(t1), WithMax::From(t2)) => WithMax::From(t1.join(t2)),
            (WithMax::Max, _) => WithMax::Max,
            (_, WithMax::Max) => WithMax::Max,
        }
    }
}

impl<T: JoinSemilattice> JoinSemilattice for WithMinMax<T> {
    fn join(self, other: Self) -> Self {
        match (self, other) {
            (WithMinMax::From(t1), WithMinMax::From(t2)) => WithMinMax::From(t1.join(t2)),
            (WithMinMax::Min, t) => t,
            (t, WithMinMax::Min) => t,
            (WithMinMax::Max, _) => WithMinMax::Max,
            (_, WithMinMax::Max) => WithMinMax::Max,
        }
    }
}
