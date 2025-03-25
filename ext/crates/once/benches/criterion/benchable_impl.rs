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
}

impl<T> Benchable<2, T> for OnceBiVec<OnceBiVec<T>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 2]) -> Self {
        let layer0 = OnceBiVec::new(min[0]);
        // Initialize with empty middle and inner vectors
        for i in min[0]..min[0] {
            let layer1 = OnceBiVec::new(min[1]);
            layer0.push_checked(layer1, i);
        }
        layer0
    }

    fn push_checked(&self, coords: [i32; 2], value: T) {
        // Get or create inner vector
        if let Some(layer0) = self.get(coords[0]) {
            layer0.push_checked(value, coords[1]);
        } else {
            let layer1 = OnceBiVec::new(coords[1]);
            layer1.push_checked(value, coords[1]);
            self.push_checked(layer1, coords[0]);
        }
    }

    fn get(&self, coords: [i32; 2]) -> Option<&T> {
        if coords[0] >= self.min_degree() && coords[0] < self.len() {
            let layer1 = self.get(coords[0])?;
            if coords[1] >= layer1.min_degree() && coords[1] < layer1.len() {
                layer1.get(coords[1])
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T> Benchable<3, T> for OnceBiVec<OnceBiVec<OnceBiVec<T>>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 3]) -> Self {
        let layer0 = OnceBiVec::new(min[0]);
        let layer1 = OnceBiVec::new(min[1]);
        let layer2 = OnceBiVec::new(min[2]);
        layer1.push_checked(layer2, min[1]);
        layer0.push_checked(layer1, min[0]);
        layer0
    }

    fn push_checked(&self, coords: [i32; 3], value: T) {
        // Get or create middle vector
        if let Some(layer1) = self.get(coords[0]) {
            if let Some(layer2) = layer1.get(coords[1]) {
                layer2.push_checked(value, coords[2]);
            } else {
                let layer2 = OnceBiVec::new(coords[2]);
                layer2.push_checked(value, coords[2]);
                layer1.push_checked(layer2, coords[1]);
            }
        } else {
            let layer1 = OnceBiVec::new(coords[1]);
            let layer2 = OnceBiVec::new(coords[2]);
            layer2.push_checked(value, coords[2]);
            layer1.push_checked(layer2, coords[1]);
            self.push_checked(layer1, coords[0]);
        }
    }

    fn get(&self, coords: [i32; 3]) -> Option<&T> {
        if coords[0] >= self.min_degree() && coords[0] < self.len() {
            let layer1 = self.get(coords[0])?;
            if coords[1] >= layer1.min_degree() && coords[1] < layer1.len() {
                let layer2 = layer1.get(coords[1])?;
                if coords[2] >= layer2.min_degree() && coords[2] < layer2.len() {
                    layer2.get(coords[2])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T> Benchable<4, T> for OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<T>>>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 4]) -> Self {
        let layer0 = OnceBiVec::new(min[0]);
        let layer1 = OnceBiVec::new(min[1]);
        let layer2 = OnceBiVec::new(min[2]);
        let layer3 = OnceBiVec::new(min[3]);
        layer2.push_checked(layer3, min[2]);
        layer1.push_checked(layer2, min[1]);
        layer0.push_checked(layer1, min[0]);
        layer0
    }

    fn push_checked(&self, coords: [i32; 4], value: T) {
        if let Some(layer1) = self.get(coords[0]) {
            if let Some(layer2) = layer1.get(coords[1]) {
                if let Some(layer3) = layer2.get(coords[2]) {
                    layer3.push_checked(value, coords[3]);
                } else {
                    let layer3 = OnceBiVec::new(coords[3]);
                    layer3.push_checked(value, coords[3]);
                    layer2.push_checked(layer3, coords[2]);
                }
            } else {
                let layer3 = OnceBiVec::new(coords[3]);
                layer3.push_checked(value, coords[3]);
                let layer2 = OnceBiVec::new(coords[2]);
                layer2.push_checked(layer3, coords[2]);
                layer1.push_checked(layer2, coords[1]);
            }
        } else {
            let layer3 = OnceBiVec::new(coords[3]);
            layer3.push_checked(value, coords[3]);
            let layer2 = OnceBiVec::new(coords[2]);
            layer2.push_checked(layer3, coords[3]);
            let layer1 = OnceBiVec::new(coords[1]);
            layer1.push_checked(layer2, coords[1]);
            self.push_checked(layer1, coords[0]);
        }
    }

    fn get(&self, coords: [i32; 4]) -> Option<&T> {
        if coords[0] >= self.min_degree() && coords[0] < self.len() {
            let layer1 = self.get(coords[0])?;
            if coords[1] >= layer1.min_degree() && coords[1] < layer1.len() {
                let layer2 = layer1.get(coords[1])?;
                if coords[2] >= layer2.min_degree() && coords[2] < layer2.len() {
                    let layer3 = layer2.get(coords[2])?;
                    if coords[3] >= layer3.min_degree() && coords[3] < layer3.len() {
                        layer3.get(coords[3])
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T> Benchable<5, T> for OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<OnceBiVec<T>>>>> {
    fn name() -> &'static str {
        "oncebivec"
    }

    fn new(min: [i32; 5]) -> Self {
        let layer0 = OnceBiVec::new(min[0]);
        let layer1 = OnceBiVec::new(min[1]);
        let layer2 = OnceBiVec::new(min[2]);
        let layer3 = OnceBiVec::new(min[3]);
        let layer4 = OnceBiVec::new(min[4]);
        layer3.push_checked(layer4, min[3]);
        layer2.push_checked(layer3, min[2]);
        layer1.push_checked(layer2, min[1]);
        layer0.push_checked(layer1, min[0]);
        layer0
    }

    fn push_checked(&self, coords: [i32; 5], value: T) {
        if let Some(layer1) = self.get(coords[0]) {
            if let Some(layer2) = layer1.get(coords[1]) {
                if let Some(layer3) = layer2.get(coords[2]) {
                    if let Some(layer4) = layer3.get(coords[3]) {
                        layer4.push_checked(value, coords[4]);
                    } else {
                        let layer4 = OnceBiVec::new(coords[4]);
                        layer4.push_checked(value, coords[4]);
                        layer3.push_checked(layer4, coords[3]);
                    }
                } else {
                    let layer4 = OnceBiVec::new(coords[4]);
                    layer4.push_checked(value, coords[4]);
                    let layer3 = OnceBiVec::new(coords[3]);
                    layer3.push_checked(layer4, coords[3]);
                    layer2.push_checked(layer3, coords[2]);
                }
            } else {
                let layer4 = OnceBiVec::new(coords[4]);
                layer4.push_checked(value, coords[4]);
                let layer3 = OnceBiVec::new(coords[3]);
                layer3.push_checked(layer4, coords[3]);
                let layer2 = OnceBiVec::new(coords[2]);
                layer2.push_checked(layer3, coords[2]);
                layer1.push_checked(layer2, coords[1]);
            }
        } else {
            let layer4 = OnceBiVec::new(coords[4]);
            layer4.push_checked(value, coords[4]);
            let layer3 = OnceBiVec::new(coords[3]);
            layer3.push_checked(layer4, coords[3]);
            let layer2 = OnceBiVec::new(coords[2]);
            layer2.push_checked(layer3, coords[3]);
            let layer1 = OnceBiVec::new(coords[1]);
            layer1.push_checked(layer2, coords[1]);
            self.push_checked(layer1, coords[0]);
        }
    }

    fn get(&self, coords: [i32; 5]) -> Option<&T> {
        if coords[0] >= self.min_degree() && coords[0] < self.len() {
            let layer1 = self.get(coords[0])?;
            if coords[1] >= layer1.min_degree() && coords[1] < layer1.len() {
                let layer2 = layer1.get(coords[1])?;
                if coords[2] >= layer2.min_degree() && coords[2] < layer2.len() {
                    let layer3 = layer2.get(coords[2])?;
                    if coords[3] >= layer3.min_degree() && coords[3] < layer3.len() {
                        let layer4 = layer3.get(coords[3])?;
                        if coords[4] >= layer4.min_degree() && coords[4] < layer4.len() {
                            layer4.get(coords[4])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
