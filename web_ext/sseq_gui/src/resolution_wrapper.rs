use rustc_hash::FxHashSet as HashSet;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use algebra::module::homomorphism::FreeModuleHomomorphism;
use algebra::module::{FreeModule, Module};
use algebra::Algebra;
use ext::chain_complex::{AugmentedChainComplex, ChainComplex, FreeChainComplex};
use ext::resolution::Resolution as ResolutionInner;
use fp::matrix::Matrix;
use fp::prime::ValidPrime;
use fp::vector::prelude::*;
use once::{OnceBiVec, OnceVec};

use ext::resolution_homomorphism::ResolutionHomomorphism as ResolutionHomomorphism_;

use crate::actions::{Action, Message};
pub type ResolutionHomomorphism<CC> =
    ResolutionHomomorphism_<ResolutionInner<CC>, ResolutionInner<CC>>;

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
    pub map: ResolutionHomomorphism<CC>,
}

enum UnitResolution<CC: ChainComplex> {
    None,
    Own,
    Some(Box<Resolution<CC>>),
}

pub struct Resolution<CC: ChainComplex> {
    pub inner: Arc<ResolutionInner<CC>>,
    sender: crate::Sender,
    sseq: crate::actions::SseqChoice,

    /// The set of names of all products and self maps
    product_names: HashSet<String>,

    /// A list of all products
    product_list: Vec<Cocycle>,

    unit_resolution: UnitResolution<CC>,

    /// List of filtration one products
    filtration_one_products: Vec<(String, i32, usize)>,

    /// s -> t -> idx -> resolution homomorphism to unit resolution. We don't populate this
    /// until we actually have a unit resolution, of course.
    chain_maps_to_unit_resolution: OnceVec<OnceBiVec<Vec<ResolutionHomomorphism<CC>>>>,

    /// A list of all self maps
    self_maps: Vec<SelfMap<CC>>,
}

impl Resolution<ext::CCC> {
    pub fn new_from_json(
        json: Value,
        algebra_name: &str,
        sseq: crate::actions::SseqChoice,
        sender: crate::Sender,
    ) -> Option<Self> {
        let inner = Arc::new(ext::utils::construct((json.clone(), algebra_name), None).ok()?);
        let algebra = inner.algebra();

        let mut result = Self {
            inner,
            sender,
            sseq,

            product_names: HashSet::default(),
            product_list: Vec::new(),
            unit_resolution: UnitResolution::None,

            filtration_one_products: algebra.default_filtration_one_products(),
            self_maps: Vec::new(),
            chain_maps_to_unit_resolution: OnceVec::new(),
        };

        // Add products
        if !json["products"].is_null() {
            for prod in json["products"].as_array()? {
                let hom_deg = prod["hom_deg"].as_u64()? as u32;
                let int_deg = prod["int_deg"].as_i64()? as i32;
                let class: Vec<u32> = Vec::<u32>::deserialize(&prod["class"]).ok()?;
                let name = prod["name"].as_str()?;

                result.add_product(hom_deg, int_deg, class, name);
            }
        }

        if !json["self_maps"].is_null() {
            for self_map in json["self_maps"].as_array()? {
                let s = self_map["hom_deg"].as_u64()? as u32;
                let t = self_map["int_deg"].as_i64()? as i32;
                let name = self_map["name"].as_str()?;

                let json_map_data: Vec<Vec<u32>> =
                    <Vec<Vec<u32>>>::deserialize(&self_map["map_data"]).ok()?;

                let rows = json_map_data.len();
                let cols = json_map_data[0].len();
                let mut map_data = Matrix::new(result.prime(), rows, cols);
                for r in 0..rows {
                    for c in 0..cols {
                        map_data[r].set_entry(c, json_map_data[r][c]);
                    }
                }
                result.add_self_map(s, t, name, map_data);
            }
        }

        Some(result)
    }
}

impl<CC: ChainComplex> Resolution<CC> {
    pub fn compute_through_stem(&self, s: u32, n: i32) {
        self.inner
            .compute_through_stem_with_callback(s, n, |s, t| self.step_after(s, t));
    }

