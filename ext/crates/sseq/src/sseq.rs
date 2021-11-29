use crate::bigraded::DenseBigradedModule;
use crate::differential::Differential;
use bivec::BiVec;
use fp::{
    matrix::{Matrix, Subquotient, Subspace},
    prime::ValidPrime,
    vector::{FpVector, Slice},
};
use std::{marker::PhantomData, sync::Arc};

/// The direction of the differentials
pub trait SseqProfile {
    const MIN_R: i32;
    fn profile(r: i32, x: i32, y: i32) -> (i32, i32);
    fn profile_inverse(r: i32, x: i32, y: i32) -> (i32, i32);
}

pub struct Adams;

impl SseqProfile for Adams {
    const MIN_R: i32 = 2;
    fn profile(r: i32, x: i32, y: i32) -> (i32, i32) {
        (x - 1, y + r)
    }
    fn profile_inverse(r: i32, x: i32, y: i32) -> (i32, i32) {
        (x + 1, y - r)
    }
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
    ///  - if `differential[x][y][r]` is defined, then `page_data[x][y][r + 1]` and `page_data[tx][ty][r + 1]` are always defined,
    page_data: BiVec<BiVec<BiVec<Subquotient>>>,

    /// x -> y -> validity. A bidegree is invalid if the page_data is no longer accurate.
    invalid: BiVec<BiVec<bool>>,

    // Docs: If your struct does not in fact own the data of type T, it is better to use a
    // reference type, like PhantomData<&'a T> (ideally) or PhantomData<*const T> (if no lifetime
    // applies), so as not to indicate ownership.
    profile: PhantomData<*const P>,
}

impl<P: SseqProfile> Sseq<P> {
    pub fn new(p: ValidPrime, min_x: i32, min_y: i32) -> Self {
        Self {
            p,
            classes: Arc::new(DenseBigradedModule::new(min_x, min_y)),
            differentials: BiVec::new(min_x),
            permanent_classes: BiVec::new(min_x),
            page_data: BiVec::new(min_x),
            invalid: BiVec::new(min_x),
            profile: PhantomData,
        }
    }

    pub fn min_x(&self) -> i32 {
        self.classes.min_x()
    }

    pub fn min_y(&self) -> i32 {
        self.classes.min_y()
    }

    pub fn classes(&self) -> Arc<DenseBigradedModule> {
        Arc::clone(&self.classes)
    }

    pub fn range(&self, x: i32) -> std::ops::Range<i32> {
        self.classes.range(x)
    }

    pub fn max_x(&self) -> i32 {
        self.classes.max_x()
    }

    pub fn max_y(&self) -> i32 {
        self.classes.max_y()
    }

    pub fn defined(&self, x: i32, y: i32) -> bool {
        self.classes.defined(x, y)
    }

