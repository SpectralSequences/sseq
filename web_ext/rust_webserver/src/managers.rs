use crate::actions::*;
use crate::sseq::SseqWrapper;

use crate::resolution_wrapper::Resolution;
use algebra::{module::Module, JsonAlgebra};
use ext::chain_complex::ChainComplex;
use ext::utils::load_module_json;
use ext::CCC;

use serde_json::{json, Value};

#[cfg(feature = "concurrent")]
use {core::num::NonZeroUsize, thread_token::TokenBucket};

use crate::Sender;

#[cfg(feature = "concurrent")]
fn num_threads() -> NonZeroUsize {
    use std::env;

    match env::var("EXT_THREADS") {
        Ok(n) => match n.parse::<core::num::NonZeroUsize>() {
            Ok(n) => return n,
            Err(_) => eprintln!("Invalid value of EXT_THREADS variable: {}", n),
        },
        Err(env::VarError::NotUnicode(_)) => eprintln!("Invalid value of EXT_THREADS variable"),
        Err(env::VarError::NotPresent) => (),
    }
    core::num::NonZeroUsize::new(2).unwrap()
}

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
    bucket: TokenBucket,
    sender: Sender,
    is_unit: bool,
    resolved: bool,
    resolution: Option<Resolution<CCC>>,
}

impl ResolutionManager {
    /// Constructs a ResolutionManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `sender` object to send messages to.
    pub fn new(sender: Sender) -> Self {
        ResolutionManager {
            #[cfg(feature = "concurrent")]
            bucket: TokenBucket::new(num_threads()),

            sender,
            resolution: None,
            is_unit: false,
            resolved: false,
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
                let resolution = self.resolution.as_mut().unwrap();
                let resolution = match msg.sseq {
                    SseqChoice::Main => resolution,
                    SseqChoice::Unit => resolution.unit_resolution_mut(),
                };

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
        let resolution = Resolution::new_from_json(&json_data, &action.algebra_name);
        self.process_bundle(resolution, json_data);

        Ok(())
    }

    /// Resolves a module specified by `json`. The result is stored in `self.bundle`.
    fn construct(&mut self, action: Construct) -> error::Result<()> {
        let json = load_module_json(&action.module_name)?;
        let resolution = Resolution::new_from_json(&json, &action.algebra_name);
        self.process_bundle(resolution, json);

        Ok(())
    }

    fn process_bundle(&mut self, mut resolution: Resolution<CCC>, json: Value) {
        self.is_unit =
            resolution.complex().modules.len() == 1 && resolution.complex().module(0).is_unit();

        if self.is_unit {
            resolution.set_unit_resolution_self();
        } else {
            let mut unit_resolution = Resolution::new_from_json(
                &json!({
                    "type": "finite dimensional module",
                    "p": *resolution.prime(),
                    "gens": {"x0": 0},
                    "actions": [],
                    "save_file": json["unit_save_file"],
                }),
                resolution.algebra().prefix(),
            );
            self.setup_callback(&mut unit_resolution, SseqChoice::Unit);

            resolution.set_unit_resolution(unit_resolution);
        }
        self.setup_callback(&mut resolution, SseqChoice::Main);

        self.resolution = Some(resolution);
    }

    fn resolve(&mut self, action: Resolve, sseq: SseqChoice) -> error::Result<()> {
        let resolution = self.resolution.as_mut().unwrap();
        let resolution = match sseq {
            SseqChoice::Main => resolution,
            SseqChoice::Unit => resolution.unit_resolution_mut(),
        };

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

        if !self.resolved {
            // We resolve main first
            assert_eq!(sseq, SseqChoice::Main);
            for (s, _, t) in resolution.inner.iter_stem() {
                resolution.step_after(s, t);
            }
            if !self.is_unit {
                let unit = resolution.unit_resolution_mut();
                for (s, _, t) in unit.inner.iter_stem() {
                    unit.step_after(s, t);
                }
            }

            self.resolved = true;
        }

        #[cfg(not(feature = "concurrent"))]
        resolution.compute_through_degree(action.max_degree);

        #[cfg(feature = "concurrent")]
        resolution.compute_through_degree_concurrent(action.max_degree, &self.bucket);

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
    sseq: Option<SseqWrapper>,
    unit_sseq: Option<SseqWrapper>,
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
                    sseq.refresh();
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

    fn get_sseq(&mut self, sseq: SseqChoice) -> Option<&mut SseqWrapper> {
        match sseq {
            SseqChoice::Main => self.sseq.as_mut(),
            SseqChoice::Unit => self.unit_sseq.as_mut(),
        }
    }

    fn resolving(&mut self, msg: Message) -> error::Result<()> {
        if let Action::Resolving(m) = &msg.action {
            if self.sseq.is_none() {
                let sender = self.sender.clone();
                self.sseq = Some(SseqWrapper::new(
                    m.p,
                    SseqChoice::Main,
                    m.min_degree,
                    0,
                    Some(sender),
                ));

                let sender = self.sender.clone();
                self.unit_sseq = Some(SseqWrapper::new(m.p, SseqChoice::Unit, 0, 0, Some(sender)));
            }
        }
        self.relay(msg)
    }

    fn relay(&self, msg: Message) -> error::Result<()> {
        self.sender.send(msg)?;
        Ok(())
    }
}
