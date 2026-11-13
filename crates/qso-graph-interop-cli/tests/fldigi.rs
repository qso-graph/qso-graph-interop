// SPDX-License-Identifier: GPL-3.0-or-later
//! Loopback-only fake XML-RPC peer; no live application or outside service.
#![forbid(unsafe_code)]
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_qg-interop"))
}
fn accept(listener: &TcpListener) -> TcpStream {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        match listener.accept() {
            Ok((s, _)) => {
                s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                return s;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until => {
                thread::sleep(Duration::from_millis(5))
            }
            Err(e) => panic!("fake peer accept: {e}"),
        }
    }
}
fn request(socket: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut b = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        socket.read_exact(&mut b).unwrap();
        bytes.push(b[0]);
        assert!(bytes.len() < 16384);
    }
    let header = String::from_utf8(bytes).unwrap();
    assert!(header.starts_with("POST /RPC2 HTTP/1.1\r\n"));
    let len: usize = header
        .lines()
        .find_map(|s| s.strip_prefix("Content-Length: "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(len < 262145);
    let mut body = vec![0; len];
    socket.read_exact(&mut body).unwrap();
    String::from_utf8(body).unwrap()
}
#[test]
fn poll_state_rx_fault_correlation_and_bounds() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let peer = thread::spawn(move || {
        for (method, value) in [
            ("rig.get_frequency", "<double>14071000.0</double>"),
            ("modem.get_carrier", "<int>850</int>"),
            ("modem.get_name", "<string>BPSK31</string>"),
            ("modem.get_submode", "<string></string>"),
            ("main.get_trx_state", "<string>RX</string>"),
            ("main.get_trx_status", "<string>tune</string>"),
            ("log.get_call", "<string>Q2BBB</string>"),
            ("log.get_locator", "fault"),
            ("rx.get_data", "<base64>Q1EgUTFBQUE=</base64>"),
        ] {
            let mut socket = accept(&listener);
            let body = request(&mut socket);
            assert_eq!(
                body,
                format!(
                    "<?xml version=\"1.0\" encoding=\"utf-8\"?><methodCall><methodName>{method}</methodName><params></params></methodCall>"
                )
            );
            let reply = if value == "fault" {
                "<methodResponse><fault><value><struct><member><name>faultCode</name><value><int>+003</int></value></member><member><name>faultString</name><value><string>synthetic</string></value></member></struct></value></fault></methodResponse>".to_owned()
            } else {
                format!(
                    "<methodResponse><params><param><value>{value}</value></param></params></methodResponse>"
                )
            };
            // A fault remains an application result even with HTTP status 500.
            write!(
                socket,
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                if value == "fault" {
                    "500 Error"
                } else {
                    "200 OK"
                },
                reply.len(),
                reply
            )
            .unwrap();
        }
    });
    let output = cli()
        .args([
            "poll",
            "--fldigi",
            &addr.to_string(),
            "--interval",
            "1",
            "--count",
            "1",
        ])
        .output()
        .unwrap();
    peer.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 9);
    assert_eq!(events[0]["fields"], json!({"dial_frequency_hz":14071000}));
    assert_eq!(events[1]["fields"], json!({"rx_offset_hz":850}));
    assert_eq!(events[5]["fields"]["transmitting"], true);
    assert_eq!(events[7]["fields"]["status"], "error");
    assert_eq!(events[7]["fields"]["native_code"], "+003");
    assert_eq!(events[8]["fields"]["text"], "CQ Q1AAA");
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e["context"]["direction"], "from_application");
        assert_eq!(e["context"]["exchange"], "response");
        assert!(
            e["context"]["correlation_id"]
                .as_str()
                .unwrap()
                .starts_with("poll-0-")
        );
        assert_eq!(e["family"], "fldigi");
        if i > 0 {
            assert_ne!(
                e["context"]["correlation_id"],
                events[i - 1]["context"]["correlation_id"]
            );
        }
    }
}
#[test]
fn rejects_nonloopback_dns_and_unbounded_http() {
    for address in [
        "192.0.2.1:7362",
        "0.0.0.0:7362",
        "[2001:db8::1]:7362",
        "localhost:7362",
        "example.com:7362",
        "127.0.0.1:0",
    ] {
        let out = cli()
            .args(["poll", "--fldigi", address, "--count", "1"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("plain HTTP"));
        assert!(out.stdout.is_empty());
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let peer = thread::spawn(move || {
        let mut socket = accept(&listener);
        request(&mut socket);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 262145\r\n\r\n")
            .unwrap();
    });
    let out = cli()
        .args(["poll", "--fldigi", &addr.to_string(), "--count", "1"])
        .output()
        .unwrap();
    peer.join().unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
}
fn decode(body: &str, args: &[&str]) -> std::process::Output {
    let mut child = cli()
        .args(["decode", "--fldigi"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn decode_requires_explicit_response_association_and_preserves_unknown_direction() {
    let response = "<methodResponse><params><param><value><double>14071000.0</double></value></param></params></methodResponse>";
    let no = decode(response, &[]);
    assert!(!no.status.success());
    assert!(no.stdout.is_empty());
    assert!(String::from_utf8_lossy(&no.stderr).contains("--request-method"));
    let yes = decode(response, &["--request-method", "rig.get_frequency"]);
    assert!(yes.status.success());
    let event: Value = serde_json::from_slice(&yes.stdout).unwrap();
    assert_eq!(event["context"]["direction"], "unknown");
    assert_eq!(event["fields"]["dial_frequency_hz"], 14071000);
    let request = "<methodCall><methodName>text.get_rx</methodName><params><param><value><int>0</int></value></param><param><value><int>2</int></value></param></params></methodCall>";
    let call = decode(request, &[]);
    assert!(call.status.success());
    let file = std::env::temp_dir().join(format!("qgi-fldigi-request-{}.xml", std::process::id()));
    std::fs::write(&file, request).unwrap();
    let rx = decode(
        "<methodResponse><params><param><value><base64>UUE=</base64></value></param></params></methodResponse>",
        &["--request-file", file.to_str().unwrap()],
    );
    std::fs::remove_file(file).unwrap();
    assert!(
        rx.status.success(),
        "{}",
        String::from_utf8_lossy(&rx.stderr)
    );
    let event: Value = serde_json::from_slice(&rx.stdout).unwrap();
    assert_eq!(event["kind"], "decode");
    assert_eq!(event["fields"]["text"], "QA");
}
#[test]
fn slow_http_peer_hits_total_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let (finish, finished) = std::sync::mpsc::channel();
    let peer = thread::spawn(move || {
        let mut socket = accept(&listener);
        request(&mut socket);
        let _ = finished.recv_timeout(Duration::from_secs(10));
    });
    let start = Instant::now();
    let out = cli()
        .args(["poll", "--fldigi", &addr.to_string(), "--count", "1"])
        .output()
        .unwrap();
    let elapsed = start.elapsed();
    let _ = finish.send(());
    peer.join().unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(elapsed >= Duration::from_secs(4) && elapsed < Duration::from_secs(8));
}
