// SPDX-License-Identifier: GPL-3.0-or-later
//! Canonical padded RFC 4648 evidence strings.
#![forbid(unsafe_code)]
use crate::{Error, error::invalid};
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for shift in [18, 12] {
            out.push(char::from(ALPHABET[((n >> shift) & 63) as usize]));
        }
        out.push(if chunk.len() > 1 {
            char::from(ALPHABET[((n >> 6) & 63) as usize])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(ALPHABET[(n & 63) as usize])
        } else {
            '='
        });
    }
    out
}
pub fn decode(text: &str) -> Result<Vec<u8>, Error> {
    if !text.len().is_multiple_of(4) {
        return Err(invalid("base64 length must be a multiple of four"));
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    for (i, c) in text.as_bytes().chunks(4).enumerate() {
        let mut n = 0_u32;
        for (j, b) in c.iter().enumerate() {
            let v = if *b == b'=' && j >= 2 && i == text.len() / 4 - 1 {
                0
            } else {
                ALPHABET
                    .iter()
                    .position(|v| v == b)
                    .ok_or_else(|| invalid("invalid base64 character"))? as u32
            };
            n = (n << 6) | v;
        }
        out.push((n >> 16) as u8);
        if c[2] != b'=' {
            out.push((n >> 8) as u8);
        }
        if c[3] != b'=' {
            out.push(n as u8);
        }
    }
    if encode(&out) != text {
        return Err(invalid("noncanonical base64 padding or trailing bits"));
    }
    Ok(out)
}
