use rust_ext::matrix::{Subspace, Matrix};
use rust_ext::fp_vector::{FpVector, FpVectorT};
use std::collections::HashMap;
use std::cmp::max;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use bivec::BiVec;
use crate::actions::*;
use crate::Sender;

const MIN_PAGE : i32 = 2;
pub const INFINITY : i32 = std::i32::MAX;

/// Given a vector `elt`, a subspace `zeros` of the total space (with a specified choice of
/// complement) and a basis `basis` of a subspace of the complement, project `elt` to the complement and express
/// as a linear combination of the basis. This assumes the projection of `elt` is indeed in the
/// span of `basis`. The result is returned as a list of coefficients.
///
/// If `zeros` is none, then the initial projection is not performed.
fn express_basis(mut elt : &mut FpVector, zeros : Option<&Subspace>, basis : &(Vec<isize>, Vec<FpVector>)) -> Vec<u32>{
    if let Some(z) = zeros {
        z.reduce(&mut elt);
    }
    let mut result = Vec::with_capacity(basis.0.len());
    for i in 0 .. basis.0.len() {
        if basis.0[i] < 0 {
            continue;
        }
        let c = elt.entry(i);
        result.push(c);
        if c != 0 {
            elt.add(&basis.1[basis.0[i] as usize], ((elt.prime() - 1) * c) % elt.prime());
        }
    }
//    assert!(elt.is_zero());
    result
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClassState {
    Error,
    Done,
    InProgress
}

pub struct Differential {
    matrix : Matrix,
    source_dim : usize,
    target_dim : usize,
    column_to_pivots_row : Vec<isize>,
    error : bool,
}

impl Differential {
    pub fn new(p : u32, source_dim : usize, target_dim : usize) -> Self {
        Differential {
            matrix : Matrix::new(p, source_dim + 1, source_dim + target_dim),
            source_dim,
            target_dim,
            column_to_pivots_row : vec![-1; source_dim + target_dim],
            error : false
        }
    }

    pub fn set_to_zero(&mut self) {
        self.matrix.set_to_zero();
        for x in &mut self.column_to_pivots_row {
            *x = -1;
        }
        self.error = false;
    }

    pub fn add(&mut self, source : &FpVector, target : Option<&FpVector>) {
        let source_dim = self.source_dim;
        let target_dim = self.target_dim;
        let last_row = &mut self.matrix[source_dim];
        last_row.set_slice(0, source_dim);
        last_row.add(source, 1);
        last_row.clear_slice();

        last_row.set_slice(source_dim, source_dim + target_dim);
        match target {
            Some(t) => last_row.shift_add(t, 1),
            None => last_row.set_to_zero()
        };
        last_row.clear_slice();

        self.matrix.row_reduce(&mut self.column_to_pivots_row);

        // Check that the differentials are consistent with each other.
        for i in 0 .. self.target_dim {
            if self.column_to_pivots_row[self.source_dim + i] >= 0 {
                self.error = true;
            }
        }
    }

    pub fn get_source_target_pairs(&mut self) -> Vec<(FpVector, FpVector)> {
        let p = self.matrix.prime();
        let source_dim = self.source_dim;
        let target_dim = self.target_dim;
        self.matrix.iter_mut()
            .filter(|d| !d.is_zero())
            .map(|d| {
                let mut source = FpVector::new(p, source_dim);
                let mut target = FpVector::new(p, target_dim);

                d.set_slice(0, source_dim);
                source.add(&d, 1);
                d.clear_slice();

                d.set_slice(source_dim, source_dim + target_dim);
                target.shift_add(&d, 1);
                d.clear_slice();
                (source, target)
            }).collect::<Vec<_>>()
    }

    /// Given a subspace of the target space, project the target vectors to the complement.
    pub fn reduce_target(&mut self, zeros : &Subspace) {
        assert_eq!(zeros.matrix.columns(), self.target_dim);

        self.matrix.set_slice(0, self.matrix.rows(), self.source_dim, self.source_dim + self.target_dim);
        for i in 0 .. self.matrix.rows() {
            zeros.shift_reduce(&mut self.matrix[i]);
        }
        self.matrix.clear_slice();

        // Knowing that things are zero might fix our previous erroneous differentials.
        self.matrix.row_reduce(&mut self.column_to_pivots_row);

        self.error = false;
        for i in 0 .. self.target_dim {
            if self.column_to_pivots_row[self.source_dim + i] >= 0 {
                self.error = true;
            }
        }

    }

    /// This evaluates the differential on `source`, adding the result to `target`. This assumes
    /// all unspecified differentials are zero. More precisely, it assumes every non-pivot column
    /// of the differential matrix has zero differential. This may or may not be actually true
    /// (e.g. if we only know d(a + b) = c, it might be that d(a) = c and d(b) = 0, or vice versa,
    /// or neither. Here we assume d(a) = c and d(b) = 0.
    pub fn evaluate(&self, mut source : FpVector, target: &mut FpVector) {
        for i in 0 .. self.source_dim {
            let row = self.column_to_pivots_row[i];
            if row < 0 {
                continue;
            }
            let row = row as usize;

            let c = source.entry(i);
            if c == 0 {
                continue;
            }
            for j in 0 .. self.target_dim {
                target.add_basis_element(j, c * self.matrix[row].entry(self.source_dim + j));
            }
            for j in 0 .. self.source_dim {
                source.add_basis_element(j, (self.prime() - 1) * c * self.matrix[row].entry(j));
            }
        }
    }

    pub fn prime(&self) -> u32 {
        self.matrix.prime()
    }
}

/// # Fields
///  * `matrices[x][y]` : This encodes the matrix of the product. If it is None, it means the
///  target of the product has dimension 0.
pub struct Product {
    name : String,
    x : i32,
    y : i32,
    left : bool,
    user : bool, // whether the product was specified by the user or the module. Products specified by the module are assumed to be permanent
    permanent : bool, // whether the product class is a permanent class
    differential : Option<(i32, bool, usize)>, // The first entry is the page of the differential. The second entry is whether or not this product is the source or target of the differential. The last index is the index of the other end of the differential.
    matrices : BiVec<BiVec<Option<Matrix>>>
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProductItem {
    name : String,
    mult_x : i32,
    mult_y : i32,
    matrices : BiVec<Vec<Vec<u32>>> // page -> matrix
}

/// Here are some blanket assumptions we make about the order in which we add things.
///  * If we add a class at (x, y), then all classes to the left and below of (x, y) have been
///  computed. Moreover, every class at (x + 1, y - r) for r >= 1 have been computed. If these have
///  not been set, the class is assumed to be zero.
///  * The same is true for products, where the grading of a product is that of its source.
///  * Whenever a product v . x is set, the target is already set.
///
/// # Fields
///  * `block_refresh` : If this is a positive number, then the spectral sequence will not
///  re-compute classes and edges. See `Actions::BlockRefresh` for details.
pub struct Sseq {
    pub p : u32,
    name : SseqChoice,
    min_x : i32,
    min_y : i32,

    pub block_refresh : u32,
    sender : Option<Sender>,
    page_list : Vec<i32>,
    product_name_to_index : HashMap<String, usize>,
    products : Arc<RwLock<Vec<Product>>>,
    classes : BiVec<BiVec<usize>>, // x -> y -> number of elements
    class_names : BiVec<BiVec<Vec<String>>>, // x -> y -> idx -> name
    differentials : BiVec<BiVec<BiVec<Differential>>>, // x -> y -> r -> differential
    permanent_classes : BiVec<BiVec<Subspace>>, // x -> y -> r -> permanent classes
    zeros : Arc<RwLock<BiVec<BiVec<BiVec<Subspace>>>>>, // x -> y -> r -> subspace of elements that are zero on page r
    page_classes : Arc<RwLock<BiVec<BiVec<BiVec<(Vec<isize>, Vec<FpVector>)>>>>>, // x -> y -> r -> list of generators on the page.
}

impl Sseq {
    pub fn new(p : u32, name : SseqChoice, min_x : i32, min_y : i32, sender : Option<Sender>) -> Self {
        rust_ext::fp_vector::initialize_limb_bit_index_table(p);
        let mut classes = BiVec::new(min_x - 1); // We have an extra column to the left so that differentials have something to hit.
        classes.push(BiVec::new(min_y));
        Self {
            p,
            min_x,
            min_y,
            name,
            sender,
            block_refresh : 0,

            page_list : vec![2],
            product_name_to_index : HashMap::new(),
            products : Arc::new(RwLock::new(Vec::new())),
            classes,
            class_names : BiVec::new(min_x),
            permanent_classes : BiVec::new(min_x),
            differentials : BiVec::new(min_x),
            page_classes : Arc::new(RwLock::new(BiVec::new(min_x))),
            zeros : Arc::new(RwLock::new(BiVec::new(min_x)))
        }
    }

    /// This clears out all the user actions. This is intended to be used when we undo, where
    /// we clear out all actions then redo the existing actions. Hence we avoid re-allocating
    /// as much as possible because we are likely to need the space anyway
    pub fn clear(&mut self) {
        let mut products = self.products.write().unwrap();
        for prod in products.iter_mut() {
            if prod.user {
                prod.permanent = false;
            }
            prod.differential = None;
        }
        drop(products);
        // We initialize to 0 and add_page so that we send it out too.
        self.page_list = vec![];
        self.add_page(MIN_PAGE);

        let mut zeros = self.zeros.write().unwrap();
        for x in 0 .. self.classes.len() {
            for y in 0 .. self.classes[x].len() {
                self.permanent_classes[x][y].set_to_zero();
                for d in self.differentials[x][y].iter_mut() {
                    d.set_to_zero();
                }
                for zero in zeros[x][y].iter_mut() {
                    zero.set_to_zero();
                }
            }
        }
        drop(zeros);

        self.refresh_all();
    }

    pub fn refresh_all(&mut self) {
        if self.block_refresh > 0 {
            return;
        }
        for x in 0 .. self.classes.len() {
            for y in 0 .. self.classes[x].len() {
                self.compute_classes(x, y, false);
            }
        }
        for x in 0 .. self.classes.len() {
            for y in 0 .. self.classes[x].len() {
                self.compute_edges(x, y);
            }
        }
    }

    /// Adds a page to the page list, which is the list of pages where something changes from the
    /// previous page. This is mainly used by the `add_differential` function.
    fn add_page(&mut self, r : i32) {
        if !self.page_list.contains(&r) {
            self.page_list.push(r);
            self.page_list.sort_unstable();

            self.send(Message {
                 recipients : vec![],
                 sseq : self.name,
                 action : Action::from(SetPageList { page_list : self.page_list.clone() })
            });
        }
    }

    /// Initializes `differentials[x][y][r]`. It sets the differentials of all known permament
    /// classes to 0.
    fn allocate_differential_matrix(&mut self, r : i32, x : i32, y : i32) {
        let source_dim = self.classes[x][y];
        let target_dim = self.classes[x - 1][y + r];
        let p = self.p;
        let mut d = Differential::new(p, source_dim, target_dim);
        for vec in self.permanent_classes[x][y].basis() {
            d.add(vec, None);
        }
        self.differentials[x][y].push(d);
    }

    fn allocate_zeros_subspace(&self, zeros : &mut BiVec<BiVec<BiVec<Subspace>>>, r : i32, x : i32, y : i32) {
        let subspace;
        if r == MIN_PAGE {
            let dim = self.classes[x][y];
            subspace = Subspace::new(self.p, dim + 1, dim);
        } else {
            subspace = zeros[x][y][r - 1].clone();
        }
        zeros[x][y].push(subspace);
    }

    /// Given a class `class` at `(x, y)` and a Product object `product`, compute the product of
    /// the class with the product. Returns the new coordinate of the product as well as the actual
    /// product. The result is None if the product is not yet computed.
    fn multiply(&self, x : i32, y : i32, class : &FpVector, product: &Product) -> Option<(i32, i32, FpVector)> {
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
            matrix.apply(&mut prod, 1, class);
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
    /// There is no good reason why `t` is a mutable borrow instead of an immutable borrow. It
    /// would have been an immutable borrow if we can implicitly cast Option<&mut A> to Option<A>,
    /// but we can't.
    ///
    /// # Return
    ///
    /// We return a pair `(r_, x_, y_, s_, t_)` which is the data of the new differential --- its
    /// page, starting coordinate and source and target vectors. Again, if s * si is permanent,
    /// then t_ is set to None. If there is not enough products computed to calculate the result,
    /// or if s * si is zero, we return None.
    fn leibniz(&self, r : i32, x : i32, y : i32, s : &FpVector, t : Option<&mut FpVector>, si : usize) -> Option<(i32, i32, i32, FpVector, Option<FpVector>)> {
        let product = &self.products.read().unwrap()[si];
        // First compute s * si.
        let (x_, y_, new_source) = self.multiply(x, y, s, product)?;

        if new_source.is_zero() {
            return None;
        }

        if product.permanent {
            if let Some(t_) = t {
                let (_, _, mut new_target) = self.multiply(x - 1, y + r, t_, product)?;
                if product.left && product.x % 2 != 0 {
                    new_target.scale(self.p - 1);
                }
                return Some((r, x_, y_, new_source, Some(new_target)));
            } else {
                return Some((INFINITY, x_, y_, new_source, None));
            }
        }

        if let Some((r_, true, ti)) = product.differential {
            if r_ < r {
                // The original differential from s to t is useless.
                let (_, _, mut new_target) = self.multiply(x, y, s, &self.products.read().unwrap()[ti])?;
                if !product.left && (x - 1) % 2 != 0 {
                    new_target.scale(self.p - 1);
                }
                return Some((r_, x_, y_, new_source, Some(new_target)));
            } else if r_ > r {
                // This is more-or-less the same as the permanent code, except we know t is not
                // permanent (or else it would be handled by the previous case).
                if let Some(t_) = t {
                    let (_, _, mut new_target) = self.multiply(x - 1, y + r, t_, product)?;
                    if product.left && product.x % 2 != 0 {
                        new_target.scale(self.p - 1);
                    }
                    return Some((r, x_, y_, new_source, Some(new_target)));
                }
            } else {
                // This is the sum of the two above.
                let (_, _, mut new_target) = self.multiply(x, y, s, &self.products.read().unwrap()[ti])?;
                if !product.left && (x - 1) % 2 != 0 {
                    new_target.scale(self.p - 1);
                }
                if let Some(t_) = t {
                    let (_, _, mut tmp) = self.multiply(x - 1, y + r, t_, product)?;
                    if product.left && product.x % 2 != 0 {
                        tmp.scale(self.p - 1);
                    }
                    new_target.add(&tmp, 1);
                }

                return Some((r, x_, y_, new_source, Some(new_target)));
            }
        }
        None
    }

    fn compute_edges_inner(x : i32, y : i32, p : u32, name : SseqChoice, sender : Sender, page_classes: Arc<RwLock<BiVec<BiVec<BiVec<(Vec<isize>, Vec<FpVector>)>>>>>, products: Arc<RwLock<Vec<Product>>>, zeros: Arc<RwLock<BiVec<BiVec<BiVec<Subspace>>>>>) {
        let page_classes = page_classes.read().unwrap();
        let products = products.read().unwrap();
        let zeros = zeros.read().unwrap();

        let mut structlines : Vec<ProductItem> = Vec::with_capacity(products.len());
        for mult in products.iter() {
            if !(mult.matrices.len() > x && mult.matrices[x].len() > y) {
                continue;
            }
            let target_dim = page_classes[x + mult.x][y + mult.y][MIN_PAGE].1.len();
            if target_dim == 0 {
                continue;
            }

            if let Some(matrix) = &mult.matrices[x][y] {
                let max_page = max(page_classes[x][y].len(), page_classes[x + mult.x][y + mult.y].len());
                let mut matrices : BiVec<Vec<Vec<u32>>> = BiVec::with_capacity(MIN_PAGE, max_page);

                // E_2 page
                matrices.push(matrix.to_vec());

                // Compute the ones where something changes.
                for r in MIN_PAGE + 1 .. max_page {
                    let source_classes = Sseq::get_page(r, &page_classes[x][y]);
                    let target_classes = Sseq::get_page(r, &page_classes[x + mult.x][y + mult.y]);
                    let target_zeros = Sseq::get_page(r, &zeros[x + mult.x][y + mult.y]);

                    let mut result = Vec::with_capacity(source_classes.1.len());
                    let mut target = FpVector::new(p, target_dim);
                    for vec in &source_classes.1 {
                        matrix.apply(&mut target, 1, vec);
                        result.push(express_basis(&mut target, Some(target_zeros), target_classes));
                        target.set_to_zero();
                    }
                    matrices.push(result);

                    if source_classes.1.len() == 0 {
                        break;
                    }
                }
                structlines.push(ProductItem {
                    name : mult.name.clone(),
                    mult_x : mult.x,
                    mult_y : mult.y,
                    matrices,
                });
            }
        }

        sender.send(Message {
            recipients : vec![],
            sseq : name,
            action : Action::from(SetStructline { x, y, structlines })
        }).unwrap();
    }

    /// Computes products whose source is at (x, y).
    fn compute_edges(&self, x : i32, y : i32) {
        if self.block_refresh > 0 { return; }

        if !self.class_defined(x, y) {
            return;
        }
        if self.classes[x][y] == 0 {
            return;
        }

        if let Some(sender) = &self.sender {
            let sender = sender.clone();
            let page_classes = Arc::clone(&self.page_classes);
            let products = Arc::clone(&self.products);
            let zeros = Arc::clone(&self.zeros);
            let p = self.p;
            let name = self.name;

            Sseq::compute_edges_inner(x, y, p, name, sender, page_classes, products, zeros);
        }
    }

    /// Compute the classes in next page assuming there is no differential coming out of the class
    /// on that page. Returns a basis of the remaining classes together with column_to_pivot_row.
    fn compute_next_page_no_d (p : u32 , old_classes : &(Vec<isize>, Vec<FpVector>), zeros : &Subspace) -> (Vec<isize>, Vec<FpVector>) {
        let source_dim = old_classes.0.len();

        let mut class_list = Vec::new();
        let mut vectors : Vec<FpVector> = Vec::with_capacity(old_classes.1.len());

        for vec in &old_classes.1 {
            let mut result = vec.clone();
            zeros.reduce(&mut result);
            vectors.push(result);
        }

        let mut matrix = Matrix::from_rows(p, vectors);
        let mut pivots = vec![-1; matrix.columns()];
        matrix.row_reduce(&mut pivots);

        for i in 0 .. matrix.rows() {
            if matrix[i].is_zero() {
                break;
            }
            let mut vec = FpVector::new(p, source_dim);
            vec.add(&matrix[i], 1);
            class_list.push(vec);
        }
        (pivots, class_list)
    }

    /// Compute the classes in next page assuming there might be a differential coming out of the
    /// class on that page. Returns a basis of the remaining classes together with
    /// column_to_pivot_row.
    fn compute_next_page_with_d (&self, r : i32, x : i32, y : i32, old_classes : &(Vec<isize>, Vec<FpVector>)) -> ((Vec<isize>, Vec<FpVector>), Vec<Vec<u32>>) {
        let zeros = self.zeros.read().unwrap();
        let page_classes = self.page_classes.read().unwrap();

        let source_zeros = Sseq::get_page(r, &zeros[x][y]);
        let target_zeros = Sseq::get_page(r - 1, &zeros[x - 1][y + r - 1]);
        let d = &self.differentials[x][y][r - 1];

        let source_dim = d.source_dim;
        let target_dim = d.target_dim;

        if target_dim == 0 {
            return (Self::compute_next_page_no_d(self.p, old_classes, source_zeros), vec![Vec::new(); source_dim]);
        }

        let mut class_list = Vec::new();
        let mut vectors : Vec<FpVector> = Vec::with_capacity(old_classes.1.len());

        let mut differentials : Vec<Vec<u32>> = Vec::with_capacity(source_dim);

        let mut dvec = FpVector::new(self.p, target_dim);
        for vec in &old_classes.1 {
            d.evaluate(vec.clone(), &mut dvec);
            target_zeros.reduce(&mut dvec);

            let mut result = FpVector::new(self.p, source_dim + target_dim);
            result.set_slice(0, source_dim);
            result.add(&vec, 1);
            source_zeros.reduce(&mut result);
            result.clear_slice();

            result.set_slice(source_dim, source_dim + target_dim);
            result.shift_add(&dvec, 1);
            result.clear_slice();

            vectors.push(result);
            differentials.push(express_basis(&mut dvec, None, Sseq::get_page(r - 1, &page_classes[x - 1][y + r - 1])));
            dvec.set_to_zero();
        }

        let mut matrix = Matrix::from_rows(self.p, vectors);
        let mut pivots = vec![-1; matrix.columns()];
        matrix.row_reduce_offset(&mut pivots, source_dim);

        let mut first_kernel_row = 0;
        for i in (source_dim .. source_dim + target_dim).rev() {
            if pivots[i] >= 0 {
                first_kernel_row = pivots[i] + 1;
                break;
            }
        }

        matrix.set_slice(first_kernel_row as usize, matrix.rows(), 0, source_dim);
        pivots.truncate(source_dim);
        matrix.row_reduce(&mut pivots);
        for i in 0 .. matrix.rows() {
            if matrix[i].is_zero() {
                break;
            }
            let mut vec = FpVector::new(self.p, source_dim);
            vec.add(&matrix[i], 1);
            class_list.push(vec);
        }
        ((pivots, class_list), differentials)
    }

    /// # Arguments
    ///  * `refresh_edge` - Whether to automatically call compute_edges after computing class. This should
    ///  almost always be yes, unless we are re-computing everything, in which case this
    ///  will result in many duplicate calls of compute_edge.
    fn compute_classes(&mut self, x : i32, y : i32, refresh_edge : bool) {
        if self.block_refresh > 0 { return; }

        if !self.class_defined(x, y) {
            return;
        }

        let source_dim = self.classes[x][y];
        if source_dim == 0 {
            let mut page_classes = self.page_classes.write().unwrap();
            page_classes[x][y] = BiVec::from_vec(MIN_PAGE, vec![(Vec::new(), Vec::new())]);
            return;
        }

        let zeros = self.zeros.read().unwrap();
        let max_page = max(zeros[x][y].len(), self.differentials[x][y].len() + 1);

        let mut classes : BiVec<(Vec<isize>, Vec<FpVector>)> = BiVec::with_capacity(MIN_PAGE, max_page);
        let mut differentials : BiVec<Vec<Vec<u32>>> = BiVec::with_capacity(MIN_PAGE, self.differentials[x][y].len());

        // r = MIN_PAGE
        let mut class_list : Vec<FpVector> = Vec::with_capacity(source_dim);
        for i in 0 .. source_dim {
            let mut vec = FpVector::new(self.p, source_dim);
            vec.set_entry(i, 1);
            class_list.push(vec);
        }
        classes.push(((0..source_dim as isize).collect(), class_list));

        for r in MIN_PAGE + 1 .. max_page {
            if classes[r - 1].1.len() == 0 {
                break;
            }

            // We only have to figure out what gets hit by differentials.
            if self.differentials[x][y].len() < r {
                classes.push(Self::compute_next_page_no_d(self.p, &classes[r - 1], Sseq::get_page(r, &zeros[x][y])));
            } else {
                let result = self.compute_next_page_with_d(r, x, y, &classes[r - 1]);
                classes.push(result.0);
                differentials.push(result.1);
            }
        }

        let mut page_classes = self.page_classes.write().unwrap();
        page_classes[x][y] = classes;
        drop(page_classes);
        self.send_class_data(x, y);
        let page_classes = self.page_classes.read().unwrap();

        let mut true_differentials = Vec::with_capacity(self.differentials[x][y].len() as usize);

        for r in MIN_PAGE .. self.differentials[x][y].len() {
            let d = &mut self.differentials[x][y][r];
            let pairs = d.get_source_target_pairs();
            true_differentials.push(pairs.into_iter()
                .map(|(mut s, mut t)| (express_basis(&mut s, Some(Sseq::get_page(r, &zeros[x][y])), &Sseq::get_page(r, &page_classes[x][y])),
                               express_basis(&mut t, Some(Sseq::get_page(r, &zeros[x - 1][y + r])), &Sseq::get_page(r, &page_classes[x - 1][y + r]))))
                .collect::<Vec<_>>())
        }

        if differentials.len() > 0 {
            self.send(Message {
                recipients : vec![],
                sseq : self.name,
                action : Action::from(SetDifferential { x, y, true_differentials, differentials })
            });
        }

        if refresh_edge {
            self.compute_edges(x, y);
            for prod in self.products.read().unwrap().iter() {
                self.compute_edges(x - prod.x, y - prod.y);
            }
        }
    }

    fn send_class_data(&self, x : i32, y : i32) {
        if self.block_refresh > 0 { return; }

        let mut error = false;
        for r in MIN_PAGE .. self.differentials[x][y].len() {
            error |= self.differentials[x][y][r].error;
        }
        for r in self.get_differentials_hitting(x, y) {
            error |= self.differentials[x + 1][y - r][r].error;
        }

        let state;
        let page_classes = self.page_classes.read().unwrap();
        if error {
            state = ClassState::Error;
        } else if page_classes[x][y].last().unwrap().1.iter().fold(true, |b, c| b && self.permanent_classes[x][y].contains(c)) {
            state = ClassState::Done;
        } else {
            state = ClassState::InProgress;
        }

        let mut decompositions : Vec<(FpVector, String, i32, i32)> = Vec::new();
        for prod in self.products.read().unwrap().iter() {
            if !self.product_defined(x - prod.x, y - prod.y, prod) {
                continue;
            }
            if let Some(matrix) = &prod.matrices[x - prod.x][y - prod.y]  {
                for i in 0 .. matrix.len() {
                    if matrix[i].is_zero() {
                        continue;
                    }
                    decompositions.push((matrix[i].clone(), format!("{} {}", prod.name, self.class_names[x - prod.x][y - prod.y][i]), prod.x, prod.y));
                }
            }
        }

        self.send(Message {
            recipients : vec![],
            sseq : self.name,
            action : Action::from(SetClass {
                x, y, state,
                permanents : self.permanent_classes[x][y].basis().to_vec(),
                class_names : self.class_names[x][y].clone(),
                decompositions : decompositions.clone(),
                classes : page_classes[x][y].iter().map(|x| x.1.clone()).collect::<Vec<Vec<FpVector>>>()
            })
        });
    }

    fn send(&self, msg : Message) {
        if let Some(sender) = &self.sender {
            sender.send(msg).unwrap();
        }
    }
}

// Wrapper functions
impl Sseq {
    fn product_defined(&self, x : i32, y : i32, product : &Product) -> bool {
        if !self.class_defined(x, y) {
            false
        } else if product.matrices.max_degree() < x {
            false
        } else if product.matrices[x].max_degree() < y {
            false
        } else {
            true
        }
    }

    fn class_defined(&self, x : i32, y : i32) -> bool {
        if x < self.min_x || y < self.min_y {
            return false;
        }
        if x > self.classes.max_degree() {
            return false;
        }
        if y > self.classes[x].max_degree() {
            return false;
        }
        true
    }

    fn get_page<T>(r : i32, bivec : &BiVec<T>) -> &T {
        if r >= bivec.len() {
            &bivec[bivec.max_degree()]
        } else {
            &bivec[r]
        }
    }

    /// Get a list of r for which there is a d_r differential hitting (x, y)
    fn get_differentials_hitting(&self, x : i32, y : i32) -> Vec<i32> {
        let max_r = self.zeros.read().unwrap()[x][y].len() - 1; // If there is a d_r hitting us, then zeros will be populated up to r + 1

        (MIN_PAGE .. max_r)
            .filter(|&r| self.differentials[x + 1].max_degree() >= y - r
                    && self.differentials[x + 1][y - r].max_degree() >= r)
            .collect::<Vec<i32>>()
    }
}
// Functions called by SseqManager
impl Sseq {
    /// This function should only be called when everything to the left and bottom of (x, y)
    /// has been defined.
    pub fn set_class(&mut self, x : i32, y : i32, num : usize) {
        if x == self.min_x {
            self.classes[self.min_x - 1].push(0);
        }
        let mut zeros = self.zeros.write().unwrap();
        if x == self.classes.len() {
            self.classes.push(BiVec::new(self.min_y));
            self.class_names.push(BiVec::new(self.min_y));
            self.differentials.push(BiVec::new(self.min_y));
            zeros.push(BiVec::new(self.min_y));
            self.permanent_classes.push(BiVec::new(self.min_y));
            self.page_classes.write().unwrap().push(BiVec::new(self.min_y));
        }

        assert_eq!(self.classes[x].len(), y);
        assert_eq!(self.permanent_classes[x].len(), y);
        self.classes[x].push(num);
        let mut names = Vec::with_capacity(num);
        if num == 1 {
            names.push(format!("x_{{{},{}}}", x, y));
        } else {
            for i in 0 .. num {
                names.push(format!("x_{{{}, {}}}^{{({})}}", x, y, i));
            }
        }
        self.class_names[x].push(names);
        self.permanent_classes[x].push(Subspace::new(self.p, num + 1, num));
        self.differentials[x].push(BiVec::new(MIN_PAGE));
        zeros[x].push(BiVec::new(MIN_PAGE));
        self.page_classes.write().unwrap()[x].push(BiVec::new(MIN_PAGE));

        self.allocate_zeros_subspace(&mut zeros, MIN_PAGE, x, y);
        drop(zeros);
        self.compute_classes(x, y, true);
    }

    pub fn set_class_name(&mut self, x : i32, y : i32, idx : usize, name : String) {
        self.class_names[x][y][idx] = name;
        self.send_class_data(x, y);
        for prod in self.products.read().unwrap().iter() {
            if self.class_defined(x + prod.x, y + prod.y) {
                self.send_class_data(x + prod.x, y+ prod.y);
            }
        }
    }

    /// Add a differential starting at (x, y). This mutates the target by reducing it via
    /// `self.zeros[x - 1][y + r][r]`
    ///
    /// Panics if the target of the differential is not yet defined
    pub fn add_differential(&mut self, r : i32, x : i32, y : i32, source : &FpVector, target : &mut FpVector) {
        assert_eq!(source.dimension(), self.classes[x][y], "length of source vector not equal to dimension of source");
        assert_eq!(target.dimension(), self.classes[x - 1][y + r], "length of target vector not equal to dimension of target");

        // We cannot use extend_with here because of borrowing rules.
        if self.differentials[x][y].len() <= r {
            for r_ in self.differentials[x][y].len() ..= r {
                self.allocate_differential_matrix(r_, x, y);
            }
        }

        let mut zeros = self.zeros.write().unwrap();
        Sseq::get_page(r, &zeros[x - 1][y + r]).reduce(target);

        self.differentials[x][y][r].add(source, Some(&target));
        for i in MIN_PAGE .. r {
            self.differentials[x][y][i].add(source, None)
        }

        if zeros[x - 1][y + r].len() <= r + 1 {
            for r_ in zeros[x - 1][y + r].len() ..= r + 1 {
                self.allocate_zeros_subspace(&mut zeros, r_, x - 1, y + r);
            }
        }

        for r_ in r + 1 .. zeros[x - 1][y + r].len() {
            zeros[x - 1][y + r][r_].add_vector(target);
            if self.class_defined(x, y + r - r_) {
                if self.differentials[x][y + r - r_].len() > r_ {
                    self.differentials[x][y + r - r_][r_].reduce_target(&zeros[x - 1][y + r][r_]);
                }
            }
        }

        let len = zeros[x - 1][y + r].len();
        drop(zeros);

        // add_permanent_class in turn sets the differentials on the targets of the differentials
        // to 0. add_differential_propagate will take care of propagating this.
        self.add_permanent_class(x - 1, y + r, target);

        self.add_page(r);
        self.add_page(r + 1);

        self.compute_classes(x - 1, y + r, true);
        self.compute_classes(x, y, true);

        // self.zeros[r] will be populated if there is a non-zero differential hit on a
        // page <= r - 1. Check if these differentials now hit 0.
        for r_ in r + 1 .. len - 1 {
            self.compute_classes(x, y + r - r_, true);
        }
    }

    /// This function recursively propagates differentials. If this function is called, it will add
    /// the corresponding differential plus all products of index at least product_index. Here we
    /// have to exercise a slight bit of care to ensure we don't set both $p_1 p_2 d$ and $p_2 p_1
    /// d$ when $p_1$, $p_2$ are products and $d$ is the differential. Our strategy is that we
    /// compute $p_2 p_1 d$ if and only if $p_1$ comes earlier in the list of products than $p_2$.
    pub fn add_differential_propagate(&mut self, r : i32, x : i32, y : i32, source : &FpVector, target : &mut Option<FpVector>, product_index : usize) {
        let num_products = self.products.read().unwrap().len();
        if product_index == num_products - 1 {
            match target.as_mut() {
                Some(target_) => self.add_differential(r, x, y, source, target_),
                None => self.add_permanent_class(x, y, source)
            };
        } else if product_index < num_products - 1 {
            self.add_differential_propagate(r, x, y, source, target, product_index + 1);
        }

        // Separate this to new line to make code easier to read.
        let new_d = self.leibniz(r, x, y, source, target.as_mut(), product_index);

        if let Some((r_, x_, y_, source_, mut target_)) = new_d {
            self.add_differential_propagate(r_, x_, y_, &source_, &mut target_, product_index);
        }
    }

    pub fn add_permanent_class(&mut self, x : i32, y : i32, class : &FpVector) {
        self.permanent_classes[x][y].add_vector(class);
        for r in MIN_PAGE .. self.differentials[x][y].len() {
            self.differentials[x][y][r].add(class, None);
        }
        self.compute_classes(x, y, true);
    }

    /// Add a product to the list of products, but don't add any computed product
    pub fn add_product_type(&mut self, name : &String, mult_x : i32, mult_y : i32, left : bool, permanent: bool) {
        let idx = self.product_name_to_index.get(name);

        let mut products = self.products.write().unwrap();

        if let Some(&i) = idx {
            products[i].user = true;
            if permanent && !products[i].permanent {
                products[i].permanent = true;
                drop(products);
                self.repropagate_product(i);
            }
        } else {
            let product = Product {
                name : name.clone(),
                x : mult_x,
                y : mult_y,
                user : true,
                left,
                permanent,
                differential : None,
                matrices : BiVec::new(self.min_x)
            };
            products.push(product);
            self.product_name_to_index.insert(name.clone(), products.len() - 1);
        }
    }

    pub fn add_product_differential(&mut self, source : &String, target: &String) {
        let source_idx = *self.product_name_to_index.get(source).unwrap();
        let target_idx = *self.product_name_to_index.get(target).unwrap();

        let mut products = self.products.write().unwrap();
        let r = products[target_idx].y - products[source_idx].y;

        products[source_idx].differential = Some((r, true, target_idx));
        products[target_idx].differential = Some((r, false, source_idx));

        drop(products);
        self.repropagate_product(source_idx);
    }

    fn repropagate_product(&mut self, idx : usize) {
        let max_x = self.products.read().unwrap()[idx].matrices.len();
        for x in self.min_x .. max_x {
            let max_y = self.products.read().unwrap()[idx].matrices[x].len();
            for y in self.min_y .. max_y {
                for r in MIN_PAGE .. self.differentials[x][y].len() {
                    let d = &mut self.differentials[x][y][r];
                    for (source, mut target) in d.get_source_target_pairs() {
                        let new_d = self.leibniz(r, x, y, &source, Some(&mut target), idx);
                        if let Some((r_, x_, y_, source_, Some(mut target_))) = new_d {
                            self.add_differential(r_, x_, y_, &source_, &mut target_);
                        }
                    }
                }

                // Find a better way to do this. This is to circumevent borrow checker.
                let classes = self.permanent_classes[x][y].basis().to_vec();
                for class in classes {
                    let new_d = self.leibniz(INFINITY, x, y, &class, None, idx);
                    if let Some((r_, x_, y_, source_, t_)) = new_d {
                        match t_ {
                            Some(mut target_) => self.add_differential(r_, x_, y_, &source_, &mut target_),
                            None => self.add_permanent_class(x_, y_, &source_)
                        };
                    }
                }
            }
        }
    }

    pub fn add_product(&mut self, name : &String, x : i32, y : i32, mult_x : i32, mult_y : i32, left : bool, matrix : &Vec<Vec<u32>>) {
        assert!(self.class_defined(x, y));
        assert!(self.class_defined(x + mult_x, y + mult_y));
        let mut products = self.products.write().unwrap();
        let idx : usize =
            match self.product_name_to_index.get(name) {
                Some(i) => *i,
                None => {
                    let product = Product {
                        name : name.clone(),
                        x : mult_x,
                        y : mult_y,
                        user : false,
                        left,
                        permanent : true,
                        differential : None,
                        matrices : BiVec::new(self.min_x)
                    };
                    products.push(product);
                    self.product_name_to_index.insert(name.clone(), products.len() - 1);
                    products.len() - 1
                }
            };
        while x > products[idx].matrices.len() {
            products[idx].matrices.push(BiVec::new(self.min_y));
        }
        if x == products[idx].matrices.len() {
            products[idx].matrices.push(BiVec::new(self.min_y));
        }
        while y > products[idx].matrices[x].len() {
            products[idx].matrices[x].push(None);
        }

        assert_eq!(y, products[idx].matrices[x].len());
        products[idx].matrices[x].push(Some(Matrix::from_vec(self.p, matrix)));

        drop(products);

        // We propagate all differentials that *hit* us, because of the order in which products
        // are added. The exception is if this product is the target of a product
        // differential on page r, we propagate the d_r differential *starting* at (x, y).
        if self.products.read().unwrap()[idx].differential.is_some() && !self.products.read().unwrap()[idx].differential.unwrap().1 {
            let (r, _ , si) = self.products.read().unwrap()[idx].differential.unwrap();
            if self.differentials[x][y].len() > r {
                let d = &mut self.differentials[x][y][r];
                for (source, mut target) in d.get_source_target_pairs() {
                    let new_d = self.leibniz(r, x, y, &source, Some(&mut target), si);
                    if let Some((r_, x_, y_, source_, Some(mut target_))) = new_d {
                        self.add_differential(r_, x_, y_, &source_, &mut target_);
                    }
                }
            }

            let classes = self.permanent_classes[x][y].basis().to_vec();
            for class in classes {
                let new_d = self.leibniz(INFINITY, x, y, &class, None, si);
                if let Some((r_, x_, y_, source_, Some(mut target_))) = new_d {
                    self.add_differential(r_, x_, y_, &source_, &mut target_);
                }
            }
        } else {
            for r in self.get_differentials_hitting(x, y) {
                let d = &mut self.differentials[x + 1][y - r][r];
                for (source, mut target) in d.get_source_target_pairs() {
                    let new_d = self.leibniz(r, x + 1, y - r, &source, Some(&mut target), idx);
                    if let Some((r_, x_, y_, source_, Some(mut target_))) = new_d {
                        self.add_differential(r_, x_, y_, &source_, &mut target_);
                    }
                }
            }

            // Find a better way to do this. This is to circumevent borrow checker.
            let classes = self.permanent_classes[x][y].basis().to_vec();
            for class in classes {
                let new_d = self.leibniz(INFINITY, x, y, &class, None, idx);
                if let Some((r_, x_, y_, source_, t_)) = new_d {
                    match t_ {
                        Some(mut target_) => self.add_differential(r_, x_, y_, &source_, &mut target_),
                        None => self.add_permanent_class(x_, y_, &source_)
                    };
                }
            }
        }
        self.compute_edges(x, y);
        self.send_class_data(x + mult_x, y + mult_y);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sseq_differential() {
        let p = 3;
        rust_ext::fp_vector::initialize_limb_bit_index_table(p);
        let mut sseq = crate::sseq::Sseq::new(p, SseqChoice::Main, 0, 0, None);
        sseq.set_class(0, 0, 1);
        sseq.set_class(1, 0, 2);
        sseq.set_class(1, 1, 2);
        sseq.set_class(0, 1, 0);
        sseq.set_class(0, 2, 3);
        sseq.set_class(0, 3, 1);

        sseq.add_differential(2, 1, 0,
                              &FpVector::from_vec(p, &vec![1, 1]),
                              &mut FpVector::from_vec(p, &vec![0, 1, 2]));

        sseq.add_differential(3, 1, 0,
                              &FpVector::from_vec(p, &vec![1, 0]),
                              &mut FpVector::from_vec(p, &vec![1]));


        let page_classes = sseq.page_classes.read().unwrap();
        assert_eq!(page_classes[1][0].max_degree(), 4);
        assert_eq!(page_classes[1][0][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(page_classes[1][0][3].1, vec![FpVector::from_vec(p, &vec![1, 0])]);
        assert_eq!(page_classes[1][0][4].1, vec![]);

        assert_eq!(page_classes[1][1].max_degree(), 2);
        assert_eq!(page_classes[1][1][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(page_classes[0][2].max_degree(), 3);
        assert_eq!(page_classes[0][2][2].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(page_classes[0][2][3].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                 FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(page_classes[0][3].max_degree(), 4);
        assert_eq!(page_classes[0][3][2].1, vec![FpVector::from_vec(p, &vec![1])]);
        assert_eq!(page_classes[0][3][3].1, vec![FpVector::from_vec(p, &vec![1])]);
        assert_eq!(page_classes[0][3][4].1, vec![]);

        drop(page_classes);
        sseq.add_differential(2, 1, 1,
                              &FpVector::from_vec(p, &vec![1, 0]),
                              &mut FpVector::from_vec(p, &vec![1]));

        let page_classes = sseq.page_classes.read().unwrap();
        assert_eq!(page_classes[1][0].max_degree(), 4);
        assert_eq!(page_classes[1][0][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(page_classes[1][0][3].1, vec![FpVector::from_vec(p, &vec![1, 0])]);
        assert_eq!(page_classes[1][0][4].1, vec![FpVector::from_vec(p, &vec![1, 0])]);

        assert_eq!(page_classes[1][1].max_degree(), 3);
        assert_eq!(page_classes[1][1][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(page_classes[1][1][3].1, vec![FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(page_classes[0][2].max_degree(), 3);
        assert_eq!(page_classes[0][2][2].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(page_classes[0][2][3].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                 FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(page_classes[0][3].max_degree(), 3);
        assert_eq!(page_classes[0][3][2].1, vec![FpVector::from_vec(p, &vec![1])]);
        assert_eq!(page_classes[0][3][3].1, vec![]);
    }

    #[test]
    fn test_sseq_differential_2() {
        let p = 2;
        rust_ext::fp_vector::initialize_limb_bit_index_table(p);
        let mut sseq = crate::sseq::Sseq::new(p, SseqChoice::Main, 0, 0, None);

        sseq.set_class(0, 0, 0);
        sseq.set_class(1, 0, 2);
        sseq.set_class(0, 1, 0);
        sseq.set_class(0, 2, 2);

        sseq.add_differential(2, 1, 0,
                              &FpVector::from_vec(p, &vec![1, 0]),
                              &mut FpVector::from_vec(p, &vec![1, 0]));
        sseq.add_differential(2, 1, 0,
                              &FpVector::from_vec(p, &vec![0, 1]),
                              &mut FpVector::from_vec(p, &vec![1, 1]));

        let page_classes = sseq.page_classes.read().unwrap();
        assert_eq!(page_classes[1][0].max_degree(), 3);
        assert_eq!(page_classes[1][0][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1])]);
        assert_eq!(page_classes[1][0][3].1.len(), 0);

        assert_eq!(page_classes[0][2].max_degree(), 3);
        assert_eq!(page_classes[0][2][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                 FpVector::from_vec(p, &vec![0, 1])]);
        assert_eq!(page_classes[0][2][3].1.len(), 0);
    }
}