    pub fn step_after(&self, s: u32, t: i32) {
        if t - (s as i32) < self.min_degree() {
            return;
        }
        self.sender
            .send(Message {
                recipients: vec![],
                sseq: self.sseq,
                action: Action::from(crate::actions::AddClass {
                    x: t - s as i32,
                    y: s as i32,
                    num: self.inner.number_of_gens_in_bidegree(s, t),
                }),
            })
            .unwrap();
        if s > 0 {
            self.compute_filtration_one_products(s, t);
        }
        self.construct_maps_to_unit(s, t);
        for product in &self.product_list {
            self.compute_product(s, t, product);
        }
        self.compute_self_maps(s, t);
    }

    fn compute_filtration_one_products(&self, target_s: u32, target_t: i32) {
        for (op_name, op_degree, op_index) in &self.filtration_one_products {
            let source_s = target_s - 1;
            let source_t = target_t - *op_degree;
            if source_t - (source_s as i32) < self.min_degree() {
                continue;
            }

            let products = self
                .inner
                .filtration_one_product(*op_degree, *op_index, source_s, source_t)
                .unwrap();
            self.add_structline(op_name, source_s, source_t, 1, *op_degree, true, products);
        }
    }

    pub fn add_structline(
        &self,
        name: &str,
        source_s: u32,
        source_t: i32,
        mult_s: u32,
        mult_t: i32,
        left: bool,
        mut product: Vec<Vec<u32>>,
    ) {
        let p = self.prime();
        let source_s = source_s as i32;
        let mult_s = mult_s as i32;

        // Product in Ext is not product in E_2
        if (left && mult_s * source_t % 2 != 0) || (!left && mult_t * source_s % 2 != 0) {
            for entry in product.iter_mut().flatten() {
                *entry = ((*p - 1) * *entry) % *p;
            }
        }

        self.sender
            .send(Message {
                recipients: vec![],
                sseq: self.sseq,
                action: Action::from(crate::actions::AddProduct {
                    mult_x: mult_t - mult_s,
                    mult_y: mult_s,
                    source_x: source_t - source_s,
                    source_y: source_s,
                    name: name.to_owned(),
                    product,
                    left,
                }),
            })
            .unwrap();
    }

    pub fn complex(&self) -> Arc<CC> {
        self.inner.target()
    }
}

// Product algorithms
impl<CC: ChainComplex> Resolution<CC> {
    pub fn add_product(&mut self, s: u32, t: i32, class: Vec<u32>, name: &str) {
        if self.product_names.contains(name) {
            return;
        }

        let name = name.to_string();
        self.product_names.insert(name.clone());

        if let UnitResolution::Some(r) = &self.unit_resolution {
            r.compute_through_stem(s, t - s as i32);
        }
        let new_product = Cocycle { s, t, class, name };

        self.product_list.push(new_product.clone());

        if self.product_list.len() == 1 {
            for (s, _, t) in self.inner.iter_stem() {
                self.construct_maps_to_unit(s, t);
            }
        }

        if self.inner.has_computed_bidegree(0, 0) {
            for (s, _, t) in self.inner.iter_stem() {
                self.compute_product(s, t, &new_product);
            }
        }
    }

    pub fn unit_resolution(&self) -> &Resolution<CC> {
        match &self.unit_resolution {
            UnitResolution::None => panic!("No unit resolution set"),
            UnitResolution::Own => self,
            UnitResolution::Some(r) => r,
        }
    }

    pub fn unit_resolution_mut(&mut self) -> &mut Resolution<CC> {
        // This diversion is needed to get around weird borrowing issues.
        if matches!(self.unit_resolution, UnitResolution::Own) {
            self
        } else {
            match &mut self.unit_resolution {
                UnitResolution::None => panic!("No unit resolution set"),
                UnitResolution::Own => unreachable!(),
                UnitResolution::Some(ref mut r) => r,
            }
        }
    }

