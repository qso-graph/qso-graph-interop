// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use qso_graph_interop::wsjtx::{self, Body, Outcome};

/// Every test here starts child processes, and the socket tests reserve a port by binding and releasing
/// it. Run in parallel, a child started by one test can briefly inherit another test's socket before
/// it executes, so the listener and its probe disagree about who holds the port. Measured: 3 to 4 of
/// 40 runs failed in parallel, 0 of 40 single-threaded. All of them hold one lock.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static SOCKETS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SOCKETS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
use std::{
    io::{BufRead, BufReader},
    net::UdpSocket,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn simulator_sends_only_to_the_named_loopback_socket() {
    let _sockets = serial();
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
            "Q7ABC",
            "--dx-call",
            "Q1AW",
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
            assert_eq!(q.dx_call.as_str(), Some("Q1AW"));
            assert_eq!(q.frequency_hz, 7_075_500);
            assert_eq!(Some(q.frequency_hz), transmitted_hz);
            assert_eq!(q.mode.as_str(), Some("FT4"));
        }
    }
    assert_eq!(types, [0, 1, 2, 2, 2, 1, 5, 3, 6]);
}

#[test]
fn listener_json_is_one_object_per_message_and_survives_malformed_input() {
    let _sockets = serial();
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
    let _sockets = serial();
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
    let _sockets = serial();
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

#[test]
fn n1mm_listener_and_capture_decoder_supply_observer_context() {
    let _sockets = serial();
    use std::io::Write;
    let bytes = include_bytes!("../../qso-graph-interop/tests/fixtures/n1mm-radio.xml");
    let reserve = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = reserve.local_addr().unwrap();
    drop(reserve);
    let mut listener = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["listen", "--n1mm", &address.to_string(), "--count", "3"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while UdpSocket::bind(address).is_ok() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = listener.kill();
            panic!("N1MM listener did not bind");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    for data in [
        b"<RadioInfo><bad></RadioInfo>".as_slice(),
        b"<Unknown/>".as_slice(),
        bytes.as_slice(),
    ] {
        sender.send_to(data, address).unwrap();
    }
    while listener.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = listener.kill();
            panic!("N1MM listener did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = listener.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout.iter().filter(|&&b| b == b'\n').count(), 1);
    let observed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(observed["family"], "n1mm");
    assert_eq!(observed["context"]["direction"], "from_application");
    assert_eq!(observed["fields"]["dial_frequency_hz"], 137500);
    assert!(!output.stderr.is_empty());
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["decode", "--n1mm", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let captured: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(captured["context"]["direction"], "unknown");
    assert_eq!(captured["native"], observed["native"]);
    let path = std::env::temp_dir().join(format!("qgi-n1mm-decode-{}.xml", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["decode", "--n1mm"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        captured
    );
    for bad in [b"<RadioInfo>".to_vec(), vec![b'x'; 65_508]] {
        std::fs::write(&path, bad).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
            .args(["decode", "--n1mm"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn multicast_arguments_name_what_to_fix() {
    let _sockets = serial();
    for base in [
        vec!["listen", "--wsjtx"],
        vec!["listen", "--n1mm"],
        vec!["sim", "--wsjtx", "--to"],
        vec!["sim", "--n1mm", "--to"],
        vec!["sim", "--capture", "unused.ndjson", "--to"],
    ] {
        for (address, interface, message) in [
            ("239.255.0.1:2237", None, "requires --interface <IPV4>"),
            ("127.0.0.1:2237", Some("127.0.0.1"), "remove --interface"),
            (
                "[ff02::1]:2237",
                None,
                "IPv6 multicast is not supported; use an IPv4 group",
            ),
            (
                "[ff02::1]:2237",
                Some("127.0.0.1"),
                "IPv6 multicast is not supported; use an IPv4 group",
            ),
            (
                "239.255.0.1:2237",
                Some("0.0.0.0"),
                "--interface must be a numeric local IPv4 address",
            ),
            (
                "239.255.0.1:2237",
                Some("224.0.0.1"),
                "--interface must be a numeric local IPv4 address",
            ),
            (
                "239.255.0.1:2237",
                Some("255.255.255.255"),
                "--interface must be a numeric local IPv4 address",
            ),
            (
                "239.255.0.1:2237",
                Some("::1"),
                "--interface must be a numeric local IPv4 address",
            ),
            (
                "239.255.0.1:2237",
                Some("localhost"),
                "--interface must be a numeric local IPv4 address",
            ),
        ] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_qg-interop"));
            command.args(&base).arg(address);
            if let Some(interface) = interface {
                command.args(["--interface", interface]);
            }
            let output = command.output().unwrap();
            assert!(!output.status.success(), "{base:?} {address} {interface:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(message),
                "{base:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// Kill and reap children on every failure path, including an interface retry.
struct MulticastListener {
    child: Option<std::process::Child>,
    stdout: Option<std::thread::JoinHandle<Vec<u8>>>,
}
impl Drop for MulticastListener {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(stdout) = self.stdout.take() {
            let _ = stdout.join();
        }
    }
}

fn multicast_pair(interface: std::net::Ipv4Addr) -> Result<(), String> {
    let reserve = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    let port = reserve.local_addr().unwrap().port();
    drop(reserve);
    // Administratively scoped test group; no discovery or hostnames.
    let address = format!("239.255.42.99:{port}");
    let interface = interface.to_string();
    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_qg-interop"))
            .args([
                "listen",
                "--wsjtx",
                &address,
                "--interface",
                &interface,
                "--count",
                "9",
                "--json",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map(|mut child| {
                let mut stdout = child.stdout.take().unwrap();
                let reader = std::thread::spawn(move || {
                    let mut bytes = Vec::new();
                    std::io::Read::read_to_end(&mut stdout, &mut bytes).unwrap();
                    bytes
                });
                MulticastListener {
                    child: Some(child),
                    stdout: Some(reader),
                }
            })
            .map_err(|e| e.to_string())
    };
    let mut listeners = [spawn()?, spawn()?];
    // Both sockets reuse the port, so a non-reusing wildcard probe fails once at least
    // one has bound. Allow the other child to finish startup before the single stream.
    let begun = Instant::now();
    while UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, port)).is_ok() {
        if begun.elapsed() > Duration::from_secs(5) {
            return Err("listeners did not bind".into());
        }
        for listener in &mut listeners {
            if let Some(status) = listener.child.as_mut().unwrap().try_wait().unwrap() {
                return Err(format!("listener exited before bind: {status}"));
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(250));
    for listener in &mut listeners {
        if let Some(status) = listener.child.as_mut().unwrap().try_wait().unwrap() {
            return Err(format!("shared-port listener exited: {status}"));
        }
    }
    let sent = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "sim",
            "--wsjtx",
            "--to",
            &address,
            "--interface",
            &interface,
            "--mode",
            "FT8",
            "--call",
            "Q7ABC",
        ])
        .output()
        .unwrap();
    if !sent.status.success() {
        return Err(format!(
            "simulator: {}",
            String::from_utf8_lossy(&sent.stderr)
        ));
    }
    let until = Instant::now();
    for listener in &mut listeners {
        while listener
            .child
            .as_mut()
            .unwrap()
            .try_wait()
            .unwrap()
            .is_none()
        {
            if until.elapsed() > Duration::from_secs(3) {
                return Err("listeners did not both receive the multicast stream".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    let mut decoded = Vec::new();
    for listener in &mut listeners {
        let output = listener.child.take().unwrap().wait_with_output().unwrap();
        let stdout = listener.stdout.take().unwrap().join().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let events: Vec<serde_json::Value> = BufReader::new(&stdout[..])
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect();
        assert_eq!(events.len(), 9);
        assert!(
            events
                .iter()
                .all(|v| v["context"]["direction"] == "from_application")
        );
        assert!(events.iter().any(|v| v["kind"] == "qso_logged"));
        decoded.push(events);
    }
    assert_eq!(
        decoded[0], decoded[1],
        "both listeners must decode the same events"
    );
    Ok(())
}

#[test]
fn two_multicast_listeners_share_one_group_and_port() {
    let _sockets = serial();
    let loopback = std::net::Ipv4Addr::LOCALHOST;
    match multicast_pair(loopback) {
        Ok(()) => {
            eprintln!(
                "multicast live test: passed on loopback ({loopback}); two listeners, 9 identical events each"
            );
            return;
        }
        Err(reason) => eprintln!("multicast loopback unavailable: {reason}"),
    }
    // connect selects a numeric route and sends no datagram.
    let primary = UdpSocket::bind("0.0.0.0:0").and_then(|probe| {
        probe.connect("192.0.2.1:9")?;
        probe.local_addr()
    });
    match primary {
        Ok(std::net::SocketAddr::V4(address)) if *address.ip() != loopback => {
            match multicast_pair(*address.ip()) {
                Ok(()) => {
                    eprintln!(
                        "multicast live test: passed on primary interface ({}); two listeners, 9 identical events each",
                        address.ip()
                    );
                    return;
                }
                Err(reason) => eprintln!("multicast primary interface unavailable: {reason}"),
            }
        }
        other => eprintln!("multicast primary interface unavailable: {other:?}"),
    }
    assert_ne!(
        std::env::var("QGI_REQUIRE_MULTICAST").as_deref(),
        Ok("1"),
        "required multicast test failed: no multicast-capable interface"
    );
    // Bypass the test harness's success-output capture so a skip is always visible.
    std::io::Write::write_all(
        &mut std::io::stderr(),
        b"skipped: no multicast-capable interface\n",
    )
    .unwrap();
}
