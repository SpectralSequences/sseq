use crate::actions::*;
use crate::Sender;
use bivec::BiVec;
use chart::Graph;
use fp::matrix::{Matrix, Subquotient, Subspace};
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use serde::{Deserialize, Serialize};
use std::cmp::{max, Ordering};
use std::collections::HashMap;

const MIN_PAGE: i32 = 2;
pub const INFINITY: i32 = std::i32::MAX;

fn sseq_profile(r: i32, x: i32, y: i32) -> (i32, i32) {
    (x - 1, y + r)
}
fn sseq_profile_i(r: i32, x: i32, y: i32) -> (i32, i32) {
    (x + 1, y - r)
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClassState {
    Error,
    Done,
    InProgress,
}

pub struct Differential {
    matrix: Matrix,
    source_dim: usize,
    target_dim: usize,
    error: bool,
}

impl Differential {
    pub fn new(p: ValidPrime, source_dim: usize, target_dim: usize) -> Self {
        let mut matrix = Matrix::new(p, source_dim + 1, source_dim + target_dim);
        matrix.initialize_pivots();
        Differential {
            matrix,
            source_dim,
            target_dim,
            error: false,
        }
    }

    pub fn set_to_zero(&mut self) {
        self.matrix.set_to_zero();
        for x in self.matrix.pivots_mut() {
            *x = -1;
        }
        self.error = false;
    }

    pub fn add(
        &mut self,
        source: &FpVector,
        target: Option<&FpVector>,
        reduce_by: Option<&Subspace>,
    ) {
        let source_dim = self.source_dim;
        let target_dim = self.target_dim;
        let last_row = &mut self.matrix[source_dim];
        last_row.slice_mut(0, source_dim).add(source.as_slice(), 1);

        let mut last_row = last_row.slice_mut(source_dim, source_dim + target_dim);
        match target {
            Some(t) => {
                last_row.add(t.as_slice(), 1);
                if let Some(s) = reduce_by {
                    s.reduce(last_row);
                }
            }
            None => last_row.set_to_zero(),
        };

        self.matrix.row_reduce();

        // Check that the differentials are consistent with each other.
        for i in 0..self.target_dim {
            if self.matrix.pivots()[self.source_dim + i] >= 0 {
                self.error = true;
            }
        }
    }

    pub fn get_source_target_pairs(&mut self) -> Vec<(FpVector, FpVector)> {
        let p = self.matrix.prime();
        let source_dim = self.source_dim;
        let target_dim = self.target_dim;
        self.matrix
            .iter_mut()
            .filter(|d| !d.is_zero())
            .map(|d| {
                let mut source = FpVector::new(p, source_dim);
                let mut target = FpVector::new(p, target_dim);

                source.as_slice_mut().assign(d.slice(0, source_dim));
                target
                    .as_slice_mut()
                    .assign(d.slice(source_dim, source_dim + target_dim));

                (source, target)
            })
            .collect::<Vec<_>>()
    }

    /// Given a subspace of the target space, project the target vectors to the complement.
    pub fn reduce_target(&mut self, zeros: &Subspace) {
        assert_eq!(zeros.matrix.columns(), self.target_dim);

        for i in 0..self.matrix.rows() {
            zeros.reduce(
                self.matrix[i].slice_mut(self.source_dim, self.source_dim + self.target_dim),
            );
        }

        // Knowing that things are zero might fix our previous erroneous differentials.
        self.matrix.row_reduce();

        self.error = false;
        for i in 0..self.target_dim {
            if self.matrix.pivots()[self.source_dim + i] >= 0 {
                self.error = true;
            }
        }
    }

    /// This evaluates the differential on `source`, adding the result to `target`. This assumes
    /// all unspecified differentials are zero. More precisely, it assumes every non-pivot column
    /// of the differential matrix has zero differential. This may or may not be actually true
    /// (e.g. if we only know d(a + b) = c, it might be that d(a) = c and d(b) = 0, or vice versa,
    /// or neither. Here we assume d(a) = c and d(b) = 0.
    pub fn evaluate(&self, mut source: FpVector, target: &mut FpVector) {
        for i in 0..self.source_dim {
            let row = self.matrix.pivots()[i];
            if row < 0 {
                continue;
            }
            let row = row as usize;

            let c = source.entry(i);
            if c == 0 {
                continue;
            }
            for j in 0..self.target_dim {
                target.add_basis_element(j, c * self.matrix[row].entry(self.source_dim + j));
            }
            for j in 0..self.source_dim {
                source.add_basis_element(j, (*self.prime() - 1) * c * self.matrix[row].entry(j));
            }
        }
    }

    pub fn prime(&self) -> ValidPrime {
        self.matrix.prime()
    }
}

/// # Fields
///  * `matrices[x][y]` : This encodes the matrix of the product. If it is None, it means the
///  target of the product has dimension 0.
pub struct Product {
    name: String,
    x: i32,
    y: i32,
    left: bool,
    user: bool, // whether the product was specified by the user or the module. Products specified by the module are assumed to be permanent
    permanent: bool, // whether the product class is a permanent class
    differential: Option<(i32, bool, usize)>, // The first entry is the page of the differential. The second entry is whether or not this product is the source or target of the differential. The last index is the index of the other end of the differential.
    matrices: BiVec<BiVec<Option<Matrix>>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProductItem {
    name: String,
    mult_x: i32,
    mult_y: i32,
    matrices: BiVec<Vec<Vec<u32>>>, // page -> matrix
}

/// Here are some blanket assumptions we make about the order in which we add things.
///  * If we add a class at (x, y), then all classes to the left and below of (x, y) have been
///  computed. Moreover, every class at (x + 1, y - r) for r >= 1 have been computed. If these have
///  not been set, the class is assumed to be zero.
///  * The same is true for products, where the grading of a product is that of its source.
///  * Whenever a product v . x is set, the target is already set.
pub struct Sseq {
    pub p: ValidPrime,
    name: SseqChoice,
    min_x: i32,
    min_y: i32,

    /// If this is a positive number, then the spectral sequence will not re-compute classes and
    /// edges. See [`Actions::BlockRefresh`] for details.
    pub block_refresh: u32,
    sender: Option<Sender>,
    page_list: Vec<i32>,
    product_name_to_index: HashMap<String, usize>,
    products: Vec<Product>,
    /// x -> y -> number of elements
    classes: BiVec<BiVec<usize>>,
    /// x -> y -> idx -> name
    class_names: BiVec<BiVec<Vec<String>>>,
    /// x -> y -> r -> differential
    differentials: BiVec<BiVec<BiVec<Differential>>>,
    /// x -> y -> permanent classes
    permanent_classes: BiVec<BiVec<Subspace>>,
    /// x -> y -> r -> E_r^{x, y} as a subquotient of the original bidegree.
    page_data: BiVec<BiVec<BiVec<Subquotient>>>,
}

impl Sseq {
    pub fn new(
        p: ValidPrime,
        name: SseqChoice,
        min_x: i32,
        min_y: i32,
        sender: Option<Sender>,
    ) -> Self {
        fp::vector::initialize_limb_bit_index_table(p);
        let mut classes = BiVec::new(min_x - 1); // We have an extra column to the left so that differentials have something to hit.
        classes.push(BiVec::new(min_y));
        Self {
            p,
            min_x,
            min_y,
            name,
            sender,
            block_refresh: 0,

            page_list: vec![2],
            product_name_to_index: HashMap::new(),
            products: Vec::new(),
            classes,
            class_names: BiVec::new(min_x),
            permanent_classes: BiVec::new(min_x),
            differentials: BiVec::new(min_x),
            page_data: BiVec::new(min_x),
        }
    }

    /// This clears out all the user actions. This is intended to be used when we undo, where
    /// we clear out all actions then redo the existing actions. Hence we avoid re-allocating
    /// as much as possible because we are likely to need the space anyway
    pub fn clear(&mut self) {
        for prod in &mut self.products {
            if prod.user {
                prod.permanent = false;
            }
            prod.differential = None;
        }

        // We initialize to 0 and add_page so that we send it out too.
        self.page_list = vec![];
        self.add_page(MIN_PAGE);

        for x in self.min_x..self.classes.len() {
            for y in self.min_y..self.classes[x].len() {
                self.permanent_classes[x][y].set_to_zero();
                for d in self.differentials[x][y].iter_mut() {
                    d.set_to_zero();
                }
                for page_data in self.page_data[x][y].iter_mut() {
                    page_data.set_to_zero();
                }
            }
        }

        self.refresh_all();
    }

    pub fn refresh_all(&mut self) {
        if self.block_refresh > 0 {
            return;
        }
        for x in self.min_x..self.classes.len() {
            for y in self.min_y..self.classes[x].len() {
                self.compute_classes(x, y, false);
            }
        }
        for x in self.min_x..self.classes.len() {
            for y in self.min_y..self.classes[x].len() {
                self.compute_edges(x, y);
            }
        }
    }

    /// Adds a page to the page list, which is the list of pages where something changes from the
    /// previous page. This is mainly used by the `add_differential` function.
    fn add_page(&mut self, r: i32) {
        if !self.page_list.contains(&r) {
            self.page_list.push(r);
            self.page_list.sort_unstable();

            self.send(Message {
                recipients: vec![],
                sseq: self.name,
                action: Action::from(SetPageList {
                    page_list: self.page_list.clone(),
                }),
            });
        }
    }

    /// Initializes `differentials[x][y][r]`. It sets the differentials of all known permament
    /// classes to 0.
    fn allocate_differential_matrix(&mut self, r: i32, x: i32, y: i32) {
        let source_dim = self.classes[x][y];
        let (tx, ty) = sseq_profile(r, x, y);
        let target_dim = self.classes[tx][ty];
        let p = self.p;
        let mut d = Differential::new(p, source_dim, target_dim);
        for vec in self.permanent_classes[x][y].basis() {
            d.add(vec, None, None);
        }
        self.differentials[x][y].push(d);
    }

    /// Given a class `class` at `(x, y)` and a Product object `product`, compute the product of
    /// the class with the product. Returns the new coordinate of the product as well as the actual
    /// product. The result is None if the product is not yet computed.
    fn multiply(
        &self,
        x: i32,
        y: i32,
        class: &FpVector,
        product: &Product,
    ) -> Option<(i32, i32, FpVector)> {
        let prod_x = product.x;
        let prod_y = product.y;
        if !self.class_defined(x + prod_x, y + prod_y) {
            return None;
        }

        let mut prod = FpVector::new(self.p, self.classes[x + prod_x][y + prod_y]);

        if self.classes[x][y] == 0 {
            return Some((x + prod_x, y + prod_y, prod));
        }

        if let Some(matrix) = &product.matrices.get(x)?.get(y)? {
            matrix.apply(prod.as_slice_mut(), 1, class.as_slice());
        }
        Some((x + prod_x, y + prod_y, prod))
    }

    /// Apply the Leibniz rule to obtain new differentials. The differential we start with is a d_r
    /// differential from (x, y) with source `s` and target `t`. If the target is None, then it
    /// means `s` is *permanent*. In this case, r should be set to INFINITY. On the other hand, if
    /// `t` is zero, it simply means d_r is zero.
    ///
    /// The other object we multiply with is the product with index `si`. If `si` is permanent,
    /// we simply multiply the differenial with `si`. If `si` is non-permanent but has a
    /// differential starting *from* it, we apply Leibniz to that differential. If `si` is the
    /// *target* of a product differential, we do nothing.
    ///
    /// If `si` is permanent and `t` is None, then we mark s * si to be permanent as well.
    ///
    /// # Return
    ///
    /// We return a pair `(r_, x_, y_, s_, t_)` which is the data of the new differential --- its
    /// page, starting coordinate and source and target vectors. Again, if s * si is permanent,
    /// then t_ is set to None. If there is not enough products computed to calculate the result,
    /// or if s * si is zero, we return None.
    fn leibniz(
        &self,
        r: i32,
        x: i32,
        y: i32,
        s: &FpVector,
        t: Option<&FpVector>,
        si: usize,
    ) -> Option<(i32, i32, i32, FpVector, Option<FpVector>)> {
        let product = &self.products[si];
        // First compute s * si.
        let (x_, y_, new_source) = self.multiply(x, y, s, product)?;

        if new_source.is_zero() {
            return None;
        }

        if product.permanent {
            if let Some(t_) = t {
                let (tx, ty) = sseq_profile(r, x, y);
                let (_, _, mut new_target) = self.multiply(tx, ty, t_, product)?;
                if product.left && product.x % 2 != 0 {
                    new_target.scale(*self.p - 1);
                }
                return Some((r, x_, y_, new_source, Some(new_target)));
            } else {
                return Some((INFINITY, x_, y_, new_source, None));
            }
        }

        if let Some((r_, true, ti)) = product.differential {
            match r_.cmp(&r) {
                Ordering::Less => {
                    // The original differential from s to t is useless.
                    let (_, _, mut new_target) = self.multiply(x, y, s, &self.products[ti])?;
                    if !product.left && (x - 1) % 2 != 0 {
                        new_target.scale(*self.p - 1);
                    }
                    return Some((r_, x_, y_, new_source, Some(new_target)));
                }
                Ordering::Greater => {
                    // This is more-or-less the same as the permanent code, except we know t is not
                    // permanent (or else it would be handled by the previous case).
                    if let Some(t_) = t {
                        let (tx, ty) = sseq_profile(r, x, y);
                        let (_, _, mut new_target) = self.multiply(tx, ty, t_, product)?;
                        if product.left && product.x % 2 != 0 {
                            new_target.scale(*self.p - 1);
                        }
                        return Some((r, x_, y_, new_source, Some(new_target)));
                    }
                }
                Ordering::Equal => {
                    // This is the sum of the two above.
                    let (_, _, mut new_target) = self.multiply(x, y, s, &self.products[ti])?;
                    if !product.left && (x - 1) % 2 != 0 {
                        new_target.scale(*self.p - 1);
                    }
                    if let Some(t_) = t {
                        let (tx, ty) = sseq_profile(r, x, y);
                        let (_, _, mut tmp) = self.multiply(tx, ty, t_, product)?;
                        if product.left && product.x % 2 != 0 {
                            tmp.scale(*self.p - 1);
                        }
                        new_target.add(&tmp, 1);
                    }

                    return Some((r, x_, y_, new_source, Some(new_target)));
                }
            }
        }
        None
    }

    /// Computes products whose source is at (x, y).
    fn compute_edges(&self, x: i32, y: i32) {
        if self.block_refresh > 0 {
            return;
        }

        if !self.class_defined(x, y) {
            return;
        }
        if self.classes[x][y] == 0 {
            return;
        }

        if let Some(sender) = &self.sender {
            let mut structlines: Vec<ProductItem> = Vec::with_capacity(self.products.len());
            for mult in &self.products {
                if !(mult.matrices.len() > x && mult.matrices[x].len() > y) {
                    continue;
                }
                let target_dim = self.classes[x + mult.x][y + mult.y];
                if target_dim == 0 {
                    continue;
                }

                if let Some(matrix) = &mult.matrices[x][y] {
                    let max_page = max(
                        self.page_data[x][y].len(),
                        self.page_data[x + mult.x][y + mult.y].len(),
                    );
                    let mut matrices: BiVec<Vec<Vec<u32>>> =
                        BiVec::with_capacity(MIN_PAGE, max_page);

                    // E_2 page
                    matrices.push(matrix.to_vec());

                    // Compute the ones where something changes.
                    for r in MIN_PAGE + 1..max_page {
                        let source_data = Sseq::get_page(r, &self.page_data[x][y]);
                        let target_data =
                            Sseq::get_page(r, &self.page_data[x + mult.x][y + mult.y]);

                        matrices.push(Subquotient::reduce_matrix(
                            &matrix,
                            source_data,
                            target_data,
                        ));

                        // In the case where the source is empty, we still want one empty array to
                        // indicate that no structlines should be drawn from this page on.
                        if source_data.is_empty() {
                            break;
                        }
                    }

                    structlines.push(ProductItem {
                        name: mult.name.clone(),
                        mult_x: mult.x,
                        mult_y: mult.y,
                        matrices,
                    });
                }
            }

            sender
                .send(Message {
                    recipients: vec![],
                    sseq: self.name,
                    action: Action::from(SetStructline { x, y, structlines }),
                })
                .unwrap();
        }
    }

    /// A version of [`Sseq::compute_next_page_with_d`] under the knowledge that there is no
    /// $d_r$ differential at $(x, y)$. This only requires updating the list of generators.
    /// on that page.
    fn compute_next_page_no_d(&mut self, r: i32, x: i32, y: i32) {
        if self.page_data[x][y].len() <= r {
            let new = self.page_data[x][y][r - 1].clone();
            self.page_data[x][y].push(new);
            return;
        }

        let (prev, cur) = self.page_data[x][y].split_borrow_mut(r - 1, r);
        cur.clear_gens();
        for gen in prev.gens() {
            cur.add_gen(gen.as_slice());
        }
    }

    /// Update the generators at (x, y) on the rth page, assuming the (r - 1)th page has been
    /// updated.
    ///
    /// This returns a matrix, which is the (estimated) matrix of $d_{r - 1}$ differentials to be
    /// drawn on the chart, expressed in terms of the $E_{r - 1}$ page basis. This is estimated
    /// because we only have partial information about the differentials.
    fn compute_next_page_with_d(&mut self, r: i32, x: i32, y: i32) -> Vec<Vec<u32>> {
        let (tx, ty) = sseq_profile(r - 1, x, y);
        if Sseq::get_page(r - 1, &self.page_data[tx][ty]).is_empty() {
            self.compute_next_page_no_d(r, x, y);
            return vec![Vec::new(); self.page_data[x][y][r].dimension()];
        }

        // Ensure we have something in this bidegree
        if self.page_data[x][y].len() <= r {
            let new = self.page_data[x][y][r - 1].clone();
            self.page_data[x][y].push(new);
        }

        // Clear out all existing generators of this subspace. To be added later.
        self.page_data[x][y][r].clear_gens();

        let (target_classes, source_classes) = self.page_data.split_borrow_mut(tx, x);
        let target_classes = &target_classes[ty][r - 1];
        let source_classes = &mut source_classes[y];

        let d = &self.differentials[x][y][r - 1];

        let source_dim = d.source_dim;
        let target_dim = d.target_dim;

        let mut vectors: Vec<FpVector> = Vec::with_capacity(source_classes[r - 1].dimension());
        let mut differentials: Vec<Vec<u32>> =
            Vec::with_capacity(source_classes[r - 1].dimension());

        let mut dvec = FpVector::new(self.p, target_dim);
        for vec in source_classes[r - 1].gens() {
            let mut result = FpVector::new(self.p, target_dim + source_dim);
            result
                .slice_mut(target_dim, target_dim + source_dim)
                .assign(vec.as_slice());

            d.evaluate(vec.clone(), &mut dvec);
            target_classes.zeros().reduce(dvec.as_slice_mut());

            result.slice_mut(0, source_dim).add(dvec.as_slice(), 1);

            vectors.push(result);
            differentials.push(target_classes.reduce(dvec.as_slice_mut()));
            dvec.set_to_zero();
        }

        let mut matrix = Matrix::from_rows(self.p, vectors, source_dim + target_dim);
        matrix.row_reduce();

        let first_kernel_row = matrix.find_first_row_in_block(target_dim);

        for row in &matrix[first_kernel_row..] {
            if row.is_zero() {
                break;
            }
            source_classes[r].add_gen(row.slice(target_dim, target_dim + source_dim));
        }
        differentials
    }

    /// At any point in time, the data of what is quotiented out in a bidegree is always accurate.
    /// However, the set of generators need not be, and we update that information in this
    /// function.
    ///
    /// # Arguments
    ///  * `refresh_edge` - Whether to automatically call compute_edges after computing class. This should
    ///  almost always be yes, unless we are re-computing everything, in which case this
    ///  will result in many duplicate calls of compute_edge.
    fn compute_classes(&mut self, x: i32, y: i32, refresh_edge: bool) {
        if self.block_refresh > 0 {
            return;
        }

        if !self.class_defined(x, y) {
            return;
        }

        let source_dim = self.classes[x][y];
        if source_dim == 0 {
            self.page_data[x][y] = BiVec::from_vec(MIN_PAGE, vec![Subquotient::new(self.p, 0)]);
            return;
        }

        let max_page = max(
            self.page_data[x][y].len(),
            self.differentials[x][y].len() + 1,
        );

        let mut differentials: BiVec<Vec<Vec<u32>>> =
            BiVec::with_capacity(MIN_PAGE, self.differentials[x][y].len());

        for r in MIN_PAGE + 1..max_page {
            if self.page_data[x][y][r - 1].is_empty() && self.page_data[x][y].len() > r {
                self.page_data[x][y][r].clear_gens();
            }

            if self.differentials[x][y].len() < r {
                // There won't be any further differentials from now on, so we don't get indexing
                // errors from not pushing
                self.compute_next_page_no_d(r, x, y);
            } else {
                differentials.push(self.compute_next_page_with_d(r, x, y));
            }
        }

        self.send_class_data(x, y);

        if !differentials.is_empty() {
            // `true_differentials` is a list of differentials of the form d(source) = target we know
            // to be true. `differentials` is our best guess at what the matrix of differentials is.
            let mut true_differentials =
                Vec::with_capacity(self.differentials[x][y].len() as usize);

            for r in MIN_PAGE..self.differentials[x][y].len() {
                let d = &mut self.differentials[x][y][r];
                let pairs = d.get_source_target_pairs();
                let (tx, ty) = sseq_profile(r, x, y);

                true_differentials.push(
                    pairs
                        .into_iter()
                        .map(|(mut s, mut t)| {
                            (
                                Sseq::get_page(r, &self.page_data[x][y]).reduce(s.as_slice_mut()),
                                Sseq::get_page(r, &self.page_data[tx][ty]).reduce(t.as_slice_mut()),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            }

            self.send(Message {
                recipients: vec![],
                sseq: self.name,
                action: Action::from(SetDifferential {
                    x,
                    y,
                    true_differentials,
                    differentials,
                }),
            });
        }

        if refresh_edge {
            self.compute_edges(x, y);
            for prod in &self.products {
                self.compute_edges(x - prod.x, y - prod.y);
            }
        }
    }

    fn send_class_data(&self, x: i32, y: i32) {
        if self.block_refresh > 0 {
            return;
        }

        let mut error = false;
        for r in MIN_PAGE..self.differentials[x][y].len() {
            error |= self.differentials[x][y][r].error;
        }
        for r in self.get_differentials_hitting(x, y) {
            let (sx, sy) = sseq_profile_i(r, x, y);
            error |= self.differentials[sx][sy][r].error;
        }

        let state;
        if error {
            state = ClassState::Error;
        } else if self.page_data[x][y]
            .last()
            .unwrap()
            .gens()
            .all(|c| self.permanent_classes[x][y].contains(c.as_slice()))
        {
            state = ClassState::Done;
        } else {
            state = ClassState::InProgress;
        }

        let mut decompositions: Vec<(FpVector, String, i32, i32)> = Vec::new();
        for prod in &self.products {
            if !self.product_defined(x - prod.x, y - prod.y, prod) {
                continue;
            }
            if let Some(matrix) = &prod.matrices[x - prod.x][y - prod.y] {
                for i in 0..matrix.len() {
                    if matrix[i].is_zero() {
                        continue;
                    }
                    decompositions.push((
                        matrix[i].clone(),
                        format!(
                            "{} {}",
                            prod.name,
                            self.class_names[x - prod.x][y - prod.y][i]
                        ),
                        prod.x,
                        prod.y,
                    ));
                }
            }
        }

        self.send(Message {
            recipients: vec![],
            sseq: self.name,
            action: Action::from(SetClass {
                x,
                y,
                state,
                permanents: self.permanent_classes[x][y].basis().to_vec(),
                class_names: self.class_names[x][y].clone(),
                decompositions,
                classes: self.page_data[x][y]
                    .iter()
                    .map(|x| x.gens().cloned().collect())
                    .collect::<Vec<Vec<FpVector>>>(),
            }),
        });
    }

    fn send(&self, msg: Message) {
        if let Some(sender) = &self.sender {
            sender.send(msg).unwrap();
        }
    }
}

// Wrapper functions
impl Sseq {
    fn product_defined(&self, x: i32, y: i32, product: &Product) -> bool {
        self.class_defined(x, y)
            && product.matrices.max_degree() >= x
            && product.matrices[x].max_degree() >= y
    }

    fn class_defined(&self, x: i32, y: i32) -> bool {
        x >= self.min_x
            && y >= self.min_y
            && self.classes.max_degree() >= x
            && self.classes[x].max_degree() >= y
    }

    fn get_page<T>(r: i32, bivec: &BiVec<T>) -> &T {
        if r >= bivec.len() {
            &bivec[bivec.max_degree()]
        } else {
            &bivec[r]
        }
    }

    /// Get a list of r for which there is a d_r differential hitting (x, y)
    fn get_differentials_hitting(&self, x: i32, y: i32) -> Vec<i32> {
        let max_r = self.page_data[x][y].len() - 1;

        (MIN_PAGE..max_r)
            .filter(|&r| {
                let (sx, sy) = sseq_profile_i(r, x, y);
                sx >= self.min_x
                    && sy >= self.min_y
                    && self.differentials.max_degree() >= sx
                    && self.differentials[sx].max_degree() >= sy
                    && self.differentials[sx][sy].max_degree() >= r
            })
            .collect::<Vec<i32>>()
    }
}
// Functions called by SseqManager
impl Sseq {
    /// This function should only be called when everything to the left and bottom of (x, y)
    /// has been defined.
    pub fn set_class(&mut self, x: i32, y: i32, num: usize) {
        if x == self.min_x {
            self.classes[self.min_x - 1].push(0);
        }
        if x == self.classes.len() {
            self.classes.push(BiVec::new(self.min_y));
            self.class_names.push(BiVec::new(self.min_y));
            self.differentials.push(BiVec::new(self.min_y));
            self.permanent_classes.push(BiVec::new(self.min_y));
            self.page_data.push(BiVec::new(self.min_y));
        }

        assert_eq!(self.classes[x].len(), y);
        assert_eq!(self.permanent_classes[x].len(), y);
        self.classes[x].push(num);
        let mut names = Vec::with_capacity(num);
        if num == 1 {
            names.push(format!("x_{{{},{}}}", x, y));
        } else {
            for i in 0..num {
                names.push(format!("x_{{{}, {}}}^{{({})}}", x, y, i));
            }
        }
        self.class_names[x].push(names);
        self.permanent_classes[x].push(Subspace::new(self.p, num + 1, num));
        self.differentials[x].push(BiVec::new(MIN_PAGE));
        self.page_data[x].push(BiVec::from_vec(
            MIN_PAGE,
            vec![Subquotient::new_full(self.p, num)],
        ));

        self.compute_classes(x, y, true);
    }

    pub fn set_class_name(&mut self, x: i32, y: i32, idx: usize, name: String) {
        self.class_names[x][y][idx] = name;
        self.send_class_data(x, y);
        for prod in &self.products {
            if self.class_defined(x + prod.x, y + prod.y) {
                self.send_class_data(x + prod.x, y + prod.y);
            }
        }
    }

    /// Add a differential starting at (x, y). This mutates the target by reducing it via
    /// `self.page_data[x - 1][y + r][r].zeros()`
    ///
    /// Panics if the target of the differential is not yet defined
    pub fn add_differential(
        &mut self,
        r: i32,
        x: i32,
        y: i32,
        source: &FpVector,
        target: &FpVector,
    ) {
        assert_eq!(
            source.dimension(),
            self.classes[x][y],
            "length of source vector not equal to dimension of source"
        );
        assert_eq!(
            target.dimension(),
            self.classes[x - 1][y + r],
            "length of target vector not equal to dimension of target"
        );

        // We cannot use extend_with here because of borrowing rules.
        if self.differentials[x][y].len() <= r {
            for r_ in self.differentials[x][y].len()..=r {
                self.allocate_differential_matrix(r_, x, y);
            }
        }

        let (tx, ty) = sseq_profile(r, x, y);

        self.differentials[x][y][r].add(
            source,
            Some(&target),
            Some(Sseq::get_page(r, &self.page_data[tx][ty]).zeros()),
        );
        for i in MIN_PAGE..r {
            self.differentials[x][y][i].add(source, None, None)
        }

        while self.page_data[tx][ty].len() <= r + 1 {
            let new = self.page_data[tx][ty].last().unwrap().clone();
            self.page_data[tx][ty].push(new);
        }

        for r_ in r + 1..self.page_data[tx][ty].len() {
            self.page_data[tx][ty][r_].quotient(target.as_slice());

            let (px, py) = sseq_profile_i(r_, tx, ty);
            if self.class_defined(px, py) && self.differentials[px][py].len() > r_ {
                self.differentials[px][py][r_].reduce_target(self.page_data[tx][ty][r_].zeros());
            }
        }

        // add_permanent_class in turn sets the differentials on the targets of the differentials
        // to 0. add_differential_propagate will take care of propagating this.
        self.add_permanent_class(tx, ty, target);

        self.add_page(r);
        self.add_page(r + 1);

        self.compute_classes(tx, ty, true);
        self.compute_classes(x, y, true);

        // page_data[r] will be populated if there is a non-zero differential hit on a
        // page <= r - 1. Check if these differentials now hit 0.
        // TODO: this needs fixing
        for r_ in r + 1..self.page_data[tx][ty].len() - 1 {
            let (px, py) = sseq_profile_i(r_, tx, ty);
            self.compute_classes(px, py, true);
        }
    }

    /// This function recursively propagates differentials. If this function is called, it will add
    /// the corresponding differential plus all products of index at least product_index. Here we
    /// have to exercise a slight bit of care to ensure we don't set both $p_1 p_2 d$ and $p_2 p_1
    /// d$ when $p_1$, $p_2$ are products and $d$ is the differential. Our strategy is that we
    /// compute $p_2 p_1 d$ if and only if $p_1$ comes earlier in the list of products than $p_2$.
    pub fn add_differential_propagate(
        &mut self,
        r: i32,
        x: i32,
        y: i32,
        source: &FpVector,
        target: &Option<FpVector>,
        product_index: usize,
    ) {
        let num_products = self.products.len();
        match product_index.cmp(&(num_products - 1)) {
            Ordering::Equal => {
                match target.as_ref() {
                    Some(target_) => self.add_differential(r, x, y, source, target_),
                    None => self.add_permanent_class(x, y, source),
                };
            }
            Ordering::Less => {
                self.add_differential_propagate(r, x, y, source, target, product_index + 1);
            }
            Ordering::Greater => {
                panic!("Product index greater than number of products")
            }
        }

        // Separate this to new line to make code easier to read.
        let new_d = self.leibniz(r, x, y, source, target.as_ref(), product_index);

        if let Some((r_, x_, y_, source_, target_)) = new_d {
            self.add_differential_propagate(r_, x_, y_, &source_, &target_, product_index);
        }
    }

    pub fn add_permanent_class(&mut self, x: i32, y: i32, class: &FpVector) {
        self.permanent_classes[x][y].add_vector(class.as_slice());
        for r in MIN_PAGE..self.differentials[x][y].len() {
            self.differentials[x][y][r].add(class, None, None);
        }
        self.compute_classes(x, y, true);
    }

    /// Add a product to the list of products, but don't add any computed product
    pub fn add_product_type(
        &mut self,
        name: &str,
        mult_x: i32,
        mult_y: i32,
        left: bool,
        permanent: bool,
    ) {
        let idx = self.product_name_to_index.get(name);

        if let Some(&i) = idx {
            self.products[i].user = true;
            if permanent && !self.products[i].permanent {
                self.products[i].permanent = true;
                self.repropagate_product(i);
            }
        } else {
            let product = Product {
                name: name.to_string(),
                x: mult_x,
                y: mult_y,
                user: true,
                left,
                permanent,
                differential: None,
                matrices: BiVec::new(self.min_x),
            };
            self.products.push(product);
            self.product_name_to_index
                .insert(name.to_string(), self.products.len() - 1);
        }
    }

    #[allow(clippy::ptr_arg)]
    pub fn add_product_differential(&mut self, source: &String, target: &String) {
        let source_idx = *self.product_name_to_index.get(source).unwrap();
        let target_idx = *self.product_name_to_index.get(target).unwrap();

        let r = self.products[target_idx].y - self.products[source_idx].y;

        self.products[source_idx].differential = Some((r, true, target_idx));
        self.products[target_idx].differential = Some((r, false, source_idx));

        self.repropagate_product(source_idx);
    }

    fn repropagate_product(&mut self, idx: usize) {
        let max_x = self.products[idx].matrices.len();
        for x in self.min_x..max_x {
            let max_y = self.products[idx].matrices[x].len();
            for y in self.min_y..max_y {
                for r in MIN_PAGE..self.differentials[x][y].len() {
                    let d = &mut self.differentials[x][y][r];
                    for (source, target) in d.get_source_target_pairs() {
                        let new_d = self.leibniz(r, x, y, &source, Some(&target), idx);
                        if let Some((r_, x_, y_, source_, Some(target_))) = new_d {
                            self.add_differential(r_, x_, y_, &source_, &target_);
                        }
                    }
                }

                // Find a better way to do this. This is to circumevent borrow checker.
                let classes = self.permanent_classes[x][y].basis().to_vec();
                for class in classes {
                    let new_d = self.leibniz(INFINITY, x, y, &class, None, idx);
                    if let Some((r_, x_, y_, source_, t_)) = new_d {
                        match t_ {
                            Some(target_) => self.add_differential(r_, x_, y_, &source_, &target_),
                            None => self.add_permanent_class(x_, y_, &source_),
                        };
                    }
                }
            }
        }
    }

    pub fn add_product(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        mult_x: i32,
        mult_y: i32,
        left: bool,
        matrix: &[Vec<u32>],
    ) {
        assert!(self.class_defined(x, y));
        assert!(self.class_defined(x + mult_x, y + mult_y));
        let idx: usize = match self.product_name_to_index.get(name) {
            Some(i) => *i,
            None => {
                let product = Product {
                    name: name.to_string(),
                    x: mult_x,
                    y: mult_y,
                    user: false,
                    left,
                    permanent: true,
                    differential: None,
                    matrices: BiVec::new(self.min_x),
                };
                self.products.push(product);
                self.product_name_to_index
                    .insert(name.to_string(), self.products.len() - 1);
                self.products.len() - 1
            }
        };
        while x > self.products[idx].matrices.len() {
            self.products[idx].matrices.push(BiVec::new(self.min_y));
        }
        if x == self.products[idx].matrices.len() {
            self.products[idx].matrices.push(BiVec::new(self.min_y));
        }
        while y > self.products[idx].matrices[x].len() {
            self.products[idx].matrices[x].push(None);
        }

        assert_eq!(y, self.products[idx].matrices[x].len());
        self.products[idx].matrices[x].push(Some(Matrix::from_vec(self.p, matrix)));

        // We propagate all differentials that *hit* us, because of the order in which products
        // are added. The exception is if this product is the target of a product
        // differential on page r, we propagate the d_r differential *starting* at (x, y).
        if self.products[idx].differential.is_some() && !self.products[idx].differential.unwrap().1
        {
            let (r, _, si) = self.products[idx].differential.unwrap();
            if self.differentials[x][y].len() > r {
                let d = &mut self.differentials[x][y][r];
                for (source, target) in d.get_source_target_pairs() {
                    let new_d = self.leibniz(r, x, y, &source, Some(&target), si);
                    if let Some((r_, x_, y_, source_, Some(target_))) = new_d {
                        self.add_differential(r_, x_, y_, &source_, &target_);
                    }
                }
            }

            let classes = self.permanent_classes[x][y].basis().to_vec();
            for class in classes {
                let new_d = self.leibniz(INFINITY, x, y, &class, None, si);
                if let Some((r_, x_, y_, source_, Some(target_))) = new_d {
                    self.add_differential(r_, x_, y_, &source_, &target_);
                }
            }
        } else {
            for r in self.get_differentials_hitting(x, y) {
                let (sx, sy) = sseq_profile_i(r, x, y);
                let d = &mut self.differentials[sx][sy][r];
                for (source, target) in d.get_source_target_pairs() {
                    let new_d = self.leibniz(r, sx, sy, &source, Some(&target), idx);
                    if let Some((r_, x_, y_, source_, Some(target_))) = new_d {
                        self.add_differential(r_, x_, y_, &source_, &target_);
                    }
                }
            }

            // Find a better way to do this. This is to circumevent borrow checker.
            let classes = self.permanent_classes[x][y].basis().to_vec();
            for class in classes {
                let new_d = self.leibniz(INFINITY, x, y, &class, None, idx);
                if let Some((r_, x_, y_, source_, t_)) = new_d {
                    match t_ {
                        Some(target_) => self.add_differential(r_, x_, y_, &source_, &target_),
                        None => self.add_permanent_class(x_, y_, &source_),
                    };
                }
            }
        }
        self.compute_edges(x, y);
        self.send_class_data(x + mult_x, y + mult_y);
    }
}

use std::io::Write;

impl Sseq {
    /// This doesn't actually modify the object
    pub fn write_to_svg(
        &mut self,
        out: impl Write,
        r: i32,
        differentials: bool,
        products: &[&str],
    ) -> std::io::Result<()> {
        assert_eq!(self.min_x, 0);
        assert_eq!(self.min_y, 0);

        let max_x = self.page_data.iter().count() - 1;
        let max_y = self
            .page_data
            .iter()
            .map(|d| d.iter().count())
            .max()
            .unwrap_or(1)
            - 1;

        let mut g = Graph::new(out, max_x as i32, max_y as i32)?;

        for (x, data) in self.page_data.iter_enum() {
            for (y, data) in data.iter_enum() {
                let data = Sseq::get_page(r, data);
                if data.is_empty() {
                    continue;
                }

                g.node(x, y, data.dimension())?;

                // Now add the products hitting this bidegree
                for &prod_name in products {
                    let prod_idx = *self.product_name_to_index.get(prod_name).unwrap();
                    let prod = &self.products[prod_idx];
                    let source_x = x - prod.x;
                    let source_y = y - prod.y;

                    if !self.class_defined(source_x, source_y) {
                        continue;
                    }

                    let source_data = Sseq::get_page(r, &self.page_data[source_x][source_y]);
                    if source_data.is_empty() {
                        continue;
                    }

                    let matrix = prod.matrices[source_x][source_y].as_ref().unwrap();
                    let matrix = Subquotient::reduce_matrix(&matrix, source_data, data);
                    g.structline_matrix((source_x, source_y), (x, y), matrix, None)?;
                }

                // Finally add the differentials
                if differentials {
                    let (tx, ty) = sseq_profile(r, x, y);
                    if tx < 0 {
                        continue;
                    }
                    if self.differentials[x][y].len() <= r {
                        continue;
                    }
                    let d = &mut self.differentials[x][y][r];
                    let target_data = Sseq::get_page(r, &self.page_data[tx][ty]);

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
                                g.structline((x, y, i), (tx, ty, j), Some(&format!("d{}", r)))?;
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
    use super::*;
    use expect_test::{expect, Expect};

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_sseq_differential() {
        let p = ValidPrime::new(3);
        fp::vector::initialize_limb_bit_index_table(p);
        let mut sseq = crate::sseq::Sseq::new(p, SseqChoice::Main, 0, 0, None);
        sseq.set_class(0, 0, 1);
        sseq.set_class(1, 0, 2);
        sseq.set_class(1, 1, 2);
        sseq.set_class(0, 1, 0);
        sseq.set_class(0, 2, 3);
        sseq.set_class(0, 3, 1);

        sseq.add_differential(
            2,
            1,
            0,
            &FpVector::from_slice(p, &[1, 1]),
            &FpVector::from_slice(p, &[0, 1, 2]),
        );

        sseq.add_differential(
            3,
            1,
            0,
            &FpVector::from_slice(p, &[1, 0]),
            &FpVector::from_slice(p, &[1]),
        );

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
            &FpVector::from_slice(p, &[1, 0]),
            &FpVector::from_slice(p, &[1]),
        );
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
        fp::vector::initialize_limb_bit_index_table(p);
        let mut sseq = crate::sseq::Sseq::new(p, SseqChoice::Main, 0, 0, None);

        sseq.set_class(0, 0, 0);
        sseq.set_class(1, 0, 2);
        sseq.set_class(0, 1, 0);
        sseq.set_class(0, 2, 2);

        sseq.add_differential(
            2,
            1,
            0,
            &FpVector::from_slice(p, &[1, 0]),
            &FpVector::from_slice(p, &[1, 0]),
        );
        sseq.add_differential(
            2,
            1,
            0,
            &FpVector::from_slice(p, &[0, 1]),
            &FpVector::from_slice(p, &[1, 1]),
        );

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
