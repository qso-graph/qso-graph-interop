// SPDX-License-Identifier: GPL-3.0-or-later
//! N1MM Logger+ UDP XML, Annex D mapping revision 1. Bounded and sans-I/O.
#![forbid(unsafe_code)]
mod xml;
use crate::{
    Category, Error, Event,
    codec::{
        Codec, DecodeOptions, EncodeOptions, Framing, Limits, Outcome, ParserLimits, Profile, Skip,
        SkipReason,
    },
    error::{invalid, oversized},
    model::{MAX_INTEGER, Verification},
};
use serde_json::{Map, Value, json};
use xml::{Document, Node};

/// Maximum complete XML UDP payload in bytes.
pub const MAX_DATAGRAM: usize = 65_507;
/// Stateless N1MM decoder and canonical field-built command encoder.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct N1mm;

/// N1MM failure retained as the source of the core error, without payload contents.
#[derive(Debug)]
#[non_exhaustive]
pub struct N1mmError {
    pub category: Category,
    message: String,
}
impl std::fmt::Display for N1mmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for N1mmError {}
impl N1mmError {
    pub fn category(&self) -> Category {
        self.category
    }
}
fn family_failure(error: Error) -> Error {
    use std::error::Error as _;
    if error.source().is_some_and(|e| e.is::<N1mmError>()) {
        return error;
    }
    Error::family(
        error.category(),
        N1mmError {
            category: error.category(),
            message: error.to_string(),
        },
    )
}

