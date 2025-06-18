#![warn(clippy::pedantic, clippy::nursery)]

use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::string::ToString;

//
// Types
//

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

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
            IoError(e) => write!(f, "I/O error: {e}"),
            InvalidMethod => write!(f, "Invalid HTTP method"),
            InvalidReqLine => write!(f, "Invalid request line"),
            OversizedBody => write!(f, "Request body is too large"),
            ParseIntError(e) => write!(f, "Failed to parse integer: {e}"),
            Utf8Error(e) => write!(f, "UTF-8 error: {e}"),
        }
    }
}

//
// Constants
//

const BASE62: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

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
    let mut headers_reader = reader.by_ref().take(8192);

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

    let method = parts[0].to_string();
    if !method.eq_ignore_ascii_case("get") && !method.eq_ignore_ascii_case("post") {
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
    if content_length > 1_048_576 {
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

#[must_use]
pub fn shorten_url(url: &str) -> String {
    let prefix = get_hash_prefix(url);
    let base62_str = to_base62(prefix);

    // Return up to 7 characters of the Base62 string
    base62_str
        .get(..7)
        .map_or_else(|| base62_str.clone(), ToString::to_string)
}

//
// Private functions
//

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
