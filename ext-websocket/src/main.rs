extern crate ws;
extern crate rust_ext;
#[macro_use]
extern crate serde_json;

use rust_ext::{Config, AlgebraicObjectsBundle};
use rust_ext::module::{Module, FiniteModule};
use std::{fs, thread};
use std::sync::mpsc;
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
/// The main function is `ResolutionManager::new`. This function does not return a ResolutionManger
/// object. Instead, the function produces a ResolutionManager object and waits for commands issued
/// by the user. The actions of the command will involve manipulating the ResolutionManger.
/// However, not everything interesting can be found inside the struct itself. Instead, some
/// variables are simply local to the function `ResolutionManager::new`. What goes into the struct
/// and what stays a local variable is simply a matter of convenience.
struct ResolutionManager {
    sender : mpsc::Sender<String>,
    bundle : Option<AlgebraicObjectsBundle<FiniteModule>>
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
             bundle : None,
        };

        for msg in receiver {
            let json : Value = serde_json::from_str(&msg).unwrap();// Implement proper error handling.
            match json["command"].as_str() {
                Some("resolve") => manager.resolve(json)?,
                Some("resolve_json") => manager.resolve_json(json)?,
                _ => {println!("Ignoring message: {:#}", json);}
            };
        }
        Ok(())
    }

    /// Resolves a module defined by a json object. The result is stored in `self.bundle`.
    fn resolve_json(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let algebra_name = json["algebra"].as_str().unwrap().to_string();
        let max_degree = json["maxDegree"].as_i64().unwrap() as i32;
        let json_data = serde_json::from_str(json["data"].as_str().unwrap())?;

        self.bundle = rust_ext::construct_from_json(json_data, algebra_name, max_degree).ok();

        self.resolve_bundle(max_degree)
    }

    /// Resolves a module specified by `json`. The result is stored in `self.bundle`.
    fn resolve(&mut self, json : Value) -> Result<(), Box<dyn Error>> {
        let module_name = json["module"].as_str().unwrap(); // Need to handle error
        let algebra_name = json["algebra"].as_str().unwrap();
        let max_degree = json["maxDegree"].as_i64().unwrap() as i32;
        let mut dir = std::env::current_dir()?;
        dir.push("modules");

        self.bundle = rust_ext::construct(&Config {
             module_paths : vec![dir],
             module_file_name : format!("{}.json", module_name),
             algebra_name : algebra_name.to_string(),
             max_degree : max_degree
        }).ok();

        self.resolve_bundle(max_degree)
    }

    /// If `self.bundle` is set, resolve the resolution in the bundle up to degree `max_degree`. If
    /// `self.bundle` is not set, the function does nothing.
    fn resolve_bundle(&mut self, max_degree : i32) -> Result<(), Box<dyn Error>> {
        if let Some(bundle) = &self.bundle {
            let data = json!(
                {
                    "command" : "resolving",
                    "minDegree" : (*bundle.module).get_min_degree(),
                    "maxDegree" : max_degree
                });

            self.sender.send(data.to_string())?;

            let sender = self.sender.clone();
            let add_class = move |s: u32, t: i32, _name: &str| {
                let data = json!(
                    {
                        "command": "addClass",
                        "s": s,
                        "t": t
                    });
                match sender.send(data.to_string()) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send class: {}", e)
                };
            };

            let sender = self.sender.clone();
            let add_structline = move |name : &str, source_s: u32, source_t: i32, source_idx: usize, target_s : u32, target_t : i32, target_idx : usize| {
                let data = json!(
                    {
                        "command": "addStructline",
                        "mult": name,
                        "source": {
                            "s": source_s,
                            "t": source_t,
                            "idx": source_idx
                        },
                        "target": {
                            "s": target_s,
                            "t": target_t,
                            "idx": target_idx
                        }
                    });
                match sender.send(data.to_string()) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send class: {}", e)
                };
            };

            let mut resolution = bundle.resolution.borrow_mut();
            resolution.add_class = Some(Box::new(add_class));
            resolution.add_structline = Some(Box::new(add_structline));
            resolution.resolve_through_degree(max_degree);

            let data = json!({ "command": "complete" });
            self.sender.send(data.to_string())?;
        }
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
        for (path, file, mime) in &FILE_LIST {
            if request_path == *path {
                let contents = fs::read(format!("interface/{}", file))?;
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
    listen("127.0.0.1:8080", |out| Server::new(out)).unwrap();
}
