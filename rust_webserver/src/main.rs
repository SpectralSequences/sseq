use crossbeam_channel::unbounded;
use std::{fs, thread};
use textwrap::Wrapper;
use time::OffsetDateTime;
use ws::{listen, Handler, Request, Response, Result as WsResult, Sender as WsSender};

use rust_webserver::actions::*;
use rust_webserver::managers::*;
use rust_webserver::Sender;

/// List of files that our webserver will serve to the user
const FILE_LIST: [(&str, &str, &[u8]); 16] = [
    ("/", "index.html", b"text/html"),
    ("/index.html", "index.html", b"text/html"),
    ("/constructor.html", "constructor.html", b"text/html"),
    ("/index.js", "index.js", b"text/javascript"),
    ("/constructor.js", "constructor.js", b"text/javascript"),
    ("/mousetrap.min.js", "mousetrap.min.js", b"text/javascript"),
    ("/canvas2svg.js", "canvas2svg.js", b"text/javascript"),
    ("/display.js", "display.js", b"text/javascript"),
    ("/pako.js", "pako.js", b"text/javascript"),
    ("/utils.js", "utils.js", b"text/javascript"),
    ("/tooltip.js", "tooltip.js", b"text/javascript"),
    ("/panels.js", "panels.js", b"text/javascript"),
    ("/sseq.js", "sseq.js", b"text/javascript"),
    ("/index.css", "index.css", b"text/css"),
    ("/common.css", "common.css", b"text/css"),
    ("/bundle.js", "bundle.js", b"text/javascript"),
];

fn ms_to_string(time: i128) -> String {
    if time < 1000 {
        format!("{}ms", time)
    } else if time < 10000 {
        format!("{}.{}s", time / 1000, time % 1000)
    } else {
        format!("{}s", time / 1000)
    }
}

fn print_time(time: OffsetDateTime) -> String {
    format!("{}:{}:{}", time.hour(), time.minute(), time.hour())
}

/// The reason the code is structured this way is that messages sent to the WebSocket are blocked
/// until `on_message` returned. Hence we start the ResolutionManager on a separate thread, and
/// when we receive a message, we can let ResolutionManager handle it asynchronously and let
/// `on_message` return as soon as possible.
///
/// We also spawn a separate thread waiting for messages from ResolutionManager, and then relay it
/// to the WebSocket, again, we do this because we don't want anything to be blocking.
pub struct Manager {
    sseq_sender: Sender,
    res_sender: Sender,
}

impl Manager {
    fn new<T>(f: T) -> Self
    where
        T: Fn(String) + Send + 'static,
    {
        let (sseq_sender, sseq_receiver) = unbounded();
        let (server_sender, server_receiver) = unbounded();
        let (res_sender, res_receiver) = unbounded();

        // ResolutionManager thread
        let sender = sseq_sender.clone();
        thread::spawn(move || {
            let mut resolution_manager = ResolutionManager::new(sender);

            let wrapper = Wrapper::with_termwidth().subsequent_indent("                    ");

            for msg in res_receiver {
                let action_string = format!("{}", msg);
                let start = OffsetDateTime::now_utc();
                println!(
                    "{}\n",
                    wrapper.fill(&format!(
                        "{} ResolutionManager: Processing {}",
                        print_time(start),
                        action_string
                    ))
                );

                resolution_manager.process_message(msg).unwrap();

                let end = OffsetDateTime::now_utc();
                let time_diff = (end - start).whole_milliseconds();
                println!(
                    "{}\n",
                    wrapper.fill(&format!(
                        "{} ResolutionManager: Completed in {}",
                        print_time(end),
                        ms_to_string(time_diff)
                    ))
                );
            }
        });

        // SseqManager thread
        let sender = server_sender;
        thread::spawn(move || {
            let mut sseq_manager = SseqManager::new(sender);

            let wrapper = Wrapper::with_termwidth().subsequent_indent("                    ");

            for msg in sseq_receiver {
                let action_string = format!("{}", msg);
                let user = SseqManager::is_user(&msg.action);
                let start = OffsetDateTime::now_utc();

                if user {
                    println!(
                        "{}\n",
                        wrapper.fill(&format!(
                            "{} SseqManager: Processing {}",
                            print_time(start),
                            action_string
                        ))
                    );
                }

                sseq_manager.process_message(msg).unwrap();

                if user {
                    let end = OffsetDateTime::now_utc();
                    let time_diff = (end - start).whole_milliseconds();
                    println!(
                        "{}\n",
                        wrapper.fill(&format!(
                            "{} SseqManager: Completed in {}",
                            print_time(end),
                            ms_to_string(time_diff)
                        ))
                    );
                }
            }
        });

        // Server thread
        thread::spawn(move || {
            for msg in server_receiver {
                f(serde_json::to_string(&msg).unwrap());
            }
        });

        Manager {
            sseq_sender,
            res_sender,
        }
    }

    fn on_message(&self, m: &str) {
        let msg: Result<Message, serde_json::Error> = serde_json::from_str(m);
        if msg.is_err() {
            println!("Unable to understand message:\n{}", m);
            println!("Error: {:?}", msg);
        }

        let msg = msg.unwrap();

        for recipient in &msg.recipients {
            match recipient {
                Recipient::Sseq => match self.sseq_sender.send(msg.clone()) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e),
                },
                Recipient::Resolver => match self.res_sender.send(msg.clone()) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to send message to ResolutionManager: {}", e),
                },
            }
        }
    }
}

/// The server implements the `ws::Handler` trait. It doesn't really do much. When we receive a
/// request, it is either looking for some static files, as specified in `FILE_LIST`, or it is
/// WebSocket message. If it is the former, we return the file. If it is the latter, we parse it
/// into a string and pass it on to Manager.
pub struct Server {
    manager: Option<Manager>,
    out: Option<WsSender>,
}

impl Handler for Server {
    fn on_request(&mut self, req: &Request) -> WsResult<Response> {
        match req.resource() {
            "/ws" => Response::from_request(req),
            _ => self.serve_files(req.resource()),
        }
    }

    fn on_message(&mut self, m: ws::Message) -> WsResult<()> {
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
    pub fn new(out: WsSender) -> Self {
        Server {
            manager: None,
            out: Some(out),
        }
    }

    pub fn serve_files(&self, request_path: &str) -> WsResult<Response> {
        println!("Request path: {}", request_path);
        let request_path = request_path.split('?').collect::<Vec<&str>>()[0]; // Ignore ?...
        let mut dir = std::env::current_exe().unwrap();
        dir.pop();
        dir.pop();
        dir.pop();
        dir.push("interface");

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
    let args: Vec<String> = std::env::args().collect();
    let mut port = "8080";
    if args.len() > 1 {
        match args[1].as_ref() {
            "--help" => {
                println!("Usage: ext-websocket [PORT]");
                std::process::exit(0)
            }
            _ => port = &args[1],
        }
    };

    println!("Opening websocket on 127.0.0.1:{}", port);
    match listen(&format!("127.0.0.1:{}", port), Server::new) {
        Ok(_) => (),
        Err(e) => eprintln!("Unable to open websocket: {}", e),
    }
}
