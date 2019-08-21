use crate::sseq::{Sseq, ProductItem, ClassState, INFINITY};
use rust_ext::module::FiniteModule;
use rust_ext::resolution::{ModuleResolution};
use rust_ext::fp_vector::FpVector;
use bivec::BiVec;
use std::rc::Rc;
use std::cell::RefCell;
use enum_dispatch::enum_dispatch;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub recipients : Vec<Recipient>,
    pub sseq : SseqChoice,
    pub action: Action
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Recipient {
    Sseq,
    Resolver,
    Server
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum SseqChoice {
    Main,
    Unit
}

/// This is just a list of everything that implements `ActionT`. We use this instead of `Box<dyn
/// ActionT>` so that Serde is happy.
#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    // JS -> Sseq
    AddProductDifferential,
    AddProductType,
    AddPermanentClass,
    AddDifferential,
    Clear,

    // Resolver -> Sseq
    AddProduct,
    AddClass,

    // Resolver -> JS
    Resolving,
    Complete,
    QueryTableResult,

    // JS -> Resolver
    Construct,
    ConstructJson,
    Resolve,
    QueryTable,

    // Sseq -> JS
    SetStructline,
    SetDifferential,
    SetClass,
    SetPageList,

    // Misc
    RequestHistory,
    ReturnHistory,
}

/// The name `Action` is sort-of a misnomer. It is the content of any message that is sent between
/// the different objects. There are three functions to implement. 
/// 
/// The function `user` really should be a trait constant, except that makes `enum_dispatch`
/// unhappy. It indicates whether this action comes from a user action. For example, `AddProduct`
/// is not a user action but `AddProductType` is. This field doesn't really make sense for messages
/// sent *to* the user, but we set it as false anyway (which is the default).
///
/// The functions `act_sseq` and `act_resolution` executes the action of the command on the
/// corresponding object. These are by default empty and should be left empty if the action is not
/// expected to act on the corresponding object. For example, `AddDifferential` has an empty
/// `act_resolution` function.
#[enum_dispatch(Action)]
#[allow(unused_variables)]
pub trait ActionT {
    fn act_sseq(&self, sseq : &mut Sseq) {
        unimplemented!();
    }
    fn act_resolution(&self, resolution : &Rc<RefCell<ModuleResolution<FiniteModule>>>) {
        unimplemented!();
    }
    // We take this because sometimes we want to only take an immutable borrow.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDifferential {
    pub x : i32,
    pub y : i32,
    pub r : i32,
    pub source : Vec<u32>,
    pub target : Vec<u32>
}

impl ActionT for AddDifferential {
    fn act_sseq(&self, sseq: &mut Sseq) {
        sseq.add_differential_propagate(
            self.r, self.x, self.y,
            &FpVector::from_vec(sseq.p, &self.source),
            &mut Some(FpVector::from_vec(sseq.p, &self.target)),
            0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clear {}
impl ActionT for Clear {
    fn act_sseq(&self, sseq: &mut Sseq) {
        sseq.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductType {
    pub x : i32,
    pub y : i32,
    pub class : Vec<u32>,
    pub name : String,
    pub permanent : bool
}

impl ActionT for AddProductType {
    fn act_sseq(&self, sseq : &mut Sseq) {
        sseq.add_product_type(&self.name, self.x, self.y, true, self.permanent);
    }

    fn act_resolution(&self, resolution : &Rc<RefCell<ModuleResolution<FiniteModule>>>) {
        let s = self.y as u32;
        let t = self.x + self.y;

        if resolution.borrow_mut().add_product(s, t, self.class.clone(), &self.name) {
            resolution.borrow().catch_up_products();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPermanentClass {
    pub x : i32,
    pub y : i32,
    pub class : Vec<u32>
}

impl ActionT for AddPermanentClass {
    fn act_sseq(&self, sseq : &mut Sseq) {
        sseq.add_differential_propagate(
            INFINITY, self.x, self.y,
            &FpVector::from_vec(sseq.p, &self.class),
            &mut None,
            0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddClass {
    pub x : i32,
    pub y : i32,
    pub num : usize
}

impl ActionT for AddClass {
    fn act_sseq(&self, sseq : &mut Sseq) {
        sseq.set_class(self.x, self.y, self.num);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProduct {
    pub mult_x : i32,
    pub mult_y : i32,
    pub source_x : i32,
    pub source_y : i32,
    pub name : String,
    pub product : Vec<Vec<u32>>,
    pub left : bool
}

impl ActionT for AddProduct {
    fn act_sseq(&self, sseq : &mut Sseq) {
        sseq.add_product(&self.name, self.source_x, self.source_y, self.mult_x, self.mult_y, self.left, &self.product);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductDifferential {
    pub source : AddProductType,
    pub target : AddProductType
}

impl ActionT for AddProductDifferential {
    fn act_sseq(&self, sseq : &mut Sseq) {
        self.source.act_sseq(sseq);
        self.target.act_sseq(sseq);
        sseq.add_product_differential(&self.source.name, &self.target.name);
    }

    fn act_resolution(&self, resolution : &Rc<RefCell<ModuleResolution<FiniteModule>>>) {
        self.source.act_resolution(resolution);
        self.target.act_resolution(resolution);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolving {
    pub p : u32,
    pub min_degree : i32,
    pub max_degree : i32,
    pub is_unit : bool,
}

impl ActionT for Resolving { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Complete { }
impl ActionT for Complete { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTableResult {
    pub s : u32,
    pub t : i32,
    pub string : String
}
impl ActionT for QueryTableResult { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Construct {
    pub module_name : String,
    pub algebra_name : String,
}
impl ActionT for Construct { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructJson {
    pub data : String,
    pub algebra_name : String,
}
impl ActionT for ConstructJson { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolve {
    pub max_degree : i32
}
impl ActionT for Resolve { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTable {
    pub s : u32,
    pub t : i32
}
impl ActionT for QueryTable { }

// Now actions for sseq -> js
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStructline {
    pub x : i32,
    pub y : i32,
    pub structlines : Vec<ProductItem>
}
impl ActionT for SetStructline { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDifferential {
    pub x : i32,
    pub y : i32,
    pub true_differentials : Vec<Vec<(Vec<u32>, Vec<u32>)>>,
    pub differentials : BiVec<Vec<Vec<u32>>>
}
impl ActionT for SetDifferential { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetClass {
    pub x : i32,
    pub y : i32,
    pub state : ClassState,
    pub permanents : Vec<FpVector>,
    pub classes : Vec<Vec<FpVector>>
}
impl ActionT for SetClass { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPageList {
    pub page_list : Vec<i32>
}
impl ActionT for SetPageList { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestHistory { }
impl ActionT for RequestHistory { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnHistory {
    pub history : Vec<Message>
}
impl ActionT for ReturnHistory { }
