use core::ops::Index;
use core::ops::IndexMut;
use std::slice::Iter;

/// A BiVec is like a Vec, except we allow indices to be negative. It has a min_degree
/// property which tells us where the starting index is.
///
/// Note that properties like length and capacity are defined to be the maximum index allowed. For
/// example, if `v.min_degree = -2` and `v.len() = 3`, it means we can access `v[-2], v[-1], v[0],
/// v[1], v[2]` but not `v[3]`.
#[derive(Debug, Clone)]
pub struct BiVec<T> {
    data : Vec<T>,
    min_degree : i32
}

impl<T> BiVec<T> {
    pub fn new(min_degree : i32) -> Self {
        BiVec {
            data : Vec::new(),
            min_degree
        }
    }

    pub fn from_vec(min_degree : i32, data : Vec<T>) -> Self {
        Self {
            data,
            min_degree
        }
    }

    pub fn with_capacity(min_degree : i32, capacity : i32) -> Self {
        debug_assert!(capacity >= min_degree);
        BiVec {
            data : Vec::with_capacity((capacity - min_degree) as usize),
            min_degree
        }
    }

    pub fn min_degree(&self) -> i32 {
        self.min_degree
    }

    pub fn max_degree(&self) -> i32 {
        self.data.len() as i32 + self.min_degree
    }

    pub fn len(&self) -> i32 {
        self.max_degree()
    }

    pub fn push(&mut self, x : T) {
        self.data.push(x);
    }

    pub fn iter(&self) -> Iter<T> {
        self.data.iter()
    }

    pub fn iter_enum(&self) -> impl Iterator<Item = (i32, &T)> {
        let min_degree = self.min_degree;
        self.data.iter().enumerate()
            .map(move |(i, t)| (i as i32 + min_degree, t))
            // .collect()
    }
}



impl<T> Index<i32> for BiVec<T> {
    type Output = T;
    fn index(&self, i : i32) -> &T {
        &(self.data[(i - self.min_degree) as usize])
    }
}

impl<T> IndexMut<i32> for BiVec<T> {
    fn index_mut(&mut self, i : i32) -> &mut T {
        &mut (self.data[(i - self.min_degree) as usize])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
