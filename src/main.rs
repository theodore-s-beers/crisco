#![warn(clippy::pedantic, clippy::nursery)]

use scratch_server::{extract_url, parse_req, shorten_url};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8887")?;
    println!("Listening on http://127.0.0.1:8887");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => eprintln!("Connection failed: {e}"),
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    let request = parse_req(&mut stream);

    match request {
        Ok(req) => {
            println!("Received {} request for path {}", req.method, req.path);
            let body = if req.method == "POST" {
                if let Some(url) = extract_url(&req.body) {
                    shorten_url(url)
                } else {
                    req.body
                }
            } else {
                format!("Path requested: {}", req.path)
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
        Err(e) => {
            eprintln!("{e}");
            let body = format!("{e}");

            if let scratch_server::ReqParseError::IoError(_) = e {
                let response = format!(
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            } else {
                let response = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        }
    }
}
