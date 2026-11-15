// SPDX-License-Identifier: GPL-3.0-or-later
//! Capture-only Annex G acceptance and planted evidence mismatches.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Codec, EncodeOptions, Event, Profile, Verification, VerifyOptions,
    pskreporter_mqtt::{self, PskReporterMqtt},
};
use serde_json::{Value, json};
#[test]
fn all_captured_messages_and_independent_field_oracles() {
    let f: Value = serde_json::from_str(include_str!("fixtures/pskreporter-mqtt.json")).unwrap();
    let mut spots = 0;
    let mut receivers = 0;
    let mut slash_spots = 0;
    let mut slash_receivers = 0;
    for (index, record) in f["records"].as_array().unwrap().iter().enumerate() {
        let out = pskreporter_mqtt::decode_record(record)
            .unwrap_or_else(|e| panic!("capture record {index}: {e}"));
        assert!(out.skips.is_empty());
        assert_eq!(out.events.len(), 1);
        let event = &out.events[0];
        assert_eq!(event.value()["native"]["data"], record["payload_b64"]);
        assert_eq!(event.value()["native"]["topic"], record["topic"]);
        let slash_call = ["sender_call", "receiver_call"].iter().any(|key| {
            event.value()["fields"][key]
                .as_str()
                .is_some_and(|call| call.contains('/'))
        });
        if event.value()["kind"] == "spot" {
            spots += 1;
            slash_spots += usize::from(slash_call);
        } else {
            receivers += 1;
            slash_receivers += usize::from(slash_call);
        }
        let codec = PskReporterMqtt::default();
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
        for oracle in f["oracles"].as_array().unwrap() {
            if oracle["record_index"] == index {
                assert_eq!(event.value()["fields"], oracle["fields"]);
            }
        }
    }
    assert_eq!((spots, receivers), (400, 60));
    assert_eq!((slash_spots, slash_receivers), (41, 1));
}
#[test]
fn tampered_evidence_cannot_be_verified_or_authored() {
    let mut codec = PskReporterMqtt::default();
    let event = codec
        .decode(
            br#"{"call":"Q1AAA","future":{"x":[null,""]}}"#,
            &pskreporter_mqtt::options("pskr/rxinfo/Q1AAA", None, None)
                .unwrap()
                .with_structured(true),
        )
        .unwrap()
        .events
        .remove(0);
    for (key, value) in [
        ("fields", json!({"receiver_call":"Q2BBB"})),
        ("kind", json!("spot")),
        ("native_version", json!("2")),
        ("wire_id", json!("invented")),
    ] {
        let mut v = event.value().clone();
        v[key] = value;
        let e = Event::from_value(v).unwrap();
        assert!(codec.verify(&e, &VerifyOptions::default()).is_err());
    }
    for (key, value) in [
        ("message", json!({})),
        ("format", json!("binary")),
        ("topic", json!("pskr/rxinfo/Q2BBB")),
        (
            "topic",
            json!("pskr/filter/v3/20m/FT8/Q1AAA/Q2BBB/AA00/BB11/1/2"),
        ),
    ] {
        let mut v = event.value().clone();
        v["native"][key] = value;
        if let Ok(e) = Event::from_value(v) {
            assert!(
                codec
                    .validate_authored(&e, &VerifyOptions::default())
                    .is_err()
            );
        }
    }
}
#[test]
fn maximal_payload_context_and_structured_unknown_data_fit_output() {
    let mut codec = PskReporterMqtt::default();
    let mut payload =
        json!({"a":"x".repeat(4096),"b":"x".repeat(4096),"c":"x".repeat(4096)}).to_string();
    payload.push_str(&" ".repeat(16384 - payload.len()));
    let context = qso_graph_interop::Context::from_value(
        json!({"direction":"unknown","exchange":"unknown","extra":"x".repeat(3900)}),
    )
    .unwrap();
    let opts = pskreporter_mqtt::options("pskr/rxinfo/Q1AAA", Some(false), Some(2))
        .unwrap()
        .with_structured(true)
        .with_context(context);
    let out = codec.decode(payload.as_bytes(), &opts).unwrap();
    assert!(
        qso_graph_interop::write(
            &out.events[0],
            &qso_graph_interop::PublicationOptions::default()
        )
        .unwrap()
        .len()
            < 262144
    );
    payload.push(' ');
    assert!(codec.decode(payload.as_bytes(), &opts).is_err());
}

#[test]
fn every_failure_carries_the_family_error_as_its_source() {
    use qso_graph_interop::{Category, DecodeOptions, Error};
    use std::error::Error as _;
    fn cause(error: &Error) -> Category {
        error
            .source()
            .and_then(|e| e.downcast_ref::<pskreporter_mqtt::MqttError>())
            .unwrap_or_else(|| panic!("no MqttError source: {error}"))
            .category()
    }
    let mut codec = PskReporterMqtt::default();
    let decoded = codec
        .decode(b"not mqtt", &DecodeOptions::default())
        .unwrap_err();
    assert_eq!(cause(&decoded), decoded.category());
    let record = pskreporter_mqtt::decode_record(&json!([])).unwrap_err();
    assert_eq!(cause(&record), Category::InvalidValue);
    let command = Event::from_value(json!({"spec":"0.2.0","family":"pskreporter-mqtt","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":{"operation":"x"}})).unwrap();
    let encoded = codec
        .encode_commands(&command, &EncodeOptions::default())
        .unwrap_err();
    assert_eq!(cause(&encoded), Category::InvalidValue);
    let reencoded = codec
        .reencode(
            &command,
            &EncodeOptions::default(),
            &VerifyOptions::default(),
        )
        .unwrap_err();
    assert_eq!(cause(&reencoded), Category::InvalidValue);
    let authored = codec
        .validate_authored(&command, &VerifyOptions::default())
        .unwrap_err();
    assert_eq!(cause(&authored), Category::InvalidValue);
}
