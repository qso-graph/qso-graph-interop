// SPDX-License-Identifier: GPL-3.0-or-later
//! Reusable sans-I/O conformance checks for third-party families.
#![forbid(unsafe_code)]
use crate::{
    Codec, Error,
    codec::{DecodeOptions, EncodeOptions, Outcome},
    model::Event,
};
use serde_json::Value;
use sha2_const_stable::Sha256;
/// Check a manifest digest over the complete vector bytes, including its final newline.
pub fn check_hash(bytes: &[u8], expected: &str) -> bool {
    Sha256::new()
        .update(bytes)
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        == expected
}
/// Decode and compare the normative event independently of its optional structured view.
pub fn compare_event(actual: &Event, expected: &Value) -> bool {
    let mut a = actual.value().clone();
    let mut b = expected.clone();
    for v in [&mut a, &mut b] {
        if let Some(n) = v.get_mut("native").and_then(Value::as_object_mut) {
            n.remove("message");
        }
    }
    crate::json::semantic_eq(&a, &b)
}
/// Drive a native vector through an independently implemented family.
pub fn decode_vector(
    codec: &mut dyn Codec,
    bytes: &[u8],
    options: &DecodeOptions<'_>,
) -> Result<Outcome, Error> {
    codec.decode(bytes, options)
}
pub fn round_trip(
    codec: &mut dyn Codec,
    event: &Event,
    options: &DecodeOptions<'_>,
) -> Result<Outcome, Error> {
    let bytes = codec.reencode(event, &EncodeOptions::default())?;
    codec.decode(&bytes, options)
}
/// Exercise every proper prefix. Successful historical tails and skips are valid outcomes.
pub fn truncations(
    codec: &mut dyn Codec,
    bytes: &[u8],
    options: &DecodeOptions<'_>,
) -> Vec<Result<Outcome, Error>> {
    (0..bytes.len())
        .map(|n| {
            codec.reset(None);
            codec.decode(&bytes[..n], options)
        })
        .collect()
}
/// Fuzz harness body: arbitrary JSON and native input, bounded by declared input limits.
pub fn fuzz_one(codec: &mut dyn Codec, data: &[u8]) {
    let _ = crate::json::parse(data, crate::model::MAX_RECORD);
    let outcome = codec.decode(data, &DecodeOptions::default());
    if let Ok(out) = outcome {
        assert!(out.events.len() <= codec.limits().events);
        for event in out.events {
            assert!(crate::json::write(&event, false, None).is_ok());
        }
    }
}

