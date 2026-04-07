use std::{cmp::max, collections::BTreeMap};

use bivec::BiVec;
use fp::{
    matrix::{Matrix, Subquotient},
    prime::ValidPrime,
    vector::{FpSlice, FpVector},
};
use once::MultiIndexed;
use serde::{Deserialize, Serialize};
use sseq::{
    Adams, Sseq, SseqProfile,
    coordinates::{Bidegree, BidegreeElement},
};

use crate::{Sender, actions::*};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClassState {
    Error,
    Done,
    InProgress,
}

/// # Fields
///  * `matrices[x][y]` : This encodes the matrix of the product. If it is None, it means the
///    target of the product has dimension 0.
pub struct Product {
    inner: sseq::Product<2>,
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
    mult_b: Bidegree,
    matrices: BiVec<Vec<Vec<u32>>>, // page -> matrix
}

const CLASS_FLAG: u8 = 1;
const EDGE_FLAG: u8 = 2;

/// Here are some blanket assumptions we make about the order in which we add things.
///  * If we add a class at (x, y), then all classes to the left and below of (x, y) have been
///    computed. Moreover, every class at (x + 1, y - r) for r >= 1 have been computed. If these have
///    not been set, the class is assumed to be zero.
///  * The same is true for products, where the grading of a product is that of its source.
///  * Whenever a product v . x is set, the target is already set.
pub struct SseqWrapper<P: SseqProfile<2> = Adams> {
    pub p: ValidPrime,
    name: SseqChoice,
    pub inner: Sseq<2, P>,

    /// Whether a bidegree is stale, i.e.\ new products have to be reported to the sender. Note
    /// that products "belong" to the source of the product.
    stale: MultiIndexed<2, u8>,

    /// If this is a positive number, then the spectral sequence will not re-compute classes and
    /// edges. See [`Actions::BlockRefresh`] for details.
    pub block_refresh: u32,
    sender: Option<Sender>,
    products: BTreeMap<String, Product>,
    /// bidegree -> idx -> name
    class_names: MultiIndexed<2, Vec<String>>,
}

