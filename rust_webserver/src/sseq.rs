use crate::actions::*;
use crate::Sender;
use bivec::BiVec;
use chart::Backend;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use fp::{
    matrix::{Matrix, Subquotient},
    vector::Slice,
};
use serde::{Deserialize, Serialize};
use sseq::{Adams, Sseq, SseqProfile};
use std::cmp::{max, Ordering};
use std::collections::HashMap;

pub const INFINITY: i32 = std::i32::MAX;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClassState {
    Error,
    Done,
    InProgress,
}

/// # Fields
///  * `matrices[x][y]` : This encodes the matrix of the product. If it is None, it means the
///  target of the product has dimension 0.
pub struct Product {
    name: String,
    x: i32,
    y: i32,
    left: bool,
    /// whether the product was specified by the user or the module. Products specified by the module are assumed to be permanent
    user: bool,
    /// whether the product class is a permanent class
    permanent: bool,
    /// The first entry is the page of the differential. The second index is true if this is the source of the differential. The last index is the index of the other end of the differential.
    differential: Option<(i32, bool, usize)>,
    matrices: BiVec<BiVec<Option<Matrix>>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProductItem {
    name: String,
    mult_x: i32,
    mult_y: i32,
    matrices: BiVec<Vec<Vec<u32>>>, // page -> matrix
}

const CLASS_FLAG: u8 = 1;
const EDGE_FLAG: u8 = 2;

/// Here are some blanket assumptions we make about the order in which we add things.
///  * If we add a class at (x, y), then all classes to the left and below of (x, y) have been
///  computed. Moreover, every class at (x + 1, y - r) for r >= 1 have been computed. If these have
///  not been set, the class is assumed to be zero.
///  * The same is true for products, where the grading of a product is that of its source.
///  * Whenever a product v . x is set, the target is already set.
pub struct SseqWrapper<P: SseqProfile = Adams> {
    pub p: ValidPrime,
    name: SseqChoice,
    pub inner: Sseq<P>,

    /// Whether a bidegree is stale, i.e.\ new products have to be reported to the sender. Note
    /// that products "belong" to the source of the product.
    stale: BiVec<BiVec<u8>>,

