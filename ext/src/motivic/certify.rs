//! Certifying that the lifts an answer reads are finished as far as it reads them.

use std::collections::BTreeMap;

use sseq::coordinates::BidegreeGenerator;

use super::{Lifted, MotivicResolution};

/// One of the lifted maps a cell can belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Map {
    /// The differential $d$.
    Differential,
    /// The product chain map $\varphi_a$.
    Product(BidegreeGenerator),
    /// The null-homotopy of $\varphi_b \varphi_a$.
    Homotopy(BidegreeGenerator, BidegreeGenerator),
}

/// Where a cell sits in the order [`MotivicResolution::certify`] settles cells in: by homological
/// degree, then by map, then by generator.
type Key = (i32, u8, [i32; 6], i32, usize);

impl Map {
    /// The cell of `g` in this map, as a [`Key`].
    fn key(self, g: BidegreeGenerator) -> Key {
        let gen_coords = |a: BidegreeGenerator| [a.s(), a.t(), a.idx() as i32];
        let (rank, params) = match self {
            Self::Differential => (0, [0; 6]),
            Self::Product(a) => {
                let [s, t, i] = gen_coords(a);
                (1, [s, t, i, 0, 0, 0])
            }
            Self::Homotopy(a, b) => {
                let [s, t, i] = gen_coords(a);
                let [s2, t2, i2] = gen_coords(b);
                (2, [s, t, i, s2, t2, i2])
            }
        };
        (g.s(), rank, params, g.t(), g.idx())
    }

    /// The map and generator a [`Key`] names.
    fn from_key((s, rank, p, t, idx): Key) -> (Self, BidegreeGenerator) {
        let gen_at = |s, t, i: i32| BidegreeGenerator::s_t(s, t, i as usize);
        let map = match rank {
            0 => Self::Differential,
            1 => Self::Product(gen_at(p[0], p[1], p[2])),
            _ => Self::Homotopy(gen_at(p[0], p[1], p[2]), gen_at(p[3], p[4], p[5])),
        };
        (map, BidegreeGenerator::s_t(s, t, idx))
    }
}

impl MotivicResolution {
    /// The lift of `map`.
    ///
    /// # Panics
    ///
    /// If `map` is a product or homotopy that has not been lifted.
    fn lifted(&self, map: Map) -> &Lifted {
        match map {
            Map::Differential => &self.differential,
            Map::Product(a) => &self.products[&a],
            Map::Homotopy(a, b) => &self.homotopies[&(a, b)],
        }
    }

    /// The bidegree `map` sends `g` to, and the weight `g`'s image is graded against.
    fn image_of(&self, map: Map, g: BidegreeGenerator) -> (i32, i32, i32) {
        let w = self.weights[&g];
        match map {
            Map::Differential => (g.s() - 1, g.t(), w),
            Map::Product(a) => (g.s() - a.s(), g.t() - a.t(), w - self.weights[&a]),
            Map::Homotopy(a, b) => (
                g.s() + 1 - a.s() - b.s(),
                g.t() - a.t() - b.t(),
                w - self.weights[&a] - self.weights[&b],
            ),
        }
    }

    /// The terms of `g`'s image under `map`, as the generator each lies on and its
    /// $\tau$-power.
    fn terms(&self, map: Map, g: BidegreeGenerator) -> Vec<(BidegreeGenerator, i32)> {
        let Some(cell) = self.lifted(map).cells.get(&g) else {
            return Vec::new();
        };
        let (s, t, w) = self.image_of(map, g);
        let module = self.module(s);
        cell.iter()
            .map(|&bidx| {
                let og = module.index_to_op_gen(t, bidx);
                let target = BidegreeGenerator::s_t(s, og.generator_degree, og.generator_index);
                (target, self.entry_weight(s, t, bidx) - w)
            })
            .collect()
    }

    /// The highest weight of an $F_s$ basis element in degree `t`, given the highest weight of
    /// an operation in each degree.
    fn max_entry_weight(&self, max_op_weight: &[i32], s: i32, t: i32) -> i32 {
        let module = self.module(s);
        // Only generators through `compute` have weights; `FIRST_MARGIN` keeps all of the box's
        // in range.
        (0..=t.min(self.compute.n() + s))
            .flat_map(|gen_t| {
                (0..module.number_of_gens_in_degree(gen_t)).map(move |idx| (gen_t, idx))
            })
            .map(|(gen_t, idx)| {
                self.weights[&BidegreeGenerator::s_t(s, gen_t, idx)]
                    + max_op_weight[(t - gen_t) as usize]
            })
            .max()
            .unwrap_or(i32::MIN)
    }

