// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent synthetic evidence, association isolation and planted API failures.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Codec, Event,
    codec::{DecodeOptions, EncodeOptions, Profile},
    fldigi::Fldigi,
    model::Verification,
};
use serde_json::json;
fn command(fields: serde_json::Value) -> Event {
    Event::from_value(json!({"spec":"0.1.0","family":"fldigi","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":fields})).unwrap()
}
#[test]
fn independent_old_frequency_evidence_and_planted_changes() {
    let bytes = include_bytes!("fixtures/fldigi-frequency-response.xml");
    let mut codec = Fldigi::default();
    let mut options = DecodeOptions::default();
    options = options.with_structured(true);
    options = DecodeOptions::new(
        qso_graph_interop::Context::from_value(
            json!({"direction":"unknown","exchange":"unknown","correlation_id":"synthetic-Q1AAA"}),
        )
        .unwrap(),
    )
    .with_structured(options.structured())
    .with_metadata(options.metadata());
    options = options.with_metadata(
        qso_graph_interop::fldigi::RequestAssociation::for_method("rig.set_frequency")
            .unwrap()
            .into(),
    );
    let event = codec.decode(bytes, &options).unwrap().events.remove(0);
    assert_eq!(
        event.value()["fields"],
        json!({"status":"ok","operation":"set_frequency","frequency_hz":137500})
    );
    assert_eq!(
        event.value()["context"],
        options.context().as_value().clone()
    );
    assert_eq!(
        codec
            .reencode(&event, &EncodeOptions::default(), &options.verification())
            .unwrap(),
        bytes
    );
    assert_eq!(
        codec.verify(&event, &options.verification()).unwrap(),
        Verification::Verified
    );
    assert_eq!(
        Fldigi::default()
            .verify(&event, &qso_graph_interop::VerifyOptions::default())
            .unwrap(),
        Verification::Unavailable
    );
    // The structured method is evidence, never an association source.
    assert_eq!(
        Fldigi::default()
            .decode(bytes, &options.clone().with_metadata(Default::default()))
            .unwrap()
            .events[0]
            .value()["kind"],
        "native"
    );
    for (key, value) in [
        ("frequency_hz", json!(137501)),
        ("operation", json!("fldigi.rig_get_frequency")),
    ] {
        let mut changed = event.value().clone();
        changed["fields"][key] = value;
        assert_eq!(
            codec
                .reencode(
                    &Event::from_value(changed).unwrap(),
                    &EncodeOptions::default(),
                    &options.verification()
                )
                .unwrap_err()
                .category(),
            Category::InvalidValue
        );
    }
    for key in ["root", "xml", "method"] {
        let mut changed = event.value().clone();
        changed["native"]["message"][key] = "changed".into();
        assert!(
            codec
                .verify(
                    &Event::from_value(changed).unwrap(),
                    &options.verification()
                )
                .is_err()
        );
    }
    let mut bad = options.clone();
    bad = bad.with_metadata(
        qso_graph_interop::codec::DecodeMetadata::extension_metadata(
            json!({"request":null}).as_object().unwrap().clone(),
        ),
    );
    assert_eq!(
        codec.decode(bytes, &bad).unwrap_err().category(),
        Category::InvalidValue
    );
}
#[test]
fn sequence_api_and_single_body_guard() {
    let codec = Fldigi::default();
    let event = command(json!({"operation":"send_text","text":"CQ Q1AAA","transmit":true}));
    let requests = codec
        .encode_commands(&event, &EncodeOptions::default())
        .unwrap();
    assert_eq!(requests.requests().len(), 2);
    assert!(
        String::from_utf8_lossy(requests.requests()[0].bytes())
            .contains("<methodName>text.add_tx</methodName>")
    );
    assert!(
        String::from_utf8_lossy(requests.requests()[1].bytes())
            .contains("<methodName>main.tx</methodName>")
    );
    assert!(
        codec
            .encode_command(&event, &EncodeOptions::default())
            .is_err()
    );
    let frequency =
        command(json!({"operation":"set_frequency","frequency_hz":14071000,"rx_offset_hz":850}));
    assert_eq!(
        codec
            .encode_commands(&frequency, &EncodeOptions::default())
            .unwrap()
            .requests()
            .len(),
        2
    );
    let mut wrong = frequency.into_value();
    wrong["fields"]["radio"] = json!({"number":1});
    assert!(
        codec
            .encode_commands(
                &Event::from_value(wrong).unwrap(),
                &EncodeOptions::default()
            )
            .is_err()
    );
}
#[test]
fn xmlrpc_stages_and_opaque_fallback() {
    let mut codec = Fldigi::default();
    let mut options = DecodeOptions::default();
    options = options.with_structured(true);
    let opaque=b"<methodResponse><params><param><value><future><int>invalid opaque</int></future></value></param></params></methodResponse>";
    let event = codec.decode(opaque, &options).unwrap().events.remove(0);
    assert_eq!(
        codec
            .verify(&event, &qso_graph_interop::VerifyOptions::default())
            .unwrap(),
        Verification::Unavailable
    );
    assert_eq!(
        codec
            .reencode(
                &event,
                &EncodeOptions::default(),
                &qso_graph_interop::VerifyOptions::default()
            )
            .unwrap(),
        opaque
    );
    for key in ["root", "xml"] {
        let mut wrong = event.value().clone();
        wrong["native"]["message"][key] = "changed".into();
        assert!(
            codec
                .reencode(
                    &Event::from_value(wrong).unwrap(),
                    &EncodeOptions::default(),
                    &qso_graph_interop::VerifyOptions::default()
                )
                .is_err()
        );
    }
    let mut wrong = event.value().clone();
    wrong["native"]["format"] = "wrong".into();
    assert!(
        codec
            .reencode(
                &Event::from_value(wrong).unwrap(),
                &EncodeOptions::default(),
                &qso_graph_interop::VerifyOptions::default()
            )
            .is_err()
    );
    assert_eq!(codec.decode(b"<methodResponse><params><param><value><s:i4 xmlns:s=\"urn:synthetic\">1</s:i4></value></param></params></methodResponse>",&options).unwrap_err().category(),Category::Malformed);
    for bad in [
        "<methodCall><methodName>future.method</methodName><params><param><value><int>NaN</int></value></param></params></methodCall>",
        "<methodResponse><params><param><value><dateTime.iso8601>20261112Té:00:00</dateTime.iso8601></value></param></params></methodResponse>",
        "<methodResponse><params><param><value><double>1e999</double></value></param></params></methodResponse>",
    ] {
        assert_eq!(
            codec
                .decode(bad.as_bytes(), &options)
                .unwrap_err()
                .category(),
            Category::InvalidValue
        );
    }
}

