// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded event JSON, reader compatibility and recursive producer validation.
#![forbid(unsafe_code)]
use crate::{
    Category, Error,
    codec::Profile,
    error::{invalid, oversized},
    model::*,
};
use serde_json::{Map, Value};

pub const MAX_DEPTH: usize = 32;
pub const MAX_ELEMENTS: usize = 65_536;
/// A general event cannot have more members/elements than its bounded UTF-8 byte count.
pub const MAX_RECORD_ELEMENTS: usize = MAX_RECORD;
pub const MAX_NUMBER: usize = 128;

/// A bounded recursive JSON parser. Scalars delegate Unicode and number syntax to serde_json;
/// objects are assembled here so reserved serde implementation keys remain ordinary event data.
struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    elements: usize,
    max_elements: usize,
}
impl Parser<'_> {
    fn malformed(&self) -> Error {
        Error::new(Category::Malformed, "invalid event JSON framing")
    }
    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }
    fn element(&mut self) -> Result<(), Error> {
        self.elements += 1;
        if self.elements > self.max_elements {
            Err(oversized("JSON element limit exceeded"))
        } else {
            Ok(())
        }
    }
    fn string(&mut self) -> Result<String, Error> {
        let start = self.at;
        self.at += 1;
        let mut escaped = false;
        while let Some(&b) = self.bytes.get(self.at) {
            self.at += 1;
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                return serde_json::from_slice(&self.bytes[start..self.at])
                    .map_err(|_| self.malformed());
            }
        }
        Err(self.malformed())
    }
    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        self.whitespace();
        let b = *self.bytes.get(self.at).ok_or_else(|| self.malformed())?;
        if matches!(b, b'{' | b'[') && depth > MAX_DEPTH {
            return Err(oversized("JSON container depth exceeded"));
        }
        match b {
            b'{' => {
                self.at += 1;
                self.whitespace();
                let mut map = Map::new();
                if self.bytes.get(self.at) == Some(&b'}') {
                    self.at += 1;
                    return Ok(Value::Object(map));
                }
                loop {
                    self.element()?;
                    self.whitespace();
                    if self.bytes.get(self.at) != Some(&b'"') {
                        return Err(self.malformed());
                    }
                    let key = self.string()?;
                    self.whitespace();
                    if map.contains_key(&key) {
                        return Err(Error::new(Category::Malformed, "duplicate JSON member"));
                    }
                    if self.bytes.get(self.at) != Some(&b':') {
                        return Err(self.malformed());
                    }
                    self.at += 1;
                    map.insert(key, self.value(depth + 1)?);
                    self.whitespace();
                    match self.bytes.get(self.at) {
                        Some(b'}') => {
                            self.at += 1;
                            break;
                        }
                        Some(b',') => self.at += 1,
                        _ => return Err(self.malformed()),
                    }
                }
                Ok(Value::Object(map))
            }
            b'[' => {
                self.at += 1;
                self.whitespace();
                let mut array = Vec::new();
                if self.bytes.get(self.at) == Some(&b']') {
                    self.at += 1;
                    return Ok(Value::Array(array));
                }
                loop {
                    self.element()?;
                    array.push(self.value(depth + 1)?);
                    self.whitespace();
                    match self.bytes.get(self.at) {
                        Some(b']') => {
                            self.at += 1;
                            break;
                        }
                        Some(b',') => self.at += 1,
                        _ => return Err(self.malformed()),
                    }
                }
                Ok(Value::Array(array))
            }
            b'"' => self.string().map(Value::String),
            b'-' | b'0'..=b'9' => {
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| matches!(b, b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'))
                {
                    self.at += 1;
                }
                if self.at - start > MAX_NUMBER {
                    return Err(oversized("JSON numeric token limit exceeded"));
                }
                std::str::from_utf8(&self.bytes[start..self.at])
                    .ok()
                    .and_then(|s| s.parse::<serde_json::Number>().ok())
                    .map(Value::Number)
                    .ok_or_else(|| self.malformed())
            }
            _ => {
                for (literal, value) in [
                    (b"true".as_slice(), Value::Bool(true)),
                    (b"false".as_slice(), Value::Bool(false)),
                    (b"null".as_slice(), Value::Null),
                ] {
                    if self.bytes[self.at..].starts_with(literal) {
                        self.at += literal.len();
                        return Ok(value);
                    }
                }
                Err(self.malformed())
            }
        }
    }
}

