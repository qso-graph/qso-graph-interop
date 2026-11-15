// SPDX-License-Identifier: GPL-3.0-or-later
//! Byte-exact lab evidence; record numbers are one-based recorder NDJSON lines.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Codec, DecodeOptions, EncodeOptions, Event,
    conformance::check_hash,
    fldigi::{Fldigi, RequestAssociation},
    n1mm::N1mm,
    wsjtx::Wsjtx,
};
use serde_json::{Value, json};

const CAPTURES: [(&str, &str); 5] = [
    (
        "fldigi-4.2.13-xmlrpc.ndjson",
        include_str!("fixtures/live-captures/fldigi-4.2.13-xmlrpc.ndjson"),
    ),
    (
        "n1mm-1.0.11480-qsy-sent.ndjson",
        include_str!("fixtures/live-captures/n1mm-1.0.11480-qsy-sent.ndjson"),
    ),
    (
        "n1mm-1.0.11480-udp.ndjson",
        include_str!("fixtures/live-captures/n1mm-1.0.11480-udp.ndjson"),
    ),
    (
        "wsjtx-3.3.0-pr-ft8-live-udp.ndjson",
        include_str!("fixtures/live-captures/wsjtx-3.3.0-pr-ft8-live-udp.ndjson"),
    ),
    (
        "wsjtx-beta6.ndjson",
        include_str!("fixtures/live-captures/wsjtx-beta6.ndjson"),
    ),
];
fn records(index: usize) -> Vec<Value> {
    CAPTURES[index]
        .1
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn bytes(record: &Value, key: &str) -> Vec<u8> {
    // Independent recorder-envelope unpacking, not the codec's native-data decoder.
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut pending = 0_u32;
    let mut bits = 0;
    let mut out = Vec::new();
    for b in record[key].as_str().unwrap().trim_end_matches('=').bytes() {
        let digit = alphabet.iter().position(|v| *v == b).unwrap() as u32;
        pending = (pending << 6) | digit;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((pending >> bits) as u8);
        }
    }
    out
}
fn decoded(codec: &mut impl Codec, wire: &[u8], options: &DecodeOptions<'_>) -> Event {
    let outcome = codec.decode(wire, options).unwrap();
    assert!(outcome.skips.is_empty());
    assert_eq!(outcome.events.len(), 1);
    let event = outcome.events.into_iter().next().unwrap();
    assert_eq!(
        codec
            .reencode(&event, &EncodeOptions::default(), &options.verification())
            .unwrap(),
        wire
    );
    event
}
fn leaf<'a>(xml: &'a str, name: &str) -> &'a str {
    xml.split_once(&format!("<{name}>"))
        .unwrap()
        .1
        .split_once(&format!("</{name}>"))
        .unwrap()
        .0
}

#[test]
fn recorder_files_match_original_sha256sums() {
    let sums = include_str!("fixtures/live-captures/SHA256SUMS");
    assert_eq!(sums.lines().count(), CAPTURES.len());
    for (name, text) in CAPTURES {
        let hash = sums
            .lines()
            .find_map(|line| {
                let (hash, file) = line.split_once("  ")?;
                (file == name).then_some(hash)
            })
            .unwrap();
        assert!(check_hash(text.as_bytes(), hash), "{name}");
        let mut planted = text.as_bytes().to_vec();
        planted[0] ^= 1;
        assert!(!check_hash(&planted, hash));
    }
}

