use parking_lot::RwLock;
use rustc_hash::FxHashSet as HashSet;
use serde_json::Value;
use std::cmp::min;
use std::sync::{Arc, Weak};
// use std::time::Instant;

use algebra::module::homomorphism::FreeModuleHomomorphism;
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use ext::chain_complex::{ChainComplex, UnitChainComplex};
use ext::resolution::Resolution as ResolutionInner;
use ext::resolution_homomorphism::{ResolutionHomomorphism, ResolutionHomomorphismToUnit};
use fp::matrix::Matrix;
use fp::prime::ValidPrime;
use once::{OnceBiVec, OnceVec};

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

/// Hack to compare two pointers of different types (in this case because they might have different
/// type parameters.
fn ptr_eq<T, S>(a: &Arc<T>, b: &Arc<S>) -> bool {
    let a = Arc::into_raw(Arc::clone(a));
    let b = Arc::into_raw(Arc::clone(b)) as *const T;
    let eq = std::ptr::eq(a, b);
    unsafe {
        let _ = Arc::from_raw(a);
        let _ = Arc::from_raw(b as *const S);
    }
    eq
}

#[derive(Clone)]
struct Cocycle {
    s: u32,
    t: i32,
    class: Vec<u32>,
    name: String,
}

pub struct SelfMap<CC: ChainComplex> {
    pub s: u32,
    pub t: i32,
    pub name: String,
    pub map_data: Matrix,
    pub map: ResolutionHomomorphism<ResolutionInner<CC>, ResolutionInner<CC>>,
}

pub type AddClassFn = Box<dyn Fn(u32, i32, usize)>;
pub type AddStructlineFn = Box<dyn Fn(&str, u32, i32, u32, i32, bool, Vec<Vec<u32>>)>;

pub struct Resolution<CC: UnitChainComplex> {
    pub inner: Arc<ResolutionInner<CC>>,

    pub add_class: Option<AddClassFn>,
    pub add_structline: Option<AddStructlineFn>,

    filtration_one_products: Vec<(String, i32, usize)>,

    // Products
    pub unit_resolution: Option<Weak<RwLock<Resolution<CC>>>>,
    pub unit_resolution_owner: Option<Arc<RwLock<Resolution<CC>>>>,
    product_names: HashSet<String>,
    product_list: Vec<Cocycle>,
    // s -> t -> idx -> resolution homomorphism to unit resolution. We don't populate this
    // until we actually have a unit resolution, of course.
    chain_maps_to_unit_resolution: OnceVec<OnceBiVec<OnceVec<ResolutionHomomorphismToUnit<CC>>>>,
    max_product_homological_degree: u32,

    // Self maps
    pub self_maps: Vec<SelfMap<CC>>,
}

impl Resolution<ext::CCC> {
    pub fn new_from_json(mut json: Value, algebra_name: &str) -> Self {
        let inner = ext::utils::construct_from_json(&mut json, algebra_name).unwrap();
        let mut result = Self::new_with_inner(inner);
        let products_value = &mut json["products"];
        if !products_value.is_null() {
            let products = products_value.as_array_mut().unwrap();
            for prod in products {
                let hom_deg = prod["hom_deg"].as_u64().unwrap() as u32;
                let int_deg = prod["int_deg"].as_i64().unwrap() as i32;
                let class: Vec<u32> = serde_json::from_value(prod["class"].take()).unwrap();
                let name = prod["name"].as_str().unwrap();

                result.add_product(hom_deg, int_deg, class, &name.to_string());
            }
        }

        let self_maps = &json["self_maps"];

        if !self_maps.is_null() {
            for self_map in self_maps.as_array().unwrap() {
                let s = self_map["hom_deg"].as_u64().unwrap() as u32;
                let t = self_map["int_deg"].as_i64().unwrap() as i32;
                let name = self_map["name"].as_str().unwrap();

                let json_map_data = self_map["map_data"].as_array().unwrap();
                let json_map_data: Vec<&Vec<Value>> = json_map_data
                    .iter()
                    .map(|x| x.as_array().unwrap())
                    .collect();

                let rows = json_map_data.len();
                let cols = json_map_data[0].len();
                let mut map_data = Matrix::new(result.prime(), rows, cols);
                for r in 0..rows {
                    for c in 0..cols {
                        map_data[r].set_entry(c, json_map_data[r][c].as_u64().unwrap() as u32);
                    }
                }
                result.add_self_map(s, t, &name.to_string(), map_data);
            }
        }

        result
    }
}

