// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use qso_graph_udp::wsjtx::{self, Body, Outcome, Variant};
use std::{
    io::{BufRead, BufReader},
    net::UdpSocket,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn simulator_sends_only_to_the_named_loopback_socket() {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_qg-udp"))
        .args([
            "sim",
            "--wsjtx",
            "--to",
            &socket.local_addr().unwrap().to_string(),
            "--call",
            "K7ABC",
            "--dx-call",
            "W1AW",
            "--mode",
            "FT4",
            "--frequency",
            "7074000",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let mut buf = [0; 65535];
    let mut types = Vec::new();
    for _ in 0..9 {
        let (n, peer) = socket.recv_from(&mut buf).unwrap();
        assert!(peer.ip().is_loopback());
        let Outcome::Message(p) = wsjtx::decode(&buf[..n], Variant::Wsjtx).unwrap() else {
            panic!("packet");
        };
        types.push(p.body.message_type());
        if let Body::QsoLogged(q) = p.body {
            assert_eq!(q.dx_call.as_str(), Some("W1AW"));
            assert_eq!(q.frequency_hz, 7_074_000);
            assert_eq!(q.mode.as_str(), Some("FT4"));
        }
    }
    assert_eq!(types, [0, 1, 2, 2, 2, 1, 5, 3, 6]);
}

#[test]
fn listener_json_is_one_object_per_message_and_survives_malformed_input() {
    // Reserve a candidate port, then wait until the child owns it. No fixed test port.
    let reserve = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = reserve.local_addr().unwrap();
    drop(reserve);
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-udp"))
        .args([
            "listen",
            "--wsjtx",
            &address.to_string(),
            "--json",
            "--count",
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if UdpSocket::bind(address).is_err() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            panic!("listener did not bind");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    sender.send_to(b"bad", address).unwrap();
    let packet = wsjtx::Packet::new("WSJT-X", Body::Close);
    sender
        .send_to(&wsjtx::encode(&packet).unwrap(), address)
        .unwrap();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            panic!("listener did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let lines = BufReader::new(&output.stdout[..])
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&lines[0]).unwrap()["type"],
        "close"
    );
    assert!(!output.stderr.is_empty());
}
