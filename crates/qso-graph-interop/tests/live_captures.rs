// SPDX-License-Identifier: GPL-3.0-or-later
//! Byte-exact lab evidence; record numbers are one-based recorder NDJSON lines.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Codec, DecodeOptions, EncodeOptions, Event, Profile, Verification, VerifyOptions,
    conformance::check_hash,
    fldigi::{Fldigi, RequestAssociation},
    n1mm::N1mm,
    pskreporter_ipfix::{
        PskReporterIpfix, SubmissionOptions, SubmissionProfile, encode_submission,
    },
    wsjtx::Wsjtx,
};
use serde_json::{Value, json};

const TRANSMITTER_LOG: (&str, &str) = (
    "fldigi-4.2.13-ipfix-tx.txt",
    include_str!("fixtures/live-captures/fldigi-4.2.13-ipfix-tx.txt"),
);
const CAPTURES: [(&str, &str); 8] = [
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
    (
        "wsjtx-3.0.2-ft8-ipfix-udp.ndjson",
        include_str!("fixtures/live-captures/wsjtx-3.0.2-ft8-ipfix-udp.ndjson"),
    ),
    (
        "wsjtx-3.3.0-beta6-ft8-ipfix-udp.ndjson",
        include_str!("fixtures/live-captures/wsjtx-3.3.0-beta6-ft8-ipfix-udp.ndjson"),
    ),
    (
        "fldigi-4.2.13-ipfix.ndjson",
        include_str!("fixtures/live-captures/fldigi-4.2.13-ipfix.ndjson"),
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
    assert_eq!(sums.lines().count(), CAPTURES.len() + 1);
    for (name, text) in CAPTURES.into_iter().chain([TRANSMITTER_LOG]) {
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

fn recorder_index(value: &Value) -> usize {
    usize::try_from(value.as_u64().unwrap()).unwrap() - 1
}

fn ipfix_sets(wire: &[u8]) -> Vec<&[u8]> {
    let mut sets = Vec::new();
    let mut offset = 16;
    while offset < wire.len() {
        let length = usize::from(u16::from_be_bytes([wire[offset + 2], wire[offset + 3]]));
        assert!(length >= 4 && offset + length <= wire.len());
        sets.push(&wire[offset..offset + length]);
        offset += length;
    }
    assert_eq!(offset, wire.len());
    sets
}

#[test]
fn fldigi_live_ipfix_matches_transmitter_log_and_exact_submission() {
    let oracle: Value = serde_json::from_str(include_str!(
        "fixtures/live-captures/fldigi-ipfix-oracle.json"
    ))
    .unwrap();
    let rows = records(7);
    assert_eq!(CAPTURES[7].0, oracle["file"]);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["received_at"], oracle["received_at"]);
    assert_eq!(row["transport"], "udp");
    assert_eq!(row["local"], "127.0.0.1:4739");
    assert_eq!(row["peer"], oracle["peer"]);
    let wire = bytes(row, "data_b64");
    assert_eq!(wire.len(), 256);
    let sets = ipfix_sets(&wire);
    assert_eq!(
        json!(
            sets.iter()
                .map(|s| u16::from_be_bytes([s[0], s[1]]))
                .collect::<Vec<_>>()
        ),
        oracle["set_ids"]
    );
    assert_eq!(
        json!(sets.iter().map(|s| s.len()).collect::<Vec<_>>()),
        oracle["set_lengths"]
    );
    let options = DecodeOptions::default()
        .with_session(row["peer"].as_str().unwrap().as_bytes())
        .with_structured(true);
    let mut codec = PskReporterIpfix::default();
    let outcome = codec.decode(&wire, &options).unwrap();
    assert!(outcome.skips.is_empty());
    assert_eq!(outcome.events.len(), 4);
    let receiver = &outcome.events[0];
    assert_eq!(receiver.kind_token(), "receiver_information");
    assert_eq!(receiver.value()["fields"], oracle["receiver"]);
    assert_eq!(receiver.value()["native"]["set_id"], 0x0219);
    let log = TRANSMITTER_LOG.1;
    assert!(!log.contains('\r'));
    assert_eq!(
        log.lines().filter(|line| line.contains("Z sent '")).count(),
        6
    );
    assert!(log.contains("rx spot auto: 1\n"));
    assert!(log.contains("modem BPSK31 carrier 1500 dial 14070000.0\n"));
    let decoded_text = log.split_once("rx decoded (166 chars): b'").unwrap().1;
    for (event, expected) in outcome.events[1..]
        .iter()
        .zip(oracle["spots"].as_array().unwrap())
    {
        assert_eq!(event.kind_token(), "spot");
        let mut fields = expected["fields"].clone();
        fields
            .as_object_mut()
            .unwrap()
            .extend(oracle["receiver"].as_object().unwrap().clone());
        assert_eq!(event.value()["fields"], fields);
        assert_eq!(event.value()["native"]["set_id"], 0x022c);
        let elements = &event.value()["native"]["message"]["elements"];
        let seconds = u32::from_be_bytes(bytes(&elements[1], "data").try_into().unwrap());
        assert_eq!(u64::from(seconds), expected["flow_start_seconds"]);
        assert!((1_794_711_039..=1_794_711_339).contains(&seconds));
        assert!(seconds < oracle["header"]["export_time"].as_u64().unwrap() as u32);
        // The transmitted frequency is zero; station settings do not override evidence.
        assert_eq!(bytes(&elements[2], "data"), [0; 4]);
        let call = expected["fields"]["sender_call"].as_str().unwrap();
        let cq = format!("CQ CQ DE {call} {call} PSE K\\n");
        assert!(log.contains(&format!(
            "{}Z sent '{cq}'\n",
            expected["sent_utc"].as_str().unwrap()
        )));
        assert_eq!(decoded_text.matches(&cq).count(), 2);
        for key in ["frequency_hz", "mode", "time", "receiver_locator"] {
            let mut planted = event.value().clone();
            planted["fields"][key] = match key {
                "frequency_hz" => json!(14_071_500),
                "mode" => json!("BPSK31"),
                "time" => json!("2026-11-15T02:55:15Z"),
                _ => json!("DN13"),
            };
            assert!(
                codec
                    .verify(
                        &Event::from_value(planted).unwrap(),
                        &VerifyOptions::default()
                    )
                    .is_err()
            );
        }
    }
    for event in &outcome.events {
        let native = &event.value()["native"];
        assert_eq!(event.value()["family"], "pskreporter-ipfix");
        assert_eq!(native["header"], oracle["header"]);
        assert_eq!(bytes(native, "message_data"), wire);
        let template =
            &oracle["templates"][usize::from(event.kind_token() == "receiver_information")];
        let template_bytes = bytes(native, "template");
        let hex = template_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(hex, template["hex"]);
        assert_eq!(native["set_id"], template["id"]);
        let message = &native["message"];
        let receiver_record = event.kind_token() == "receiver_information";
        assert_eq!(message["options"], receiver_record);
        assert_eq!(message["scope_fields"], 0);
        let elements = message["elements"].as_array().unwrap();
        let expected_elements = template["elements"].as_array().unwrap();
        assert_eq!(elements.len(), expected_elements.len());
        for (element, expected) in elements.iter().zip(expected_elements) {
            assert_eq!(
                json!([element["enterprise"], element["id"], element["length"]]),
                *expected
            );
        }
        assert_eq!(
            codec.verify(event, &VerifyOptions::default()).unwrap(),
            Verification::Verified
        );
        assert_eq!(
            codec
                .reencode(event, &EncodeOptions::default(), &VerifyOptions::default())
                .unwrap(),
            event.native_bytes().unwrap()
        );
    }
    let mut encode = SubmissionOptions::default();
    encode.profile = SubmissionProfile::Fldigi;
    encode.export_time = oracle["header"]["export_time"].as_u64().unwrap() as u32;
    encode.sequence_number = oracle["header"]["sequence_number"].as_u64().unwrap() as u32;
    encode.observation_domain_id =
        oracle["header"]["observation_domain_id"].as_u64().unwrap() as u32;
    assert_eq!(
        encode_submission(receiver, &outcome.events[1..], &encode).unwrap(),
        wire
    );
    assert!(
        PskReporterIpfix::default()
            .decode(&wire[..255], &options)
            .is_err()
    );
    let mut data_only = wire[..16].to_vec();
    for set in &sets[2..] {
        data_only.extend_from_slice(set);
    }
    let length = u16::try_from(data_only.len()).unwrap();
    data_only[2..4].copy_from_slice(&length.to_be_bytes());
    let missing = PskReporterIpfix::default()
        .decode(&data_only, &options)
        .unwrap();
    assert!(missing.events.is_empty());
    assert_eq!(missing.skips.len(), 2);
}

#[test]
fn wsjtx_live_ipfix_matches_audio_decodes_and_refreshes_templates() {
    let oracles: Value =
        serde_json::from_str(include_str!("fixtures/live-captures/ipfix-oracles.json")).unwrap();
    for (capture, oracle) in oracles["captures"].as_array().unwrap().iter().enumerate() {
        let rows = records(capture + 5);
        assert_eq!(CAPTURES[capture + 5].0, oracle["file"]);
        assert_eq!(rows.len() as u64, oracle["records"]);
        let mut wsjtx = Wsjtx::default();
        let mut udp_events = std::collections::BTreeMap::new();
        let mut counts = std::collections::BTreeMap::new();
        let mut submission_rows = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row["transport"], "udp");
            let wire = bytes(row, "data_b64");
            match row["local"].as_str().unwrap() {
                "127.0.0.1:4739" => submission_rows.push(index),
                "127.0.0.1:2238" => {
                    let event = decoded(&mut wsjtx, &wire, &DecodeOptions::default());
                    assert_eq!(event.value()["family"], "wsjtx");
                    *counts
                        .entry(u32::from_be_bytes(wire[8..12].try_into().unwrap()))
                        .or_insert(0) += 1;
                    udp_events.insert(index, event);
                }
                address => panic!("unexpected recorder destination {address}"),
            }
        }
        assert_eq!(counts.get(&2), Some(&40));
        assert_eq!(
            counts.get(&1),
            Some(&(if capture == 0 { 142 } else { 149 }))
        );
        assert_eq!(counts.get(&0), Some(&24));
        assert_eq!(
            counts.get(&6).copied().unwrap_or(0),
            usize::from(capture == 0)
        );
        assert_eq!(counts.len(), if capture == 0 { 4 } else { 3 });
        // Record the observed ending, without inferring a shutdown outside the capture.
        assert_eq!(
            udp_events[&(rows.len() - 1)].kind_token(),
            if capture == 0 {
                "ambiguous"
            } else {
                "radio_state"
            }
        );
        assert_eq!(
            rows.last().unwrap()["received_at"],
            if capture == 0 {
                "2026-11-15T02:00:46.697Z"
            } else {
                "2026-11-15T02:06:59.391Z"
            }
        );
        for expected in oracle["decodes"].as_array().unwrap() {
            let event = &udp_events[&recorder_index(&expected["record"])];
            assert_eq!(event.value()["fields"], expected["fields"]);
        }
        let submissions = oracle["submissions"].as_array().unwrap();
        assert_eq!(
            submission_rows,
            submissions
                .iter()
                .map(|s| recorder_index(&s["record"]))
                .collect::<Vec<_>>()
        );
        assert_eq!(submissions.len(), 2);
        assert_eq!(
            submissions[1]["header"]["export_time"].as_u64().unwrap()
                - submissions[0]["header"]["export_time"].as_u64().unwrap(),
            121
        );
        let mut codec = PskReporterIpfix::default();
        let first_wire = bytes(&rows[submission_rows[0]], "data_b64");
        let first_sets = ipfix_sets(&first_wire);
        for (number, expected) in submissions.iter().enumerate() {
            let row = &rows[submission_rows[number]];
            assert_eq!(row["received_at"], expected["received_at"]);
            assert_eq!(row["peer"], expected["peer"]);
            assert_eq!(row["peer"], rows[submission_rows[0]]["peer"]);
            let session = row["peer"].as_str().unwrap().as_bytes();
            let options = DecodeOptions::default()
                .with_session(session)
                .with_structured(true);
            let wire = bytes(row, "data_b64");
            let sets = ipfix_sets(&wire);
            assert_eq!(
                json!(
                    sets.iter()
                        .map(|s| u16::from_be_bytes([s[0], s[1]]))
                        .collect::<Vec<_>>()
                ),
                expected["set_ids"]
            );
            assert_eq!(
                json!(sets.iter().map(|s| s.len()).collect::<Vec<_>>()),
                expected["set_lengths"]
            );
            // The second submission refreshes both definitions and repeats receiver data exactly.
            assert_eq!(sets[..3], first_sets[..3]);
            let outcome = codec.decode(&wire, &options).unwrap();
            assert!(outcome.skips.is_empty());
            let spots = expected["spots"].as_array().unwrap();
            assert_eq!(outcome.events.len(), 1 + spots.len());
            // A fresh collector also decodes submission two: it does not need cached templates.
            let fresh = PskReporterIpfix::default().decode(&wire, &options).unwrap();
            assert!(fresh.skips.is_empty());
            assert_eq!(fresh.events, outcome.events);
            let receiver = &outcome.events[0];
            assert_eq!(receiver.kind_token(), "receiver_information");
            assert_eq!(receiver.value()["fields"], expected["receiver"]);
            assert_eq!(receiver.value()["native"]["set_id"], 0x50e2);
            assert_eq!(receiver.value()["native"]["message"]["options"], true);
            assert_eq!(receiver.value()["native"]["message"]["scope_fields"], 1);
            let receiver_elements = &receiver.value()["native"]["message"]["elements"];
            assert_eq!(receiver_elements.as_array().unwrap().len(), 5);
            for (element, id) in receiver_elements
                .as_array()
                .unwrap()
                .iter()
                .zip([2, 4, 8, 9, 13])
            {
                assert_eq!(element["enterprise"], 30351);
                assert_eq!(element["id"], id);
                assert_eq!(element["length"], 65535);
            }
            for (event, expected_spot) in outcome.events[1..].iter().zip(spots) {
                assert_eq!(event.kind_token(), "spot");
                let value = event.value();
                let fields = &value["fields"];
                let mut want = expected_spot["fields"].clone();
                want.as_object_mut()
                    .unwrap()
                    .extend(expected["receiver"].as_object().unwrap().clone());
                assert_eq!(fields, &want);
                assert_eq!(value["native"]["set_id"], 0x50e3);
                let elements = value["native"]["message"]["elements"].as_array().unwrap();
                assert_eq!(elements.len(), 7);
                for (element, (pen, id, length)) in elements.iter().zip([
                    (30351, 1, 65535),
                    (30351, 5, 5),
                    (30351, 6, 1),
                    (30351, 10, 65535),
                    (30351, 3, 65535),
                    (30351, 11, 1),
                    (0, 150, 4),
                ]) {
                    assert_eq!(element["enterprise"], pen);
                    assert_eq!(element["id"], id);
                    assert_eq!(element["length"], length);
                }
                assert_eq!(value["native"]["message"]["options"], false);
                let seconds = bytes(&elements[6], "data");
                assert_eq!(
                    u32::from_be_bytes(seconds.try_into().unwrap()),
                    expected_spot["flow_start_seconds"].as_u64().unwrap() as u32
                );
                let decode_row = recorder_index(&expected_spot["decode_record"]);
                let decode = &udp_events[&decode_row].value()["fields"];
                let decode_oracle = oracle["decodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|d| d["record"] == expected_spot["decode_record"])
                    .unwrap();
                let status =
                    &udp_events[&recorder_index(&decode_oracle["status_record"])].value()["fields"];
                assert_eq!(
                    fields["frequency_hz"].as_u64().unwrap(),
                    status["dial_frequency_hz"].as_u64().unwrap()
                        + decode["audio_offset_hz"].as_u64().unwrap()
                );
                assert_eq!(fields["snr_db"], decode["snr_db"]);
                assert_eq!(fields["mode"], status["mode"]);
                assert_eq!(decode["mode"], "~"); // Native WSJT-X Decode marker for FT8.
                let words = decode["text"]
                    .as_str()
                    .unwrap()
                    .split_whitespace()
                    .collect::<Vec<_>>();
                let sender = if words[0] == "CQ" && words[1] == "DX" {
                    words[2]
                } else {
                    words[1]
                };
                assert_eq!(fields["sender_call"], sender);
                assert_eq!(fields["sender_locator"], *words.last().unwrap());
                assert_eq!(
                    fields["time"],
                    format!(
                        "{}T{}Z",
                        &rows[decode_row]["received_at"].as_str().unwrap()[..10],
                        &decode["time_of_day"].as_str().unwrap()[..8]
                    )
                );
                assert_ne!(fields["time"], row["received_at"]); // Reception time, not export/recorder time.
                assert_eq!(fields["receiver_call"], status["station_call"]);
                assert_eq!(fields["receiver_locator"], status["station_grid"]);
                // Planted common-field drift must fail evidence verification.
                let mut planted = value.clone();
                planted["fields"]["snr_db"] = 0.into();
                assert!(
                    codec
                        .verify(
                            &Event::from_value(planted).unwrap(),
                            &VerifyOptions::default()
                        )
                        .is_err()
                );
            }
            for event in &outcome.events {
                assert_eq!(event.value()["family"], "pskreporter-ipfix");
                assert_eq!(event.value()["native"]["header"], expected["header"]);
                assert_eq!(bytes(&event.value()["native"], "message_data"), wire);
                assert_eq!(
                    codec.verify(event, &VerifyOptions::default()).unwrap(),
                    Verification::Verified
                );
                assert_eq!(
                    codec
                        .reencode(event, &EncodeOptions::default(), &VerifyOptions::default())
                        .unwrap(),
                    event.native_bytes().unwrap()
                );
            }
            let mut encode = SubmissionOptions::default(); // WSJT-X profile, both templates included.
            encode.export_time = expected["header"]["export_time"].as_u64().unwrap() as u32;
            encode.sequence_number = expected["header"]["sequence_number"].as_u64().unwrap() as u32;
            encode.observation_domain_id = expected["header"]["observation_domain_id"]
                .as_u64()
                .unwrap() as u32;
            assert_eq!(
                encode_submission(receiver, &outcome.events[1..], &encode).unwrap(),
                wire
            );
            // Planted truncation and missing definitions cannot pass a fresh decoder.
            assert!(
                PskReporterIpfix::default()
                    .decode(&wire[..wire.len() - 1], &options)
                    .is_err()
            );
            let mut data_only = wire[..16].to_vec();
            for set in &sets[2..] {
                data_only.extend_from_slice(set);
            }
            let length = u16::try_from(data_only.len()).unwrap();
            data_only[2..4].copy_from_slice(&length.to_be_bytes());
            let missing = PskReporterIpfix::default()
                .decode(&data_only, &options)
                .unwrap();
            assert!(missing.events.is_empty());
            assert_eq!(missing.skips.len(), sets.len() - 2);
        }
    }
}