impl<CC: UnitChainComplex> Resolution<CC> {
    pub fn new_with_inner(inner: ResolutionInner<CC>) -> Self {
        let inner = Arc::new(inner);
        let algebra = inner.complex().algebra();

        Self {
            inner,

            add_class: None,
            add_structline: None,

            filtration_one_products: algebra.default_filtration_one_products(),

            chain_maps_to_unit_resolution: OnceVec::new(),
            max_product_homological_degree: 0,
            product_names: HashSet::default(),
            product_list: Vec::new(),
            unit_resolution: None,
            unit_resolution_owner: None,

            self_maps: Vec::new(),
        }
    }

    #[cfg(feature = "concurrent")]
    pub fn resolve_through_bidegree_concurrent(&self, s: u32, t: i32, bucket: &TokenBucket) {
        if let Some(unit_res) = &self.unit_resolution {
            let unit_res = unit_res.upgrade().unwrap();
            let unit_res = unit_res.read();
            // Avoid a deadlock
            if !ptr_eq(&unit_res.inner, &self.inner) {
                unit_res.resolve_through_bidegree_concurrent(
                    self.max_product_homological_degree,
                    t - self.min_degree(),
                    bucket,
                );
            }
        }

        self.inner
            .resolve_through_bidegree_concurrent_with_callback(s, t, bucket, |s, t| {
                self.step_after(s, t)
            });
    }

    pub fn resolve_through_bidegree(&self, s: u32, t: i32) {
        if let Some(unit_res) = &self.unit_resolution {
            let unit_res = unit_res.upgrade().unwrap();
            let unit_res = unit_res.read();
            // Avoid a deadlock
            if !ptr_eq(&unit_res.inner, &self.inner) {
                unit_res.resolve_through_bidegree(
                    self.max_product_homological_degree,
                    t - self.min_degree(),
                );
            }
        }

        self.inner
            .resolve_through_bidegree_with_callback(s, t, |s, t| self.step_after(s, t));
    }

    #[cfg(feature = "concurrent")]
    pub fn resolve_through_degree_concurrent(&self, degree: i32, bucket: &TokenBucket) {
        self.resolve_through_bidegree_concurrent(degree as u32, degree, bucket);
    }

    pub fn resolve_through_degree(&self, degree: i32) {
        self.resolve_through_bidegree(degree as u32, degree);
    }

    fn step_after(&self, s: u32, t: i32) {
        if t - (s as i32) < self.min_degree() {
            return;
        }
        let module = self.module(s);
        let num_gens = module.number_of_gens_in_degree(t);
        if let Some(f) = &self.add_class {
            f(s, t, num_gens);
        }
        self.compute_filtration_one_products(s, t);
        self.construct_maps_to_unit(s, t);
        self.extend_maps_to_unit(s, t);
        self.compute_products(s, t, &self.product_list);
        self.compute_self_maps(s, t);
    }

    #[allow(clippy::needless_range_loop)]
    fn compute_filtration_one_products(&self, target_s: u32, target_t: i32) {
        if target_s == 0 {
            return;
        }
        let source_s = target_s - 1;

        let source = self.module(source_s);
        let target = self.module(target_s);

        let target_dim = target.number_of_gens_in_degree(target_t);

        for (op_name, op_degree, op_index) in &self.filtration_one_products {
            let source_t = target_t - *op_degree;
            if source_t - (source_s as i32) < self.min_degree() {
                continue;
            }
            let source_dim = source.number_of_gens_in_degree(source_t);

            let d = self.differential(target_s);

            let mut products = vec![Vec::with_capacity(target_dim); source_dim];

            for i in 0..target_dim {
                let dx = d.output(target_t, i);

                for j in 0..source_dim {
                    let idx =
                        source.operation_generator_to_index(*op_degree, *op_index, source_t, j);
                    products[j].push(dx.entry(idx));
                }
            }

            self.add_structline(
                op_name, source_s, source_t, target_s, target_t, true, products,
            );
        }
    }

    pub fn add_structline(
        &self,
        name: &str,
        source_s: u32,
        source_t: i32,
        target_s: u32,
        target_t: i32,
        left: bool,
        products: Vec<Vec<u32>>,
    ) {
        if let Some(add_structline) = &self.add_structline {
            add_structline(name, source_s, source_t, target_s, target_t, left, products);
        }
    }

    pub fn max_computed_degree(&self) -> i32 {
        self.inner.max_computed_degree()
    }

    pub fn max_computed_homological_degree(&self) -> u32 {
        self.inner.max_computed_homological_degree()
    }

