// SPDX-License-Identifier: GPL-3.0-or-later
//! Family-free API, parser and profile checks; also runs with default features disabled.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Codec, Context, Event, Text,
    codec::*,
    json,
    model::{Instant, Integer, MAX_RECORD, TimeOfDay, Verification},
};
use serde_json::{Value, json};
fn command() -> Value {
    json!({"spec":"0.1.0","family":"qgi","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":{"operation":"replay"}})
}
#[test]
fn bounded_json_preserves_unknown_values_and_rejects_duplicates() {
    let mut value = command();
    value["future"] = json!({"$serde_json::private::Number":"12345"});
    let mut bytes = serde_json::to_vec(&value).unwrap();
    // Unknown integer tokens remain exact, even beyond the machine integer range.
    bytes.pop();
    bytes.extend_from_slice(b",\"big\":123456789012345678901234567890}");
    let event = json::parse(&bytes, MAX_RECORD).unwrap();
    assert_eq!(event.value()["future"], value["future"]);
    assert_eq!(
        event.value()["big"].to_string(),
        "123456789012345678901234567890"
    );
    #[cfg(feature = "serde")]
    {
        let serialized = serde_json::to_vec(&event).unwrap();
        let roundtrip: Event = serde_json::from_slice(&serialized).unwrap();
        assert!(json::semantic_eq(event.value(), roundtrip.value()));
    }
    for member in [
        "\"fields\":{}",
        "\"future\":{\"x\":1,\"x\":2}",
        "\"future\":[{\"x\":1,\"x\":2}]",
    ] {
        let mut raw = serde_json::to_string(&command()).unwrap();
        raw.pop();
        raw.push(',');
        raw.push_str(member);
        raw.push('}');
        assert_eq!(
            json::parse(raw.as_bytes(), MAX_RECORD)
                .unwrap_err()
                .category(),
            Category::Malformed
        );
        #[cfg(feature = "serde")]
        assert!(serde_json::from_str::<Event>(&raw).is_err());
    }
    assert_eq!(
        json::parse(&vec![b' '; MAX_RECORD + 1], MAX_RECORD)
            .unwrap_err()
            .category(),
        Category::Oversized
    );
}
#[test]
fn neutral_values_keep_null_empty_absent_precision_and_invalid_text() {
    let mut context = Context::default();
    context = context.with_source_id(Text::Null);
    context = context.with_target_id(Text::from(""));
    assert_eq!(context.value()["source_id"], Value::Null);
    assert_eq!(context.value()["target_id"], "");
    assert!(context.value().get("correlation_id").is_none());
    #[cfg(feature = "serde")]
    {
        let context: Context = serde_json::from_value(
            json!({"direction":"unknown","exchange":"unknown","source_id":null,"target_id":""}),
        )
        .unwrap();
        assert_eq!(context.as_ref().source_id(), Some(Text::Null));
        assert_eq!(context.as_ref().target_id(), Some(Text::from("")));
        assert_eq!(context.as_ref().correlation_id(), None);
        assert!(
            serde_json::to_value(context)
                .unwrap()
                .get("correlation_id")
                .is_none()
        );
    }
    assert!(Integer::try_from(9_007_199_254_740_991).is_ok());
    assert!(Integer::try_from(9_007_199_254_740_992).is_err());
    assert!(TimeOfDay::try_from("12:34:56.123".to_owned()).is_ok());
    assert!(TimeOfDay::try_from("24:00:00".to_owned()).is_err());
    assert!(Instant::try_from("2000-02-29T00:00:00Z".to_owned()).is_ok());
    assert!(Instant::try_from("1900-02-29T00:00:00Z".to_owned()).is_err());
    #[cfg(feature = "serde")]
    for bits in [
        0,
        0x8000_0000_0000_0000,
        0x7ff8_0123_4567_89ab,
        0xfff0_0000_0000_0000,
    ] {
        let f = qso_graph_interop::model::Binary64::from_bits(bits);
        let b = serde_json::to_vec(&f).unwrap();
        let again: qso_graph_interop::model::Binary64 = serde_json::from_slice(&b).unwrap();
        assert_eq!(again.to_bits(), bits);
    }
    for (units, valid) in [
        (json!({"utf8_bytes":[255]}), true),
        (json!({"utf16_units":[55296]}), true),
        (json!({"utf8_bytes":[65]}), false),
        (json!({"utf16_units":[65]}), false),
    ] {
        let mut v = command();
        v["wire_id"] = units;
        assert_eq!(Event::from_value(v).is_ok(), valid);
    }
}
#[test]
fn recursive_reader_and_producer_profiles() {
    let mut native = json!({"spec":"0.1.0","family":"qgi","mapping":1,"kind":"decode","context":{"direction":"unknown","exchange":"unknown"},"fields":{"radio":{"instance":"synthetic-instance"}},"native":{"format":"datagram","data":""}});
    let base = Event::from_value(native.clone()).unwrap();
    assert!(
        json::write(
            &base,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Authored
            )
        )
        .is_ok()
    );
    native["fields"]["radio"]["addition"] = true.into();
    let future = Event::from_value(native).unwrap();
    assert!(
        json::write(
            &future,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Preserved
            )
        )
        .is_ok()
    );
    assert_eq!(
        json::write(
            &future,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Authored
            )
        )
        .unwrap_err()
        .category(),
        Category::InvalidValue
    );
    let mut v = command();
    v["context"]["direction"] = "future".into();
    v["fields"]["addition"] = true.into();
    let event = Event::from_value(v).unwrap();
    let view = event.recognized(None).unwrap();
    assert_eq!(view.context().as_value()["direction"], "unknown");
    assert_eq!(
        event.context().direction(),
        qso_graph_interop::model::DirectionRef::Other("future")
    );
    assert_eq!(
        view.context().direction(),
        qso_graph_interop::model::DirectionRef::Unknown
    );
    assert!(view.fields().as_value().get("addition").is_none());
    assert!(
        json::write(
            &event,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Authored
            )
        )
        .is_err()
    );
    for native in [
        json!({"format":"mqtt_publish","data":"","topic":null,"retained":false}),
        json!({"format":"ipfix_record","data":"","template":"","message_data":"","header":{"version":10,"length":16,"export_time":0,"sequence_number":0,"observation_domain_id":0},"set_id":256,"set_index":0,"record_index":0}),
    ] {
        let mut v = command();
        v["native"] = native;
        let event = Event::from_value(v.clone()).unwrap();
        assert!(
            json::write(
                &event,
                &qso_graph_interop::PublicationOptions::new(
                    qso_graph_interop::PublicationMode::Authored
                )
            )
            .is_ok()
        );
        v["native"]["header"] = json!({"version":65536,"length":16,"export_time":0,"sequence_number":0,"observation_domain_id":0});
        assert!(Event::from_value(v).is_err());
    }
}