    pub fn set_dimension(&mut self, x: i32, y: i32, dim: usize) {
        // This already ensures it is valid to set x, y
        self.classes.set_dimension(x, y, dim);
        if self.differentials.len() == x {
            let min_y = self.classes.min_y();
            self.differentials.push(BiVec::new(min_y));
            self.permanent_classes.push(BiVec::new(min_y));
            self.page_data.push(BiVec::new(min_y));
            self.invalid.push(BiVec::new(min_y));
        }

        self.differentials[x].push(BiVec::new(P::MIN_R));
        self.page_data[x].push(BiVec::new(P::MIN_R));
        self.page_data[x][y].push(Subquotient::new_full(self.p, dim));
        self.permanent_classes[x].push(Subspace::new(self.p, dim + 1, dim));
        self.invalid[x].push(false);
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

    pub fn dimension(&self, x: i32, y: i32) -> usize {
        self.classes.dimension(x, y)
    }

    /// # Returns
    ///
    /// Whether a new permanent class was added
    pub fn add_permanent_class(&mut self, x: i32, y: i32, class: Slice) -> bool {
        let old_dim = self.permanent_classes[x][y].dimension();
        let new_dim = self.permanent_classes[x][y].add_vector(class);
        if old_dim != new_dim {
            // This was a new permanent class
            for d in self.differentials[x][y].iter_mut() {
                d.add(class, None);
            }
            self.invalid[x][y] = true;
        }
        old_dim != new_dim
    }

    /// Ensure `self.differentials[x][y][r]` is defined. Must call `extend_page_data` on the source
    /// and target after this.
    fn extend_differential(&mut self, r: i32, x: i32, y: i32) {
        let source_dim = self.classes.dimension(x, y);
        while self.differentials[x][y].len() <= r {
            let r = self.differentials[x][y].len();
            let (target_x, target_y) = P::profile(r, x, y);
            let mut differential = Differential::new(
                self.p,
                source_dim,
                self.classes.dimension(target_x, target_y),
            );

            for class in self.permanent_classes[x][y].basis() {
                differential.add(class.as_slice(), None);
            }
            self.differentials[x][y].push(differential);
        }
    }

    /// Ensure `self.page_data[x][y][r]` is defined
    fn extend_page_data(&mut self, r: i32, x: i32, y: i32) {
        let page_data = &mut self.page_data[x][y];
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
    pub fn add_differential(
        &mut self,
        r: i32,
        x: i32,
        y: i32,
        source: Slice,
        target: Slice,
    ) -> bool {
        let (tx, ty) = P::profile(r, x, y);

        self.extend_differential(r, x, y);
        self.extend_page_data(r + 1, x, y);
        self.extend_page_data(r + 1, tx, ty);

        for r in P::MIN_R..r {
            self.differentials[x][y][r].add(source, None);
            let (tx, ty) = P::profile(r, x, y);
            self.extend_page_data(r + 1, tx, ty);
        }
        let is_new = self.differentials[x][y][r].add(source, Some(target));
        if is_new {
            self.invalid[x][y] = true;
            if !target.is_zero() {
                self.invalid[tx][ty] = true;
                self.add_permanent_class(tx, ty, target);
                for r in r + 1..self.page_data[tx][ty].len() {
                    self.page_data[tx][ty][r].quotient(target);

                    let (px, py) = P::profile_inverse(r, tx, ty);
                    if self.defined(px, py) {
                        self.invalid[px][py] = true;
                    }
                }
            }
        }
        is_new
    }

    pub fn invalid(&self, x: i32, y: i32) -> bool {
        self.invalid[x][y]
    }

    pub fn update(&mut self) {
        for x in self.invalid.range() {
            for y in self.invalid[x].range() {
                if self.invalid[x][y] {
                    self.update_bidegree(x, y);
                }
            }
        }
    }

    /// This returns the vec of differentials to draw on each page.
    pub fn update_bidegree(&mut self, x: i32, y: i32) -> BiVec<Vec<Vec<u32>>> {
        self.invalid[x][y] = false;
        for (r, d) in self.differentials[x][y].iter_mut_enum() {
            let (tx, ty) = P::profile(r, x, y);
            d.reduce_target(self.page_data[tx][ty][r].zeros());
        }

        // For each page, the array of differentials to draw
        let mut differentials: BiVec<Vec<Vec<u32>>> =
            BiVec::with_capacity(P::MIN_R, self.differentials[x][y].len());

        for r in self.page_data[x][y].range().skip(1) {
            let (tx, ty) = P::profile(r - 1, x, y);

            self.page_data[x][y][r].clear_gens();

            if r > self.differentials[x][y].len() || self.page_data[tx][ty][r - 1].is_empty() {
                let (prev, cur) = self.page_data[x][y].split_borrow_mut(r - 1, r);
                for gen in prev.gens() {
                    cur.add_gen(gen.as_slice());
                }
                if r - 1 < self.differentials[x][y].len() {
                    differentials.push(vec![Vec::new(); self.page_data[x][y][r].dimension()]);
                }
            } else {
                let d = &self.differentials[x][y][r - 1];

                let source_dim = self.dimension(x, y);
                let target_dim = self.dimension(tx, ty);

                let mut vectors: Vec<FpVector> =
                    Vec::with_capacity(self.page_data[x][y][r - 1].dimension());

                let mut drawn_differentials: Vec<Vec<u32>> =
                    Vec::with_capacity(self.page_data[x][y][r - 1].dimension());

                let mut dvec = FpVector::new(self.p, target_dim);
                vectors.extend(self.page_data[x][y][r - 1].gens().map(|gen| {
                    let mut result = FpVector::new(self.p, target_dim + source_dim);
                    result
                        .slice_mut(target_dim, target_dim + source_dim)
                        .assign(gen.as_slice());

                    d.evaluate(gen.as_slice(), dvec.as_slice_mut());
                    result.slice_mut(0, target_dim).assign(dvec.as_slice());

                    drawn_differentials
                        .push(self.page_data[tx][ty][r - 1].reduce(dvec.as_slice_mut()));
                    dvec.set_to_zero();
                    result
                }));
                differentials.push(drawn_differentials);

                let mut matrix = Matrix::from_rows(self.p, vectors, source_dim + target_dim);
                matrix.row_reduce();

                let first_kernel_row = matrix.find_first_row_in_block(target_dim);

                for row in &matrix[first_kernel_row..] {
                    if row.is_zero() {
                        break;
                    }
                    self.page_data[x][y][r].add_gen(row.slice(target_dim, target_dim + source_dim));
                }
            }
        }
        differentials
    }

    /// Whether the calcuations at bidegree (x, y) are complete. This means all classes on the
    /// final page are known to be permanent.
    pub fn complete(&self, x: i32, y: i32) -> bool {
        self.page_data[x][y]
            .last()
            .unwrap()
            .gens()
            .all(|v| self.permanent_classes[x][y].contains(v.as_slice()))
    }

    /// Whether there is an inconsistent differential involving bidegree (x, y).
    pub fn inconsistent(&self, x: i32, y: i32) -> bool {
        self.differentials(x, y)
            .iter()
            .any(Differential::inconsistent)
            || self
                .differentials_hitting(x, y)
                .any(|(_, d)| d.inconsistent())
    }

    pub fn differentials(&self, x: i32, y: i32) -> &BiVec<Differential> {
        &self.differentials[x][y]
    }

    pub fn differentials_hitting(
        &self,
        x: i32,
        y: i32,
    ) -> impl Iterator<Item = (i32, &'_ Differential)> + '_ {
        let max_r = self.page_data[x][y].len() - 1;
        (P::MIN_R..max_r).filter_map(move |r| {
            let (sx, sy) = P::profile_inverse(r, x, y);
            Some((r, self.differentials.get(sx)?.get(sy)?.get(r)?))
        })
    }

    pub fn permanent_classes(&self, x: i32, y: i32) -> &Subspace {
        &self.permanent_classes[x][y]
    }

    pub fn page_data(&self, x: i32, y: i32) -> &BiVec<Subquotient> {
        &self.page_data[x][y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{expect, Expect};

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_sseq_differential() {
        let p = ValidPrime::new(3);
        let mut sseq = Sseq::<Adams>::new(p, 0, 0);
        sseq.set_dimension(0, 0, 1);
        sseq.set_dimension(1, 0, 2);
        sseq.set_dimension(1, 1, 2);
        sseq.set_dimension(0, 1, 0);
        sseq.set_dimension(0, 2, 3);
        sseq.set_dimension(0, 3, 1);

        sseq.add_differential(
            2,
            1,
            0,
            FpVector::from_slice(p, &[1, 1]).as_slice(),
            FpVector::from_slice(p, &[0, 1, 2]).as_slice(),
        );

        sseq.add_differential(
            3,
            1,
            0,
            FpVector::from_slice(p, &[1, 0]).as_slice(),
            FpVector::from_slice(p, &[1]).as_slice(),
        );

        sseq.update();

        let check = |x, y, r, e: Expect| {
            e.assert_eq(&sseq.page_data[x][y][r].to_string());
        };

        check(
            1,
            0,
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            1,
            0,
            3,
            expect![[r#"
            Generators:
            [1, 0]

            Zeros:

        "#]],
        );
        check(
            1,
            0,
            4,
            expect![[r#"
            Generators:

            Zeros:

        "#]],
        );

        check(
            1,
            1,
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );

        check(
            0,
            2,
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
            0,
            2,
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
            0,
            3,
            2,
            expect![[r#"
            Generators:
            [1]

            Zeros:

        "#]],
        );
        check(
            0,
            3,
            3,
            expect![[r#"
            Generators:
            [1]

            Zeros:

        "#]],
        );
        check(
            0,
            3,
            4,
            expect![[r#"
            Generators:

            Zeros:
            [1]

        "#]],
        );

        sseq.add_differential(
            2,
            1,
            1,
            FpVector::from_slice(p, &[1, 0]).as_slice(),
            FpVector::from_slice(p, &[1]).as_slice(),
        );
        sseq.update();
        let check = |x, y, r, e: Expect| {
            e.assert_eq(&sseq.page_data[x][y][r].to_string());
        };

        check(
            1,
            0,
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            1,
            0,
            3,
            expect![[r#"
            Generators:
            [1, 0]

            Zeros:

        "#]],
        );
        check(
            1,
            0,
            4,
            expect![[r#"
            Generators:
            [1, 0]

            Zeros:

        "#]],
        );

        check(
            1,
            1,
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            1,
            1,
            3,
            expect![[r#"
            Generators:
            [0, 1]

            Zeros:

        "#]],
        );

        check(
            0,
            2,
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
            0,
            2,
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
            0,
            3,
            2,
            expect![[r#"
            Generators:
            [1]

            Zeros:

        "#]],
        );
        check(
            0,
            3,
            3,
            expect![[r#"
            Generators:

            Zeros:
            [1]

        "#]],
        );
        check(
            0,
            3,
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
        let mut sseq = Sseq::<Adams>::new(p, 0, 0);

        sseq.set_dimension(0, 0, 0);
        sseq.set_dimension(1, 0, 2);
        sseq.set_dimension(0, 1, 0);
        sseq.set_dimension(0, 2, 2);

        sseq.add_differential(
            2,
            1,
            0,
            FpVector::from_slice(p, &[1, 0]).as_slice(),
            FpVector::from_slice(p, &[1, 0]).as_slice(),
        );
        sseq.add_differential(
            2,
            1,
            0,
            FpVector::from_slice(p, &[0, 1]).as_slice(),
            FpVector::from_slice(p, &[1, 1]).as_slice(),
        );
        sseq.update();

        let check = |x, y, r, e: Expect| {
            e.assert_eq(&sseq.page_data[x][y][r].to_string());
        };

        check(
            1,
            0,
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            1,
            0,
            3,
            expect![[r#"
            Generators:

            Zeros:

        "#]],
        );
        check(
            0,
            2,
            2,
            expect![[r#"
            Generators:
            [1, 0]
            [0, 1]

            Zeros:

        "#]],
        );
        check(
            0,
            2,
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