    pub fn graded_dimension_vec(&self) -> Vec<Vec<usize>> {
        let min_degree = self.min_degree();
        let max_degree = self.max_computed_degree();
        let max_hom_deg = self.max_computed_homological_degree();
        let mut result = Vec::with_capacity(max_hom_deg as usize + 1);
        for i in (0..=max_hom_deg).rev() {
            let module = self.module(i);
            result.push(
                (min_degree + i as i32..=max_degree)
                    .map(|j| module.number_of_gens_in_degree(j))
                    .collect::<Vec<_>>(),
            );
        }
        result
    }

    pub fn complex(&self) -> Arc<CC> {
        self.inner.complex()
    }
}

// Product algorithms
impl<CC: UnitChainComplex> Resolution<CC> {
    /// This function computes the products between the element most recently added to product_list
    /// and the parts of Ext that have already been computed. This function should be called right
    /// after `add_product`, unless `resolve_through_degree`/`resolve_through_bidegree` has never been
    /// called.
    ///
    /// This is made separate from `add_product` because extend_maps_to_unit needs a borrow of
    /// `self`, but `add_product` takes in a mutable borrow.
    pub fn catch_up_products(&self) {
        let new_product = [self.product_list.last().unwrap().clone()];
        if self.inner.has_computed_bidegree(0, 0) {
            let min_degree = self.min_degree();
            let max_s = self.max_computed_homological_degree();
            let max_t = self.max_computed_degree();

            self.construct_maps_to_unit(max_s, max_t);

            self.extend_maps_to_unit(max_s, max_t);

            for t in min_degree..=max_t {
                for s in 0..=max_s {
                    self.compute_products(s, t, &new_product);
                }
            }
        }
    }

    /// The return value is whether the product was actually added. If the product is already
    /// present, we do nothing.
    pub fn add_product(&mut self, s: u32, t: i32, class: Vec<u32>, name: &str) -> bool {
        let name = name.to_string();
        if self.product_names.contains(&name) {
            false
        } else {
            self.product_names.insert(name.clone());
            if s > self.max_product_homological_degree {
                self.max_product_homological_degree = s;
            }

            // We must add a product into product_list before calling compute_products, since
            // compute_products aborts when product_list is empty.
            self.product_list.push(Cocycle { s, t, class, name });
            true
        }
    }

    pub fn set_unit_resolution(&mut self, unit_res: Weak<RwLock<Resolution<CC>>>) {
        if !self.chain_maps_to_unit_resolution.is_empty() {
            panic!("Cannot change unit resolution after you start computing products");
        }
        self.unit_resolution = Some(unit_res);
    }

    /// Compute products whose result lie in degrees up to (s, t)
    fn compute_products(&self, s: u32, t: i32, products: &[Cocycle]) {
        for elt in products {
            self.compute_product_step(elt, s, t);
        }
    }

    /// Target = result of the product
    /// Source = multiplicand
    fn compute_product_step(&self, elt: &Cocycle, target_s: u32, target_t: i32) {
        if target_s < elt.s {
            return;
        }
        let source_s = target_s - elt.s;
        let source_t = target_t - elt.t;

        if source_t - (source_s as i32) < self.min_degree() {
            return;
        }

        let source_dim = self.inner.number_of_gens_in_bidegree(source_s, source_t);
        let target_dim = self.inner.number_of_gens_in_bidegree(target_s, target_t);

        let mut products = Vec::with_capacity(source_dim);
        for k in 0..source_dim {
            products.push(Vec::with_capacity(target_dim));

            let f = &self.chain_maps_to_unit_resolution[source_s][source_t][k];

            let unit_res_ = self.unit_resolution.as_ref().unwrap().upgrade().unwrap();
            let unit_res = unit_res_.read();
            let output_module = unit_res.module(elt.s);

            for l in 0..target_dim {
                let result = f.get_map(elt.s).output(target_t, l);
                let mut val = 0;
                for i in 0..elt.class.len() {
                    if elt.class[i] != 0 {
                        let idx = output_module.operation_generator_to_index(0, 0, elt.t, i);
                        val += elt.class[i] * result.entry(idx);
                    }
                }
                products[k].push(val % *self.prime());
            }
        }
        self.add_structline(
            &elt.name, source_s, source_t, target_s, target_t, true, products,
        );
    }

