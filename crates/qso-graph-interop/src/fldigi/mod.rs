// SPDX-License-Identifier: GPL-3.0-or-later
//! fldigi 4.2.13 XML-RPC bodies, Annex E revision 1. No HTTP or session state.
#![forbid(unsafe_code)]
mod catalog;
mod command;
mod rpc;
use crate::{
    Category, Error, Event, Text,
    codec::{Codec, DecodeOptions, EncodeOptions, Framing, Limits, Outcome, ParserLimits, Profile},
    error::{invalid, oversized},
    model::{MAX_INTEGER, Verification},
    xml::{self, Document},
};
use rpc::Scalar;
use serde_json::{Value, json};
/// Maximum XML-RPC body, before parsing or allocating XML structures.
pub const MAX_BODY: usize = 262_144;
/// Stateless codec. An optional association is caller-owned verification metadata.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Fldigi;
/// fldigi failure retained as a typed source without copying input payloads.
#[derive(Debug)]
#[non_exhaustive]
pub struct FldigiError {
    pub category: Category,
    message: String,
}
impl std::fmt::Display for FldigiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for FldigiError {}
impl FldigiError {
    pub fn category(&self) -> Category {
        self.category
    }
}
fn family_failure(error: Error) -> Error {
    use std::error::Error as _;
    if error.source().is_some_and(|e| e.is::<FldigiError>()) {
        return error;
    }
    Error::family(
        error.category(),
        FldigiError {
            category: error.category(),
            message: error.to_string(),
        },
    )
}
/// Validated association for a single XML-RPC request. It implies no execution.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RequestAssociation {
    value: Value,
}
impl RequestAssociation {
    pub fn for_method(method: &str) -> Result<Self, Error> {
        Self::from_value(json!({"method":method}))
    }
    pub fn from_value(value: Value) -> Result<Self, Error> {
        association(Some(&value))?;
        Ok(Self { value })
    }
    pub fn from_request_body(body: &[u8]) -> Result<Self, Error> {
        let doc = parse(body)?;
        let message = rpc::parse(&doc.root)?;
        let method = message
            .method
            .ok_or_else(|| invalid("association requires a methodCall"))?;
        let method = rpc::method(&method.text)?;
        let mut arguments = message
            .values
            .iter()
            .map(|node| match rpc::scalar(node)? {
                Scalar::Int(n) => Ok(Value::from(n)),
                Scalar::String(v) => Ok(v),
                Scalar::Double(n) => Ok(Value::from(n)),
                _ => Ok(Value::Null),
            })
            .collect::<Result<Vec<_>, Error>>()?;
        // API-2: a double spelled as an integer cannot establish an XML-RPC int RX range.
        if method == "text.get_rx"
            && (message.values.len() != 2
                || message.values.iter().any(|node| rpc::typ(node) != "int"))
        {
            arguments = vec![Value::Null, Value::Null];
        }
        Self::from_value(json!({"method":method,"arguments":arguments}))
    }
    pub fn as_value(&self) -> &Value {
        &self.value
    }
}
impl From<RequestAssociation> for crate::codec::DecodeMetadata {
    fn from(request: RequestAssociation) -> Self {
        let mut native = serde_json::Map::new();
        native.insert("request".into(), request.value);
        Self::extension_metadata(native)
    }
}
fn parse(bytes: &[u8]) -> Result<Document, Error> {
    xml::parse_with(
        bytes,
        xml::XmlLimits {
            bytes: MAX_BODY,
            elements: 8192,
            declaration_eof_malformed: true,
        },
    )
}
pub(super) fn operation(method: &str) -> String {
    match method {
        "rig.set_frequency" | "main.set_frequency" => "set_frequency".into(),
        "text.add_tx" => "send_text".into(),
        "main.abort" => "halt_tx".into(),
        _ => format!(
            "fldigi.{}",
            method
                .strip_prefix("fldigi.")
                .unwrap_or(method)
                .replace('.', "_")
        ),
    }
}
fn association(v: Option<&Value>) -> Result<Option<&str>, Error> {
    let Some(v) = v else {
        return Ok(None);
    };
    let o = v
        .as_object()
        .ok_or_else(|| invalid("request association must be an object"))?;
    if o.keys()
        .any(|k| !matches!(k.as_str(), "method" | "arguments"))
    {
        return Err(invalid("unknown request association member"));
    }
    let method = rpc::method(&v["method"])?;
    if let Some(a) = v.get("arguments") {
        let a = a
            .as_array()
            .ok_or_else(|| invalid("request arguments must be an array"))?;
        if a.len() > 32 {
            return Err(oversized("request argument count"));
        }
    }
    Ok(Some(method))
}
fn range(request: Option<&Value>) -> bool {
    request
        .and_then(|v| v.get("arguments"))
        .and_then(Value::as_array)
        .is_some_and(|a| {
            a.len() == 2
                && a.iter()
                    .all(|v| crate::json::integer_i64(v).is_some_and(|n| i32::try_from(n).is_ok()))
                && crate::json::integer_i64(&a[0]).is_some_and(|n| n >= 0)
                && crate::json::integer_i64(&a[1]).is_some_and(|n| n >= -1)
        })
}
fn hz(f: f64) -> Option<Value> {
    (f.fract() == 0.0 && f >= -(MAX_INTEGER as f64) && f <= MAX_INTEGER as f64)
        .then(|| (f as i64).into())
}
fn mapped_scalar(s: Scalar) -> Option<Value> {
    match s {
        Scalar::String(v) => Some(v),
        Scalar::Int(n) => Some(n.into()),
        Scalar::Double(f) => hz(f),
        Scalar::Bytes(b) => Some(Text::utf8(Some(&b)).value()),
        _ => None,
    }
}
fn budget_context(options: &DecodeOptions<'_>) -> Result<(), Error> {
    let mut contexts = vec![&options.context];
    if let Some(hops) = options.via.as_ref().and_then(Value::as_array) {
        contexts.extend(hops.iter().map(|h| &h["context"]));
    }
    for c in contexts {
        if serde_json::to_vec(c)
            .map_err(|_| invalid("invalid context"))?
            .len()
            > 4096
        {
            return Err(oversized("context exceeds 4096 bytes"));
        }
    }
    if options
        .via
        .as_ref()
        .and_then(Value::as_array)
        .is_some_and(|h| h.len() > 8)
    {
        return Err(oversized("more than eight via hops"));
    }
    Ok(())
}
fn observe(
    bytes: &[u8],
    options: &DecodeOptions<'_>,
    request: Option<&Value>,
) -> Result<(Event, bool), Error> {
    let doc = parse(bytes)?;
    let msg = rpc::parse(&doc.root)?;
    let call = msg.method.map(|n| rpc::method(&n.text)).transpose()?;
    let associated = association(request)?;
    if let (Some(a), Some(b)) = (call, associated)
        && a != b
    {
        return Err(invalid("request association disagrees with methodCall"));
    }
    let method = call.or(associated);
    budget_context(options)?;
    let mut v = json!({"spec":crate::model::SPEC,"family":"fldigi","mapping":1,"kind":"native","context":options.context,"fields":{},"native":{"format":"fldigi_xmlrpc","data":""}});
    if let Some(via) = options.via.as_ref() {
        v["via"] = via.clone();
    }
    Event::from_value(v.clone())?;
    let mut verifiable = !msg.opaque;
    if msg.fault {
        v["kind"] = "result".into();
        v["fields"] = json!({"status":"error"});
        for member in &msg.values[0].children[0].children {
            let name = &member.children[0].text;
            let val = rpc::content(&member.children[1]);
            if name == "faultCode" {
                v["fields"]["native_code"] = val.clone();
            }
            if name == "faultString" {
                v["fields"]["text"] = val.clone();
            }
        }
        if let Some(m) = associated {
            let op = operation(m);
            // Faults may name methods that do not yield a legal common operation token.
            if op.split_once('.').is_some_and(|(family, token)| {
                family == "fldigi"
                    && token.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                    && token
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            }) || matches!(op.as_str(), "set_frequency" | "send_text" | "halt_tx")
            {
                v["fields"]["operation"] = op.into();
            }
        }
        verifiable = true;
    } else if !msg.opaque {
        if let Some(m) = method.and_then(catalog::find) {
            let matches = if call.is_some() {
                msg.values.len() == m.args.len()
                    && msg
                        .values
                        .iter()
                        .zip(m.args)
                        .all(|(n, (_, t))| rpc::typ(n) == *t)
            } else {
                msg.values.len() == 1 && rpc::typ(msg.values[0]) == m.returns
            };
            if call.is_none()
                && (associated.is_none() || (m.name == "text.get_rx" && !range(request)))
            {
                verifiable = false;
            } else if matches {
                let f = v["fields"].as_object_mut().expect("fields");
                if call.is_some() {
                    f.insert("operation".into(), operation(m.name).into());
                    for (node, (name, _)) in msg.values.iter().zip(m.args) {
                        if let Some(value) = mapped_scalar(rpc::scalar(node)?) {
                            f.insert((*name).into(), value);
                        }
                    }
                    v["kind"] = "control".into();
                } else {
                    if m.kind == "result" {
                        f.insert("status".into(), "ok".into());
                        f.insert("operation".into(), operation(m.name).into());
                    }
                    let scalar = rpc::scalar(msg.values[0])?;
                    let value = if m.field == "transmitting" {
                        match rpc::content(msg.values[0]).as_str() {
                            Some("TX") if m.name == "main.get_trx_state" => Some(true.into()),
                            Some("RX") if m.name == "main.get_trx_state" => Some(false.into()),
                            Some("tx" | "tune") if m.name == "main.get_trx_status" => {
                                Some(true.into())
                            }
                            Some("rx") if m.name == "main.get_trx_status" => Some(false.into()),
                            _ => None,
                        }
                    } else {
                        mapped_scalar(scalar)
                    };
                    if m.field == "native_version" {
                        if value
                            .as_ref()
                            .and_then(Value::as_str)
                            .is_some_and(|s| !s.is_empty())
                        {
                            v["native_version"] = value.expect("version");
                        } else if let Some(value) = value {
                            v["fields"]["text"] = value;
                        }
                    } else if !m.field.is_empty()
                        && let Some(value) = value
                    {
                        v["fields"][m.field] = value;
                    }
                    v["kind"] = m.kind.into();
                }
            }
        } else if call.is_none() && associated.is_none() {
            verifiable = false;
        }
    }
    v["native"]["data"] = crate::base64::encode(bytes).into();
    if options.structured {
        let mut view = json!({"root":doc.root.name,"xml":doc.xml});
        if let Some(method) = method {
            view["method"] = method.into();
        }
        v["native"]["message"] = view;
    }
    let event = Event::from_value(v)?;
    crate::json::write(
        &event,
        &crate::PublicationOptions::new(crate::PublicationMode::Preserved),
    )?;
    Ok((event, verifiable))
}
impl Profile for Fldigi {
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

