// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline PSK Reporter MQTT message decoding, Annex G revision 1.
#![forbid(unsafe_code)]
use crate::{
    Category, Codec, DecodeOptions, EncodeOptions, Error, Event, Outcome, Profile, Verification,
    VerifyOptions,
    codec::{EncodedCommands, Framing, Limits},
    error::{invalid, oversized},
};
use serde_json::{Map, Value, json};

pub const MAX_PAYLOAD: usize = 16_384;
pub const MAX_TOPIC: usize = 512;
pub const MAX_COMPONENT: usize = 128;
pub const MAX_FIELD: usize = 4096;
pub const MAX_DEPTH: usize = 16;
pub const MAX_ELEMENTS: usize = 4096;
const MAX_INTEGER: i64 = 9_007_199_254_740_991;

/// The family's own failure, carried as the source of every core `Error` this module returns
/// (`QGI-RI-0024`); recover it with `source().downcast_ref::<MqttError>()`.
#[derive(Debug)]
#[non_exhaustive]
pub struct MqttError {
    pub category: Category,
    message: String,
}
impl std::fmt::Display for MqttError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for MqttError {}
impl MqttError {
    pub fn category(&self) -> Category {
        self.category
    }
}
fn family_failure(error: Error) -> Error {
    use std::error::Error as _;
    if error.source().is_some_and(|e| e.is::<MqttError>()) {
        return error;
    }
    Error::family(
        error.category(),
        MqttError {
            category: error.category(),
            message: error.to_string(),
        },
    )
}

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PskReporterMqtt {}

/// The topic is association evidence, supplied separately from the exact payload bytes.
/// Capture-format default: an omitted retain flag supplies false. QoS remains absent.
pub fn options(
    topic: &str,
    retain: Option<bool>,
    qos: Option<u8>,
) -> Result<DecodeOptions<'static>, Error> {
    let mut native = Map::new();
    native.insert("topic".into(), topic.into());
    native.insert("retained".into(), retain.unwrap_or(false).into());
    if let Some(qos) = qos {
        native.insert("qos".into(), qos.into());
    }
    Ok(DecodeOptions::default()
        .with_metadata(crate::codec::DecodeMetadata::extension_metadata(native)))
}

/// Decode one capture record. Additional capture-wrapper members are not wire payload.
pub fn decode_record(record: &Value) -> Result<Outcome, Error> {
    decode_record_value(record).map_err(family_failure)
}
fn decode_record_value(record: &Value) -> Result<Outcome, Error> {
    let record = record
        .as_object()
        .ok_or_else(|| invalid("MQTT capture record must be an object"))?;
    let topic = record
        .get("topic")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("MQTT capture topic must be text"))?;
    let data = record
        .get("payload_b64")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("MQTT capture payload_b64 must be text"))?;
    if data.len() > 4 * MAX_PAYLOAD.div_ceil(3) {
        return Err(oversized("MQTT payload base64 limit"));
    }
    let retain = record
        .get("retain")
        .map(|v| {
            v.as_bool()
                .ok_or_else(|| invalid("MQTT retain must be boolean"))
        })
        .transpose()?;
    let qos = record
        .get("qos")
        .map(|v| {
            v.as_u64()
                .filter(|n| *n <= 2)
                .map(|n| n as u8)
                .ok_or_else(|| invalid("MQTT QoS must be 0, 1 or 2"))
        })
        .transpose()?;
    PskReporterMqtt::default().decode(&crate::base64::decode(data)?, &options(topic, retain, qos)?)
}

