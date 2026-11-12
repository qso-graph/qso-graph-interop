// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent synthetic evidence and planted consistency/producer failures.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Codec, Event,
    codec::{DecodeOptions, EncodeOptions, Profile},
    n1mm::N1mm,
};
use serde_json::json;

#[test]
fn independent_radio_evidence_and_planted_swaps() {
    let bytes = include_bytes!("fixtures/n1mm-radio.xml");
    let mut codec = N1mm::default();
    let event = codec
        .decode(bytes, &DecodeOptions::default())
        .unwrap()
        .events
        .remove(0);
    assert_eq!(
        event.value()["fields"],
        json!({"radio":{"instance":"SYNTHETIC-PC","number":2},"dial_frequency_hz":137500,"focus_radio":1,"active_radio":2,"running":false,"stereo":true,"transmitting":false})
    );
    assert_eq!(
        codec.reencode(&event, &EncodeOptions::default()).unwrap(),
        bytes
    );
    assert!(event.value()["context"].get("source_id").is_none());
    for (a, b) in [
        ("focus_radio", "active_radio"),
        ("running", "stereo"),
        ("stereo", "transmitting"),
    ] {
        let mut value = event.value().clone();
        let x = value["fields"][a].clone();
        value["fields"][a] = value["fields"][b].clone();
        value["fields"][b] = x;
        assert_eq!(
            codec
                .verify(&Event::from_value(value).unwrap())
                .unwrap_err()
                .category,
            Category::InvalidValue
        );
    }
    let mut value = event.into_value();
    value["fields"]["radio"]["future"] = true.into();
    let added = Event::from_value(value).unwrap();
    assert!(qso_graph_interop::json::write(&added, true, Some(&codec)).is_err());
}

#[test]
fn profile_capacity_and_structured_view_are_closed_recursively() {
    let mut options = DecodeOptions::default();
    options.structured = true;
    let codec = N1mm::default();
    let event = N1mm::default()
        .decode(
            b"<lookup><timestamp>0001-01-01 00:00:00</timestamp></lookup>",
            &options,
        )
        .unwrap()
        .events
        .remove(0);
    assert_eq!(
        event.recognized(Some(&codec)).unwrap().fields,
        json!({"start":"0001-01-01T00:00:00Z"})
    );
    qso_graph_interop::json::write(&event, true, Some(&codec)).unwrap();
    for key in ["extra", "xml"] {
        let mut value = event.value().clone();
        value["native"]["message"][key] = "planted".into();
        assert!(
            qso_graph_interop::json::write(&Event::from_value(value).unwrap(), true, Some(&codec))
                .is_err()
        );
    }
    let mut value = event.into_value();
    value["family"] = "wsjtx".into();
    assert!(
        qso_graph_interop::json::write(&Event::from_value(value).unwrap(), true, None).is_err()
    );
}

#[test]
fn tokenizer_fault_order_and_original_unit_budgets() {
    let mut codec = N1mm::default();
    for (input, category) in [
        (
            b"<?xml version=\"1.1\"?><!DOCTYPE x><x/>".as_slice(),
            Category::UnsupportedVersion,
        ),
        (b"<x><!DOCTYPE x>".as_slice(), Category::Malformed),
        (b"<x><y".as_slice(), Category::Truncated),
        (b"<x a=\"\xff\"/>".as_slice(), Category::Malformed),
        (b"<x><!--\xff--></x>".as_slice(), Category::Malformed),
        (
            b"<RadioInfo><Freq>\xff</Freq></RadioInfo>".as_slice(),
            Category::InvalidValue,
        ),
    ] {
        let result = codec.decode(input, &DecodeOptions::default());
        if category == Category::InvalidValue {
            // invalid numeric text is native-only, a successful observation
            assert!(
                result.unwrap().events[0].value()["fields"]
                    .as_object()
                    .unwrap()
                    .is_empty()
            );
        } else {
            assert_eq!(result.unwrap_err().category, category);
        }
    }
}

