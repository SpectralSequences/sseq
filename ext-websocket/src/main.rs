extern crate ws;
extern crate rust_ext;
#[macro_use]
extern crate serde_json;

use rust_ext::Config;
use rust_ext::module::{Module, FiniteModule};
use rust_ext::resolution::{ModuleResolution};
use rust_ext::chain_complex::ChainComplex;

use std::{fs, thread};
use std::sync::mpsc;
use std::cell::RefCell;
use std::rc::Rc;
use std::error::Error;
use serde_json::value::Value;

use ws::{listen, Handler, Message, Request, Response, Sender};
use ws::Result as WsResult;

/// List of files that our webserver will serve to the user
const FILE_LIST : [(&str, &str, &[u8]); 6] = [
    ("/", "index.html", b"text/html"),
    ("/index.html", "index.html", b"text/html"),
    ("/index.js", "index.js", b"text/javascript"),
    ("/display.js", "display.js", b"text/javascript"),
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
    sender : mpsc::Sender<String>,
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
    fn new(receiver : mpsc::Receiver<String>, sender : mpsc::Sender<String>) -> Result<(), Box<dyn Error>> {
        let mut manager = ResolutionManager {
             sender : sender,
             resolution : None,
        };

        for msg in receiver {
            let json : Value = serde_json::from_str(&msg).unwrap();// Implement proper error handling.
            println!("Received message:\n{}", serde_json::to_string_pretty(&json)?);
            match json["command"].as_str() {
                Some("resolve") => manager.construct_resolution(json)?,
                Some("resolve_json") => manager.construct_resolution_json(json)?,
                Some("resolve_further") => manager.resolve_further(json)?,
                Some("resolve_unit") => manager.resolve_unit(json)?,
                Some("add_product") => manager.add_product(json)?,
                Some("query_table") => manager.query_table(json)?,
                _ => {println!("Ignoring message:\n{:#}", json);}
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
        self.resolve(max_degree)
    }

    fn resolve_unit(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let max_degree = json["maxDegree"].as_i64().unwrap() as i32;
        let unit_resolution_option = &self.resolution().borrow().unit_resolution;
        if let Some(unit_resolution) = unit_resolution_option {
            unit_resolution.borrow().resolve_through_degree(max_degree);
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

        self.setup_callback(&self.resolution, "");
        self.setup_callback(&self.resolution().borrow().unit_resolution, "Unit");
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

        self.setup_callback(&self.resolution, "");
        self.setup_callback(&self.resolution().borrow().unit_resolution, "Unit");
        self.resolve(max_degree)
    }

    fn query_table(&self, json : Value) -> Result<(), Box<dyn Error>> {
        let s = json["s"].as_u64().unwrap() as u32;
        let t = json["t"].as_i64().unwrap() as i32;

        let resolution = self.resolution().borrow();
        let module = resolution.get_module(s);
        let string = module.generator_list_string(t);
        let data = json!(
            {
                "command": "tableResult",
                "s": s,
                "t": t,
                "string": string
            });
        self.sender.send(data.to_string())?;
        Ok(())
    }
}

impl ResolutionManager {
    fn resolution(&self) -> &Rc<RefCell<ModuleResolution<FiniteModule>>> {
        &self.resolution.as_ref().unwrap()
    }

    fn setup_callback(&self, resolution : &Option<Rc<RefCell<ModuleResolution<FiniteModule>>>>, postfix : &'static str) {

        let sender = self.sender.clone();
        let add_class = move |s: u32, t: i32, num_gen: usize| {
            let data = json!(
                {
                    "command": format!("addClass{}", postfix),
                    "s": s,
                    "t": t
                });
            for _ in 0 .. num_gen {
                match sender.send(data.to_string()) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send class: {}", e)
                };
            }
        };

        let sender = self.sender.clone();
        let add_structline = move |name : &str, source_s: u32, source_t: i32, target_s : u32, target_t : i32, products : Vec<Vec<u32>>| {
            for i in 0 .. products.len() {
                for j in 0 .. products[i].len() {
                    if products[i][j] != 0 {
                        let data = json!(
                            {
                                "command": format!("addStructline{}", postfix),
                                "mult": name,
                                "source": {
                                    "s": source_s,
                                    "t": source_t,
                                    "idx": i
                                },
                                "target": {
                                    "s": target_s,
                                    "t": target_t,
                                    "idx": j
                                }
                            });
                        match sender.send(data.to_string()) {
                            Ok(_) => (),
                            Err(e) => eprintln!("Failed to send class: {}", e)
                        };
                    }
                }
            }
        };

        let mut resolution = resolution.as_ref().unwrap().borrow_mut();
        resolution.add_class = Some(Box::new(add_class));
        resolution.add_structline = Some(Box::new(add_structline));
    }

    fn resolve(&self, max_degree : i32) -> Result<(), Box<dyn Error>> {
        let data = json!(
            {
                "command" : "resolving",
                "minDegree" : self.resolution.as_ref().unwrap().borrow().get_min_degree(),
                "maxDegree" : max_degree
            });
        self.sender.send(data.to_string())?;

        if let Some(resolution) = &self.resolution {
            resolution.borrow().resolve_through_degree(max_degree);
        }

        let data = json!({ "command": "complete" });
        self.sender.send(data.to_string())?;
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
    sender : mpsc::Sender<String>,
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
        match self.sender.send(msg) {
            Ok(_) => (),
            Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e)
        }
        Ok(())
    }
}

// When Server is dropped:
//  - server_sender is dropped. This causes manager_receiver to terminate.
//  - When manager_receiver is terminated, ResolutionManager::new() ends and manger is dropped. The
//  manager thread ends.
//  - When manager is dropped, manager_sender is dropped. So server_receiver terminates, and the
//  messager thread ends.
impl Server {
    fn new(out : Sender) -> Self {
        let (manager_sender, server_receiver) = mpsc::channel();
        let (server_sender, manager_receiver) = mpsc::channel();

        // Manager thread
        thread::spawn(move|| {
            match ResolutionManager::new(manager_receiver, manager_sender) {
                Ok(_) => (),
                Err(e) => eprintln!("Error in ResolutionManager: {}", e)
            }
        });

        // Server thread
        thread::spawn(move|| {
            for msg in server_receiver {
                out.send(msg).unwrap();
            }
        });

        Server {
            sender: server_sender
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