    fn native_version_supported(&self, _: &Event) -> bool {
        true
    }
    fn family(&self) -> &str {
        "fldigi"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn verify(
        &self,
        event: &Event,
        verify: &crate::codec::VerifyOptions,
    ) -> Result<Verification, Error> {
        (|| {
            let v = event.value();
            if event.is_command()
                || v["family"] != "fldigi"
                || !crate::json::known_kind(v["kind"].as_str().unwrap_or(""))
            {
                return Ok(Verification::Unavailable);
            }
            let mut options = verify.decode_options();
            options.context = v["context"].clone();
            options.structured = v["native"].get("message").is_some();
            if let Some(via) = v.get("via") {
                options.via = Some(via.clone());
            }
            let (expected, available) = observe(
                &event.native_bytes()?,
                &options,
                options.native.get("request"),
            )?;
            if v["native"]["format"] != "fldigi_xmlrpc" || v.get("wire_id").is_some() {
                return Err(invalid("invalid fldigi evidence envelope"));
            }
            if !available {
                // Even an opaque body establishes its XML view independently of mapping.
                if let Some(view) = v["native"].get("message") {
                    let mut supplied = view.clone();
                    let wanted = &expected.value()["native"]["message"];
                    if let Some(view) = supplied.as_object_mut() {
                        if v["mapping"] != 1 || v["spec"] != crate::model::SPEC {
                            view.retain(|k, _| matches!(k.as_str(), "root" | "xml" | "method"));
                        }
                        if wanted.get("method").is_none() {
                            view.remove("method");
                        }
                    }
                    if !crate::json::semantic_eq(&supplied, wanted) {
                        return Err(invalid(
                            "fldigi XML evidence view disagrees with original body",
                        ));
                    }
                }
                return Ok(Verification::Unavailable);
            }
            let mut actual = v.clone();
            let mut wanted = expected.into_value();
            for value in [&mut actual, &mut wanted] {
                value.as_object_mut().expect("event").remove("via");
            }
            if v["mapping"] != 1 || v["spec"] != crate::model::SPEC {
                actual["fields"] = crate::json::known_fields(v);
                actual["spec"] = wanted["spec"].clone();
                actual["mapping"] = wanted["mapping"].clone();
                actual.as_object_mut().expect("event").retain(|k, _| {
                    wanted.get(k).is_some() || matches!(k.as_str(), "wire_id" | "native_version")
                });
                actual["native"]
                    .as_object_mut()
                    .expect("native")
                    .retain(|k, _| matches!(k.as_str(), "format" | "data" | "message"));
                if let Some(view) = actual["native"]
                    .get_mut("message")
                    .and_then(Value::as_object_mut)
                {
                    view.retain(|k, _| matches!(k.as_str(), "root" | "xml" | "method"));
                }
            }
            // A stored fault operation or view method cannot supply missing caller metadata.
            // Its required error content is still checked; dependent content is unavailable.
            let mut partial = false;
            if options.native.get("request").is_none() && wanted["fields"]["status"] == "error" {
                partial |= actual["fields"]
                    .as_object_mut()
                    .expect("fields")
                    .remove("operation")
                    .is_some();
                if let Some(view) = actual["native"]
                    .get_mut("message")
                    .and_then(Value::as_object_mut)
                {
                    partial |= view.remove("method").is_some();
                }
            }
            if !crate::json::semantic_eq(&actual, &wanted) {
                return Err(invalid(
                    "event disagrees with fldigi evidence and caller association",
                ));
            }
            Ok(if partial {
                Verification::Unavailable
            } else {
                Verification::Verified
            })
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
                    "unsupported fldigi mapping",
                ));
            }
            if event.is_command() {
                self.encode_commands(event, &EncodeOptions::default())?;
            } else {
                self.verify(event, options)?;
            }
            Ok(())
        })()
        .map_err(family_failure)
    }
}
impl Codec for Fldigi {
    fn framing(&self) -> Framing {
        Framing::RequestResponse
    }
    fn limits(&self) -> Limits {
        let mut l = Limits::stateless(MAX_BODY, MAX_BODY);
        l.parser = ParserLimits {
            depth: Some(16),
            elements: Some(8192),
            attributes: Some(256),
            numeric_token_bytes: Some(128),
            decoded_text_bytes: Some(MAX_BODY),
        };
        l
    }
    fn reset(&mut self, _: Option<&[u8]>) {}
    fn decode(&mut self, bytes: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error> {
        observe(bytes, options, options.native.get("request"))
            .map(|(e, _)| Outcome::event(e))
            .map_err(family_failure)
    }
    fn encode_commands(
        &self,
        event: &Event,
        options: &EncodeOptions,
    ) -> Result<crate::codec::EncodedCommands, Error> {
        (|| {
            let requests = command::encode(event, options)?
                .into_iter()
                .map(|bytes| {
                    let metadata = RequestAssociation::from_request_body(&bytes)?.into();
                    Ok(crate::codec::EncodedRequest::new(bytes, Some(metadata)))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            crate::codec::EncodedCommands::new(requests)
        })()
        .map_err(family_failure)
    }
    fn reencode(
        &self,
        event: &Event,
        options: &EncodeOptions,
        decode: &crate::codec::VerifyOptions,
    ) -> Result<Vec<u8>, Error> {
        (|| {
            if !options.native.is_empty() {
                return Err(invalid("fldigi re-encoding takes no encoding options"));
            }
            self.verify(event, decode)?;
            crate::json::write(
                event,
                &crate::PublicationOptions::new(crate::PublicationMode::Preserved),
            )?;
            event.native_bytes()
        })()
        .map_err(family_failure)
    }
}