/// Lexical resource admission precedes allocation by serde. JSON syntax is checked by serde.
fn preflight(bytes: &[u8], limit: usize) -> Result<(), Error> {
    if bytes.len() > limit {
        return Err(oversized("JSON frame exceeds byte limit"));
    }
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
                // Decoding cannot expand JSON text beyond the source span.
                if i - start > limit {
                    return Err(oversized("decoded string limit exceeded"));
                }
            }
        } else {
            match b {
                b'"' => {
                    quoted = true;
                    start = i + 1;
                }
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err(oversized("JSON container depth exceeded"));
                    }
                }
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                }
                b'-' | b'0'..=b'9' => {
                    let begin = i;
                    while i < bytes.len()
                        && matches!(bytes[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                    {
                        i += 1;
                    }
                    if i - begin > MAX_NUMBER {
                        return Err(oversized("JSON numeric token limit exceeded"));
                    }
                    continue;
                }
                _ => {}
            }
        }
        i += 1;
    }
    Ok(())
}
/// Parse one record, excluding its stream delimiter, with the transport's input bound.
pub fn parse(bytes: &[u8], limit: usize) -> Result<Event, Error> {
    parse_limited(bytes, limit, MAX_RECORD_ELEMENTS)
}
/// Read the event-carriage parser profile (Annex I) before appending its receiving hop.
pub fn parse_carriage(bytes: &[u8], limit: usize) -> Result<Event, Error> {
    parse_limited(bytes, limit, MAX_ELEMENTS)
}
fn parse_limited(bytes: &[u8], limit: usize, max_elements: usize) -> Result<Event, Error> {
    let limit = limit.min(MAX_RECORD);
    preflight(bytes, limit)?;
    let mut parser = Parser {
        bytes,
        at: 0,
        elements: 0,
        max_elements,
    };
    let v = parser.value(1)?;
    parser.whitespace();
    if parser.at != bytes.len() || !v.is_object() {
        return Err(Error::new(
            Category::Malformed,
            "event JSON must be one complete object",
        ));
    }
    bounds_with_elements(&v, limit, max_elements)?;
    Event::from_value(v)
}
/// Enforce all parser limits against a resulting value (including newly appended hops).
pub fn bounds(v: &Value, string_limit: usize) -> Result<(), Error> {
    bounds_with_elements(v, string_limit, MAX_ELEMENTS)
}
fn bounds_with_elements(v: &Value, string_limit: usize, max_elements: usize) -> Result<(), Error> {
    fn walk(
        v: &Value,
        depth: usize,
        count: &mut usize,
        limit: usize,
        max_elements: usize,
    ) -> Result<(), Error> {
        match v {
            Value::Object(m) => {
                if depth > MAX_DEPTH {
                    return Err(oversized("JSON container depth exceeded"));
                }
                *count = count
                    .checked_add(m.len())
                    .ok_or_else(|| oversized("JSON element limit exceeded"))?;
                if *count > max_elements {
                    return Err(oversized("JSON element limit exceeded"));
                }
                for (k, v) in m {
                    if k.len() > limit {
                        return Err(oversized("JSON key limit exceeded"));
                    }
                    walk(v, depth + 1, count, limit, max_elements)?;
                }
            }
            Value::Array(a) => {
                if depth > MAX_DEPTH {
                    return Err(oversized("JSON container depth exceeded"));
                }
                *count += a.len();
                if *count > max_elements {
                    return Err(oversized("JSON element limit exceeded"));
                }
                for v in a {
                    walk(v, depth + 1, count, limit, max_elements)?;
                }
            }
            Value::String(s) if s.len() > limit => {
                return Err(oversized("decoded string limit exceeded"));
            }
            Value::Number(n) if n.to_string().len() > MAX_NUMBER => {
                return Err(oversized("numeric token limit exceeded"));
            }
            _ => {}
        }
        Ok(())
    }
    walk(v, 1, &mut 0, string_limit, max_elements)
}
/// Compact, UTF-8 output, with exact record admission before publication.
pub fn write(
    event: &Event,
    producer_strict: bool,
    profile: Option<&dyn Profile>,
) -> Result<Vec<u8>, Error> {
    validate(event.value(), producer_strict)?;
    if producer_strict
        && event.value()["native"].get("message").is_some()
        && !profile.is_some_and(|p| event.value()["family"] == p.family())
    {
        return Err(invalid(
            "strict structured-view publication requires its family profile",
        ));
    }
    if producer_strict && let Some(p) = profile {
        p.validate_authored(event)?;
    }
    bounds_with_elements(event.value(), MAX_RECORD, MAX_RECORD_ELEMENTS)?;
    let mut output = RecordBuffer {
        data: Vec::new(),
        limit: MAX_RECORD,
    };
    serde_json::to_writer(&mut output, event.value())
        .map_err(|_| oversized("event exceeds the 1 MiB compact record bound"))?;
    Ok(output.data)
}
/// A serde output sink that never grows beyond the record budget.
struct RecordBuffer {
    data: Vec<u8>,
    limit: usize,
}
impl std::io::Write for RecordBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.data.len()) {
            return Err(std::io::Error::other("event record limit exceeded"));
        }
        self.data.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn object(v: &Value) -> Result<&Map<String, Value>, Error> {
    v.as_object().ok_or_else(|| invalid("expected object"))
}
fn string(v: &Value) -> Result<&str, Error> {
    v.as_str().ok_or_else(|| invalid("expected string"))
}
fn token(s: &str) -> bool {
    let mut b = s.bytes();
    b.next().is_some_and(|b| b.is_ascii_lowercase())
        && b.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn operation(s: &str) -> bool {
    if let Some((prefix, t)) = s.rsplit_once('.') {
        !prefix.is_empty()
            && prefix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-'))
            && token(t)
    } else {
        token(s)
    }
}
pub fn valid_clock(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 8 || b[2] != b':' || b[5] != b':' {
        return false;
    }
    for i in [0, 1, 3, 4, 6, 7] {
        if !b[i].is_ascii_digit() {
            return false;
        }
    }
    let n = |i: usize| u16::from(b[i] - b'0') * 10 + u16::from(b[i + 1] - b'0');
    n(0) < 24
        && n(3) < 60
        && n(6) <= 60
        && (b.len() == 8 || b.len() > 9 && b[8] == b'.' && b[9..].iter().all(u8::is_ascii_digit))
}
pub fn valid_instant(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || !s.ends_with('Z') {
        return false;
    }
    if ![0, 1, 2, 3, 5, 6, 8, 9]
        .iter()
        .all(|&i| b[i].is_ascii_digit())
    {
        return false;
    }
    let Ok(y) = s[..4].parse::<u32>() else {
        return false;
    };
    let Ok(m) = s[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(d) = s[8..10].parse::<u32>() else {
        return false;
    };
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28 + u32::from(y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)),
        _ => 0,
    };
    y > 0 && d > 0 && d <= days && valid_clock(&s[11..s.len() - 1])
}
fn text(v: &Value, strict: bool) -> Result<(), Error> {
    if v.is_null() || v.is_string() {
        return Ok(());
    }
    let m = object(v)?;
    let (key, max) = if m.contains_key("utf8_bytes") {
        ("utf8_bytes", 255)
    } else {
        ("utf16_units", 65535)
    };
    if m.contains_key("utf8_bytes") && m.contains_key("utf16_units") {
        return Err(invalid("text has two encodings"));
    }
    if strict && m.len() != 1 {
        return Err(invalid("unknown text wrapper member"));
    }
    let a = m
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("invalid text representation"))?;
    if !a.iter().all(|v| integer_u64(v).is_some_and(|n| n <= max)) {
        return Err(invalid("raw text unit out of range"));
    }
    let valid = if key == "utf8_bytes" {
        std::str::from_utf8(
            &a.iter()
                .map(|v| integer_u64(v).unwrap_or(0) as u8)
                .collect::<Vec<_>>(),
        )
        .is_ok()
    } else {
        String::from_utf16(
            &a.iter()
                .map(|v| integer_u64(v).unwrap_or(0) as u16)
                .collect::<Vec<_>>(),
        )
        .is_ok()
    };
    if valid {
        return Err(invalid("valid text must use a JSON string"));
    }
    Ok(())
}
// Canonical exact decimal values avoid rounding integer comparisons through binary64.
fn decimal(n: &serde_json::Number) -> (bool, String, String) {
    fn magnitude_add(a: &str, b: &str, subtract: bool) -> String {
        let mut out = Vec::new();
        let mut carry = 0_i16;
        let mut aa = a.bytes().rev();
        let mut bb = b.bytes().rev();
        loop {
            let x = aa.next();
            let y = bb.next();
            if x.is_none() && y.is_none() {
                break;
            }
            let x = i16::from(x.unwrap_or(b'0') - b'0');
            let y = i16::from(y.unwrap_or(b'0') - b'0');
            let v = if subtract {
                x - y + carry
            } else {
                x + y + carry
            };
            out.push((v.rem_euclid(10) as u8) + b'0');
            carry = v.div_euclid(10);
        }
        if carry > 0 {
            out.push(carry as u8 + b'0');
        }
        while out.len() > 1 && out.last() == Some(&b'0') {
            out.pop();
        }
        out.reverse();
        String::from_utf8(out).expect("decimal digits")
    }
    let text = n.to_string();
    let negative = text.starts_with('-');
    let text = text.trim_start_matches('-');
    let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
    let fraction = mantissa.split_once('.').map_or(0, |(_, s)| s.len());
    let digits = mantissa.replace('.', "");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return (false, "0".into(), "0".into());
    }
    let coefficient = digits.trim_end_matches('0').to_owned();
    let delta = (digits.len() - coefficient.len()) as isize - fraction as isize;
    let eneg = exponent.starts_with('-');
    let mag = exponent
        .trim_start_matches(['-', '+'])
        .trim_start_matches('0');
    let mag = if mag.is_empty() { "0" } else { mag };
    let adjustment = delta.unsigned_abs().to_string();
    let dneg = delta < 0;
    let (negative_exp, adjusted) = if eneg == dneg {
        (eneg, magnitude_add(mag, &adjustment, false))
    } else if (mag.len(), mag) >= (adjustment.len(), adjustment.as_str()) {
        (eneg, magnitude_add(mag, &adjustment, true))
    } else {
        (dneg, magnitude_add(&adjustment, mag, true))
    };
    let exponent = if negative_exp && adjusted != "0" {
        format!("-{adjusted}")
    } else {
        adjusted
    };
    (negative, coefficient, exponent)
}
fn integral(v: &Value) -> Option<i128> {
    let Value::Number(n) = v else {
        return None;
    };
    let (negative, mut coefficient, exponent) = decimal(n);
    let exponent = exponent.parse::<usize>().ok()?;
    if coefficient.len().checked_add(exponent)? > 39 {
        return None;
    }
    coefficient.extend(std::iter::repeat_n('0', exponent));
    let value = coefficient.parse::<i128>().ok()?;
    Some(if negative { -value } else { value })
}
/// Read a mathematical JSON integer without rounding through binary64.
pub fn integer_i64(v: &Value) -> Option<i64> {
    v.as_i64()
        .or_else(|| integral(v).and_then(|n| i64::try_from(n).ok()))
}
/// Read a nonnegative mathematical JSON integer without precision loss.
pub fn integer_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| integral(v).and_then(|n| u64::try_from(n).ok()))
}
fn integer(v: &Value, min: i64, max: u64) -> bool {
    integral(v).is_some_and(|n| n >= i128::from(min) && n <= i128::from(max))
}