    /// The cells the box's own differential reads: every box generator's, in full and through
    /// the highest $\tau$-order its $d_{s-1} d_s$ can have.
    pub(super) fn box_roots(&self) -> Vec<(Map, BidegreeGenerator, i32)> {
        let max_op_weight: Vec<i32> = (0..=self.compute.t())
            .map(|t| {
                (0..self.engine.dimension(t))
                    .map(|idx| self.op_weight(t, idx))
                    .max()
                    .unwrap_or(i32::MIN)
            })
            .collect();
        let mut roots = Vec::new();
        for s in 2..=self.max.s() {
            for t in 0..=self.max.n() + s {
                let top = self.max_entry_weight(&max_op_weight, s - 2, t);
                for idx in 0..self.num_gens(s, t) {
                    let g = BidegreeGenerator::s_t(s, t, idx);
                    roots.push((Map::Differential, g, top - self.weights[&g]));
                }
            }
        }
        roots
    }

    /// Check that every cell `roots` reads is finished through the $\tau$-order it is read at.
    ///
    /// Each root is a cell and the order through which an answer reads it. A cell's defect is
    /// a sum of composites, each applying an inner map to the terms of an outer one, and a term
    /// $\tau^P m \otimes g'$ of an outer cell read through order $R$ reads the inner map's cell
    /// at $g'$ through order $R - P$:
    ///
    /// - $d(g)$: $d$ on the terms of $d(g)$.
    /// - $\varphi_a(g)$: $d$ on its terms, and $\varphi_a$ on the terms of $d(g)$.
    /// - $H(g)$, null-homotopy of $\varphi_b \varphi_a$: $d$ on its terms, $H$ on the terms of
    ///   $d(g)$, and $\varphi_b$ on the terms of $\varphi_a(g)$.
    ///
    /// If every stalled cell is finished through the order it is read at, keeping each cell
    /// through that order and solving the orders above it again gives lifts of all three maps,
    /// over the whole resolution, that agree with these on what the roots read.
    ///
    /// Every read lands in a lower homological degree, or in the same one and an earlier map in
    /// the order $H$, $\varphi$, $d$, so settling cells in that order settles each one's order
    /// before it is checked.
    ///
    /// # Errors
    ///
    /// The first cell read past where it stalled.
    pub(super) fn certify(
        &self,
        roots: Vec<(Map, BidegreeGenerator, i32)>,
    ) -> Result<(), (Map, BidegreeGenerator)> {
        let mut read: BTreeMap<Key, i32> = BTreeMap::new();
        let mut require = |map: Map, g: BidegreeGenerator, order: i32| {
            let r = read.entry(map.key(g)).or_insert(order);
            *r = (*r).max(order);
        };
        for (map, g, order) in roots {
            require(map, g, order);
        }

        while let Some((key, order)) = read.pop_last() {
            // Order 0 is the reduction mod τ, which every cell has right.
            if order < 1 {
                continue;
            }
            let (map, g) = Map::from_key(key);
            if self
                .lifted(map)
                .stalled
                .get(&g)
                .is_some_and(|&stall| stall <= order)
            {
                return Err((map, g));
            }
            let mut require = |map: Map, g: BidegreeGenerator, order: i32| {
                let r = read.entry(map.key(g)).or_insert(order);
                *r = (*r).max(order);
            };
            for (target, power) in self.terms(map, g) {
                require(Map::Differential, target, order - power);
            }
            match map {
                Map::Differential => {}
                Map::Product(_) => {
                    require(Map::Differential, g, order);
                    for (target, power) in self.terms(Map::Differential, g) {
                        require(map, target, order - power);
                    }
                }
                Map::Homotopy(a, b) => {
                    require(Map::Differential, g, order);
                    for (target, power) in self.terms(Map::Differential, g) {
                        require(map, target, order - power);
                    }
                    require(Map::Product(a), g, order);
                    for (target, power) in self.terms(Map::Product(a), g) {
                        require(Map::Product(b), target, order - power);
                    }
                }
            }
        }
        Ok(())
    }
}
