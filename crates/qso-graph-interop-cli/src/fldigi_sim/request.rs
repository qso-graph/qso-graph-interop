// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded scalar subset of bundled XmlRpcServer::parseRequest / XmlRpcValue.
//! It deliberately searches for methodName without requiring a well-formed outer document.
#![forbid(unsafe_code)]
use serde_json::{Value, json};
// parseTag/findTag skip preceding bytes and attributes. There is no entity resolver.
fn find_open<'a>(input: &'a str, tag: &str) -> Option<(&'a str, bool)> {
    let start = input.find(&format!("<{tag}"))?;
    let rest = &input[start + tag.len() + 1..];
    let end = rest.find('>')?;
    Some((&rest[end + 1..], rest[..end].ends_with('/')))
}
fn next_open<'a>(input: &'a str, tag: &str) -> Option<(&'a str, bool)> {
    let input = input.trim_start_matches([' ', '\t', '\r', '\n']);
    let rest = input.strip_prefix(&format!("<{tag}"))?;
    if !rest.starts_with(['>', '/', ' ', '\t', '\r', '\n']) {
        return None;
    }
    let end = rest.find('>')?;
    Some((&rest[end + 1..], rest[..end].ends_with('/')))
}
fn decoded(s: &str) -> String {
    // XmlRpcUtil::xmlDecode recognizes only these five names, in one pass.
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        if let Some((entity, ch)) = [
            ("&amp;", '&'),
            ("&lt;", '<'),
            ("&gt;", '>'),
            ("&quot;", '"'),
            ("&apos;", '\''),
        ]
        .into_iter()
        .find(|(e, _)| rest.starts_with(e))
        {
            out.push(ch);
            rest = &rest[entity.len()..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}
fn scalar(input: &str) -> Value {
    let Some((v, empty)) = next_open(input, "value") else {
        return Value::Null;
    };
    if empty {
        return json!({"string":""});
    }
    for tag in ["int", "i4", "double", "string"] {
        if let Some((rest, empty)) = next_open(v, tag) {
            let text = if empty {
                ""
            } else {
                rest.split('<').next().unwrap_or("")
            };
            return match tag {
                "int" | "i4" => {
                    if empty {
                        json!({"int":0})
                    } else {
                        text.trim()
                            .parse::<i32>()
                            .map_or(Value::Null, |n| json!({"int":n}))
                    }
                }
                "double" => {
                    if empty {
                        json!({"double":0.0})
                    } else {
                        text.trim()
                            .parse::<f64>()
                            .ok()
                            .filter(|n| n.is_finite())
                            .map_or(Value::Null, |n| json!({"double":n}))
                    }
                }
                _ => json!({"string":decoded(text)}),
            };
        }
    }
    if !v.starts_with('<') || v.starts_with("</value>") {
        json!({"string":decoded(v.split('<').next().unwrap_or(""))})
    } else {
        Value::Null
    }
}
pub fn parse(body: &[u8]) -> (String, Result<Value, String>) {
    let raw = std::str::from_utf8(body).unwrap_or("");
    if raw.chars().any(
        |c| !matches!(c as u32, 9 | 10 | 13 | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff),
    ) {
        return (String::new(), Ok(json!({})));
    }
    let Some((tail, empty)) = find_open(raw, "methodName") else {
        return (String::new(), Ok(json!({})));
    };
    let (method, tail) = if empty {
        ("", tail)
    } else {
        let Some((method, tail)) = tail.split_once("</methodName>") else {
            return (String::new(), Ok(json!({})));
        };
        (method, tail)
    };
    let expected: &[(&str, &str)] = match method {
        "rig.set_frequency" | "main.set_frequency" => &[("frequency_hz", "double")],
        "modem.set_carrier" => &[("audio_offset_hz", "int")],
        "modem.set_by_name" | "rig.set_mode" => &[("mode", "string")],
        "text.add_tx" => &[("text", "string")],
        "log.set_call" => &[("dx_call", "string")],
        "log.set_locator" => &[("dx_grid", "string")],
        "text.get_rx" => &[("rx_start", "int"), ("rx_length", "int")],
        _ => &[],
    };
    if expected.is_empty() {
        return (method.into(), Ok(json!({})));
    }
    let mut params = vec![];
    if let Some((mut tail, false)) = find_open(tail, "params") {
        while let Some((rest, empty)) = next_open(tail, "param") {
            if params.len() == 32 {
                return (method.into(), Err("simulator argument limit".into()));
            }
            if empty {
                params.push(json!({"string":""}));
                tail = rest;
            } else {
                params.push(scalar(rest));
                let Some((_, after)) = rest.split_once("</param>") else {
                    break;
                };
                tail = after;
            }
        }
    }
    let err = if params.is_empty() {
        "type error: expected an array"
    } else if params.len() < expected.len() {
        "range error: array index too large"
    } else {
        "type error"
    };
    if method == "text.get_rx" && params.len() != 2 {
        return (method.into(), Err("Bad size".into()));
    }
    let mut fields = json!({});
    for (i, (key, typ)) in expected.iter().enumerate() {
        let Some(value) = params.get(i).and_then(|p| p.get(typ)) else {
            return (method.into(), Err(err.into()));
        };
        fields[key] = value.clone();
    }
    (method.into(), Ok(fields))
}