fn bound_value(v: &Value, depth: usize, elements: &mut usize) -> Result<(), Error> {
    match v {
        Value::Object(m) => {
            if depth > MAX_DEPTH {
                return Err(oversized("MQTT JSON depth limit"));
            }
            *elements += m.len();
            for (k, v) in m {
                if k.len() > MAX_FIELD {
                    return Err(oversized("MQTT JSON field limit"));
                }
                bound_value(v, depth + 1, elements)?;
            }
        }
        Value::Array(a) => {
            if depth > MAX_DEPTH {
                return Err(oversized("MQTT JSON depth limit"));
            }
            *elements += a.len();
            for v in a {
                bound_value(v, depth + 1, elements)?;
            }
        }
        Value::String(s) if s.len() > MAX_FIELD => return Err(oversized("MQTT JSON field limit")),
        _ => {}
    }
    if *elements > MAX_ELEMENTS {
        return Err(oversized("MQTT JSON element limit"));
    }
    Ok(())
}
fn text(v: &Value) -> Result<(), Error> {
    if v.is_null() || v.is_string() {
        Ok(())
    } else {
        Err(invalid("MQTT field must be text or null"))
    }
}
fn number(v: &Value, min: i64, max: i64) -> Result<(), Error> {
    if v.is_null() || v.as_i64().is_some_and(|n| (min..=max).contains(&n)) {
        Ok(())
    } else {
        Err(invalid("MQTT integer type or range"))
    }
}
fn component(s: &str) -> Result<(), Error> {
    if s.len() > MAX_COMPONENT {
        return Err(oversized("MQTT topic component limit"));
    }
    if s.is_empty()
        || !s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_'))
    {
        return Err(invalid("MQTT topic component grammar"));
    }
    Ok(())
}
fn agree(
    payload: &Value,
    key: &str,
    topic: &str,
    locator: bool,
    numeric: bool,
) -> Result<(), Error> {
    let Some(v) = payload.get(key).filter(|v| !v.is_null()) else {
        return Ok(());
    };
    let same = if numeric {
        v.as_u64() == topic.parse::<u64>().ok()
    } else if locator {
        v.as_str().is_some_and(|v| {
            v.get(..4)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(topic))
        })
    } else if matches!(key, "sc" | "rc" | "call") {
        // Capture-observed escaping is one-way: a topic period means payload slash.
        // A literal payload period cannot agree, so ambiguous calls are rejected.
        v.as_str().is_some_and(|v| v == topic.replace('.', "/"))
    } else {
        v.as_str() == Some(topic)
    };
    if same {
        Ok(())
    } else {
        Err(invalid("MQTT topic/payload disagreement"))
    }
}
fn timestamp(seconds: u64) -> String {
    // Gregorian civil date from nonnegative Unix seconds (bounded through year 9999).
    let z = (seconds / 86400) as i64 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds % 86400 / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}
