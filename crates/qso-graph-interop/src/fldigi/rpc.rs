// SPDX-License-Identifier: GPL-3.0-or-later
//! XML-RPC grammar and staged Annex E admission above the shared XML tokenizer.
#![forbid(unsafe_code)]
use crate::{
    Category, Error, base64,
    error::{invalid, oversized},
    xml::Node,
};
use serde_json::Value;

pub(super) fn malformed() -> Error {
    Error::new(Category::Malformed, "invalid XML-RPC container")
}
pub(super) fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\r', '\n'])
}
fn blank(n: &Node) -> bool {
    n.text.as_str().is_some_and(|s| trim(s).is_empty())
}
fn tag(n: &Node, name: &str) -> bool {
    n.plain && n.attrs.is_empty() && n.name == name
}
fn container(n: &Node, name: &str) -> Result<(), Error> {
    if tag(n, name) && blank(n) {
        Ok(())
    } else {
        Err(malformed())
    }
}
fn leaf(n: &Node, name: &str) -> Result<(), Error> {
    if tag(n, name) && n.children.is_empty() {
        Ok(())
    } else {
        Err(malformed())
    }
}
pub(super) fn typ(n: &Node) -> &str {
    n.children
        .first()
        .map_or("string", |n| if n.name == "i4" { "int" } else { &n.name })
}
fn known(s: &str) -> bool {
    matches!(
        s,
        "int"
            | "i4"
            | "boolean"
            | "double"
            | "string"
            | "dateTime.iso8601"
            | "base64"
            | "nil"
            | "struct"
            | "array"
    )
}
pub(super) fn content(n: &Node) -> &Value {
    &n.children.first().unwrap_or(n).text
}
fn children(n: &Node) -> Vec<&Node> {
    match typ(n) {
        "array" => n.children[0].children[0].children.iter().collect(),
        "struct" => n.children[0]
            .children
            .iter()
            .map(|m| &m.children[1])
            .collect(),
        _ => vec![],
    }
}
// Grammar is completed for the whole message before any XML-RPC resources/scalars.
fn grammar(n: &Node) -> Result<bool, Error> {
    if !tag(n, "value") {
        return Err(malformed());
    }
    if n.children.is_empty() {
        return Ok(false);
    }
    if n.children.len() != 1 || !blank(n) {
        return Err(malformed());
    }
    let t = &n.children[0];
    if !known(typ(n)) {
        if known(t.name.rsplit(':').next().unwrap_or("")) {
            return Err(malformed());
        }
        return Ok(true);
    }
    if !t.plain || !t.attrs.is_empty() {
        return Err(malformed());
    }
    match typ(n) {
        "array" => {
            container(t, "array")?;
            if t.children.len() != 1 {
                return Err(malformed());
            }
            container(&t.children[0], "data")?;
        }
        "struct" => {
            container(t, "struct")?;
            for m in &t.children {
                container(m, "member")?;
                if m.children.len() != 2 {
                    return Err(malformed());
                }
                leaf(&m.children[0], "name")?;
            }
        }
        _ => {
            if !t.children.is_empty() {
                return Err(malformed());
            }
        }
    }
    let mut opaque = false;
    for c in children(n) {
        opaque |= grammar(c)?;
    }
    Ok(opaque)
}
fn resources(n: &Node, numeric: bool) -> Result<(), Error> {
    if numeric {
        if matches!(typ(n), "int" | "boolean" | "double") {
            let count = content(n).as_str().map_or_else(
                || {
                    content(n)
                        .get("utf8_bytes")
                        .or_else(|| content(n).get("utf16_units"))
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len)
                },
                |s| trim(s).chars().count(),
            );
            if count > 128 {
                return Err(oversized("XML-RPC numeric token"));
            }
        }
    } else if (typ(n) == "array" && children(n).len() > 1024)
        || (typ(n) == "struct" && children(n).len() > 256)
    {
        return Err(oversized("XML-RPC container count"));
    }
    for c in children(n) {
        resources(c, numeric)?;
    }
    Ok(())
}
#[derive(Debug)]
pub(super) enum Scalar {
    String(Value),
    Int(i32),
    Double(f64),
    Bytes(Vec<u8>),
    Nil,
    Other,
}
fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}
fn decimal(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let mut e = s.split(['e', 'E']);
    let m = e.next().unwrap_or("");
    if let Some(exp) = e.next()
        && (!digits(exp.strip_prefix(['+', '-']).unwrap_or(exp)) || e.next().is_some())
    {
        return false;
    }
    if let Some((a, b)) = m.split_once('.') {
        (a.is_empty() || digits(a))
            && (b.is_empty() || digits(b))
            && !(a.is_empty() && b.is_empty())
    } else {
        digits(m)
    }
}
pub(super) fn scalar(n: &Node) -> Result<Scalar, Error> {
    let raw = content(n);
    if typ(n) == "string" {
        return Ok(Scalar::String(raw.clone()));
    }
    if !known(typ(n)) {
        return Ok(Scalar::Other);
    }
    if matches!(typ(n), "array" | "struct") {
        for c in children(n) {
            scalar(c)?;
        }
        return Ok(Scalar::Other);
    }
    let s = trim(
        raw.as_str()
            .ok_or_else(|| invalid("invalid XML-RPC scalar text"))?,
    );
    let bad = || invalid("invalid XML-RPC scalar");
    Ok(match typ(n) {
        "int" => {
            if !digits(s.strip_prefix(['+', '-']).unwrap_or(s)) {
                return Err(bad());
            }
            Scalar::Int(s.parse().map_err(|_| bad())?)
        }
        "boolean" => {
            if !matches!(s, "0" | "1") {
                return Err(bad());
            }
            Scalar::Other
        }
        "double" => {
            if !decimal(s) {
                return Err(bad());
            }
            let f: f64 = s.parse().map_err(|_| bad())?;
            if !f.is_finite() {
                return Err(bad());
            }
            Scalar::Double(f)
        }
        "nil" => {
            if !s.is_empty() {
                return Err(bad());
            }
            Scalar::Nil
        }
        "base64" => Scalar::Bytes(base64::decode(
            &s.chars()
                .filter(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
                .collect::<String>(),
        )?),
        "dateTime.iso8601" => {
            if s.len() != 17
                || !s.is_ascii()
                || s.as_bytes().get(8) != Some(&b'T')
                || !digits(&s[..8])
                || !s.is_ascii()
            {
                return Err(bad());
            }
            let date = format!("{}-{}-{}T{}Z", &s[..4], &s[4..6], &s[6..8], &s[9..]);
            if !crate::json::valid_instant(&date) {
                return Err(bad());
            }
            Scalar::Other
        }
        _ => Scalar::Other,
    })
}
pub(super) struct Message<'a> {
    pub method: Option<&'a Node>,
    pub values: Vec<&'a Node>,
    pub fault: bool,
    pub opaque: bool,
    pub recognized: bool,
}
pub(super) fn parse(root: &Node) -> Result<Message<'_>, Error> {
    let call = root.plain && root.name == "methodCall";
    let response = root.plain && root.name == "methodResponse";
    let mut out = Message {
        method: None,
        values: vec![],
        fault: false,
        opaque: false,
        recognized: call || response,
    };
    if !out.recognized {
        out.opaque = true;
        return Ok(out);
    }
    container(root, if call { "methodCall" } else { "methodResponse" })?;
    let params = if call {
        if !(1..=2).contains(&root.children.len()) {
            return Err(malformed());
        }
        leaf(&root.children[0], "methodName")?;
        out.method = Some(&root.children[0]);
        root.children.get(1)
    } else {
        if root.children.len() != 1 {
            return Err(malformed());
        }
        Some(&root.children[0])
    };
    if let Some(p) = params {
        if response && p.name == "fault" {
            container(p, "fault")?;
            if p.children.len() != 1 {
                return Err(malformed());
            }
            out.fault = true;
            out.values.push(&p.children[0]);
        } else {
            container(p, "params")?;
            if response && p.children.len() != 1 {
                return Err(malformed());
            }
            for param in &p.children {
                container(param, "param")?;
                if param.children.len() != 1 {
                    return Err(malformed());
                }
                out.values.push(&param.children[0]);
            }
        }
    }
    for v in &out.values {
        out.opaque |= grammar(v)?;
    }
    if out.fault {
        let v = out.values[0];
        if typ(v) != "struct" {
            return Err(malformed());
        }
        for (name, want) in [("faultCode", "int"), ("faultString", "string")] {
            let found: Vec<_> = v.children[0]
                .children
                .iter()
                .filter(|m| m.children[0].text == name)
                .collect();
            if found.len() != 1 || typ(&found[0].children[1]) != want {
                return Err(malformed());
            }
        }
    }
    if out.values.len() > 32 {
        return Err(oversized("XML-RPC params"));
    }
    for numeric in [false, true] {
        for v in &out.values {
            resources(v, numeric)?;
        }
    }
    for v in &out.values {
        scalar(v)?;
    }
    Ok(out)
}
pub(super) fn method(v: &Value) -> Result<&str, Error> {
    let s = v
        .as_str()
        .ok_or_else(|| invalid("invalid XML-RPC method name"))?;
    if s.len() > 128 {
        return Err(oversized("XML-RPC method name"));
    }
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'/'))
    {
        return Err(invalid("invalid XML-RPC method name"));
    }
    Ok(s)
}
