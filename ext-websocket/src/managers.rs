use crate::actions::*;
use crate::sseq::Sseq;

use rust_ext::Config;
use rust_ext::AlgebraicObjectsBundle;
use rust_ext::CCC;
use rust_ext::module::Module;
use rust_ext::resolution::Resolution;
use rust_ext::chain_complex::ChainComplex;
use std::error::Error;

use std::sync::{RwLock, Arc};
#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;
#[cfg(feature = "concurrent")]
const NUM_THREADS : usize = 2;

use crate::Sender;

/// ResolutionManager is a struct that manipulates an AlgebraicObjectsBundle. It is constructed
/// with a "sender" which is used to relay the results of the computation. This sender should send
/// all messages to SseqManager.
///
/// # Fields
///  * `sender` : A Sender object.
///  * `is_unit` : Whether this resolution is a resolution of the unit. This is useful for adding
///  products etc.
///  * `resolution` : The resolution object itself.
pub struct ResolutionManager {
    #[cfg(feature = "concurrent")]
    bucket : Arc<TokenBucket>,
    sender : Sender,
    is_unit : bool,
    resolution : Option<Arc<RwLock<Resolution<CCC>>>>
}

impl ResolutionManager {
    /// Constructs a ResolutionManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `sender` object to send messages to.
    pub fn new(sender : Sender) -> Self {
        ResolutionManager {
            #[cfg(feature = "concurrent")]
            bucket : Arc::new(TokenBucket::new(NUM_THREADS)),

            sender : sender,
            resolution : None,
            is_unit : false,
        }
    }

    /// Reads a message and performs the actions as instructed.
    pub fn process_message(&mut self, msg : Message) -> Result<(), Box<dyn Error>> {
        // If the message is BlockRefresh, SseqManager is responsible for marking
        // it as complete.
        let isblock = match msg.action { Action::BlockRefresh(_) => true, _ => false };
        let target_sseq = msg.sseq;

        let mut ret = None;
        match msg.action {
            Action::Construct(a) => self.construct(a)?,
            Action::ConstructJson(a) => self.construct_json(a)?,
            Action::Resolve(a) => self.resolve(a, msg.sseq)?,
            Action::BlockRefresh(_) => self.sender.send(msg)?,
            _ => {
                // Find a better way to make this work.
                match msg.sseq {
                    SseqChoice::Main => {
                        if let Some(resolution) = &self.resolution {
                            ret = msg.action.act_resolution(resolution);
                        }
                    },
                    SseqChoice::Unit => {
                        if let Some(main_resolution) = &self.resolution {
                            if let Some(resolution) = &main_resolution.read().unwrap().unit_resolution {
                                ret = msg.action.act_resolution(&resolution.upgrade().unwrap());
                            }
                        }
                    }
                }
            }
        };

        if let Some(mut m) = ret {
            m.sseq = target_sseq;
            self.sender.send(m)?;
        }

        if !isblock {
            self.sender.send(Message {
                recipients : vec![],
                sseq : target_sseq,
                action : Action::from(Complete {})
            })?;
        }
        Ok(())
    }

    /// Resolves a module defined by a json object. The result is stored in `self.bundle`.
    fn construct_json(&mut self, action : ConstructJson) -> Result<(), Box<dyn Error>> {
        let json_data = serde_json::from_str(&action.data)?;

        let bundle = rust_ext::construct_from_json(json_data, action.algebra_name).unwrap();

        self.process_bundle(bundle);

        Ok(())
    }

    /// Resolves a module specified by `json`. The result is stored in `self.bundle`.
    fn construct(&mut self, action : Construct) -> Result<(), Box<dyn Error>> {
        let mut dir = std::env::current_exe().unwrap();
        dir.pop(); dir.pop(); dir.pop();
        dir.push("modules");

        let bundle = rust_ext::construct(&Config {
             module_paths : vec![dir],
             module_file_name : format!("{}.json", action.module_name),
             algebra_name : action.algebra_name.to_string(),
             max_degree : 0 // This is not used.
        }).unwrap();

        self.process_bundle(bundle);

        Ok(())
    }

    fn process_bundle(&mut self, bundle : AlgebraicObjectsBundle) {
        self.is_unit = bundle.chain_complex.modules.len() == 1 && bundle.chain_complex.module(0).is_unit();
        if self.is_unit {
            bundle.resolution.write().unwrap().set_unit_resolution(Arc::downgrade(&bundle.resolution));
        } else {
            bundle.resolution.write().unwrap().construct_unit_resolution();
        }
        self.resolution = Some(bundle.resolution);

        if let Some(resolution) = &self.resolution {
            self.setup_callback(&mut resolution.write().unwrap(), SseqChoice::Main);
            if !self.is_unit {
                if let Some(unit_res) = &resolution.read().unwrap().unit_resolution {
                    self.setup_callback(&mut unit_res.upgrade().unwrap().write().unwrap(), SseqChoice::Unit);

                }
            }
        }
   }

