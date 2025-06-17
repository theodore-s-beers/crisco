#![warn(clippy::pedantic, clippy::nursery)]

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => eprintln!("Connection failed: {e}"),
        }
    }

    Ok(())
}

fn extract_url(request: &str) -> Option<&str> {
    let first_line = request.lines().next()?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        Some(parts[1])
    } else {
        None
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0u8; 1024];

    if stream.read(&mut buffer).is_ok() {
        let request = String::from_utf8_lossy(&buffer);
        println!("Received request:\n{request}");

        let url = extract_url(&request).unwrap_or("/");
        let body = format!("Path requested: {url}");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let _ = stream.write_all(response.as_bytes());
    } else {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
    }
}
