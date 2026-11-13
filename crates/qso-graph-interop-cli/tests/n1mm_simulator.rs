// SPDX-License-Identifier: GPL-3.0-or-later
//! Loopback peer tests with a receiver ledger independent of the simulator.
#![forbid(unsafe_code)]
use qso_graph_interop::{Codec, codec::DecodeOptions, n1mm::N1mm};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{SocketAddr, UdpSocket},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn socket() -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    socket
}
fn child(socket: &UdpSocket, args: &[&str]) -> Process {
    Process(
        Command::new(env!("CARGO_BIN_EXE_qg-interop"))
            .args([
                "sim",
                "--n1mm",
                "--to",
                &socket.local_addr().unwrap().to_string(),
            ])
            .args(args)
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
fn fields(bytes: &[u8]) -> Value {
    let out = N1mm::default()
        .decode(bytes, &DecodeOptions::default())
        .unwrap();
    assert!(out.skips.is_empty());
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0].native_bytes().unwrap(), bytes);
    out.events[0].value()["fields"].clone()
}
fn text(bytes: &[u8], name: &str) -> String {
    // This is an oracle for our simple generated XML, after the shared codec has
    // validated the whole document. Do not use it as an input protocol parser.
    std::str::from_utf8(bytes)
        .unwrap()
        .split_once(&format!("<{name}>"))
        .unwrap()
        .1
        .split_once(&format!("</{name}>"))
        .unwrap()
        .0
        .into()
}
fn root(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes)
        .unwrap()
        .split_once("?>")
        .unwrap()
        .1
        .strip_prefix('<')
        .unwrap()
        .split_once('>')
        .unwrap()
        .0
}
fn collect(
    mut process: Process,
    socket: &UdpSocket,
    mut react: impl FnMut(&[u8], SocketAddr),
) -> Vec<Vec<u8>> {
    let start = Instant::now();
    let mut packets = Vec::new();
    let mut buffer = [0; 65_535];
    loop {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "simulator did not exit"
        );
        match socket.recv_from(&mut buffer) {
            Ok((n, peer)) => {
                fields(&buffer[..n]);
                react(&buffer[..n], peer);
                packets.push(buffer[..n].to_vec());
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if let Some(status) = process.0.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
            }
            Err(e) => panic!("{e}"),
        }
    }
    packets
}
fn run(args: &[&str]) -> Vec<Vec<u8>> {
    let receiver = socket();
    collect(child(&receiver, args), &receiver, |_, _| {})
}

