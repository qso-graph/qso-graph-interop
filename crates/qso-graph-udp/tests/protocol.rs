// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use qso_graph_udp::{model, wsjtx::*};

fn vectors() -> serde_json::Value {
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
fn fixture(name: &str) -> Packet {
    let all = vectors();
    let vector = all["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    let packet: Packet = serde_json::from_value(vector["packet"].clone()).unwrap();
    let bytes = unhex(vector["wire_hex"].as_str().unwrap());
    assert_eq!(
        decode(&bytes, packet.variant).unwrap(),
        Outcome::Message(Box::new(packet.clone())),
        "{name}"
    );
    assert_eq!(encode(&packet).unwrap(), bytes, "{name}");
    let common = model::Message::from(packet.clone());
    assert_eq!(
        common.clone().into_family(),
        packet,
        "{name}: common model lost data"
    );
    let json = serde_json::to_vec(&common).unwrap();
    assert_eq!(
        serde_json::from_slice::<model::Message>(&json).unwrap(),
        common
    );
    packet
}

#[test]
fn wsjtx_heartbeat() {
    fixture("wsjtx_Heartbeat");
}

#[test]
fn wsjtx_status() {
    fixture("wsjtx_Status");
}

#[test]
fn wsjtx_decode() {
    fixture("wsjtx_Decode");
}

#[test]
fn wsjtx_clear() {
    fixture("wsjtx_Clear");
}

#[test]
fn wsjtx_reply() {
    fixture("wsjtx_Reply");
}

#[test]
fn wsjtx_qsologged() {
    fixture("wsjtx_QsoLogged");
}

#[test]
fn wsjtx_close() {
    fixture("wsjtx_Close");
}

#[test]
fn wsjtx_replay() {
    fixture("wsjtx_Replay");
}

#[test]
fn wsjtx_halttx() {
    fixture("wsjtx_HaltTx");
}

#[test]
fn wsjtx_freetext() {
    fixture("wsjtx_FreeText");
}

#[test]
fn wsjtx_wsprdecode() {
    fixture("wsjtx_WsprDecode");
}

#[test]
fn wsjtx_location() {
    fixture("wsjtx_Location");
}

#[test]
fn wsjtx_loggedadif() {
    fixture("wsjtx_LoggedAdif");
}

#[test]
fn wsjtx_highlightcallsign() {
    fixture("wsjtx_HighlightCallsign");
}

#[test]
fn wsjtx_switchconfiguration() {
    fixture("wsjtx_SwitchConfiguration");
}

#[test]
fn wsjtx_configure() {
    fixture("wsjtx_Configure");
}

#[test]
fn wsjtx_annotationinfo() {
    fixture("wsjtx_AnnotationInfo");
}

#[test]
fn jtdx_heartbeat() {
    fixture("jtdx_Heartbeat");
}

#[test]
fn jtdx_status() {
    fixture("jtdx_Status");
}

#[test]
fn jtdx_decode() {
    fixture("jtdx_Decode");
}

#[test]
fn jtdx_clear() {
    fixture("jtdx_Clear");
}

#[test]
fn jtdx_reply() {
    fixture("jtdx_Reply");
}

#[test]
fn jtdx_qsologged() {
    fixture("jtdx_QsoLogged");
}

#[test]
fn jtdx_close() {
    fixture("jtdx_Close");
}

#[test]
fn jtdx_replay() {
    fixture("jtdx_Replay");
}

#[test]
fn jtdx_halttx() {
    fixture("jtdx_HaltTx");
}

#[test]
fn jtdx_freetext() {
    fixture("jtdx_FreeText");
}

#[test]
fn jtdx_wsprdecode() {
    fixture("jtdx_WsprDecode");
}

#[test]
fn jtdx_loggedadif() {
    fixture("jtdx_LoggedAdif");
}

#[test]
fn jtdx_settxdeltafreq() {
    fixture("jtdx_SetTxDeltaFreq");
}

#[test]
fn jtdx_triggercq() {
    fixture("jtdx_TriggerCq");
}

#[test]
fn jtdx_triggercq_source() {
    fixture("jtdx_TriggerCq_source");
}

#[test]
fn jtdx_reply_source() {
    fixture("jtdx_Reply_source");
}

#[test]
fn jtdx_wsprdecode_source() {
    fixture("jtdx_WsprDecode_source");
}

#[test]
fn datetime_s1_spec0() {
    fixture("datetime_s1_spec0");
}

#[test]
fn datetime_s1_spec1() {
    fixture("datetime_s1_spec1");
}

#[test]
fn datetime_s1_spec2() {
    fixture("datetime_s1_spec2");
}

#[test]
fn datetime_s1_spec3() {
    fixture("datetime_s1_spec3");
}

#[test]
fn datetime_s2_spec0() {
    fixture("datetime_s2_spec0");
}

#[test]
fn datetime_s2_spec1() {
    fixture("datetime_s2_spec1");
}

#[test]
fn datetime_s2_spec2() {
    fixture("datetime_s2_spec2");
}

#[test]
fn datetime_s2_spec3() {
    fixture("datetime_s2_spec3");
}

#[test]
fn datetime_s3_spec0() {
    fixture("datetime_s3_spec0");
}

#[test]
fn datetime_s3_spec1() {
    fixture("datetime_s3_spec1");
}

#[test]
fn datetime_s3_spec2() {
    fixture("datetime_s3_spec2");
}

#[test]
fn datetime_s3_spec3() {
    fixture("datetime_s3_spec3");
}

#[test]
fn datetime_null() {
    fixture("datetime_null");
}

#[test]
fn datetime_custom_zone() {
    fixture("datetime_custom_zone");
}

#[test]
fn prototype_clear() {
    fixture("prototype_Clear");
}

#[test]
fn prototype_heartbeat_old() {
    fixture("prototype_Heartbeat_old");
}

#[test]
fn prototype_status_old() {
    fixture("prototype_Status_old");
}

#[test]
fn null_string() {
    fixture("null_string");
}

#[test]
fn empty_string() {
    fixture("empty_string");
}

#[test]
fn utf8_unicode() {
    fixture("utf8_unicode");
}

#[test]
fn utf8_invalid() {
    fixture("utf8_invalid");
}

#[test]
fn future_tail() {
    fixture("future_tail");
}

#[test]
fn all_120_sample_decodes() {
    let all = vectors();
    let names: Vec<_> = all["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["name"].as_str().filter(|s| s.starts_with("sample_")))
        .collect();
    assert_eq!(names.len(), 120);
    for name in names {
        fixture(name);
    }
}

#[test]
fn every_datagram_prefix_is_safe_and_partial_fields_are_errors() {
    let all = vectors();
    for v in all["vectors"].as_array().unwrap() {
        let packet: Packet = serde_json::from_value(v["packet"].clone()).unwrap();
        let bytes = unhex(v["wire_hex"].as_str().unwrap());
        for end in 0..bytes.len() {
            let result = decode(&bytes[..end], packet.variant);
            if let Ok(Outcome::Message(short)) = result {
                // Only a boundary between complete fields may succeed. Re-encoding
                // gives exactly that shorter message for all canonical fixtures.
                assert_eq!(
                    encode(&short).unwrap(),
                    bytes[..end],
                    "{} at {end}",
                    v["name"]
                );
            }
        }
    }
    let p = fixture("wsjtx_Decode");
    let b = encode(&p).unwrap();
    assert!(matches!(
        decode(&b[..b.len() - 5], Variant::Wsjtx),
        Err(Error::Truncated { .. })
    ));
}

#[test]
fn unknown_types_and_future_schemas_are_skipped_without_parsing_their_layout() {
    let p = fixture("wsjtx_Decode");
    let mut b = encode(&p).unwrap();
    b[8..12].copy_from_slice(&99_u32.to_be_bytes());
    assert_eq!(
        decode(&b, Variant::Wsjtx),
        Ok(Outcome::Skipped {
            schema: 3,
            message_type: Some(99),
            reason: SkipReason::UnknownType
        })
    );
    b[4..8].copy_from_slice(&4_u32.to_be_bytes());
    assert_eq!(
        decode(&b[..8], Variant::Wsjtx),
        Ok(Outcome::Skipped {
            schema: 4,
            message_type: None,
            reason: SkipReason::FutureSchema
        })
    );
    let mut location = encode(&fixture("wsjtx_Location")).unwrap();
    assert!(matches!(
        decode(&location, Variant::Jtdx),
        Ok(Outcome::Skipped {
            reason: SkipReason::UnknownType,
            ..
        })
    ));
    location[4..8].copy_from_slice(&0_u32.to_be_bytes());
    assert_eq!(
        decode(&location, Variant::Wsjtx),
        Err(Error::UnsupportedSchema(0))
    );
}

#[test]
fn wrong_magic_and_malicious_lengths() {
    for bytes in [&[][..], &[0; 20][..]] {
        assert!(decode(bytes, Variant::Wsjtx).is_err());
    }
    let mut bytes = encode(&fixture("wsjtx_FreeText")).unwrap();
    bytes[0] = 0;
    assert_eq!(decode(&bytes, Variant::Wsjtx), Err(Error::WrongMagic));
    bytes[0] = 0xad;
    for length in [257_u32, 65_507, 65_536, 0xffff_fffe] {
        bytes[12..16].copy_from_slice(&length.to_be_bytes());
        assert_eq!(decode(&bytes, Variant::Wsjtx), Err(Error::Length));
    }
    let mut bytes = encode(&fixture("wsjtx_FreeText")).unwrap();
    bytes[22..26].copy_from_slice(&0xffff_fffe_u32.to_be_bytes());
    assert_eq!(decode(&bytes, Variant::Wsjtx), Err(Error::Length));
    assert_eq!(
        decode(&vec![0; MAX_DATAGRAM + 1], Variant::Wsjtx),
        Err(Error::Length)
    );
}

#[test]
fn maximum_datagram_and_id_are_bounded_in_both_directions() {
    let mut p = Packet::new(
        "x".repeat(MAX_ID),
        Body::FreeText(FreeText {
            text: Utf8::from(""),
            send: false,
        }),
    );
    // 12 header + 4 ID length + 256 ID + 4 text length + 1 boolean.
    let max_text = MAX_DATAGRAM - (12 + 4 + MAX_ID + 4 + 1);
    if let Body::FreeText(t) = &mut p.body {
        t.text = Utf8 {
            bytes: Some(vec![b'x'; max_text]),
        };
    }
    let bytes = encode(&p).unwrap();
    assert_eq!(bytes.len(), MAX_DATAGRAM);
    assert_eq!(
        decode(&bytes, Variant::Wsjtx).unwrap(),
        Outcome::Message(Box::new(p.clone()))
    );
    if let Body::FreeText(t) = &mut p.body {
        t.text.bytes.as_mut().unwrap().push(b'x');
    }
    assert_eq!(encode(&p), Err(Error::Length));
    p.id = Utf8::from("x".repeat(MAX_ID + 1));
    assert_eq!(encode(&p), Err(Error::Length));
}

#[test]
fn null_id_and_optional_null_strings_remain_distinct_from_absent_and_empty() {
    let mut p = Packet::new(
        Utf8::default(),
        Body::Heartbeat(Heartbeat {
            max_schema: Some(3),
            version: Some(Utf8::default()),
            revision: Some("".into()),
        }),
    );
    let b = encode(&p).unwrap();
    assert_eq!(&b[12..16], &u32::MAX.to_be_bytes());
    assert_eq!(
        decode(&b, Variant::Wsjtx).unwrap(),
        Outcome::Message(Box::new(p.clone()))
    );
    let json = serde_json::to_vec(&p).unwrap();
    assert_eq!(serde_json::from_slice::<Packet>(&json).unwrap(), p);
    p.body = Body::Heartbeat(Heartbeat::default());
    assert_eq!(encode(&p).unwrap().len(), 16);
}

#[test]
fn noncontiguous_optional_fields_and_variant_confusion_are_refused() {
    let p = Packet::new(
        "WSJT-X",
        Body::Heartbeat(Heartbeat {
            max_schema: None,
            version: Some("x".into()),
            revision: None,
        }),
    );
    assert_eq!(encode(&p), Err(Error::NonContiguousTail));
    let mut p = fixture("wsjtx_Status");
    p.variant = Variant::Jtdx;
    assert_eq!(encode(&p), Err(Error::VariantMismatch));
    let mut p = fixture("jtdx_Status");
    p.variant = Variant::Wsjtx;
    assert_eq!(encode(&p), Err(Error::VariantMismatch));
}

#[test]
fn schema_negotiation_never_answers_above_the_peer() {
    let h = Heartbeat {
        max_schema: Some(4),
        ..Heartbeat::default()
    };
    assert_eq!(negotiate_schema(3, 3, &h), Ok(3));
    assert_eq!(negotiate_schema(3, 2, &h), Ok(2));
    assert_eq!(negotiate_schema(3, 3, &Heartbeat::default()), Ok(2));
    assert_eq!(negotiate_schema(1, 3, &h), Ok(1));
    assert_eq!(negotiate_schema(0, 3, &h), Err(Error::UnsupportedSchema(0)));
}

#[test]
fn all_mode_characters_match_the_reference() {
    for (mode, code) in [
        ("FT8", "~"),
        ("FT4", "+"),
        ("JT65", "#"),
        ("JT9", "@"),
        ("JT4", "$"),
        ("MSK144", "&"),
        ("Q65", ":"),
        ("FST4", "`"),
        ("SOMETHING", "SOMETHING"),
    ] {
        assert_eq!(mode_character(mode), code);
    }
}

#[test]
fn normalized_fields_keep_units_and_variant_meaning() {
    let message = model::Message::from(fixture("jtdx_Status"));
    let model::Kind::RadioState(radio) = message.kind else {
        panic!("radio");
    };
    assert_eq!(radio.rx_offset_hz, -150);
    for (name, variant) in [
        ("wsjtx_QsoLogged", Variant::Wsjtx),
        ("jtdx_QsoLogged", Variant::Jtdx),
    ] {
        let message = model::Message::from(fixture(name));
        let model::Kind::LoggedQso(q) = message.kind else {
            panic!("QSO");
        };
        assert_eq!(
            q.tx_frequency_hz,
            (variant == Variant::Wsjtx).then_some(14_074_000)
        );
        assert_eq!(
            q.dial_frequency_hz,
            (variant == Variant::Jtdx).then_some(14_074_000)
        );
    }
    let message = model::Message::from(fixture("wsjtx_Decode"));
    let model::Kind::Decode(d) = message.kind else {
        panic!("decode");
    };
    assert_eq!(d.offset_hz, Some(1234));
    assert_eq!(d.call, None); // A callsign in free text is not an explicit field.
}

#[test]
fn invalid_datetime_layout_is_refused() {
    let mut p = fixture("wsjtx_QsoLogged");
    let Body::QsoLogged(q) = &mut p.body else {
        panic!("QSO");
    };
    q.time_off.time_spec = 2;
    assert_eq!(encode(&p), Err(Error::InconsistentDateTime));
    let Body::QsoLogged(q) = &mut p.body else {
        panic!("QSO");
    };
    q.time_off.time_spec = 99;
    assert_eq!(encode(&p), Err(Error::InvalidTimeSpec(99)));
    let mut bytes = encode(&fixture("wsjtx_QsoLogged")).unwrap();
    bytes[22 + 12] = 99;
    assert_eq!(
        decode(&bytes, Variant::Wsjtx),
        Err(Error::InvalidTimeSpec(99))
    );
}

#[test]
fn ieee_nan_infinity_and_signed_zero_preserve_wire_bits() {
    for bits in [
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        (-0.0_f64).to_bits(),
        0x7ff8_0123_4567_89ab,
    ] {
        let mut p = fixture("wsjtx_Decode");
        if let Body::Decode(d) = &mut p.body {
            d.delta_time_seconds = f64::from_bits(bits);
        }
        let bytes = encode(&p).unwrap();
        let Outcome::Message(decoded) = decode(&bytes, Variant::Wsjtx).unwrap() else {
            panic!("message");
        };
        let Body::Decode(d) = &decoded.body else {
            panic!("decode");
        };
        assert_eq!(d.delta_time_seconds.to_bits(), bits);
        assert_eq!(encode(&decoded).unwrap(), bytes);
        let json = serde_json::to_vec(&decoded).unwrap();
        let from_json: Packet = serde_json::from_slice(&json).unwrap();
        assert_eq!(encode(&from_json).unwrap(), bytes);
    }
}

#[test]
fn extensions_cannot_fill_missing_known_suffix_fields() {
    let mut p = fixture("prototype_Heartbeat_old");
    p.extensions = vec![1, 2, 3];
    assert_eq!(encode(&p), Err(Error::NonContiguousTail));
    let mut p = fixture("jtdx_Heartbeat");
    p.extensions = vec![1, 2, 3];
    let bytes = encode(&p).unwrap();
    assert_eq!(
        decode(&bytes, Variant::Jtdx).unwrap(),
        Outcome::Message(Box::new(p))
    );
    let mut p = fixture("jtdx_TriggerCq_source");
    p.extensions = vec![0];
    assert_eq!(encode(&p), Err(Error::NonContiguousTail));
}