impl PskReporterMqtt {
    fn message(&self, bytes: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error> {
        if bytes.len() > MAX_PAYLOAD {
            return Err(oversized("MQTT payload limit"));
        }
        let metadata = options.metadata();
        let n = metadata.extensions();
        if n.keys()
            .any(|k| !["topic", "retained", "qos"].contains(&k.as_str()))
        {
            return Err(invalid("unknown MQTT association member"));
        }
        let topic = n
            .get("topic")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("MQTT topic association required"))?;
        if topic.len() > MAX_TOPIC {
            return Err(oversized("MQTT topic limit"));
        }
        let retained = n.get("retained").cloned().unwrap_or(Value::Bool(false));
        if !retained.is_boolean() {
            return Err(invalid("MQTT retained must be boolean"));
        }
        if let Some(qos) = n.get("qos") {
            number(qos, 0, 2)?;
            if qos.is_null() {
                return Err(invalid("MQTT QoS must be integer"));
            }
        }
        let parts: Vec<_> = topic.split('/').collect();
        for p in &parts {
            component(p)?;
        }
        if parts.len() == 11
            && parts[..2] == ["pskr", "filter"]
            && let Some(version) = parts[2]
                .strip_prefix('v')
                .filter(|s| !s.starts_with('0'))
                .and_then(|s| s.parse::<u32>().ok())
        {
            if version > 2 {
                return Ok(Outcome {
                    events: vec![],
                    skips: vec![crate::codec::Skip::new(
                        crate::codec::SkipReason::FutureVersion,
                        Some(version.to_string()),
                        None,
                    )],
                });
            }
            if version < 2 {
                return Err(Error::new(
                    crate::Category::UnsupportedVersion,
                    "unsupported MQTT topic version",
                ));
            }
        }
        let spot = parts.len() == 11 && parts[..3] == ["pskr", "filter", "v2"];
        let receiver = parts.len() == 3 && parts[..2] == ["pskr", "rxinfo"];
        if !spot && !receiver {
            return Err(invalid("MQTT topic grammar"));
        }
        if spot {
            for p in [&parts[7], &parts[8]] {
                if p.len() != 4
                    || !p.as_bytes()[..2].iter().all(|c| (b'A'..=b'R').contains(c))
                    || !p.as_bytes()[2..].iter().all(u8::is_ascii_digit)
                {
                    return Err(invalid("MQTT topic locator grammar"));
                }
            }
            for p in [&parts[9], &parts[10]] {
                if !p.bytes().all(|c| c.is_ascii_digit())
                    || (p.len() > 1 && p.starts_with('0'))
                    || p.parse::<u16>().is_err()
                {
                    return Err(invalid("MQTT topic DXCC grammar"));
                }
            }
        }
        let payload = crate::json::parse_value(bytes, MAX_PAYLOAD)?;
        let object = payload
            .as_object()
            .ok_or_else(|| invalid("MQTT payload must be an object"))?;
        bound_value(&payload, 1, &mut 0)?;
        let mut fields = Map::new();
        if spot {
            for k in ["sc", "sl", "rc", "rl", "md", "b"] {
                if let Some(v) = object.get(k) {
                    text(v)?;
                }
            }
            for (k, min, max) in [
                ("sq", 0, MAX_INTEGER),
                ("f", 0, MAX_INTEGER),
                ("rp", -128, 127),
                ("t", 0, 253402300799),
                ("t_tx", 0, 253402300799),
                ("sa", 0, 65535),
                ("ra", 0, 65535),
            ] {
                if let Some(v) = object.get(k) {
                    number(v, min, max)?;
                }
            }
            for (key, index, loc, num) in [
                ("b", 3, false, false),
                ("md", 4, false, false),
                ("sc", 5, false, false),
                ("rc", 6, false, false),
                ("sl", 7, true, false),
                ("rl", 8, true, false),
                ("sa", 9, false, true),
                ("ra", 10, false, true),
            ] {
                agree(&payload, key, parts[index], loc, num)?;
            }
            for (key, name) in [
                ("sc", "sender_call"),
                ("sl", "sender_locator"),
                ("rc", "receiver_call"),
                ("rl", "receiver_locator"),
                ("md", "mode"),
                ("b", "band"),
                ("f", "frequency_hz"),
                ("rp", "snr_db"),
                ("sa", "sender_dxcc"),
                ("ra", "receiver_dxcc"),
                ("t", "time"),
                ("t_tx", "transmission_start"),
            ] {
                if let Some(v) = object
                    .get(key)
                    .filter(|v| !v.is_null() || ["sc", "sl", "rc", "rl", "md", "b"].contains(&key))
                {
                    fields.insert(
                        name.into(),
                        if ["t", "t_tx"].contains(&key) && !v.is_null() {
                            timestamp(v.as_u64().expect("checked timestamp")).into()
                        } else {
                            v.clone()
                        },
                    );
                }
            }
        } else {
            for k in ["call", "antenna", "rig", "decoder", "grid"] {
                if let Some(v) = object.get(k) {
                    text(v)?;
                }
            }
            for k in ["time_start", "ts"] {
                if let Some(v) = object.get(k) {
                    number(v, -MAX_INTEGER, MAX_INTEGER)?;
                }
            }
            agree(&payload, "call", parts[2], false, false)?;
            fields.insert("receiver_call".into(), parts[2].replace('.', "/").into());
            // The supplied page does not define other rxinfo members. Keep them native.
        }
        let mut native = json!({"format":"mqtt_publish","topic":topic,"retained":retained,"data":crate::base64::encode(bytes)});
        if let Some(qos) = n.get("qos") {
            native["qos"] = qos.clone();
        }
        if options.structured() {
            native["message"] = payload;
        }
        let event = Event::from_value(
            json!({"spec":crate::model::SPEC,"family":"pskreporter-mqtt","mapping":1,"native_version":if spot {"2"} else {"rxinfo"},"kind":if spot {"spot"} else {"receiver_information"},"fields":fields,"context":options.context().as_value(),"native":native}),
        )?;
        crate::write(&event, &crate::PublicationOptions::default())?;
        Ok(Outcome::event(event))
    }
}
impl Profile for PskReporterMqtt {
    fn family(&self) -> &str {
        "pskreporter-mqtt"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn native_version_supported(&self, event: &Event) -> bool {
        matches!(
            event.value()["native_version"].as_str(),
            Some("2" | "rxinfo")
        )
    }
    fn recognize(
        &self,
        event: &Event,
        options: &VerifyOptions,
    ) -> Result<crate::model::ProfileView, Error> {
        crate::json::compatible_view(
            event,
            self.native_version_supported(event),
            self.verify(event, options)?,
        )
    }
    fn verify(&self, event: &Event, options: &VerifyOptions) -> Result<Verification, Error> {
        self.verify_evidence(event, options).map_err(family_failure)
    }
    fn validate_authored(&self, event: &Event, options: &VerifyOptions) -> Result<(), Error> {
        if self.verify(event, options)? == Verification::Verified {
            Ok(())
        } else {
            Err(family_failure(invalid(
                "MQTT authored evidence unavailable",
            )))
        }
    }
}
impl PskReporterMqtt {
    fn verify_evidence(&self, event: &Event, _: &VerifyOptions) -> Result<Verification, Error> {
        let v = event.value();
        if v["family"] != self.family()
            || !self.native_version_supported(event)
            || event.is_command()
        {
            return Ok(Verification::Unavailable);
        }
        let mut n = Map::new();
        for k in ["topic", "retained", "qos"] {
            if let Some(value) = v["native"].get(k) {
                n.insert(k.into(), value.clone());
            }
        }
        let options = DecodeOptions::default()
            .with_metadata(crate::codec::DecodeMetadata::extension_metadata(n))
            .with_structured(true);
        if v["native"]["data"]
            .as_str()
            .is_some_and(|s| s.len() > 4 * MAX_PAYLOAD.div_ceil(3))
        {
            return Err(oversized("MQTT evidence payload limit"));
        }
        let bytes = event.native_bytes()?;
        let decoded = self.message(&bytes, &options)?;
        let expected = decoded
            .events
            .first()
            .ok_or_else(|| invalid("MQTT evidence version unavailable"))?
            .value();
        for k in ["family", "kind", "native_version"] {
            if !crate::json::semantic_eq(&v[k], &expected[k]) {
                return Err(invalid("MQTT evidence/common mismatch"));
            }
        }
        if !crate::json::semantic_eq(&crate::json::known_fields(v), &expected["fields"]) {
            return Err(invalid("MQTT common fields mismatch"));
        }
        if v["native"]["format"] != "mqtt_publish" || v.get("wire_id").is_some() {
            return Err(invalid("MQTT envelope mismatch"));
        }
        if let Some(message) = v["native"].get("message")
            && !crate::json::semantic_eq(message, &expected["native"]["message"])
        {
            return Err(invalid("MQTT structured evidence mismatch"));
        }
        Ok(Verification::Verified)
    }
}
impl Codec for PskReporterMqtt {
    fn framing(&self) -> Framing {
        Framing::PublishSubscribe
    }
    fn limits(&self) -> Limits {
        let mut l = Limits::stateless(MAX_PAYLOAD, MAX_FIELD);
        l.parser.depth = Some(MAX_DEPTH);
        l.parser.elements = Some(MAX_ELEMENTS);
        l.parser.numeric_token_bytes = Some(128);
        l
    }
    fn decode(&mut self, bytes: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error> {
        self.message(bytes, options).map_err(family_failure)
    }
    fn encode_commands(&self, _: &Event, _: &EncodeOptions) -> Result<EncodedCommands, Error> {
        Err(family_failure(invalid(
            "MQTT capture decoding has no commands",
        )))
    }
    fn reencode(
        &self,
        event: &Event,
        encode: &EncodeOptions,
        verify: &VerifyOptions,
    ) -> Result<Vec<u8>, Error> {
        if !encode.extensions().is_empty() || self.verify(event, verify)? != Verification::Verified
        {
            return Err(family_failure(invalid(
                "MQTT reencode evidence unavailable",
            )));
        }
        event.native_bytes().map_err(family_failure)
    }
    fn reset(&mut self, _: Option<&[u8]>) {}
}
