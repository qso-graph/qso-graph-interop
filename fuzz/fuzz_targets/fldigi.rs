// SPDX-License-Identifier: GPL-3.0-or-later
//! fldigi bodies, caller association, field commands and exact round trips.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_interop::{
    Codec,
    codec::{DecodeOptions, EncodeOptions},
    fldigi::{Fldigi, MAX_BODY},
    model::MAX_RECORD,
};
fn observation(bytes: &[u8], options: &DecodeOptions<'_>) {
    let mut codec = Fldigi::default();
    if let Ok(out) = codec.decode(bytes, options) {
        assert_eq!(out.events.len(), 1);
        assert!(out.skips.is_empty());
        let event = &out.events[0];
        assert_eq!(event.native_bytes().unwrap(), bytes);
        let encoded = codec
            .reencode(event, &EncodeOptions::default(), &options.verification())
            .unwrap();
        assert_eq!(encoded, bytes);
        let again = codec.decode(&encoded, options).unwrap();
        assert_eq!(event, &again.events[0]);
        let json = qso_graph_interop::json::write(
            event,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Preserved,
            ),
        )
        .unwrap();
        let read = qso_graph_interop::json::parse(&json, MAX_RECORD).unwrap();
        assert!(qso_graph_interop::json::semantic_eq(
            event.value(),
            read.value()
        ));
    }
}
fuzz_target!(|data: &[u8]| {
    let mut options = DecodeOptions::default();
    options = options.with_structured(true);
    observation(data, &options);
    if let Ok(input) = serde_json::from_slice::<serde_json::Value>(data) {
        let mut native = serde_json::Map::new();
        for key in ["request", "via"] {
            if let Some(value) = input.get(key) {
                native.insert(key.into(), value.clone());
            }
        }
        if let Ok(boundary) =
            DecodeOptions::from_value(&serde_json::json!({"native":native,"structured":true}))
        {
            options = boundary;
        } else {
            return;
        }
        if let Some(context) = input.get("context") {
            if let Ok(context) = qso_graph_interop::Context::from_value(context.clone()) {
                options = options.with_context(context);
            } else {
                return;
            }
        }
        if let Some(bytes) = input["data"].as_str().and_then(|s| evidence_bytes(s).ok()) {
            observation(&bytes, &options);
        }
    }
    if let Ok(command) = qso_graph_interop::json::parse(data, MAX_RECORD)
        && let Ok(bodies) = Fldigi::default().encode_commands(&command, &EncodeOptions::default())
    {
        assert!((1..=2).contains(&bodies.requests().len()));
        for request in bodies.requests() {
            let bytes = request.bytes();
            assert!(bytes.len() <= MAX_BODY);
            observation(bytes, &DecodeOptions::default());
        }
    }
});

fn evidence_bytes(data: &str) -> Result<Vec<u8>, qso_graph_interop::Error> {
    qso_graph_interop::model::NativeEvidence::from_value(
        serde_json::json!({"format":"test","data":data}),
    )?
    .data()
}
