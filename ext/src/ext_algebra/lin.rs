//! Products in Ext by W. Lin's method ("Efficient computations of Yoneda product of Ext groups",
//! ICERM, June 2026): one chain map per basis *monomial*, each lifted only over a small corner,
//! instead of one chain map per class lifted over the whole computed range.
//!
//! Order the indecomposables `a_1 < a_2 < ...` of `Ext(k, k)` by `(t, s)` and take as additive basis
//! of `Ext(M, k)` monomials `m = u · a_{i_1} ⋯ a_{i_r}`, `i_1 <= ... <= i_r`, where `u` is the unit
//! (for the ring itself) or a generator of `Ext(M, k)` as an `Ext(k, k)`-module. Write `lo(m)` for
//! the smallest index (`∞` when there is none). Then every product `m · a_j` is
//!
//! * read off the chain map `f_m` of `m` at the generator `a_j` of the target resolution, if
//!   `a_j <= a_lo(m)` -- so `f_m` is needed only over the bidegrees `<= |a_j|` for those `j`;
//! * `±(m' · a_j) · a_k` for `m = m' · a_k`, `k = lo(m) < j`, otherwise: by induction on the
//!   degree, and then on the index, which drops from `j` to `k` at equal degree. The sign is that
//!   of graded commutativity, `(-1)^{s s' + t t'}`.
//!
//! Bidegrees are decided in order of `t`. At each one the candidates `m' · a_k` (`m'` a basis
//! monomial, `a_k <= a_lo(m')`, all read directly) span the decomposables; taken in increasing
//! order, the independent ones are new basis monomials. The rest of the bidegree is spanned by new
//! generators (of the ring, or of the module): the resolution's generators at the non-pivot
//! columns.
//!
//! At `p = 2` over the Milnor basis, products by the `h_i` are read off the differentials (see
//! [`super::h_product`]), so no map is lifted for them, and the first indecomposable read off a
//! chain map, `c_0`, has `t = 11`. The bidegrees are then decided in windows of 11 values of `t`,
//! with the maps they read lifted together, by one [`MultiLift`] sweep, before each window.
//! Otherwise every product raises `t` by at least one, and windows are single values of `t`.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use anyhow::ensure;
use fp::{
    matrix::Matrix,
    prime::{Prime, ValidPrime},
    vector::FpVector,
};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

use crate::{
    chain_complex::{AugmentedChainComplex, FreeChainComplex},
    lift::{LiftRequest, Liftable, MultiLift},
    resolution_homomorphism::ResolutionHomomorphism,
};

/// The `t` of `c_0`, the first indecomposable of `Ext(F_2, F_2)` off the `h_i` line: the width of
/// the windows when the `h_i` products are read off the differentials.
const H_FREE_WINDOW: i32 = 11;

/// The bidegrees `min_n <= n <= max_n`, `0 <= s <= max_s` a [`Lin`] has decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinRange {
    pub min_n: i32,
    pub max_n: i32,
    pub max_s: i32,
}

impl LinRange {
    pub fn contains(&self, b: Bidegree) -> bool {
        (0..=self.max_s).contains(&b.s()) && (self.min_n..=self.max_n).contains(&b.n())
    }

    /// The smallest range containing both.
    fn union(self, other: Self) -> Self {
        Self {
            min_n: self.min_n.min(other.min_n),
            max_n: self.max_n.max(other.max_n),
            max_s: self.max_s.max(other.max_s),
        }
    }
}

/// How a basis monomial was built.
#[derive(Clone, Copy, Debug)]
pub enum Repr {
    /// The unit of the ring.
    Unit,
    /// The indecomposable `a_j` of the ring.
    Gen(usize),
    /// The generator `j` of the module.
    ModuleGen(usize),
    /// `m · a_k` for the basis monomial `m`, with `a_k <= a_lo(m)`.
    Mul(usize, usize),
}

/// A basis monomial: the indices of its indecomposable factors (ascending), how it was built, and
/// its value in the resolution's dual basis of `Ext` at `b`.
pub struct Monomial {
    pub b: Bidegree,
    pub factors: Vec<u32>,
    pub repr: Repr,
    pub vec: FpVector,
    /// Index into the maps, once `f_m` has been needed.
    map: Option<usize>,
}

impl Monomial {
    /// `lo(m)`; `usize::MAX` when `m` has no indecomposable factor.
    fn lo(&self) -> usize {
        self.factors.first().map_or(usize::MAX, |&i| i as usize)
    }
}

