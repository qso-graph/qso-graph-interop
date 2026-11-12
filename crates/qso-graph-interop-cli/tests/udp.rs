// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use qso_graph_interop::wsjtx::{self, Body, Outcome};
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
    let status = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
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
    let mut transmitted_hz = None;
    for _ in 0..9 {
        let (n, peer) = socket.recv_from(&mut buf).unwrap();
        assert!(peer.ip().is_loopback());
        let Outcome::Message(p) = wsjtx::decode(&buf[..n]).unwrap() else {
            panic!("packet");
        };
        types.push(p.body.message_type());
        if let Body::Status(s) = &p.body {
            transmitted_hz = s.dial_frequency_hz.checked_add(u64::from(s.tx_df_hz));
        }
        if let Body::QsoLogged(q) = p.body {
            assert_eq!(q.dx_call.as_str(), Some("W1AW"));
            assert_eq!(q.frequency_hz, 7_075_500);
            assert_eq!(Some(q.frequency_hz), transmitted_hz);
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
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
    // Close from a WSJT-X instance, as the listener observes it (QGI-WSJ-0022).
    let event = serde_json::from_str::<serde_json::Value>(&lines[0]).unwrap();
    assert_eq!(event["kind"], "lifecycle");
    assert_eq!(event["fields"]["state"], "closed");
    assert!(!output.stderr.is_empty());
}

#[test]
fn simulator_to_listener_reports_consistent_transmit_frequency() {
    let reserve = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = reserve.local_addr().unwrap();
    drop(reserve);
    let mut listener = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "listen",
            "--wsjtx",
            &address.to_string(),
            "--json",
            "--count",
            "9",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while UdpSocket::bind(address).is_ok() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = listener.kill();
            panic!("listener did not bind");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let sim = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "sim",
            "--wsjtx",
            "--to",
            &address.to_string(),
            "--mode",
            "FT8",
        ])
        .status()
        .unwrap();
    assert!(sim.success());
    while listener.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = listener.kill();
            panic!("listener did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = listener.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let events: Vec<serde_json::Value> = BufReader::new(&output.stdout[..])
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(events.len(), 9);
    // The listener is the observer on a WSJT-X server port, so it supplies the direction.
    assert!(
        events
            .iter()
            .all(|v| v["context"]["direction"] == "from_application")
    );
    let status = events.iter().find(|v| v["kind"] == "radio_state").unwrap();
    let qso = events.iter().find(|v| v["kind"] == "qso_logged").unwrap();
    assert_eq!(status["fields"]["dial_frequency_hz"], 14_074_000);
    assert_eq!(
        qso["fields"]["tx_frequency_hz"],
        status["fields"]["dial_frequency_hz"].as_u64().unwrap()
            + status["fields"]["tx_offset_hz"].as_u64().unwrap()
    );
    assert_eq!(qso["fields"]["tx_frequency_hz"], 14_075_500);
}

#[test]
fn decode_reads_file_and_stdin_and_rejects_an_oversized_capture() {
    use std::io::Write;
    let bytes = wsjtx::encode(&wsjtx::Packet::new("synthetic-instance", Body::Close)).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["decode", "--wsjtx", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let from_stdin: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // A file's direction is unknown, so a Close is ambiguous (QGI-WSJ-0022).
    assert_eq!(from_stdin["context"]["direction"], "unknown");
    assert_eq!(from_stdin["kind"], "ambiguous");
    let path = std::env::temp_dir().join(format!("qgi-decode-{}.bin", std::process::id()));
    std::fs::write(&path, &bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["decode", "--wsjtx"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        from_stdin,
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    );
    std::fs::write(&path, vec![0; 65_508]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["decode", "--wsjtx"])
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
