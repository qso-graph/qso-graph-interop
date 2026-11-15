// SPDX-License-Identifier: GPL-3.0-or-later
//! Arbitrary payloads, topics and capture wrappers; exact evidence verification.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_interop::{
    Codec, EncodeOptions, Profile, Verification, VerifyOptions,
    pskreporter_mqtt::{self, PskReporterMqtt},
};
fn check(data: &[u8], topic: &str) {
    let mut codec = PskReporterMqtt::default();
    if let Ok(out) = codec.decode(
        data,
        &pskreporter_mqtt::options(topic, Some(true), Some(0))
            .unwrap()
            .with_structured(true),
    ) {
        assert!(out.events.len() <= 1);
        for event in out.events {
            assert_eq!(
                codec.verify(&event, &VerifyOptions::default()).unwrap(),
                Verification::Verified
            );
            assert_eq!(
                codec
                    .reencode(&event, &EncodeOptions::default(), &VerifyOptions::default())
                    .unwrap(),
                data
            );
        }
    }
}
fuzz_target!(|data: &[u8]| {
    check(data, "pskr/rxinfo/Q1AAA");
    check(data, "pskr/filter/v2/20m/FT8/Q1AAA/Q2BBB/AA00/BB11/1/2");
    if let Some(split) = data.iter().position(|b| *b == b'\n')
        && let Ok(topic) = std::str::from_utf8(&data[..split])
    {
        check(&data[split + 1..], topic);
    }
    if let Ok(v) = qso_graph_interop::json::parse_value(data, 32768) {
        let _ = pskreporter_mqtt::decode_record(&v);
    }
});
