extern crate ws;
extern crate rust_ext;
#[macro_use]
extern crate serde_json;

mod sseq;

use sseq::Sseq;
use rust_ext::Config;
use rust_ext::module::{Module, FiniteModule};
use rust_ext::resolution::{ModuleResolution};
use rust_ext::chain_complex::ChainComplex;
use rust_ext::fp_vector::FpVector;

use std::{fs, thread};
use std::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;
use std::error::Error;
use serde_json::value::Value;

use ws::{listen, Handler, Message, Request, Response, Sender};
use ws::Result as WsResult;

/// List of files that our webserver will serve to the user
const FILE_LIST : [(&str, &str, &[u8]); 7] = [
    ("/", "index.html", b"text/html"),
    ("/index.html", "index.html", b"text/html"),
    ("/index.js", "index.js", b"text/javascript"),
    ("/display.js", "display.js", b"text/javascript"),
    ("/sseq.js", "sseq.js", b"text/javascript"),
    ("/index.css", "index.css", b"text/css"),
    ("/bundle.js", "bundle.js", b"text/javascript")];

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
    sender : mpsc::Sender<Value>,
    resolution : Option<Rc<RefCell<ModuleResolution<FiniteModule>>>>
}

impl ResolutionManager {
    /// Constructs a ResolutionManager object and waits for messages coming from `receiver`. The
    /// results of calculations are relayed back via `sender` in the form of stringified JSON. When
    /// the `receiver` stream ends, the function terminates and returns `()`, dropping the
    /// ResolutionManager object.
    ///
    /// # Arguments
    ///  * `receiver` - The `mpsc::Receiver` object to listen commands from.
    ///  * `sender` - The `mpsc::Sender` object to send messages to.
    fn new(receiver : mpsc::Receiver<Value>, sender : mpsc::Sender<Value>) -> Result<(), Box<dyn Error>> {
        let mut manager = ResolutionManager {
             sender : sender,
             resolution : None,
        };

        for json in receiver {
            match json["command"].as_str() {
                Some("resolve") => manager.construct_resolution(json)?,
                Some("resolve_json") => manager.construct_resolution_json(json)?,
                Some("resolve_further") => manager.resolve_further(json)?,
                Some("add_product") => manager.add_product(json)?,
                Some("query_table") => manager.query_table(json)?,
                _ => {println!("ResolutionManager ignoring message:\n{:#}", json);}
            };
        }
        Ok(())
    }

    /// Resolve existing resolution to a larger degree
    fn add_product(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let s = json["s"].as_u64().unwrap() as u32;
        let t = json["t"].as_i64().unwrap() as i32;
        let idx = json["idx"].as_u64().unwrap() as usize;
        let name = json["name"].as_str().unwrap().to_string();

        self.resolution().borrow_mut().add_product(s, t, idx, name);
        self.resolution().borrow().catch_up_products();
        Ok(())
    }

    /// Resolve existing resolution to a larger degree
    fn resolve_further(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let max_degree = json["maxDegree"].as_i64().unwrap() as i32;
        match json["origin"].as_str() {
            Some("main") => self.resolve(max_degree)?,
            Some("unit") => self.resolve_unit(max_degree)?,
            e => { eprintln!("Origin not recognized: {:?}. Unable to resolve further", e) }
        }
        Ok(())
    }

    /// Resolves a module defined by a json object. The result is stored in `self.bundle`.
    fn construct_resolution_json(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let algebra_name = json["algebra"].as_str().unwrap().to_string();
        let max_degree = json["maxDegree"].as_i64().unwrap() as i32;
        let json_data = serde_json::from_str(json["data"].as_str().unwrap())?;

        let bundle = rust_ext::construct_from_json(json_data, algebra_name).unwrap();

        bundle.resolution.borrow_mut().construct_unit_resolution();
        self.resolution = Some(bundle.resolution);

        self.setup_callback(&self.resolution, "main");
        self.setup_callback(&self.resolution().borrow().unit_resolution, "unit");
        self.resolve(max_degree)
    }

