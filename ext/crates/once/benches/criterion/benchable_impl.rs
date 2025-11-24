use once::OnceBiVec;

use crate::Benchable;

impl<T> Benchable<1, T> for OnceBiVec<T> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 1]) -> Self {
        OnceBiVec::new(min[0])
    }

    fn push_checked(&self, coords: [i32; 1], value: T) {
        self.push_checked(value, coords[0]);
    }

    fn get(&self, coords: [i32; 1]) -> Option<&T> {
        self.get(coords[0])
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; 1], &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().map(|(i, v)| ([i as i32], v))
    }
}

impl<T> Benchable<2, T> for OnceBiVec<OnceBiVec<T>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 2]) -> Self {
        OnceBiVec::new(min[0])
    }

    fn push_checked(&self, coords: [i32; 2], value: T) {
        self.get_or_insert(coords[0], || OnceBiVec::new(coords[1]))
            .get_or_insert(coords[1], || value);
    }

    fn get(&self, coords: [i32; 2]) -> Option<&T> {
        if !(coords[0] >= self.min_degree() && coords[0] < self.len()) {
            return None;
        }
        let layer1 = self.get(coords[0])?;
        if !(coords[1] >= layer1.min_degree() && coords[1] < layer1.len()) {
            return None;
        }
        layer1.get(coords[1])
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; 2], &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().flat_map(|(i, v)| {
            v.iter()
                .enumerate()
                .map(move |(j, w)| ([i as i32, j as i32], w))
        })
    }
}

impl<T> Benchable<3, T> for OnceBiVec<OnceBiVec<OnceBiVec<T>>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 3]) -> Self {
        OnceBiVec::new(min[0])
    }

    fn push_checked(&self, coords: [i32; 3], value: T) {
        self.get_or_insert(coords[0], || OnceBiVec::new(coords[1]))
            .get_or_insert(coords[1], || OnceBiVec::new(coords[2]))
            .get_or_insert(coords[2], || value);
    }

    fn get(&self, coords: [i32; 3]) -> Option<&T> {
        if !(coords[0] >= self.min_degree() && coords[0] < self.len()) {
            return None;
        }
        let layer1 = self.get(coords[0])?;
        if !(coords[1] >= layer1.min_degree() && coords[1] < layer1.len()) {
            return None;
        }
        let layer2 = layer1.get(coords[1])?;
        if !(coords[2] >= layer2.min_degree() && coords[2] < layer2.len()) {
            return None;
        }
        layer2.get(coords[2])
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; 3], &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().flat_map(|(i, v)| {
            v.iter().enumerate().flat_map(move |(j, w)| {
                w.iter()
                    .enumerate()
                    .map(move |(k, z)| ([i as i32, j as i32, k as i32], z))
            })
        })
    }
}

impl<T> Benchable<4, T> for OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<T>>>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 4]) -> Self {
        OnceBiVec::new(min[0])
    }

    fn push_checked(&self, coords: [i32; 4], value: T) {
        self.get_or_insert(coords[0], || OnceBiVec::new(coords[1]))
            .get_or_insert(coords[1], || OnceBiVec::new(coords[2]))
            .get_or_insert(coords[2], || OnceBiVec::new(coords[3]))
            .get_or_insert(coords[3], || value);
    }

    fn get(&self, coords: [i32; 4]) -> Option<&T> {
        if !(coords[0] >= self.min_degree() && coords[0] < self.len()) {
            return None;
        }
        let layer1 = self.get(coords[0])?;
        if !(coords[1] >= layer1.min_degree() && coords[1] < layer1.len()) {
            return None;
        }
        let layer2 = layer1.get(coords[1])?;
        if !(coords[2] >= layer2.min_degree() && coords[2] < layer2.len()) {
            return None;
        }
        let layer3 = layer2.get(coords[2])?;
        if !(coords[3] >= layer3.min_degree() && coords[3] < layer3.len()) {
            return None;
        }
        layer3.get(coords[3])
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; 4], &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().flat_map(|(i, v)| {
            v.iter().enumerate().flat_map(move |(j, w)| {
                w.iter().enumerate().flat_map(move |(k, z)| {
                    z.iter()
                        .enumerate()
                        .map(move |(l, x)| ([i as i32, j as i32, k as i32, l as i32], x))
                })
            })
        })
    }
}

impl<T> Benchable<5, T> for OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<T>>>>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 5]) -> Self {
        OnceBiVec::new(min[0])
    }

    fn push_checked(&self, coords: [i32; 5], value: T) {
        self.get_or_insert(coords[0], || OnceBiVec::new(coords[1]))
            .get_or_insert(coords[1], || OnceBiVec::new(coords[2]))
            .get_or_insert(coords[2], || OnceBiVec::new(coords[3]))
            .get_or_insert(coords[3], || OnceBiVec::new(coords[4]))
            .get_or_insert(coords[4], || value);
    }

    fn get(&self, coords: [i32; 5]) -> Option<&T> {
        if !(coords[0] >= self.min_degree() && coords[0] < self.len()) {
            return None;
        }
        let layer1 = self.get(coords[0])?;
        if !(coords[1] >= layer1.min_degree() && coords[1] < layer1.len()) {
            return None;
        }
        let layer2 = layer1.get(coords[1])?;
        if !(coords[2] >= layer2.min_degree() && coords[2] < layer2.len()) {
            return None;
        }
        let layer3 = layer2.get(coords[2])?;
        if !(coords[3] >= layer3.min_degree() && coords[3] < layer3.len()) {
            return None;
        }
        let layer4 = layer3.get(coords[3])?;
        if !(coords[4] >= layer4.min_degree() && coords[4] < layer4.len()) {
            return None;
        }
        layer4.get(coords[4])
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = ([i32; 5], &'a T)>
    where
        T: 'a,
    {
        self.iter().enumerate().flat_map(|(i, v)| {
            v.iter().enumerate().flat_map(move |(j, w)| {
                w.iter().enumerate().flat_map(move |(k, z)| {
                    z.iter().enumerate().flat_map(move |(l, x)| {
                        x.iter().enumerate().map(move |(m, y)| {
                            ([i as i32, j as i32, k as i32, l as i32, m as i32], y)
                        })
                    })
                })
            })
        })
    }
}
