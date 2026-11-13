// SPDX-License-Identifier: GPL-3.0-or-later
//! Annex D's bounded XML 1.0 tokenizer. No I/O, DTDs, or entity resolver exists.
#![forbid(unsafe_code)]
use crate::{Category, Error, Text, error::oversized};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn malformed() -> Error {
    Error::new(Category::Malformed, "invalid bounded XML")
}
fn truncated() -> Error {
    Error::new(Category::Truncated, "unfinished bounded XML")
}
pub(crate) fn legal(c: char) -> bool {
    matches!(c as u32, 9 | 10 | 13 | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}
pub(crate) fn space(c: u16) -> bool {
    matches!(c, 9 | 10 | 13 | 32)
}
#[derive(Clone, Copy)]
enum Encoding {
    Utf8,
    Le,
    Be,
}
impl Encoding {
    fn width(self) -> usize {
        if matches!(self, Self::Utf8) { 1 } else { 2 }
    }
    fn string(self, units: &[u16]) -> Option<String> {
        if matches!(self, Self::Utf8) {
            String::from_utf8(units.iter().map(|&u| u as u8).collect()).ok()
        } else {
            String::from_utf16(units).ok()
        }
    }
    fn value(self, units: &[u16]) -> Value {
        if matches!(self, Self::Utf8) {
            Text::utf8(Some(&units.iter().map(|&u| u as u8).collect::<Vec<_>>())).value()
        } else {
            Text::utf16(Some(units)).value()
        }
    }
    fn push(self, c: char, out: &mut Vec<u16>) {
        if matches!(self, Self::Utf8) {
            let mut buf = [0; 4];
            out.extend(c.encode_utf8(&mut buf).bytes().map(u16::from));
        } else {
            let mut buf = [0; 2];
            out.extend_from_slice(c.encode_utf16(&mut buf));
        }
    }
    // Valid scalar, or one original invalid code unit, plus consumed unit count.
    fn scalar(self, units: &[u16]) -> (Option<char>, usize) {
        if matches!(self, Self::Utf8) {
            let b = units[0] as u8;
            let n = match b {
                0..=0x7f => 1,
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => return (None, 1),
            };
            if units.len() < n {
                return (None, 1);
            }
            let bytes: Vec<_> = units[..n].iter().map(|&u| u as u8).collect();
            let scalar = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|s| s.chars().next());
            (scalar, if scalar.is_some() { n } else { 1 })
        } else {
            let a = units[0];
            if (0xd800..=0xdbff).contains(&a) {
                if let Some(&b) = units.get(1).filter(|&&b| (0xdc00..=0xdfff).contains(&b)) {
                    (
                        char::from_u32(
                            0x10000 + ((u32::from(a) - 0xd800) << 10) + u32::from(b) - 0xdc00,
                        ),
                        2,
                    )
                } else {
                    (None, 1)
                }
            } else {
                (char::from_u32(u32::from(a)), 1)
            }
        }
    }
}
#[derive(Debug)]
pub(crate) struct Node {
    pub name: String,
    pub plain: bool,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Node>,
    pub text: Value,
}
#[cfg(feature = "n1mm")]
impl Node {
    pub fn named(&self, name: &str) -> impl Iterator<Item = &Node> {
        self.children
            .iter()
            .filter(move |n| n.plain && n.name == name)
    }
    pub fn one(&self, name: &str) -> Option<&Node> {
        let mut iter = self.named(name);
        let n = iter.next()?;
        iter.next().is_none().then_some(n)
    }
    pub fn leaf(&self, name: &str) -> Option<&Value> {
        self.one(name)
            .filter(|n| n.children.is_empty())
            .map(|n| &n.text)
    }
}
pub(crate) struct Document {
    pub root: Node,
    pub xml: Value,
}
struct Parser {
    units: Vec<u16>,
    encoding: Encoding,
    pos: usize,
    elements: usize,
    attributes: usize,
    text_bytes: usize,
    limits: XmlLimits,
}
impl Parser {
    fn at(&self, s: &str) -> bool {
        self.at_pos(self.pos, s)
    }
    fn at_pos(&self, pos: usize, s: &str) -> bool {
        self.units
            .get(pos..pos + s.len())
            .is_some_and(|u| u.iter().copied().eq(s.bytes().map(u16::from)))
    }
    fn end(&self, from: usize, s: &str) -> Result<usize, Error> {
        (from..self.units.len())
            .find(|&p| self.at_pos(p, s))
            .ok_or_else(truncated)
    }
    fn ws(&mut self) -> bool {
        let start = self.pos;
        while self.units.get(self.pos).is_some_and(|&u| space(u)) {
            self.pos += 1;
        }
        self.pos != start
    }
    fn strict(&self, units: &[u16]) -> Result<String, Error> {
        self.encoding
            .string(units)
            .filter(|s| s.chars().all(legal))
            .ok_or_else(malformed)
    }
    fn name(&mut self) -> Result<(String, usize), Error> {
        let start = self.pos;
        while self
            .units
            .get(self.pos)
            .is_some_and(|&u| !space(u) && !matches!(u, 47 | 62 | 61 | 63))
        {
            self.pos += 1;
        }
        if self.pos == self.units.len() {
            return Err(truncated());
        }
        let name = self.strict(&self.units[start..self.pos])?;
        let mut parts = name.split(':');
        if !parts.all(valid_name) || name.matches(':').count() > 1 {
            return Err(malformed());
        }
        Ok((name, (self.pos - start) * self.encoding.width()))
    }
    fn processed(&self, raw: &[u16], refs: bool, attribute: bool) -> Result<Vec<u16>, Error> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < raw.len() {
            if refs && raw[i] == 38 {
                let end = raw[i..]
                    .iter()
                    .position(|&u| u == 59)
                    .ok_or_else(truncated)?
                    + i;
                let token = self.strict(&raw[i + 1..end])?;
                let ch = match token.as_str() {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "apos" => '\'',
                    "quot" => '"',
                    _ => {
                        let (digits, radix) = if let Some(s) = token.strip_prefix("#x") {
                            (s, 16)
                        } else if let Some(s) = token.strip_prefix('#') {
                            (s, 10)
                        } else {
                            return Err(malformed());
                        };
                        if digits.is_empty()
                            || !digits.bytes().all(|b| {
                                if radix == 16 {
                                    b.is_ascii_hexdigit()
                                } else {
                                    b.is_ascii_digit()
                                }
                            })
                        {
                            return Err(malformed());
                        }
                        u32::from_str_radix(digits, radix)
                            .ok()
                            .and_then(char::from_u32)
                            .filter(|&c| legal(c))
                            .ok_or_else(malformed)?
                    }
                };
                self.encoding.push(ch, &mut out);
                i = end + 1;
            } else if raw[i] == 13 {
                out.push(if attribute { 32 } else { 10 });
                i += 1;
                if raw.get(i) == Some(&10) {
                    i += 1;
                }
            } else {
                out.push(if attribute && matches!(raw[i], 9 | 10) {
                    32
                } else {
                    raw[i]
                });
                i += 1;
            }
        }
        Ok(out)
    }
    fn budget(&mut self, units: &[u16], allow_invalid: bool) -> Result<(), Error> {
        let mut i = 0;
        while i < units.len() {
            let (c, n) = self.encoding.scalar(&units[i..]);
            let size = match c {
                Some(c) if legal(c) => c.len_utf8(),
                None if allow_invalid => self.encoding.width(),
                _ => return Err(malformed()),
            };
            if self.text_bytes + size > self.limits.bytes {
                return Err(oversized("bounded decoded text budget"));
            }
            self.text_bytes += size;
            i += n;
        }
        Ok(())
    }
    // Character data is admitted as consumed, so an earlier expansion overflow wins
    // over a later malformed or unfinished reference in the same text node.
    fn character_data(&mut self, start: usize, end: usize) -> Result<Vec<u16>, Error> {
        let mut out = Vec::new();
        let mut at = start;
        while at < end {
            if self.at_pos(at, "]]>") {
                return Err(malformed());
            }
            let count = if self.units[at] == 38 {
                self.units[at..end]
                    .iter()
                    .position(|&u| u == 59)
                    .map(|n| n + 1)
                    .ok_or_else(|| {
                        if end == self.units.len() {
                            truncated()
                        } else {
                            malformed()
                        }
                    })?
            } else if self.units[at] == 13 && self.units.get(at + 1) == Some(&10) {
                2
            } else {
                self.encoding.scalar(&self.units[at..end]).1
            };
            let text = self.processed(&self.units[at..at + count], true, false)?;
            self.budget(&text, true)?;
            out.extend(text);
            at += count;
        }
        Ok(out)
    }
    fn comment(&mut self) -> Result<(), Error> {
        let end = self.end(self.pos + 4, "-->")?;
        let text = self.strict(&self.units[self.pos + 4..end])?;
        if text.contains("--") || text.ends_with('-') {
            return Err(malformed());
        }
        self.pos = end + 3;
        Ok(())
    }
    fn pi(&mut self) -> Result<(), Error> {
        let end = self.end(self.pos + 2, "?>")?;
        let text = self.strict(&self.units[self.pos + 2..end])?;
        let target = text.split([' ', '\t', '\r', '\n']).next().unwrap_or("");
        let mut chars = target.chars();
        let valid = chars.next().is_some_and(|c| c == ':' || name_start(c))
            && chars.all(|c| c == ':' || name_char(c));
        if !valid || target.eq_ignore_ascii_case("xml") {
            return Err(malformed());
        }
        let bytes = if matches!(self.encoding, Encoding::Utf8) {
            target.len()
        } else {
            target.encode_utf16().count() * 2
        };
        if bytes > 128 {
            return Err(oversized("bounded processing instruction name"));
        }
        self.pos = end + 2;
        Ok(())
    }
    // Isolation precedes structural/resource faults anywhere in the document. Skip lexical
    // comments, CDATA, PI and quoted tag values, without allocating or decoding their contents.
    fn isolation(&self) -> Result<(), Error> {
        let mut i = self.pos;
        while i < self.units.len() {
            if self.at_pos(i, "<!--") || self.at_pos(i, "<![CDATA[") || self.at_pos(i, "<?") {
                let (offset, end) = if self.at_pos(i, "<!--") {
                    (4, "-->")
                } else if self.at_pos(i, "<?") {
                    (2, "?>")
                } else {
                    (9, "]]>")
                };
                match self.end(i + offset, end) {
                    Ok(p) => i = p + end.len(),
                    Err(_) => break,
                }
            } else if self.at_pos(i, "<!DOCTYPE") || self.at_pos(i, "<!ENTITY") {
                return Err(malformed());
            } else if self.units[i] == 60 {
                i += 1;
                let mut quote = None;
                while i < self.units.len() {
                    let u = self.units[i];
                    i += 1;
                    if let Some(q) = quote {
                        if u == q {
                            quote = None;
                        }
                    } else if matches!(u, 34 | 39) {
                        quote = Some(u);
                    } else if u == 62 {
                        break;
                    }
                }
            } else {
                i += 1;
            }
        }
        Ok(())
    }
    fn declaration(&mut self) -> Result<(), Error> {
        if !self.at("<?xml")
            || self
                .units
                .get(self.pos + 5)
                .is_some_and(|&u| !space(u) && u != 63)
        {
            return Ok(());
        }
        let end = self.end(self.pos + 5, "?>")?;
        self.strict(&self.units[self.pos..end])?;
        self.pos += 5;
        let mut attributes = Vec::new();
        while self.pos < end {
            if !self.ws() {
                return Err(malformed());
            }
            if self.pos == end {
                break;
            }
            let (name, _) = self.name()?;
            self.ws();
            if self.units.get(self.pos) != Some(&61) {
                return Err(malformed());
            }
            self.pos += 1;
            self.ws();
            let value = self.quoted()?;
            attributes.push((name, self.strict(&value)?));
        }
        if self.pos != end {
            return Err(malformed());
        }
        let mut iter = attributes.iter();
        let Some((name, version)) = iter.next() else {
            return Err(malformed());
        };
        if name != "version"
            || !version
                .strip_prefix("1.")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(malformed());
        }
        let mut encoding = None;
        let mut standalone = false;
        for (name, value) in iter {
            match name.as_str() {
                "encoding" if encoding.is_none() && !standalone => {
                    if !value
                        .as_bytes()
                        .first()
                        .is_some_and(|b| b.is_ascii_alphabetic())
                        || !value
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
                    {
                        return Err(malformed());
                    }
                    encoding = Some(value.to_ascii_lowercase());
                }
                "standalone" if !standalone && matches!(value.as_str(), "yes" | "no") => {
                    standalone = true
                }
                _ => return Err(malformed()),
            }
        }
        if version != "1.0" {
            return Err(Error::new(
                Category::UnsupportedVersion,
                "unsupported XML version",
            ));
        }
        if let Some(label) = encoding {
            if !matches!(label.as_str(), "utf-8" | "utf-16" | "utf-16le" | "utf-16be") {
                return Err(Error::new(
                    Category::UnsupportedVersion,
                    "unsupported XML encoding",
                ));
            }
            let agree = match self.encoding {
                Encoding::Utf8 => label == "utf-8",
                Encoding::Le => matches!(label.as_str(), "utf-16" | "utf-16le"),
                Encoding::Be => matches!(label.as_str(), "utf-16" | "utf-16be"),
            };
            if !agree {
                return Err(malformed());
            }
        }
        self.pos = end + 2;
        Ok(())
    }
    fn quoted(&mut self) -> Result<Vec<u16>, Error> {
        let quote = *self.units.get(self.pos).ok_or_else(truncated)?;
        if !matches!(quote, 34 | 39) {
            return Err(malformed());
        }
        self.pos += 1;
        let start = self.pos;
        while let Some(&u) = self.units.get(self.pos) {
            if u == quote {
                let v = self.units[start..self.pos].to_vec();
                self.pos += 1;
                return Ok(v);
            }
            if u == 60 {
                return Err(malformed());
            }
            self.pos += 1;
        }
        Err(truncated())
    }
    fn node(&mut self, depth: usize, inherited: &BTreeMap<String, String>) -> Result<Node, Error> {
        // Establish a complete lexical start tag before syntax and resource admission.
        // A slash at EOF in '<RadioInfo/' is unfinished, rather than malformed.
        let mut end = self.pos + 1;
        let mut quote = None;
        loop {
            let unit = *self.units.get(end).ok_or_else(truncated)?;
            if let Some(q) = quote {
                if unit == q {
                    quote = None;
                }
            } else if matches!(unit, 34 | 39) {
                quote = Some(unit);
            } else if unit == 62 {
                break;
            }
            end += 1;
        }
        self.pos += 1;
        let (name, name_bytes) = self.name()?;
        let mut names = vec![name_bytes];
        let mut raw_attrs = Vec::new();
        let mut empty = false;
        loop {
            let separated = self.ws();
            if self.at("/>") {
                self.pos += 2;
                empty = true;
                break;
            }
            if self.at(">") {
                self.pos += 1;
                break;
            }
            if self.pos == self.units.len() {
                return Err(truncated());
            }
            if !separated {
                return Err(malformed());
            }
            let (key, bytes) = self.name()?;
            names.push(bytes);
            self.ws();
            if self.units.get(self.pos) != Some(&61) {
                return Err(malformed());
            }
            self.pos += 1;
            self.ws();
            let raw = self.quoted()?;
            let value = self.processed(&raw, true, true).map_err(|e| {
                if e.category() == Category::Truncated {
                    malformed()
                } else {
                    e
                }
            })?;
            let text = self.strict(&value)?;
            if raw_attrs.iter().any(|(k, _, _)| k == &key) {
                return Err(malformed());
            }
            raw_attrs.push((key, text, value));
        }
        let mut ns = inherited.clone();
        for (key, value, _) in &raw_attrs {
            let prefix = if key == "xmlns" {
                Some("")
            } else {
                key.strip_prefix("xmlns:")
            };
            if let Some(prefix) = prefix {
                if prefix == "xmlns"
                    || value == "http://www.w3.org/2000/xmlns/"
                    || (prefix == "xml") != (value == "http://www.w3.org/XML/1998/namespace")
                    || !prefix.is_empty() && value.is_empty()
                {
                    return Err(malformed());
                }
                ns.insert(prefix.into(), value.clone());
            }
        }
        let resolve = |key: &str, attribute: bool| -> Result<(String, String), Error> {
            if let Some((prefix, local)) = key.split_once(':') {
                if prefix == "xmlns" {
                    return Err(malformed());
                }
                Ok((
                    ns.get(prefix)
                        .cloned()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(malformed)?,
                    local.into(),
                ))
            } else {
                Ok((
                    if attribute {
                        String::new()
                    } else {
                        ns.get("").cloned().unwrap_or_default()
                    },
                    key.into(),
                ))
            }
        };
        let plain = resolve(&name, false)?.0.is_empty() && !name.contains(':');
        let mut expanded = BTreeSet::new();
        let mut attrs = BTreeMap::new();
        for (key, value, _) in &raw_attrs {
            if key == "xmlns" || key.starts_with("xmlns:") {
                continue;
            }
            if !expanded.insert(resolve(key, true)?) {
                return Err(malformed());
            }
            attrs.insert(key.clone(), value.clone());
        }
        // Complete lexical start-tag syntax and namespace constraints precede its resources.
        if depth > 16 {
            return Err(oversized("bounded XML depth"));
        }
        if self.elements == self.limits.elements {
            return Err(oversized("bounded XML element count"));
        }
        self.elements += 1;
        if raw_attrs.len() > 16 || self.attributes + raw_attrs.len() > 256 {
            return Err(oversized("bounded XML attribute count"));
        }
        self.attributes += raw_attrs.len();
        if names.iter().any(|&n| n > 128) {
            return Err(oversized("bounded XML lexical name"));
        }
        for (_, _, value) in &raw_attrs {
            self.budget(value, false)?;
        }
        let mut children = Vec::new();
        let mut content = Vec::new();
        if !empty {
            loop {
                if self.pos == self.units.len() {
                    return Err(truncated());
                }
                if self.at("</") {
                    self.pos += 2;
                    let (close, bytes) = self.name()?;
                    self.ws();
                    if !self.at(">") {
                        return Err(if self.pos == self.units.len() {
                            truncated()
                        } else {
                            malformed()
                        });
                    }
                    self.pos += 1;
                    if close != name {
                        return Err(malformed());
                    }
                    if bytes > 128 {
                        return Err(oversized("bounded XML lexical name"));
                    }
                    break;
                } else if self.at("<!--") {
                    self.comment()?;
                } else if self.at("<?") {
                    self.pi()?;
                } else if self.at("<![CDATA[") {
                    let end = self.end(self.pos + 9, "]]>")?;
                    let text = self.processed(&self.units[self.pos + 9..end], false, false)?;
                    self.budget(&text, true)?;
                    content.extend(text);
                    self.pos = end + 3;
                } else if self.at("<!") {
                    return Err(malformed());
                } else if self.at("<") {
                    children.push(self.node(depth + 1, &ns)?);
                } else {
                    let start = self.pos;
                    while self.units.get(self.pos).is_some_and(|&u| u != 60) {
                        self.pos += 1;
                    }
                    let text = self.character_data(start, self.pos)?;
                    content.extend(text);
                }
            }
        }
        Ok(Node {
            name,
            plain,
            attrs,
            children,
            text: self.encoding.value(&content),
        })
    }
}
fn valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    name_start(first) && chars.all(name_char)
}
fn name_char(c: char) -> bool {
    name_start(c)
        || c.is_ascii_digit()
        || matches!(c as u32, 0x2d|0x2e|0xb7|0x300..=0x36f|0x203f..=0x2040)
}
fn name_start(c: char) -> bool {
    c.is_ascii_alphabetic()
        || c == '_'
        || matches!(c as u32, 0xc0..=0xd6|0xd8..=0xf6|0xf8..=0x2ff|0x370..=0x37d|0x37f..=0x1fff|0x200c..=0x200d|0x2070..=0x218f|0x2c00..=0x2fef|0x3001..=0xd7ff|0xf900..=0xfdcf|0xfdf0..=0xfffd|0x10000..=0xeffff)
}
#[derive(Clone, Copy)]
pub(crate) struct XmlLimits {
    pub bytes: usize,
    pub elements: usize,
    pub declaration_eof_malformed: bool,
}
pub(crate) fn parse_with(bytes: &[u8], limits: XmlLimits) -> Result<Document, Error> {
    if bytes.len() > limits.bytes {
        return Err(oversized("bounded datagram bound"));
    }
    let (encoding, skip) = if bytes.starts_with(&[0xff, 0xfe]) {
        (Encoding::Le, 2)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (Encoding::Be, 2)
    } else if bytes.starts_with(&[0x3c, 0, 0x3f, 0]) {
        (Encoding::Le, 0)
    } else if bytes.starts_with(&[0, 0x3c, 0, 0x3f]) {
        (Encoding::Be, 0)
    } else {
        (
            Encoding::Utf8,
            if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                3
            } else {
                0
            },
        )
    };
    let bytes = &bytes[skip..];
    if encoding.width() == 2 && !bytes.len().is_multiple_of(2) {
        return Err(truncated());
    }
    let units = match encoding {
        Encoding::Utf8 => bytes.iter().map(|&b| u16::from(b)).collect(),
        Encoding::Le => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect(),
        Encoding::Be => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .collect(),
    };
    let mut p = Parser {
        units,
        encoding,
        pos: 0,
        elements: 0,
        attributes: 0,
        text_bytes: 0,
        limits,
    };
    p.declaration().map_err(|e| {
        if limits.declaration_eof_malformed && e.category() == Category::Truncated {
            malformed()
        } else {
            e
        }
    })?;
    p.isolation()?;
    let mut root = None;
    while p.pos < p.units.len() {
        if p.at("<!--") {
            p.comment()?;
        } else if p.at("<?") {
            p.pi()?;
        } else if p.at("<!") || p.at("</") {
            return Err(malformed());
        } else if p.at("<") {
            if root.is_some() {
                return Err(malformed());
            }
            let ns =
                BTreeMap::from([("xml".into(), "http://www.w3.org/XML/1998/namespace".into())]);
            root = Some(p.node(1, &ns)?);
        } else {
            let start = p.pos;
            while p.units.get(p.pos).is_some_and(|&u| u != 60) {
                p.pos += 1;
            }
            if !p.units[start..p.pos].iter().all(|&u| space(u)) {
                return Err(malformed());
            }
            let text = p.processed(&p.units[start..p.pos], false, false)?;
            p.budget(&text, false)?;
        }
    }
    Ok(Document {
        root: root.ok_or_else(truncated)?,
        xml: encoding.value(&p.units),
    })
}
