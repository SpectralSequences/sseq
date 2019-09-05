extern crate rust_ext;
extern crate bivec;
extern crate serde_json;

mod sseq;
mod actions;

use sseq::Sseq;
use actions::*;
use rust_ext::Config;
use rust_ext::AlgebraicObjectsBundle;
use rust_ext::module::{Module, FiniteModule};
use rust_ext::resolution::{ModuleResolution};
use rust_ext::chain_complex::ChainComplex;

use std::{fs, thread};
use std::cell::RefCell;
use std::rc::Rc;
use std::error::Error;

extern crate ws;
extern crate chrono;
extern crate textwrap;
use chrono::Local;
use std::sync::mpsc;
use ws::{Handler, Request, Response, Sender as WsSender, Result as WsResult};
use textwrap::Wrapper;

type Sender = mpsc::Sender<Message>;
type Receiver = mpsc::Receiver<Message>;

/// List of files that our webserver will serve to the user
const FILE_LIST : [(&str, &str, &[u8]); 12] = [
    ("/", "index.html", b"text/html"),
    ("/index.html", "index.html", b"text/html"),
    ("/index.js", "index.js", b"text/javascript"),
    ("/mousetrap.min.js", "mousetrap.min.js", b"text/javascript"),
    ("/canvas2svg.js", "canvas2svg.js", b"text/javascript"),
    ("/display.js", "display.js", b"text/javascript"),
    ("/utils.js", "utils.js", b"text/javascript"),
    ("/tooltip.js", "tooltip.js", b"text/javascript"),
    ("/panels.js", "panels.js", b"text/javascript"),
    ("/sseq.js", "sseq.js", b"text/javascript"),
    ("/index.css", "index.css", b"text/css"),
    ("/bundle.js", "bundle.js", b"text/javascript")];

fn ms_to_string(time : i64) -> String {
    if time < 1000 {
        format!("{}ms", time)
    } else if time < 10000 {
        format!("{}.{}s", time / 1000, time % 1000)
    } else {
        format!("{}s", time / 1000)
    }
}

/// ResolutionManager is a struct that manipulates an AlgebraicObjectsBundle. At the moment, it
/// only understands the "resolve" command which causes it to resolve a module and report back the
/// results.
///
/// The main function is `ResolutionManager::new`. This function does not return a ResolutionManager
/// object. Instead, the function produces a ResolutionManager object and waits for commands issued
/// by the user. The actions of the command will involve manipulating the ResolutionManager.
/// However, not everything interesting can be found inside the struct itself. Instead, some
/// variables are simply local to the function `ResolutionManager::new`. What goes into the struct
/// and what stays a local variable is simply a matter of convenience.
struct ResolutionManager {
    sender : Sender,
    is_unit : bool,
    resolution : Option<Rc<RefCell<ModuleResolution<FiniteModule>>>>
}

impl ResolutionManager {
    /// Constructs a ResolutionManager object and waits for messages coming from `receiver`. The
    /// results of calculations are relayed back via `sender` in the form of stringified JSON. When
    /// the `receiver` stream ends, the function terminates and returns `()`, dropping the
    /// ResolutionManager object.
    ///
    /// # Arguments
    ///  * `receiver` - The `eceiver` object to listen commands from.
    ///  * `sender` - The `ender` object to send messages to.
    fn new(receiver : Receiver, sender : Sender) -> Result<(), Box<dyn Error>> {
        let mut manager = ResolutionManager {
             sender : sender,
             resolution : None,
             is_unit : false,
        };

        let wrapper = Wrapper::with_termwidth()
            .subsequent_indent("                    ");

        for msg in receiver {
            let action_string;
            let start;
            action_string = format!("{}", msg);
            start = Local::now();
            println!("{}\n", wrapper.fill(&format!("{} ResolutionManager: Processing {}", start.format("%F %T"), action_string)));

            manager.process_message(msg)?;

            let end = Local::now();
            let time_diff = (end - start).num_milliseconds();
            println!("{}\n", wrapper.fill(&format!("{} ResolutionManager: Completed in {}", start.format("%F %T"), ms_to_string(time_diff))));
            manager.sender.send(Message {
                recipients : vec![],
                sseq : SseqChoice::Main, // Doesn't matter
                action : Action::from(Complete {})
            })?;
        }
        Ok(())
    }