/// A chain map lifted only over a downward-closed staircase of output bidegrees: row `s` through
/// stem `stair[s]` (`-1`: nothing), `stair[s - 1] >= stair[s]`.
struct Staircase<CC: FreeChainComplex> {
    hom: ResolutionHomomorphism<CC, CC>,
    stair: RwLock<Vec<i32>>,
}

impl<CC: FreeChainComplex> Staircase<CC> {
    fn covers(&self, c: Bidegree) -> bool {
        let stair = self.stair.read().unwrap();
        c.s() >= 0 && stair.get(c.s() as usize).is_some_and(|&n| c.n() <= n)
    }

    /// Grow the staircase to cover the rows `0..=c.s()` through stem `c.n()`. Whether it grew.
    fn grow(&self, c: Bidegree) -> bool {
        let mut stair = self.stair.write().unwrap();
        let rows = c.s() as usize + 1;
        if stair.len() < rows {
            stair.resize(rows, -1);
        }
        let mut grew = false;
        for n in &mut stair[..rows] {
            if c.n() > *n {
                *n = c.n();
                grew = true;
            }
        }
        grew
    }

    /// The largest output stem and row covered.
    fn extent(&self) -> Bidegree {
        let stair = self.stair.read().unwrap();
        let max_n = stair.iter().copied().max().unwrap_or(-1);
        let max_s = stair.iter().rposition(|&n| n >= 0).map_or(-1, |s| s as i32);
        Bidegree::n_s(max_n, max_s)
    }
}

impl<CC> Liftable for Staircase<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex + 'static,
{
    fn prepare(&self, b: Bidegree) -> Option<LiftRequest<'_>> {
        if !self.covers(b) {
            return None;
        }
        self.hom.prepare(b)
    }

    fn lifts_through(&self, target: &dyn std::any::Any) -> bool {
        self.hom.lifts_through(target)
    }
}

/// A decided bidegree: its basis monomials, and the inverse of their matrix (row `y` holds the
/// coordinates of the resolution generator `e_y` in the basis monomials).
struct Cell {
    ids: Vec<usize>,
    inv: Vec<FpVector>,
}

/// The basis of the ring at a bidegree `c`, as read by [`Lin::right_products`]: each basis
/// monomial as the indecomposables multiplied in, in order, and the coordinates of the resolution
/// generators in these monomials.
pub struct RingCell {
    chains: Vec<Vec<usize>>,
    inv: Vec<FpVector>,
}

/// The products of `Ext(M, k)` by `Ext(k, k)` through a [`LinRange`], by Lin's method. With
/// `M = k` this is the ring itself, and it finds the indecomposables; otherwise it reads them from
/// the ring's [`Lin`].
pub struct Lin<CC: FreeChainComplex> {
    /// The resolution of `M`.
    source: Arc<CC>,
    /// The resolution of `k`.
    target: Arc<CC>,
    /// Whether this is the ring (`source == target`), whose indecomposables it finds.
    is_ring: bool,
    /// Whether products by the `h_i` are read off the differentials.
    free_h: bool,
    prime: ValidPrime,
    /// The indecomposables of the ring found so far, in the order found (not sorted).
    pub gens: Vec<BidegreeGenerator>,
    /// The generators of the module, when this is not the ring.
    pub module_gens: Vec<BidegreeGenerator>,
    pub basis: Vec<Monomial>,
    /// The basis monomial of each indecomposable, when this is the ring.
    gen_monomial: Vec<usize>,
    cells: HashMap<Bidegree, Cell>,
    maps: Vec<Arc<Staircase<CC>>>,
    /// `h_k` products out of a bidegree, read off the differentials.
    h_rows: HashMap<(u32, Bidegree), Matrix>,
    memo: HashMap<(usize, usize), FpVector>,
    range: Option<LinRange>,
    /// Candidates that were dependent: the leading terms of relations (not minimised).
    pub relations: usize,
}