    /// If this is a positive number, then the spectral sequence will not re-compute classes and
    /// edges. See [`Actions::BlockRefresh`] for details.
    pub block_refresh: u32,
    sender: Option<Sender>,
    product_name_to_index: HashMap<String, usize>,
    products: Vec<Product>,
    /// x -> y -> idx -> name
    class_names: BiVec<BiVec<Vec<String>>>,
}

impl<P: SseqProfile> SseqWrapper<P> {
    pub fn new(
        p: ValidPrime,
        name: SseqChoice,
        min_x: i32,
        min_y: i32,
        sender: Option<Sender>,
    ) -> Self {
        fp::vector::initialize_limb_bit_index_table(p);
        Self {
            p,
            name,
            sender,
            block_refresh: 0,
            inner: Sseq::new(p, min_x, min_y),

            product_name_to_index: HashMap::new(),
            products: Vec::new(),
            class_names: BiVec::new(min_x),
            stale: BiVec::new(min_x),
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

        self.inner.clear();
    }

    pub fn refresh(&mut self) {
        if self.block_refresh > 0 {
            return;
        }

        for x in self.inner.min_x()..=self.inner.max_x() {
            for y in self.inner.range(x) {
                if !self.inner.invalid(x, y) {
                    continue;
                }
                self.stale[x][y] |= CLASS_FLAG | EDGE_FLAG;
                for product in &self.products {
                    let prod_x = product.x;
                    let prod_y = product.y;
                    if self.inner.defined(x - prod_x, y - prod_y) {
                        self.stale[x - prod_x][y - prod_y] |= EDGE_FLAG;
                    }
                }
                let differentials = self.inner.update_bidegree(x, y);
                if !differentials.is_empty() {
                    // `true_differentials` is a list of differentials of the form d(source) = target we know
                    // to be true. `differentials` is our best guess at what the matrix of differentials is.
                    let true_differentials = self
                        .inner
                        .differentials(x, y)
                        .iter_enum()
                        .map(|(r, d)| {
                            let (tx, ty) = P::profile(r, x, y);
                            d.get_source_target_pairs()
                                .into_iter()
                                .map(|(mut s, mut t)| {
                                    (
                                        self.inner.page_data(x, y)[r].reduce(s.as_slice_mut()),
                                        self.inner.page_data(tx, ty)[r].reduce(t.as_slice_mut()),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();

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
            }
        }

        for x in self.stale.range() {
            for y in self.stale[x].range() {
                if self.stale[x][y] & CLASS_FLAG > 0 {
                    self.send_class_data(x, y);
                }
                if self.stale[x][y] & EDGE_FLAG > 0 {
                    self.send_products(x, y);
                }
                self.stale[x][y] = 0;
            }
        }
    }

    /// Given a class `class` at `(x, y)` and a Product object `product`, compute the product of
    /// the class with the product. Returns the new coordinate of the product as well as the actual
    /// product. The result is None if the product is not yet computed.
    fn multiply(
        &self,
        x: i32,
        y: i32,
        class: Slice,
        product: &Product,
    ) -> Option<(i32, i32, FpVector)> {
        let prod_x = product.x;
        let prod_y = product.y;
        if !self.inner.defined(x + prod_x, y + prod_y) {
            return None;
        }

        let mut prod = FpVector::new(self.p, self.inner.dimension(x + prod_x, y + prod_y));

        if self.inner.dimension(x, y) == 0 {
            return Some((x + prod_x, y + prod_y, prod));
        }

        if let Some(matrix) = &product.matrices.get(x)?.get(y)? {
            matrix.apply(prod.as_slice_mut(), 1, class);
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
        s: Slice,
        t: Option<Slice>,
        source_idx: usize,
    ) -> Option<(i32, i32, i32, FpVector, Option<FpVector>)> {
        let product = &self.products[source_idx];
        // First compute s * si.
        let (x_, y_, new_source) = self.multiply(x, y, s, product)?;

        let ret = if product.permanent {
            if let Some(t_) = t {
                let (tx, ty) = P::profile(r, x, y);
                let (_, _, mut new_target) = self.multiply(tx, ty, t_, product)?;
                if product.left && product.x % 2 != 0 {
                    new_target.scale(*self.p - 1);
                }
                Some((r, x_, y_, new_source, Some(new_target)))
            } else {
                Some((INFINITY, x_, y_, new_source, None))
            }
        } else if let Some((r_, true, ti)) = product.differential {
            match r_.cmp(&r) {
                Ordering::Less => {
                    // The original differential from s to t is useless.
                    let (_, _, mut new_target) = self.multiply(x, y, s, &self.products[ti])?;
                    if !product.left && (x - 1) % 2 != 0 {
                        new_target.scale(*self.p - 1);
                    }
                    Some((r_, x_, y_, new_source, Some(new_target)))
                }
                Ordering::Greater => {
                    // This is more-or-less the same as the permanent code, except we know t is not
                    // permanent (or else it would be handled by the previous case).
                    if let Some(t_) = t {
                        let (tx, ty) = P::profile(r, x, y);
                        let (_, _, mut new_target) = self.multiply(tx, ty, t_, product)?;
                        if product.left && product.x % 2 != 0 {
                            new_target.scale(*self.p - 1);
                        }
                        Some((r, x_, y_, new_source, Some(new_target)))
                    } else {
                        unreachable!()
                    }
                }
                Ordering::Equal => {
                    // This is the sum of the two above.
                    let (_, _, mut new_target) = self.multiply(x, y, s, &self.products[ti])?;
                    if !product.left && (x - 1) % 2 != 0 {
                        new_target.scale(*self.p - 1);
                    }
                    if let Some(t_) = t {
                        let (tx, ty) = P::profile(r, x, y);
                        let (_, _, mut tmp) = self.multiply(tx, ty, t_, product)?;
                        if product.left && product.x % 2 != 0 {
                            tmp.scale(*self.p - 1);
                        }
                        new_target.add(&tmp, 1);
                    }

                    Some((r, x_, y_, new_source, Some(new_target)))
                }
            }
        } else {
            None
        };

        if let Some((_, _, _, s, t)) = ret.as_ref() {
            if s.is_zero() && (t.is_none() || t.as_ref().unwrap().is_zero()) {
                return None;
            }
        }
        ret
    }

    /// Computes products whose source is at (x, y).
    fn send_products(&self, x: i32, y: i32) {
        if !self.inner.defined(x, y) {
            return;
        }
        if self.inner.dimension(x, y) == 0 {
            return;
        }

        let mut structlines: Vec<ProductItem> = Vec::with_capacity(self.products.len());
        for mult in &self.products {
            if !(mult.matrices.len() > x && mult.matrices[x].len() > y) {
                continue;
            }
            let target_dim = self.inner.dimension(x + mult.x, y + mult.y);
            if target_dim == 0 {
                continue;
            }

            if let Some(matrix) = &mult.matrices[x][y] {
                let max_page = max(
                    self.inner.page_data(x, y).len(),
                    self.inner.page_data(x + mult.x, y + mult.y).len(),
                );
                let mut matrices: BiVec<Vec<Vec<u32>>> = BiVec::with_capacity(P::MIN_R, max_page);

                // E_2 page
                matrices.push(matrix.to_vec());

                // Compute the ones where something changes.
                for r in P::MIN_R + 1..max_page {
                    let source_data = Self::get_page(r, self.inner.page_data(x, y));
                    let target_data =
                        Self::get_page(r, self.inner.page_data(x + mult.x, y + mult.y));

                    matrices.push(Subquotient::reduce_matrix(matrix, source_data, target_data));

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

        self.send(Message {
            recipients: vec![],
            sseq: self.name,
            action: Action::from(SetStructline { x, y, structlines }),
        });
    }

    fn send_class_data(&self, x: i32, y: i32) {
        if self.block_refresh > 0 {
            return;
        }

        let state = if self.inner.inconsistent(x, y) {
            ClassState::Error
        } else if self.inner.complete(x, y) {
            ClassState::Done
        } else {
            ClassState::InProgress
        };

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
                permanents: self.inner.permanent_classes(x, y).basis().to_vec(),
                class_names: self.class_names[x][y].clone(),
                decompositions,
                classes: self
                    .inner
                    .page_data(x, y)
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
impl<P: SseqProfile> SseqWrapper<P> {
    fn product_defined(&self, x: i32, y: i32, product: &Product) -> bool {
        self.inner.defined(x, y)
            && product.matrices.max_degree() >= x
            && product.matrices[x].max_degree() >= y
    }

    fn get_page<T>(r: i32, bivec: &BiVec<T>) -> &T {
        if r >= bivec.len() {
            &bivec[bivec.max_degree()]
        } else {
            &bivec[r]
        }
    }
}

// Functions called by SseqManager
impl<P: SseqProfile> SseqWrapper<P> {
    /// This function should only be called when everything to the left and bottom of (x, y)
    /// has been defined.
    pub fn set_dimension(&mut self, x: i32, y: i32, dim: usize) {
        self.inner.set_dimension(x, y, dim);
        if x == self.class_names.len() {
            self.class_names.push(BiVec::new(self.inner.min_y()));
            self.stale.push(BiVec::new(self.inner.min_y()));
        }
        let mut names = Vec::with_capacity(dim);
        if dim == 1 {
            names.push(format!("x_{{{},{}}}", x, y));
        } else {
            names.extend((0..dim).map(|i| format!("x_{{{}, {}}}^{{({})}}", x, y, i)));
        }
        self.class_names[x].push(names);
        self.stale[x].push(CLASS_FLAG);
    }

    pub fn set_class_name(&mut self, x: i32, y: i32, idx: usize, name: String) {
        self.class_names[x][y][idx] = name;
        self.send_class_data(x, y);
        for prod in &self.products {
            if self.inner.defined(x + prod.x, y + prod.y) {
                self.send_class_data(x + prod.x, y + prod.y);
            }
        }
    }

    /// This function recursively propagates differentials. If this function is called, it will add
    /// the corresponding differential plus all products of index at least product_index. Here we
    /// have to exercise a slight bit of care to ensure we don't set both $p_1 p_2 d$ and $p_2 p_1
    /// d$ when $p_1$, $p_2$ are products and $d$ is the differential. Our strategy is that we
    /// compute $p_2 p_1 d$ if and only if $p_1$ comes earlier in the list of products than $p_2$.
    ///
    /// # Arguments
    ///  - `added`: Whether the differential has already been added
    pub fn add_differential_propagate(
        &mut self,
        r: i32,
        x: i32,
        y: i32,
        source: Slice,
        target: Option<Slice>,
        product_index: usize,
        added: bool,
    ) {
        // This is useful for batch adding differentials from external sources, where not all
        // classes have been added.
        if !self.inner.defined(x, y) {
            return;
        }
        if target.is_some() {
            let (tx, ty) = P::profile(r, x, y);
            if !self.inner.defined(tx, ty) {
                return;
            }
        }

        if !added {
            let new = match target {
                Some(target) => self.inner.add_differential(r, x, y, source, target),
                None => self.inner.add_permanent_class(x, y, source),
            };
            // The differential is not new, so there is no need to propagate.
            if !new {
                return;
            }
        }

        if product_index + 1 < self.products.len() {
            self.add_differential_propagate(r, x, y, source, target, product_index + 1, true);
        }

        // Separate this to new line to make code easier to read.
        let new_d = self.leibniz(r, x, y, source, target, product_index);

        if let Some((r_, x_, y_, source_, target_)) = new_d {
            self.add_differential_propagate(
                r_,
                x_,
                y_,
                source_.as_slice(),
                target_.as_ref().map(FpVector::as_slice),
                product_index,
                false,
            );
        }
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
                self.propagate_product_all(i);
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
                matrices: BiVec::new(self.inner.min_x()),
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

        self.propagate_product_all(source_idx);
    }

    /// Propagate products by the product indexed by `idx`.
    fn propagate_product_all(&mut self, idx: usize) {
        // We only use this to figure out the range
        for x in self.products[idx].matrices.range() {
            for y in self.products[idx].matrices[x].range() {
                self.propagate_product(x, y, idx);
            }
        }
    }

    /// Propagate products by the product indexed by `idx` at (x, y). The product must either be
    /// permanent or the source of a differential.
    fn propagate_product(&mut self, x: i32, y: i32, idx: usize) {
        for r in self.inner.differentials(x, y).range() {
            let pairs = self.inner.differentials(x, y)[r].get_source_target_pairs();
            for (source, target) in pairs {
                let new_d = self.leibniz(r, x, y, source.as_slice(), Some(target.as_slice()), idx);
                if let Some((r_, x_, y_, source_, Some(target_))) = new_d {
                    self.inner
                        .add_differential(r_, x_, y_, source_.as_slice(), target_.as_slice());
                }
            }
        }

        // Find a better way to do this. This is to circumevent borrow checker.
        let classes = self.inner.permanent_classes(x, y).basis().to_vec();
        for class in classes {
            let new_d = self.leibniz(INFINITY, x, y, class.as_slice(), None, idx);
            if let Some((r_, x_, y_, source_, t_)) = new_d {
                match t_ {
                    Some(target_) => self.inner.add_differential(
                        r_,
                        x_,
                        y_,
                        source_.as_slice(),
                        target_.as_slice(),
                    ),
                    None => self.inner.add_permanent_class(x_, y_, source_.as_slice()),
                };
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
        assert!(self.inner.defined(x, y));
        assert!(self.inner.defined(x + mult_x, y + mult_y));
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
                    matrices: BiVec::new(self.inner.min_x()),
                };
                self.products.push(product);
                self.product_name_to_index
                    .insert(name.to_string(), self.products.len() - 1);
                self.products.len() - 1
            }
        };
        while x >= self.products[idx].matrices.len() {
            self.products[idx]
                .matrices
                .push(BiVec::new(self.inner.min_y()));
        }
        while y > self.products[idx].matrices[x].len() {
            self.products[idx].matrices[x].push(None);
        }

        let matrix = Matrix::from_vec(self.p, matrix);

        if self.inner.dimension(x, y) != 0 && self.inner.dimension(x + mult_x, y + mult_y) != 0 {
            self.stale[x][y] |= EDGE_FLAG;
            if !matrix.is_zero() {
                self.stale[x + mult_x][y + mult_y] |= CLASS_FLAG;
            }
        }

        assert_eq!(y, self.products[idx].matrices[x].len());
        self.products[idx].matrices[x].push(Some(matrix));

        // To propagate a differential on along d(α) = β, we need to compute the α product on the
        // source and target, and the β product on the source.
        if let Some((_, false, source_idx)) = self.products[idx].differential {
            self.propagate_product(x, y, source_idx);
        } else if matches!(self.products[idx].differential, Some((_, true, _)))
            || self.products[idx].permanent
        {
            self.propagate_product(x, y, idx);
            let hitting: Vec<i32> = self
                .inner
                .differentials_hitting(x, y)
                .map(|(r, _)| r)
                .collect();
            for r in hitting {
                let (sx, sy) = P::profile_inverse(r, x, y);
                if self.inner.defined(sx, sy) {
                    self.propagate_product(sx, sy, idx);
                }
            }
        }
    }
}

impl<P: SseqProfile> SseqWrapper<P> {
    pub fn write_to_graph<T: Backend>(
        &self,
        mut g: T,
        r: i32,
        differentials: bool,
        products: &[&str],
    ) -> std::result::Result<(), T::Error> {
        assert_eq!(self.inner.min_x(), 0);
        assert_eq!(self.inner.min_y(), 0);

        let max_x = self.inner.max_x();
        let max_y = self.inner.max_y();

        g.init(max_x as i32, max_y as i32)?;

        for x in self.inner.min_x()..=self.inner.max_x() {
            for y in self.inner.range(x) {
                let data = Self::get_page(r, self.inner.page_data(x, y));
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

                    if !self.inner.defined(source_x, source_y) {
                        continue;
                    }

                    let source_data = Self::get_page(r, self.inner.page_data(source_x, source_y));
                    if source_data.is_empty() {
                        continue;
                    }

                    let matrix = prod.matrices[source_x][source_y].as_ref().unwrap();
                    let matrix = Subquotient::reduce_matrix(matrix, source_data, data);
                    g.structline_matrix((source_x, source_y), (x, y), matrix, None)?;
                }

                // Finally add the differentials
                if differentials {
                    let (tx, ty) = P::profile(r, x, y);
                    if tx < 0 {
                        continue;
                    }
                    let d = self.inner.differentials(x, y);
                    if d.len() <= r {
                        continue;
                    }
                    let d = &d[r];
                    let target_data = Self::get_page(r, self.inner.page_data(tx, ty));

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
