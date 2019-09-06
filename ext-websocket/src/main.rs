use ws::listen;
use ext_websocket::Server;

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
