// SPDX-License-Identifier: GPL-3.0-or-later
//! Scripted observer rules and loopback-only CLI submissions.
#![forbid(unsafe_code)]
use qso_graph_interop::{Codec, DecodeOptions, pskreporter_ipfix::PskReporterIpfix};
use qso_graph_interop_cli::collector::Collector;
use serde_json::Value;
use std::{net::UdpSocket, process::Command, time::Duration};
fn fixture(name: &str) -> Vec<u8> {
    let f: Value = serde_json::from_str(include_str!(
        "../../qso-graph-interop/tests/fixtures/pskreporter-ipfix.json"
    ))
    .unwrap();
    f["profiles"][name]["wire_hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
fn data_only(bytes: &[u8]) -> Vec<u8> {
    let mut p = 16;
    for _ in 0..2 {
        p += usize::from(u16::from_be_bytes([bytes[p + 2], bytes[p + 3]]));
    }
    let mut raw = bytes[..16].to_vec();
    raw.extend(&bytes[p..]);
    let len = raw.len() as u16;
    raw[2..4].copy_from_slice(&len.to_be_bytes());
    raw
}
#[test]
fn interval_duplicates_templates_and_recommendations_use_observer_time() {
    let raw = fixture("wsjtx");
    let mut observer = Collector::default();
    assert!(observer.observe(&raw, b"peer-1", 0).unwrap().1.is_empty());
    let (_, diag) = observer
        .observe(&data_only(&raw), b"peer-1", 299_999)
        .unwrap();
    for rule in ["H.1-INTERVAL", "H.1-DUPLICATE", "H.1-STARTUP"] {
        assert!(diag.iter().any(|d| d.rule == rule), "{rule}");
    }
    assert!(
        diag.iter()
            .all(|d| d.set_index.is_some() && d.record_index.is_some())
    );
    let (_, diag) = observer
        .observe(&data_only(&raw), b"peer-1", 3_900_000)
        .unwrap();
    assert!(diag.iter().any(|d| d.rule == "H.1-TEMPLATES"));
    assert!(!diag.iter().any(|d| d.rule == "H.1-UNCHANGED"));
    let mut other = Collector::default();
    other.observe(&raw, b"peer-1", 0).unwrap();
    assert!(
        !other
            .observe(&raw, b"peer-1", 300_000)
            .unwrap()
            .1
            .iter()
            .any(|d| d.rule == "H.1-INTERVAL" || d.rule == "H.1-DUPLICATE")
    );
    assert!(
        other
            .observe(&raw, b"peer-2", 300_001)
            .unwrap()
            .1
            .is_empty()
    );
    assert!(other.observe(&raw, b"peer-1", 1).is_err());
}
#[test]
fn legacy_scope_padding_and_bad_record_content_are_located() {
    let raw = fixture("fldigi");
    let diag = Collector::default().observe(&raw, b"peer", 0).unwrap().1;
    let scope = diag.iter().find(|d| d.rule == "H.1-SCOPE").unwrap();
    assert_eq!(scope.set_index, Some(2));
    assert_eq!(scope.record_index, Some(0));
    let mut raw = fixture("wsjtx");
    let call = raw.windows(5).rposition(|v| v == b"Q1AAA").unwrap();
    raw[call] = 0xff;
    let diag = Collector::default().observe(&raw, b"peer", 0).unwrap().1;
    assert!(
        diag.iter()
            .any(|d| d.rule == "H.1-FIELDS" && d.record_index == Some(0))
    );
    let mut raw = fixture("wsjtx");
    let mut p = 16;
    while p < raw.len() {
        let len = usize::from(u16::from_be_bytes([raw[p + 2], raw[p + 3]]));
        if raw[p..p + 2] == [0, 3] {
            raw[p + len - 1] = 7;
            break;
        }
        p += len;
    }
    assert!(
        Collector::default()
            .observe(&raw, b"peer", 0)
            .unwrap()
            .1
            .iter()
            .any(|d| d.rule == "H.1-PADDING")
    );
}
#[test]
fn simulator_sends_both_profiles_only_to_explicit_numeric_loopback() {
    let binary = env!("CARGO_BIN_EXE_qg-interop");
    for profile in ["wsjtx", "fldigi"] {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let result = Command::new(binary)
            .args([
                "sim",
                "--pskreporter-ipfix",
                "--to",
                &receiver.local_addr().unwrap().to_string(),
                "--profile",
                profile,
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let mut bytes = [0; 2048];
        let (n, _) = receiver.recv_from(&mut bytes).unwrap();
        let out = PskReporterIpfix::default()
            .decode(&bytes[..n], &DecodeOptions::default())
            .unwrap();
        assert_eq!(out.events.len(), 2);
        assert!(
            out.events[1].value()["fields"]["sender_call"]
                .as_str()
                .unwrap()
                .starts_with('Q')
        );
    }
    for args in [
        vec!["collect", "--pskreporter-ipfix", "[::ffff:239.1.2.3]:4739"],
        vec![
            "sim",
            "--pskreporter-ipfix",
            "--to",
            "[::ffff:239.1.2.3]:4739",
        ],
        vec!["sim", "--pskreporter-ipfix"],
        vec!["sim", "--pskreporter-ipfix", "--to", "example.invalid:4739"],
        vec!["sim", "--pskreporter-ipfix", "--to", "239.1.2.3:4739"],
        vec!["collect", "--pskreporter-ipfix", "239.1.2.3:4739"],
        vec!["collect", "--pskreporter-ipfix", "[ff02::1]:4739"],
        vec!["collect", "--pskreporter-ipfix", "example.invalid:4739"],
    ] {
        assert!(
            !Command::new(binary)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
#[test]
fn collector_emits_ndjson_and_survives_a_malformed_datagram() {
    use std::process::Stdio;
    use std::time::Instant;
    let reservation = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let mut child = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "collect",
            "--pskreporter-ipfix",
            &address.to_string(),
            "--count",
            "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while UdpSocket::bind(address).is_ok() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            panic!("collector did not bind");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    sender.send_to(b"bad", address).unwrap();
    sender.send_to(&fixture("wsjtx"), address).unwrap();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            panic!("collector did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let lines: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["kind"], "receiver_information");
    assert_eq!(lines[1]["kind"], "spot");
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("H.1-FRAMING")
    );
}
#[test]
fn future_version_sets_are_never_interpreted_by_the_observer() {
    let mut bytes = vec![0, 11, 0, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    bytes.extend([0, 2, 0, 8, 1, 0, 255, 255]);
    let (out, diagnostics) = Collector::default().observe(&bytes, b"peer", 0).unwrap();
    assert!(out.events.is_empty());
    assert_eq!(out.skips.len(), 1);
    assert_eq!(diagnostics[0].rule, "H.1-FRAMING");
}

#[test]
fn ipfix_record_capture_requires_qgi_instead_of_sending_a_record_as_a_datagram() {
    let f: Value = serde_json::from_str(include_str!(
        "../../qso-graph-interop/tests/fixtures/pskreporter-ipfix.json"
    ))
    .unwrap();
    let path =
        std::env::temp_dir().join(format!("qgi-ipfix-capture-{}.ndjson", std::process::id()));
    std::fs::write(
        &path,
        serde_json::to_vec(&f["profiles"]["wsjtx"]["events"][0]).unwrap(),
    )
    .unwrap();
    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_qg-interop"))
        .args([
            "sim",
            "--capture",
            path.to_str().unwrap(),
            "--to",
            &receiver.local_addr().unwrap().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("one record")
    );
    let mut bytes = [0; 4096];
    assert!(receiver.recv_from(&mut bytes).is_err());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn fullness_exception_includes_the_next_records_alignment_padding() {
    use qso_graph_interop::{
        Event,
        pskreporter_ipfix::{SubmissionOptions, SubmissionProfile, encode_submission},
    };
    let f: Value = serde_json::from_str(include_str!(
        "../../qso-graph-interop/tests/fixtures/pskreporter-ipfix.json"
    ))
    .unwrap();
    for (software_len, expected_size, violation) in [(242, 676, true), (246, 680, false)] {
        let mut receiver = f["profiles"]["fldigi"]["events"][0].clone();
        receiver["fields"]["reporting_software"] = "s".repeat(software_len).into();
        receiver["fields"]["antenna_information"] = "a".repeat(254).into();
        let mut spot = f["profiles"]["fldigi"]["events"][1].clone();
        for field in ["reporting_software", "antenna_information"] {
            spot["fields"][field] = receiver["fields"][field].clone();
        }
        let mut options = SubmissionOptions::default();
        options.profile = SubmissionProfile::Fldigi;
        let raw = encode_submission(
            &Event::from_value(receiver).unwrap(),
            &[Event::from_value(spot).unwrap()],
            &options,
        )
        .unwrap();
        assert_eq!(raw.len(), expected_size);
        let mut collector = Collector::default();
        collector.observe(&raw, b"peer", 0).unwrap();
        let (_, d) = collector.observe(&raw, b"peer", 1).unwrap();
        assert_eq!(d.iter().any(|d| d.rule == "H.1-INTERVAL"), violation);
    }
}