#[test]
fn n1mm_live_broadcasts_and_truncated_peer_payloads() {
    let rows = records(2);
    assert_eq!(rows.len(), 130);
    let options = DecodeOptions::default().with_structured(true);
    let mut codec = N1mm::default();
    let mut counts = std::collections::BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let wire = bytes(row, "data_b64");
        let xml = std::str::from_utf8(&wire).unwrap();
        if let Some(ending) = match index + 1 {
            19 | 38 => Some("</contacti"),
            33 => Some("</co"),
            51 => Some("</contactrepl"),
            _ => None,
        } {
            assert!(xml.ends_with(ending));
            assert_eq!(
                codec.decode(&wire, &options).unwrap_err().category(),
                Category::Truncated
            );
            // Text is evidence even when the enclosing document is incomplete.
            match index + 1 {
                33 => {
                    assert_eq!(leaf(xml, "name"), "Zoë");
                    assert_eq!(leaf(xml, "comment"), "nicht ÄÖÜ ß");
                }
                38 => {
                    assert_eq!(leaf(xml, "name"), "Ñandú");
                    assert_eq!(leaf(xml, "comment"), "??? test");
                }
                51 => {
                    assert_eq!(leaf(xml, "operator"), " ");
                    assert_eq!(leaf(xml, "gridsquare"), " ");
                }
                _ => {}
            }
            continue;
        }
        let event = decoded(&mut codec, &wire, &options);
        let v = event.value();
        let f = &v["fields"];
        let root = v["native"]["message"]["root"].as_str().unwrap();
        *counts.entry(root.to_owned()).or_insert(0) += 1;
        match root {
            "contactinfo" => {
                let id = f["record_id"].as_str().unwrap();
                assert_eq!(id.len(), 32);
                assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
                assert_eq!(f["dial_frequency_hz"], 14_200_000);
                assert_eq!(f["tx_frequency_hz"], 14_200_000);
                assert_eq!(f["operator_call"], "");
                assert_eq!(f["grid"], "");
                assert_eq!(f["exchange_received"], "");
                if index + 1 == 23 {
                    assert_eq!(f["comments"], "xml <&>\"' test");
                }
            }
            "contactdelete" => assert_eq!(
                f,
                &json!({"call":"Q5GHI","station_call":"","band":"14","radio":{"instance":"BUILD-BASE"},"record_id":"ca78d3f0affe4f46acb0eb535852f959","start":"2026-11-15T00:09:11Z"})
            ),
            "spot" => assert_eq!(f["frequency_hz"], 14_200_000),
            "RadioInfo" => {
                let freq = leaf(xml, "Freq").parse::<i64>().unwrap();
                assert_eq!(f["dial_frequency_hz"], freq * 10);
                assert_eq!(f["running"], false);
                assert_eq!(f["transmitting"], false);
            }
            "dynamicresults" => {
                assert_eq!(f["total_score"], leaf(xml, "score").parse::<i64>().unwrap());
                if index + 1 == 18 {
                    assert_eq!(f["qso_count"], 1);
                    assert_eq!(f["points"], 1);
                }
            }
            "AppInfo" => {
                assert!(xml.contains("<contestname />"));
                assert_eq!(f, &json!({}));
            }
            _ => panic!("unexpected root at record {}", index + 1),
        }
    }
    assert_eq!(
        counts,
        std::collections::BTreeMap::from([
            ("RadioInfo".into(), 61),
            ("dynamicresults".into(), 51),
            ("spot".into(), 10),
            ("contactinfo".into(), 2),
            ("contactdelete".into(), 1),
            ("AppInfo".into(), 1),
        ])
    );
    // A complete captured packet with a planted incomplete closing tag must fail too.
    let mut planted = bytes(&rows[13], "data_b64");
    planted.pop();
    assert_eq!(
        codec.decode(&planted, &options).unwrap_err().category(),
        Category::Truncated
    );
}

#[test]
fn n1mm_sent_qsy_values_and_following_radio_observations() {
    let sent = records(1);
    let broadcasts = records(2);
    assert_eq!(sent.len(), 9);
    let mut codec = N1mm::default();
    for (i, row) in sent.iter().enumerate() {
        let wire = bytes(row, "data_b64");
        let xml = std::str::from_utf8(&wire).unwrap();
        let event = decoded(&mut codec, &wire, &DecodeOptions::default());
        let expected = [
            Some(14_053_284),
            Some(7_074_000),
            None,
            Some(14_074_000),
            Some(14_075_000),
            Some(14_076_000),
            Some(14_077_000),
            Some(0),
            Some(14_200_000),
        ][i];
        if let Some(hz) = expected {
            assert_eq!(event.value()["fields"]["frequency_hz"], hz);
            if i != 4 {
                let radio = if matches!(i, 5 | 6) { 2 } else { 1 };
                let command = Event::from_value(json!({"spec":"0.2.0","family":"n1mm","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":{"operation":"set_frequency","app":"qgi-capture","radio":{"number":radio},"frequency_hz":hz}})).unwrap();
                let encoded = codec
                    .encode_command(&command, &EncodeOptions::default())
                    .unwrap();
                let encoded = std::str::from_utf8(&encoded).unwrap();
                for tag in ["app", "radionr", "frequency"] {
                    assert_eq!(leaf(encoded, tag), leaf(xml, tag));
                }
                // The canonical writer always adds its radio-derived mousebutton.
                assert_eq!(
                    leaf(encoded, "mousebutton"),
                    if radio == 1 { "Left" } else { "Right" }
                );
            }
        } else {
            // Sub-Hz input cannot be represented by integer common Hz; keep native precision.
            assert!(event.value()["fields"].get("frequency_hz").is_none());
            assert_eq!(leaf(xml, "frequency"), "21074.1234567");
        }
    }
    // First relevant RadioInfo after each sent command; ignored commands use periodic reports.
    for (record, hz, active) in [
        (77, 14_053_280, 1),
        (80, 7_074_000, 1),
        (83, 21_074_120, 1),
        (88, 14_074_000, 1),
        (91, 14_074_000, 1),
        (93, 14_074_000, 2),
        (95, 14_074_000, 1),
        (98, 14_074_000, 1),
        (102, 14_200_000, 1),
    ] {
        let event = decoded(
            &mut codec,
            &bytes(&broadcasts[record - 1], "data_b64"),
            &DecodeOptions::default(),
        );
        assert_eq!(event.value()["fields"]["dial_frequency_hz"], hz);
        assert_eq!(event.value()["fields"]["active_radio"], active);
    }
}

