use std::{marker::PhantomData, sync::Arc};

use bivec::BiVec;
use fp::{
    matrix::{Matrix, Subquotient, Subspace},
    prime::ValidPrime,
    vector::{FpSlice, FpVector},
};

use crate::{
    bigraded::DenseBigradedModule,
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
    pub matrices: BiVec<BiVec<Option<Matrix>>>,
}

pub struct Sseq<P: SseqProfile = Adams> {
    p: ValidPrime,

    /// The first page of the spectral sequence
    classes: Arc<DenseBigradedModule>,

    /// x -> y -> r -> differential
    ///
    /// If the bidegree is valid (see [`Sseq::invalid`]), then the differential is reduced.
    differentials: BiVec<BiVec<BiVec<Differential>>>,

    /// x -> y -> permanent_classes
    permanent_classes: BiVec<BiVec<Subspace>>,

    /// x -> y -> r -> E_r^{x, y} as a subquotient of the original bidegree.
    ///
    /// The "quotient" part of the subquotient is always accurate, but the "sub" part may not be.
    /// The `invalid` field tracks which bidegrees are valid.
    ///
    /// This is always the full ambient space when `r = P::MIN_R`, but we set `min_degree` to
    /// `P::MIN_R` to make code a bit more streamlined.
    ///
    /// # Invariants:
    ///  - if `differential[x][y][r]` is defined, then `page_data[x][y][r + 1]` and
    ///    `page_data[tx][ty][r + 1]` are always defined,
    page_data: BiVec<BiVec<BiVec<Subquotient>>>,

    /// x -> y -> validity. A bidegree is invalid if the page_data is no longer accurate.
    invalid: BiVec<BiVec<bool>>,

    // `P` is itself a marker, so it's safe to claim that we own one. As opposed to
    // `PhantomData<*const P>`, this lets us implement `Send` and `Sync`.
    profile: PhantomData<P>,
}

impl<P: SseqProfile> Sseq<P> {
    pub fn new(p: ValidPrime, min: Bidegree) -> Self {
        Self {
            p,
            classes: Arc::new(DenseBigradedModule::new(min)),
            differentials: BiVec::new(min.x()),
            permanent_classes: BiVec::new(min.x()),
            page_data: BiVec::new(min.x()),
            invalid: BiVec::new(min.x()),
            profile: PhantomData,
        }
    }

    pub fn min(&self) -> Bidegree {
        self.classes.min()
    }

    pub fn classes(&self) -> Arc<DenseBigradedModule> {
        Arc::clone(&self.classes)
    }

    pub fn range(&self, x: i32) -> std::ops::Range<i32> {
        self.classes.range(x)
    }

    pub fn max(&self) -> Bidegree {
        self.classes.max()
    }

    pub fn defined(&self, b: Bidegree) -> bool {
        self.classes.defined(b)
    }

    pub fn set_dimension(&mut self, b: Bidegree, dim: usize) {
        // This already ensures it is valid to set b
        self.classes.set_dimension(b, dim);
        if self.differentials.len() == b.x() {
            let min_y = self.classes.min().y();
            self.differentials.push(BiVec::new(min_y));
            self.permanent_classes.push(BiVec::new(min_y));
            self.page_data.push(BiVec::new(min_y));
            self.invalid.push(BiVec::new(min_y));
        }

        self.differentials[b.x()].push(BiVec::new(P::MIN_R));
        self.page_data[b.x()].push(BiVec::new(P::MIN_R));
        self.page_data[b.x()][b.y()].push(Subquotient::new_full(self.p, dim));
        self.permanent_classes[b.x()].push(Subspace::new(self.p, dim));
        self.invalid[b.x()].push(false);
    }

    pub fn clear(&mut self) {
        self.permanent_classes
            .iter_mut()
            .flatten()
            .for_each(Subspace::set_to_zero);
        self.differentials
            .iter_mut()
            .flatten()
            .flatten()
            .for_each(Differential::set_to_zero);
        self.page_data
            .iter_mut()
            .flatten()
            .flatten()
            .for_each(Subquotient::set_to_full);
        self.invalid.iter_mut().flatten().for_each(|x| *x = true);
    }

    pub fn dimension(&self, b: Bidegree) -> usize {
        self.classes.dimension(b)
    }

