// SPDX-License-Identifier: GPL-3.0-or-later
//! Real CLI server and poller over loopback; serialize child/socket lifetimes as udp.rs does.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Codec, Event,
    codec::{DecodeOptions, EncodeOptions},
    fldigi::Fldigi,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static SOCKETS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SOCKETS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qg-interop"))
}
struct Server {
    child: Child,
    address: SocketAddr,
}
impl Server {
    fn start(speed: &str) -> Self {
        let reserve = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reserve.local_addr().unwrap();
        drop(reserve);
        let mut child = cli()
            .args([
                "sim",
                "--fldigi",
                &address.to_string(),
                "--speed",
                speed,
                "--seed",
                "42",
                "--call",
                "Q7ABC",
                "--grid",
                "EM10",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stderr.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert!(line.contains("fldigi simulator listening"), "{line}");
        Self { child, address }
    }
    fn socket(&self) -> TcpStream {
        let s = TcpStream::connect_timeout(&self.address, Duration::from_secs(3)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(7))).unwrap();
        s.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
        s
    }
    fn request(&self, body: &[u8]) -> Vec<u8> {
        let mut socket = self.socket();
        exchange(&mut socket, body, true)
    }
    fn command(&self, fields: Value) -> Vec<Vec<u8>> {
        let event = Event::from_value(json!({"spec":"0.1.0","family":"fldigi","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":fields})).unwrap();
        Fldigi::default()
            .encode_commands(&event, &EncodeOptions::default())
            .unwrap()
            .iter()
            .map(|b| self.request(b))
            .collect()
    }
    fn get(&self, method: &str) -> Value {
        observed(method, &self.request(&request(method, "")))
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn request(method: &str, args: &str) -> Vec<u8> {
    format!("<methodCall><methodName>{method}</methodName><params>{args}</params></methodCall>")
        .into_bytes()
}
fn exchange(socket: &mut TcpStream, body: &[u8], close: bool) -> Vec<u8> {
    write!(
        socket,
        "POST /RPC2 HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n",
        body.len(),
        if close { "close" } else { "keep-alive" }
    )
    .unwrap();
    socket.write_all(body).unwrap();
    read_response(socket)
}
fn read_response(socket: &mut TcpStream) -> Vec<u8> {
    let mut header = vec![];
    let mut b = [0];
    while !header.ends_with(b"\r\n\r\n") {
        socket.read_exact(&mut b).unwrap();
        header.push(b[0]);
        assert!(header.len() < 16384);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with("HTTP/1.1 200 OK\r\n"));
    let len: usize = header
        .lines()
        .find_map(|s| s.strip_prefix("Content-Length: "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(len <= 262144);
    let mut body = vec![0; len];
    socket.read_exact(&mut body).unwrap();
    body
}
fn observed(method: &str, body: &[u8]) -> Value {
    Fldigi::with_request(json!({"method":method,"arguments":[0,-1]}))
        .decode(body, &DecodeOptions::default())
        .unwrap()
        .events
        .remove(0)
        .into_value()
}
fn wait_for(server: &Server, method: &str, field: &str, wanted: Value) {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let event = server.get(method);
        if event["fields"][field] == wanted {
            return;
        }
        assert!(Instant::now() < end, "{event}");
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn poll_commands_every_mapped_method_and_keep_alive() {
    let _lock = serial();
    let server = Server::start("100");
    let poll = cli()
        .args([
            "poll",
            "--fldigi",
            &server.address.to_string(),
            "--count",
            "2",
            "--interval",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        poll.status.success(),
        "{}",
        String::from_utf8_lossy(&poll.stderr)
    );
    assert!(poll.stderr.is_empty());
    let events: Vec<Value> = String::from_utf8(poll.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 18);
    assert!(events.iter().all(|e| e["family"] == "fldigi"
        && e["kind"] != "native"
        && e["fields"]["status"] != "error"));
    let replies = server
        .command(json!({"operation":"set_frequency","frequency_hz":7070000,"rx_offset_hz":850}));
    assert_eq!(
        observed("rig.set_frequency", &replies[0])["fields"]["frequency_hz"],
        14070000
    );
    assert_eq!(
        server.get("rig.get_frequency")["fields"]["dial_frequency_hz"],
        7070000
    );
    assert_eq!(
        server.get("modem.get_carrier")["fields"]["rx_offset_hz"],
        850
    );
    for (op, key, val, get) in [
        ("fldigi.log_set_call", "dx_call", "Q9XYZ", "log.get_call"),
        (
            "fldigi.log_set_locator",
            "dx_grid",
            "FN20",
            "log.get_locator",
        ),
        (
            "fldigi.modem_set_by_name",
            "mode",
            "BPSK63",
            "modem.get_name",
        ),
    ] {
        server.command(json!({"operation":op,key:val}));
        assert_eq!(server.get(get)["fields"][key], val);
    }
    // A long queue makes TX observable independently of machine scheduling speed.
    server.command(json!({"operation":"send_text","text":"Q".repeat(60000),"transmit":true}));
    assert_eq!(
        server.get("main.get_trx_status")["fields"]["transmitting"],
        true
    );
    server.command(json!({"operation":"halt_tx"}));
    assert_eq!(
        server.get("main.get_trx_status")["fields"]["transmitting"],
        false
    );
    server.command(json!({"operation":"fldigi.text_clear_tx"}));
    server.command(json!({"operation":"send_text","text":"CQ Q7ABC","transmit":true}));
    wait_for(&server, "main.get_trx_status", "transmitting", json!(false));
    let annex = include_str!("../../../spec/annex-e-fldigi.md");
    let table = annex
        .split("## E.4 Complete mapping catalog")
        .nth(1)
        .unwrap()
        .split("### Remaining pinned methods")
        .next()
        .unwrap();
    let mut count = 0;
    for row in table.lines().filter(|l| l.starts_with("| `")) {
        let cols: Vec<_> = row.split('|').collect();
        let method = cols[1].trim().trim_matches('`');
        let sig = cols[2].trim().trim_matches('`');
        let Some((_, arg)) = sig.split_once(':') else {
            continue;
        };
        let args = match arg {
            "n" => "",
            "s" => "<param><value><string>Q1AAA</string></value></param>",
            "d" => "<param><value><double>14070000</double></value></param>",
            "i" => "<param><value><int>1500</int></value></param>",
            "ii" => {
                "<param><value><int>0</int></value></param><param><value><int>0</int></value></param>"
            }
            _ => panic!("{sig}"),
        };
        let e = observed(method, &server.request(&request(method, args)));
        assert_ne!(e["fields"]["status"], "error", "{method}: {e}");
        count += 1;
    }
    assert_eq!(count, 50);
    let mut socket = server.socket();
    for method in ["fldigi.version", "rig.get_frequency", "modem.get_carrier"] {
        let body = exchange(&mut socket, &request(method, ""), false);
        assert_ne!(observed(method, &body)["kind"], "native");
    }
    let unknown = server.get("made.up");
    assert_eq!(unknown["fields"]["native_code"], "-1");
    assert_eq!(unknown["fields"]["text"], "made.up: unknown method name");
    assert_eq!(
        observed("unknown", &server.request(b"not XML"))["fields"]["text"],
        ": unknown method name"
    );
}
#[test]
fn rx_offsets_drain_and_clear_over_loopback() {
    let _lock = serial();
    let server = Server::start("1000");
    // The finite stream completes in under a second at this speed. Wait on actual byte count.
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let body = server.request(&request("text.get_rx_length", ""));
        if String::from_utf8_lossy(&body).contains("<int>96</int>") {
            break;
        }
        if Instant::now() >= end {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    // Freeze the RX script with tune so overlapping reads are stable even on a slow runner.
    server.command(json!({"operation":"fldigi.main_tune"}));
    let all =
        server.command(json!({"operation":"fldigi.text_get_rx","rx_start":0,"rx_length":100}));
    // The encoder accepts nonnegative explicit lengths; request the full range independently.
    assert_eq!(all.len(), 1);
    let range = |start: i64, n: i64| {
        format!(
            "<param><value><int>{start}</int></value></param><param><value><int>{n}</int></value></param>"
        )
    };
    let all = observed(
        "text.get_rx",
        &server.request(&request("text.get_rx", &range(0, -1))),
    )["fields"]["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(all.contains("Q7ABC"));
    assert!(all.contains("Q7BBB"));
    for (start, n) in [(0, 6), (6, 10), (3, 8)] {
        let got = observed(
            "text.get_rx",
            &server.request(&request("text.get_rx", &range(start as i64, n as i64))),
        );
        assert_eq!(got["fields"]["text"], &all[start..start + n]);
    }
    server.get("rx.get_data");
    assert_eq!(server.get("rx.get_data")["fields"]["text"], "");
    assert_eq!(
        observed(
            "text.get_rx",
            &server.request(&request("text.get_rx", &range(0, -1)))
        )["fields"]["text"],
        all
    );
    server.command(json!({"operation":"fldigi.text_clear_rx"}));
    assert!(
        String::from_utf8_lossy(&server.request(&request("text.get_rx_length", "")))
            .contains("<int>0</int>")
    );
}
#[test]
fn listener_refuses_external_dns_and_conflicting_options_without_hanging() {
    let _lock = serial();
    for address in [
        "0.0.0.0:7362",
        "192.0.2.1:7362",
        "[::]:7362",
        "[2001:db8::1]:7362",
        "localhost:7362",
        "127.0.0.1:0",
    ] {
        let mut child = cli()
            .args(["sim", "--fldigi", address])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let end = Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(s) = child.try_wait().unwrap() {
                break Some(s);
            }
            if Instant::now() >= end {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            thread::sleep(Duration::from_millis(5));
        };
        assert!(
            status.is_some_and(|s| !s.success()),
            "non-loopback accepted: {address}"
        );
    }
    for args in [
        ["--to", "127.0.0.1:2237"],
        ["--wsjtx", ""],
        ["--n1mm", ""],
        ["--file", "ALL.TXT"],
        ["--count", "1"],
    ] {
        assert!(
            !cli()
                .args(["sim", "--fldigi"])
                .args(args.into_iter().filter(|s| !s.is_empty()))
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
#[test]
fn slow_peers_capacity_deadline_and_http_limits() {
    let _lock = serial();
    let server = Server::start("1");
    let mut slow = server.socket();
    slow.write_all(b"POST /RPC2 HTTP/1.1\r\nContent-Length: 100\r\n\r\n<")
        .unwrap();
    // One incomplete request cannot block the poller.
    let poll = cli()
        .args([
            "poll",
            "--fldigi",
            &server.address.to_string(),
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    assert!(poll.status.success());
    let start = Instant::now();
    let mut b = [0];
    assert_eq!(slow.read(&mut b).unwrap(), 0);
    assert!(start.elapsed() < Duration::from_secs(7));
    for header in [
        "Content-Length: 262145",
        "Content-Length: 0",
        "Content-Length: 1\r\nContent-Length: 1",
        "Transfer-Encoding: chunked",
    ] {
        let mut socket = server.socket();
        write!(socket, "POST /RPC2 HTTP/1.1\r\n{header}\r\n\r\n").unwrap();
        assert_eq!(socket.read(&mut b).unwrap(), 0);
    }
    let mut peers = vec![];
    for _ in 0..16 {
        let mut s = server.socket();
        s.write_all(b"P").unwrap();
        peers.push(s);
    }
    thread::sleep(Duration::from_millis(100));
    let mut overflow = server.socket();
    let overflow_start = Instant::now();
    assert_eq!(overflow.read(&mut b).unwrap(), 0);
    assert!(overflow_start.elapsed() < Duration::from_secs(2));
    drop(peers);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(server.get("fldigi.version")["native_version"], "4.2.13");
}