fn kind(n: &Node) -> Option<&'static str> {
    if !n.plain {
        return None;
    }
    Some(match n.name.as_str() {
        "AppInfo" | "lookupinfo" | "lookup" => "native",
        "contactinfo" => "qso_logged",
        "contactreplace" => "qso_changed",
        "contactdelete" => "qso_deleted",
        "RadioInfo" => "radio_state",
        "spot" => "spot",
        "dynamicresults" => "score",
        "Spectrum" | "radio_setfrequency" => "control",
        _ => return None,
    })
}
fn trimmed(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\r', '\n'])
}
fn integer(s: &str, factor: i64) -> Option<Value> {
    let s = trimmed(s);
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n = s.parse::<i64>().ok()?.checked_mul(factor)?;
    (-MAX_INTEGER..=MAX_INTEGER).contains(&n).then(|| n.into())
}
fn khz(s: &str) -> Option<Value> {
    let s = trimmed(s);
    let (negative, s) = if let Some(s) = s.strip_prefix('-') {
        (true, s)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    };
    let (whole, fraction) = s.split_once('.').map_or((s, None), |(w, f)| (w, Some(f)));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if fraction.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let fraction = fraction.unwrap_or("");
    if fraction
        .as_bytes()
        .get(3..)
        .is_some_and(|b| b.iter().any(|&b| b != b'0'))
    {
        return None;
    }
    let mut n = 0_i64;
    for b in whole
        .bytes()
        .chain(fraction.bytes().take(3))
        .chain(std::iter::repeat_n(
            b'0',
            3usize.saturating_sub(fraction.len()),
        ))
    {
        n = n.checked_mul(10)?.checked_add(i64::from(b - b'0'))?;
    }
    if negative {
        n = -n;
    }
    (-MAX_INTEGER..=MAX_INTEGER).contains(&n).then(|| n.into())
}
fn timestamp(s: &str) -> Option<Value> {
    if s.len() != 19
        || s.as_bytes().get(10) != Some(&b' ')
        || !s.is_ascii()
        || &s.as_bytes()[17..] > b"59"
    {
        return None;
    }
    let value = format!("{}T{}Z", &s[..10], &s[11..]);
    crate::json::valid_instant(&value).then(|| value.into())
}
fn scalar(v: &Value, typ: &str) -> Result<Option<Value>, Error> {
    if typ == "text" {
        return Ok(Some(v.clone()));
    }
    let Some(s) = v.as_str() else {
        return Ok(None);
    };
    if matches!(typ, "int" | "10hz" | "khz") && trimmed(s).chars().count() > 128 {
        return Err(oversized("N1MM numeric token"));
    }
    Ok(match typ {
        "int" => integer(s, 1),
        "10hz" => integer(s, 10),
        "khz" => khz(s),
        "time" => timestamp(s),
        "bool" => match trimmed(s) {
            "True" | "true" | "1" => Some(true.into()),
            "False" | "false" | "0" => Some(false.into()),
            _ => None,
        },
        _ => None,
    })
}
fn put(
    fields: &mut Map<String, Value>,
    node: &Node,
    wire: &str,
    common: &str,
    typ: &str,
) -> Result<(), Error> {
    if let Some(v) = node
        .leaf(wire)
        .map(|v| scalar(v, typ))
        .transpose()?
        .flatten()
    {
        fields.insert(common.into(), v);
    }
    Ok(())
}
fn fields(node: &Node) -> Result<Value, Error> {
    let mut f = Map::new();
    let mut radio = Map::new();
    let mut layout: Vec<(&str, &str, &str)> = Vec::new();
    match node.name.as_str() {
        "contactinfo" | "contactreplace" => {
            layout.extend([
                ("contestname", "contest_name", "text"),
                ("mycall", "station_call", "text"),
                ("band", "band", "text"),
                ("rxfreq", "dial_frequency_hz", "10hz"),
                ("txfreq", "tx_frequency_hz", "10hz"),
                ("operator", "operator_call", "text"),
                ("mode", "mode", "text"),
                ("call", "call", "text"),
                ("snt", "report_sent", "text"),
                ("sntnr", "serial_sent", "text"),
                ("rcv", "report_received", "text"),
                ("rcvnr", "serial_received", "text"),
                ("gridsquare", "grid", "text"),
                ("section", "section", "text"),
                ("comment", "comments", "text"),
                ("name", "name", "text"),
                ("prec", "precedence", "text"),
                ("ck", "check", "text"),
                ("ID", "record_id", "text"),
            ]);
            let alias = if node.named("exchange1").next().is_some() {
                "exchange1"
            } else {
                "exchangel"
            };
            layout.push((alias, "exchange_received", "text"));
            put(&mut radio, node, "StationName", "instance", "text")?;
            put(&mut radio, node, "radionr", "number", "int")?;
        }
        "contactdelete" => {
            layout.extend([
                ("mycall", "station_call", "text"),
                ("band", "band", "text"),
                ("call", "call", "text"),
                ("ID", "record_id", "text"),
            ]);
            put(&mut radio, node, "StationName", "instance", "text")?;
        }
        "RadioInfo" => {
            layout.extend([
                ("Freq", "dial_frequency_hz", "10hz"),
                ("Mode", "mode", "text"),
                ("mycall", "station_call", "text"),
                ("IsRunning", "running", "bool"),
                ("FocusRadioNr", "focus_radio", "int"),
                ("IsStereo", "stereo", "bool"),
                ("ActiveRadioNr", "active_radio", "int"),
                ("IsTransmitting", "transmitting", "bool"),
            ]);
            put(&mut radio, node, "StationName", "instance", "text")?;
            put(&mut radio, node, "RadioNr", "number", "int")?;
        }
        "spot" => {
            layout.extend([
                ("dxcall", "call", "text"),
                ("frequency", "frequency_hz", "khz"),
                ("spottercall", "spotter_call", "text"),
                ("timestamp", "time", "time"),
                ("mode", "mode", "text"),
                ("comment", "text", "text"),
            ]);
            put(&mut radio, node, "StationName", "instance", "text")?;
        }
        "dynamicresults" => {
            layout.extend([
                ("contest", "contest_name", "text"),
                ("call", "station_call", "text"),
                ("score", "total_score", "int"),
                ("timestamp", "time", "time"),
            ]);
            if let Some(b) = node.one("breakdown") {
                for (wire, common) in [("qso", "qso_count"), ("point", "points")] {
                    let mut selected = b.named(wire).filter(|n| {
                        n.attrs.get("band").is_some_and(|v| v == "total")
                            && n.attrs.get("mode").is_some_and(|v| v == "ALL")
                    });
                    if let Some(n) = selected.next()
                        && selected.next().is_none()
                        && n.children.is_empty()
                        && let Some(v) = scalar(&n.text, "int")?
                    {
                        f.insert(common.into(), v);
                    }
                }
            }
        }
        "radio_setfrequency" => {
            f.insert("operation".into(), "set_frequency".into());
            layout.extend([("app", "app", "text"), ("frequency", "frequency_hz", "khz")]);
            put(&mut radio, node, "radionr", "number", "int")?;
        }
        "Spectrum" => {
            f.insert("operation".into(), "n1mm.spectrum".into());
            layout.extend([
                ("app", "app", "text"),
                ("Name", "spectrum_name", "text"),
                ("LowScopeFrequency", "low_scope_frequency", "khz"),
                ("HighScopeFrequency", "high_scope_frequency", "khz"),
                ("ScalingFactor", "scaling_factor", "text"),
            ]);
        }
        _ => {}
    }
    if matches!(
        node.name.as_str(),
        "contactinfo" | "contactreplace" | "contactdelete" | "lookupinfo" | "lookup"
    ) {
        layout.push(("timestamp", "start", "time"));
    }
    if matches!(
        node.name.as_str(),
        "contactinfo" | "contactreplace" | "lookupinfo" | "lookup"
    ) {
        layout.push(("oldtimestamp", "previous_start", "time"));
    }
    for (wire, common, typ) in layout {
        put(&mut f, node, wire, common, typ)?;
    }
    if !radio.is_empty() {
        f.insert("radio".into(), radio.into());
    }
    Ok(f.into())
}
// Verification of a newer mapping checks only this revision's native assignments,
// including assignments whose leaves happen to be absent in this particular packet.
fn mapped_keys(root: &str) -> &'static [&'static str] {
    match root {
        "contactinfo" | "contactreplace" => &[
            "contest_name",
            "station_call",
            "band",
            "dial_frequency_hz",
            "tx_frequency_hz",
            "operator_call",
            "mode",
            "call",
            "report_sent",
            "serial_sent",
            "report_received",
            "serial_received",
            "grid",
            "section",
            "comments",
            "name",
            "precedence",
            "check",
            "record_id",
            "exchange_received",
            "radio",
            "start",
            "previous_start",
        ],
        "contactdelete" => &[
            "station_call",
            "band",
            "call",
            "record_id",
            "radio",
            "start",
        ],
        "RadioInfo" => &[
            "dial_frequency_hz",
            "mode",
            "station_call",
            "running",
            "focus_radio",
            "stereo",
            "active_radio",
            "transmitting",
            "radio",
        ],
        "spot" => &[
            "call",
            "frequency_hz",
            "spotter_call",
            "time",
            "mode",
            "text",
            "radio",
        ],
        "dynamicresults" => &[
            "contest_name",
            "station_call",
            "total_score",
            "time",
            "qso_count",
            "points",
        ],
        "radio_setfrequency" => &["operation", "app", "frequency_hz", "radio"],
        "Spectrum" => &[
            "operation",
            "app",
            "spectrum_name",
            "low_scope_frequency",
            "high_scope_frequency",
            "scaling_factor",
        ],
        "lookup" | "lookupinfo" => &["start", "previous_start"],
        _ => &[],
    }
}
fn mapped(
    doc: &Document,
    bytes: &[u8],
    options: &DecodeOptions<'_>,
) -> Result<Option<Event>, Error> {
    let Some(kind) = kind(&doc.root) else {
        return Ok(None);
    };
    let mut value = json!({"spec":crate::model::SPEC,"family":"n1mm","mapping":1,"kind":kind,"context":options.context,"fields":fields(&doc.root)?,"native":{"format":"n1mm_xml","data":crate::base64::encode(bytes)}});
    if options.structured {
        value["native"]["message"] = json!({"root":doc.root.name,"xml":doc.xml});
    }
    let event = Event::from_value(value)?;
    crate::json::write(
        &event,
        &crate::PublicationOptions::new(crate::PublicationMode::Preserved),
    )?;
    Ok(Some(event))
}
impl Profile for N1mm {
    fn recognize(
        &self,
        event: &Event,
        options: &crate::codec::VerifyOptions,
    ) -> Result<crate::model::ProfileView, crate::Error> {
        crate::json::compatible_view(
            event,
            self.native_version_supported(event),
            self.verify(event, options)?,
        )
    }