    /// # Returns
    ///
    /// Whether a new permanent class was added
    pub fn add_permanent_class(&mut self, elem: &BidegreeElement) -> bool {
        let old_dim = self.permanent_classes[elem.x()][elem.y()].dimension();
        let new_dim = self.permanent_classes[elem.x()][elem.y()].add_vector(elem.vec());
        if old_dim != new_dim {
            // This was a new permanent class
            for d in self.differentials[elem.x()][elem.y()].iter_mut() {
                d.add(elem.vec(), None);
            }
            self.invalid[elem.x()][elem.y()] = true;
        }
        old_dim != new_dim
    }

    /// Ensure `self.differentials[b.x()][b.y()][r]` is defined. Must call `extend_page_data` on the source
    /// and target after this.
    fn extend_differential(&mut self, r: i32, b: Bidegree) {
        let source_dim = self.classes.dimension(b);
        while self.differentials[b.x()][b.y()].len() <= r {
            let r = self.differentials[b.x()][b.y()].len();
            let target = P::profile(r, b);
            let mut differential =
                Differential::new(self.p, source_dim, self.classes.dimension(target));

            for class in self.permanent_classes[b.x()][b.y()].basis() {
                differential.add(class, None);
            }
            self.differentials[b.x()][b.y()].push(differential);
        }
    }

    /// Ensure `self.page_data[b.x()][b.y()][r]` is defined
    fn extend_page_data(&mut self, r: i32, b: Bidegree) {
        let page_data = &mut self.page_data[b.x()][b.y()];
        while page_data.len() <= r {
            page_data.push(page_data.last().unwrap().clone())
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
            self.differentials[source.x()][source.y()][r].add(source.vec(), None);
            self.extend_page_data(r + 1, P::profile(r, source.degree()));
        }
        let is_new = self.differentials[source.x()][source.y()][r].add(source.vec(), Some(target));
        if is_new {
            self.invalid[source.x()][source.y()] = true;
            if !target.is_zero() {
                self.invalid[target_b.x()][target_b.y()] = true;
                self.add_permanent_class(&BidegreeElement::new(target_b, target.to_owned()));
                for r in r + 1..self.page_data[target_b.x()][target_b.y()].len() {
                    self.page_data[target_b.x()][target_b.y()][r].quotient(target);

                    let p = P::profile_inverse(r, target_b);
                    if self.defined(p) {
                        self.invalid[p.x()][p.y()] = true;
                    }
                }
            }
        }
        is_new
    }

    pub fn invalid(&self, b: Bidegree) -> bool {
        self.invalid[b.x()][b.y()]
    }

    pub fn update(&mut self) {
        for x in self.invalid.range() {
            for y in self.invalid[x].range() {
                if self.invalid[x][y] {
                    self.update_bidegree(Bidegree::x_y(x, y));
                }
            }
        }
    }

    /// This returns the vec of differentials to draw on each page.
    pub fn update_bidegree(&mut self, b: Bidegree) -> BiVec<Vec<Vec<u32>>> {
        self.invalid[b.x()][b.y()] = false;
        for (r, d) in self.differentials[b.x()][b.y()].iter_mut_enum() {
            let target_b = P::profile(r, b);
            d.reduce_target(self.page_data[target_b.x()][target_b.y()][r].zeros());
        }

        // For each page, the array of differentials to draw
        let mut differentials: BiVec<Vec<Vec<u32>>> =
            BiVec::with_capacity(P::MIN_R, self.differentials[b.x()][b.y()].len());

        for r in self.page_data[b.x()][b.y()].range().skip(1) {
            let target_b = P::profile(r - 1, b);

            self.page_data[b.x()][b.y()][r].clear_gens();

            if r > self.differentials[b.x()][b.y()].len()
                || self.page_data[target_b.x()][target_b.y()][r - 1].is_empty()
            {
                let (prev, cur) = self.page_data[b.x()][b.y()].split_borrow_mut(r - 1, r);
                for g in prev.gens() {
                    cur.add_gen(g);
                }
                if r - 1 < self.differentials[b.x()][b.y()].len() {
                    differentials.push(vec![
                        Vec::new();
                        self.page_data[b.x()][b.y()][r].dimension()
                    ]);
                }
            } else {
                let d = &self.differentials[b.x()][b.y()][r - 1];

                let source_dim = self.dimension(b);
                let target_dim = self.dimension(target_b);

                let mut drawn_differentials: Vec<Vec<u32>> =
                    Vec::with_capacity(self.page_data[b.x()][b.y()][r - 1].dimension());

                let mut dvec = FpVector::new(self.p, target_dim);
                let mut matrix = Matrix::new(
                    self.p,
                    self.page_data[b.x()][b.y()][r - 1].dimension(),
                    source_dim + target_dim,
                );

                for (mut row, g) in std::iter::zip(
                    matrix.iter_mut(),
                    self.page_data[b.x()][b.y()][r - 1].gens(),
                ) {
                    row.slice_mut(target_dim, target_dim + source_dim).assign(g);

                    d.evaluate(g, dvec.as_slice_mut());
                    row.slice_mut(0, target_dim).assign(dvec.as_slice());

                    drawn_differentials.push(
                        self.page_data[target_b.x()][target_b.y()][r - 1]
                            .reduce(dvec.as_slice_mut()),
                    );
                    dvec.set_to_zero();
                }
                differentials.push(drawn_differentials);

                matrix.row_reduce();

                let first_kernel_row = matrix.find_first_row_in_block(target_dim);

                for row in matrix.iter().skip(first_kernel_row) {
                    if row.is_zero() {
                        break;
                    }
                    self.page_data[b.x()][b.y()][r]
                        .add_gen(row.restrict(target_dim, target_dim + source_dim));
                }
            }
        }
        differentials
    }