impl<CC> Lin<CC>
where
    CC: FreeChainComplex + AugmentedChainComplex + 'static,
{
    /// The ring `Ext(k, k)` of a minimal resolution of `k`, with nothing decided yet.
    pub fn ring(res: Arc<CC>) -> Self {
        let free_h = super::reads_h_products(&*res);
        Self::new(Arc::clone(&res), res, true, free_h)
    }

    /// `Ext(M, k)` over the ring of [`Lin::ring`]`(target)`, from minimal resolutions of `M` and `k`,
    /// with nothing decided yet.
    pub fn module(source: Arc<CC>, target: Arc<CC>) -> Self {
        let free_h = super::reads_h_products(&*source) && source.min_degree() >= 0;
        Self::new(source, target, false, free_h)
    }

    fn new(source: Arc<CC>, target: Arc<CC>, is_ring: bool, free_h: bool) -> Self {
        let prime = source.prime();
        Self {
            source,
            target,
            is_ring,
            free_h,
            prime,
            gens: vec![],
            module_gens: vec![],
            basis: vec![],
            gen_monomial: vec![],
            cells: HashMap::new(),
            maps: vec![],
            h_rows: HashMap::new(),
            memo: HashMap::new(),
            range: None,
            relations: 0,
        }
    }

    /// The range decided so far.
    pub fn range(&self) -> Option<LinRange> {
        self.range
    }

    /// The number of chain maps lifted.
    pub fn maps(&self) -> usize {
        self.maps.len()
    }

    /// The sort key of the indecomposable `a_j`.
    fn key(&self, j: usize) -> (i32, i32, usize) {
        let g = self.gens[j];
        (g.t(), g.s(), g.idx())
    }

    /// `a_j <= a_lo`, with `lo` from [`Monomial::lo`] (whose `usize::MAX` is above all).
    fn le(&self, j: usize, lo: usize) -> bool {
        lo == usize::MAX || self.key(j) <= self.key(lo)
    }

    /// `Some(k)` if products by the indecomposable `g` are read off the differentials as `h_k`.
    fn h_index(&self, g: BidegreeGenerator) -> Option<u32> {
        (self.free_h && g.s() == 1).then(|| g.t().trailing_zeros())
    }

    /// The sign of `a_j a_k = ± a_k a_j`: `(-1)^{s s' + t t'}`, as a coefficient.
    fn swap_sign(&self, j: usize, k: usize) -> u32 {
        let (a, b) = (self.gens[j], self.gens[k]);
        if (a.s() * b.s() + a.t() * b.t()) % 2 == 0 {
            1
        } else {
            self.prime.as_u32() - 1
        }
    }

    fn dim(&self, b: Bidegree) -> usize {
        if self.range.is_some_and(|r| r.contains(b)) {
            self.source.number_of_gens_in_bidegree(b)
        } else {
            0
        }
    }

    /// The `B`-coordinates of `v ∈ Ext(b)`, as `(basis monomial, coefficient)`.
    fn coords(&self, b: Bidegree, v: &FpVector) -> Vec<(usize, u32)> {
        let Some(cell) = self.cells.get(&b) else {
            assert!(v.is_zero(), "a non-zero vector at {b}, which has no basis");
            return vec![];
        };
        let mut c = FpVector::new(self.prime, cell.ids.len());
        for (y, v) in v.iter_nonzero() {
            c.add(&cell.inv[y], v);
        }
        c.iter_nonzero().map(|(i, v)| (cell.ids[i], v)).collect()
    }

    /// The `h_k` products out of `b`, rows over the generators at `b`.
    fn h_block(&mut self, k: u32, b: Bidegree) -> &Matrix {
        let source = &self.source;
        self.h_rows.entry((k, b)).or_insert_with(|| {
            super::h_product(&**source, k, b).expect("h_k products in range are readable")
        })
    }

    /// `m · a_j` read directly: an `h_k` off the differentials, otherwise `f_m` at `a_j` (which
    /// requires `a_j <= a_lo(m)`).
    fn direct(&mut self, j: usize, m: usize) -> FpVector {
        let g = self.gens[j];
        let mb = self.basis[m].b;
        let d = self.dim(g.degree() + mb);
        let mut v = FpVector::new(self.prime, d);
        if let Repr::Unit = self.basis[m].repr {
            v.set_entry(g.idx(), 1);
            return v;
        }
        if d == 0 {
            return v;
        }
        if let Some(k) = self.h_index(g) {
            let mono = self.basis[m].vec.clone();
            let rows = self.h_block(k, mb);
            for (y, c) in mono.iter_nonzero() {
                v.as_slice_mut().add(rows.row(y), c);
            }
            return v;
        }
        assert!(
            self.le(j, self.basis[m].lo()),
            "a direct product needs a_j <= a_lo(m)"
        );
        let map = &self.maps[self.basis[m]
            .map
            .expect("the monomial's chain map was lifted")];
        map.hom.act(v.as_slice_mut(), 1, g);
        v
    }

    /// `m · a_j` for an indecomposable `a_j` and a basis monomial `m`, the product in range.
    fn mul(&mut self, j: usize, m: usize) -> FpVector {
        if let Some(v) = self.memo.get(&(j, m)) {
            return v.clone();
        }
        let v = if self.h_index(self.gens[j]).is_some() || self.le(j, self.basis[m].lo()) {
            self.direct(j, m)
        } else {
            match self.basis[m].repr {
                Repr::Unit | Repr::ModuleGen(_) => unreachable!("lo is infinite"),
                // a_l a_j = ± a_j a_l, read off f_{a_j} since a_l < a_j.
                Repr::Gen(l) => {
                    let mut v = self.direct(l, self.gen_monomial[j]);
                    v.scale(self.swap_sign(j, l));
                    v
                }
                // (m' a_k) a_j = ± (m' a_j) a_k.
                Repr::Mul(k, m1) => {
                    let w = self.mul(j, m1);
                    let wb = self.gens[j].degree() + self.basis[m1].b;
                    let mut v = self.mul_vec(k, wb, &w);
                    v.scale(self.swap_sign(j, k));
                    v
                }
            }
        };
        self.memo.insert((j, m), v.clone());
        v
    }

    /// `w · a_k` for `w ∈ Ext(b)`.
    fn mul_vec(&mut self, k: usize, b: Bidegree, w: &FpVector) -> FpVector {
        let mut out = FpVector::new(self.prime, self.dim(b + self.gens[k].degree()));
        for (e, c) in self.coords(b, w) {
            let p = self.mul(k, e);
            out.add(&p, c);
        }
        out
    }

    /// The candidates at `b`: `(factors, k, m)` for `m' · a_k` with `m` a basis monomial at
    /// `b - |a_k|` and `a_k <= a_lo(m)`, the unit excluded, in increasing monomial order.
    fn candidates(&self, b: Bidegree) -> Vec<(Vec<u32>, usize, usize)> {
        let mut cands = vec![];
        for (k, g) in self.gens.iter().enumerate() {
            let Some(cell) = self.cells.get(&(b - g.degree())) else {
                continue;
            };
            for &m in &cell.ids {
                let mono = &self.basis[m];
                if matches!(mono.repr, Repr::Unit) || !self.le(k, mono.lo()) {
                    continue;
                }
                let mut factors = Vec::with_capacity(mono.factors.len() + 1);
                factors.push(k as u32);
                factors.extend_from_slice(&mono.factors);
                cands.push((factors, k, m));
            }
        }
        let module_gen = |m: usize| match self.basis[m].repr {
            Repr::ModuleGen(l) => l,
            _ => 0,
        };
        cands.sort_by_cached_key(|(f, _, m)| {
            let keys: Vec<_> = f.iter().map(|&j| self.key(j as usize)).collect();
            (keys, module_gen(*m))
        });
        cands
    }

    /// The chain map of the basis monomial `m`, created on first use with nothing lifted.
    fn map_of(&mut self, m: usize) -> usize {
        if let Some(i) = self.basis[m].map {
            return i;
        }
        let mono = &self.basis[m];
        let class: Vec<u32> = mono.vec.iter().collect();
        let hom = ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(&self.source),
            Arc::clone(&self.target),
            mono.b,
            &class,
        );
        let i = self.maps.len();
        self.maps.push(Arc::new(Staircase {
            hom,
            stair: RwLock::new(vec![]),
        }));
        self.basis[m].map = Some(i);
        i
    }

    /// Decide every bidegree of `range` not yet decided. For a module, `ring` is the ring's
    /// [`Lin`], decided through every degree an indecomposable factor can have here.
    ///
    /// On an error the products are unusable and must be discarded.
    pub fn extend(&mut self, range: LinRange, ring: Option<&Self>) -> anyhow::Result<()> {
        let old = self.range;
        let new = old.map_or(range, |o| o.union(range));
        if old == Some(new) {
            return Ok(());
        }
        if let Some(ring) = ring {
            ensure!(!self.is_ring, "the ring finds its own indecomposables");
            self.gens.clone_from(&ring.gens);
        } else {
            ensure!(
                self.is_ring,
                "a module reads the indecomposables from its ring"
            );
        }
        self.range = Some(new);
        self.memo.clear();
        let unit = Bidegree::s_t(0, 0);
        if self.is_ring && old.is_none() {
            let mut one = FpVector::new(self.prime, 1);
            one.set_entry(0, 1);
            self.basis.push(Monomial {
                b: unit,
                factors: vec![],
                repr: Repr::Unit,
                vec: one.clone(),
                map: None,
            });
            self.cells.insert(
                unit,
                Cell {
                    ids: vec![0],
                    inv: vec![one],
                },
            );
        }
        let is_ring = self.is_ring;
        let decided = |b: Bidegree| old.is_some_and(|o| o.contains(b)) || (is_ring && b == unit);
        let width = if self.free_h { H_FREE_WINDOW } else { 1 };
        let (t_lo, t_hi) = (new.min_n, new.max_n + new.max_s);
        let mut t0 = t_lo;
        while t0 <= t_hi {
            let t1 = (t0 + width - 1).min(t_hi);
            let window: Vec<Bidegree> = (t0..=t1)
                .flat_map(|t| (0..=new.max_s).map(move |s| Bidegree::s_t(s, t)))
                .filter(|&b| {
                    new.contains(b) && !decided(b) && self.source.number_of_gens_in_bidegree(b) > 0
                })
                .collect();
            self.lift_window(&window)?;
            for &b in &window {
                self.decide(b, width)?;
            }
            t0 = t1 + 1;
        }
        Ok(())
    }

    /// Lift the maps the candidates of `window` are read off, each over the bidegrees up to the
    /// indecomposables it is read at.
    fn lift_window(&mut self, window: &[Bidegree]) -> anyhow::Result<()> {
        let mut grown: Vec<usize> = vec![];
        for &b in window {
            for (_, k, m) in self.candidates(b) {
                let g = self.gens[k];
                if self.h_index(g).is_some() {
                    continue;
                }
                let i = self.map_of(m);
                if self.maps[i].grow(g.degree()) {
                    grown.push(i);
                }
            }
        }
        grown.sort_unstable();
        grown.dedup();
        if grown.is_empty() {
            return Ok(());
        }
        let bound = grown.iter().fold(Bidegree::n_s(-1, -1), |acc, &i| {
            let e = self.maps[i].extent();
            Bidegree::n_s(acc.n().max(e.n()), acc.s().max(e.s()))
        });
        let liftables = grown
            .iter()
            .map(|&i| Arc::clone(&self.maps[i]) as Arc<dyn Liftable>)
            .collect();
        MultiLift::new(Arc::clone(&self.target), liftables).extend_through_stem(bound)?;
        Ok(())
    }

    /// Decide the bidegree `b`: its basis monomials and new generators.
    fn decide(&mut self, b: Bidegree, width: i32) -> anyhow::Result<()> {
        let p = self.prime;
        let d = self.dim(b);
        let mut ids = vec![];
        let mut rows: Vec<FpVector> = vec![];
        // Echelon rows with leading coefficient one, by pivot.
        let mut ech: Vec<(usize, FpVector)> = vec![];
        for (factors, k, m) in self.candidates(b) {
            let v = self.direct(k, m);
            let mut r = v.clone();
            for (piv, row) in &ech {
                let c = r.entry(*piv);
                if c != 0 {
                    r.add(row, p.as_u32() - c);
                }
            }
            let Some((piv, c)) = r.first_nonzero() else {
                self.relations += 1;
                continue;
            };
            r.scale(fp::prime::inverse(p, c));
            ech.push((piv, r));
            ids.push(self.basis.len());
            rows.push(v.clone());
            self.basis.push(Monomial {
                b,
                factors,
                repr: Repr::Mul(k, m),
                vec: v,
                map: None,
            });
        }
        let mut pivot = vec![false; d];
        for (piv, _) in &ech {
            pivot[*piv] = true;
        }
        for idx in (0..d).filter(|&i| !pivot[i]) {
            let g = BidegreeGenerator::new(b, idx);
            let mut v = FpVector::new(p, d);
            v.set_entry(idx, 1);
            let (factors, repr) = if self.is_ring {
                ensure!(
                    !self.free_h || b.s() != 1 || (b.t() as u32).is_power_of_two(),
                    "an indecomposable at {b} in filtration 1 is not an h_i"
                );
                ensure!(
                    b.s() == 1 || b.t() >= width,
                    "an indecomposable at {b} off the h_i line below t = {width}"
                );
                let j = self.gens.len();
                self.gens.push(g);
                self.gen_monomial.push(self.basis.len());
                (vec![j as u32], Repr::Gen(j))
            } else {
                self.module_gens.push(g);
                (vec![], Repr::ModuleGen(self.module_gens.len() - 1))
            };
            ids.push(self.basis.len());
            rows.push(v.clone());
            self.basis.push(Monomial {
                b,
                factors,
                repr,
                vec: v,
                map: None,
            });
        }
        ensure!(
            ids.len() == d,
            "{b}: {} basis monomials, dim {d}",
            ids.len()
        );
        let inv = invert(p, &rows);
        self.cells.insert(b, Cell { ids, inv });
        Ok(())
    }

    /// The ring's basis at `c` for [`Lin::right_products`]. `None` if `c` is out of range.
    pub fn ring_cell(&self, c: Bidegree) -> Option<RingCell> {
        assert!(self.is_ring);
        if !self.range?.contains(c) {
            return None;
        }
        let Some(cell) = self.cells.get(&c) else {
            return Some(RingCell {
                chains: vec![],
                inv: vec![],
            });
        };
        let chain = |mut m: usize| {
            let mut chain = vec![];
            loop {
                match self.basis[m].repr {
                    Repr::Unit => break,
                    Repr::Gen(l) => {
                        chain.push(l);
                        break;
                    }
                    Repr::Mul(k, m1) => {
                        chain.push(k);
                        m = m1;
                    }
                    Repr::ModuleGen(_) => unreachable!(),
                }
            }
            chain.reverse();
            chain
        };
        Some(RingCell {
            chains: cell.ids.iter().map(|&m| chain(m)).collect(),
            inv: cell.inv.clone(),
        })
    }

    /// The products `x · e_y` of `x ∈ Ext(M)` at `b` with the generators `e_y` of the ring at `c`,
    /// whose basis is `cell` (from the ring's [`Lin::ring_cell`]): row `y` over the generators of
    /// `Ext(M)` at `b + c`. `None` if a bidegree is out of range.
    pub fn right_products(
        &mut self,
        b: Bidegree,
        x: &FpVector,
        c: Bidegree,
        cell: &RingCell,
    ) -> Option<Matrix> {
        let range = self.range?;
        if !range.contains(b) || !range.contains(b + c) {
            return None;
        }
        let dc = cell.inv.len();
        let mut matrix = Matrix::new(self.prime, dc, self.dim(b + c));
        for (mu, chain) in cell.chains.iter().enumerate() {
            // x · μ, multiplying in the factors of μ one at a time.
            let (mut w, mut wb) = (x.clone(), b);
            for &k in chain {
                w = self.mul_vec(k, wb, &w);
                wb = wb + self.gens[k].degree();
            }
            for (y, row) in cell.inv.iter().enumerate() {
                let coef = row.entry(mu);
                if coef != 0 {
                    matrix.row_mut(y).add(w.as_slice(), coef);
                }
            }
        }
        Some(matrix)
    }
}