fn shape(v: &Value, name: &str, strict: bool) -> Result<(), Error> {
    let (props, required) =
        crate::catalog::catalog(name).ok_or_else(|| invalid("unknown shape"))?;
    let m = object(v)?;
    for k in required {
        if !m.contains_key(*k) {
            return Err(invalid("required member absent"));
        }
    }
    for (k, v) in m {
        let Some((_, typ)) = props.iter().find(|(n, _)| *n == k) else {
            if strict {
                return Err(invalid("unknown producer member"));
            }
            continue;
        };
        let ok = match *typ {
            "text" => {
                text(v, strict)?;
                true
            }
            "integer" => integer(v, -MAX_INTEGER, MAX_INTEGER as u64),
            "boolean" => v.is_boolean(),
            "string" => v.is_string(),
            "object" => v.is_object(),
            "array" => v.is_array(),
            "operation" => v.as_str().is_some_and(operation),
            "token" => v.as_str().is_some_and(token),
            "enum_status" => matches!(v.as_str(), Some("ok" | "error")),
            "time_of_day" => v.as_str().is_some_and(valid_clock),
            "instant" => v.as_str().is_some_and(valid_instant),
            "base64" => v.as_str().is_some_and(|s| crate::base64::decode(s).is_ok()),
            "binary64" => {
                v.as_f64().is_some_and(f64::is_finite)
                    || v.as_str().is_some_and(|s| {
                        s.strip_prefix("ieee754:").is_some_and(|s| {
                            s.len() == 16
                                && s.bytes()
                                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
                                && u64::from_str_radix(s, 16)
                                    .is_ok_and(|n| !f64::from_bits(n).is_finite())
                        })
                    })
            }
            "radio" | "color" | "ipfix_header" | "context" | "native_payload" => {
                shape(v, typ, strict)?;
                true
            }
            _ => true,
        };
        if !ok {
            return Err(invalid("member has an invalid type or value"));
        }
    }
    match name {
        "native_payload" if v["format"].as_str().is_none_or(str::is_empty) => {
            return Err(invalid("native format must be nonempty"));
        }
        "context" => {
            for k in ["direction", "exchange"] {
                if string(&v[k])?.is_empty() {
                    return Err(invalid("empty context enum"));
                }
            }
            if strict
                && (!matches!(
                    v["direction"].as_str(),
                    Some("from_application" | "to_application" | "unknown")
                ) || !matches!(
                    v["exchange"].as_str(),
                    Some("notification" | "request" | "response" | "unknown")
                ))
            {
                return Err(invalid("unknown producer context enum"));
            }
            let mut context = RecordBuffer {
                data: Vec::new(),
                limit: MAX_CONTEXT,
            };
            if serde_json::to_writer(&mut context, v).is_err() {
                return Err(oversized("context exceeds 4096 bytes"));
            }
        }
        "color" => {
            if !integer(&v["spec"], 0, 255)
                || !v["channels"]
                    .as_array()
                    .is_some_and(|a| a.len() == 5 && a.iter().all(|v| integer(v, 0, 65535)))
            {
                return Err(invalid("invalid color"));
            }
        }
        "radio" => {
            if m.is_empty() {
                return Err(invalid("empty radio label"));
            }
        }
        "ipfix_header" => {
            for (k, v) in m {
                let max = if matches!(k.as_str(), "version" | "length") {
                    65535
                } else {
                    u64::from(u32::MAX)
                };
                if props.iter().any(|(p, _)| *p == k) && !integer(v, 0, max) {
                    return Err(invalid("IPFIX header out of range"));
                }
            }
        }
        "native_payload" => {
            for k in ["set_id", "set_index", "record_index"] {
                if let Some(v) = m.get(k)
                    && !integer(
                        v,
                        0,
                        if k == "set_id" {
                            65535
                        } else {
                            MAX_INTEGER as u64
                        },
                    )
                {
                    return Err(invalid("native index out of range"));
                }
            }
            if let Some(a) = m.get("dependencies") {
                for d in a
                    .as_array()
                    .ok_or_else(|| invalid("invalid dependencies"))?
                {
                    shape(d, "ipfix_dependency", strict)?;
                    if !integer(&d["set_id"], 0, 65535) {
                        return Err(invalid("invalid dependency Set ID"));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn known_kind(kind: &str) -> bool {
    crate::catalog::catalog(kind).is_some()
        && matches!(
            kind,
            "decode"
                | "radio_state"
                | "qso_logged"
                | "qso_changed"
                | "qso_deleted"
                | "spot"
                | "score"
                | "control"
                | "result"
                | "notification"
                | "lifecycle"
                | "ambiguous"
                | "native"
        )
}
pub fn official(family: &str) -> bool {
    matches!(
        family,
        "wsjtx" | "qgi" | "n1mm" | "js8call" | "fldigi" | "pskreporter-mqtt" | "pskreporter-ipfix"
    )
}
pub fn validate(v: &Value, strict: bool) -> Result<(), Error> {
    let m = object(v)?;
    let version = string(&v["spec"])?;
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|s| {
            s.is_empty()
                || !s.bytes().all(|b| b.is_ascii_digit())
                || s.len() > 1 && s.starts_with('0')
        })
    {
        return Err(invalid("invalid specification version"));
    }
    if parts[0] != "0" {
        return Err(Error::new(
            Category::UnsupportedVersion,
            "unsupported specification major version",
        ));
    }
    if strict && version != SPEC {
        return Err(Error::new(
            Category::UnsupportedVersion,
            "unsupported producer specification version",
        ));
    }
    let family = string(&v["family"])?;
    if family.is_empty()
        || !family.as_bytes()[0].is_ascii_lowercase() && !family.as_bytes()[0].is_ascii_digit()
        || !family
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-'))
    {
        return Err(invalid("invalid family name"));
    }
    if !integer(&v["mapping"], 1, MAX_INTEGER as u64) {
        return Err(invalid("invalid mapping revision"));
    }
    let kind = string(&v["kind"])?;
    if kind.is_empty() || strict && !known_kind(kind) {
        return Err(invalid("unknown producer kind"));
    }
    object(&v["fields"])?;
    shape(&v["context"], "context", strict)?;
    if let Some(id) = m.get("wire_id") {
        text(id, strict)?;
    }
    if let Some(nv) = m.get("native_version") {
        string(nv)?;
    }
    if let Some(n) = m.get("native") {
        shape(n, "native_payload", strict)?;
        let forbidden: &[&str] = match n["format"].as_str() {
            Some("mqtt_publish") => {
                if !n
                    .as_object()
                    .is_some_and(|m| m.contains_key("topic") && m.contains_key("retained"))
                {
                    return Err(invalid("PUBLISH lacks topic or retain flag"));
                }
                &[
                    "template",
                    "header",
                    "message_data",
                    "set_id",
                    "set_index",
                    "record_index",
                    "dependencies",
                ]
            }
            Some("ipfix_record") => {
                for k in [
                    "template",
                    "header",
                    "message_data",
                    "set_id",
                    "set_index",
                    "record_index",
                ] {
                    if n.get(k).is_none() {
                        return Err(invalid("IPFIX record lacks evidence"));
                    }
                }
                &["topic", "retained"]
            }
            _ => &[
                "topic",
                "retained",
                "template",
                "header",
                "message_data",
                "set_id",
                "set_index",
                "record_index",
                "dependencies",
            ],
        };
        if forbidden.iter().any(|k| n.get(*k).is_some()) {
            return Err(invalid("native member belongs to a different class"));
        }
    } else if kind != "control"
        || m.contains_key("native_version")
        || v["fields"].get("operation").is_none()
    {
        return Err(invalid("invalid command form"));
    }
    if official(family) && known_kind(kind) {
        shape(&v["fields"], kind, strict)?;
    }
    if let Some(hops) = m.get("via") {
        let a = hops
            .as_array()
            .ok_or_else(|| invalid("via must be an array"))?;
        if a.len() > 8 {
            return Err(invalid("more than eight carriage hops"));
        }
        for h in a {
            let hm = object(h)?;
            if h["family"] != "qgi" {
                return Err(invalid("invalid carriage hop family"));
            }
            shape(&h["context"], "context", strict)?;
            if strict
                && hm
                    .keys()
                    .any(|k| !matches!(k.as_str(), "family" | "context"))
            {
                return Err(invalid("unknown hop member"));
            }
        }
    }
    if strict
        && m.keys().any(|k| {
            !matches!(
                k.as_str(),
                "spec"
                    | "family"
                    | "mapping"
                    | "kind"
                    | "context"
                    | "fields"
                    | "native"
                    | "native_version"
                    | "wire_id"
                    | "via"
            )
        })
    {
        return Err(invalid("unknown envelope member"));
    }
    Ok(())
}
fn project(v: &Value, name: &str) -> Value {
    let Some((props, _)) = crate::catalog::catalog(name) else {
        return serde_json::json!({});
    };
    let mut result = Map::new();
    if let Some(m) = v.as_object() {
        for (k, val) in m {
            if let Some((_, typ)) = props.iter().find(|(p, _)| *p == k) {
                result.insert(
                    k.clone(),
                    if crate::catalog::catalog(typ).is_some() {
                        project(val, typ)
                    } else {
                        val.clone()
                    },
                );
            }
        }
    }
    Value::Object(result)
}
pub fn known_fields(v: &Value) -> Value {
    project(&v["fields"], v["kind"].as_str().unwrap_or("native"))
}
pub fn recognized(event: &Event, profile: Option<&dyn Profile>) -> Result<Recognized, Error> {
    let v = event.value();
    let family = v["family"].as_str().unwrap_or("");
    let kind = v["kind"].as_str().unwrap_or("native");
    let mut context = project(&v["context"], "context");
    if !matches!(
        context["direction"].as_str(),
        Some("from_application" | "to_application" | "unknown")
    ) {
        context["direction"] = "unknown".into();
    }
    if !matches!(
        context["exchange"].as_str(),
        Some("notification" | "request" | "response" | "unknown")
    ) {
        context["exchange"] = "unknown".into();
    }
    let profile = profile.filter(|p| p.family() == family);
    let verification = if let Some(p) = profile {
        p.verify(event)?
    } else {
        Verification::Unavailable
    };
    let native_version_supported = profile.is_none_or(|p| p.native_version_supported(event));
    let recognized_kind = if !official(family) || !known_kind(kind) || !native_version_supported {
        "native"
    } else {
        kind
    };
    let fields = if recognized_kind == "native" {
        serde_json::json!({})
    } else if let Some(p) = profile {
        p.fields(event)
    } else {
        known_fields(v)
    };
    Ok(Recognized {
        kind: recognized_kind.into(),
        fields,
        context,
        native: v.get("native").cloned(),
        verification,
    })
}
/// Parsed-value equality; floating fields compare correctly rounded binary64, including signed zero.
pub fn semantic_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter().all(|(k, v)| {
                    b.get(k).is_some_and(|w| {
                        if k == "dt_seconds" || k == "delta_time_seconds" {
                            match (v.as_f64(), w.as_f64()) {
                                (Some(x), Some(y)) => x == y,
                                _ => semantic_eq(v, w),
                            }
                        } else {
                            semantic_eq(v, w)
                        }
                    })
                })
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(v, w)| semantic_eq(v, w))
        }
        (Value::Number(a), Value::Number(b)) => decimal(a) == decimal(b),
        _ => a == b,
    }
}