impl<P: SseqProfile<2>> SseqWrapper<P> {
    pub fn new(p: ValidPrime, name: SseqChoice, sender: Option<Sender>) -> Self {
        Self {
            p,
            name,
            sender,
            block_refresh: 0,
            inner: Sseq::new(p),

            products: BTreeMap::default(),
            class_names: MultiIndexed::new(),
            stale: MultiIndexed::new(),
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

        // Collect invalid bidegrees first: update_bidegree needs &mut self.inner, so we can't hold
        // an iterator across those calls.
        let invalid: Vec<_> = self
            .inner
            .iter_degrees()
            .filter(|&b| self.inner.invalid(b))
            .collect();
        for b in invalid {
            self.stale[b] |= CLASS_FLAG | EDGE_FLAG;
            for product in self.products.values() {
                let prod_origin_b = b - product.inner.b;
                if let Some(flags) = self.stale.get_mut(prod_origin_b) {
                    *flags |= EDGE_FLAG;
                }
            }
            let differentials = self.inner.update_degree(b);
            if !differentials.is_empty() {
                // `true_differentials` is a list of differentials of the form d(source) = target we
                // know to be true. `differentials` is our best guess at what the matrix of
                // differentials is.
                let true_differentials = self
                    .inner
                    .differentials(b)
                    .iter_enum()
                    .map(|(r, d)| {
                        let target_b = P::profile(r, b);
                        d.get_source_target_pairs()
                            .into_iter()
                            .map(|(mut s, mut t)| {
                                (
                                    self.inner.page_data(b)[r].reduce(s.as_slice_mut()),
                                    self.inner.page_data(target_b)[r].reduce(t.as_slice_mut()),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                self.send(Message {
                    recipients: vec![],
                    sseq: self.name,
                    action: Action::from(SetDifferential {
                        b,
                        true_differentials,
                        differentials,
                    }),
                });
            }
        }

        let stale_bidegrees: Vec<Bidegree> = self
            .stale
            .iter()
            .filter(|(_, flags)| **flags != 0)
            .map(|(b, _)| b.into())
            .collect();
        for b in stale_bidegrees {
            let flags = self.stale.get(b).copied().unwrap_or(0);
            if flags & CLASS_FLAG > 0 {
                self.send_class_data(b);
            }
            if flags & EDGE_FLAG > 0 {
                self.send_products(b);
            }
            self.stale[b] = 0;
        }
    }

    /// Computes products whose source is at `b`.
    fn send_products(&self, b: Bidegree) {
        if !self.inner.defined(b) {
            return;
        }
        if self.inner.dimension(b) == 0 {
            return;
        }

        let mut structlines: Vec<ProductItem> = Vec::with_capacity(self.products.len());
        for (name, mult) in &self.products {
            let prod_b = mult.inner.b;
            let prod_output_b = b + prod_b;

            let Some(matrix) = mult.inner.matrices.get(b) else {
                continue;
            };

            let target_dim = self.inner.dimension(prod_output_b);
            if target_dim == 0 {
                continue;
            }

            let max_page = max(
                self.inner.page_data(b).len(),
                self.inner.page_data(prod_output_b).len(),
            );
            let mut matrices: BiVec<Vec<Vec<u32>>> = BiVec::with_capacity(P::MIN_R, max_page);

            // E_2 page
            matrices.push(matrix.to_vec());

            // Compute the ones where something changes.
            for r in P::MIN_R + 1..max_page {
                let source_data = self.inner.page_data(b).get_max(r);
                let target_data = self.inner.page_data(prod_output_b).get_max(r);

                matrices.push(Subquotient::reduce_matrix(matrix, source_data, target_data));

                // In the case where the source is empty, we still want one empty array to
                // indicate that no structlines should be drawn from this page on.
                if source_data.is_empty() {
                    break;
                }
            }

            structlines.push(ProductItem {
                name: name.clone(),
                mult_b: prod_b,
                matrices,
            });
        }

        self.send(Message {
            recipients: vec![],
            sseq: self.name,
            action: Action::from(SetStructline { b, structlines }),
        });
    }

    fn send_class_data(&self, b: Bidegree) {
        if self.block_refresh > 0 {
            return;
        }

        let state = if self.inner.inconsistent(b) {
            ClassState::Error
        } else if self.inner.complete(b) {
            ClassState::Done
        } else {
            ClassState::InProgress
        };

        // `SetClass` carries vectors as `Vec<u32>` so the JS frontend can read them as plain arrays
        // (see the comment on `SetClass`). Convert here rather than relying on `FpVector`'s
        // `Serialize` impl.
        let mut decompositions: Vec<(Vec<u32>, String, Bidegree)> = Vec::new();
        for (name, prod) in &self.products {
            let prod_b = prod.inner.b;
            let prod_origin_b = b - prod_b;

            if let Some(matrix) = prod.inner.matrices.get(prod_origin_b) {
                for i in 0..matrix.rows() {
                    if matrix.row(i).is_zero() {
                        continue;
                    }
                    decompositions.push((
                        matrix.row(i).iter().collect(),
                        format!("{name} {}", self.class_names[prod_origin_b][i]),
                        prod_b,
                    ));
                }
            }
        }

        self.send(Message {
            recipients: vec![],
            sseq: self.name,
            action: Action::from(SetClass {
                b,
                state,
                permanents: self
                    .inner
                    .permanent_classes(b)
                    .basis()
                    .map(|row| row.iter().collect())
                    .collect(),
                class_names: self.class_names[b].clone(),
                decompositions,
                classes: self
                    .inner
                    .page_data(b)
                    .iter()
                    .map(|x| x.gens().map(|row| row.iter().collect()).collect())
                    .collect(),
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
impl<P: SseqProfile<2>> SseqWrapper<P> {
    /// This function should only be called when everything to the left and bottom of (x, y)
    /// has been defined.
    pub fn set_dimension(&mut self, b: Bidegree, dim: usize) {
        self.inner.set_dimension(b, dim);
        let mut names = Vec::with_capacity(dim);
        if dim == 1 {
            names.push(format!("x_{{{x},{y}}}", x = b.x(), y = b.y()));
        } else {
            names.extend(
                (0..dim).map(|i| format!("x_{{{x}, {y}}}^{{({i})}}", x = b.x(), y = b.y())),
            );
        }
        self.class_names.insert(b, names);
        self.stale.insert(b, CLASS_FLAG);
    }

    pub fn set_class_name(&mut self, b: Bidegree, idx: usize, name: String) {
        self.class_names[b][idx] = name;
        self.send_class_data(b);
        for prod in self.products.values() {
            let prod_output_b = b + prod.inner.b;
            if self.inner.defined(prod_output_b) {
                self.send_class_data(prod_output_b);
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
        source: &BidegreeElement,
        product_index: usize,
    ) {
        if self.products.is_empty() {
            return;
        }
        // This is useful for batch adding differentials from external sources, where not all
        // classes have been added.
        if !self.inner.defined(source.degree()) {
            return;
        }
        if r != i32::MAX {
            let target_b = P::profile(r, source.degree());
            if !self.inner.defined(target_b) {
                return;
            }
        }

        if product_index + 1 < self.products.len() {
            self.add_differential_propagate(r, source, product_index + 1);
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
        let new_d = self.inner.leibniz(r, source, &product.inner, target);

        if let Some((r, source)) = new_d {
            self.add_differential_propagate(r, &source, product_index);
        }
    }

    /// Add a product to the list of products, but don't add any computed product
    pub fn add_product_type(&mut self, name: &str, mult_b: Bidegree, left: bool, permanent: bool) {
        if let Some(product) = self.products.get_mut(name) {
            product.user = true;
            if permanent && !product.permanent {
                product.permanent = true;
                self.propagate_product_all(name);
            }
        } else {
            let product = Product {
                inner: sseq::Product {
                    b: mult_b,
                    left,
                    matrices: MultiIndexed::new(),
                },
                user: true,
                permanent,
                differential: None,
            };
            self.products.insert(name.to_string(), product);
        }
    }

    pub fn add_product_differential(&mut self, source: &str, target: &str) {
        let offset = self.products[target].inner.b - self.products[source].inner.b;
        let r = P::differential_length(offset);

        self.products.get_mut(source).unwrap().differential = Some((r, true, target.to_owned()));
        self.products.get_mut(target).unwrap().differential = Some((r, false, source.to_owned()));

        self.propagate_product_all(source);
    }

    /// Propagate products by the product indexed by `idx`.
    fn propagate_product_all(&mut self, name: &str) {
        let bidegrees: Vec<Bidegree> = self.products[name]
            .inner
            .matrices
            .iter()
            .map(|(b, _)| b.into())
            .collect();
        for b in bidegrees {
            self.propagate_product(b, name);
        }
    }

    /// Propagate products by the product indexed by `idx` at `b`. The product must either be
    /// permanent or the source of a differential.
    fn propagate_product(&mut self, b: Bidegree, name: &str) {
        let product = &self.products[name];
        let target = if product.permanent {
            None
        } else if let Some((_, true, target_name)) = &product.differential {
            Some(&self.products[target_name].inner)
        } else {
            return;
        };

        for r in self.inner.differentials(b).range() {
            let pairs = self.inner.differentials(b)[r].get_source_target_pairs();
            for (source, _) in pairs {
                self.inner
                    .leibniz(r, &BidegreeElement::new(b, source), &product.inner, target);
            }
        }

        let permanent_classes = self
            .inner
            .permanent_classes(b)
            .basis()
            .map(FpSlice::to_owned)
            .collect::<Vec<_>>();
        for class in permanent_classes {
            self.inner.leibniz(
                i32::MAX,
                &BidegreeElement::new(b, class),
                &product.inner,
                target,
            );
        }
    }

    pub fn add_product(
        &mut self,
        name: &str,
        b: Bidegree,
        mult_b: Bidegree,
        left: bool,
        matrix: &[Vec<u32>],
    ) {
        let prod_output_b = b + mult_b;
        assert!(self.inner.defined(b));
        assert!(self.inner.defined(prod_output_b));

        if !self.products.contains_key(name) {
            let product = Product {
                inner: sseq::Product {
                    b: mult_b,
                    left,
                    matrices: MultiIndexed::new(),
                },
                user: false,
                permanent: true,
                differential: None,
            };
            self.products.insert(name.to_string(), product);
        };

        let product = self.products.get_mut(name).unwrap();
        let matrix = Matrix::from_vec(self.p, matrix);

        if self.inner.dimension(b) != 0 && self.inner.dimension(prod_output_b) != 0 {
            self.stale[b] |= EDGE_FLAG;
            if !matrix.is_zero() {
                self.stale[prod_output_b] |= CLASS_FLAG;
            }
        }

        product.inner.matrices.insert(b, matrix);

        let product = &*product;

        // To propagate a differential on along d(α) = β, we need to compute the α product on the
        // source and target, and the β product on the source.
        if let Some((_, false, source_name)) = &product.differential {
            let source_name = source_name.clone();
            self.propagate_product(b, &source_name);
        } else if matches!(product.differential, Some((_, true, _))) || product.permanent {
            self.propagate_product(b, name);
            let hitting: Vec<i32> = self
                .inner
                .differentials_hitting(b)
                .map(|(r, _)| r)
                .collect();
            for r in hitting {
                let source_b = P::profile_inverse(r, b);
                if self.inner.defined(source_b) {
                    self.propagate_product(source_b, name);
                }
            }
        }
    }
}