#[test]
fn per_call_metadata_reaches_reading_and_publication() {
    use std::error::Error as _;
    let body=b"<methodResponse><fault><value><struct><member><name>faultCode</name><value><int>-003</int></value></member><member><name>faultString</name><value>synthetic failure</value></member></struct></value></fault></methodResponse>";
    let mut options = DecodeOptions::default();
    options = options.with_structured(true);
    options = options.with_metadata(
        qso_graph_interop::fldigi::RequestAssociation::from_value(json!({"method":"text.add_tx"}))
            .unwrap()
            .into(),
    );
    let mut codec = Fldigi::default();
    let event = codec.decode(body, &options).unwrap().events.remove(0);
    assert_eq!(event.value()["fields"]["operation"], "send_text");
    assert_eq!(
        event
            .recognized_with(Some(&codec), &options.verification())
            .unwrap()
            .verification(),
        Verification::Verified
    );
    assert_eq!(
        event.recognized(Some(&codec)).unwrap().verification(),
        Verification::Unavailable
    );
    qso_graph_interop::json::write(
        &event,
        &qso_graph_interop::PublicationOptions::new(qso_graph_interop::PublicationMode::Authored)
            .with_profile(&codec)
            .with_verification(options.verification()),
    )
    .unwrap();
    let mut wrong = event.into_value();
    wrong["fields"]["native_code"] = "-004".into();
    let wrong = Event::from_value(wrong).unwrap();
    let error = qso_graph_interop::json::write(
        &wrong,
        &qso_graph_interop::PublicationOptions::new(qso_graph_interop::PublicationMode::Authored)
            .with_profile(&codec)
            .with_verification(options.verification()),
    )
    .unwrap_err();
    assert_eq!(error.category(), Category::InvalidValue);
    assert!(
        error
            .source()
            .unwrap()
            .is::<qso_graph_interop::fldigi::FldigiError>()
    );
    assert!(
        codec
            .verify(&wrong, &qso_graph_interop::VerifyOptions::default())
            .is_err()
    );
}

