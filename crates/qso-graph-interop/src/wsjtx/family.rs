// SPDX-License-Identifier: GPL-3.0-or-later
//! The Annex B codec, using only the public neutral core at its boundary.
#![forbid(unsafe_code)]
use super::{qt::Writer, *};
use crate::{
    Category,
    codec::{
        Codec, DecodeOptions, EncodeOptions, Framing, Limits, Outcome as Events, Profile, Skip,
        SkipReason as Reason,
    },
    error::invalid,
    model::{Event, Text, Verification},
};
use serde_json::{Map, Value, json};

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Wsjtx;
impl From<Error> for crate::Error {
    fn from(e: Error) -> Self {
        let category = match e {
            Error::Truncated { .. } => Category::Truncated,
            Error::Length => Category::Oversized,
            Error::Event { category, .. } => category,
            Error::WrongMagic | Error::OddQStringLength => Category::Malformed,
            Error::UnsupportedSchema(_) => Category::UnsupportedVersion,
            _ => Category::InvalidValue,
        };
        Self::family(category, e)
    }
}
fn text_value(v: &Utf8) -> Value {
    serde_json::to_value(Text::utf8(v.bytes.as_deref())).expect("text serialization")
}
fn clock(ms: u32) -> Option<String> {
    (ms < 86_400_000).then(|| {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            ms / 3_600_000,
            ms / 60_000 % 60,
            ms / 1000 % 60,
            ms % 1000
        )
    })
}
fn timestamp(ms: i64) -> Option<String> {
    let z = ms.div_euclid(86_400_000).checked_add(719_468)?;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    if !(1..=9999).contains(&year) {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{}Z",
        clock(ms.rem_euclid(86_400_000) as u32)?
    ))
}
fn datetime_view(t: &QDateTime) -> Value {
    let mut v = json!({"julian_day":t.julian_day.to_string(),"milliseconds":t.time.0,"time_spec":t.time_spec});
    if let Some(o) = t.offset_seconds {
        v["offset_seconds"] = o.into();
    }
    if let Some(z) = &t.time_zone {
        v["zone"] = match z {
            TimeZone::Iana(q) => json!({"key":Text::utf16(q.0.as_deref())}),
            TimeZone::Custom {
                id,
                offset_seconds,
                name,
                abbreviation,
                country,
                comment,
            } => {
                json!({"key":"OffsetFromUtc","id":Text::utf16(id.0.as_deref()),"offset_seconds":offset_seconds,"name":Text::utf16(name.0.as_deref()),"abbreviation":Text::utf16(abbreviation.0.as_deref()),"country":country,"comment":Text::utf16(comment.0.as_deref())})
            }
        };
    }
    v
}
fn metadata(n: u32, direction: &str) -> (&'static str, Option<(&'static str, &'static str)>) {
    match n {
        0 => ("lifecycle", Some(("state", "heartbeat"))),
        1 => ("radio_state", None),
        2 | 10 => ("decode", None),
        5 | 12 => ("qso_logged", None),
        3 => (
            match direction {
                "from_application" => "notification",
                "to_application" => "control",
                _ => "ambiguous",
            },
            Some(("operation", "clear")),
        ),
        6 => match direction {
            "from_application" => ("lifecycle", Some(("state", "closed"))),
            "to_application" => ("control", Some(("operation", "close"))),
            _ => ("ambiguous", Some(("operation", "close"))),
        },
        _ => ("control", Some(("operation", operation(n).unwrap_or("")))),
    }
}
fn operation(n: u32) -> Option<&'static str> {
    Some(match n {
        0 => "wsjtx.heartbeat",
        3 => "clear",
        4 => "reply",
        6 => "close",
        7 => "replay",
        8 => "halt_tx",
        9 => "send_text",
        11 => "wsjtx.location",
        13 => "wsjtx.highlight_callsign",
        14 => "wsjtx.switch_configuration",
        15 => "wsjtx.configure",
        16 => "wsjtx.annotation_info",
        _ => return None,
    })
}
fn mapped(
    packet: &Packet,
    bytes: &[u8],
    options: &DecodeOptions<'_>,
) -> Result<Event, crate::Error> {
    let n = packet.body.message_type();
    let body = serde_json::to_value(&packet.body).map_err(|_| invalid("invalid wire body"))?;
    let body = body
        .as_object()
        .and_then(|m| m.values().next())
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut fields = Map::new();
    let mut structured = Map::new();
    let (kind, tag) = metadata(
        n,
        options.context["direction"].as_str().unwrap_or("unknown"),
    );
    if let Some((k, v)) = tag {
        fields.insert(k.into(), v.into());
    }
    for &(wire, common, typ, _) in super::mapping::layout(n) {
        let Some(v) = body.get(wire) else {
            continue;
        };
        let view = match typ {
            "utf8" => text_value(
                &serde_json::from_value::<Utf8>(v.clone())
                    .map_err(|_| invalid("invalid UTF-8 wire value"))?,
            ),
            "QDateTime" => datetime_view(
                &serde_json::from_value::<QDateTime>(v.clone())
                    .map_err(|_| invalid("invalid date/time wire value"))?,
            ),
            "u64" | "i64" => Value::String(v.to_string()),
            _ => v.clone(),
        };
        structured.insert(wire.into(), view.clone());
        if common == "native only" {
            continue;
        }
        let value = match typ {
            "QTime" => v
                .as_u64()
                .and_then(|ms| clock(ms as u32))
                .map(Value::String),
            "QDateTime" => serde_json::from_value::<QDateTime>(v.clone())
                .ok()
                .and_then(|t| t.unix_milliseconds(packet.schema))
                .and_then(timestamp)
                .map(Value::String),
            "u64" => v
                .as_u64()
                .filter(|n| *n <= crate::model::MAX_INTEGER as u64)
                .map(Value::from),
            "u32"
                if n == 1
                    && matches!(wire, "frequency_tolerance_hz" | "tr_period_seconds")
                    && v.as_u64() == Some(u64::from(u32::MAX)) =>
            {
                None
            }
            _ => Some(view),
        };
        if let Some(v) = value {
            fields.insert(common.into(), v);
        }
    }
    if matches!(n, 1 | 2 | 5 | 10 | 12) {
        fields.insert("radio".into(), json!({"instance":text_value(&packet.id)}));
    }
    let mut v = json!({"spec":crate::model::SPEC,"family":"wsjtx","mapping":1,"kind":kind,"context":options.context,"native_version":packet.schema.to_string(),"wire_id":text_value(&packet.id),"fields":fields,"native":{"format":"wsjtx_datagram","data":crate::base64::encode(bytes)}});
    if options.structured {
        v["native"]["message"] = json!({"schema":packet.schema,"type":n,"id":text_value(&packet.id),"body":structured,"extensions":crate::base64::encode(&packet.extensions)});
    }
    let event = Event::from_value(v)?;
    crate::json::write(&event, false, None)?;
    Ok(event)
}
impl Profile for Wsjtx {
    fn family(&self) -> &str {
        "wsjtx"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn native_version_supported(&self, event: &Event) -> bool {
        event.is_command()
            || matches!(
                event.value()["native_version"].as_str(),
                Some("1" | "2" | "3")
            )
    }
    fn fields(&self, event: &Event) -> Value {
        crate::json::known_fields(event.value())
    }
    fn verify(&self, event: &Event) -> Result<Verification, crate::Error> {
        (|| {
            let v = event.value();
            if v["family"] != "wsjtx" || event.is_command() {
                return Ok(Verification::Unavailable);
            }
            if !crate::json::known_kind(v["kind"].as_str().unwrap_or("")) {
                return Ok(Verification::Unavailable);
            }
            let bytes = event.native_bytes()?;
            let packet = match super::decode(&bytes)? {
                Outcome::Skipped { .. } => return Ok(Verification::Unavailable),
                Outcome::Message(p) => p,
            };
            if v["native_version"]
                .as_str()
                .and_then(|s| s.parse::<u32>().ok())
                != Some(packet.schema)
            {
                return Err(invalid("native schema disagrees with event"));
            }
            let options = DecodeOptions {
                context: v["context"].clone(),
                ..DecodeOptions::default()
            };
            let actual = mapped(&packet, &bytes, &options)?;
            let newer = crate::json::integer_u64(&v["mapping"]).is_some_and(|n| n > 1)
                || v["spec"] != crate::model::SPEC;
            let mut expected = actual.into_value();
            let mut supplied = v.clone();
            for value in [&mut supplied, &mut expected] {
                if let Some(n) = value.get_mut("native").and_then(Value::as_object_mut) {
                    n.remove("message");
                }
                if let Some(m) = value.as_object_mut() {
                    m.remove("via");
                }
            }
            if newer {
                supplied["fields"] = crate::json::known_fields(v);
                let layout = super::mapping::layout(packet.body.message_type());
                supplied["fields"]
                    .as_object_mut()
                    .expect("common fields")
                    .retain(|key, _| {
                        matches!(key.as_str(), "operation" | "state" | "radio")
                            || layout.iter().any(|(_, common, _, _)| *common == key)
                    });
                if let Some(radio) = supplied["fields"]
                    .get_mut("radio")
                    .and_then(Value::as_object_mut)
                {
                    radio.retain(|key, _| key == "instance");
                }
                supplied["native"]
                    .as_object_mut()
                    .expect("native evidence")
                    .retain(|key, _| matches!(key.as_str(), "format" | "data"));
                supplied["spec"] = expected["spec"].clone();
                supplied["mapping"] = expected["mapping"].clone();
                // Ignore additive envelope and context members in the recognized consistency comparison.
                supplied
                    .as_object_mut()
                    .expect("event object")
                    .retain(|k, _| expected.get(k).is_some());
                supplied["context"] = expected["context"].clone();
            }
            if !crate::json::semantic_eq(&supplied, &expected) {
                return Err(invalid("event disagrees with WSJT-X native evidence"));
            }
            Ok(Verification::Verified)
        })()
        .map_err(family_failure)
    }
    fn validate_authored(&self, event: &Event) -> Result<(), crate::Error> {
        (|| {
            if crate::json::integer_u64(&event.value()["mapping"]) != Some(1) {
                return Err(crate::Error::new(
                    Category::UnsupportedVersion,
                    "unsupported WSJT-X mapping revision",
                ));
            }
            if !event.is_command() {
                self.verify(event)?;
            }
            if event.value()["native"].get("message").is_some() {
                let bytes = event.native_bytes()?;
                let Outcome::Message(packet) = super::decode(&bytes)? else {
                    return Err(invalid("structured view has no supported native message"));
                };
                let options = DecodeOptions {
                    context: event.value()["context"].clone(),
                    structured: true,
                    ..DecodeOptions::default()
                };
                let expected = mapped(&packet, &bytes, &options)?;
                if !crate::json::semantic_eq(
                    &event.value()["native"]["message"],
                    &expected.value()["native"]["message"],
                ) {
                    return Err(invalid(
                        "structured view differs from its declared catalog or native evidence",
                    ));
                }
            }
            Ok(())
        })()
        .map_err(family_failure)
    }
}
impl Codec for Wsjtx {
    fn framing(&self) -> Framing {
        Framing::Datagram
    }
    fn limits(&self) -> Limits {
        Limits::stateless(MAX_DATAGRAM, MAX_DATAGRAM)
    }
    fn reset(&mut self, _: Option<&[u8]>) {}
    fn decode(
        &mut self,
        input: &[u8],
        options: &DecodeOptions<'_>,
    ) -> Result<Events, crate::Error> {
        (|| match super::decode(input)? {
            Outcome::Message(p) => Ok(Events::event(mapped(&p, input, options)?)),
            Outcome::Skipped {
                schema,
                message_type,
                reason,
            } => Ok(Events {
                events: Vec::new(),
                skips: vec![Skip::new(
                    match reason {
                        SkipReason::FutureSchema => Reason::FutureVersion,
                        SkipReason::UnknownType => Reason::UnknownType,
                    },
                    Some(schema.to_string()),
                    message_type.map(u64::from),
                )],
            }),
        })()
        .map_err(family_failure)
    }
    fn encode_command(
        &self,
        event: &Event,
        options: &EncodeOptions,
    ) -> Result<Vec<u8>, crate::Error> {
        (|| {
            let v = event.value();
            if v["family"] != "wsjtx" || !event.is_command() {
                return Err(invalid("expected a WSJT-X command"));
            }
            if crate::json::integer_u64(&v["mapping"]) != Some(1) || v["spec"] != crate::model::SPEC
            {
                return Err(crate::Error::new(
                    Category::UnsupportedVersion,
                    "unsupported WSJT-X command mapping or specification",
                ));
            }
            let schema = match options.native.get("schema") {
                None => 3,
                Some(n) => crate::json::integer_u64(n)
                    .filter(|n| (1..=3).contains(n))
                    .ok_or_else(|| {
                        crate::Error::new(
                            Category::UnsupportedVersion,
                            "WSJT-X schema must be an integer in 1–3",
                        )
                    })? as u32,
            };
            let op = v["fields"]["operation"]
                .as_str()
                .ok_or_else(|| invalid("missing command operation"))?;
            let n = (0..=16)
                .find(|&n| operation(n) == Some(op))
                .ok_or_else(|| invalid("unsupported WSJT-X operation"))?;
            let id = utf8(
                v.get("wire_id")
                    .ok_or_else(|| invalid("missing command wire_id"))?,
                MAX_ID,
            )?;
            if id.bytes.as_ref().is_some_and(|b| b.len() > MAX_ID) {
                return Err(Error::Length.into());
            }
            let mut w = Writer::new(schema);
            w.u32(MAGIC)?;
            w.u32(schema)?;
            w.u32(n)?;
            w.utf8(&id)?;
            let mut omitted = false;
            let allowed = super::mapping::layout(n);
            if v["fields"].as_object().is_some_and(|m| {
                m.keys()
                    .any(|k| k != "operation" && !allowed.iter().any(|(_, c, _, _)| *c == k))
            }) {
                return Err(invalid("unsupported command argument combination"));
            }
            for &(_, common, typ, optional) in allowed {
                let Some(value) = v["fields"].get(common) else {
                    if optional {
                        omitted = true;
                        continue;
                    }
                    return Err(invalid("missing required command argument"));
                };
                if omitted {
                    return Err(invalid("command optional tail has a hole"));
                }
                match typ {
                    "utf8" => w.utf8(&utf8(value, MAX_DATAGRAM)?)?,
                    "bool" => w.boolean(
                        value
                            .as_bool()
                            .ok_or_else(|| invalid("command requires a boolean"))?,
                    )?,
                    "u8" => {
                        let b = u8::try_from(unsigned(value)?)
                            .map_err(|_| invalid("command u8 out of range"))?;
                        if n == 3 && b > 2 {
                            return Err(invalid("Clear window must be 0, 1 or 2"));
                        }
                        w.u8(b)?;
                    }
                    "u32" => w.u32(
                        u32::try_from(unsigned(value)?)
                            .map_err(|_| invalid("command u32 out of range"))?,
                    )?,
                    "i32" => w.i32(
                        i32::try_from(
                            crate::json::integer_i64(value)
                                .ok_or_else(|| invalid("command requires an integer"))?,
                        )
                        .map_err(|_| invalid("command i32 out of range"))?,
                    )?,
                    "QTime" => w.u32(milliseconds(value)?)?,
                    "QColor" => {
                        let spec = u8::try_from(unsigned(&value["spec"])?)
                            .map_err(|_| invalid("invalid color spec"))?;
                        let channels = value["channels"]
                            .as_array()
                            .filter(|a| a.len() == 5)
                            .ok_or_else(|| invalid("invalid color channels"))?;
                        let mut color = QColor {
                            spec,
                            ..QColor::default()
                        };
                        for (target, v) in color.channels.iter_mut().zip(channels) {
                            *target = u16::try_from(unsigned(v)?)
                                .map_err(|_| invalid("invalid color channel"))?;
                        }
                        w.qcolor(&color)?;
                    }
                    "f64" => w.f64(binary64(value)?)?,
                    _ => return Err(invalid("unsupported command wire field")),
                }
            }
            Ok(w.data)
        })()
        .map_err(family_failure)
    }
    fn reencode(&self, event: &Event, _: &EncodeOptions) -> Result<Vec<u8>, crate::Error> {
        (|| {
            if self.verify(event)? != Verification::Verified {
                return Err(invalid("native evidence cannot be verified"));
            }
            let Outcome::Message(p) = super::decode(&event.native_bytes()?)? else {
                return Err(invalid("native evidence is unavailable"));
            };
            super::encode(&p).map_err(Into::into)
        })()
        .map_err(family_failure)
    }
}
fn unsigned(v: &Value) -> Result<u64, crate::Error> {
    crate::json::integer_u64(v).ok_or_else(|| invalid("command requires an unsigned integer"))
}
fn utf8(v: &Value, limit: usize) -> Result<Utf8, crate::Error> {
    if v.is_null() {
        return Ok(Utf8::default());
    }
    if let Some(s) = v.as_str() {
        if s.len() > limit {
            return Err(Error::Length.into());
        }
        return Ok(s.into());
    }
    let b = v
        .get("utf8_bytes")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("command requires UTF-8 text"))?;
    if b.len() > limit {
        return Err(Error::Length.into());
    }
    let bytes: Vec<u8> = b
        .iter()
        .map(|v| {
            unsigned(v)
                .and_then(|n| u8::try_from(n).map_err(|_| invalid("UTF-8 byte out of range")))
        })
        .collect::<Result<_, _>>()?;
    if std::str::from_utf8(&bytes).is_ok() {
        return Err(invalid("valid UTF-8 must use a string"));
    }
    Ok(Utf8 { bytes: Some(bytes) })
}
fn binary64(v: &Value) -> Result<f64, crate::Error> {
    if let Some(n) = v.as_f64().filter(|f| f.is_finite()) {
        return Ok(n);
    }
    let s = v
        .as_str()
        .and_then(|s| s.strip_prefix("ieee754:"))
        .filter(|s| {
            s.len() == 16
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        })
        .ok_or_else(|| invalid("invalid binary64 argument"))?;
    let f =
        f64::from_bits(u64::from_str_radix(s, 16).map_err(|_| invalid("invalid binary64 bits"))?);
    if f.is_finite() {
        return Err(invalid("finite binary64 must be a number"));
    }
    Ok(f)
}
fn milliseconds(v: &Value) -> Result<u32, crate::Error> {
    let s = v
        .as_str()
        .filter(|s| crate::json::valid_clock(s))
        .ok_or_else(|| invalid("invalid reply time"))?;
    let h: u32 = s[0..2].parse().map_err(|_| invalid("invalid hour"))?;
    let m: u32 = s[3..5].parse().map_err(|_| invalid("invalid minute"))?;
    let sec: u32 = s[6..8].parse().map_err(|_| invalid("invalid second"))?;
    let frac = s.get(9..).unwrap_or("");
    if sec >= 60 || frac.get(3..).is_some_and(|s| s.bytes().any(|b| b != b'0')) {
        return Err(invalid("reply time is not representable as milliseconds"));
    }
    let mut f = frac.chars().take(3).collect::<String>();
    while f.len() < 3 {
        f.push('0');
    }
    Ok(h * 3_600_000
        + m * 60_000
        + sec * 1000
        + f.parse::<u32>().map_err(|_| invalid("invalid fraction"))?)
}

fn family_failure(e: crate::Error) -> crate::Error {
    use std::error::Error as _;
    if e.source().is_some_and(|s| s.is::<Error>()) {
        e
    } else {
        Error::Event {
            category: e.category,
            message: e.to_string(),
        }
        .into()
    }
}
