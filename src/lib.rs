#![warn(clippy::pedantic, clippy::nursery)]

use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasher;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::TcpStream;

//
// Types
//

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug)]
pub enum ReqParseError {
    ConnectionClosed,
    InvalidMethod,
    InvalidReqLine,
    IoError(std::io::Error),
    OversizedBody,
    ParseIntError(std::num::ParseIntError),
    Utf8Error(std::string::FromUtf8Error),
}

impl From<std::io::Error> for ReqParseError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<std::num::ParseIntError> for ReqParseError {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::ParseIntError(err)
    }
}

impl From<std::string::FromUtf8Error> for ReqParseError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

impl fmt::Display for ReqParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ReqParseError::{
            ConnectionClosed, InvalidMethod, InvalidReqLine, IoError, OversizedBody, ParseIntError,
            Utf8Error,
        };

        match self {
            ConnectionClosed => write!(f, "Connection closed by client"),
            InvalidMethod => write!(f, "Invalid HTTP method"),
            InvalidReqLine => write!(f, "Invalid request line"),
            IoError(e) => write!(f, "{e}"),
            OversizedBody => write!(f, "Request body is too large"),
            ParseIntError(e) => write!(f, "{e}"),
            Utf8Error(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReqParseError {}

//
// Constants
//

const BASE62: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const MAX_BODY: usize = 1_048_576;
const MAX_HEADER: u64 = 8192;

//
// Public functions
//

/// # Errors
///
/// Returns `ReqParseError` if the request cannot be parsed,
/// such as if the connection is closed, the request line is invalid,
/// or there are issues reading the headers or body.
pub fn parse_req(stream: &mut TcpStream) -> Result<HttpRequest, ReqParseError> {
    let mut reader = BufReader::new(stream);
    let mut headers_reader = reader.by_ref().take(MAX_HEADER);

    let mut headers: Vec<(String, String)> = Vec::new();
    let mut req_line = String::new();

    let req_line_size = headers_reader.read_line(&mut req_line)?;
    if req_line_size == 0 {
        return Err(ReqParseError::ConnectionClosed);
    }

    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(ReqParseError::InvalidReqLine);
    }

    let method = parts[0].to_ascii_uppercase();
    if method != "GET" && method != "POST" {
        return Err(ReqParseError::InvalidMethod);
    }

    let path = parts[1].to_string();

    // Read headers
    loop {
        let mut line = String::new();
        if headers_reader.read_line(&mut line)? == 0 {
            return Err(ReqParseError::ConnectionClosed);
        }

        if line == "\r\n" {
            break;
        }

        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }

    let content_length_str = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Content-Length"))
        .map_or("0", |(_, v)| v.as_str());

    let content_length: usize = content_length_str.parse()?;
    if content_length > MAX_BODY {
        return Err(ReqParseError::OversizedBody);
    }

    let mut body_bytes = vec![0u8; content_length];
    reader.read_exact(&mut body_bytes)?;
    let body = String::from_utf8(body_bytes)?;

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

pub fn handle_get<S: BuildHasher>(
    mut stream: TcpStream,
    store: &mut HashMap<String, String, S>,
    req: &HttpRequest,
) {
    if req.path == "/" {
        return handle_root(stream);
    }

    let short = req.path.trim_start_matches('/');
    if short.is_empty()
        || short.len() > 7
        || !short.is_ascii()
        || !short.bytes().all(|b| BASE62.contains(&b))
    {
        return redirect_to_root(stream);
    }

    if let Some(url) = store.get(short) {
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: {url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );

        let _ = stream.write_all(response.as_bytes());
    } else {
        redirect_to_root(stream);
    }
}

pub fn handle_post<S: BuildHasher>(
    mut stream: TcpStream,
    store: &mut HashMap<String, String, S>,
    req: &HttpRequest,
) {
    if let Some(url) = extract_url(&req.body) {
        let short = shorten_url(url);
        store.insert(short.clone(), url.to_owned());

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            short.len(),
            short
        );

        let _ = stream.write_all(response.as_bytes());
    } else {
        let msg = "Missing or invalid URL in request body";
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            msg.len(),
            msg
        );

        let _ = stream.write_all(response.as_bytes());
    }
}

pub fn handle_err(mut stream: TcpStream, err: &ReqParseError) {
    let msg = format!("{err}");

    let response = if let ReqParseError::IoError(_) = err {
        format!(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            msg.len(),
            msg
        )
    } else {
        format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            msg.len(),
            msg
        )
    };

    let _ = stream.write_all(response.as_bytes());
}

//
// Private functions
//

fn handle_root(mut stream: TcpStream) {
    let msg = "Try POST with {\"url\": \"https://...\"}";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        msg.len(),
        msg
    );

    let _ = stream.write_all(response.as_bytes());
}

fn redirect_to_root(mut stream: TcpStream) {
    let response =
        "HTTP/1.1 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let _ = stream.write_all(response.as_bytes());
}

fn extract_url(body: &str) -> Option<&str> {
    let key = "\"url\":";

    let start: usize = body.find(key)? + key.len();
    let remainder: &str = body[start..].trim_start();
    if !remainder.starts_with('"') {
        return None;
    }

    let end = remainder[1..].find('"')?;
    let url = &remainder[1..=end];
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }

    Some(url)
}

fn shorten_url(url: &str) -> String {
    let prefix = get_hash_prefix(url);
    let base62_str = to_base62(prefix);

    // Return up to 7 characters of the Base62 string
    base62_str
        .get(..7)
        .map_or_else(|| base62_str.clone(), ToString::to_string)
}

fn get_hash_prefix(url: &str) -> u64 {
    djb2(url) & 0x0000_FFFF_FFFF_FFFF // Keep bottom 48 bits
}

fn djb2(s: &str) -> u64 {
    let mut hash = 5381u64;
    for byte in s.bytes() {
        hash = (hash << 5).wrapping_add(hash) ^ u64::from(byte);
    }
    hash
}

fn to_base62(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    // 9 Base62 characters are enough for any 48-bit number
    let mut buf: Vec<u8> = Vec::with_capacity(9);
    while n > 0 {
        buf.push(BASE62[(n % 62) as usize]);
        n /= 62;
    }

    buf.reverse();
    String::from_utf8(buf).unwrap()
}
