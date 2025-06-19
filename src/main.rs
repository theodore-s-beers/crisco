#![warn(clippy::pedantic, clippy::nursery)]

use scratch_server::{handle_err, handle_get, handle_post, parse_req};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};

fn main() -> std::io::Result<()> {
    let mut store: HashMap<String, String> = HashMap::new();

    let listener = TcpListener::bind("127.0.0.1:8887")?;
    println!("Listening on http://127.0.0.1:8887");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream, &mut store),
            Err(e) => eprintln!("Connection failed: {e}"),
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream, store: &mut HashMap<String, String>) {
    let req = parse_req(&mut stream);

    match req {
        Ok(req) => {
            println!("Received {} request for path {}", req.method, req.path);

            if req.method == "POST" {
                handle_post(stream, store, &req);
            } else {
                handle_get(stream, store, &req);
            }
        }

        Err(e) => {
            eprintln!("{e}");
            handle_err(stream, &e);
        }
    }
}
