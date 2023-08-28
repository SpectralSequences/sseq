use crate::actions::*;
use crate::sseq::SseqWrapper;

use crate::resolution_wrapper::Resolution;
use algebra::module::Module;
use algebra::Algebra;
use ext::chain_complex::{BoundedChainComplex, ChainComplex};
use ext::utils::load_module_json;
use ext::CCC;

use anyhow::{anyhow, Context};
use serde_json::json;
use sseq::coordinates::Bidegree;

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
    sender: Sender,
    is_unit: bool,
    resolution: Option<Resolution<CCC>>,
}

impl ResolutionManager {
    /// Constructs a ResolutionManager object.
    ///
    /// # Arguments
    ///  * `sender` - The `sender` object to send messages to.
    pub fn new(sender: Sender) -> Self {
        ResolutionManager {
            sender,
            resolution: None,
            is_unit: false,
        }
    }

    /// Reads a message and performs the actions as instructed.
    pub fn process_message(&mut self, msg: Message) {
        if let Err(e) = self.process_message_inner(msg) {
            self.send_error(format!("{e:?}"));
        }
    }

    fn process_message_inner(&mut self, msg: Message) -> anyhow::Result<()> {
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
                let resolution = self
                    .resolution
                    .as_mut()
                    .ok_or_else(|| anyhow!("Resolution not yet constructed"))?;
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
    fn construct_json(&mut self, action: ConstructJson) -> anyhow::Result<()> {
        let json_data = serde_json::from_str(&action.data)
            .with_context(|| format!("Failed to parse json {}", action.data))?;
        let resolution = Resolution::new_from_json(
            json_data,
            &action.algebra_name,
            SseqChoice::Main,
            self.sender.clone(),
        )
        .ok_or_else(|| anyhow!("Invalid json encountered when parsing module file"))?;
        self.process_bundle(resolution)
    }

    /// Resolves a module specified by `json`. The result is stored in `self.bundle`.
    fn construct(&mut self, action: Construct) -> anyhow::Result<()> {
        let json = load_module_json(&action.module_name)?;
        let resolution = Resolution::new_from_json(
            json,
            &action.algebra_name,
            SseqChoice::Main,
            self.sender.clone(),
        )
        .ok_or_else(|| anyhow!("Invalid json encountered when parsing module file"))?;
        self.process_bundle(resolution)
    }

    fn process_bundle(&mut self, mut resolution: Resolution<CCC>) -> anyhow::Result<()> {
        self.is_unit =
            resolution.complex().max_s() == 1 && resolution.complex().module(0).is_unit();

        if self.is_unit {
            resolution.set_unit_resolution_self();
        } else {
            let unit_resolution = Resolution::new_from_json(
                json!({
                    "type": "finite dimensional module",
                    "p": *resolution.prime(),
                    "gens": {"x0": 0},
                    "actions": [],
                }),
                resolution.algebra().prefix(),
                SseqChoice::Unit,
                self.sender.clone(),
            )
            .unwrap();

            // Setting the unit resolution also resolves it enough to compute the product. So send
            // the Resolving message now.
            let msg = Message {
                recipients: vec![],
                sseq: SseqChoice::Unit,
                action: Action::from(Resolving {
                    p: resolution.prime(),
                    min_degree: 0,
                    // Dummy value that doesn't really matter
                    max_degree: 1,
                    is_unit: self.is_unit,
                }),
            };
            self.sender.send(msg)?;

            resolution.set_unit_resolution(unit_resolution);
        }

        self.resolution = Some(resolution);
        Ok(())
    }

    fn resolve(&self, action: Resolve, sseq: SseqChoice) -> anyhow::Result<()> {
        let resolution = self
            .resolution
            .as_ref()
            .ok_or_else(|| anyhow!("Calling Resolve before Construct"))?;
        let resolution = match sseq {
            SseqChoice::Main => resolution,
            SseqChoice::Unit => resolution.unit_resolution(),
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

        resolution.compute_through_stem(Bidegree::n_s(
            action.max_degree,
            action.max_degree as u32 / 2 + 5,
        ));

        Ok(())
    }

    pub fn send_error(&self, message: String) {
        self.sender
            .send(Message {
                recipients: Vec::new(),
                sseq: SseqChoice::Main,
                action: Action::from(Error { message }),
            })
            .unwrap()
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

    pub fn process_message(&mut self, msg: Message) {
        if let Err(e) = self.process_message_inner(msg) {
            self.send_error(format!("{e:?}"));
        }
    }

    fn process_message_inner(&mut self, msg: Message) -> anyhow::Result<bool> {
        let user = Self::is_user(&msg.action);
        let target_sseq = msg.sseq;

        match msg.action {
            Action::Resolving(_) => self.resolving(msg)?,
            Action::Complete(_) => self.relay(msg)?,
            Action::QueryTableResult(_) => self.relay(msg)?,
            Action::QueryCocycleStringResult(_) => self.relay(msg)?,
            Action::Error(_) => self.relay(msg)?,
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

    pub fn send_error(&self, message: String) {
        self.sender
            .send(Message {
                recipients: Vec::new(),
                sseq: SseqChoice::Main,
                action: Action::from(Error { message }),
            })
            .unwrap()
    }

    fn get_sseq(&mut self, sseq: SseqChoice) -> Option<&mut SseqWrapper> {
        match sseq {
            SseqChoice::Main => self.sseq.as_mut(),
            SseqChoice::Unit => self.unit_sseq.as_mut(),
        }
    }

    fn resolving(&mut self, msg: Message) -> anyhow::Result<()> {
        if let Action::Resolving(m) = &msg.action {
            let target = match msg.sseq {
                SseqChoice::Main => &mut self.sseq,
                SseqChoice::Unit => &mut self.unit_sseq,
            };
            if target.is_none() {
                *target = Some(SseqWrapper::new(
                    m.p,
                    msg.sseq,
                    m.min_degree,
                    0,
                    Some(self.sender.clone()),
                ));
            }
        }
        self.relay(msg)
    }

    fn relay(&self, msg: Message) -> anyhow::Result<()> {
        self.sender.send(msg)?;
        Ok(())
    }
}
