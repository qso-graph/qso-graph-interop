// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal bounded HTTP client and read-only fldigi polling.
#![forbid(unsafe_code)]
use crate::{Poll, loopback_tcp::LoopbackTcp};
use qso_graph_interop::{
    Codec, Event,
    codec::{DecodeOptions, EncodeOptions},
    fldigi::{Fldigi, MAX_BODY},
};
use serde_json::json;
use std::io::{self, BufReader, Read, Write};
use std::net::SocketAddr;
use std::time::Duration;
const MAX_HEADER: usize = 16_384;
const METHODS: &[&str] = &[
    "rig.get_frequency",
    "modem.get_carrier",
    "modem.get_name",
    "modem.get_submode",
    "main.get_trx_state",
    "main.get_trx_status",
    "log.get_call",
    "log.get_locator",
    "rx.get_data",
];
fn bad(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn line(input: &mut impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let mut b = [0];
        input.read_exact(&mut b)?;
        out.push(b[0]);
        if out.len() > limit {
            return Err(bad("HTTP line/header bound exceeded"));
        }
        if out.ends_with(b"\r\n") {
            out.truncate(out.len() - 2);
            return Ok(out);
        }
    }
}
fn headers(input: &mut impl Read) -> io::Result<Vec<(String, String)>> {
    let mut size = 0;
    let mut result = vec![];
    loop {
        let bytes = line(input, MAX_HEADER - size)?;
        size += bytes.len() + 2;
        if size > MAX_HEADER {
            return Err(bad("HTTP header bound exceeded"));
        }
        if bytes.is_empty() {
            return Ok(result);
        }
        let s = std::str::from_utf8(&bytes).map_err(|_| bad("non-ASCII HTTP header"))?;
        let (key, value) = s
            .split_once(':')
            .ok_or_else(|| bad("invalid HTTP header"))?;
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !value.bytes().all(|b| b == 9 || (32..=126).contains(&b))
        {
            return Err(bad("invalid HTTP header"));
        }
        result.push((key.to_ascii_lowercase(), value.trim().to_owned()));
    }
}
fn read_body(input: &mut impl Read) -> io::Result<Vec<u8>> {
    let status = line(input, 128)?;
    let status = std::str::from_utf8(&status).map_err(|_| bad("invalid HTTP status"))?;
    let mut parts = status.splitn(3, ' ');
    if !matches!(parts.next(), Some("HTTP/1.1" | "HTTP/1.0"))
        || !parts
            .next()
            .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(bad("invalid HTTP status"));
    }
    let hdr = headers(input)?;
    let unique = |key: &str| -> io::Result<Option<&str>> {
        let mut values = hdr
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str());
        let first = values.next();
        if values.next().is_some() {
            return Err(bad("duplicate HTTP framing header"));
        }
        Ok(first)
    };
    let length = unique("content-length")?;
    let transfer = unique("transfer-encoding")?;
    if unique("content-encoding")?.is_some_and(|s| !s.eq_ignore_ascii_case("identity")) {
        return Err(bad("unsupported HTTP content encoding"));
    }
    if length.is_some() && transfer.is_some() {
        return Err(bad("conflicting HTTP framing"));
    }
    let mut body = Vec::new();
    if let Some(length) = length {
        if length.is_empty() || !length.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad("invalid HTTP content length"));
        }
        let n: usize = length
            .parse()
            .map_err(|_| bad("HTTP body bound exceeded"))?;
        if n > MAX_BODY {
            return Err(bad("HTTP body bound exceeded"));
        }
        body.resize(n, 0);
        input.read_exact(&mut body)?;
    } else if let Some(transfer) = transfer {
        if !transfer.eq_ignore_ascii_case("chunked") {
            return Err(bad("unsupported HTTP transfer encoding"));
        }
        for _ in 0..1024 {
            let bytes = line(input, 128)?;
            let s = std::str::from_utf8(&bytes).map_err(|_| bad("invalid HTTP chunk"))?;
            let s = s.split(';').next().unwrap_or("");
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad("invalid HTTP chunk size"));
            }
            let n = usize::from_str_radix(s, 16).map_err(|_| bad("HTTP body bound exceeded"))?;
            if n == 0 {
                headers(input)?;
                return Ok(body);
            }
            if n > MAX_BODY - body.len() {
                return Err(bad("HTTP body bound exceeded"));
            }
            let start = body.len();
            body.resize(start + n, 0);
            input.read_exact(&mut body[start..])?;
            let mut crlf = [0; 2];
            input.read_exact(&mut crlf)?;
            if crlf != *b"\r\n" {
                return Err(bad("invalid HTTP chunk delimiter"));
            }
        }
        return Err(bad("HTTP chunk count exceeded"));
    } else {
        input.take((MAX_BODY + 1) as u64).read_to_end(&mut body)?;
        if body.len() > MAX_BODY {
            return Err(bad("HTTP body bound exceeded"));
        }
    }
    Ok(body)
}
fn request(address: SocketAddr, body: &[u8]) -> io::Result<Vec<u8>> {
    let mut socket = LoopbackTcp::connect(address)?;
    write!(
        socket,
        "POST /RPC2 HTTP/1.1\r\nHost: {address}\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    socket.write_all(body)?;
    read_body(&mut BufReader::new(socket))
}
pub fn run(options: Poll) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut cycle = 0_u64;
    loop {
        for method in METHODS {
            let association = json!({"method":method});
            let mut codec = Fldigi::with_request(association);
            let op = format!("fldigi.{}", method.replace('.', "_"));
            let command = Event::from_value(
                json!({"spec":"0.1.0","family":"fldigi","mapping":1,"kind":"control","context":{"direction":"to_application","exchange":"request"},"fields":{"operation":op}}),
            )?;
            let bytes = codec.encode_command(&command, &EncodeOptions::default())?;
            let body = request(options.address, &bytes)?;
            let mut decode = DecodeOptions::default();
            decode.structured = true;
            decode.context = json!({"direction":"from_application","exchange":"response","correlation_id":format!("poll-{cycle}-{method}")});
            for event in codec.decode(&body, &decode)?.events {
                output.write_all(&qso_graph_interop::json::write(&event, true, Some(&codec))?)?;
                writeln!(output)?;
            }
            output.flush()?;
        }
        cycle = cycle
            .checked_add(1)
            .ok_or_else(|| bad("poll counter exhausted"))?;
        if options.count != 0 && cycle >= options.count {
            break;
        }
        std::thread::sleep(Duration::from_millis(options.interval));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing_and_planted_http_failures() {
        assert_eq!(
            read_body(&mut &b"HTTP/1.1 500 Error\r\nContent-Length: 3\r\n\r\nabc"[..]).unwrap(),
            b"abc"
        );
        assert_eq!(
            read_body(
                &mut &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n0\r\n\r\n"
                    [..]
            )
            .unwrap(),
            b"abc"
        );
        for data in [
            "HTTP/1.1 200 OK\r\nContent-Length: 262145\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\na",
            "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n40001\r\n",
            "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\n\r\n",
        ] {
            assert!(read_body(&mut data.as_bytes()).is_err(), "{data}");
        }
        let huge = format!("HTTP/1.1 200 OK\r\nX: {}\r\n\r\n", "x".repeat(MAX_HEADER));
        assert!(read_body(&mut huge.as_bytes()).is_err());
    }
    #[test]
    fn http_resource_boundaries_have_planted_overflows() {
        let head = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
        let max = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {MAX_BODY}\r\n\r\n{}",
            "x".repeat(MAX_BODY)
        );
        assert_eq!(read_body(&mut max.as_bytes()).unwrap().len(), MAX_BODY);
        let many = format!("{head}{}0\r\n\r\n", "1\r\nx\r\n".repeat(1023));
        assert_eq!(read_body(&mut many.as_bytes()).unwrap().len(), 1023);
        let overflow = format!("{head}{}0\r\n\r\n", "1\r\nx\r\n".repeat(1024));
        assert!(read_body(&mut overflow.as_bytes()).is_err());
        for bad in [
            format!("{head}{}\r\n", "1".repeat(129)),
            format!("{head}0\r\nX: {}\r\n\r\n", "x".repeat(MAX_HEADER)),
            format!("{head}20000\r\n{}\r\n20001\r\n", "x".repeat(MAX_BODY / 2)),
            format!("HTTP/1.1 200 OK\r\n\r\n{}", "x".repeat(MAX_BODY + 1)),
        ] {
            assert!(read_body(&mut bad.as_bytes()).is_err());
        }
    }
}
