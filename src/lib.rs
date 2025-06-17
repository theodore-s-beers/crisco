#![warn(clippy::pedantic, clippy::nursery)]

use sha2::{Digest, Sha256};
use std::string::ToString;

const BASE62: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[must_use]
pub fn shorten_url(url: &str) -> String {
    let prefix = get_hash_prefix(url);
    let base62_str = to_base62(prefix);

    // Return up to 7 characters of the Base62 string
    base62_str
        .get(..7)
        .map_or_else(|| base62_str.clone(), ToString::to_string)
}

fn get_hash_prefix(url: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let digest = hasher.finalize();

    let mut bytes = [0u8; 8];
    bytes[2..].copy_from_slice(&digest[..6]); // Fill last 6 bytes
    u64::from_be_bytes(bytes)
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