#[test]
fn future_observer_enums_preserve_reader_context_without_claiming_producer_validity() {
    let input: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/fldigi-unknown-context.json")).unwrap();
    let bytes = evidence_bytes(input["data"].as_str().unwrap()).unwrap();
    let mut options = DecodeOptions::default();
    options = options.with_structured(true);
    options = DecodeOptions::new(
        qso_graph_interop::Context::from_value(input["context"].clone()).unwrap(),
    )
    .with_structured(options.structured())
    .with_metadata(options.metadata());
    options = options.with_metadata(
        qso_graph_interop::fldigi::RequestAssociation::from_value(input["request"].clone())
            .unwrap()
            .into(),
    );
    let mut codec = Fldigi::default();
    let event = codec.decode(&bytes, &options).unwrap().events.remove(0);
    assert_eq!(event.value()["context"], input["context"]);
    assert_eq!(
        event
            .recognized_with(Some(&codec), &options.verification())
            .unwrap()
            .context()
            .as_value()["direction"],
        "unknown"
    );
    let output = qso_graph_interop::json::write(
        &event,
        &qso_graph_interop::PublicationOptions::new(qso_graph_interop::PublicationMode::Preserved),
    )
    .unwrap();
    assert_eq!(
        qso_graph_interop::json::parse(&output, qso_graph_interop::model::MAX_RECORD).unwrap(),
        event
    );
    assert_eq!(
        codec
            .reencode(&event, &EncodeOptions::default(), &options.verification())
            .unwrap(),
        bytes
    );
    assert!(
        qso_graph_interop::json::write(
            &event,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Authored
            )
            .with_profile(&codec)
            .with_verification(options.verification())
        )
        .is_err()
    );
}

fn evidence_bytes(data: &str) -> Result<Vec<u8>, qso_graph_interop::Error> {
    qso_graph_interop::model::NativeEvidence::from_value(
        serde_json::json!({"format":"test","data":data}),
    )?
    .data()
}

#[test]
fn request_body_association_preserves_rx_integer_type_and_ordered_request_metadata() {
    use qso_graph_interop::fldigi::RequestAssociation;
    let body = b"<methodResponse><params><param><value><base64>Q1EgUTFBQUE=</base64></value></param></params></methodResponse>";
    for (typ, kind) in [("int", "decode"), ("double", "native")] {
        let request = format!(
            "<methodCall><methodName>text.get_rx</methodName><params><param><value><{typ}>0</{typ}></value></param><param><value><int>64</int></value></param></params></methodCall>"
        );
        let options = DecodeOptions::default().with_metadata(
            RequestAssociation::from_request_body(request.as_bytes())
                .unwrap()
                .into(),
        );
        let event = Fldigi::default()
            .decode(body, &options)
            .unwrap()
            .events
            .remove(0);
        assert_eq!(event.kind_token(), kind);
    }
    let plan = Fldigi::default()
        .encode_commands(
            &command(json!({"operation":"send_text","text":"CQ Q1AAA","transmit":true})),
            &EncodeOptions::default(),
        )
        .unwrap();
    for (request, method) in plan.requests().iter().zip(["text.add_tx", "main.tx"]) {
        assert_eq!(
            request.response_metadata().unwrap().extensions()["request"]["method"],
            method
        );
        let association = RequestAssociation::from_request_body(request.bytes()).unwrap();
        assert_eq!(association.as_value()["method"], method);
    }
}
