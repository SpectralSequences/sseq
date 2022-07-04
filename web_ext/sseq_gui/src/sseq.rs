use crate::actions::*;
use crate::Sender;
use bivec::BiVec;
use fp::prime::ValidPrime;
use fp::vector::FpVector;
use fp::{
    matrix::{Matrix, Subquotient},
    vector::{prelude::*, Slice},
};
use serde::{Deserialize, Serialize};
use sseq::{Adams, Sseq, SseqProfile};
use std::cmp::max;
use std::collections::BTreeMap;

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
    inner: sseq::Product,
    /// whether the product was specified by the user or the module. Products specified by the module are assumed to be permanent
    user: bool,
    /// whether the product class is a permanent class
    permanent: bool,
    /// The first entry is the page of the differential. The second index is true if this is the source of the differential. The last index is the name of the other end of the differential.
    differential: Option<(i32, bool, String)>,
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
    products: BTreeMap<String, Product>,
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
        Self {
            p,
            name,
            sender,
            block_refresh: 0,
            inner: Sseq::new(p, min_x, min_y),

            products: BTreeMap::default(),
            class_names: BiVec::new(min_x),
            stale: BiVec::new(min_x),
        }
    }

    /// This clears out all the user actions. This is intended to be used when we undo, where
    /// we clear out all actions then redo the existing actions. Hence we avoid re-allocating
    /// as much as possible because we are likely to need the space anyway
    pub fn clear(&mut self) {
        for prod in self.products.values_mut() {
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
                for product in self.products.values() {
                    let prod_x = product.inner.x;
                    let prod_y = product.inner.y;
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

    /// Computes products whose source is at (x, y).
    fn send_products(&self, x: i32, y: i32) {
        if !self.inner.defined(x, y) {
            return;
        }
        if self.inner.dimension(x, y) == 0 {
            return;
        }

        let mut structlines: Vec<ProductItem> = Vec::with_capacity(self.products.len());
        for (name, mult) in &self.products {
            if !(mult.inner.matrices.len() > x && mult.inner.matrices[x].len() > y) {
                continue;
            }
            let prod_x = mult.inner.x;
            let prod_y = mult.inner.y;

            let target_dim = self.inner.dimension(x + prod_x, y + prod_y);
            if target_dim == 0 {
                continue;
            }

            if let Some(matrix) = &mult.inner.matrices[x][y] {
                let max_page = max(
                    self.inner.page_data(x, y).len(),
                    self.inner.page_data(x + prod_x, y + prod_y).len(),
                );
                let mut matrices: BiVec<Vec<Vec<u32>>> = BiVec::with_capacity(P::MIN_R, max_page);

                // E_2 page
                matrices.push(matrix.to_vec());

                // Compute the ones where something changes.
                for r in P::MIN_R + 1..max_page {
                    let source_data = self.inner.page_data(x, y).get_max(r);
                    let target_data = self.inner.page_data(x + prod_x, y + prod_y).get_max(r);

                    matrices.push(Subquotient::reduce_matrix(matrix, source_data, target_data));

                    // In the case where the source is empty, we still want one empty array to
                    // indicate that no structlines should be drawn from this page on.
                    if source_data.is_empty() {
                        break;
                    }
                }

                structlines.push(ProductItem {
                    name: name.clone(),
                    mult_x: prod_x,
                    mult_y: prod_y,
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
        for (name, prod) in &self.products {
            let prod_x = prod.inner.x;
            let prod_y = prod.inner.y;

            if let Some(Some(Some(matrix))) = &prod
                .inner
                .matrices
                .get(x - prod_x)
                .map(|m| m.get(y - prod_y))
            {
                for i in 0..matrix.rows() {
                    if matrix[i].is_zero() {
                        continue;
                    }
                    decompositions.push((
                        matrix[i].clone(),
                        format!("{name} {}", self.class_names[x - prod_x][y - prod_y][i]),
                        prod_x,
                        prod_y,
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
                    .map(|x| x.gens().map(Slice::into_owned).collect())
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
        for prod in self.products.values() {
            if self.inner.defined(x + prod.inner.x, y + prod.inner.y) {
                self.send_class_data(x + prod.inner.x, y + prod.inner.y);
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
        product_index: usize,
    ) {
        if self.products.is_empty() {
            return;
        }
        // This is useful for batch adding differentials from external sources, where not all
        // classes have been added.
        if !self.inner.defined(x, y) {
            return;
        }
        if r != i32::MAX {
            let (tx, ty) = P::profile(r, x, y);
            if !self.inner.defined(tx, ty) {
                return;
            }
        }

        if product_index + 1 < self.products.len() {
            self.add_differential_propagate(r, x, y, source, product_index + 1);
        }

        let product = self.products.values().nth(product_index).unwrap();
        let target = if product.permanent {
            None
        } else if let Some((_, true, target_name)) = &product.differential {
            Some(&self.products[target_name].inner)
        } else {
            return;
        };

        // Separate this to new line to make code easier to read.
        let new_d = self.inner.leibniz(r, x, y, source, &product.inner, target);

        if let Some((r, x, y, source)) = new_d {
            self.add_differential_propagate(r, x, y, source.as_slice(), product_index);
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
        if let Some(product) = self.products.get_mut(name) {
            product.user = true;
            if permanent && !product.permanent {
                product.permanent = true;
                self.propagate_product_all(name);
            }
        } else {
            let product = Product {
                inner: sseq::Product {
                    x: mult_x,
                    y: mult_y,
                    left,
                    matrices: BiVec::new(self.inner.min_x()),
                },
                user: true,
                permanent,
                differential: None,
            };
            self.products.insert(name.to_string(), product);
        }
    }

    pub fn add_product_differential(&mut self, source: &str, target: &str) {
        let r = P::differential_length(
            self.products[target].inner.x - self.products[source].inner.x,
            self.products[target].inner.y - self.products[source].inner.y,
        );

        self.products.get_mut(source).unwrap().differential = Some((r, true, target.to_owned()));
        self.products.get_mut(target).unwrap().differential = Some((r, false, source.to_owned()));

        self.propagate_product_all(source);
    }

    /// Propagate products by the product indexed by `idx`.
    fn propagate_product_all(&mut self, name: &str) {
        // We only use this to figure out the range
        for x in self.products[name].inner.matrices.range() {
            for y in self.products[name].inner.matrices[x].range() {
                self.propagate_product(x, y, name);
            }
        }
    }

    /// Propagate products by the product indexed by `idx` at (x, y). The product must either be
    /// permanent or the source of a differential.
    fn propagate_product(&mut self, x: i32, y: i32, name: &str) {
        let product = &self.products[name];
        let target = if product.permanent {
            None
        } else if let Some((_, true, target_name)) = &product.differential {
            Some(&self.products[target_name].inner)
        } else {
            return;
        };

        for r in self.inner.differentials(x, y).range() {
            let pairs = self.inner.differentials(x, y)[r].get_source_target_pairs();
            for (source, _) in pairs {
                self.inner
                    .leibniz(r, x, y, source.as_slice(), &product.inner, target);
            }
        }

        let permanent_classes = self.inner.permanent_classes(x, y).basis().to_vec();
        for class in permanent_classes {
            self.inner
                .leibniz(i32::MAX, x, y, class.as_slice(), &product.inner, target);
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
        if !self.products.contains_key(name) {
            let product = Product {
                inner: sseq::Product {
                    x: mult_x,
                    y: mult_y,
                    left,
                    matrices: BiVec::new(self.inner.min_x()),
                },
                user: false,
                permanent: true,
                differential: None,
            };
            self.products.insert(name.to_string(), product);
        };

        let product = self.products.get_mut(name).unwrap();
        product
            .inner
            .matrices
            .extend_with(x, |_| BiVec::new(self.inner.min_y()));
        product.inner.matrices[x].extend_with(y - 1, |_| None);

        let matrix = Matrix::from_vec(self.p, matrix);

        if self.inner.dimension(x, y) != 0 && self.inner.dimension(x + mult_x, y + mult_y) != 0 {
            self.stale[x][y] |= EDGE_FLAG;
            if !matrix.is_zero() {
                self.stale[x + mult_x][y + mult_y] |= CLASS_FLAG;
            }
        }

        assert_eq!(y, product.inner.matrices[x].len());
        product.inner.matrices[x].push(Some(matrix));

        let product = &*product;

        // To propagate a differential on along d(α) = β, we need to compute the α product on the
        // source and target, and the β product on the source.
        if let Some((_, false, source_name)) = &product.differential {
            let source_name = source_name.clone();
            self.propagate_product(x, y, &source_name);
        } else if matches!(product.differential, Some((_, true, _))) || product.permanent {
            self.propagate_product(x, y, name);
            let hitting: Vec<i32> = self
                .inner
                .differentials_hitting(x, y)
                .map(|(r, _)| r)
                .collect();
            for r in hitting {
                let (sx, sy) = P::profile_inverse(r, x, y);
                if self.inner.defined(sx, sy) {
                    self.propagate_product(sx, sy, name);
                }
            }
        }
    }
}