#[test]
fn text_admission_precedes_later_reference_faults_and_errors_keep_family_causes() {
    use std::error::Error as _;
    let mut codec = N1mm::default();
    // UTF-16 expands to 65,508 UTF-8 bytes before the later bad reference.
    for tail in ["&missing;</RadioInfo>", "&unfinished"] {
        let xml = format!("<RadioInfo>{}{tail}", "\u{0800}".repeat(21_836));
        let mut bytes = vec![0xff, 0xfe];
        for unit in xml.encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        let error = codec.decode(&bytes, &DecodeOptions::default()).unwrap_err();
        assert_eq!(error.category, Category::Oversized);
        assert_eq!(
            error
                .source()
                .unwrap()
                .downcast_ref::<qso_graph_interop::n1mm::N1mmError>()
                .unwrap()
                .category,
            Category::Oversized
        );
    }
    for (input, category) in [
        (b"<RadioInfo>&unfinished".as_slice(), Category::Truncated),
        (
            b"<RadioInfo>&unfinished</RadioInfo>".as_slice(),
            Category::Malformed,
        ),
    ] {
        assert_eq!(
            codec
                .decode(input, &DecodeOptions::default())
                .unwrap_err()
                .category,
            category
        );
    }
}

#[test]
fn xml_names_and_closed_attribute_references_are_validated_before_resources() {
    let mut codec = N1mm::default();
    assert!(
        codec
            .decode(b"<?:1 ok?><AppInfo/>", &DecodeOptions::default())
            .is_ok()
    );
    for input in [
        b"<?1bad?><AppInfo/>".as_slice(),
        b"<AppInfo a='&missing'/>".as_slice(),
    ] {
        assert_eq!(
            codec
                .decode(input, &DecodeOptions::default())
                .unwrap_err()
                .category,
            Category::Malformed
        );
    }
}

#[test]
fn future_mapping_checks_assigned_fields_without_guessing_new_native_versions() {
    use qso_graph_interop::model::Verification;
    let mut codec = N1mm::default();
    let mut value = codec
        .decode(
            b"<spot><dxcall>Q2AAA</dxcall></spot>",
            &DecodeOptions::default(),
        )
        .unwrap()
        .events
        .remove(0)
        .into_value();
    value["mapping"] = 2.into();
    value["fields"]["grid"] = "AA00".into(); // core capacity, not an Annex D assignment
    value["fields"]["radio"] = json!({"number":2}); // a future assignment, not a carried StationName
    assert_eq!(
        codec
            .verify(&Event::from_value(value.clone()).unwrap())
            .unwrap(),
        Verification::Verified
    );
    value["fields"]["call"] = "Q2BBB".into();
    assert!(
        codec
            .verify(&Event::from_value(value.clone()).unwrap())
            .is_err()
    );
    value["fields"]["call"] = "Q2AAA".into();
    value["native_version"] = "future".into();
    let event = Event::from_value(value.clone()).unwrap();
    let read = event.recognized(Some(&codec)).unwrap();
    assert_eq!(read.verification, Verification::Unavailable);
    assert_eq!(read.kind, "native");
    assert_eq!(read.fields, json!({}));
    for key in ["native_version", "wire_id"] {
        let mut authored = value.clone();
        authored["mapping"] = 1.into();
        authored.as_object_mut().unwrap().remove("native_version");
        authored[key] = "planted".into();
        assert!(
            codec
                .validate_authored(&Event::from_value(authored).unwrap())
                .is_err()
        );
    }
}

#[test]
fn every_proper_prefix_of_complete_xml_is_unfinished() {
    let mut codec = N1mm::default();
    for xml in [
        b"<RadioInfo/>".as_slice(),
        b"<RadioInfo a='x'><Mode>CW</Mode></RadioInfo>".as_slice(),
    ] {
        assert!(codec.decode(xml, &DecodeOptions::default()).is_ok());
        for end in 0..xml.len() {
            assert_eq!(
                codec
                    .decode(&xml[..end], &DecodeOptions::default())
                    .unwrap_err()
                    .category,
                Category::Truncated,
                "prefix {end}"
            );
        }
    }
    assert_eq!(
        codec
            .decode(b"<RadioInfo/ x>", &DecodeOptions::default())
            .unwrap_err()
            .category,
        Category::Malformed
    );
}
