// SPDX-License-Identifier: GPL-3.0-or-later
//! Supplemental legacy cases: sequence variation, exhaustive prefixes and randomized binary64.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Codec,
    codec::{DecodeOptions, EncodeOptions, Profile},
    model::Event,
    wsjtx::{self, Wsjtx},
};
use serde_json::{Value, json};
fn fixtures() -> Value {
    serde_json::from_str(include_str!("fixtures/wire.json")).unwrap()
}
fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn all_120_synthetic_sample_decodes_and_every_prefix() {
    let all = fixtures();
    let vectors = all["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 120);
    for v in vectors {
        let bytes = unhex(v["wire_hex"].as_str().unwrap());
        let event = Wsjtx::default()
            .decode(&bytes, &DecodeOptions::default())
            .unwrap()
            .events
            .remove(0);
        assert!(qso_graph_interop::json::semantic_eq(
            &event.value()["fields"],
            &v["fields"]
        ));
        assert_eq!(
            Wsjtx::default()
                .reencode(&event, &EncodeOptions::default())
                .unwrap(),
            bytes
        );
        for n in 0..bytes.len() {
            if let Ok(out) = Wsjtx::default().decode(&bytes[..n], &DecodeOptions::default()) {
                assert_eq!(out.events.len(), 1);
                assert_eq!(
                    Wsjtx::default()
                        .reencode(&out.events[0], &EncodeOptions::default())
                        .unwrap(),
                    bytes[..n]
                );
            }
        }
    }
}
#[test]
fn finite_float_json_preserves_boundary_and_10000_random_bits() {
    let check = |bits: u64| {
        let f = f64::from_bits(bits);
        if !f.is_finite() {
            return;
        }
        let mut fields = json!({"operation":"reply","time_of_day":"12:34:56.000","snr_db":-12,"dt_seconds":f,"audio_offset_hz":1234,"mode":"~","text":"CQ W1AAA FN42"});
        fields["dt_seconds"] = serde_json::to_value(f).unwrap();
        let command=Event::from_value(json!({"spec":"0.1.0","family":"wsjtx","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"wire_id":"synthetic-instance","fields":fields})).unwrap();
        let bytes = Wsjtx::default()
            .encode_command(&command, &EncodeOptions::default())
            .unwrap();
        let mut candidates = vec![bytes];
        let mut decode = wsjtx::Decode::default();
        decode.delta_time_seconds = f;
        let mut wspr = wsjtx::WsprDecode::default();
        wspr.delta_time_seconds = f;
        for body in [wsjtx::Body::Decode(decode), wsjtx::Body::WsprDecode(wspr)] {
            let packet = wsjtx::Packet::new("synthetic-instance", body);
            let bytes = wsjtx::encode(&packet).unwrap();
            let packet_json = serde_json::to_vec(&packet).unwrap();
            let restored: wsjtx::Packet = serde_json::from_slice(&packet_json).unwrap();
            assert_eq!(wsjtx::encode(&restored).unwrap(), bytes);
            candidates.push(bytes);
        }
        for bytes in candidates {
            let event = Wsjtx::default()
                .decode(&bytes, &DecodeOptions::default())
                .unwrap()
                .events
                .remove(0);
            let json =
                qso_graph_interop::json::write(&event, true, Some(&Wsjtx::default())).unwrap();
            let again = qso_graph_interop::json::parse(&json, qso_graph_interop::model::MAX_RECORD)
                .unwrap();
            assert_eq!(
                again.value()["fields"]["dt_seconds"]
                    .as_f64()
                    .unwrap()
                    .to_bits(),
                bits
            );
            assert_eq!(
                Wsjtx::default()
                    .reencode(&again, &EncodeOptions::default())
                    .unwrap(),
                bytes
            );
            assert_eq!(
                Wsjtx::default().verify(&again).unwrap(),
                qso_graph_interop::model::Verification::Verified
            );
        }
    };
    for bits in [
        0x3f99_867e_711c_7600,
        0x7800_fec1_8e28_0768,
        0,
        1,
        0x8000_0000_0000_0000,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x0010_0000_0000_0001,
        0x3fef_ffff_ffff_ffff,
        0x3ff0_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
    ] {
        check(bits);
    }
    let mut bits = 42_u64;
    for _ in 0..10_000 {
        bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
        check(bits);
    }
}
#[test]
fn negotiation_and_mode_character_helpers() {
    let mut h = wsjtx::Heartbeat::default();
    h.max_schema = Some(4);
    assert_eq!(wsjtx::negotiate_schema(3, 3, &h), Ok(3));
    assert_eq!(wsjtx::negotiate_schema(3, 2, &h), Ok(2));
    assert_eq!(
        wsjtx::negotiate_schema(3, 3, &wsjtx::Heartbeat::default()),
        Ok(2)
    );
    assert_eq!(wsjtx::negotiate_schema(1, 3, &h), Ok(1));
    assert!(wsjtx::negotiate_schema(0, 3, &h).is_err());
    for (mode, code) in [
        ("FT8", "~"),
        ("FT4", "+"),
        ("JT65", "#"),
        ("JT9", "@"),
        ("JT4", "$"),
        ("MSK144", "&"),
        ("Q65", ":"),
        ("FST4", "`"),
        ("OTHER", "OTHER"),
    ] {
        assert_eq!(wsjtx::mode_character(mode), code);
    }
}

#[test]
fn strict_structured_output_and_typed_family_causes() {
    use std::error::Error as _;
    let fixture = &fixtures()["vectors"][0];
    let bytes = unhex(fixture["wire_hex"].as_str().unwrap());
    let mut options = DecodeOptions::default();
    options.structured = true;
    let event = Wsjtx::default()
        .decode(&bytes, &options)
        .unwrap()
        .events
        .remove(0);
    assert!(qso_graph_interop::json::write(&event, true, Some(&Wsjtx::default())).is_ok());
    let mut v = event.into_value();
    v["native"]["message"]["body"]["addition"] = true.into();
    let event = Event::from_value(v).unwrap();
    assert!(qso_graph_interop::json::write(&event, true, Some(&Wsjtx::default())).is_err());
    let err = Wsjtx::default()
        .decode(b"bad", &DecodeOptions::default())
        .unwrap_err();
    assert!(err.source().unwrap().is::<wsjtx::Error>());
    assert!(!err.to_string().contains("Truncated"));
    let command=Event::from_value(json!({"spec":"0.1.0","family":"wsjtx","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"wire_id":"synthetic-instance","fields":{"operation":"replay","auto_tx_only":false}})).unwrap();
    let err = Wsjtx::default()
        .encode_command(&command, &EncodeOptions::default())
        .unwrap_err();
    assert!(err.source().unwrap().is::<wsjtx::Error>());
}

#[test]
fn maximum_invalid_text_observation_fits_annex_b_budget_with_structured_view() {
    let mut bytes = Vec::new();
    for word in [wsjtx::MAGIC, 3, 9, 0] {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    let text_len = wsjtx::MAX_DATAGRAM - bytes.len() - 5;
    bytes.extend_from_slice(&(text_len as u32).to_be_bytes());
    bytes.resize(bytes.len() + text_len, 255);
    bytes.push(0);
    let mut options = DecodeOptions::default();
    options.structured = true;
    let event = Wsjtx::default()
        .decode(&bytes, &options)
        .unwrap()
        .events
        .remove(0);
    let record = qso_graph_interop::json::write(&event, true, Some(&Wsjtx::default())).unwrap();
    assert!(record.len() <= qso_graph_interop::model::MAX_RECORD);
    let restored =
        qso_graph_interop::json::parse(&record, qso_graph_interop::model::MAX_RECORD).unwrap();
    assert_eq!(restored.native_bytes().unwrap(), bytes);
    assert_eq!(
        qso_graph_interop::json::parse_carriage(&record, qso_graph_interop::model::MAX_RECORD)
            .unwrap_err()
            .category,
        qso_graph_interop::Category::Oversized
    );
    bytes.push(0);
    assert_eq!(
        Wsjtx::default()
            .decode(&bytes, &options)
            .unwrap_err()
            .category,
        qso_graph_interop::Category::Oversized
    );
}

#[test]
fn additive_mapping_checks_only_revision_one_mapped_content() {
    let mut halt = wsjtx::HaltTx::default();
    halt.auto_tx_only = true;
    let bytes = wsjtx::encode(&wsjtx::Packet::new(
        "synthetic-instance",
        wsjtx::Body::HaltTx(halt),
    ))
    .unwrap();
    let mut v = Wsjtx::default()
        .decode(&bytes, &DecodeOptions::default())
        .unwrap()
        .events
        .remove(0)
        .into_value();
    v["mapping"] = 2.into();
    v["fields"]["frequency_hz"] = 14_074_000.into();
    v["native"]["new_evidence"] = true.into();
    let event = Event::from_value(v.clone()).unwrap();
    assert_eq!(
        Wsjtx::default().verify(&event).unwrap(),
        qso_graph_interop::model::Verification::Verified
    );
    v["fields"]["auto_tx_only"] = false.into();
    let invalid = Event::from_value(v).unwrap();
    assert_eq!(
        Wsjtx::default().verify(&invalid).unwrap_err().category,
        qso_graph_interop::Category::InvalidValue
    );
}