    fn process_message(&mut self, msg : Message) -> Result<(), Box<dyn Error>> {
        match msg.action {
            Action::Construct(a) => self.construct(a)?,
            Action::ConstructJson(a) => self.construct_json(a)?,
            Action::Resolve(a) => self.resolve(a, msg.sseq)?,
            Action::BlockRefresh(_) => self.sender.send(msg)?,
            Action::QueryTable(a) => self.query_table(a, msg.sseq)?,
            _ => {
                // Find a better way to make this work.
                match msg.sseq {
                    SseqChoice::Main => {
                        if let Some(resolution) = &self.resolution {
                            msg.action.act_resolution(resolution)
                        }
                    },
                    SseqChoice::Unit => {
                        if let Some(main_resolution) = &self.resolution {
                            if let Some(resolution) = &main_resolution.borrow().unit_resolution {
                                msg.action.act_resolution(&resolution.upgrade().unwrap());
                            }
                        }
                    }
                }
            }
        };
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
        dir.push("static/modules");

        let bundle = rust_ext::construct(&Config {
             module_paths : vec![dir],
             module_file_name : format!("{}.json", action.module_name),
             algebra_name : action.algebra_name.to_string(),
             max_degree : 0 // This is not used.
        }).unwrap();

        self.process_bundle(bundle);

        Ok(())
    }

    fn process_bundle(&mut self, bundle : AlgebraicObjectsBundle<FiniteModule>) {
        self.is_unit = bundle.module.is_unit();
        if self.is_unit {
            bundle.resolution.borrow_mut().set_unit_resolution(Rc::downgrade(&bundle.resolution));
        } else {
            bundle.resolution.borrow_mut().construct_unit_resolution();
        }
        self.resolution = Some(bundle.resolution);

        if let Some(resolution) = &self.resolution {
            self.setup_callback(&mut resolution.borrow_mut(), SseqChoice::Main);
            if !self.is_unit {
                if let Some(unit_res) = &resolution.borrow().unit_resolution {
                    self.setup_callback(&mut unit_res.upgrade().unwrap().borrow_mut(), SseqChoice::Unit);

                }
            }
        }
   }

    fn resolve(&self, action : Resolve, sseq : SseqChoice) -> Result<(), Box<dyn Error>> {
        let resolution = &self.resolution.as_ref().unwrap();
        let min_degree = match sseq {
            SseqChoice::Main => resolution.borrow().min_degree(),
            SseqChoice::Unit => 0
        };

        let msg = Message {
            recipients : vec![],
            sseq,
            action : Action::from(Resolving {
                p : resolution.borrow().prime(),
                min_degree,
                max_degree : action.max_degree,
                is_unit : self.is_unit
            })
        };
        self.sender.send(msg)?;

        match sseq {
            SseqChoice::Main => resolution.borrow().resolve_through_degree(action.max_degree),
            SseqChoice::Unit => {
                if let Some(r) = &resolution.borrow().unit_resolution {
                    r.upgrade().unwrap().borrow().resolve_through_degree(action.max_degree)
                }
            }
        };

        Ok(())
    }

    fn query_table(&self, action : QueryTable, sseq : SseqChoice) -> Result<(), Box<dyn Error>> {
        if let SseqChoice::Main = sseq {
            let resolution = self.resolution.as_ref().unwrap().borrow();

            let s = action.s;
            let t = action.t;

            let module = resolution.module(s);
            if t < module.min_degree() {
                return Ok(());
            }
            let string = module.generator_list_string(t);
            let msg = Message {
                recipients : vec![],
                sseq : sseq,
                action : Action::from(QueryTableResult { s, t, string })
            };
            self.sender.send(msg)?;
        }
        Ok(())
    }
}