fn http_body(wire: &[u8]) -> &[u8] {
    let boundary = wire.windows(4).position(|v| v == b"\r\n\r\n").unwrap();
    let headers = std::str::from_utf8(&wire[..boundary]).unwrap();
    let body = &wire[boundary + 4..];
    assert!(
        headers.starts_with("HTTP/1.1 200 OK\r\n")
            || headers.starts_with("POST /RPC2 HTTP/1.1\r\n")
    );
    assert!(headers.contains("Content-Type: text/xml"));
    let length = headers
        .lines()
        .find_map(|line| {
            line.strip_prefix("Content-length: ")
                .or_else(|| line.strip_prefix("Content-Length: "))
        })
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert_eq!(body.len(), length);
    body
}

#[test]
fn fldigi_live_requests_responses_and_fault() {
    let rows = records(0);
    assert_eq!(rows.len(), 61);
    let mut codec = Fldigi::default();
    let mut responses = 0;
    for (i, row) in rows.iter().enumerate() {
        let request_wire = bytes(row, "request_b64");
        let request = http_body(&request_wire);
        let request_event = decoded(&mut codec, request, &DecodeOptions::default());
        assert_eq!(request_event.value()["family"], "fldigi");
        let association = RequestAssociation::from_request_body(request).unwrap();
        assert_eq!(association.as_value()["method"], row["method"]);
        let wire = bytes(row, "response_b64");
        if row["timed_out"] == true {
            assert_eq!(i + 1, 19);
            assert!(wire.is_empty());
            continue; // No response was received: absence is not an XML-RPC message.
        }
        let options = DecodeOptions::default().with_metadata(association.into());
        let event = decoded(&mut codec, http_body(&wire), &options);
        let f = &event.value()["fields"];
        match i + 1 {
            1 => assert_eq!(event.value()["native_version"], "4.2.13"),
            7 | 29 | 56 => assert_eq!(f["dial_frequency_hz"], 14_070_000),
            53 => assert_eq!(f["frequency_hz"], 14_070_000),
            54 => assert_eq!(f["dx_call"], "Q1ABC"),
            58 => {
                assert_eq!(f["status"], "error");
                assert_eq!(f["native_code"], "-1");
                assert_eq!(f["text"], "no.such_method: unknown method name");
            }
            59 => {
                assert_eq!(wire.len(), 3237);
                assert_eq!(http_body(&wire).len(), 3150);
            }
            _ => {}
        }
        responses += 1;
    }
    assert_eq!(responses, 60);
}

#[test]
fn wsjtx_live_ft8_and_beta6() {
    let oracles: Value =
        serde_json::from_str(include_str!("fixtures/live-captures/ft8-oracles.json")).unwrap();
    for (index, count) in [(3, 20), (4, 40)] {
        let rows = records(index);
        assert_eq!(rows.len(), count);
        let mut codec = Wsjtx::default();
        let mut decodes = 0;
        for (i, row) in rows.iter().enumerate() {
            let wire = bytes(row, "data_b64");
            let event = decoded(&mut codec, &wire, &DecodeOptions::default());
            let v = event.value();
            if index == 3 && (13..=16).contains(&(i + 1)) {
                assert_eq!(v["kind"], "decode");
                let f = &v["fields"];
                let expected = &oracles["decodes"][decodes];
                for key in ["text", "snr_db", "audio_offset_hz", "dt_seconds"] {
                    assert_eq!(f[key], expected[key], "record {}: {key}", i + 1);
                }
                assert_eq!(f["mode"], "~");
                assert_eq!(f["is_new"], true);
                assert_eq!(f["low_confidence"], false);
                assert_eq!(f["off_air"], false);
                assert_eq!(f["time_of_day"], "01:04:00.000");
                decodes += 1;
            }
        }
        assert_eq!(decodes, if index == 3 { 4 } else { 0 });
    }
}
