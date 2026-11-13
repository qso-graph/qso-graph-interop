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
    let mut codec = if let Some(request) = options.native.get("request") {
        Fldigi::with_request(request.clone())
    } else {
        Fldigi::default()
    };
    if let Ok(out) = codec.decode(bytes, options) {
        assert_eq!(out.events.len(), 1);
        assert!(out.skips.is_empty());
        let event = &out.events[0];
        assert_eq!(event.native_bytes().unwrap(), bytes);
        let encoded = codec
            .reencode_with(event, &EncodeOptions::default(), options)
            .unwrap();
        assert_eq!(encoded, bytes);
        let again = codec.decode(&encoded, options).unwrap();
        assert_eq!(event, &again.events[0]);
        let json = qso_graph_interop::json::write(event, false, None).unwrap();
        let read = qso_graph_interop::json::parse(&json, MAX_RECORD).unwrap();
        assert!(qso_graph_interop::json::semantic_eq(
            event.value(),
            read.value()
        ));
    }
}
fuzz_target!(|data: &[u8]| {
    let mut options = DecodeOptions::default();
    options.structured = true;
    observation(data, &options);
    if let Ok(input) = serde_json::from_slice::<serde_json::Value>(data) {
        for key in ["request", "via"] {
            if let Some(value) = input.get(key) {
                options.native.insert(key.into(), value.clone());
            }
        }
        if let Some(context) = input.get("context") {
            options.context = context.clone();
        }
        if let Some(bytes) = input["data"]
            .as_str()
            .and_then(|s| qso_graph_interop::base64::decode(s).ok())
        {
            observation(&bytes, &options);
        }
    }
    if let Ok(command) = qso_graph_interop::json::parse(data, MAX_RECORD)
        && let Ok(bodies) = Fldigi::default().encode_commands(&command, &EncodeOptions::default())
    {
        assert!((1..=2).contains(&bodies.len()));
        for bytes in bodies {
            assert!(bytes.len() <= MAX_BODY);
            observation(&bytes, &DecodeOptions::default());
        }
    }
});