    /// Resolves a module specified by `json`. The result is stored in `self.bundle`.
    fn construct_resolution(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let module_name = json["module"].as_str().unwrap(); // Need to handle error
        let algebra_name = json["algebra"].as_str().unwrap();
        let max_degree = json["maxDegree"].as_i64().unwrap() as i32;
        let mut dir = std::env::current_exe().unwrap();
        dir.pop(); dir.pop(); dir.pop();
        dir.push("static/modules");

        let bundle = rust_ext::construct(&Config {
             module_paths : vec![dir],
             module_file_name : format!("{}.json", module_name),
             algebra_name : algebra_name.to_string(),
             max_degree : max_degree
        }).unwrap();

        bundle.resolution.borrow_mut().construct_unit_resolution();
        self.resolution = Some(bundle.resolution);

        self.setup_callback(&self.resolution, "main");
        self.setup_callback(&self.resolution().borrow().unit_resolution, "unit");
        self.resolve(max_degree)
    }

    fn query_table(&self, json : Value) -> Result<(), Box<dyn Error>> {
        let s = json["s"].as_u64().unwrap() as u32;
        let t = json["t"].as_i64().unwrap() as i32;

        let resolution = self.resolution().borrow();
        let module = resolution.get_module(s);
        if t < module.get_min_degree() {
            return Ok(());
        }
        let string = module.generator_list_string(t);
        let data = json!(
            {
                "command": "queryTableResult",
                "s": s,
                "t": t,
                "string": string
            });
        self.sender.send(data)?;
        Ok(())
    }
}

impl ResolutionManager {
    fn resolution(&self) -> &Rc<RefCell<ModuleResolution<FiniteModule>>> {
        &self.resolution.as_ref().unwrap()
    }

    fn setup_callback(&self, resolution : &Option<Rc<RefCell<ModuleResolution<FiniteModule>>>>, sseq_name : &'static str) {

        let sender = self.sender.clone();
        let add_class = move |s: u32, t: i32, num_gen: usize| {
            let data = json!(
                {
                    "command": "addClass",
                    "origin": sseq_name,
                    "s": s,
                    "t": t,
                    "num": num_gen
                });
            match sender.send(data) {
                Ok(_) => (),
                Err(e) => {eprintln!("Failed to send class: {}", e); panic!("")}
            };
        };

        let sender = self.sender.clone();
        let add_structline = move |name : &str, source_s: u32, source_t: i32, target_s : u32, target_t : i32, left : bool, products : Vec<Vec<u32>>| {
            let mult_s = target_s - source_s;
            let mult_t = target_t - source_t;

            let data = json!(
                {
                    "command": "addStructline",
                    "origin": sseq_name,
                    "name": name,
                    "source_s": source_s,
                    "source_t": source_t,
                    "mult_s": mult_s,
                    "mult_t": mult_t,
                    "left": left,
                    "products": products
                });

            match sender.send(data) {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to send class: {}", e)
            };
        };

        let mut resolution = resolution.as_ref().unwrap().borrow_mut();
        resolution.add_class = Some(Box::new(add_class));
        resolution.add_structline = Some(Box::new(add_structline));
    }

    fn resolve_unit(&self, max_degree : i32) -> Result<(), Box<dyn Error>> {
        let unit_resolution_option = &self.resolution().borrow().unit_resolution;
        if let Some(unit_resolution) = unit_resolution_option {
            unit_resolution.borrow().resolve_through_degree(max_degree);
        }
        Ok(())
    }

    fn resolve(&self, max_degree : i32) -> Result<(), Box<dyn Error>> {
        if let Some(resolution) = &self.resolution {
            let data = json!(
                {
                    "command" : "resolving",
                    "p" : resolution.borrow().prime(),
                    "minDegree" : resolution.borrow().get_min_degree(),
                    "maxDegree" : max_degree
                });
            self.sender.send(data)?;

            resolution.borrow().resolve_through_degree(max_degree);
        }

        let data = json!({ "command": "complete" });
        self.sender.send(data)?;
        Ok(())
    }
}

struct SseqManager {
    sender : mpsc::Sender<Value>,
    sseq : Option<Sseq>,
    unit_sseq : Option<Sseq>
}

