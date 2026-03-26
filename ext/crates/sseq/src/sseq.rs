use std::marker::PhantomData;

use bivec::BiVec;
use fp::{
    matrix::{Matrix, Subquotient, Subspace},
    prime::ValidPrime,
    vector::{FpSlice, FpVector},
};

use crate::{
    Bigraded,
    coordinates::{Bidegree, BidegreeElement, BidegreeGenerator},
    differential::Differential,
};

/// The direction of the differentials
pub trait SseqProfile {
    const MIN_R: i32;
    fn profile(r: i32, b: Bidegree) -> Bidegree;
    fn profile_inverse(r: i32, b: Bidegree) -> Bidegree;
    fn differential_length(offset: Bidegree) -> i32;
}

pub struct Adams;

impl SseqProfile for Adams {
    const MIN_R: i32 = 2;

    fn profile(r: i32, b: Bidegree) -> Bidegree {
        b + Bidegree::x_y(-1, r)
    }

    fn profile_inverse(r: i32, b: Bidegree) -> Bidegree {
        b + Bidegree::x_y(1, -r)
    }

    fn differential_length(offset: Bidegree) -> i32 {
        offset.y()
    }
}

pub struct Product {
    pub b: Bidegree,
    /// Whether the product acts on the left or not. This affects the sign in the Leibniz rule.
    pub left: bool,
    pub matrices: Bigraded<Matrix>,
}

struct BidegreeData {
    /// The dimension of the module at this bidegree (i.e. the number of generators).
    dimension: usize,

    /// r -> differential
    ///
    /// If the bidegree is valid (see [`BidegreeData::invalid`]), then the differential is reduced.
    differentials: BiVec<Differential>,

    /// r -> E_r^{x, y} as a subquotient of the original bidegree.
    ///
    /// The "quotient" part of the subquotient is always accurate, but the "sub" part may not be.
    /// The `invalid` field tracks which bidegrees are valid.
    ///
    /// This is always the full ambient space when `r = P::MIN_R`, but we set `min_degree` to
    /// `P::MIN_R` to make code a bit more streamlined.
    page_data: BiVec<Subquotient>,

    permanent_classes: Subspace,

    /// Whether the page_data is no longer accurate.
    invalid: bool,
}

pub struct Sseq<P: SseqProfile = Adams> {
    p: ValidPrime,

    /// Per-bidegree data: differentials, page data, permanent classes, and validity.
    ///
    /// # Invariants:
    /// - if `data[b].differentials[r]` is defined, then `data[b].page_data[r + 1]` and
    ///   `data[target].page_data[r + 1]` are always defined,
    data: Bigraded<BidegreeData>,

    // `P` is itself a marker, so it's safe to claim that we own one. As opposed to
    // `PhantomData<*const P>`, this lets us implement `Send` and `Sync`.
    profile: PhantomData<P>,
}

impl<P: SseqProfile> Sseq<P> {
    pub fn new(p: ValidPrime) -> Self {
        Self {
            p,
            data: Bigraded::new(),
            profile: PhantomData,
        }
    }

    pub fn min(&self) -> Bidegree {
        self.data.min().unwrap_or(Bidegree::zero())
    }

    pub fn max(&self) -> Bidegree {
        self.data.max().unwrap_or(Bidegree::zero())
    }

    pub fn defined(&self, b: Bidegree) -> bool {
        self.data.get(b).is_some()
    }

