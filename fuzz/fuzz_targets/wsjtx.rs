// SPDX-License-Identifier: GPL-3.0-or-later
//! WSJT-X: bounded decoding, successful re-encoding and lossless model transport.
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
use qso_graph_interop::{
    model,
    wsjtx::{self, Outcome},
};

fuzz_target!(|data: &[u8]| {
    if let Ok(Outcome::Message(packet)) = wsjtx::decode(data) {
        let encoded = wsjtx::encode(&packet).expect("a decoded packet must encode");
        assert!(encoded.len() <= wsjtx::MAX_DATAGRAM);
        let Outcome::Message(again) = wsjtx::decode(&encoded).expect("encoded packet must decode")
        else {
            panic!("known packet became unknown");
        };
        // Boolean encodings normalize to 0/1. All other raw values, including
        // NaN payload bits, must survive. Canonicalization must be idempotent.
        assert_eq!(wsjtx::encode(&again).unwrap(), encoded);
        let json = serde_json::to_vec(&packet).unwrap();
        let from_json: wsjtx::Packet = serde_json::from_slice(&json).unwrap();
        assert_eq!(wsjtx::encode(&from_json).unwrap(), encoded);
        let common = model::Message::from(packet);
        let json = serde_json::to_vec(&common).unwrap();
        let from_json: model::Message = serde_json::from_slice(&json).unwrap();
        // Also check normalized fields, including float bits; family equality alone
        // would miss a changed normalized observation (NaN prevents PartialEq).
        assert_eq!(serde_json::to_vec(&from_json).unwrap(), json);
        assert_eq!(wsjtx::encode(&from_json.into_family()).unwrap(), encoded);
        assert_eq!(wsjtx::encode(&common.into_family()).unwrap(), encoded);
    }
});