impl SseqManager {
    /// Constructs a SseqManager object and waits for messages coming from `receiver`. When the
    /// `receiver` stream ends, the function terminates and returns `()`, dropping the
    /// SseqManager object.
    ///
    /// # Arguments
    ///  * `receiver` - The `mpsc::Receiver` object to listen commands from.
    ///  * `sender` - The `mpsc::Sender` object to send messages to.
    fn new(receiver : mpsc::Receiver<Value>, sender : mpsc::Sender<Value>) -> Result<(), Box<dyn Error>> {
        let mut manager = SseqManager {
             sender : sender,
             sseq : None,
             unit_sseq : None,
        };

        for json in receiver {
            match json["command"].as_str() {
                Some("resolving") => manager.resolving(json)?,
                Some("complete") => manager.relay(json)?,
                Some("queryTableResult") => manager.relay(json)?,
                Some("add_differential") => manager.add_differential(json)?,
                Some("add_permanent") => manager.add_permanent(json)?,
                Some("addClass") => manager.add_class(json)?,
                Some("addStructline") => manager.add_structline(json)?,
                _ => {println!("SseqManager ignoring message:\n{:#}", json);}
            };
        }
        Ok(())
    }

    fn resolving(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let p = json["p"].as_u64().unwrap() as u32;
        let min_degree = json["minDegree"].as_i64().unwrap() as i32;

        if self.sseq.is_none() {
            let sender = self.sender.clone();
            self.sseq = Some(Sseq::new(p, "main".to_string(), min_degree, 0, Some(sender)));

            let sender = self.sender.clone();
            self.unit_sseq = Some(Sseq::new(p, "unit".to_string(), 0, 0, Some(sender)));
        }

        self.relay(json)
    }

    fn get_sseq(&mut self, name : Option<&str>) -> Option<&mut Sseq> {
        match name {
            Some("main") => self.sseq.as_mut(),
            Some("unit") => self.unit_sseq.as_mut(),
            _ => { eprintln!("Unknown spectral sequence origin: {:?}", name); None }
        }
    }

    fn add_permanent(&mut self, mut json : Value) -> Result<(), Box<dyn Error>> {
        let x = json["x"].as_i64().unwrap() as i32;
        let y = json["y"].as_i64().unwrap() as i32;
        let class : Vec<u32> = serde_json::from_value(json["class"].take()).unwrap();

        let origin = json["origin"].as_str();

        if let Some(sseq) = self.get_sseq(origin) {
            sseq.add_permanent_class_propagate(x, y, &FpVector::from_vec(sseq.p, &class), 0);
//            sseq.add_permanent_class(x, y, &FpVector::from_vec(sseq.p, &class));
        }
        Ok(())
    }

    fn add_differential(&mut self, mut json : Value) -> Result<(), Box<dyn Error>> {
        let x = json["x"].as_i64().unwrap() as i32;
        let y = json["y"].as_i64().unwrap() as i32;
        let r = json["r"].as_i64().unwrap() as i32;
        let source : Vec<u32> = serde_json::from_value(json["source"].take()).unwrap();
        let target : Vec<u32> = serde_json::from_value(json["target"].take()).unwrap();

        let origin = json["origin"].as_str();

        if let Some(sseq) = self.get_sseq(origin) {
            sseq.add_differential_propagate(r, x, y, &FpVector::from_vec(sseq.p, &source), &FpVector::from_vec(sseq.p, &target), 0);
        }
        Ok(())
    }

    fn add_class(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let s = json["s"].as_i64().unwrap() as i32;
        let t = json["t"].as_i64().unwrap() as i32;
        let num = json["num"].as_u64().unwrap() as usize;
        let origin = json["origin"].as_str();

        let x = t - s;
        let y = s;

        if let Some(sseq) = self.get_sseq(origin) {
            sseq.set_class(x, y, num);
        }
        Ok(())
    }