    /// Iterate over all defined bidegrees (in sorted order).
    pub fn iter_bidegrees(&self) -> impl Iterator<Item = Bidegree> + '_ {
        self.data.iter().map(|(b, _)| b)
    }

    pub fn set_dimension(&mut self, b: Bidegree, dim: usize) {
        let mut page_data = BiVec::new(P::MIN_R);
        page_data.push(Subquotient::new_full(self.p, dim));
        self.data.insert(
            b,
            BidegreeData {
                dimension: dim,
                differentials: BiVec::new(P::MIN_R),
                page_data,
                permanent_classes: Subspace::new(self.p, dim),
                invalid: false,
            },
        );
    }

    pub fn clear(&mut self) {
        for (_, bd) in self.data.iter_mut() {
            bd.permanent_classes.set_to_zero();
            bd.differentials
                .iter_mut()
                .for_each(Differential::set_to_zero);
            bd.page_data.iter_mut().for_each(Subquotient::set_to_full);
            bd.invalid = true;
        }
    }

    pub fn dimension(&self, b: Bidegree) -> usize {
        self.data[b].dimension
    }

    /// The dimension in a bidegree, `None` if not yet defined.
    pub fn get_dimension(&self, b: Bidegree) -> Option<usize> {
        Some(self.data.get(b)?.dimension)
    }

    /// # Returns
    ///
    /// Whether a new permanent class was added
    pub fn add_permanent_class(&mut self, elem: &BidegreeElement) -> bool {
        let bd = &mut self.data[elem.degree()];
        let old_dim = bd.permanent_classes.dimension();
        let new_dim = bd.permanent_classes.add_vector(elem.vec());
        if old_dim != new_dim {
            // This was a new permanent class
            for d in bd.differentials.iter_mut() {
                d.add(elem.vec(), None);
            }
            bd.invalid = true;
        }
        old_dim != new_dim
    }

    /// Ensure `self.data[b].differentials[r]` is defined. Must call `extend_page_data` on the source
    /// and target after this.
    fn extend_differential(&mut self, r: i32, b: Bidegree) {
        let source_dim = self.dimension(b);
        while self.data[b].differentials.len() <= r {
            let r = self.data[b].differentials.len();
            let target = P::profile(r, b);
            let mut differential = Differential::new(self.p, source_dim, self.dimension(target));

            for class in self.data[b].permanent_classes.basis() {
                differential.add(class, None);
            }
            self.data[b].differentials.push(differential);
        }
    }

    /// Ensure `self.data[b].page_data[r]` is defined
    fn extend_page_data(&mut self, r: i32, b: Bidegree) {
        let bd = &mut self.data[b];
        while bd.page_data.len() <= r {
            bd.page_data.push(bd.page_data.last().unwrap().clone())
        }
    }

    /// Add a $d_r$ differential from bidegree $(x, y)$, with the given `source` and `target`
    /// classes.
    ///
    /// # Return
    ///
    /// Whether the differential is new
    pub fn add_differential(&mut self, r: i32, source: &BidegreeElement, target: FpSlice) -> bool {
        let target_b = P::profile(r, source.degree());

        self.extend_differential(r, source.degree());
        self.extend_page_data(r + 1, source.degree());
        self.extend_page_data(r + 1, target_b);

        for r in P::MIN_R..r {
            self.data[source.degree()].differentials[r].add(source.vec(), None);
            self.extend_page_data(r + 1, P::profile(r, source.degree()));
        }
        let is_new = self.data[source.degree()].differentials[r].add(source.vec(), Some(target));
        if is_new {
            self.data[source.degree()].invalid = true;
            if !target.is_zero() {
                self.data[target_b].invalid = true;
                self.add_permanent_class(&BidegreeElement::new(target_b, target.to_owned()));
                let target_page_len = self.data[target_b].page_data.len();
                for r in r + 1..target_page_len {
                    self.data[target_b].page_data[r].quotient(target);

                    let p = P::profile_inverse(r, target_b);
                    if self.defined(p) {
                        self.data[p].invalid = true;
                    }
                }
            }
        }
        is_new
    }

    pub fn invalid(&self, b: Bidegree) -> bool {
        self.data[b].invalid
    }

    pub fn update(&mut self) {
        let invalid_bidegrees: Vec<_> = self
            .data
            .iter()
            .filter(|(_, bd)| bd.invalid)
            .map(|(b, _)| b)
            .collect();
        for b in invalid_bidegrees {
            self.update_bidegree(b);
        }
    }

    /// This returns the vec of differentials to draw on each page.
    pub fn update_bidegree(&mut self, b: Bidegree) -> BiVec<Vec<Vec<u32>>> {
        self.data[b].invalid = false;

        // Collect target zeros first to avoid simultaneous cross-bidegree borrows.
        let diff_range = self.data[b].differentials.range();
        for r in diff_range {
            let target_b = P::profile(r, b);
            let zeros = self.data[target_b].page_data[r].zeros().clone();
            self.data[b].differentials[r].reduce_target(&zeros);
        }

        // For each page, the array of differentials to draw
        let mut differentials: BiVec<Vec<Vec<u32>>> =
            BiVec::with_capacity(P::MIN_R, self.data[b].differentials.len());

        let page_range = self.data[b].page_data.range();
        for r in page_range.skip(1) {
            let target_b = P::profile(r - 1, b);

            self.data[b].page_data[r].clear_gens();

            if r > self.data[b].differentials.len()
                || self.data[target_b].page_data[r - 1].is_empty()
            {
                let (prev, cur) = self.data[b].page_data.split_borrow_mut(r - 1, r);
                for g in prev.gens() {
                    cur.add_gen(g);
                }
                if r - 1 < self.data[b].differentials.len() {
                    differentials.push(vec![Vec::new(); self.data[b].page_data[r].dimension()]);
                }
            } else {
                let d = &self.data[b].differentials[r - 1];

                let source_dim = self.dimension(b);
                let target_dim = self.dimension(target_b);

                let mut drawn_differentials: Vec<Vec<u32>> =
                    Vec::with_capacity(self.data[b].page_data[r - 1].dimension());

                let mut dvec = FpVector::new(self.p, target_dim);
                let mut matrix = Matrix::new(
                    self.p,
                    self.data[b].page_data[r - 1].dimension(),
                    source_dim + target_dim,
                );

                for (mut row, g) in
                    std::iter::zip(matrix.iter_mut(), self.data[b].page_data[r - 1].gens())
                {
                    row.slice_mut(target_dim, target_dim + source_dim).assign(g);

                    d.evaluate(g, dvec.as_slice_mut());
                    row.slice_mut(0, target_dim).assign(dvec.as_slice());

                    drawn_differentials
                        .push(self.data[target_b].page_data[r - 1].reduce(dvec.as_slice_mut()));
                    dvec.set_to_zero();
                }
                differentials.push(drawn_differentials);

                matrix.row_reduce();

                let first_kernel_row = matrix.find_first_row_in_block(target_dim);

                for row in matrix.iter().skip(first_kernel_row) {
                    if row.is_zero() {
                        break;
                    }
                    self.data[b].page_data[r]
                        .add_gen(row.restrict(target_dim, target_dim + source_dim));
                }
            }
        }
        differentials
    }

    /// Whether the calcuations at bidegree (x, y) are complete. This means all classes on the
    /// final page are known to be permanent.
    pub fn complete(&self, b: Bidegree) -> bool {
        let bd = &self.data[b];
        bd.page_data
            .last()
            .unwrap()
            .gens()
            .all(|v| bd.permanent_classes.contains(v))
    }

    /// Whether there is an inconsistent differential involving bidegree (x, y).
    pub fn inconsistent(&self, b: Bidegree) -> bool {
        self.differentials(b).iter().any(Differential::inconsistent)
            || self.differentials_hitting(b).any(|(_, d)| d.inconsistent())
    }

    pub fn differentials(&self, b: Bidegree) -> &BiVec<Differential> {
        &self.data[b].differentials
    }

    pub fn differentials_hitting(
        &self,
        b: Bidegree,
    ) -> impl Iterator<Item = (i32, &'_ Differential)> + '_ {
        let max_r = self.data[b].page_data.len() - 1;
        (P::MIN_R..max_r).filter_map(move |r| {
            let source_b = P::profile_inverse(r, b);
            Some((r, self.data.get(source_b)?.differentials.get(r)?))
        })
    }

    pub fn permanent_classes(&self, b: Bidegree) -> &Subspace {
        &self.data[b].permanent_classes
    }

    pub fn page_data(&self, b: Bidegree) -> &BiVec<Subquotient> {
        &self.data[b].page_data
    }

    /// Compute the product between `product` and the class `class` at `(x, y)`. Returns `None` if
    /// the product is not yet computed.
    pub fn multiply(&self, elem: &BidegreeElement, prod: &Product) -> Option<BidegreeElement> {
        let target_b = elem.degree() + prod.b;
        let matrix = prod.matrices.get(elem.degree())?;
        let mut result = FpVector::new(self.p, self.get_dimension(target_b)?);
        matrix.apply(result.as_slice_mut(), 1, elem.vec());
        Some(BidegreeElement::new(target_b, result))
    }

    /// Apply the Leibniz rule to obtain new differentials. The differential we start with is a d_r
    /// differential from (x, y) with source `s` and target `t`. If the source is permanent, then r
    /// should be set to [`i32::MAX`].
    ///
    /// # Arguments
    ///  - `source_product` the product to multiply the class with
    ///  - `target_product` the differential on `source_product`. If `source_product` is permanent,
    ///    then this is None.
    ///
    /// # Return
    ///
    /// We return a tuple `(r, x, y, class)` recording the (source of) the new differential.
    /// If the function returns None, this means no differential was added. This can either be
    /// because the differential was trivial, or the data needed to compute the product is not yet
    /// available.
    pub fn leibniz(
        &mut self,
        r: i32,
        elem: &BidegreeElement,
        source_product: &Product,
        target_product: Option<&Product>,
    ) -> Option<(i32, BidegreeElement)> {
        let source = self.multiply(elem, source_product)?;

        // The class and the product are both permanent.
        if r == i32::MAX && target_product.is_none() {
            if self.add_permanent_class(&source) {
                return Some((i32::MAX, source));
            } else {
                return None;
            }
        }

        let neg_1 = self.p - 1;

        let target_r = target_product
            .map(|prod| P::differential_length(elem.degree() + prod.b - source.degree()))
            .unwrap_or(i32::MAX);

        let result_r = std::cmp::min(r, target_r);

        let result_b = P::profile(result_r, source.degree());
        let mut result = FpVector::new(self.p, self.get_dimension(result_b)?);

        if r == result_r {
            let diffs = &self.data[elem.degree()].differentials[r];
            let d_b = P::profile(r, elem.degree());
            let mut dx = FpVector::new(self.p, self.dimension(d_b));
            diffs.evaluate(elem.vec(), dx.as_slice_mut());
            let d = BidegreeElement::new(d_b, dx);
            let target = self.multiply(&d, source_product)?;

            if source_product.left && source_product.b.x() % 2 != 0 {
                result.add(&target.into_vec(), neg_1);
            } else {
                result.add(&target.into_vec(), 1);
            }
        }

        if target_r == result_r {
            let target = self.multiply(elem, target_product.unwrap())?;
            // why is this x - 1 but not x? This is what the original code does and came from trial
            // and error(?)
            if !source_product.left && (elem.x() - 1) % 2 != 0 {
                result.add(&target.into_vec(), neg_1);
            } else {
                result.add(&target.into_vec(), 1);
            }
        }

        if self.add_differential(result_r, &source, result.as_slice()) {
            Some((result_r, source))
        } else {
            None
        }
    }

    /// This shifts the sseq horizontally so that the minimum x is 0.
    pub fn write_to_graph<'a, T: crate::charting::Backend>(
        &self,
        mut g: T,
        r: i32,
        differentials: bool,
        products: impl Iterator<Item = &'a (String, Product)> + Clone,
        header: impl FnOnce(&mut T) -> Result<(), T::Error>,
    ) -> Result<(), T::Error> {
        let min = self.min();
        assert_eq!(min.y(), 0);

        let max = self.max();

        g.init(max - min)?;
        header(&mut g)?;

        for b in self.iter_bidegrees() {
            let shifted_b = b - min;

            let bd = self.page_data(b).get_max(r);
            if bd.is_empty() {
                continue;
            }

            g.node(shifted_b, bd.dimension())?;

            // Now add the products hitting this bidegree
            for (name, prod) in products.clone() {
                let source_b = b - prod.b;
                let shifted_source = source_b - min;

                if !self.defined(source_b) {
                    continue;
                }

                let source_data = self.page_data(source_b).get_max(r);
                if source_data.is_empty() {
                    continue;
                }

                // For unstable charts this is None in low degrees.
                if let Some(matrix) = prod.matrices.get(source_b) {
                    let matrix = Subquotient::reduce_matrix(matrix, source_data, bd);
                    g.structline_matrix(shifted_source, shifted_b, matrix, Some(name))?;
                }
            }

            // Finally add the differentials
            if differentials {
                let target_b = P::profile(r, b);
                let shifted_target = target_b - min;

                if target_b.x() < 0 {
                    continue;
                }
                let d = self.differentials(b);
                if d.len() <= r {
                    continue;
                }
                let d = &d[r];
                let target_data = self.page_data(target_b).get_max(r);

                let pairs = d
                    .get_source_target_pairs()
                    .into_iter()
                    .map(|(mut s, mut t)| {
                        (
                            bd.reduce(s.as_slice_mut()),
                            target_data.reduce(t.as_slice_mut()),
                        )
                    });

                for (source, target) in pairs {
                    for (i, v) in source.into_iter().enumerate() {
                        if v == 0 {
                            continue;
                        }
                        for (j, &v) in target.iter().enumerate() {
                            if v == 0 {
                                continue;
                            }
                            g.structline(
                                BidegreeGenerator::new(shifted_b, i),
                                BidegreeGenerator::new(shifted_target, j),
                                Some(&format!("d{r}")),
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};

    use super::*;

    #[test]
    fn test_sseq_differential() {
        let p = ValidPrime::new(3);
        let mut sseq = Sseq::<Adams>::new(p);
        sseq.set_dimension(Bidegree::x_y(0, 0), 1);
        sseq.set_dimension(Bidegree::x_y(1, 0), 2);
        sseq.set_dimension(Bidegree::x_y(1, 1), 2);
        sseq.set_dimension(Bidegree::x_y(0, 1), 0);
        sseq.set_dimension(Bidegree::x_y(0, 2), 3);
        sseq.set_dimension(Bidegree::x_y(0, 3), 1);

        sseq.add_differential(
            2,
            &BidegreeElement::new(Bidegree::x_y(1, 0), FpVector::from_slice(p, &[1, 1])),
            FpVector::from_slice(p, &[0, 1, 2]).as_slice(),
        );

        sseq.add_differential(
            3,
            &BidegreeElement::new(Bidegree::x_y(1, 0), FpVector::from_slice(p, &[1, 0])),
            FpVector::from_slice(p, &[1]).as_slice(),
        );

        sseq.update();

        let check = |b, r, e: Expect| {
            e.assert_eq(&sseq.page_data(b)[r].to_string());
        };

        check(
            Bidegree::x_y(1, 0),
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(1, 0),
            3,
            expect![[r#"
            Generators:
            [1, 0]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(1, 0),
            4,
            expect![[r#"
            Generators:

            Zeros:

        "#]],
        );

        check(
            Bidegree::x_y(1, 1),
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );

        check(
            Bidegree::x_y(0, 2),
            2,
            expect![[r#"
            Generators:
            [1, 0, 0]
            [0, 1, 0]
            [0, 0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 2),
            3,
            expect![[r#"
            Generators:
            [1, 0, 0]
            [0, 0, 1]

            Zeros:
            [0, 1, 2]

        "#]],
        );

        check(
            Bidegree::x_y(0, 3),
            2,
            expect![[r#"
            Generators:
            [1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 3),
            3,
            expect![[r#"
            Generators:
            [1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 3),
            4,
            expect![[r#"
            Generators:

            Zeros:
            [1]

        "#]],
        );

        sseq.add_differential(
            2,
            &BidegreeElement::new(Bidegree::x_y(1, 1), FpVector::from_slice(p, &[1, 0])),
            FpVector::from_slice(p, &[1]).as_slice(),
        );
        sseq.update();

        // Redefine `check` for borrow-checker reasons
        let check = |b, r, e: Expect| {
            e.assert_eq(&sseq.page_data(b)[r].to_string());
        };

        check(
            Bidegree::x_y(1, 0),
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(1, 0),
            3,
            expect![[r#"
            Generators:
            [1, 0]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(1, 0),
            4,
            expect![[r#"
            Generators:
            [1, 0]

            Zeros:

        "#]],
        );

        check(
            Bidegree::x_y(1, 1),
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(1, 1),
            3,
            expect![[r#"
            Generators:
            [0, 1]

            Zeros:

        "#]],
        );

        check(
            Bidegree::x_y(0, 2),
            2,
            expect![[r#"
            Generators:
            [1, 0, 0]
            [0, 1, 0]
            [0, 0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 2),
            3,
            expect![[r#"
            Generators:
            [1, 0, 0]
            [0, 0, 1]

            Zeros:
            [0, 1, 2]

        "#]],
        );

        check(
            Bidegree::x_y(0, 3),
            2,
            expect![[r#"
            Generators:
            [1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 3),
            3,
            expect![[r#"
            Generators:

            Zeros:
            [1]

        "#]],
        );
        check(
            Bidegree::x_y(0, 3),
            4,
            expect![[r#"
            Generators:

            Zeros:
            [1]

        "#]],
        );
    }

    #[test]
    fn test_sseq_differential_2() {
        let p = ValidPrime::new(2);
        let mut sseq = Sseq::<Adams>::new(p);

        sseq.set_dimension(Bidegree::x_y(0, 0), 0);
        sseq.set_dimension(Bidegree::x_y(1, 0), 2);
        sseq.set_dimension(Bidegree::x_y(0, 1), 0);
        sseq.set_dimension(Bidegree::x_y(0, 2), 2);

        sseq.add_differential(
            2,
            &BidegreeElement::new(Bidegree::x_y(1, 0), FpVector::from_slice(p, &[1, 0])),
            FpVector::from_slice(p, &[1, 0]).as_slice(),
        );
        sseq.add_differential(
            2,
            &BidegreeElement::new(Bidegree::x_y(1, 0), FpVector::from_slice(p, &[0, 1])),
            FpVector::from_slice(p, &[1, 1]).as_slice(),
        );
        sseq.update();

        let check = |b: Bidegree, r, e: Expect| {
            e.assert_eq(&sseq.page_data(b)[r].to_string());
        };

        check(
            Bidegree::x_y(1, 0),
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(1, 0),
            3,
            expect![[r#"
            Generators:

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 2),
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            Bidegree::x_y(0, 2),
            3,
            expect![[r#"
            Generators:

            Zeros:
            [1, 0]
            [0, 1]

        "#]],
        );
    }
}