    pub fn set_unit_resolution(&mut self, unit_res: Resolution<CC>) {
        assert!(
            self.chain_maps_to_unit_resolution.is_empty(),
            "Cannot change unit resolution after you start computing products"
        );
        for product in &self.product_list {
            unit_res.compute_through_stem(product.s, product.t - product.s as i32);
        }
        self.unit_resolution = UnitResolution::Some(Box::new(unit_res));
    }

    pub fn set_unit_resolution_self(&mut self) {
        self.unit_resolution = UnitResolution::Own;
    }

    /// Target = result of the product
    /// Source = multiplicand
    fn compute_product(&self, target_s: u32, target_t: i32, elt: &Cocycle) {
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
            f.extend_through_stem(target_s, target_t - target_s as i32);

            let unit_res = self.unit_resolution();
            let output_module = unit_res.module(elt.s);

            for l in 0..target_dim {
                let map = f.get_map(target_s);
                let result = map.output(target_t, l);
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
        self.add_structline(&elt.name, source_s, source_t, elt.s, elt.t, true, products);
    }

    fn construct_maps_to_unit(&self, s: u32, t: i32) {
        // If there are no products, we return
        if self.product_list.is_empty() {
            return;
        }

        let p = self.prime();
        let s_idx = s as usize;

        if s_idx == self.chain_maps_to_unit_resolution.len() {
            self.chain_maps_to_unit_resolution
                .push_checked(OnceBiVec::new(self.min_degree() + s as i32), s_idx);
        }

        if t < self.chain_maps_to_unit_resolution[s_idx].len() {
            return;
        }
        let num_gens = self.module(s).number_of_gens_in_degree(t);
        let mut maps = Vec::with_capacity(num_gens);

        if num_gens > 0 {
            let mut unit_vector = Matrix::new(p, num_gens, 1);
            for j in 0..num_gens {
                let f = ResolutionHomomorphism::new(
                    format!("(hom_deg : {}, int_deg : {}, idx : {})", s, t, j),
                    Arc::clone(&self.inner),
                    Arc::clone(&self.unit_resolution().inner),
                    s,
                    t,
                );
                unit_vector[j].set_entry(0, 1);
                f.extend_step(s, t, Some(&unit_vector));
                unit_vector[j].set_to_zero();
                maps.push(f);
            }
        }
        self.chain_maps_to_unit_resolution[s_idx].push_checked(maps, t);
    }
}

// Self map algorithms
impl<CC: ChainComplex> Resolution<CC> {
    /// The return value is whether the self map was actually added. If the self map is already
    /// present, we do nothing.
    pub fn add_self_map(&mut self, s: u32, t: i32, name: &str, map_data: Matrix) -> bool {
        if self.product_names.contains(name) {
            false
        } else {
            self.product_names.insert(name.to_owned());
            self.self_maps.push(SelfMap {
                s,
                t,
                name: name.to_owned(),
                map_data,
                map: ResolutionHomomorphism::new(
                    "".to_string(),
                    Arc::clone(&self.inner),
                    Arc::clone(&self.inner),
                    s,
                    t,
                ),
            });
            true
        }
    }

    /// We compute the products by self maps where the result has degree (s, t).
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
            f.map
                .extend_through_stem(target_s, target_t - target_s as i32);

            let source = self.module(source_s);
            let target = self.module(target_s);

            let source_dim = source.number_of_gens_in_degree(source_t);
            let target_dim = target.number_of_gens_in_degree(target_t);

            let mut products = vec![Vec::with_capacity(target_dim); source_dim];

            for j in 0..target_dim {
                let map = f.map.get_map(target_s);
                let result = map.output(target_t, j);

                #[allow(clippy::needless_range_loop)]
                for k in 0..source_dim {
                    let vector_idx = source.operation_generator_to_index(0, 0, source_t, k);
                    products[k].push(result.entry(vector_idx));
                }
            }
            self.add_structline(&f.name, source_s, source_t, f.s, f.t, false, products);
        }
    }
}

impl<CC: ChainComplex> Resolution<CC> {
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
