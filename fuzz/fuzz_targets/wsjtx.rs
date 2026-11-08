// SPDX-License-Identifier: GPL-3.0-or-later
//! Both variants: bounded decoding, successful re-encoding and lossless model transport.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_udp::{
    model,
    wsjtx::{self, Outcome, Variant},
};

fuzz_target!(|data: &[u8]| {
    for variant in [Variant::Wsjtx, Variant::Jtdx] {
        if let Ok(Outcome::Message(packet)) = wsjtx::decode(data, variant) {
            let encoded = wsjtx::encode(&packet).expect("a decoded packet must encode");
            assert!(encoded.len() <= wsjtx::MAX_DATAGRAM);
            let Outcome::Message(again) =
                wsjtx::decode(&encoded, variant).expect("encoded packet must decode")
            else {
                panic!("known packet became unknown");
            };
            // Boolean encodings normalize to 0/1. All other raw values, including
            // NaN payload bits, must survive. Canonicalization must be idempotent.
            assert_eq!(wsjtx::encode(&again).unwrap(), encoded);
            let common = model::Message::from(packet);
            assert_eq!(wsjtx::encode(&common.into_family()).unwrap(), encoded);
        }
    }
});
