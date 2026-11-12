// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Codec,
    codec::DecodeOptions,
    wsjtx::{self, Body, Packet},
};
use std::{
    net::UdpSocket,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn expected() -> Vec<Packet> {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../qso-graph-interop/tests/fixtures/sim-python-synthetic.json"
    ))
    .unwrap();
    v["datagrams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| packet(&bytes(v.as_str().unwrap())))
        .collect()
}
fn packet(bytes: &[u8]) -> Packet {
    let out = wsjtx::Wsjtx::default()
        .decode(bytes, &DecodeOptions::default())
        .unwrap();
    assert_eq!(out.events.len(), 1);
    assert!(out.skips.is_empty());
    assert_eq!(out.events[0].native_bytes().unwrap(), bytes);
    let wsjtx::Outcome::Message(p) = wsjtx::decode(bytes).unwrap() else {
        panic!("message");
    };
    *p
}
fn normalized(packets: &[Packet]) -> Vec<Packet> {
    let base = packets
        .iter()
        .find_map(|p| {
            if let Body::Decode(d) = &p.body {
                Some(d.time.0)
            } else {
                None
            }
        })
        .unwrap_or(0);
    packets
        .iter()
        .cloned()
        .map(|mut p| {
            match &mut p.body {
                Body::Decode(d) => d.time.0 = (d.time.0 + 86_400_000 - base) % 86_400_000,
                Body::Heartbeat(h) => {
                    h.version = Some("version".into());
                    h.revision = Some("revision".into());
                }
                _ => {}
            }
            p
        })
        .collect()
}
fn matches_capture(actual: &[Packet], expected: &[Packet]) -> bool {
    normalized(actual) == normalized(expected)
}
fn socket() -> UdpSocket {
    let s = UdpSocket::bind("127.0.0.1:0").unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s
}
#[test]
fn sample_matches_python_capture_and_filters_foreign_commands() {
    let receiver = socket();
    let stranger = socket();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "sim",
            "--wsjtx",
            "--to",
            &receiver.local_addr().unwrap().to_string(),
            "--call",
            "W1ZZZ",
            "--grid",
            "DN13",
            "--speed",
            "100",
        ])
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let expected = expected();
    let mut actual = Vec::new();
    let mut buffer = [0; 65535];
    loop {
        let (n, peer) = receiver.recv_from(&mut buffer).unwrap();
        let p = packet(&buffer[..n]);
        let done = matches!(p.body, Body::Close);
        actual.push(p);
        if actual.len() == 9 {
            stranger
                .send_to(
                    &wsjtx::encode(&Packet::new("WSJT-X", Body::Replay)).unwrap(),
                    peer,
                )
                .unwrap();
            receiver
                .send_to(
                    &wsjtx::encode(&Packet::new("wrong-id", Body::Replay)).unwrap(),
                    peer,
                )
                .unwrap();
            receiver.send_to(b"malformed", peer).unwrap();
            receiver
                .send_to(
                    &wsjtx::encode(&Packet::new("WSJT-X", Body::Replay)).unwrap(),
                    peer,
                )
                .unwrap();
        }
        if done {
            break;
        }
        if actual.len() > expected.len() {
            let _ = child.kill();
            panic!("unexpected response");
        }
    }
    assert!(child.wait().unwrap().success());
    assert_eq!(actual.len(), 159);
    for (i, (a, e)) in normalized(&actual)
        .iter()
        .zip(normalized(&expected))
        .enumerate()
    {
        assert_eq!(a, &e, "capture packet {i}");
    }
    assert!(matches_capture(&actual, &expected));
    // Planted output faults must be detected by the same full-field/order oracle.
    let mut bad = actual.clone();
    bad.swap(1, 2);
    assert!(
        !matches_capture(&bad, &expected),
        "out-of-order message passed"
    );
    let replay = actual
        .iter()
        .position(|p| matches!(&p.body,Body::Decode(d) if !d.new))
        .unwrap();
    let mut bad = actual.clone();
    if let Body::Decode(d) = &mut bad[replay].body {
        d.new = true;
    }
    assert!(!matches_capture(&bad, &expected), "wrong Replay passed");
    let mut bad = actual.clone();
    bad.insert(replay, actual[replay].clone());
    assert!(
        !matches_capture(&bad, &expected),
        "answer to non-target Replay passed"
    );
}
fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("qgi-sim-{name}-{}", std::process::id()))
}
#[test]
fn ft4_cycles_loop_timestamps_and_close_command() {
    let file = temp("ft4.txt");
    std::fs::write(&file,"260407_120000 7.0475 Rx FT4 -2 0.1 500 CQ W1AAA AA00\n260407_120007 7.0475 Rx FT4 -1 0.2 600 W1AAA W1AAB 73\n").unwrap();
    let receiver = socket();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["sim", "--wsjtx", "--file"])
        .arg(&file)
        .args([
            "--to",
            &receiver.local_addr().unwrap().to_string(),
            "--speed",
            "100",
            "--loop",
        ])
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let mut decodes = Vec::new();
    let mut tail = Vec::new();
    let mut buffer = [0; 65535];
    let mut sent = false;
    let start = Instant::now();
    loop {
        let (n, peer) = receiver.recv_from(&mut buffer).unwrap();
        let p = packet(&buffer[..n]);
        if let Body::Decode(d) = &p.body {
            decodes.push(d.clone());
        }
        if decodes.len() >= 3 && !sent {
            receiver
                .send_to(
                    &wsjtx::encode(&Packet::new("WSJT-X", Body::Close)).unwrap(),
                    peer,
                )
                .unwrap();
            sent = true;
        }
        tail.push(p.body.message_type());
        if matches!(p.body, Body::Close) {
            break;
        }
    }
    assert!(child.wait().unwrap().success());
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(decodes.len() >= 3);
    assert_eq!(
        (decodes[1].time.0 + 86_400_000 - decodes[0].time.0) % 86_400_000,
        7500
    );
    assert_eq!(
        (decodes[2].time.0 + 86_400_000 - decodes[1].time.0) % 86_400_000,
        7500
    );
    assert_eq!(&tail[tail.len() - 2..], [3, 6]);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn mixed_modes_play_known_periods_and_report_skips() {
    let file = temp("mixed-modes.txt");
    let modes = [
        ("FT8", 15_000, "~", ""),
        ("FT4", 7500, "+", ""),
        ("JT4", 60_000, "$", ""),
        ("JT65", 60_000, "#", ""),
        ("Q65-15A", 15_000, ":", "A"),
        ("Q65-30D", 30_000, ":", "D"),
        ("Q65-60C", 60_000, ":", "C"),
        ("Q65-120F", 120_000, ":", "F"),
        ("Q65-300E", 300_000, ":", "E"),
        ("FT8", 15_000, "~", ""),
        ("FT8_SH", 15_000, "~", ""),
    ];
    let mut text = String::from("header\n260407_120000 14 Tx FT8 -1 0 1500 CQ W1AAA AA00\n");
    for (i, (mode, _, _, _)) in modes.iter().enumerate() {
        // Unknown periods between playable lines must not abort or create empty cycles.
        text.push_str(&format!(
            "260407_12{i:02}00 14 Rx Q65 -1 0 1500 CQ W1AAA AA00\n\
             260407_12{i:02}00 14 Rx {mode} -2 0.1 1500 CQ W1AAB AA01\n"
        ));
    }
    for mode in ["JT9", "MSK144", "FST4", "FST4W", "UNKNOWN"] {
        text.push_str(&format!(
            "260407_123000 14 Rx {mode} -1 0 1500 CQ W1AAA AA00\n"
        ));
    }
    std::fs::write(&file, text).unwrap();
    let receiver = socket();
    let child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["sim", "--wsjtx", "--file"])
        .arg(&file)
        .args([
            "--to",
            &receiver.local_addr().unwrap().to_string(),
            "--speed",
            "1000",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut decodes = Vec::new();
    let mut statuses = Vec::new();
    let mut buf = [0; 65535];
    loop {
        let (n, _) = receiver.recv_from(&mut buf).unwrap();
        match packet(&buf[..n]).body {
            Body::Decode(d) => decodes.push(d),
            Body::Status(s) if s.decoding => statuses.push(s),
            Body::Close => break,
            _ => {}
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "11 cycles, 11 decodes from {} (18 lines skipped: Tx, not a decode, or a mode without an established period: FST4, FST4W, JT9, MSK144, Q65, UNKNOWN)\n",
            file.display()
        )
    );
    assert_eq!(decodes.len(), modes.len());
    assert_eq!(statuses.len(), modes.len());
    let mut elapsed = 0;
    for (i, (mode, period, wire_mode, submode)) in modes.iter().enumerate() {
        let d = &decodes[i];
        let s = &statuses[i];
        assert_eq!(
            (d.time.0 + 86_400_000 - decodes[0].time.0) % 86_400_000,
            elapsed
        );
        assert_eq!(d.mode.as_str(), Some(*wire_mode));
        assert_eq!(d.message.as_str(), Some("CQ W1AAB AA01"));
        assert_eq!(
            s.mode.as_str(),
            Some(if !submode.is_empty() {
                "Q65"
            } else if *mode == "FT8_SH" {
                "FT8"
            } else {
                mode
            })
        );
        assert_eq!(s.tx_mode, s.mode);
        assert_eq!(s.sub_mode.as_ref().unwrap().as_str(), Some(*submode));
        assert_eq!(
            s.tr_period_seconds,
            Some(if *period == 7500 {
                u32::MAX
            } else {
                period / 1000
            })
        );
        elapsed += period;
    }
    std::fs::remove_file(file).unwrap();
}

#[test]
fn all_unknown_modes_fail_with_skipped_modes() {
    let path = temp("unknown-modes.txt");
    std::fs::write(
        &path,
        "header\n260407_120000 14 Tx TXMODE -1 0 1 CQ\n260407_120000 14 Rx ZZZ -1 0 1 CQ\n260407_120000 14 Rx JT9 -1 0 1 CQ\n260407_120000 14 Rx Q65 -1 0 1 CQ\n260407_120000 14 Rx JT9 -1 0 1 CQ\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["sim", "--wsjtx", "--to", "127.0.0.1:2237", "--file"])
        .arg(&path)
        .args(["--from", "260407_120000"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "qg-interop: no received decodes in {} from 260407_120000 (6 lines skipped: Tx, not a decode, or a mode without an established period: JT9, Q65, ZZZ)\n",
            path.display()
        )
    );
    std::fs::remove_file(path).unwrap();
}
#[test]
fn native_and_qgi_capture_playback() {
    let receiver = socket();
    let path = temp("capture.jsonl");
    let packets = expected();
    let mut lines = Vec::new();
    let mut wire = Vec::new();
    for (i, p) in packets[..3].iter().enumerate() {
        let b = wsjtx::encode(p).unwrap();
        let e = wsjtx::Wsjtx::default()
            .decode(&b, &Default::default())
            .unwrap()
            .events
            .remove(0);
        let mut v = e.into_value();
        v["received_at"] = format!("2026-11-12T00:00:0{i}.000Z").into();
        lines.extend(serde_json::to_vec(&v).unwrap());
        lines.push(b'\n');
        wire.push(b);
    }
    std::fs::write(&path, &lines).unwrap();
    for qgi in [false, true] {
        let start = Instant::now();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_qg-interop"));
        cmd.args(["sim", "--capture"]).arg(&path).args([
            "--to",
            &receiver.local_addr().unwrap().to_string(),
            "--speed",
            "100",
        ]);
        if qgi {
            cmd.arg("--qgi");
        }
        let mut child = cmd.spawn().unwrap();
        let mut buf = [0; 65535];
        for original in &wire {
            let (n, _) = receiver.recv_from(&mut buf).unwrap();
            if qgi {
                let o = qso_graph_interop::qgi::Qgi::new(qso_graph_interop::qgi::Transport::Udp)
                    .decode(&buf[..n], &Default::default())
                    .unwrap();
                assert_eq!(o.events[0].native_bytes().unwrap(), *original);
            } else {
                assert_eq!(&buf[..n], original);
                packet(&buf[..n]);
            }
        }
        assert!(child.wait().unwrap().success());
        assert!(start.elapsed() >= Duration::from_millis(18));
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    std::fs::remove_file(path).unwrap();
}
#[test]
fn drop_in_validation_and_read_errors() {
    for (arg, value) in [
        ("--to", "localhost:2237"),
        ("--to", "127.0.0.1:0"),
        ("--call", "BAD!"),
        ("--grid", "dn13"),
        ("--grid", "SS00"),
        ("--id", ""),
        ("--speed", "NaN"),
        ("--speed", "0"),
        ("--speed", "1001"),
        ("--from", "2026-04-07"),
    ] {
        let args = if arg == "--to" {
            vec!["sim", "--wsjtx", arg, value]
        } else {
            vec!["sim", "--wsjtx", "--to", "127.0.0.1:2237", arg, value]
        };
        assert_eq!(
            Command::new(env!("CARGO_BIN_EXE_qg-interop"))
                .args(args)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2),
            "{arg} {value}"
        );
    }
    let path = temp("missing.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["sim", "--wsjtx", "--to", "127.0.0.1:2237", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot read "));
    std::fs::write(
        &path,
        "header\n260407_120000 14 Tx FT8 1 0 1 CQ W1AAA AA00\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args(["sim", "--wsjtx", "--to", "127.0.0.1:2237", "--file"])
        .arg(&path)
        .args(["--from", "260407_120000"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "qg-interop: no received decodes in {} from 260407_120000\n",
            path.display()
        )
    );
    std::fs::remove_file(path).unwrap();
}
#[cfg(unix)]
#[test]
fn interrupt_and_termination_send_clear_then_close() {
    for signal in ["-INT", "-TERM"] {
        let receiver = socket();
        let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
            .args([
                "sim",
                "--wsjtx",
                "--to",
                &receiver.local_addr().unwrap().to_string(),
                "--loop",
            ])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let mut buf = [0; 65535];
        let mut kinds = Vec::new();
        // The first decoding-end Status proves initialization and signal registration completed.
        loop {
            let (n, _) = receiver.recv_from(&mut buf).unwrap();
            let p = packet(&buf[..n]);
            if matches!(&p.body,Body::Status(s) if !s.decoding) {
                break;
            }
        }
        assert!(
            Command::new("kill")
                .args([signal, &child.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        loop {
            let (n, _) = receiver.recv_from(&mut buf).unwrap();
            let p = packet(&buf[..n]);
            kinds.push(p.body.message_type());
            if matches!(p.body, Body::Close) {
                break;
            }
        }
        assert!(child.wait().unwrap().success());
        assert_eq!(kinds, [3, 6]);
    }
}
