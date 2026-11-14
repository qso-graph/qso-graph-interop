// SPDX-License-Identifier: GPL-3.0-or-later
//! Source-derived independent program bytes, cache transactions, and evidence integrity.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Codec, DecodeOptions, EncodeOptions, Event, Profile, VerifyOptions,
    pskreporter_ipfix::{
        PskReporterIpfix, SubmissionOptions, SubmissionProfile, encode_submission,
    },
};
use serde_json::{Value, json};
fn fixture(name: &str) -> (Vec<u8>, Vec<Event>) {
    let f: Value = serde_json::from_str(include_str!("fixtures/pskreporter-ipfix.json")).unwrap();
    let p = &f["profiles"][name];
    let s = p["wire_hex"].as_str().unwrap();
    let raw = s
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect();
    (
        raw,
        p["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| Event::from_value(e.clone()).unwrap())
            .collect(),
    )
}
fn sets(raw: &[u8]) -> Vec<Vec<u8>> {
    let mut p = 16;
    let mut result = vec![];
    while p < raw.len() {
        let len = usize::from(u16::from_be_bytes([raw[p + 2], raw[p + 3]]));
        result.push(raw[p..p + len].to_vec());
        p += len;
    }
    result
}
fn message(sets: &[Vec<u8>], domain: u32) -> Vec<u8> {
    let mut raw = vec![0, 10];
    let size = 16 + sets.iter().map(Vec::len).sum::<usize>();
    raw.extend((size as u16).to_be_bytes());
    raw.extend(1700000000_u32.to_be_bytes());
    raw.extend([0; 4]);
    raw.extend(domain.to_be_bytes());
    for s in sets {
        raw.extend(s);
    }
    raw
}
fn set(id: u16, body: &[u8]) -> Vec<u8> {
    let mut s = vec![];
    s.extend(id.to_be_bytes());
    s.extend(((body.len() + 4) as u16).to_be_bytes());
    s.extend(body);
    s
}
fn options() -> DecodeOptions<'static> {
    DecodeOptions::default().with_session(b"peer-1")
}
#[test]
fn program_profiles_exact_both_directions_and_evidence() {
    for name in ["wsjtx", "fldigi", "fldigi-short"] {
        let (raw, expected) = fixture(name);
        for structured in [false, true] {
            let mut codec = PskReporterIpfix::default();
            let actual = codec
                .decode(&raw, &options().with_structured(structured))
                .unwrap();
            assert_eq!(actual.events.len(), 2);
            for (e, want) in actual.events.iter().zip(&expected) {
                let mut v = e.value().clone();
                v["native"].as_object_mut().unwrap().remove("message");
                assert!(qso_graph_interop::semantic_eq(&v, want.value()), "{name}");
                assert_eq!(
                    codec.verify(e, &VerifyOptions::default()).unwrap(),
                    qso_graph_interop::Verification::Verified
                );
                assert_eq!(
                    codec
                        .reencode(e, &EncodeOptions::default(), &VerifyOptions::default())
                        .unwrap(),
                    e.native_bytes().unwrap()
                );
                let mut mutant = e.value().clone();
                mutant["fields"]["receiver_call"] = "Q9BAD".into();
                assert!(
                    codec
                        .verify(
                            &Event::from_value(mutant).unwrap(),
                            &VerifyOptions::default()
                        )
                        .is_err()
                );
            }
        }
        let mut encode = SubmissionOptions::default();
        encode.profile = if name == "wsjtx" {
            SubmissionProfile::Wsjtx
        } else {
            SubmissionProfile::Fldigi
        };
        encode.fldigi_short = name == "fldigi-short";
        encode.export_time = 1700000000;
        encode.observation_domain_id = 0x51474901;
        assert_eq!(
            encode_submission(&expected[0], &expected[1..], &encode).unwrap(),
            raw,
            "{name}"
        );
    }
}
#[test]
fn cache_is_scoped_transactional_and_resettable() {
    let (raw, _) = fixture("wsjtx");
    let all = sets(&raw);
    let mut codec = PskReporterIpfix::default();
    let data = message(&all[2..], 0x51474901);
    assert_eq!(codec.decode(&data, &options()).unwrap().skips.len(), 2);
    codec.decode(&raw, &options()).unwrap();
    assert_eq!(codec.decode(&data, &options()).unwrap().events.len(), 2);
    assert_eq!(
        codec
            .decode(&data, &options().with_session(b"peer-2"))
            .unwrap()
            .skips
            .len(),
        2
    );
    assert_eq!(
        codec
            .decode(&message(&all[2..], 7), &options())
            .unwrap()
            .skips
            .len(),
        2
    );
    let mut broken = all[..2].to_vec();
    broken.push(set(0x50e3, b"\xff\xff\xffabcdefghijk"));
    let mut clean = PskReporterIpfix::default();
    assert!(
        clean
            .decode(&message(&broken, 0x51474901), &options())
            .is_err()
    );
    assert_eq!(clean.decode(&data, &options()).unwrap().skips.len(), 2);
    // RFC 7011 §8.4: UDP withdrawals, individual and all-template, never remove definitions.
    for (kind, id) in [(2, 0x50e3), (2, 2), (3, 0x50e2), (3, 3)] {
        let mut withdrawal = vec![];
        withdrawal.extend((id as u16).to_be_bytes());
        withdrawal.extend([0, 0]);
        codec
            .decode(&message(&[set(kind, &withdrawal)], 0x51474901), &options())
            .unwrap();
    }
    assert_eq!(codec.decode(&data, &options()).unwrap().events.len(), 2);
    // An explicit export-time cutoff expires state; receipt order never supplies time.
    assert_eq!(
        codec
            .decode(&data, &options().with_expiry(Some(1700000001)))
            .unwrap()
            .skips
            .len(),
        2
    );
    codec.decode(&raw, &options()).unwrap();
    codec.reset(Some(b"peer-2"));
    assert_eq!(codec.decode(&data, &options()).unwrap().events.len(), 2);
    codec.reset(Some(b"peer-1"));
    assert_eq!(codec.decode(&data, &options()).unwrap().skips.len(), 2);
    codec.decode(&raw, &options()).unwrap();
    codec.reset(None);
    assert_eq!(codec.decode(&data, &options()).unwrap().skips.len(), 2);
}
#[test]
fn receiver_dependencies_are_message_local_and_unambiguous() {
    let (raw, _) = fixture("wsjtx");
    let s = sets(&raw);
    let mut codec = PskReporterIpfix::default();
    codec.decode(&raw, &options()).unwrap();
    let sender = codec
        .decode(&message(&s[3..], 0x51474901), &options())
        .unwrap()
        .events
        .remove(0);
    assert!(sender.value()["fields"].get("receiver_call").is_none());
    assert!(sender.value()["native"].get("dependencies").is_none());
    // A following receiver in the same message supplies fields without changing wire event order.
    let later = codec
        .decode(
            &message(&[s[3].clone(), s[2].clone()], 0x51474901),
            &options(),
        )
        .unwrap();
    assert_eq!(later.events[0].kind_token(), "spot");
    assert_eq!(later.events[0].value()["fields"]["receiver_call"], "Q2AAA");
    assert!(
        codec
            .verify(&later.events[0], &VerifyOptions::default())
            .is_ok()
    );
    let ambiguous = codec
        .decode(
            &message(&[s[2].clone(), s[2].clone(), s[3].clone()], 0x51474901),
            &options(),
        )
        .unwrap();
    assert!(
        ambiguous.events[2].value()["fields"]
            .get("receiver_call")
            .is_none()
    );
    let mut mutant = later.events[0].value().clone();
    mutant["native"]["dependencies"][0]["data"] = "".into();
    assert!(
        codec
            .verify(
                &Event::from_value(mutant).unwrap(),
                &VerifyOptions::default()
            )
            .is_err()
    );
}
#[test]
fn unknown_elements_variable_lengths_and_nonzero_padding_are_preserved() {
    // Arbitrary PEN/ID remains opaque; variable prefixes are RFC §7, including extended zero.
    for n in [0, 1, 254, 255, 256, 1024] {
        let template = set(2, &[1, 0, 0, 1, 0x80, 99, 0xff, 0xff, 0, 0, 0, 42]);
        let mut data = vec![];
        if n < 255 {
            data.push(n as u8);
        } else {
            data.push(255);
            data.extend((n as u16).to_be_bytes());
        }
        data.extend(vec![0xfd; n]);
        let raw = message(&[template, set(256, &data)], 1);
        let mut codec = PskReporterIpfix::default();
        let out = codec
            .decode(&raw, &options().with_structured(true))
            .unwrap();
        assert_eq!(out.events.len(), 1);
        assert_eq!(out.events[0].kind_token(), "native");
        assert_eq!(out.events[0].value()["fields"], json!({}));
        assert_eq!(out.events[0].native_bytes().unwrap(), data);
        assert!(
            codec
                .verify(&out.events[0], &VerifyOptions::default())
                .is_ok()
        );
    }
    let (raw, _) = fixture("wsjtx");
    let mut s = sets(&raw);
    let last = s[1].len() - 1;
    s[1][last] = 9;
    let out = PskReporterIpfix::default()
        .decode(&message(&s, 0x51474901), &options())
        .unwrap();
    assert_eq!(out.events.len(), 2);
}
#[test]
fn replacement_refresh_cap_and_session_overflow_never_evict() {
    let template = |id: u16| {
        let mut b = vec![];
        b.extend(id.to_be_bytes());
        b.extend([0, 1, 0, 99, 0, 1]);
        set(2, &b)
    };
    let mut codec = PskReporterIpfix::default();
    let ts: Vec<_> = (256..320).map(template).collect();
    codec.decode(&message(&ts, 1), &options()).unwrap();
    codec
        .decode(&message(&[template(256)], 1), &options())
        .unwrap();
    assert_eq!(
        codec
            .decode(&message(&[template(320)], 1), &options())
            .unwrap_err()
            .category(),
        Category::Oversized
    );
    assert_eq!(
        codec
            .decode(&message(&[set(256, &[42])], 1), &options())
            .unwrap()
            .events
            .len(),
        1
    );
    let mut changed = template(256);
    let last = changed.len() - 1;
    changed[last] = 2;
    codec.decode(&message(&[changed], 1), &options()).unwrap();
    let out = codec
        .decode(&message(&[set(256, &[42, 43])], 1), &options())
        .unwrap();
    assert_eq!(out.events[0].native_bytes().unwrap(), vec![42, 43]);
    let mut codec = PskReporterIpfix::default();
    for n in 0..16 {
        codec
            .decode(
                &message(&[template(256)], 1),
                &DecodeOptions::default().with_session(&[n]),
            )
            .unwrap();
    }
    assert_eq!(
        codec
            .decode(
                &message(&[template(256)], 1),
                &DecodeOptions::default().with_session(&[16])
            )
            .unwrap_err()
            .category(),
        Category::Oversized
    );
    assert_eq!(
        codec
            .decode(
                &message(&[set(256, &[42])], 1),
                &DecodeOptions::default().with_session(&[0])
            )
            .unwrap()
            .events
            .len(),
        1
    );
}
#[test]
fn largest_observation_fits_with_raw_text_structured_view_and_context() {
    let mut record = vec![255];
    record.extend(59961_u16.to_be_bytes());
    record.extend(vec![255; 59961]);
    let raw = message(
        &[
            set(2, &[1, 0, 0, 1, 0x80, 1, 0xff, 0xff, 0, 0, 0x76, 0x8f]),
            set(256, &record),
        ],
        1,
    );
    assert_eq!(raw.len(), 60000);
    let context = qso_graph_interop::Context::from_value(
        json!({"direction":"unknown","exchange":"unknown","source_id":"x".repeat(4000)}),
    )
    .unwrap();
    let codec = &mut PskReporterIpfix::default();
    let event = codec
        .decode(&raw, &options().with_context(context).with_structured(true))
        .unwrap()
        .events
        .remove(0);
    let output =
        qso_graph_interop::write(&event, &qso_graph_interop::PublicationOptions::default())
            .unwrap();
    assert!(output.len() < 600_000);
    assert!(codec.verify(&event, &VerifyOptions::default()).is_ok());
    assert_eq!(
        codec
            .decode(&vec![0; 60001], &options())
            .unwrap_err()
            .category(),
        Category::Oversized
    );
}
#[test]
fn encoder_rejects_unrepresentable_fields_without_truncation() {
    let (_, events) = fixture("wsjtx");
    let mut options = SubmissionOptions::default();
    options.export_time = 1700000000;
    for (name, value) in [
        ("snr_db", json!(128)),
        ("frequency_hz", json!(1099511627776_i64)),
        ("sender_call", json!("Q".repeat(33))),
        ("mode", json!(null)),
        ("information_source", json!(0)),
        ("time", json!("1969-12-31T23:59:59Z")),
        ("receiver_call", json!("Q9BAD")),
        ("extra", json!(1)),
    ] {
        let mut v = events[1].value().clone();
        v["fields"][name] = value;
        if let Ok(event) = Event::from_value(v) {
            assert!(
                encode_submission(&events[0], &[event], &options).is_err(),
                "{name}"
            );
        }
    }
    let mut source = events[1].value().clone();
    source["fields"]["information_source"] = 2.into();
    let source = Event::from_value(source).unwrap();
    let wire = encode_submission(&events[0], &[source], &options).unwrap();
    assert_eq!(
        PskReporterIpfix::default()
            .decode(&wire, &DecodeOptions::default())
            .unwrap()
            .events[1]
            .value()["fields"]["information_source"],
        2
    );
}
#[test]
fn cached_standard_options_template_is_not_mistaken_for_a_normal_template() {
    let mut codec = PskReporterIpfix::default();
    codec
        .decode(
            &message(&[set(3, &[1, 0, 0, 1, 0, 1, 0, 150, 0, 4])], 1),
            &options(),
        )
        .unwrap();
    let out = codec
        .decode(&message(&[set(256, &[0, 0, 0, 42])], 1), &options())
        .unwrap();
    assert_eq!(out.events.len(), 1);
    assert_eq!(out.events[0].kind_token(), "native");
    assert_eq!(
        codec
            .verify(&out.events[0], &VerifyOptions::default())
            .unwrap(),
        qso_graph_interop::Verification::Verified
    );
}
#[test]
fn verification_does_not_apply_later_templates_to_earlier_unknown_sets() {
    let (raw, _) = fixture("wsjtx");
    let s = sets(&raw);
    let bad = set(0x50e3, &[255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    for cached_receiver in [false, true] {
        let mut codec = PskReporterIpfix::default();
        let message_sets = if cached_receiver {
            codec
                .decode(&message(&[s[1].clone()], 0x51474901), &options())
                .unwrap();
            vec![bad.clone(), s[0].clone(), s[2].clone(), s[3].clone()]
        } else {
            vec![
                bad.clone(),
                s[0].clone(),
                s[1].clone(),
                s[2].clone(),
                s[3].clone(),
            ]
        };
        let out = codec
            .decode(&message(&message_sets, 0x51474901), &options())
            .unwrap();
        assert_eq!(out.skips.len(), 1);
        assert_eq!(out.events.len(), 2);
        for event in &out.events {
            assert_eq!(
                codec.verify(event, &VerifyOptions::default()).unwrap(),
                qso_graph_interop::Verification::Verified
            );
        }
    }
}
#[test]
fn submission_presence_is_explicit_and_receiver_minimum_is_not_invented() {
    let (_, events) = fixture("wsjtx");
    let options = SubmissionOptions::default();
    let mut receiver = events[0].value().clone();
    receiver["fields"]
        .as_object_mut()
        .unwrap()
        .remove("antenna_information");
    assert!(
        encode_submission(
            &Event::from_value(receiver).unwrap(),
            &events[1..],
            &options
        )
        .is_err()
    );
    let mut spot = events[1].value().clone();
    spot["fields"]
        .as_object_mut()
        .unwrap()
        .remove("sender_locator");
    assert!(encode_submission(&events[0], &[Event::from_value(spot).unwrap()], &options).is_err());
    let mut receiver = events[0].value().clone();
    for name in [
        "receiver_call",
        "receiver_locator",
        "reporting_software",
        "antenna_information",
        "rig_information",
    ] {
        receiver["fields"][name] = "".into();
    }
    let receiver = Event::from_value(receiver).unwrap();
    let raw = encode_submission(&receiver, &[], &options).unwrap();
    let observed = PskReporterIpfix::default()
        .decode(&raw, &DecodeOptions::default())
        .unwrap();
    assert_eq!(
        observed.events[0].value()["fields"],
        receiver.value()["fields"]
    );
    let (_, fldigi) = fixture("fldigi");
    let mut options = SubmissionOptions::default();
    options.profile = SubmissionProfile::Fldigi;
    assert!(encode_submission(&fldigi[0], &[], &options).is_err());
}
