// SPDX-License-Identifier: GPL-3.0-or-later
//! Canonical independent request bodies, never HTTP or multicall.
#![forbid(unsafe_code)]
use super::{catalog, operation, parse, rpc};
use crate::{
    Category, Error, Event,
    codec::EncodeOptions,
    error::{invalid, oversized},
    xml,
};
use serde_json::Value;
fn integer(v: &Value, min: i64) -> Result<String, Error> {
    crate::json::integer_i64(v)
        .filter(|n| (min..=i64::from(i32::MAX)).contains(n))
        .map(|n| n.to_string())
        .ok_or_else(|| invalid("fldigi command integer out of range"))
}
fn text(v: &Value) -> Result<String, Error> {
    let s = v
        .as_str()
        .ok_or_else(|| invalid("fldigi command requires valid XML string"))?;
    if s.chars().any(|c| c == '\r' || !xml::legal(c)) {
        return Err(invalid(
            "fldigi command contains CR or forbidden XML character",
        ));
    }
    if s.len() > super::MAX_BODY {
        return Err(oversized("fldigi command text"));
    }
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
        if out.len() > super::MAX_BODY {
            return Err(oversized("fldigi escaped command text"));
        }
    }
    Ok(out)
}
fn body(name: &str, args: &[(&str, String)]) -> Result<Vec<u8>, Error> {
    let mut s = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><methodCall><methodName>{name}</methodName><params>"
    );
    for (typ, content) in args {
        s.push_str(&format!(
            "<param><value><{typ}>{content}</{typ}></value></param>"
        ));
    }
    s.push_str("</params></methodCall>");
    let bytes = s.into_bytes();
    let doc = parse(&bytes)?;
    rpc::parse(&doc.root)?;
    Ok(bytes)
}
pub(super) fn encode(event: &Event, options: &EncodeOptions) -> Result<Vec<Vec<u8>>, Error> {
    let v = event.value();
    if v["mapping"] != 1 || v["spec"] != crate::model::SPEC {
        return Err(Error::new(
            Category::UnsupportedVersion,
            "unsupported fldigi command revision",
        ));
    }
    if v["family"] != "fldigi"
        || v["kind"] != "control"
        || !event.is_command()
        || v.get("native_version").is_some()
        || v.get("wire_id").is_some()
        || !options.native.is_empty()
    {
        return Err(invalid("invalid fldigi command envelope or options"));
    }
    let f = &v["fields"];
    let op = f["operation"].as_str().unwrap_or("");
    let mut allowed = vec!["operation"];
    let mut requests = Vec::new();
    match op {
        "set_frequency" => {
            allowed.extend(["frequency_hz", "rx_offset_hz"]);
            requests.push(body(
                "rig.set_frequency",
                &[("double", format!("{}.0", integer(&f["frequency_hz"], 1)?))],
            )?);
            if let Some(offset) = f.get("rx_offset_hz") {
                requests.push(body("modem.set_carrier", &[("int", integer(offset, 1)?)])?);
            }
        }
        "send_text" => {
            allowed.extend(["text", "transmit"]);
            let transmit = f["transmit"]
                .as_bool()
                .ok_or_else(|| invalid("send_text requires boolean transmit"))?;
            requests.push(body("text.add_tx", &[("string", text(&f["text"])?)])?);
            if transmit {
                requests.push(body("main.tx", &[])?)
            }
        }
        "halt_tx" => requests.push(body("main.abort", &[])?),
        _ => {
            let m = catalog::METHODS
                .iter()
                .find(|m| operation(m.name) == op)
                .ok_or_else(|| invalid("unsupported fldigi command operation"))?;
            let mut args = Vec::new();
            for (name, typ) in m.args {
                allowed.push(name);
                args.push((
                    *typ,
                    if *typ == "string" {
                        text(&f[name])?
                    } else {
                        integer(&f[name], if *name == "audio_offset_hz" { 1 } else { 0 })?
                    },
                ));
            }
            requests.push(body(m.name, &args)?);
        }
    }
    if f.as_object()
        .is_none_or(|f| f.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        return Err(invalid("unsupported fldigi command arguments"));
    }
    Ok(requests)
}
