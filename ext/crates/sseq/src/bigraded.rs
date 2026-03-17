use std::ops::{Index, IndexMut};

use once::MultiIndexed;

use crate::coordinates::Bidegree;

/// A sparse collection indexed by [`Bidegree`].
///
/// This is a thin wrapper around [`MultiIndexed<2, V>`] that maps bidegrees to `[x, y]`
/// coordinates.
pub struct Bigraded<V>(MultiIndexed<2, V>);

impl<V> Bigraded<V> {
    pub fn new() -> Self {
        Self(MultiIndexed::new())
    }

    pub fn get(&self, b: Bidegree) -> Option<&V> {
        self.0.get([b.x(), b.y()])
    }

    pub fn get_mut(&mut self, b: Bidegree) -> Option<&mut V> {
        self.0.get_mut([b.x(), b.y()])
    }

    pub fn insert(&self, b: Bidegree, value: V) {
        self.0.insert([b.x(), b.y()], value);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn min(&self) -> Option<Bidegree> {
        self.0.min_coords().map(|c| Bidegree::x_y(c[0], c[1]))
    }

    pub fn max(&self) -> Option<Bidegree> {
        self.0.max_coords().map(|c| Bidegree::x_y(c[0], c[1]))
    }

    pub fn iter(&self) -> impl Iterator<Item = (Bidegree, &V)> {
        self.0.iter().map(|(c, v)| (Bidegree::x_y(c[0], c[1]), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Bidegree, &mut V)> {
        self.0
            .iter_mut()
            .map(|(c, v)| (Bidegree::x_y(c[0], c[1]), v))
    }
}

impl<V> Index<Bidegree> for Bigraded<V> {
    type Output = V;

    fn index(&self, b: Bidegree) -> &V {
        self.get(b).unwrap()
    }
}

impl<V> IndexMut<Bidegree> for Bigraded<V> {
    fn index_mut(&mut self, b: Bidegree) -> &mut V {
        self.get_mut(b).unwrap()
    }
}

impl<V> Default for Bigraded<V> {
    fn default() -> Self {
        Self::new()
    }
}