// Maintains latest radio states and the contact ledger just as a dashboard would.
// This checks every score snapshot, including snapshots immediately after edits.
fn consistent(packets: &[Vec<u8>], radios: usize) -> bool {
    let mut tx = [false; 2];
    let mut running = [false; 2];
    let mut active = 0;
    let mut contacts: BTreeMap<String, (String, u64)> = BTreeMap::new();
    for bytes in packets {
        let f = fields(bytes);
        match root(bytes) {
            "RadioInfo" => {
                let Some(n) = f["radio"]["number"]
                    .as_u64()
                    .filter(|&n| (1..=radios as u64).contains(&n))
                else {
                    return false;
                };
                for field in ["focus_radio", "active_radio"] {
                    if !f[field]
                        .as_u64()
                        .is_some_and(|n| (1..=radios as u64).contains(&n))
                    {
                        return false;
                    }
                }
                active = f["active_radio"].as_u64().unwrap();
                tx[n as usize - 1] = f["transmitting"].as_bool().unwrap();
                running[n as usize - 1] = f["running"].as_bool().unwrap();
                if tx.iter().filter(|&&b| b).count() > 1
                    || running.iter().filter(|&&b| b).count() != 1
                {
                    return false;
                }
            }
            "contactinfo" | "contactreplace" => {
                if f["radio"]["number"].as_u64() != Some(active) {
                    return false;
                }
                if !tx[active as usize - 1] {
                    return false;
                }
                let id = f["record_id"].as_str().unwrap().to_owned();
                let call = f["call"].as_str().unwrap().to_owned();
                if root(bytes) == "contactreplace" {
                    let Some((old_call, _)) = contacts.get(&id) else {
                        return false;
                    };
                    if &text(bytes, "oldcall") != old_call || f["previous_start"] != f["start"] {
                        return false;
                    }
                } else if contacts.contains_key(&id) {
                    return false;
                }
                contacts.insert(id, (call, text(bytes, "points").parse().unwrap()));
            }
            "contactdelete" => {
                if contacts.remove(f["record_id"].as_str().unwrap()).is_none() {
                    return false;
                }
            }
            "dynamicresults" => {
                let points: u64 = contacts.values().map(|(_, points)| points).sum();
                if f["qso_count"] != contacts.len()
                    || f["points"] != points
                    || f["total_score"] != points
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    !tx.iter().any(|&b| b)
}
#[test]
fn contest_traffic_so2r_scores_and_planted_failures() {
    // 35 exercises a replacement followed by deletion of the same record.
    let packets = run(&[
        "--radios", "2", "--rate", "3600", "--speed", "20", "--count", "35",
    ]);
    assert!(consistent(&packets, 2));
    let fixture: Value = serde_json::from_str(include_str!(
        "../../qso-graph-interop/tests/fixtures/n1mm-simulator.json"
    ))
    .unwrap();
    assert_eq!(packets[0], fixture["app_xml"].as_str().unwrap().as_bytes());
    assert_eq!(
        packets[1],
        fixture["radio_xml"].as_str().unwrap().as_bytes()
    );
    assert_eq!(root(&packets[0]), "AppInfo");
    assert_eq!(text(&packets[0], "mycall"), "N0CALL");
    assert_eq!(text(&packets[0], "StationName"), "SYNTHETIC-01");
    for (name, expected) in [
        ("AppInfo", 1),
        ("contactinfo", 35),
        ("contactreplace", 7),
        ("contactdelete", 5),
        ("spot", 70),
        ("dynamicresults", 48),
    ] {
        assert_eq!(
            packets.iter().filter(|b| root(b) == name).count(),
            expected,
            "{name}"
        );
    }
    let contacts: Vec<_> = packets
        .iter()
        .filter(|b| root(b) == "contactinfo")
        .collect();
    let mut first_contact = fields(contacts[0]);
    // Calls vary with the seed; all other fields have hand-authored expectations.
    assert!(
        first_contact
            .as_object_mut()
            .unwrap()
            .remove("call")
            .is_some()
    );
    assert_eq!(first_contact, fixture["first_contact_fields"]);
    assert_eq!(
        fields(
            packets
                .iter()
                .rev()
                .find(|b| root(b) == "dynamicresults")
                .unwrap()
        ),
        fixture["last_score_fields"]
    );
    for (i, bytes) in contacts.iter().enumerate() {
        let f = fields(bytes);
        assert_eq!(f["radio"]["number"], i % 2 + 1);
        assert_eq!(f["radio"]["instance"], "SYNTHETIC-01");
        assert_eq!(
            f["dial_frequency_hz"],
            if i % 2 == 0 { 14_025_000 } else { 7_025_000 }
        );
        assert_eq!(
            text(bytes, "timestamp"),
            format!("2026-01-01 00:00:{:02}", i + 1)
        );
        assert_eq!(text(bytes, "sntnr"), format!("{:03}", i + 1));
    }
    let mut spots = BTreeSet::new();
    for b in packets.iter().filter(|b| root(b) == "spot") {
        let f = fields(b);
        assert!(matches!(
            f["frequency_hz"].as_u64(),
            Some(14_025_000 | 7_025_000)
        ));
        let key = (
            text(b, "dxcall"),
            text(b, "frequency"),
            text(b, "timestamp"),
        );
        if text(b, "action") == "add" {
            assert!(spots.insert(key));
        } else {
            assert!(spots.remove(&key));
        }
    }
    assert!(spots.is_empty());
    assert_eq!(
        fields(
            packets
                .iter()
                .rev()
                .find(|b| root(b) == "dynamicresults")
                .unwrap()
        )["points"],
        66
    );

    // Force both latest RadioNr states to say TX using valid, decodable XML.
    let mut bad = packets.clone();
    let second_tx = bad
        .iter()
        .position(|b| {
            root(b) == "RadioInfo"
                && fields(b)["radio"]["number"] == 2
                && fields(b)["transmitting"] == true
        })
        .unwrap();
    let xml = String::from_utf8(bad[second_tx - 1].clone()).unwrap();
    bad[second_tx - 1] = xml
        .replace(
            "<IsTransmitting>False</IsTransmitting>",
            "<IsTransmitting>True</IsTransmitting>",
        )
        .into_bytes();
    assert!(
        !consistent(&bad, 2),
        "two simultaneous transmitters escaped the oracle"
    );
    let mut bad = packets.clone();
    let score = bad
        .iter()
        .position(|b| root(b) == "dynamicresults")
        .unwrap();
    bad[score] = String::from_utf8(bad[score].clone())
        .unwrap()
        .replace("<score>0</score>", "<score>1</score>")
        .into_bytes();
    assert!(
        !consistent(&bad, 2),
        "inconsistent score escaped the contact ledger"
    );
    let mut bad = packets.clone();
    let qso = bad.iter().position(|b| root(b) == "contactinfo").unwrap();
    bad[qso] = String::from_utf8(bad[qso].clone())
        .unwrap()
        .replace("<radionr>1</radionr>", "<radionr>2</radionr>")
        .into_bytes();
    assert!(
        !consistent(&bad, 2),
        "wrong contact radio escaped the oracle"
    );
}

#[test]
fn seed_determinism_single_radio_and_option_effects() {
    let args = [
        "--count",
        "8",
        "--rate",
        "3600",
        "--speed",
        "20",
        "--seed",
        "0",
        "--call",
        "q1zzz/p",
        "--station",
        "SYNTHETIC & <PC>",
    ];
    let first = run(&args);
    assert_eq!(first, run(&args));
    assert!(consistent(&first, 1));
    assert_eq!(text(&first[0], "mycall"), "Q1ZZZ/P");
    for b in first.iter().filter(|b| root(b) == "contactinfo") {
        assert_eq!(fields(b)["radio"]["instance"], "SYNTHETIC & <PC>");
    }
    let mut different = args;
    different[7] = "1";
    assert_ne!(first, run(&different));
    let mut faster = args;
    faster[5] = "40";
    assert_eq!(first, run(&faster), "speed changed logical traffic");
    let looped = run(&["--loop", "--count", "13", "--rate", "3600", "--speed", "40"]);
    assert_eq!(
        looped.iter().filter(|b| root(b) == "contactinfo").count(),
        13
    );
    assert!(consistent(&looped, 1));
    let default = run(&["--rate", "3600", "--speed", "40"]);
    assert_eq!(
        default.iter().filter(|b| root(b) == "contactinfo").count(),
        12
    );
}
fn qsy(radio: u8, frequency: &str) -> Vec<u8> {
    format!("<radio_setfrequency><app>synthetic-client</app><radionr>{radio}</radionr><frequency>{frequency}</frequency><mousebutton>{}</mousebutton></radio_setfrequency>", if radio == 1 { "Left" } else { "Right" }).into_bytes()
}
fn correct_frequencies(packets: &[Vec<u8>]) -> bool {
    let mut observed = [false; 2];
    for b in packets.iter().filter(|b| root(b) == "RadioInfo") {
        let f = fields(b);
        let n = f["radio"]["number"].as_u64().unwrap() as usize - 1;
        let expected = [14_053_280, 7_061_200][n];
        let hz = f["dial_frequency_hz"].as_u64().unwrap();
        if observed[n] && hz != expected {
            return false;
        }
        if hz == expected {
            observed[n] = true;
        }
    }
    observed == [true, true]
}
#[test]
fn qsy_from_target_only_and_spectrum_has_no_output_effect_or_ack() {
    let receiver = socket();
    let stranger = socket();
    let process = child(
        &receiver,
        &[
            "--radios", "2", "--count", "10", "--rate", "3600", "--speed", "5",
        ],
    );
    let mut initial = false;
    let mut foreign = false;
    let packets = collect(process, &receiver, |b, peer| {
        if !initial {
            receiver.send_to(&qsy(1, "14053.28"), peer).unwrap();
            receiver.send_to(&qsy(2, "7061.2"), peer).unwrap();
            receiver.send_to(b"malformed", peer).unwrap();
            receiver.send_to(&qsy(3, "21000"), peer).unwrap();
            initial = true;
        }
        if !foreign && root(b) == "RadioInfo" && fields(b)["dial_frequency_hz"] == 7_061_200 {
            stranger.send_to(&qsy(1, "21000"), peer).unwrap();
            stranger.send_to(&qsy(2, "28000"), peer).unwrap();
            receiver.send_to(b"<Spectrum><app>synthetic-client</app><Name>SYNTHETIC</Name><LowScopeFrequency>14000</LowScopeFrequency><HighScopeFrequency>14055</HighScopeFrequency><ScalingFactor>1</ScalingFactor><DataCount>2</DataCount><SpectrumData>10,20</SpectrumData></Spectrum>", peer).unwrap();
            foreign = true;
        }
    });
    assert!(initial && foreign);
    assert!(correct_frequencies(&packets));
    assert!(consistent(&packets, 2));
    for radio in [1, 2] {
        let states: Vec<_> = packets
            .iter()
            .filter(|b| root(b) == "RadioInfo" && fields(b)["radio"]["number"] == radio)
            .collect();
        // Initial state was sent before the command could arrive. The next report
        // must already carry the tuned frequency, without an immediate reply.
        assert_eq!(
            fields(states[1])["dial_frequency_hz"],
            if radio == 1 { 14_053_280 } else { 7_061_200 }
        );
    }
    let baseline_socket = socket();
    let baseline_process = child(
        &baseline_socket,
        &[
            "--radios", "2", "--count", "10", "--rate", "3600", "--speed", "5",
        ],
    );
    let mut sent = false;
    let baseline = collect(baseline_process, &baseline_socket, |_, peer| {
        if !sent {
            baseline_socket.send_to(&qsy(1, "14053.28"), peer).unwrap();
            baseline_socket.send_to(&qsy(2, "7061.2"), peer).unwrap();
            sent = true;
        }
    });
    assert_eq!(
        packets, baseline,
        "Spectrum, invalid input, or a foreign QSY changed output or produced an acknowledgment"
    );
    // All traffic has scheduled broadcast roots; there is no command reply.
    assert!(packets.iter().all(|b| matches!(
        root(b),
        "AppInfo"
            | "RadioInfo"
            | "contactinfo"
            | "contactreplace"
            | "contactdelete"
            | "spot"
            | "dynamicresults"
    )));
    for b in packets.iter().filter(|b| root(b) == "contactinfo") {
        let f = fields(b);
        assert_eq!(
            f["dial_frequency_hz"],
            if f["radio"]["number"] == 1 {
                14_053_280
            } else {
                7_061_200
            }
        );
    }
    // Plant obedience to the foreign QSY in a later, valid RadioInfo packet.
    let mut bad = packets;
    let last = bad.iter().rposition(|b| root(b) == "RadioInfo").unwrap();
    bad[last] = String::from_utf8(bad[last].clone())
        .unwrap()
        .replace("<Freq>706120</Freq>", "<Freq>2800000</Freq>")
        .into_bytes();
    assert!(
        !correct_frequencies(&bad),
        "foreign QSY obedience escaped the oracle"
    );
}

#[test]
fn invalid_options_fail_before_sending() {
    for (arg, value) in [
        ("--to", "localhost:12060"),
        ("--to", "127.0.0.1:0"),
        ("--radios", "0"),
        ("--radios", "3"),
        ("--radios", "1.5"),
        ("--rate", "0"),
        ("--rate", "3601"),
        ("--rate", "NaN"),
        ("--rate", "-1"),
        ("--count", "0"),
        ("--count", "100001"),
        ("--count", "-1"),
        ("--seed", "-1"),
        ("--seed", "18446744073709551616"),
        ("--station", ""),
        ("--station", "  "),
        ("--station", "bad\nname"),
        ("--station", &"x".repeat(65)),
        ("--call", "BAD!"),
        ("--speed", "0"),
        ("--speed", "NaN"),
        ("--speed", "inf"),
        ("--speed", "1001"),
    ] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_qg-interop"));
        cmd.args(["sim", "--n1mm"]);
        if arg != "--to" {
            cmd.args(["--to", "127.0.0.1:12060"]);
        }
        assert_eq!(
            cmd.args([arg, value]).output().unwrap().status.code(),
            Some(2),
            "{arg} {value:?}"
        );
    }
    for incompatible in [
        "--wsjtx",
        "--capture",
        "--file",
        "--from",
        "--grid",
        "--id",
        "--dx-call",
        "--mode",
        "--frequency",
        "--qgi",
    ] {
        let mut args = vec!["sim", "--n1mm", "--to", "127.0.0.1:12060", incompatible];
        if !matches!(incompatible, "--wsjtx" | "--qgi") {
            args.push("ignored");
        }
        assert_eq!(
            Command::new(env!("CARGO_BIN_EXE_qg-interop"))
                .args(args)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2),
            "{incompatible}"
        );
    }
    for option in ["--station", "--radios", "--rate", "--seed", "--count"] {
        assert_eq!(
            Command::new(env!("CARGO_BIN_EXE_qg-interop"))
                .args(["sim", "--wsjtx", "--to", "127.0.0.1:2237", option, "1"])
                .output()
                .unwrap()
                .status
                .code(),
            Some(2),
            "{option} accepted without --n1mm"
        );
    }
    assert_eq!(
        Command::new(env!("CARGO_BIN_EXE_qg-interop"))
            .args(["sim", "--n1mm"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
    // An unrepresentably slow schedule must fail before opening a socket.
    let receiver = socket();
    let out = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "sim",
            "--n1mm",
            "--to",
            &receiver.local_addr().unwrap().to_string(),
            "--speed",
            "1e-300",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let mut buffer = [0; 65_535];
    assert!(receiver.recv_from(&mut buffer).is_err());
}

#[cfg(unix)]
#[test]
fn loop_interrupt_and_termination_clear_transmit_state() {
    for signal in ["-INT", "-TERM"] {
        let receiver = socket();
        let process = child(
            &receiver,
            &["--radios", "2", "--rate", "3600", "--speed", "5", "--loop"],
        );
        let pid = process.0.id().to_string();
        let mut sent = false;
        let packets = collect(process, &receiver, |b, _| {
            if !sent && root(b) == "contactinfo" {
                assert!(
                    Command::new("kill")
                        .args([signal, &pid])
                        .status()
                        .unwrap()
                        .success()
                );
                sent = true;
            }
        });
        assert!(sent);
        assert!(consistent(&packets, 2));
        for b in &packets[packets.len() - 2..] {
            assert_eq!(root(b), "RadioInfo");
            assert_eq!(fields(b)["transmitting"], false);
        }
    }
}
