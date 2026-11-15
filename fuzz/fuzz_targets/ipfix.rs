// SPDX-License-Identifier: GPL-3.0-or-later
//! Arbitrary IPFIX messages, cached templates, sessions, and record evidence.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_interop::{
    Codec, DecodeOptions, EncodeOptions, Profile, VerifyOptions,
    pskreporter_ipfix::PskReporterIpfix,
};
fn check(codec: &mut PskReporterIpfix, data: &[u8], session: &[u8]) {
    if let Ok(out) = codec.decode(
        data,
        &DecodeOptions::default()
            .with_structured(true)
            .with_session(session),
    ) {
        assert!(out.events.len() <= 4096);
        for event in out.events.into_iter().take(2) {
            assert_eq!(
                codec.verify(&event, &VerifyOptions::default()).unwrap(),
                qso_graph_interop::Verification::Verified
            );
            assert_eq!(
                codec
                    .reencode(&event, &EncodeOptions::default(), &VerifyOptions::default())
                    .unwrap(),
                event.native_bytes().unwrap()
            );
            let json =
                qso_graph_interop::write(&event, &qso_graph_interop::PublicationOptions::default())
                    .unwrap();
            assert!(json.len() <= qso_graph_interop::model::MAX_RECORD + 1);
        }
    }
}
fuzz_target!(|data: &[u8]| {
    let mut codec = PskReporterIpfix::default();
    check(&mut codec, data, b"exporter-1");
    // A valid source-profile template seed exposes the record parser to arbitrary data Sets.
    let seed = &[
        0, 10, 0, 128, 101, 83, 241, 0, 0, 0, 0, 0, 81, 71, 73, 1, 0, 2, 0, 60, 80, 227, 0, 7, 128,
        1, 255, 255, 0, 0, 118, 143, 128, 5, 0, 5, 0, 0, 118, 143, 128, 6, 0, 1, 0, 0, 118, 143,
        128, 10, 255, 255, 0, 0, 118, 143, 128, 3, 255, 255, 0, 0, 118, 143, 128, 11, 0, 1, 0, 0,
        118, 143, 0, 150, 0, 4, 0, 3, 0, 52, 80, 226, 0, 5, 0, 1, 128, 2, 255, 255, 0, 0, 118, 143,
        128, 4, 255, 255, 0, 0, 118, 143, 128, 8, 255, 255, 0, 0, 118, 143, 128, 9, 255, 255, 0, 0,
        118, 143, 128, 13, 255, 255, 0, 0, 118, 143, 0, 0,
    ];
    check(&mut codec, seed, b"exporter-1");
    check(&mut codec, data, b"exporter-1");
    check(&mut codec, data, b"exporter-2");
    if data.len() > 2 {
        let split = usize::from(u16::from_be_bytes([data[0], data[1]])) % (data.len() - 1) + 1;
        check(&mut codec, &data[..split], b"exporter-1");
        check(&mut codec, &data[split..], b"exporter-1");
    }
    codec.reset(Some(b"exporter-1"));
    check(&mut codec, data, b"exporter-1");
});