/// Invert a square matrix over `F_p` given by its rows: row `y` of the result holds the
/// coordinates of the unit vector `e_y` in the basis of the given rows.
fn invert(p: ValidPrime, rows: &[FpVector]) -> Vec<FpVector> {
    let d = rows.len();
    let mut a: Vec<FpVector> = rows.to_vec();
    let mut inv: Vec<FpVector> = (0..d)
        .map(|i| {
            let mut v = FpVector::new(p, d);
            v.set_entry(i, 1);
            v
        })
        .collect();
    for col in 0..d {
        let r = (col..d)
            .find(|&r| a[r].entry(col) != 0)
            .expect("basis monomials are independent");
        a.swap(col, r);
        inv.swap(col, r);
        let scale = fp::prime::inverse(p, a[col].entry(col));
        a[col].scale(scale);
        inv[col].scale(scale);
        let (pivot_a, pivot_inv) = (a[col].clone(), inv[col].clone());
        for r in 0..d {
            let c = a[r].entry(col);
            if r != col && c != 0 {
                a[r].add(&pivot_a, p.as_u32() - c);
                inv[r].add(&pivot_inv, p.as_u32() - c);
            }
        }
    }
    // `a` is now the identity, so `inv · rows = 1`: `e_y = Σ_i inv[y][i] rows[i]`.
    inv
}
