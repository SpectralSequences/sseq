use crate::sseq::{Sseq, ProductItem, ClassState, INFINITY};
use rust_ext::resolution::Resolution;
use rust_ext::fp_vector::FpVector;
use rust_ext::module::Module;
use rust_ext::CCC;
use bivec::BiVec;
use std::sync::{Arc, RwLock};
use enum_dispatch::enum_dispatch;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub recipients : Vec<Recipient>,
    pub sseq : SseqChoice,
    pub action: Action
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({:?})", self.action.to_string(), self.sseq)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Recipient {
    Sseq,
    Resolver,
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
    SetClassName,
    Clear,
    BlockRefresh,

    // Resolver -> Sseq
    AddProduct,
    AddClass,

    // Resolver -> JS
    Resolving,
    Complete,

    // JS -> Resolver
    Construct,
    ConstructJson,
    Resolve,

    // Sseq -> JS
    SetStructline,
    SetDifferential,
    SetClass,
    SetPageList,

    // Queries
    QueryTable,
    QueryTableResult,
    QueryCocycleString,
    QueryCocycleStringResult,
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
pub trait ActionT : std::fmt::Debug {
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message>{
        unimplemented!();
    }
    fn act_resolution(&self, resolution : &Arc<RwLock<Resolution<CCC>>>) -> Option<Message> {
        unimplemented!();
    }
    // We take this because sometimes we want to only take an immutable borrow.

    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
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
    fn act_sseq(&self, sseq: &mut Sseq) -> Option<Message> {
        sseq.add_differential_propagate(
            self.r, self.x, self.y,
            &FpVector::from_vec(sseq.p, &self.source),
            &mut Some(FpVector::from_vec(sseq.p, &self.target)),
            0);
        None
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
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message> {
        sseq.add_product_type(&self.name, self.x, self.y, true, self.permanent);
        None
    }

    fn act_resolution(&self, resolution : &Arc<RwLock<Resolution<CCC>>>) -> Option<Message> {
        let s = self.y as u32;
        let t = self.x + self.y;

        if resolution.write().unwrap().add_product(s, t, self.class.clone(), &self.name) {
            resolution.read().unwrap().catch_up_products();
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPermanentClass {
    pub x : i32,
    pub y : i32,
    pub class : Vec<u32>
}

impl ActionT for AddPermanentClass {
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message> {
        sseq.add_differential_propagate(
            INFINITY, self.x, self.y,
            &FpVector::from_vec(sseq.p, &self.class),
            &mut None,
            0);
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetClassName {
    pub x : i32,
    pub y : i32,
    pub idx : usize,
    pub name : String
}

impl ActionT for SetClassName {
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message> {
        sseq.set_class_name(self.x, self.y, self.idx, self.name.clone());
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clear {}
impl ActionT for Clear {
    fn act_sseq(&self, sseq: &mut Sseq) -> Option<Message> {
        sseq.clear();
        None
    }
}

/// This blocks the sseq object from recomputing classes and edges. This is useful when performing
/// a large number of operations in a row, e.g. when loading files or undoing. 
///
/// When loading a new file, this is both sent to the SseqManager and the ResolutionManager. All
/// the ResolutionManager does is forward this request to the SseqManager. This is important, since
/// these two messages go in different queues. If we only send it to the Sseq, then refresh will be
/// unblocked once the Sseq is done processing the queries, which is too early, since we are still
/// resolving and a lot of AddProduct messages are being sent out. By adding a message to the
/// ResolutionManger's queue, the refreshing will be unblocked only after the resolving is done
/// resolving.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRefresh {
    block : bool
}
impl ActionT for BlockRefresh {
    fn act_sseq(&self, sseq: &mut Sseq) -> Option<Message> {
        if self.block {
            sseq.block_refresh += 1;
        } else {
            sseq.block_refresh -= 1;
            if sseq.block_refresh == 0 {
                sseq.refresh_all();
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddClass {
    pub x : i32,
    pub y : i32,
    pub num : usize
}

impl ActionT for AddClass {
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message> {
        sseq.set_class(self.x, self.y, self.num);
        None
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
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message> {
        sseq.add_product(&self.name, self.source_x, self.source_y, self.mult_x, self.mult_y, self.left, &self.product);
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductDifferential {
    pub source : AddProductType,
    pub target : AddProductType
}

impl ActionT for AddProductDifferential {
    fn act_sseq(&self, sseq : &mut Sseq) -> Option<Message> {
        self.source.act_sseq(sseq);
        self.target.act_sseq(sseq);
        sseq.add_product_differential(&self.source.name, &self.target.name);
        None
    }

    fn act_resolution(&self, resolution : &Arc<RwLock<Resolution<CCC>>>) -> Option<Message> {
        self.source.act_resolution(resolution);
        self.target.act_resolution(resolution);
        None
    }

    fn to_string(&self) -> String {
        format!("{:?}", self).replace("AddProductType ","")
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
    pub classes : Vec<Vec<FpVector>>,
    pub decompositions : Vec<(FpVector, String, i32, i32)>,
    pub class_names : Vec<String>
}
impl ActionT for SetClass { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPageList {
    pub page_list : Vec<i32>
}
impl ActionT for SetPageList { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTable {
    pub s : u32,
    pub t : i32
}
impl ActionT for QueryTable {
    fn act_resolution(&self, resolution : &Arc<RwLock<Resolution<CCC>>>) -> Option<Message> {
        let resolution = resolution.read().unwrap();
        let s = self.s;
        let t = self.t;

        let module = resolution.module(s);
        if t < module.min_degree() {
            return None;
        }
        if t > module.max_computed_degree() {
            return None;
        }
        let string = module.generator_list_string(t);
        Some(Message {
            recipients : vec![],
            sseq : SseqChoice::Main, // This will be overwritten
            action : Action::from(QueryTableResult { s, t, string })
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTableResult {
    s : u32,
    t : i32,
    string : String
}
impl ActionT for QueryTableResult { }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCocycleString {
    s : u32,
    t : i32,
    idx : usize
}
impl ActionT for QueryCocycleString {
    fn act_resolution(&self, resolution : &Arc<RwLock<Resolution<CCC>>>) -> Option<Message> {
        let resolution = resolution.read().unwrap();
        let s = self.s;
        let t = self.t;
        let idx = self.idx;

        // Ensure bidegree is defined
        let module = resolution.module(s);
        if t < module.min_degree() {
            return None;
        }
        if t > module.max_computed_degree() {
            return None;
        }

        let string = resolution.inner.cocycle_string(s, t, idx);
        Some(Message{
            recipients : vec![],
            sseq : SseqChoice::Main, // This will be overwritten
            action : Action::from(QueryCocycleStringResult { s, t, idx, string })
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCocycleStringResult {
    s : u32,
    t : i32,
    idx : usize,
    string : String
}
impl ActionT for QueryCocycleStringResult { }