    fn add_structline(&mut self, mut json : Value) -> Result<(), Box<dyn Error>> {
        let mult_s = json["mult_s"].as_i64().unwrap() as i32;
        let mult_t = json["mult_t"].as_i64().unwrap() as i32;
        let mult_x = mult_t - mult_s;
        let mult_y = mult_s;

        let source_s = json["source_s"].as_i64().unwrap() as i32;
        let source_t = json["source_t"].as_i64().unwrap() as i32;
        let source_x = source_t - source_s;
        let source_y = source_s;

        let mut product : Vec<Vec<u32>> = serde_json::from_value(json["products"].take()).unwrap();

        let name = json["name"].as_str().unwrap();

        // Left is a boolean telling us whether we multiply on the left or right. I don't
        // really know what this means with all the duals all around. By convention, compositions with
        // maps S^k -> S^l are multiplication on the left; compositions with self maps are
        // multiplication on the right.
        let left = json["left"].as_bool().unwrap();

        let origin = json["origin"].as_str();

        if let Some(sseq) = self.get_sseq(origin) {
            if (left && mult_s * source_t % 2 != 0) ||
               (!left && mult_t * source_s % 2 != 0) {
                for a in 0 .. product.len() {
                    for b in 0 .. product[a].len() {
                        product[a][b] = ((sseq.p - 1) * product[a][b]) % sseq.p;
                    }
                }
            }
            sseq.add_product(&name, source_x, source_y, mult_x, mult_y, left, product);
        }
        Ok(())
    }

    fn relay(&self, msg : Value) -> Result<(), Box<dyn Error>> {
        self.sender.send(msg)?;
        Ok(())
    }
}

/// The server implements the `ws::Handler` trait. It doesn't really do much. When we receive a
/// request, it is either looking for some static files, as specified in `FILE_LIST`, or it is
/// WebSocket message. If it is the former, we return the file. If it is the latter, we parse it
/// into a string and pass it on to ResolutionManager.
///
/// The reason the code is structured this way is that messages sent to the WebSocket are blocked
/// until `on_message` returned. Hence we start the ResolutionManager on a separate thread, and
/// when we receive a message, we can let ResolutionManager handle it asynchronously and let
/// `on_message` return as soon as possible.
///
/// We also spawn a separate thread waiting for messages from ResolutionManager, and then relay it
/// to the WebSocket, again, we do this because we don't want anything to be blocking.
struct Server {
    sseq_sender : mpsc::Sender<Value>,
    res_sender : mpsc::Sender<Value>,
}

impl Handler for Server {
    fn on_request(&mut self, req: &Request) -> WsResult<(Response)> {
         match req.resource() {
             "/ws" => Response::from_request(req),
             _ => self.serve_files(req.resource())
         }
    }

    fn on_message(&mut self, msg : Message) -> WsResult<()> {
        let msg = msg.into_text()?;
        let json : Value = serde_json::from_str(&msg).unwrap();
        println!("Received message:\n{}", serde_json::to_string_pretty(&json).unwrap());

        let recipient = json["recipient"].as_str();
        match recipient {
            Some("resolver") => {
                match self.res_sender.send(json) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e)
                }
            },
            Some("sseq") => {
                match self.sseq_sender.send(json) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e)
                }
            },
            _ => eprintln!("Unknown target: {:?}", recipient)
        }
        Ok(())
    }
}

impl Server {
    fn new(out : Sender) -> Self {
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
                out.send(msg.to_string()).unwrap();
            }
        });

        Server {
            sseq_sender,
            res_sender
        }
    }

    fn serve_files(&self, request_path: &str) -> WsResult<(Response)> {
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

fn main() {
    let args : Vec<String> = std::env::args().collect();
    let mut port = "8080";
    if args.len() > 1 {
        match args[1].as_ref() {
            "--help" => { println!("Usage: ext-websocket [PORT]"); std::process::exit(0) },
            _ => port = &args[1]
        }
    };

    println!("Opening websocket on 127.0.0.1:{}", port);
    match listen(&format!("127.0.0.1:{}", port), |out| Server::new(out)) {
        Ok(_) => (),
        Err(e) => eprintln!("Unable to open websocket: {}", e)
    }
}