    fn resolve(&self, action : Resolve, sseq : SseqChoice) -> Result<(), Box<dyn Error>> {
        let resolution = &self.resolution.as_ref().unwrap();
        let min_degree = match sseq {
            SseqChoice::Main => resolution.read().unwrap().min_degree(),
            SseqChoice::Unit => 0
        };

        let msg = Message {
            recipients : vec![],
            sseq,
            action : Action::from(Resolving {
                p : resolution.read().unwrap().prime(),
                min_degree,
                max_degree : action.max_degree,
                is_unit : self.is_unit
            })
        };
        self.sender.send(msg)?;

        #[cfg(not(feature = "concurrent"))]
        match sseq {
            SseqChoice::Main => resolution.read().unwrap().resolve_through_degree(action.max_degree),
            SseqChoice::Unit => {
                if let Some(r) = &resolution.read().unwrap().unit_resolution {
                    r.upgrade().unwrap().read().unwrap().resolve_through_degree(action.max_degree)
                }
            }
        };

        #[cfg(feature = "concurrent")]
        match sseq {
            SseqChoice::Main => resolution.read().unwrap().resolve_through_degree_concurrent(action.max_degree, &self.bucket),
            SseqChoice::Unit => {
                if let Some(r) = &resolution.read().unwrap().unit_resolution {
                    r.upgrade().unwrap().read().unwrap().resolve_through_degree_concurrent(action.max_degree, &self.bucket)
                }
            }
        };

        Ok(())
    }
}

impl ResolutionManager {
    fn setup_callback(&self, resolution : &mut Resolution<CCC>, sseq : SseqChoice) {
        let p = resolution.prime();

        let sender = self.sender.clone();
        let add_class = move |s: u32, t: i32, num_gen: usize| {
            let msg = Message {
                recipients : vec![],
                sseq : sseq,
                action : Action::from(AddClass {
                    x : t - s as i32,
                    y : s as i32,
                    num : num_gen
                })
            };
            match sender.send(msg) {
                Ok(_) => (),
                Err(e) => {eprintln!("Failed to send class: {}", e); panic!("")}
            };
        };

        let sender = self.sender.clone();
        let add_structline = move |name : &str, source_s: u32, source_t: i32, target_s : u32, target_t : i32, left : bool, mut product : Vec<Vec<u32>>| {
            let mult_s = (target_s - source_s) as i32;
            let mult_t = target_t - source_t;
            let source_s = source_s as i32;

            // Product in Ext is not product in E_2
            if (left && mult_s * source_t % 2 != 0) ||
               (!left && mult_t * source_s % 2 != 0) {
                for a in 0 .. product.len() {
                    for b in 0 .. product[a].len() {
                        product[a][b] = ((p - 1) * product[a][b]) % p;
                    }
                }
            }

            let msg = Message {
                recipients : vec![],
                sseq : sseq,
                action : Action::from(AddProduct {
                    mult_x : mult_t - mult_s,
                    mult_y : mult_s,
                    source_x : source_t - source_s,
                    source_y : source_s,
                    name : name.to_string(),
                    product,
                    left
                })
            };

            match sender.send(msg) {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to send product: {}", e)
            };
        };

        resolution.add_class = Some(Box::new(add_class));
        resolution.add_structline = Some(Box::new(add_structline));
    }
}

/// This is more-or-less the same as the ResolutionManager, except it manages the Sseq object. The
/// `sender` should send the information to the display frontend.
pub struct SseqManager {
    sender : Sender,
    sseq : Option<Sseq>,
    unit_sseq : Option<Sseq>
}

impl SseqManager {
    /// Constructs a SseqManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `Sender` object to send messages to.
    pub fn new(sender : Sender) -> Self {
        SseqManager {
             sender : sender,
             sseq : None,
             unit_sseq : None
        }
    }

    /// # Return
    /// Whether this was a user action. If it is a user action, we want to send a "Complete" when
    /// completed, and also report the time.
    pub fn is_user(action : &Action) -> bool{
        match action {
            Action::AddClass(_) => false,
            Action::AddProduct(_) => false,
            Action::Complete(_) => false,
            Action::QueryTableResult(_) => false,
            Action::QueryCocycleStringResult(_) => false,
            Action::Resolving(_) => false,
            _ => true
        }
    }

    pub fn process_message(&mut self, msg : Message) -> Result<bool, Box<dyn Error>> {
        let user = Self::is_user(&msg.action);
        let target_sseq = msg.sseq;

        match msg.action {
            Action::Resolving(_) => self.resolving(msg)?,
            Action::Complete(_) => self.relay(msg)?,
            Action::QueryTableResult(_) => self.relay(msg)?,
            Action::QueryCocycleStringResult(_) => self.relay(msg)?,
            _ => {
                if let Some(sseq) = self.get_sseq(msg.sseq) {
                    msg.action.act_sseq(sseq);
                }
            }
        };

        if user {
            self.sender.send(Message {
                recipients : vec![],
                sseq : target_sseq,
                action : Action::from(Complete {})
            })?;
        }
        Ok(user)
    }

    fn get_sseq(&mut self, sseq : SseqChoice) -> Option<&mut Sseq> {
        match sseq {
            SseqChoice::Main => self.sseq.as_mut(),
            SseqChoice::Unit => self.unit_sseq.as_mut()
        }
    }

    fn resolving(&mut self, msg : Message) -> Result<(), Box<dyn Error>> {
        if let Action::Resolving(m) = &msg.action {
            if self.sseq.is_none() {
                let sender = self.sender.clone();
                self.sseq = Some(Sseq::new(m.p, SseqChoice::Main, m.min_degree, 0, Some(sender)));

                let sender = self.sender.clone();
                self.unit_sseq = Some(Sseq::new(m.p, SseqChoice::Unit, 0, 0, Some(sender)));
            }
        }
        self.relay(msg)
    }

    fn relay(&self, msg : Message) -> Result<(), Box<dyn Error>> {
        self.sender.send(msg)?;
        Ok(())
    }
}