/// An external crate implements the trait with bounded caller-scoped state, without family imports.
#[derive(Default)]
struct External {
    session: Vec<u8>,
    messages: u64,
}
impl Profile for External {
    fn native_version_supported(&self, _: &Event) -> bool {
        true
    }
    fn validate_authored(
        &self,
        _: &Event,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<(), qso_graph_interop::Error> {
        Ok(())
    }
    fn recognize(
        &self,
        _: &Event,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<qso_graph_interop::model::ProfileView, qso_graph_interop::Error> {
        qso_graph_interop::model::ProfileView::new(
            qso_graph_interop::Kind::Decode,
            json!({"text":"external interpretation"})
                .as_object()
                .unwrap()
                .clone(),
            Verification::Unavailable,
        )
    }
    fn family(&self) -> &str {
        "x-org.example.test"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn verify(
        &self,
        _: &Event,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<Verification, qso_graph_interop::Error> {
        Ok(Verification::Unavailable)
    }
}
impl Codec for External {
    fn framing(&self) -> Framing {
        Framing::Templated
    }
    fn limits(&self) -> Limits {
        let mut l = Limits::stateless(8, 8);
        l.sessions = 1;
        l.templates_per_session = 1;
        l.template_bytes = 8;
        l.dependency_bytes = 8;
        l
    }
    fn reset(&mut self, session: Option<&[u8]>) {
        if session.is_none_or(|s| s == self.session) {
            self.session.clear();
            self.messages = 0;
        }
    }
    fn decode(
        &mut self,
        input: &[u8],
        options: &DecodeOptions<'_>,
    ) -> Result<Outcome, qso_graph_interop::Error> {
        if input.len() > 8
            || options.session().len() > 8
            || !self.session.is_empty() && self.session != options.session()
        {
            return Err(qso_graph_interop::Error::new(
                Category::Oversized,
                "external state bound exceeded",
            ));
        }
        if input == b"bad" {
            return Err(qso_graph_interop::Error::new(
                Category::Malformed,
                "planted failure after staging",
            ));
        }
        let staged = (options.session().to_vec(), self.messages + 1);
        self.session = staged.0;
        self.messages = staged.1;
        Ok(Outcome::default())
    }
    fn encode_commands(
        &self,
        _: &Event,
        _: &EncodeOptions,
    ) -> Result<qso_graph_interop::codec::EncodedCommands, qso_graph_interop::Error> {
        Err(qso_graph_interop::Error::new(
            Category::InvalidValue,
            "no commands",
        ))
    }
    fn reencode(
        &self,
        _: &Event,
        _: &EncodeOptions,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<Vec<u8>, qso_graph_interop::Error> {
        Err(qso_graph_interop::Error::new(
            Category::InvalidValue,
            "no evidence",
        ))
    }
}
#[test]
fn external_implementation_session_bounds_reset_and_atomic_errors() {
    let mut codec = External::default();
    let mut options = DecodeOptions::default();
    options = options.with_session(b"session");
    options = options.with_expiry(Some(42));
    assert!(
        codec
            .decode(b"template", &options)
            .unwrap()
            .events
            .is_empty()
    );
    assert_eq!(codec.messages, 1);
    assert!(codec.decode(b"bad", &options).is_err());
    assert_eq!(codec.messages, 1);
    options = options.with_session(b"other");
    assert_eq!(
        codec.decode(b"template", &options).unwrap_err().category(),
        Category::Oversized
    );
    codec.reset(Some(b"unrelated"));
    assert_eq!(codec.messages, 1);
    codec.reset(Some(b"session"));
    assert_eq!(codec.messages, 0);
    assert!(codec.decode(b"template", &options).is_ok());
    codec.reset(None);
    assert!(codec.session.is_empty());
}

#[test]
fn canonical_base64_and_exact_semantic_numbers() {
    let number = |s: &str| Value::Number(s.parse().unwrap());
    for (a, b) in [
        ("1", "1.0"),
        ("1000", "1e3"),
        ("12300", "1.2300e4"),
        ("-0", "0.00e99999999999999999999999999999"),
        (
            "1e99999999999999999999999999999",
            "10e99999999999999999999999999998",
        ),
        (
            "1e-99999999999999999999999999999",
            "10e-100000000000000000000000000000",
        ),
    ] {
        assert!(json::semantic_eq(&number(a), &number(b)), "{a} versus {b}");
    }
    for (a, b) in [
        ("9007199254740992", "9007199254740993"),
        ("1e9999999999999999999999", "1e9999999999999999999998"),
        ("1e-9999999999999999999999", "1e-9999999999999999999998"),
    ] {
        assert!(!json::semantic_eq(&number(a), &number(b)));
    }
    assert_eq!(
        json::integer_u64(&number("4294967295.000")),
        Some(4294967295)
    );
    assert_eq!(
        json::integer_i64(&number("-2147483648.0")),
        Some(-2147483648)
    );
    assert_eq!(json::integer_u64(&number("0.1")), None);
}

#[test]
fn request_response_context_and_application_faults_need_no_family_dependency() {
    use qso_graph_interop::model::{Direction, Exchange, NativeEvidence};
    let mut context = Context::new(Direction::FromApplication, Exchange::Response);
    context = context.with_correlation_id(Text::from("synthetic-request"));
    let bytes = b"<methodResponse><fault>synthetic rejection</fault></methodResponse>";
    let native = NativeEvidence::new("xmlrpc_response", bytes).unwrap();
    let event=Event::from_value(json!({"spec":"0.1.0","family":"fldigi","mapping":1,"kind":"result","context":context.value(),"fields":{"status":"error","text":"synthetic rejection"},"native":native.into_value()})).unwrap();
    assert_eq!(event.native_bytes().unwrap(), bytes);
    assert!(event.value().get("wire_id").is_none());
    assert!(event.value()["context"].get("source_id").is_none());
    let read = event.recognized(None).unwrap();
    assert_eq!(read.fields().as_value()["status"], "error");
    assert_eq!(
        read.context().as_value()["correlation_id"],
        "synthetic-request"
    );
    assert_eq!(read.verification(), Verification::Unavailable);
}

#[test]
fn n1mm_profile_shapes_work_without_loading_a_family() {
    let mut value = command();
    value["family"] = "n1mm".into();
    value["fields"] = json!({"operation":"n1mm.spectrum","spectrum_data":[0,65535],"app":"","scaling_factor":"001.00"});
    let event = Event::from_value(value.clone()).unwrap();
    json::write(
        &event,
        &qso_graph_interop::PublicationOptions::new(qso_graph_interop::PublicationMode::Authored),
    )
    .unwrap();
    assert_eq!(
        event.recognized(None).unwrap().fields().as_value().clone(),
        value["fields"]
    );
    for bad in [
        json!([-1]),
        json!([65536]),
        json!([0.5]),
        json!(["1"]),
        json!([true]),
        json!(vec![0; 4097]),
    ] {
        value["fields"]["spectrum_data"] = bad;
        assert!(Event::from_value(value.clone()).is_err());
    }
    value["fields"]["spectrum_data"] = json!([]); // reader capacity; command writer requires at least one
    assert!(Event::from_value(value.clone()).is_ok());
    value["family"] = "wsjtx".into();
    let other = Event::from_value(value).unwrap();
    assert_eq!(
        other.recognized(None).unwrap().fields().as_value().clone(),
        json!({"operation":"n1mm.spectrum"})
    );
    assert!(
        json::write(
            &other,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Authored
            )
        )
        .is_err()
    );
}

#[test]
fn matching_external_profile_reaches_recognition() {
    let mut value = command();
    value["family"] = "x-org.example.test".into();
    let event = Event::from_value(value).unwrap();
    assert_eq!(
        event
            .recognized(Some(&External::default()))
            .unwrap()
            .fields()
            .as_value()
            .clone(),
        json!({"text":"external interpretation"})
    );
}

#[test]
fn typed_views_preserve_text_bits_family_additions_and_original_event() {
    use qso_graph_interop::model::{FamilyRef, FieldsRef, Kind};
    let mut value = command();
    value["family"] = "wsjtx".into();
    value["kind"] = "decode".into();
    value["native"] = json!({"format":"test","data":"AA==","addition":true});
    value["fields"] = json!({"text":{"utf8_bytes":[255]},"mode":null,"call":"",
        "dt_seconds":"ieee754:7ff80123456789ab","addition":true});
    value["wire_id"] = json!(null);
    let event = Event::from_value(value.clone()).unwrap();
    assert_eq!(event.family(), FamilyRef::Wsjtx);
    assert_eq!(event.kind_token(), "decode");
    assert_eq!(event.mapping(), 1);
    assert_eq!(event.wire_id(), Some(Text::Null));
    assert_eq!(event.native().unwrap().format(), "test");
    assert_eq!(event.native().unwrap().data().unwrap(), [0]);
    let recognized = event.recognized(None).unwrap();
    assert_eq!(recognized.kind(), Kind::Decode);
    assert!(std::ptr::eq(recognized.event(), &event));
    let FieldsRef::Decode(fields) = recognized.fields() else {
        panic!("decode view")
    };
    assert_eq!(
        fields.text(),
        Some(Text::Utf8 {
            utf8_bytes: vec![255]
        })
    );
    assert_eq!(fields.mode(), Some(Text::Null));
    assert_eq!(fields.call(), Some(Text::from("")));
    assert_eq!(fields.grid(), None);
    assert_eq!(
        fields.dt_seconds().unwrap().to_bits(),
        0x7ff8_0123_4567_89ab
    );
    assert!(fields.as_value().get("addition").is_none());
    assert_eq!(event.value(), &value);
    value["fields"]["dt_seconds"] = serde_json::from_str("-0.0").unwrap();
    let negative_zero = Event::from_value(value.clone()).unwrap();
    let recognized = negative_zero.recognized(None).unwrap();
    let FieldsRef::Decode(fields) = recognized.fields() else {
        panic!("decode view")
    };
    assert_eq!(fields.dt_seconds().unwrap().to_bits(), (-0.0_f64).to_bits());
    value["fields"]["dt_seconds"] = serde_json::from_str("0.0").unwrap();
    let positive_zero = Event::from_value(value).unwrap();
    assert_ne!(negative_zero, positive_zero);
    assert!(negative_zero.semantic_eq(&positive_zero));

    let mut value = command();
    value["family"] = "n1mm".into();
    value["kind"] = "qso_changed".into();
    value["native"] = json!({"format":"test","data":""});
    value["fields"] =
        json!({"call":null,"start":"2000-01-01T00:00:00Z","previous_start":"1999-12-31T23:59:59Z"});
    let event = Event::from_value(value).unwrap();
    assert_eq!(event.family(), FamilyRef::N1mm);
    let recognized = event.recognized(None).unwrap();
    let FieldsRef::QsoChanged(fields) = recognized.fields() else {
        panic!("QSO view")
    };
    assert_eq!(fields.call(), Some(Text::Null));
    assert_eq!(fields.start().unwrap().as_str(), "2000-01-01T00:00:00Z");
    assert_eq!(
        fields.previous_start().unwrap().as_str(),
        "1999-12-31T23:59:59Z"
    );
    let mut value = command();
    value["family"] = "fldigi".into();
    value["fields"] = json!({"operation":"fldigi.text_get_rx","rx_start":0,"rx_length":-1});
    let event = Event::from_value(value).unwrap();
    assert_eq!(event.family(), FamilyRef::Fldigi);
    let recognized = event.recognized(None).unwrap();
    let FieldsRef::Control(fields) = recognized.fields() else {
        panic!("control view")
    };
    assert_eq!(fields.operation(), "fldigi.text_get_rx");
    assert_eq!(fields.rx_start().unwrap().get(), 0);
    assert_eq!(fields.rx_length().unwrap().get(), -1);
}

#[test]
fn interpretation_and_opaque_evidence_constructors_reject_planted_invalid_values() {
    use qso_graph_interop::model::{Kind, NativeEvidence, ProfileView};
    for fields in [
        json!({"dt_seconds":"ieee754:3ff0000000000000"}),
        json!({"snr_db":0.5}),
        json!({"text":{"utf8_bytes":[256]}}),
    ] {
        assert!(
            ProfileView::new(
                Kind::Decode,
                fields.as_object().unwrap().clone(),
                Verification::Verified
            )
            .is_err()
        );
    }
    assert!(
        ProfileView::new(
            Kind::Control,
            json!({"operation":"read","rx_start":0.5})
                .as_object()
                .unwrap()
                .clone(),
            Verification::Unavailable
        )
        .is_err()
    );
    assert!(
        ProfileView::new(
            Kind::Result,
            json!({"status":"future"}).as_object().unwrap().clone(),
            Verification::Unavailable
        )
        .is_err()
    );
    for native in [
        json!({"format":"","data":""}),
        json!({"format":"test","data":"Zh=="}),
        json!({"format":"test","data":"","retained":false}),
        json!({"format":"mqtt_publish","data":""}),
        json!({"format":"ipfix_record","data":""}),
    ] {
        assert!(NativeEvidence::from_value(native).is_err());
    }
    let value = json!({"format":"mqtt_publish","data":"","topic":null,"retained":false,"future":{"nested":true}});
    let evidence = NativeEvidence::from_value(value.clone()).unwrap();
    assert_eq!(evidence.as_value(), &value);
    assert_eq!(evidence.into_value(), value);
    assert!(qso_graph_interop::codec::EncodedCommands::new(Vec::new()).is_err());
}

#[test]
fn value_construction_enforces_general_record_admission() {
    let mut value = command();
    value["future"] = Value::String("x".repeat(MAX_RECORD));
    assert_eq!(
        Event::from_value(value).unwrap_err().category(),
        Category::Oversized
    );
    let mut value = command();
    let mut nested = json!({});
    for _ in 0..33 {
        nested = json!([nested]);
    }
    value["future"] = nested;
    assert_eq!(
        Event::from_value(value).unwrap_err().category(),
        Category::Oversized
    );
    let mut value = command();
    value["future"] = Value::Number("1".repeat(129).parse().unwrap());
    assert_eq!(
        Event::from_value(value).unwrap_err().category(),
        Category::Oversized
    );
}