    /// Whether the calcuations at bidegree (x, y) are complete. This means all classes on the
    /// final page are known to be permanent.
    pub fn complete(&self, b: Bidegree) -> bool {
        self.page_data[b.x()][b.y()]
            .last()
            .unwrap()
            .gens()
            .all(|v| self.permanent_classes[b.x()][b.y()].contains(v))
    }

    /// Whether there is an inconsistent differential involving bidegree (x, y).
    pub fn inconsistent(&self, b: Bidegree) -> bool {
        self.differentials(b).iter().any(Differential::inconsistent)
            || self.differentials_hitting(b).any(|(_, d)| d.inconsistent())
    }

    pub fn differentials(&self, b: Bidegree) -> &BiVec<Differential> {
        &self.differentials[b.x()][b.y()]
    }

    pub fn differentials_hitting(
        &self,
        b: Bidegree,
    ) -> impl Iterator<Item = (i32, &'_ Differential)> + '_ {
        let max_r = self.page_data[b.x()][b.y()].len() - 1;
        (P::MIN_R..max_r).filter_map(move |r| {
            let source_b = P::profile_inverse(r, b);
            Some((
                r,
                self.differentials
                    .get(source_b.x())?
                    .get(source_b.y())?
                    .get(r)?,
            ))
        })
    }

    pub fn permanent_classes(&self, b: Bidegree) -> &Subspace {
        &self.permanent_classes[b.x()][b.y()]
    }

    pub fn page_data(&self, b: Bidegree) -> &BiVec<Subquotient> {
        &self.page_data[b.x()][b.y()]
    }

    /// Compute the product between `product` and the class `class` at `(x, y)`. Returns `None` if
    /// the product is not yet computed.
    pub fn multiply(&self, elem: &BidegreeElement, prod: &Product) -> Option<BidegreeElement> {
        let mut result = FpVector::new(self.p, self.classes.get_dimension(elem.degree() + prod.b)?);
        if let Some(matrix) = &prod.matrices.get(elem.x())?.get(elem.y())? {
            matrix.apply(result.as_slice_mut(), 1, elem.vec());
        }
        Some(BidegreeElement::new(elem.degree() + prod.b, result))
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
        let mut result = FpVector::new(self.p, self.classes.get_dimension(result_b)?);

        if r == result_r {
            let diffs = &self.differentials[elem.x()][elem.y()][r];
            let d_b = P::profile(r, elem.degree());
            let mut dx = FpVector::new(self.p, self.classes.dimension(d_b));
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

        for x in min.x()..=max.x() {
            for y in self.range(x) {
                let b = Bidegree::x_y(x, y);
                let shifted_b = b - min;

                let data = self.page_data(b).get_max(r);
                if data.is_empty() {
                    continue;
                }

                g.node(shifted_b, data.dimension())?;

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
                    if let Some(matrix) = &prod.matrices[source_b.x()][source_b.y()] {
                        let matrix = Subquotient::reduce_matrix(matrix, source_data, data);
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
                                data.reduce(s.as_slice_mut()),
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
        let mut sseq = Sseq::<Adams>::new(p, Bidegree::zero());
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
        let mut sseq = Sseq::<Adams>::new(p, Bidegree::zero());

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
            e.assert_eq(&sseq.page_data[b.x()][b.y()][r].to_string());
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