/// Read instructions while retaining an event that cannot inhabit a Rust JSON value (for
/// example an unpaired surrogate) for the command-construction rejection path.
pub fn read_vector(bytes: &[u8]) -> Result<Value, String> {
    use serde_json::value::RawValue;
    use std::collections::BTreeMap;
    let mut raw: BTreeMap<String, Box<RawValue>> =
        serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut input: BTreeMap<String, Box<RawValue>> =
        serde_json::from_str(raw.remove("input").ok_or("missing input")?.get())
            .map_err(|e| e.to_string())?;
    let event = input.remove("event");
    let mut value: Value =
        serde_json::from_str(&serde_json::to_string(&raw).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    value["input"] =
        serde_json::from_str(&serde_json::to_string(&input).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if let Some(event) = event {
        match serde_json::from_str::<Value>(event.get()) {
            Ok(event) => value["input"]["event"] = event,
            Err(_) if value["class"] == "command_negative" => {
                value["input"]["event_json"] = event.get().into()
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(value)
}
/// Execute every defined vector class through an external codec. Unknown instructions fail closed.
/// The caller verifies manifest hashes and selects the transport/delimiter before calling.
pub fn run_vector(codec: &mut dyn Codec, vector: &Value) -> Result<(), String> {
    use crate::{base64, codec::SkipReason};
    let input = &vector["input"];
    let expected = &vector["expected"];
    let class = vector["class"].as_str().ok_or("missing vector class")?;
    let options = DecodeOptions {
        context: input
            .get("context")
            .cloned()
            .unwrap_or_else(|| DecodeOptions::default().context),
        structured: vector.get("expected_message").is_some(),
        ..DecodeOptions::default()
    };
    let encode_options = EncodeOptions {
        native: input
            .get("options")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    };
    let check_error = |result: Result<(), Error>| -> Result<(), String> {
        let err = result
            .err()
            .ok_or("expected an atomic error, received success")?;
        let category = serde_json::to_value(err.category).map_err(|e| e.to_string())?;
        if category != expected["error"]["category"] {
            return Err(format!(
                "expected {}, received {} ({err})",
                expected["error"]["category"], category
            ));
        }
        Ok(())
    };
    let read = |codec: &dyn Codec, e: &Event| -> Result<(), String> {
        let r = e.recognized(Some(codec)).map_err(|e| e.to_string())?;
        if let Some(wanted) = expected.get("read") {
            let mut actual =
                serde_json::json!({"kind":r.kind,"fields":r.fields,"context":r.context});
            if let Some(n) = r.native {
                actual["native"] = n;
            }
            actual
                .as_object_mut()
                .expect("recognized object")
                .retain(|k, _| wanted.get(k).is_some());
            if !crate::json::semantic_eq(&actual, wanted) {
                return Err(format!("recognized view differs: {actual}"));
            }
        }
        if let Some(wanted) = expected.get("verification") {
            let actual = match r.verification {
                crate::model::Verification::Verified => "verified",
                _ => "unavailable",
            };
            if wanted != actual {
                return Err(format!("expected verification {wanted}, received {actual}"));
            }
        }
        if let Some(original) = expected.get("preserved_command")
            && !crate::json::semantic_eq(e.value(), original)
        {
            return Err("original command was rewritten".into());
        }
        Ok(())
    };
    match class {
        "command" | "command_negative" => {
            let authored = if let Some(raw) = input.get("event_json").and_then(Value::as_str) {
                serde_json::from_str(raw)
                    .map_err(|_| crate::error::invalid("command cannot represent invalid Unicode"))
            } else {
                Ok(input["event"].clone())
            };
            let result = authored
                .and_then(Event::from_value)
                .and_then(|e| codec.encode_command(&e, &encode_options));
            if class == "command_negative" {
                check_error(result.map(|_| ()))
            } else {
                let actual = result.map_err(|e| e.to_string())?;
                let wanted =
                    base64::decode(expected["data"].as_str().ok_or("missing expected bytes")?)
                        .map_err(|e| e.to_string())?;
                if actual != wanted {
                    return Err("command bytes differ".into());
                }
                Ok(())
            }
        }
        "compatibility" => {
            let event = Event::from_value(input["event"].clone());
            if expected.get("error").is_some() {
                return check_error(event.and_then(|e| e.recognized(Some(codec)).map(|_| ())));
            }
            read(codec, &event.map_err(|e| e.to_string())?)
        }
        "negative" => {
            let result = if let Some(data) = input.get("data") {
                let bytes = base64::decode(data.as_str().ok_or("invalid input data")?)
                    .map_err(|e| e.to_string())?;
                codec.decode(&bytes, &options).map(|_| ())
            } else if let Some(json) = input.get("event_json") {
                crate::json::parse(
                    json.as_str().ok_or("invalid event_json")?.as_bytes(),
                    crate::model::MAX_RECORD,
                )
                .and_then(|e| e.recognized(Some(codec)).map(|_| ()))
            } else if let Some(event) = input.get("event") {
                Event::from_value(event.clone()).and_then(|e| e.recognized(Some(codec)).map(|_| ()))
            } else {
                return Err("unhandled negative vector instruction".into());
            };
            check_error(result)
        }
        "positive" | "skip" | "reencode" => {
            let bytes = base64::decode(input["data"].as_str().ok_or("missing native input")?)
                .map_err(|e| e.to_string())?;
            let out = codec.decode(&bytes, &options).map_err(|e| e.to_string())?;
            if class == "skip" {
                if !out.events.is_empty() || out.skips.len() != 1 {
                    return Err("expected one skip and no events".into());
                }
                let skip = &out.skips[0];
                let reason = match skip.reason {
                    SkipReason::FutureVersion => "future_version",
                    SkipReason::UnknownType => "unknown_type",
                    SkipReason::UnknownTemplate => "unknown_template",
                };
                let mut actual = serde_json::json!({"reason":reason});
                if let Some(v) = &skip.native_version {
                    actual["native_version"] = v.clone().into();
                }
                if let Some(t) = skip.native_type {
                    actual["type"] = t.into();
                }
                if let Some(t) = &skip.native_name {
                    actual["type"] = t.clone().into();
                }
                if !crate::json::semantic_eq(&actual, &expected["skip"]) {
                    return Err(format!("skip differs: {actual}"));
                }
                return Ok(());
            }
            if out.events.len() != 1 || !out.skips.is_empty() {
                return Err("expected exactly one event and no skips".into());
            }
            let event = &out.events[0];
            if !compare_event(event, &expected["event"]) {
                return Err("normative event differs".into());
            }
            if let Some(wanted) = vector.get("expected_message")
                && !crate::json::semantic_eq(&event.value()["native"]["message"], wanted)
            {
                return Err("structured view differs".into());
            }
            if expected.get("read").is_some() {
                read(codec, event)?;
            }
            if vector["family"] != "qgi" {
                if event.native_bytes().map_err(|e| e.to_string())? != bytes {
                    return Err("original-byte retrieval differs".into());
                }
                if codec.verify(event).map_err(|e| e.to_string())?
                    != crate::model::Verification::Verified
                {
                    return Err("supported native evidence was not verified".into());
                }
                crate::json::write(event, true, Some(codec)).map_err(|e| e.to_string())?;
            }
            if let Some(reencode) = vector.get("reencode") {
                let encoded = codec
                    .reencode(event, &EncodeOptions::default())
                    .map_err(|e| e.to_string())?;
                if let Some(wanted) = reencode.get("data") {
                    if encoded
                        != base64::decode(wanted.as_str().ok_or("invalid reencode data")?)
                            .map_err(|e| e.to_string())?
                    {
                        return Err("re-encoded bytes differ".into());
                    }
                } else if let Some(wanted) = reencode.get("event") {
                    let delimiter = reencode["delimiter"]
                        .as_str()
                        .ok_or("missing reencode delimiter")?
                        .as_bytes();
                    if !encoded.ends_with(delimiter) {
                        return Err("wrong TCP delimiter".into());
                    }
                    let forwarded = crate::json::parse(
                        &encoded[..encoded.len() - delimiter.len()],
                        crate::model::MAX_RECORD,
                    )
                    .map_err(|e| e.to_string())?;
                    if !compare_event(&forwarded, wanted) {
                        return Err("forwarded event differs".into());
                    }
                } else {
                    return Err("unhandled reencode instruction".into());
                }
            }
            Ok(())
        }
        _ => Err(format!("unhandled vector class {class}")),
    }
}

/// Exercise the declared message bound, returning an error if an over-limit message is admitted.
pub fn check_message_limit(codec: &mut dyn Codec) -> Result<(), String> {
    let limit = codec.limits().message_bytes;
    let bytes = vec![0; limit.checked_add(1).ok_or("message limit overflow")?];
    match codec.decode(&bytes, &DecodeOptions::default()) {
        Err(e) if e.category == crate::Category::Oversized => Ok(()),
        _ => Err("codec did not reject its message limit as oversized".into()),
    }
}
/// A dependency-macro invocation for an external cargo-fuzz target; supply your family constructor.
pub const FUZZ_TARGET_TEMPLATE: &str = r#"// SPDX-License-Identifier: GPL-3.0-or-later
#![no_main]
#![forbid(unsafe_code)]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let mut codec = your_family::Codec::default();
    qso_graph_interop::conformance::fuzz_one(&mut codec, data);
});
"#;