impl ResolutionManager {
    fn setup_callback(&self, resolution : &mut ModuleResolution<FiniteModule>, sseq : SseqChoice) {
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

struct SseqManager {
    sender : Sender,
    sseq : Option<Sseq>,
    unit_sseq : Option<Sseq>
}

impl SseqManager {
    /// Constructs a SseqManager object and waits for messages coming from `receiver`. When the
    /// `receiver` stream ends, the function terminates and returns `()`, dropping the
    /// SseqManager object.
    ///
    /// # Arguments
    ///  * `receiver` - The `Receiver` object to listen commands from.
    ///  * `sender` - The `Sender` object to send messages to.
    fn new(receiver : Receiver, sender : Sender) -> Result<(), Box<dyn Error>> {
        let mut manager = SseqManager {
             sender : sender,
             sseq : None,
             unit_sseq : None
        };

        let wrapper = Wrapper::with_termwidth()
            .subsequent_indent("                    ");

        for msg in receiver {
            let user = match msg.action {
                Action::AddClass(_) => false,
                Action::AddProduct(_) => false,
                Action::Complete(_) => false,
                Action::Resolving(_) => false,
                _ => true
            };
            let action_string;
            let start;
            action_string = format!("{}", msg);
            start = Local::now();
            if user {
                println!("{}\n", wrapper.fill(&format!("{} SseqManager: Processing {}", start.format("%F %T"), action_string)));
            }

            manager.process_message(msg)?;

            if user {
                let end = Local::now();
                let time_diff = (end - start).num_milliseconds();
                println!("{}\n", wrapper.fill(&format!("{} SseqManager: Completed in {}", start.format("%F %T"), ms_to_string(time_diff))));
                manager.sender.send(Message {
                    recipients : vec![],
                    sseq : SseqChoice::Main, // Doesn't matter
                    action : Action::from(Complete {})
                })?;
            }
        }
        Ok(())
    }

    fn process_message(&mut self, msg : Message) -> Result<(), Box<dyn Error>> {
        match msg.action {
            Action::Resolving(_) => self.resolving(msg)?,
            Action::Complete(_) => self.relay(msg)?,
            Action::QueryTableResult(_) => self.relay(msg)?,
            _ => {
                if let Some(sseq) = self.get_sseq(msg.sseq) {
                    msg.action.act_sseq(sseq);
                }
            }
        };
        Ok(())
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

/// The reason the code is structured this way is that messages sent to the WebSocket are blocked
/// until `on_message` returned. Hence we start the ResolutionManager on a separate thread, and
/// when we receive a message, we can let ResolutionManager handle it asynchronously and let
/// `on_message` return as soon as possible.
///
/// We also spawn a separate thread waiting for messages from ResolutionManager, and then relay it
/// to the WebSocket, again, we do this because we don't want anything to be blocking.
pub struct Manager {
    sseq_sender : Sender,
    res_sender : Sender
}

impl Manager {
    fn new<T>(f : T) -> Self where T : Fn(String) -> () + Send + 'static
    {
        let (sseq_sender, sseq_receiver) = mpsc::channel();
        let (server_sender, server_receiver) = mpsc::channel();
        let (res_sender, res_receiver) = mpsc::channel();

        // ResolutionManager thread
        let sender = sseq_sender.clone();
        thread::spawn(move|| {
            match ResolutionManager::new(res_receiver, sender) {
                Ok(_) => (),
                Err(e) => eprintln!("Error in ResolutionManager: {}", e)
            }
        });

        // SseqManager thread
        let sender = server_sender.clone();
        thread::spawn(move|| {
            match SseqManager::new(sseq_receiver, sender) {
                Ok(_) => (),
                Err(e) => eprintln!("Error in ResolutionManager: {}", e)
            }
        });

        // Server thread
        thread::spawn(move|| {
            for msg in server_receiver {
                f(serde_json::to_string(&msg).unwrap());
            }
        });

        Manager {
            sseq_sender,
            res_sender
        }
    }

    fn on_message(&self, m : &str) {
        let msg : Result<Message, serde_json::Error> = serde_json::from_str(m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();

        for recipient in &msg.recipients {
            match recipient {
                Recipient::Sseq => {
                    match self.sseq_sender.send(msg.clone()) {
                        Ok(_) => (),
                        Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e)
                    }
                },
                Recipient::Resolver => {
                    match self.res_sender.send(msg.clone()) {
                        Ok(_) => (),
                        Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e)
                    }
                }
            }
        }
    }
}

/// The server implements the `ws::Handler` trait. It doesn't really do much. When we receive a
/// request, it is either looking for some static files, as specified in `FILE_LIST`, or it is
/// WebSocket message. If it is the former, we return the file. If it is the latter, we parse it
/// into a string and pass it on to Manager.
pub struct Server {
    manager : Option<Manager>,
    out : Option<WsSender>
}

impl Handler for Server {
    fn on_request(&mut self, req: &Request) -> WsResult<(Response)> {
         match req.resource() {
             "/ws" => Response::from_request(req),
             _ => self.serve_files(req.resource())
         }
    }

    fn on_message(&mut self, m : ws::Message) -> WsResult<()> {
        let m = m.into_text()?;
        if self.manager.is_none() {
            let out = self.out.take().unwrap();
            self.manager = Some(Manager::new(move |s| out.send(s).unwrap()));
        }

        if let Some(manager) = &self.manager {
            manager.on_message(&m);
        }
        Ok(())
    }
}

impl Server {
    pub fn new(out : WsSender) -> Self {
        Server {
            manager : None,
            out : Some(out)
        }
    }

    pub fn serve_files(&self, request_path: &str) -> WsResult<(Response)> {
        println!("Request path: {}", request_path);
        let request_path = request_path.split("?").collect::<Vec<&str>>()[0]; // Ignore ?...
        let mut dir = std::env::current_exe().unwrap();
        dir.pop(); dir.pop(); dir.pop();
        dir.push("ext-websocket/interface");

        for (path, file, mime) in &FILE_LIST {
            if request_path == *path {
                dir.push(file);
                let contents = fs::read(dir)?;
                let mut response = Response::new(200, "OK", contents);
                let headers = response.headers_mut();
                headers.push(("Content-type".to_string(), (*mime).into()));
                return Ok(response);
            }
        }
        Ok(Response::new(404, "Not Found", b"404 - Not Found".to_vec()))
    }
}
