use crate::actions::*;
use crate::sseq::Sseq;

use algebra::module::{FDModule, FiniteModule, Module};
use bivec::BiVec;
use ext::chain_complex::{ChainComplex, FiniteChainComplex};
use ext::resolution::Resolution;
use ext::utils::Config;
use ext::CCC;

use parking_lot::RwLock;
use std::sync::Arc;
#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;
#[cfg(feature = "concurrent")]
const NUM_THREADS: usize = 2;

use crate::Sender;

/// ResolutionManager is a struct that manipulates a Resolution. It is constructed with a "sender"
/// which is used to relay the results of the computation. This sender should send all messages to
/// SseqManager.
///
/// # Fields
///  * `sender` : A Sender object.
///  * `is_unit` : Whether this resolution is a resolution of the unit. This is useful for adding
///  products etc.
///  * `resolution` : The resolution object itself.
pub struct ResolutionManager {
    #[cfg(feature = "concurrent")]
    bucket: Arc<TokenBucket>,
    sender: Sender,
    is_unit: bool,
    resolution: Option<Arc<RwLock<Resolution<CCC>>>>,
    unit_resolution: Option<Arc<RwLock<Resolution<CCC>>>>,
}

impl ResolutionManager {
    /// Constructs a ResolutionManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `sender` object to send messages to.
    pub fn new(sender: Sender) -> Self {
        ResolutionManager {
            #[cfg(feature = "concurrent")]
            bucket: Arc::new(TokenBucket::new(NUM_THREADS)),

            sender,
            resolution: None,
            unit_resolution: None,
            is_unit: false,
        }
    }

    /// Reads a message and performs the actions as instructed.
    pub fn process_message(&mut self, msg: Message) -> error::Result<()> {
        // If the message is BlockRefresh, SseqManager is responsible for marking
        // it as complete.
        let isblock = matches!(msg.action, Action::BlockRefresh(_));
        let target_sseq = msg.sseq;

        let mut ret = None;
        match msg.action {
            Action::Construct(a) => self.construct(a)?,
            Action::ConstructJson(a) => self.construct_json(a)?,
            Action::Resolve(a) => self.resolve(a, msg.sseq)?,
            Action::BlockRefresh(_) => self.sender.send(msg)?,
            _ => {
                let resolution = match msg.sseq {
                    SseqChoice::Main => &self.resolution,
                    SseqChoice::Unit => &self.unit_resolution,
                };
                let resolution = resolution.as_ref().unwrap();

                ret = msg.action.act_resolution(resolution);
            }
        };

        if let Some(mut m) = ret {
            m.sseq = target_sseq;
            self.sender.send(m)?;
        }

        if !isblock {
            self.sender.send(Message {
                recipients: vec![],
                sseq: target_sseq,
                action: Action::from(Complete {}),
            })?;
        }
        Ok(())
    }

    /// Resolves a module defined by a json object. The result is stored in `self.bundle`.
    fn construct_json(&mut self, action: ConstructJson) -> error::Result<()> {
        let json_data = serde_json::from_str(&action.data)?;

        let bundle = ext::utils::construct_from_json(json_data, &action.algebra_name).unwrap();

        self.process_bundle(bundle);

        Ok(())
    }

    /// Resolves a module specified by `json`. The result is stored in `self.bundle`.
    fn construct(&mut self, action: Construct) -> error::Result<()> {
        let mut dir = std::env::current_exe().unwrap();
        dir.pop();
        dir.pop();
        dir.pop();
        dir.push("modules");

        let resolution = ext::utils::construct(&Config {
            module_paths: vec![dir],
            module_file_name: format!("{}.json", action.module_name),
            algebra_name: action.algebra_name,
            max_degree: 0, // This is not used.
        })
        .unwrap();

        self.process_bundle(resolution);

        Ok(())
    }

    fn process_bundle(&mut self, mut resolution: Resolution<CCC>) {
        self.is_unit =
            resolution.complex().modules.len() == 1 && resolution.complex().module(0).is_unit();

        if self.is_unit {
            let resolution = Arc::new(RwLock::new(resolution));
            resolution
                .write()
                .set_unit_resolution(Arc::downgrade(&resolution));
            self.unit_resolution = Some(Arc::clone(&resolution));
            self.resolution = Some(resolution);
        } else {
            let algebra = resolution.algebra();

            let unit_module = Arc::new(FiniteModule::from(FDModule::new(
                algebra,
                String::from("unit"),
                BiVec::from_vec(0, vec![1]),
            )));
            let ccdz = Arc::new(FiniteChainComplex::ccdz(unit_module));
            let unit_resolution = Arc::new(RwLock::new(Resolution::new(ccdz, None, None)));

            resolution.set_unit_resolution(Arc::downgrade(&unit_resolution));
            self.unit_resolution = Some(Arc::clone(&unit_resolution));
            self.resolution = Some(Arc::new(RwLock::new(resolution)));
        }

        let resolution = self.resolution.as_ref().unwrap();
        let mut resolution = resolution.write();
        self.setup_callback(&mut resolution, SseqChoice::Main);

        if !self.is_unit {
            let unit_resolution = self.unit_resolution.as_ref().unwrap();
            let mut unit_resolution = unit_resolution.write();
            self.setup_callback(&mut unit_resolution, SseqChoice::Unit);
        }
    }

