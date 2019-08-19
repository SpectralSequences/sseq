use rust_ext::matrix::{Subspace, Matrix};
use rust_ext::fp_vector::{FpVector, FpVectorT};
use std::collections::HashMap;
use std::cmp::max;
use std::sync::mpsc;
use serde::{Serialize, Deserialize};
use bivec::BiVec;
use crate::actions::*;

const MIN_PAGE : i32 = 2;

/// Given a vector `elt`, a subspace `zeros` of the total space (with a specified choice of
/// complement) and a basis `basis` of a subspace of the complement, project `elt` to the complement and express
/// as a linear combination of the basis. This assumes the projection of `elt` is indeed in the
/// span of `basis`. The result is returned as a list of coefficients.
///
/// If `zeros` is none, then the initial projection is not performed.
fn express_basis(mut elt : FpVector, zeros : Option<&Subspace>, basis : &(Vec<isize>, Vec<FpVector>)) -> Vec<u32>{
    if let Some(z) = zeros {
        z.reduce(&mut elt);
    }
    let mut result = Vec::with_capacity(basis.0.len());
    for i in 0 .. basis.0.len() {
        if basis.0[i] < 0 {
            continue;
        }
        let c = elt.get_entry(i);
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
        assert_eq!(zeros.matrix.get_columns(), self.target_dim);

        self.matrix.set_slice(0, self.matrix.get_rows(), self.source_dim, self.source_dim + self.target_dim);
        for i in 0 .. self.matrix.get_rows() {
            zeros.shift_reduce(&mut self.matrix[i]);
        }
        self.matrix.clear_slice();
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

            let c = source.get_entry(i);
            if c == 0 {
                continue;
            }
            for j in 0 .. self.target_dim {
                target.add_basis_element(j, c * self.matrix[row].get_entry(self.source_dim + j));
            }
            for j in 0 .. self.source_dim {
                source.add_basis_element(j, (self.prime() - 1) * c * self.matrix[row].get_entry(j));
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
pub struct Sseq {
    pub p : u32,
    name : SseqChoice,
    min_x : i32,
    min_y : i32,

    sender : Option<mpsc::Sender<Message>>,
    page_list : Vec<i32>,
    product_name_to_index : HashMap<String, usize>,
    products : Vec<Product>,
    classes : BiVec<BiVec<usize>>, // x -> y -> number of elements
    differentials : BiVec<BiVec<BiVec<Differential>>>, // x -> y -> r -> differential
    permanent_classes : BiVec<BiVec<Subspace>>, // x -> y -> r -> permanent classes
    zeros : BiVec<BiVec<BiVec<Subspace>>>, // x -> y -> r -> subspace of elements that are zero on page r
    page_classes : BiVec<BiVec<BiVec<(Vec<isize>, Vec<FpVector>)>>>, // x -> y -> r -> list of generators on the page.
}

impl Sseq {
    pub fn new(p : u32, name : SseqChoice, min_x : i32, min_y : i32, sender : Option<mpsc::Sender<Message>>) -> Self {
        let mut classes = BiVec::new(min_x - 1); // We have an extra column to the left so that differentials have something to hit.
        classes.push(BiVec::new(min_y));
        Self {
            p,
            min_x,
            min_y,
            name,
            sender,

            page_list : vec![2],
            product_name_to_index : HashMap::new(),
            products : Vec::new(),
            classes,
            permanent_classes : BiVec::new(min_x),
            differentials : BiVec::new(min_x),
            page_classes : BiVec::new(min_x),
            zeros : BiVec::new(min_x)
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

    fn add_zeros(&mut self, r : i32, x : i32, y : i32, target : &FpVector) {
        for r_ in r .. self.zeros[x][y].len() {
            self.zeros[x][y][r_].add_vector(target);
            if self.class_defined(x + 1, y - r_) {
                if self.differentials[x + 1][y - r_].len() > r_ {
                    self.differentials[x + 1][y - r_][r_].reduce_target(&self.zeros[x][y][r_]);
                }
            }
        }
    }

    /// Initializes `differentials[x][y][r]`. It sets the differentials of all known permament
    /// classes to 0.
    fn allocate_differential_matrix(&mut self, r : i32, x : i32, y : i32) {
        let source_dim = self.classes[x][y];
        let target_dim = self.classes[x - 1][y + r];
        let p = self.p;
        let mut d = Differential::new(p, source_dim, target_dim);
        for vec in self.permanent_classes[x][y].get_basis() {
            d.add(vec, None);
        }
        self.differentials[x][y].push(d);
    }

    fn allocate_zeros_subspace(&mut self, r : i32, x : i32, y : i32) {
        let subspace;
        if r == MIN_PAGE {
            let dim = self.classes[x][y];
            subspace = Subspace::new(self.p, dim + 1, dim);
        } else {
            subspace = self.zeros[x][y][r - 1].clone();
        }
        self.zeros[x][y].push(subspace);
    }

    /// Given a class `class` at `(x, y)` and a Product object `product`, compute the product of
    /// the class with the product. Returns the new coordinate of the product as well as the actual
    /// product. The result is None if the product is not defined.
    fn multiply(&self, x : i32, y : i32, class : &FpVector, product: &Product) -> Option<(i32, i32, FpVector)> {
        if product.matrices.max_degree() < x {
            return None;
        }
        if product.matrices[x].max_degree() < y {
            return None;
        }

        if let Some(matrix) = &product.matrices[x][y] {
            let prod_x = product.x;
            let prod_y = product.y;

            let mut prod = FpVector::new(self.p, self.classes[x + prod_x][y + prod_y]);
            matrix.apply(&mut prod, 1, class);
            return Some((x + prod_x, y + prod_y, prod));
        }
        None
    }

    /// Given a page `r` and coordinates `x`, `y`, a differerntial from `source` to `target`
    /// starting at (x, y), and a product `product`, return the result of propagating the
    /// differential along the product. If the product is not defined or the product with the
    /// source is zero, this returns None. Otherwise, it returns `Some((new_x, new_y, new_source,
    /// new_target))`, where `new_x/new_y` are the coordinates of the source of the new
    /// differential.
    fn product_differential(&self, r : i32, x : i32, y : i32, source : &FpVector, target : &FpVector, product : &Product) -> Option<(i32, i32, FpVector, FpVector)>{
        // Handle non-zero product differential.
        if !product.permanent {
            return None;
        }

        if let Some((new_x, new_y, prod_source)) = self.multiply(x, y, source, product) {
            if prod_source.is_zero() {
                return None;
            }
            if let Some((_, _, mut prod_target)) = self.multiply(x - 1, y + r, target, product) {
                if product.left && product.x % 2 == 1 {
                    prod_target.scale(self.p - 1);
                }

                return Some((new_x, new_y, prod_source, prod_target));
            }
        }
        None
    }

    /// This function is called by add_product, and propagates products with differentials
    /// accordingly. The input (x, y) is the coordinate of the point we want to take the product
    /// with.
    ///
    /// Observe that if we have a differential $d(a) = b$, then for any $z$, to compute $d(az)$, we
    /// need to know the products $a d(z)$ and $bz$, and of course $az$ as well. We are able to
    /// compute $d(az)$ if and only if we have computed all these three values. By our assumption,
    /// $az$ is always computed before $a d(z)$. On the other hand, there are no guarantees on the
    /// order in which different products come in.
    ///
    /// Thus this function should be called whenever bz and a dz is calculated.
    ///
    /// The return type is Option<()> so that we can use the question mark operator. In practice,
    /// the return value will be None if the differential was not successfully calculated because
    /// the products have not yet been defined.
    fn attempt_propagate_product_differential(&mut self, r: i32, x : i32, y : i32, source_idx : usize, target_idx : usize) -> Option<()> {
        if !self.class_defined(x, y) {
            return None;
        }

        if self.differentials[x][y].len() > r {
            // Work with the differentials
            let pairs = self.differentials[x][y][r].get_source_target_pairs();
            for (s, t) in pairs {
                // If this fails, its is because some multiplication isn't defined,
                // in which case we should abort. Then it is also not defined for
                // all other classes.
                self.leibniz(r, x, y, &s, Some(&t), source_idx, target_idx)?;
            }
        } else {
            // Propagate permanent cycles.
            let classes = self.permanent_classes[x][y].get_basis().to_vec();

            for class in classes {
                self.leibniz(r, x, y, &class, None, source_idx, target_idx)?;
            }
        }
        Some(())
    }

    /// Apply Leibniz's rule to the d_r differential starting at (x, y) from `s` to `t and
    /// another d_r differential from `products[si]` to `products[ti]`. Set `t` to be None if
    /// the differential is zero.
    fn leibniz(&mut self, r : i32, x : i32, y : i32, s : &FpVector, t : Option<&FpVector>, si : usize, ti : usize) -> Option<()> {
        let (x_, y_, a_s) = self.multiply(x, y, s, &self.products[si])?;
        if a_s.is_zero() {
            return Some(());
        }

        let (_, _, mut bs) = self.multiply(x, y, s, &self.products[ti])?;
        if let Some(t_) = t {
            let (_, _, mut at) = self.multiply(x - 1, y + r, t_, &self.products[si])?;
            if self.products[si].x % 2 != 0 {
                at.scale(self.p - 1);
            }
            bs.add(&at, 1);
        }
        self.add_differential(r, x_, y_, &a_s, &mut bs);

        Some(())
    }

    /// Computes products whose source is at (x, y).
    fn compute_edges(&self, x : i32, y : i32) {
        if !self.class_defined(x, y) {
            return;
        }
        if self.classes[x][y] == 0 {
            return;
        }

        let mut global_max_page = self.page_classes[x][y].len();
        for mult in &self.products {
            if self.class_defined(x + mult.x, y + mult.y) {
                global_max_page = max(global_max_page, self.page_classes[x + mult.x][y + mult.y].len());
            }
        }

        let mut structlines : Vec<ProductItem> = Vec::with_capacity(self.products.len());
        for mult in &self.products {
            if !(mult.matrices.len() > x && mult.matrices[x].len() > y) {
                continue;
            }
            let target_dim = self.classes[x + mult.x][y + mult.y];
            if target_dim == 0 {
                continue;
            }

            if let Some(matrix) = &mult.matrices[x][y] {
                let max_page = max(self.page_classes[x][y].len(), self.page_classes[x + mult.x][y + mult.y].len());
                let mut matrices : BiVec<Vec<Vec<u32>>> = BiVec::with_capacity(MIN_PAGE, max_page);

                // E_2 page
                matrices.push(matrix.to_vec());

                // Compute the ones where something changes.
                for r in MIN_PAGE + 1 .. max_page {
                    let source_classes = self.get_page_classes(r, x, y);
                    if source_classes.1.len() == 0 {
                        break;
                    }
                    let target_classes = self.get_page_classes(r, x + mult.x, y + mult.y);
                    let target_zeros = self.get_page_zeros(r, x + mult.x, y + mult.y);

                    let mut result = Vec::with_capacity(source_classes.1.len());
                    for vec in &source_classes.1 {
                        let mut target = FpVector::new(self.p, target_dim);
                        matrix.apply(&mut target, 1, vec);
                        result.push(express_basis(target, Some(target_zeros), target_classes));
                    }
                    matrices.push(result);
                }
                while matrices.len() < global_max_page {
                    matrices.push(matrices.last().unwrap().clone());
                }
                assert_eq!(matrices.len(), global_max_page);

                structlines.push(ProductItem {
                    name : mult.name.clone(),
                    mult_x : mult.x,
                    mult_y : mult.y,
                    matrices,
                });
            }
        }

        self.send(Message {
            recipients : vec![],
            sseq : self.name,
            action : Action::from(SetStructline { x, y, structlines })
        });
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
        let mut pivots = vec![-1; matrix.get_columns()];
        matrix.row_reduce(&mut pivots);

        for i in 0 .. matrix.get_rows() {
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
        let source_zeros = self.get_page_zeros(r, x, y);
        let target_zeros = self.get_page_zeros(r - 1, x - 1, y + r - 1);
        let d = &self.differentials[x][y][r - 1];

        let source_dim = d.source_dim;
        let target_dim = d.target_dim;

        if target_dim == 0 {
            return (Self::compute_next_page_no_d(self.p, old_classes, source_zeros), vec![Vec::new(); source_dim]);
        }

        let mut class_list = Vec::new();
        let mut vectors : Vec<FpVector> = Vec::with_capacity(old_classes.1.len());

        let mut differentials : Vec<Vec<u32>> = Vec::with_capacity(source_dim);

        for vec in &old_classes.1 {
            let mut dvec = FpVector::new(self.p, target_dim);
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
            differentials.push(express_basis(dvec, None, self.get_page_classes(r - 1, x - 1, y + r - 1)));
        }

        let mut matrix = Matrix::from_rows(self.p, vectors);
        let mut pivots = vec![-1; matrix.get_columns()];
        matrix.row_reduce_offset(&mut pivots, source_dim);

        let mut first_kernel_row = 0;
        for i in (source_dim .. source_dim + target_dim).rev() {
            if pivots[i] >= 0 {
                first_kernel_row = pivots[i] + 1;
            }
        }

        matrix.set_slice(first_kernel_row as usize, matrix.get_rows(), 0, source_dim);
        pivots.truncate(source_dim);
        matrix.row_reduce(&mut pivots);
        for i in 0 .. matrix.get_rows() {
            if matrix[i].is_zero() {
                break;
            }
            let mut vec = FpVector::new(self.p, source_dim);
            vec.add(&matrix[i], 1);
            class_list.push(vec);
        }
        ((pivots, class_list), differentials)
    }

    fn compute_classes(&mut self, x : i32, y : i32) {
        if !self.class_defined(x, y) {
            return;
        }

        let source_dim = self.classes[x][y];
        if source_dim == 0 {
            self.page_classes[x][y] = BiVec::from_vec(MIN_PAGE, vec![(Vec::new(), Vec::new())]);
            return;
        }

        let max_page = max(self.zeros[x][y].len(), self.differentials[x][y].len() + 1);

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
                classes.push(Self::compute_next_page_no_d(self.p, &classes[r - 1], self.get_page_zeros(r, x, y)));
            } else {
                let result = self.compute_next_page_with_d(r, x, y, &classes[r - 1]);
                classes.push(result.0);
                differentials.push(result.1);
            }
        }

        self.page_classes[x][y] = classes;

        self.send_class_data(x, y);

        let mut true_differentials = Vec::with_capacity(self.differentials[x][y].len() as usize);

        for r in MIN_PAGE .. self.differentials[x][y].len() {
            let d = &mut self.differentials[x][y][r];
            let pairs = d.get_source_target_pairs();
            true_differentials.push(pairs.into_iter()
                .map(|(s, t)| (express_basis(s, Some(self.get_page_zeros(r, x, y)), &self.get_page_classes(r, x, y)),
                               express_basis(t, Some(self.get_page_zeros(r, x - 1, y + r)), &self.get_page_classes(r, x - 1, y + r))))
                .collect::<Vec<_>>())
        }

        if differentials.len() > 0 {
            self.send(Message {
                recipients : vec![],
                sseq : self.name,
                action : Action::from(SetDifferential { x, y, true_differentials, differentials })
            });
        }

        self.compute_edges(x, y);
        for prod in &self.products {
            self.compute_edges(x - prod.x, y - prod.y);
        }
    }

    fn send_class_data(&self, x : i32, y : i32) {
        let mut error = false;
        for r in MIN_PAGE .. self.differentials[x][y].len() {
            error |= self.differentials[x][y][r].error;
        }
        for r in self.get_differentials_hitting(x, y) {
            error |= self.differentials[x + 1][y - r][r].error;
        }

        let state;
        if error {
            state = ClassState::Error;
        } else if self.page_classes[x][y].last().unwrap().1.iter().fold(true, |b, c| b && self.permanent_classes[x][y].contains(c)) {
            state = ClassState::Done;
        } else {
            state = ClassState::InProgress;
        }

        self.send(Message {
            recipients : vec![],
            sseq : self.name,
            action : Action::from(SetClass {
                x, y, state,
                permanents : self.permanent_classes[x][y].get_basis().to_vec(),
                classes : self.page_classes[x][y].iter().map(|x| x.1.clone()).collect::<Vec<Vec<FpVector>>>()
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

    fn get_page_zeros(&self, r : i32, x : i32, y : i32) -> &Subspace {
        if r >= self.zeros[x][y].len() {
            &self.zeros[x][y][self.zeros[x][y].max_degree()]
        } else {
            &self.zeros[x][y][r]
        }
    }

    fn get_page_classes(&self, r : i32, x : i32, y : i32) -> &(Vec<isize>, Vec<FpVector>) {
        assert!(self.page_classes[x][y].len() > MIN_PAGE, "No classes defined at ({}, {})", x, y);
        if r >= self.page_classes[x][y].len() {
            &self.page_classes[x][y][self.page_classes[x][y].max_degree()]
        } else {
            &self.page_classes[x][y][r]
        }
    }

    /// Get a list of r for which there is a d_r differential hitting (x, y)
    fn get_differentials_hitting(&self, x : i32, y : i32) -> Vec<i32> {
        let max_r = self.zeros[x][y].len() - 1; // If there is a d_r hitting us, then zeros will be populated up to r + 1

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
        while x > self.classes.len() {
            self.set_class(self.classes.len(), self.min_y - 1, 0);
        }
        if x == self.classes.len() {
            self.classes.push(BiVec::new(self.min_y));
            self.differentials.push(BiVec::new(self.min_y));
            self.zeros.push(BiVec::new(self.min_y));
            self.permanent_classes.push(BiVec::new(self.min_y));
            self.page_classes.push(BiVec::new(self.min_y));
        }

        if y < self.min_y {
            return; // This happens when we are padding as above
        }

        while y > self.classes[x].len() {
            self.set_class(x, self.classes[x].len(), 0);
        }

        assert_eq!(self.classes[x].len(), y);
        assert_eq!(self.permanent_classes[x].len(), y);
        self.classes[x].push(num);
        self.permanent_classes[x].push(Subspace::new(self.p, num + 1, num));
        self.differentials[x].push(BiVec::new(MIN_PAGE));
        self.zeros[x].push(BiVec::new(MIN_PAGE));
        self.page_classes[x].push(BiVec::new(MIN_PAGE));

        self.allocate_zeros_subspace(MIN_PAGE, x, y);
        self.compute_classes(x, y);
    }

    /// Add a differential starting at (x, y). This mutates the target by reducing it via
    /// `self.zeros[x - 1][y + r][r]`
    ///
    /// Panics if the target of the differential is not yet defined
    pub fn add_differential(&mut self, r : i32, x : i32, y : i32, source : &FpVector, target : &mut FpVector) {
        assert_eq!(source.get_dimension(), self.classes[x][y], "length of source vector not equal to dimension of source");
        assert_eq!(target.get_dimension(), self.classes[x - 1][y + r], "length of target vector not equal to dimension of target");

        // We cannot use extend_with here because of borrowing rules.
        if self.differentials[x][y].len() <= r {
            for r_ in self.differentials[x][y].len() ..= r {
                self.allocate_differential_matrix(r_, x, y);
            }
        }

        self.get_page_zeros(r, x - 1, y + r).reduce(target);

        self.differentials[x][y][r].add(source, Some(&target));
        for i in MIN_PAGE .. r {
            self.differentials[x][y][i].add(source, None)
        }

        if self.zeros[x - 1][y + r].len() <= r + 1 {
            for r_ in self.zeros[x - 1][y + r].len() ..= r + 1 {
                self.allocate_zeros_subspace(r_, x - 1, y + r);
            }
        }

        self.add_zeros(r + 1, x - 1, y + r, target);
        // add_permanent_class in turn sets the differentials on the targets of the differentials
        // to 0.
        self.add_permanent_class(x - 1, y + r, target);

        self.add_page(r);
        self.add_page(r + 1);

        for i in 0 .. self.products.len() {
            if let Some((r_, is_source, i_)) = self.products[i].differential {
                if !is_source {
                    continue;
                }
                if r_ == r {
                    self.leibniz(r_, x, y, source, Some(target), i, i_);
                } else if r_ < r {
                    self.leibniz(r_, x, y, source, None, i, i_);
                }
            }
        }

        self.compute_classes(x - 1, y + r);
        self.compute_classes(x, y);

        // self.zeros[r] will be populated if there is a non-zero differential hit on a
        // page <= r - 1. Check if these differentials now hit 0.
        for r_ in r + 1 .. self.zeros[x - 1][y + r].len() - 1 {
            self.compute_classes(x, y + r - r_);
        }
    }

    /// This function recursively propagates differentials. If this function is called, it will add
    /// the corresponding differential plus all products of index at least product_index. Here we
    /// have to exercise a slight bit of care to ensure we don't set both $p_1 p_2 d$ and $p_2 p_1
    /// d$ when $p_1$, $p_2$ are products and $d$ is the differential. Our strategy is that we
    /// compute $p_2 p_1 d$ if and only if $p_1$ comes earlier in the list of products than $p_2$.
    pub fn add_differential_propagate(&mut self, r : i32, x : i32, y : i32, source : &FpVector, target : &mut FpVector, product_index : usize) {
        if product_index == self.products.len() - 1 {
            self.add_differential(r, x, y, source, target);
        } else if product_index < self.products.len() - 1 {
            self.add_differential_propagate(r, x, y, source, target, product_index + 1);
        }

        let new_d = self.product_differential(r, x, y, source, target, &self.products[product_index]);

        if let Some((new_x, new_y, new_source, mut new_target)) = new_d {
            self.add_differential_propagate(r, new_x, new_y, &new_source, &mut new_target, product_index);
        }
    }


    pub fn add_permanent_class(&mut self, x : i32, y : i32, class : &FpVector) {
        self.permanent_classes[x][y].add_vector(class);
        if self.differentials.len() <= x {
            return;
        }
        if self.differentials[x].len() <= y {
            return;
        }
        for r in MIN_PAGE .. self.differentials[x][y].len() {
            self.differentials[x][y][r].add(class, None);
        }

        for i in 0 .. self.products.len() {
            if let Some((r, is_source, i_)) = self.products[i].differential {
                if !is_source {
                    continue;
                }

                if let Some((x_, y_, a)) = self.multiply(x, y, class, &self.products[i]) {
                    if a.is_zero() {
                        continue;
                    }
                    if let Some((_, _, mut b)) = self.multiply(x, y, class, &self.products[i_]) {
                        self.add_differential(r, x_, y_, &a, &mut b);
                    }
                }
            }
        }

        self.send_class_data(x, y);
    }

    /// Same logic as add_differential_propagate
    pub fn add_permanent_class_propagate(&mut self, x : i32, y : i32, class : &FpVector, product_index : usize) {
        if product_index == self.products.len() - 1 {
            self.add_permanent_class(x, y, class);
        } else if product_index < self.products.len() - 1 {
            self.add_permanent_class_propagate(x, y, class, product_index + 1);
        }

        if self.products[product_index].permanent {
            if let Some((x_, y_, prod)) = self.multiply(x, y, class, &self.products[product_index]) {
                if !prod.is_zero() {
                    self.add_permanent_class_propagate(x_, y_, &prod, product_index);
                }
            }
        }
    }

    /// Add a product to the list of products, but don't add any computed product
    pub fn add_product_type(&mut self, name : &String, mult_x : i32, mult_y : i32, left : bool, permanent: bool) {
        if !self.product_name_to_index.contains_key(name) {
            let product = Product {
                name : name.clone(),
                x : mult_x,
                y : mult_y,
                left,
                permanent,
                differential : None,
                matrices : BiVec::new(self.min_x)
            };
            self.products.push(product);
            self.product_name_to_index.insert(name.clone(), self.products.len() - 1);
        }
    }

    pub fn add_product_differential(&mut self, source : &String, target: &String) {
        let source_idx = *self.product_name_to_index.get(source).unwrap();
        let target_idx = *self.product_name_to_index.get(target).unwrap();

        let r = self.products[target_idx].y - self.products[source_idx].y;

        self.products[source_idx].differential = Some((r, true, target_idx));
        self.products[target_idx].differential = Some((r, false, source_idx));
    }

    pub fn add_product(&mut self, name : &String, x : i32, y : i32, mult_x : i32, mult_y : i32, left : bool, matrix : &Vec<Vec<u32>>) {
        if !self.class_defined(x, y) {
            return;
        }
        if !self.class_defined(x + mult_x, y + mult_y) {
            return;
        }

        let idx : usize =
            match self.product_name_to_index.get(name) {
                Some(i) => *i,
                None => {
                    let product = Product {
                        name : name.clone(),
                        x : mult_x,
                        y : mult_y,
                        left,
                        permanent : true,
                        differential : None,
                        matrices : BiVec::new(self.min_x)
                    };
                    self.products.push(product);
                    self.product_name_to_index.insert(name.clone(), self.products.len() - 1);
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

        self.products[idx].matrices[x].push(Some(Matrix::from_vec(self.p, matrix)));

        // Now propagate differentials. We propagate differentials that *hit* us, because the
        // target product is always set after the source product.

        if self.products[idx].permanent {
            for r in self.get_differentials_hitting(x, y) {
                let d = &mut self.differentials[x + 1][y - r][r];
                for (source, target) in d.get_source_target_pairs() {
                    let new_d = self.product_differential(r, x + 1, y - r, &source, &target, &self.products[idx]);
                    if let Some((x_, y_, source_, mut target_)) = new_d {
                        self.add_differential(r, x_, y_, &source_, &mut target_);
                    }
                }
            }

            // Find a better way to do this. This is to circumevent borrow checker.
            let classes = self.permanent_classes[x][y].get_basis().to_vec();
            for class in classes {
                if let Some((x_, y_, product)) = self.multiply(x, y, &class,  &self.products[idx]) {
                    self.add_permanent_class(x_, y_, &product);
                }
            }
        }

        // Now propagate differentials in products. See documentation of
        // attempt_propagate_product_differential for details.
        if let Some((r, is_source, other_idx)) = self.products[idx].differential {
            if is_source {
                self.attempt_propagate_product_differential(r, x + 1, y - r, idx, other_idx);
            } else {
                self.attempt_propagate_product_differential(r, x, y, other_idx, idx);
            }
        }
        self.compute_edges(x, y);
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


        assert_eq!(sseq.page_classes[1][0].max_degree(), 4);
        assert_eq!(sseq.page_classes[1][0][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                    FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(sseq.page_classes[1][0][3].1, vec![FpVector::from_vec(p, &vec![1, 0])]);
        assert_eq!(sseq.page_classes[1][0][4].1, vec![]);

        assert_eq!(sseq.page_classes[1][1].max_degree(), 2);
        assert_eq!(sseq.page_classes[1][1][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                    FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(sseq.page_classes[0][2].max_degree(), 3);
        assert_eq!(sseq.page_classes[0][2][2].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                    FpVector::from_vec(p, &vec![0, 1, 0]),
                                                    FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(sseq.page_classes[0][2][3].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                    FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(sseq.page_classes[0][3].max_degree(), 4);
        assert_eq!(sseq.page_classes[0][3][2].1, vec![FpVector::from_vec(p, &vec![1])]);
        assert_eq!(sseq.page_classes[0][3][3].1, vec![FpVector::from_vec(p, &vec![1])]);
        assert_eq!(sseq.page_classes[0][3][4].1, vec![]);

//        assert_eq!(sseq.page_differentials[1][0].max_degree(), 3);
//        assert_eq!(sseq.page_differentials[1][0][2], vec![FpVector::from_vec(p, &vec![0, 0, 0]),
//                                                          FpVector::from_vec(p, &vec![0, 1, 2])]);

//        assert_eq!(sseq.page_differentials[1][0][3], vec![FpVector::from_vec(p, &vec![1])]);

//        assert_eq!(sseq.page_differentials[1][1].max_degree(), 1);


        sseq.add_differential(2, 1, 1,
                              &FpVector::from_vec(p, &vec![1, 0]),
                              &mut FpVector::from_vec(p, &vec![1]));

        assert_eq!(sseq.page_classes[1][0].max_degree(), 4);
        assert_eq!(sseq.page_classes[1][0][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                    FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(sseq.page_classes[1][0][3].1, vec![FpVector::from_vec(p, &vec![1, 0])]);
        assert_eq!(sseq.page_classes[1][0][4].1, vec![FpVector::from_vec(p, &vec![1, 0])]);

        assert_eq!(sseq.page_classes[1][1].max_degree(), 3);
        assert_eq!(sseq.page_classes[1][1][2].1, vec![FpVector::from_vec(p, &vec![1, 0]),
                                                    FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(sseq.page_classes[1][1][3].1, vec![FpVector::from_vec(p, &vec![0, 1])]);

        assert_eq!(sseq.page_classes[0][2].max_degree(), 3);
        assert_eq!(sseq.page_classes[0][2][2].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                    FpVector::from_vec(p, &vec![0, 1, 0]),
                                                    FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(sseq.page_classes[0][2][3].1, vec![FpVector::from_vec(p, &vec![1, 0, 0]),
                                                    FpVector::from_vec(p, &vec![0, 0, 1])]);

        assert_eq!(sseq.page_classes[0][3].max_degree(), 3);
        assert_eq!(sseq.page_classes[0][3][2].1, vec![FpVector::from_vec(p, &vec![1])]);
        assert_eq!(sseq.page_classes[0][3][3].1, vec![]);

//        assert_eq!(sseq.page_differentials[1][0].max_degree(), 3);
//        assert_eq!(sseq.page_differentials[1][0][2], vec![FpVector::from_vec(p, &vec![0, 0, 0]),
//                                                          FpVector::from_vec(p, &vec![0, 1, 2])]);

//        assert_eq!(sseq.page_differentials[1][0][3], vec![FpVector::from_vec(p, &vec![0])]);

//        assert_eq!(sseq.page_differentials[1][1].max_degree(), 2);
//        assert_eq!(sseq.page_differentials[1][1][2], vec![FpVector::from_vec(p, &vec![1]),
//                                                          FpVector::from_vec(p, &vec![0])]);
    }
}