    fn family(&self) -> &str {
        "n1mm"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn native_version_supported(&self, event: &Event) -> bool {
        event.value().get("native_version").is_none()
    }
    fn verify(
        &self,
        event: &Event,
        _options: &crate::codec::VerifyOptions,
    ) -> Result<Verification, Error> {
        (|| {
            let v = event.value();
            if event.is_command()
                || v["family"] != "n1mm"
                || !crate::json::known_kind(v["kind"].as_str().unwrap_or(""))
                || !self.native_version_supported(event)
            {
                return Ok(Verification::Unavailable);
            }
            let bytes = event.native_bytes()?;
            let doc = xml::parse(&bytes)?;
            let options = DecodeOptions {
                context: v["context"].clone(),
                ..DecodeOptions::default()
            };
            let Some(actual) = mapped(&doc, &bytes, &options)? else {
                return Ok(Verification::Unavailable);
            };
            let mut supplied = v.clone();
            let mut expected = actual.into_value();
            let newer = v["mapping"] != 1 || v["spec"] != crate::model::SPEC;
            for value in [&mut supplied, &mut expected] {
                value.as_object_mut().expect("event").remove("via");
                value["native"]
                    .as_object_mut()
                    .expect("native")
                    .remove("message");
            }
            if newer {
                supplied["fields"] = crate::json::known_fields(v);
                supplied["fields"]
                    .as_object_mut()
                    .expect("fields")
                    .retain(|k, _| mapped_keys(&doc.root.name).contains(&k.as_str()));
                if let Some(radio) = supplied["fields"]
                    .get_mut("radio")
                    .and_then(Value::as_object_mut)
                {
                    radio.retain(|k, _| match doc.root.name.as_str() {
                        "spot" | "contactdelete" => k == "instance",
                        "radio_setfrequency" => k == "number",
                        _ => matches!(k.as_str(), "instance" | "number"),
                    });
                    if radio.is_empty() {
                        supplied["fields"]
                            .as_object_mut()
                            .expect("fields")
                            .remove("radio");
                    }
                }
                supplied["spec"] = expected["spec"].clone();
                supplied["mapping"] = expected["mapping"].clone();
                supplied
                    .as_object_mut()
                    .expect("event")
                    .retain(|k, _| expected.get(k).is_some());
                supplied["native"]
                    .as_object_mut()
                    .expect("native")
                    .retain(|k, _| matches!(k.as_str(), "format" | "data"));
            }
            if !crate::json::semantic_eq(&supplied, &expected) {
                return Err(invalid("event disagrees with N1MM native evidence"));
            }
            Ok(Verification::Verified)
        })()
        .map_err(family_failure)
    }
    fn validate_authored(
        &self,
        event: &Event,
        options: &crate::codec::VerifyOptions,
    ) -> Result<(), Error> {
        (|| {
            if event.value()["mapping"] != 1 {
                return Err(Error::new(
                    Category::UnsupportedVersion,
                    "unsupported N1MM mapping",
                ));
            }
            if event.is_command() {
                self.encode_command(event, &EncodeOptions::default())?;
            } else {
                if event.value().get("native_version").is_some()
                    || event.value().get("wire_id").is_some()
                {
                    return Err(invalid(
                        "N1MM observations carry no native version or wire ID",
                    ));
                }
                self.verify(event, options)?;
                if let Some(view) = event.value()["native"].get("message") {
                    let bytes = event.native_bytes()?;
                    let doc = xml::parse(&bytes)?;
                    if !crate::json::semantic_eq(view, &json!({"root":doc.root.name,"xml":doc.xml}))
                    {
                        return Err(invalid("invalid N1MM structured view"));
                    }
                }
            }
            Ok(())
        })()
        .map_err(family_failure)
    }
}
impl Codec for N1mm {
    fn framing(&self) -> Framing {
        Framing::Datagram
    }
    fn limits(&self) -> Limits {
        let mut limits = Limits::stateless(MAX_DATAGRAM, MAX_DATAGRAM);
        limits.parser = ParserLimits {
            depth: Some(16),
            elements: Some(1024),
            attributes: Some(256),
            numeric_token_bytes: Some(128),
            decoded_text_bytes: Some(MAX_DATAGRAM),
        };
        limits
    }
    fn reset(&mut self, _: Option<&[u8]>) {}
    fn decode(&mut self, bytes: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error> {
        (|| {
            let doc = xml::parse(bytes)?;
            if let Some(event) = mapped(&doc, bytes, options)? {
                Ok(Outcome::event(event))
            } else {
                let mut skip = Skip::new(SkipReason::UnknownType, None, None);
                skip.native_name = Some(doc.root.name);
                Ok(Outcome {
                    events: Vec::new(),
                    skips: vec![skip],
                })
            }
        })()
        .map_err(family_failure)
    }
    fn encode_commands(
        &self,
        event: &Event,
        options: &EncodeOptions,
    ) -> Result<crate::codec::EncodedCommands, crate::Error> {
        crate::codec::EncodedCommands::new(vec![crate::codec::EncodedRequest::new(
            self.encode_native_command(event, options)?,
            None,
        )])
    }
    fn reencode(
        &self,
        event: &Event,
        options: &EncodeOptions,
        verify: &crate::codec::VerifyOptions,
    ) -> Result<Vec<u8>, Error> {
        (|| {
            if !options.native.is_empty() {
                return Err(invalid("N1MM takes no encoding options"));
            }
            if self.verify(event, verify)? != Verification::Verified {
                return Err(invalid("N1MM evidence cannot be verified"));
            }
            event.native_bytes()
        })()
        .map_err(family_failure)
    }
}
fn hz(value: &Value) -> Result<i64, Error> {
    crate::json::integer_i64(value)
        .filter(|&n| (0..=MAX_INTEGER).contains(&n))
        .ok_or_else(|| invalid("N1MM frequency requires nonnegative integer Hz"))
}
fn decimal(n: i64) -> String {
    let whole = n / 1000;
    let fraction = n % 1000;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:03}")
            .trim_end_matches('0')
            .into()
    }
}
fn escaped(value: &Value) -> Result<String, Error> {
    let s = value
        .as_str()
        .ok_or_else(|| invalid("N1MM command requires XML text"))?;
    if s.len() > MAX_DATAGRAM {
        return Err(oversized("N1MM command text"));
    }
    let mut out = String::new();
    for c in s.chars() {
        if !xml::legal(c) {
            return Err(invalid("forbidden XML command character"));
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(c),
        }
        if out.len() > MAX_DATAGRAM {
            return Err(oversized("N1MM escaped command text"));
        }
    }
    Ok(out)
}
fn command(event: &Event, options: &EncodeOptions) -> Result<Vec<u8>, Error> {
    let v = event.value();
    if v["family"] != "n1mm" || !event.is_command() {
        return Err(invalid("expected N1MM command"));
    }
    if v["mapping"] != 1 || v["spec"] != crate::model::SPEC {
        return Err(Error::new(
            Category::UnsupportedVersion,
            "unsupported N1MM command revision",
        ));
    }
    if v.get("wire_id").is_some() || v.get("native_version").is_some() || !options.native.is_empty()
    {
        return Err(invalid(
            "N1MM command has no wire ID, native version, or options",
        ));
    }
    let f = &v["fields"];
    let op = f["operation"].as_str().unwrap_or("");
    let mut children = Vec::new();
    let (root, allowed): (&str, &[&str]) = match op {
        "set_frequency" => (
            "radio_setfrequency",
            &["operation", "app", "radio", "frequency_hz"],
        ),
        "n1mm.spectrum" => (
            "Spectrum",
            &[
                "operation",
                "app",
                "spectrum_name",
                "low_scope_frequency",
                "high_scope_frequency",
                "scaling_factor",
                "spectrum_data",
            ],
        ),
        _ => return Err(invalid("unsupported N1MM operation")),
    };
    if f.as_object()
        .expect("fields")
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
    {
        return Err(invalid("disallowed N1MM argument"));
    }
    children.push(("app", escaped(&f["app"])?));
    if op == "set_frequency" {
        let radio = f["radio"]
            .as_object()
            .filter(|r| r.len() == 1)
            .and_then(|r| r.get("number"))
            .and_then(crate::json::integer_i64)
            .filter(|&n| matches!(n, 1 | 2))
            .ok_or_else(|| invalid("N1MM radio must contain number 1 or 2"))?;
        children.extend([
            ("radionr", radio.to_string()),
            ("frequency", decimal(hz(&f["frequency_hz"])?)),
            (
                "mousebutton",
                if radio == 1 { "Left" } else { "Right" }.into(),
            ),
        ]);
    } else {
        let low = hz(&f["low_scope_frequency"])?;
        let high = hz(&f["high_scope_frequency"])?;
        if high <= low {
            return Err(invalid("N1MM scope frequencies must increase"));
        }
        let scale = f["scaling_factor"]
            .as_str()
            .ok_or_else(|| invalid("N1MM scale requires decimal text"))?;
        if scale.len() > 128 {
            return Err(oversized("N1MM scale token"));
        }
        let mut parts = scale.split('.');
        if !parts
            .by_ref()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            || scale.matches('.').count() > 1
            || !scale.bytes().any(|b| matches!(b, b'1'..=b'9'))
        {
            return Err(invalid("N1MM scale must be positive decimal text"));
        }
        let data = f["spectrum_data"]
            .as_array()
            .filter(|a| (1..=4096).contains(&a.len()))
            .ok_or_else(|| invalid("N1MM spectrum requires 1–4096 levels"))?;
        let mut levels = Vec::new();
        for level in data {
            levels.push(
                crate::json::integer_u64(level)
                    .filter(|&n| n <= 65535)
                    .ok_or_else(|| invalid("N1MM level out of range"))?
                    .to_string(),
            );
        }
        children.extend([
            ("Name", escaped(&f["spectrum_name"])?),
            ("LowScopeFrequency", decimal(low)),
            ("HighScopeFrequency", decimal(high)),
            ("ScalingFactor", scale.into()),
            ("DataCount", data.len().to_string()),
            ("SpectrumData", levels.join(",")),
        ]);
    }
    let mut out = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><{root}>");
    for (name, text) in children {
        if out.len() + 2 * name.len() + text.len() + 5 + root.len() + 3 > MAX_DATAGRAM {
            return Err(oversized("N1MM command datagram"));
        }
        out.push_str(&format!("<{name}>{text}</{name}>"));
    }
    out.push_str(&format!("</{root}>"));
    // The same bounded reader checks all result budgets before publication.
    xml::parse(out.as_bytes())?;
    Ok(out.into_bytes())
}

impl N1mm {
    fn encode_native_command(
        &self,
        event: &Event,
        options: &EncodeOptions,
    ) -> Result<Vec<u8>, Error> {
        command(event, options).map_err(family_failure)
    }
}
