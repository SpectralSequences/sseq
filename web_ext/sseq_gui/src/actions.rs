use crate::resolution_wrapper::Resolution;
use crate::sseq::{ClassState, ProductItem, SseqWrapper};
use algebra::module::Module;
use bivec::BiVec;
use enum_dispatch::enum_dispatch;
use ext::{chain_complex::FreeChainComplex, CCC};
use fp::vector::FpVector;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sseq::coordinates::{Bidegree, BidegreeGenerator};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub recipients: Vec<Recipient>,
    pub sseq: SseqChoice,
    pub action: Action,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({:?})", self.action.to_string(), self.sseq)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recipient {
    Sseq,
    Resolver,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SseqChoice {
    Main,
    Unit,
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

    // Error
    Error,
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
pub trait ActionT: std::fmt::Debug {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        unimplemented!();
    }
    fn act_resolution(&self, resolution: &mut Resolution<CCC>) -> Option<Message> {
        unimplemented!();
    }
    fn to_string(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDifferential {
    pub x: i32,
    pub y: i32,
    pub r: i32,
    pub source: Vec<u32>,
    pub target: Vec<u32>,
}

impl ActionT for AddDifferential {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        let source = FpVector::from_slice(sseq.p, &self.source);
        let target = FpVector::from_slice(sseq.p, &self.target);

        sseq.inner
            .add_differential(self.r, self.x, self.y, source.as_slice(), target.as_slice());
        sseq.add_differential_propagate(self.r, self.x, self.y, source.as_slice(), 0);
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductType {
    pub x: i32,
    pub y: i32,
    pub class: Vec<u32>,
    pub name: String,
    pub permanent: bool,
}

impl ActionT for AddProductType {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        sseq.add_product_type(&self.name, self.x, self.y, true, self.permanent);
        None
    }

    fn act_resolution(&self, resolution: &mut Resolution<CCC>) -> Option<Message> {
        let b = Bidegree::s_t(self.y as u32, self.x + self.y);

        resolution.add_product(b, self.class.clone(), &self.name);
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPermanentClass {
    pub x: i32,
    pub y: i32,
    pub class: Vec<u32>,
}

impl ActionT for AddPermanentClass {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        let class = FpVector::from_slice(sseq.p, &self.class);

        sseq.inner
            .add_permanent_class(self.x, self.y, class.as_slice());
        sseq.add_differential_propagate(i32::MAX, self.x, self.y, class.as_slice(), 0);

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetClassName {
    pub x: i32,
    pub y: i32,
    pub idx: usize,
    pub name: String,
}

impl ActionT for SetClassName {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        sseq.set_class_name(self.x, self.y, self.idx, self.name.clone());
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clear {}
impl ActionT for Clear {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
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
    block: bool,
}
impl ActionT for BlockRefresh {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        if self.block {
            sseq.block_refresh += 1;
        } else {
            sseq.block_refresh -= 1;
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddClass {
    pub x: i32,
    pub y: i32,
    pub num: usize,
}

impl ActionT for AddClass {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        sseq.set_dimension(self.x, self.y, self.num);
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProduct {
    pub mult_x: i32,
    pub mult_y: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub name: String,
    pub product: Vec<Vec<u32>>,
    pub left: bool,
}

impl ActionT for AddProduct {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        sseq.add_product(
            &self.name,
            self.source_x,
            self.source_y,
            self.mult_x,
            self.mult_y,
            self.left,
            &self.product,
        );
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProductDifferential {
    pub source: AddProductType,
    pub target: AddProductType,
}

impl ActionT for AddProductDifferential {
    fn act_sseq(&self, sseq: &mut SseqWrapper) -> Option<Message> {
        self.source.act_sseq(sseq);
        self.target.act_sseq(sseq);
        sseq.add_product_differential(&self.source.name, &self.target.name);
        None
    }

    fn act_resolution(&self, resolution: &mut Resolution<CCC>) -> Option<Message> {
        self.source.act_resolution(resolution);
        self.target.act_resolution(resolution);
        None
    }

    fn to_string(&self) -> String {
        format!("{self:?}").replace("AddProductType ", "")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolving {
    pub p: fp::prime::ValidPrime,
    pub min_degree: i32,
    pub max_degree: i32,
    pub is_unit: bool,
}

impl ActionT for Resolving {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Complete {}
impl ActionT for Complete {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Construct {
    pub module_name: String,
    pub algebra_name: String,
}
impl ActionT for Construct {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructJson {
    pub data: String,
    pub algebra_name: String,
}
impl ActionT for ConstructJson {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolve {
    pub max_degree: i32,
}
impl ActionT for Resolve {}

// Now actions for sseq -> js
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStructline {
    pub x: i32,
    pub y: i32,
    pub structlines: Vec<ProductItem>,
}
impl ActionT for SetStructline {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDifferential {
    pub x: i32,
    pub y: i32,
    pub true_differentials: Vec<Vec<(Vec<u32>, Vec<u32>)>>,
    pub differentials: BiVec<Vec<Vec<u32>>>,
}
impl ActionT for SetDifferential {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetClass {
    pub x: i32,
    pub y: i32,
    pub state: ClassState,
    pub permanents: Vec<FpVector>,
    pub classes: Vec<Vec<FpVector>>,
    pub decompositions: Vec<(FpVector, String, i32, i32)>,
    pub class_names: Vec<String>,
}
impl ActionT for SetClass {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPageList {
    pub page_list: Vec<i32>,
}
impl ActionT for SetPageList {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTable {
    pub b: Bidegree,
}
impl ActionT for QueryTable {
    fn act_resolution(&self, resolution: &mut Resolution<CCC>) -> Option<Message> {
        let module = resolution.module(self.b.s());
        if self.b.t() < module.min_degree() {
            return None;
        }
        if self.b.t() > module.max_computed_degree() {
            return None;
        }
        let dimension = module.dimension(self.b.t());
        let string = (0..dimension)
            .map(|i| module.basis_element_to_string(self.b.t(), i))
            .join(", ");
        Some(Message {
            recipients: vec![],
            sseq: SseqChoice::Main, // This will be overwritten
            action: Action::from(QueryTableResult { b: self.b, string }),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTableResult {
    b: Bidegree,
    string: String,
}
impl ActionT for QueryTableResult {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCocycleString {
    g: BidegreeGenerator,
}
impl ActionT for QueryCocycleString {
    fn act_resolution(&self, resolution: &mut Resolution<CCC>) -> Option<Message> {
        // Ensure bidegree is defined
        let module = resolution.module(self.g.s());
        if self.g.t() < module.min_degree() {
            return None;
        }
        if self.g.t() > module.max_computed_degree() {
            return None;
        }

        let string = resolution.inner.cocycle_string(self.g, true);
        Some(Message {
            recipients: vec![],
            sseq: SseqChoice::Main, // This will be overwritten
            action: Action::from(QueryCocycleStringResult { g: self.g, string }),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCocycleStringResult {
    g: BidegreeGenerator,
    string: String,
}
impl ActionT for QueryCocycleStringResult {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub message: String,
}

impl ActionT for Error {}
