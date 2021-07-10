use core::ops::Index;
use core::ops::IndexMut;
#[cfg(feature = "json")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::slice::{Iter, IterMut};

/// A BiVec is like a Vec, except we allow indices to be negative. It has a min_degree
/// property which tells us where the starting index is.
///
/// Note that properties like length and capacity are defined to be the maximum index allowed. For
/// example, if `v.min_degree = -2` and `v.len() = 3`, it means we can access `v[-2], v[-1], v[0],
/// v[1], v[2]` but not `v[3]`.
#[derive(Clone, PartialEq, Eq)]
pub struct BiVec<T> {
    pub data: Vec<T>,
    min_degree: i32,
}

impl<T: fmt::Debug> fmt::Debug for BiVec<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "BiVec({}) ", self.min_degree)?;
        self.data.fmt(formatter)
    }
}

impl<T> std::default::Default for BiVec<T> {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<T: Clone> BiVec<T> {
    /// If `min_degree < self.min_degree`, set `self.min_degree` to `min_degree` and pad the
    /// remaining spaces with `default`.
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let mut v = BiVec::from_vec(-2, vec![3, 4, 6, 2]);
    /// v.extend_negative(-4, 0);
    /// assert_eq!(v[1], 2);
    /// assert_eq!(v[-4], 0);
    /// assert_eq!(v.min_degree(), -4);
    /// ```
    pub fn extend_negative(&mut self, min_degree: i32, default: T) {
        let shift = self.min_degree - min_degree;
        if shift <= 0 {
            return;
        }
        self.data
            .splice(..0, std::iter::repeat(default).take(shift as usize));
        self.min_degree = min_degree;
    }
}

impl<T> BiVec<T> {
    pub fn new(min_degree: i32) -> Self {
        Self {
            data: Vec::new(),
            min_degree,
        }
    }

    pub fn from_vec(min_degree: i32, data: Vec<T>) -> Self {
        Self { data, min_degree }
    }

    pub fn into_vec(self: BiVec<T>) -> Vec<T> {
        self.data
    }

    pub fn with_capacity(min_degree: i32, capacity: i32) -> Self {
        debug_assert!(capacity >= min_degree);
        Self {
            data: Vec::with_capacity((capacity - min_degree) as usize),
            min_degree,
        }
    }

    pub const fn min_degree(&self) -> i32 {
        self.min_degree
    }

    /// This returns the largest degree in the bivector. This is equal to `self.len() - 1`.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let v = BiVec::from_vec(-2, vec![3, 4, 6, 8, 2]);
    /// assert_eq!(v.max_degree(), 2);
    /// ```
    pub fn max_degree(&self) -> i32 {
        self.len() - 1
    }

    /// This returns the "length" of the bivector, defined to be the smallest i such that
    /// `v[i]`
    /// is not defined.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let v = BiVec::from_vec(-2, vec![3, 4, 6, 8, 2]);
    /// assert_eq!(v.len(), 3);
    /// ```
    pub fn len(&self) -> i32 {
        self.data.len() as i32 + self.min_degree
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn push(&mut self, x: T) {
        self.data.push(x);
    }

    pub fn get(&self, idx: i32) -> Option<&T> {
        self.data.get((idx - self.min_degree) as usize)
    }

    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }
    pub fn iter(&self) -> Iter<T> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.data.iter_mut()
    }

    pub fn iter_enum(&self) -> impl DoubleEndedIterator<Item = (i32, &T)> {
        let min_degree = self.min_degree;
        self.data
            .iter()
            .enumerate()
            .map(move |(i, t)| (i as i32 + min_degree, t))
    }

    pub fn iter_mut_enum(&mut self) -> impl DoubleEndedIterator<Item = (i32, &mut T)> {
        let min_degree = self.min_degree;
        self.data
            .iter_mut()
            .enumerate()
            .map(move |(i, t)| (i as i32 + min_degree, t))
    }

    pub fn into_iter_enum(self) -> impl DoubleEndedIterator<Item = (i32, T)> {
        let min_degree = self.min_degree;
        self.data
            .into_iter()
            .enumerate()
            .map(move |(i, t)| (i as i32 + min_degree, t))
    }

    /// Extends the bivec such that `max_degree()` is at least `max`. If `max_degree()` is
    /// already at least `max`, the function does nothing. Otherwise, it fills the new entries
    /// with the return value of `F(i)`, where i is the index of the new entry.
    pub fn extend_with<F>(&mut self, max: i32, mut f: F)
    where
        F: FnMut(i32) -> T,
    {
        if max > self.max_degree() {
            self.data.reserve((max - self.max_degree()) as usize);
            for i in self.len()..=max {
                self.data.push(f(i));
            }
        }
    }

    pub fn reserve(&mut self, num: usize) {
        self.data.reserve(num);
    }

    /// Mutably borrows i and j. Panic if i != j.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let mut v = BiVec::from_vec(1, vec![3, 5, 2]);
    /// let (x, y) = v.split_borrow_mut(1, 3);
    /// assert_eq!(*x, 3);
    /// assert_eq!(*y, 2);
    ///
    /// let (x, y) = v.split_borrow_mut(3, 2);
    /// assert_eq!(*x, 2);
    /// assert_eq!(*y, 5);
    /// ```
    pub fn split_borrow_mut(&mut self, i: i32, j: i32) -> (&mut T, &mut T) {
        assert!(i != j);
        let min = self.min_degree;
        if i > j {
            let (f, s) = self.data.split_at_mut((i - min) as usize);
            (&mut s[0], &mut f[(j - min) as usize])
        } else {
            let (f, s) = self.data.split_at_mut((j - min) as usize);
            (&mut f[(i - min) as usize], &mut s[0])
        }
    }

    pub fn range(&self) -> std::ops::Range<i32> {
        self.min_degree..self.len()
    }
}

impl<'a, T> IntoIterator for &'a BiVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut BiVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(feature = "json")]
impl<T: Serialize> Serialize for BiVec<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer) // Do better than this
    }
}

#[cfg(feature = "json")]
impl<'de, T: Deserialize<'de>> Deserialize<'de> for BiVec<T> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}

impl<T> Index<i32> for BiVec<T> {
    type Output = T;
    fn index(&self, i: i32) -> &T {
        &(self.data[(i - self.min_degree) as usize])
    }
}

impl<T> IndexMut<i32> for BiVec<T> {
    fn index_mut(&mut self, i: i32) -> &mut T {
        &mut (self.data[(i - self.min_degree) as usize])
    }
}
use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl<T: Save> Save for BiVec<T> {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.data.save(buffer)
    }
}

impl<T: Load> Load for BiVec<T> {
    type AuxData = (i32, T::AuxData);

    fn load(buffer: &mut impl Read, data: &Self::AuxData) -> io::Result<Self> {
        let min_degree = data.0;
        let data = Load::load(buffer, &data.1)?;

        Ok(Self { data, min_degree })
    }
}