    fn construct_maps_to_unit(&self, s: u32, t: i32) {
        // If there are no products, we return
        if self.product_list.is_empty() {
            return;
        }

        let p = self.prime();
        let s_idx = s as usize;

        // Populate the arrays if the ResolutionHomomorphisms have not been defined.
        for new_s in 0..=s_idx {
            if new_s == self.chain_maps_to_unit_resolution.len() {
                self.chain_maps_to_unit_resolution
                    .push(OnceBiVec::new(self.min_degree()));
            }

            while t >= self.chain_maps_to_unit_resolution[new_s].len() {
                let new_t = self.chain_maps_to_unit_resolution[new_s].len();
                self.chain_maps_to_unit_resolution[new_s].push(OnceVec::new());

                let num_gens = self.module(new_s as u32).number_of_gens_in_degree(new_t);
                if num_gens > 0 {
                    let mut unit_vector = Matrix::new(p, num_gens, 1);
                    for j in 0..num_gens {
                        let f = ResolutionHomomorphism::new(
                            format!("(hom_deg : {}, int_deg : {}, idx : {})", new_s, new_t, j),
                            Arc::downgrade(&self.inner),
                            Arc::downgrade(
                                &self
                                    .unit_resolution
                                    .as_ref()
                                    .unwrap()
                                    .upgrade()
                                    .unwrap()
                                    .read()
                                    .inner,
                            ),
                            new_s as u32,
                            new_t,
                        );
                        unit_vector[j].set_entry(0, 1);
                        f.extend_step(new_s as u32, new_t, Some(&unit_vector));
                        unit_vector[j].set_to_zero();
                        self.chain_maps_to_unit_resolution[new_s][new_t].push(f);
                    }
                }
            }
        }
    }

    /// This ensures the chain_maps_to_unit_resolution are defined such that we can compute products up
    /// to bidegree (s, t)
    fn extend_maps_to_unit(&self, s: u32, t: i32) {
        // If there are no products, we return
        if self.product_list.is_empty() {
            return;
        }

        // Now we actually extend the maps.
        let min_degree = self.min_degree();
        for i in 0..=s {
            for j in min_degree..=t {
                let max_s = min(s, i + self.max_product_homological_degree);
                let num_gens = self.module(i).number_of_gens_in_degree(j);
                for k in 0..num_gens {
                    let f = &self.chain_maps_to_unit_resolution[i as usize][j][k];
                    f.extend(max_s, t);
                }
            }
        }
    }
}

// Self map algorithms
impl<CC: UnitChainComplex> Resolution<CC> {
    /// The return value is whether the self map was actually added. If the self map is already
    /// present, we do nothing.
    pub fn add_self_map(&mut self, s: u32, t: i32, name: &str, map_data: Matrix) -> bool {
        let name = name.to_string();
        if self.product_names.contains(&name) {
            false
        } else {
            self.product_names.insert(name.clone());
            self.self_maps.push(SelfMap {
                s,
                t,
                name,
                map_data,
                map: ResolutionHomomorphism::new(
                    "".to_string(),
                    Arc::downgrade(&self.inner),
                    Arc::downgrade(&self.inner),
                    s,
                    t,
                ),
            });
            true
        }
    }

    /// We compute the products by self maps where the result has degree (s, t).
    #[allow(clippy::needless_range_loop)]
    fn compute_self_maps(&self, target_s: u32, target_t: i32) {
        for f in &self.self_maps {
            if target_s < f.s {
                return;
            }
            let source_s = target_s - f.s;
            let source_t = target_t - f.t;

            if source_t - (source_s as i32) < self.min_degree() {
                continue;
            }
            if source_s == 0 && source_t == self.min_degree() {
                f.map.extend_step(target_s, target_t, Some(&f.map_data));
            }
            f.map.extend(target_s, target_t);

            let source = self.module(source_s);
            let target = self.module(target_s);

            let source_dim = source.number_of_gens_in_degree(source_t);
            let target_dim = target.number_of_gens_in_degree(target_t);

            let mut products = vec![Vec::with_capacity(target_dim); source_dim];

            for j in 0..target_dim {
                let result = f.map.get_map(source_s).output(target_t, j);

                for k in 0..source_dim {
                    let vector_idx = source.operation_generator_to_index(0, 0, source_t, k);
                    products[k].push(result.entry(vector_idx));
                }
            }
            self.add_structline(
                &f.name, source_s, source_t, target_s, target_t, false, products,
            );
        }
    }
}

impl<CC: UnitChainComplex> Resolution<CC> {
    pub fn algebra(&self) -> Arc<<CC::Module as Module>::Algebra> {
        self.complex().algebra()
    }

    pub fn prime(&self) -> ValidPrime {
        self.inner.prime()
    }

    pub fn module(
        &self,
        homological_degree: u32,
    ) -> Arc<FreeModule<<CC::Module as Module>::Algebra>> {
        self.inner.module(homological_degree)
    }

    pub fn min_degree(&self) -> i32 {
        self.complex().min_degree()
    }

    pub fn differential(
        &self,
        s: u32,
    ) -> Arc<FreeModuleHomomorphism<FreeModule<<CC::Module as Module>::Algebra>>> {
        self.inner.differential(s)
    }
}