    fn resolve(&self, action: Resolve, sseq: SseqChoice) -> error::Result<()> {
        let resolution = match sseq {
            SseqChoice::Main => &self.resolution,
            SseqChoice::Unit => &self.unit_resolution,
        };

        let resolution = resolution.as_ref().unwrap();
        let resolution = resolution.read();

        let min_degree = resolution.min_degree();

        let msg = Message {
            recipients: vec![],
            sseq,
            action: Action::from(Resolving {
                p: resolution.prime(),
                min_degree,
                max_degree: action.max_degree,
                is_unit: self.is_unit,
            }),
        };
        self.sender.send(msg)?;

        #[cfg(not(feature = "concurrent"))]
        resolution.resolve_through_degree(action.max_degree);

        #[cfg(feature = "concurrent")]
        resolution.resolve_through_degree_concurrent(action.max_degree, &self.bucket);

        Ok(())
    }
}

impl ResolutionManager {
    fn setup_callback(&self, resolution: &mut Resolution<CCC>, sseq: SseqChoice) {
        let p = resolution.prime();

        let sender = self.sender.clone();
        let add_class = move |s: u32, t: i32, num_gen: usize| {
            let msg = Message {
                recipients: vec![],
                sseq,
                action: Action::from(AddClass {
                    x: t - s as i32,
                    y: s as i32,
                    num: num_gen,
                }),
            };
            match sender.send(msg) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Failed to send class: {}", e);
                    panic!("")
                }
            };
        };

        let sender = self.sender.clone();
        let add_structline = move |name: &str,
                                   source_s: u32,
                                   source_t: i32,
                                   target_s: u32,
                                   target_t: i32,
                                   left: bool,
                                   mut product: Vec<Vec<u32>>| {
            let mult_s = (target_s - source_s) as i32;
            let mult_t = target_t - source_t;
            let source_s = source_s as i32;

            // Product in Ext is not product in E_2
            if (left && mult_s * source_t % 2 != 0) || (!left && mult_t * source_s % 2 != 0) {
                for prod_row in &mut product {
                    for prod_entry in prod_row {
                        *prod_entry = ((*p - 1) * *prod_entry) % *p;
                    }
                }
            }

            let msg = Message {
                recipients: vec![],
                sseq,
                action: Action::from(AddProduct {
                    mult_x: mult_t - mult_s,
                    mult_y: mult_s,
                    source_x: source_t - source_s,
                    source_y: source_s,
                    name: name.to_string(),
                    product,
                    left,
                }),
            };

            match sender.send(msg) {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to send product: {}", e),
            };
        };

        resolution.add_class = Some(Box::new(add_class));
        resolution.add_structline = Some(Box::new(add_structline));
    }
}

/// This is more-or-less the same as the ResolutionManager, except it manages the Sseq object. The
/// `sender` should send the information to the display frontend.
pub struct SseqManager {
    sender: Sender,
    sseq: Option<Sseq>,
    unit_sseq: Option<Sseq>,
}

impl SseqManager {
    /// Constructs a SseqManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `Sender` object to send messages to.
    pub fn new(sender: Sender) -> Self {
        SseqManager {
            sender,
            sseq: None,
            unit_sseq: None,
        }
    }

    /// # Return
    /// Whether this was a user action. If it is a user action, we want to send a "Complete" when
    /// completed, and also report the time.
    pub fn is_user(action: &Action) -> bool {
        !matches!(
            action,
            Action::AddClass(_)
                | Action::AddProduct(_)
                | Action::Complete(_)
                | Action::QueryTableResult(_)
                | Action::QueryCocycleStringResult(_)
                | Action::Resolving(_)
        )
    }

    pub fn process_message(&mut self, msg: Message) -> error::Result<bool> {
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
                recipients: vec![],
                sseq: target_sseq,
                action: Action::from(Complete {}),
            })?;
        }
        Ok(user)
    }

    fn get_sseq(&mut self, sseq: SseqChoice) -> Option<&mut Sseq> {
        match sseq {
            SseqChoice::Main => self.sseq.as_mut(),
            SseqChoice::Unit => self.unit_sseq.as_mut(),
        }
    }

    fn resolving(&mut self, msg: Message) -> error::Result<()> {
        if let Action::Resolving(m) = &msg.action {
            if self.sseq.is_none() {
                let sender = self.sender.clone();
                self.sseq = Some(Sseq::new(
                    m.p,
                    SseqChoice::Main,
                    m.min_degree,
                    0,
                    Some(sender),
                ));

                let sender = self.sender.clone();
                self.unit_sseq = Some(Sseq::new(m.p, SseqChoice::Unit, 0, 0, Some(sender)));
            }
        }
        self.relay(msg)
    }

    fn relay(&self, msg: Message) -> error::Result<()> {
        self.sender.send(msg)?;
        Ok(())
    }
}
