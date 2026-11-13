// SPDX-License-Identifier: GPL-3.0-or-later
//! Arbitrary binary and event JSON input through the public sans-I/O API.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_interop::{
    Codec,
    codec::{DecodeOptions, EncodeOptions},
    model::MAX_RECORD,
    qgi::{Qgi, Transport},
    wsjtx::Wsjtx,
};
use qso_graph_interop_cli::alltxt;
fuzz_target!(|data: &[u8]| {
    if let Ok(log) = alltxt::parse(data, "") {
        for cycle in log.cycles {
            let _ = alltxt::period_ms(&cycle.mode);
            assert!(!cycle.stamp.is_empty());
            let _ = cycle.dial_hz;
        }
    }
    let options = DecodeOptions::default();
    let _ = qso_graph_interop::json::parse(data, MAX_RECORD);
    for transport in [Transport::Udp, Transport::Tcp] {
        let _ = Qgi::new(transport).decode(data, &options);
    }
    if let Ok(out) = Wsjtx::default().decode(data, &options) {
        for event in out.events {
            assert_eq!(event.native_bytes().unwrap(), data);
            let bytes = Wsjtx::default()
                .reencode(
                    &event,
                    &EncodeOptions::default(),
                    &qso_graph_interop::VerifyOptions::default(),
                )
                .unwrap();
            assert!(bytes.len() <= 65_507);
            let again = Wsjtx::default()
                .decode(&bytes, &options)
                .unwrap()
                .events
                .remove(0);
            assert_eq!(
                Wsjtx::default()
                    .reencode(
                        &again,
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
                .with_profile(&Wsjtx::default()),
            )
            .unwrap();
            let read = qso_graph_interop::json::parse(&json, MAX_RECORD).unwrap();
            assert!(qso_graph_interop::json::semantic_eq(
                event.value(),
                read.value()
            ));
        }
    }
});
