// SPDX-License-Identifier: GPL-3.0-or-later
//! N1MM arbitrary XML and field-built commands through the public API.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_interop::{
    Codec,
    codec::{DecodeOptions, EncodeOptions},
    model::MAX_RECORD,
    n1mm::{MAX_DATAGRAM, N1mm},
};
fn observation(bytes: &[u8]) {
    let mut codec = N1mm::default();
    let mut options = DecodeOptions::default();
    options = options.with_structured(true);
    if let Ok(out) = codec.decode(bytes, &options) {
        assert!(out.events.len() + out.skips.len() == 1);
        for event in out.events {
            assert_eq!(event.native_bytes().unwrap(), bytes);
            assert_eq!(
                codec
                    .reencode(
                        &event,
                        &EncodeOptions::default(),
                        &qso_graph_interop::VerifyOptions::default()
                    )
                    .unwrap(),
                bytes
            );
            let json = qso_graph_interop::json::write(
                &event,
                &qso_graph_interop::PublicationOptions::new(
                    qso_graph_interop::PublicationMode::Authored,
                )
                .with_profile(&codec),
            )
            .unwrap();
            let read = qso_graph_interop::json::parse(&json, MAX_RECORD).unwrap();
            assert!(qso_graph_interop::json::semantic_eq(
                event.value(),
                read.value()
            ));
        }
    }
}
fuzz_target!(|data: &[u8]| {
    observation(data);
    if let Ok(command) = qso_graph_interop::json::parse(data, MAX_RECORD)
        && let Ok(bytes) = N1mm::default().encode_command(&command, &EncodeOptions::default())
    {
        assert!(bytes.len() <= MAX_DATAGRAM);
        let observed = N1mm::default()
            .decode(&bytes, &DecodeOptions::default())
            .unwrap();
        assert_eq!(observed.events.len(), 1);
        assert!(observed.skips.is_empty());
        let mut expected = command.value()["fields"].clone();
        expected.as_object_mut().unwrap().remove("spectrum_data");
        assert!(qso_graph_interop::json::semantic_eq(
            &expected,
            &observed.events[0].value()["fields"]
        ));
        observation(&bytes);
    }
});
