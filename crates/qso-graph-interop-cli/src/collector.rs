// SPDX-License-Identifier: GPL-3.0-or-later
//! Local, bounded observer of Annex H.1 rules. Time and exporter identity are caller supplied.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Codec, DecodeOptions, Error, Event, Outcome, pskreporter_ipfix::PskReporterIpfix,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    pub rule: &'static str,
    pub severity: &'static str,
    pub set_index: Option<usize>,
    pub record_index: Option<usize>,
    pub message: &'static str,
}
#[derive(Default)]
struct History {
    last_message: Option<u64>,
    templates: BTreeMap<u16, u64>,
    calls: BTreeMap<String, u64>,
    messages: u64,
}
#[derive(Default)]
#[non_exhaustive]
pub struct Collector {
    codec: PskReporterIpfix,
    histories: BTreeMap<(Vec<u8>, u32), History>,
    last_time: Option<u64>,
}
fn diagnostic(
    event: Option<&Event>,
    rule: &'static str,
    severity: &'static str,
    message: &'static str,
) -> Diagnostic {
    Diagnostic {
        rule,
        severity,
        message,
        set_index: event
            .and_then(|e| e.value()["native"]["set_index"].as_u64())
            .map(|v| v as usize),
        record_index: event
            .and_then(|e| e.value()["native"]["record_index"].as_u64())
            .map(|v| v as usize),
    }
}
fn error(message: &str) -> Error {
    Error::new(qso_graph_interop::Category::InvalidValue, message)
}
impl Collector {
    /// Inspect one datagram. `now_ms` is monotonic observer time, never the untrusted export time.
    pub fn observe(
        &mut self,
        bytes: &[u8],
        session: &[u8],
        now_ms: u64,
    ) -> Result<(Outcome, Vec<Diagnostic>), Error> {
        if self.last_time.is_some_and(|time| now_ms < time) {
            return Err(error("collector time moved backward"));
        }
        if session.len() > 128 {
            return Err(error("collector session key limit"));
        }
        let domain = bytes
            .get(12..16)
            .map(|v| u32::from_be_bytes(v.try_into().expect("four")))
            .unwrap_or(0);
        let key = (session.to_vec(), domain);
        if !self.histories.contains_key(&key) && self.histories.len() == 64 {
            return Err(error("collector history cap reached; reset the collector"));
        }
        let options = DecodeOptions::default()
            .with_session(session)
            .with_structured(true);
        let out = self.codec.decode(bytes, &options)?;
        if bytes.get(..2) != Some(&[0, 10]) {
            return Ok((
                out,
                vec![diagnostic(
                    None,
                    "H.1-FRAMING",
                    "notice",
                    "unsupported future IPFIX version skipped without interpreting Sets",
                )],
            ));
        }
        self.last_time = Some(now_ms);
        let history = self.histories.entry(key).or_default();
        let mut diagnostics = Vec::new();
        let mut templates_in_message = Vec::new();
        let mut offset = 16;
        while let Some(set) = bytes.get(offset..offset + 4) {
            let id = u16::from_be_bytes([set[0], set[1]]);
            let len = usize::from(u16::from_be_bytes([set[2], set[3]]));
            if len < 4 || offset + len > bytes.len() {
                break;
            }
            if id == 2 || id == 3 {
                // Template Records are bounded; their exact field lengths delimit each definition.
                let mut pos = offset + 4;
                while pos + 4 <= offset + len {
                    let tid = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
                    let count = usize::from(u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]));
                    pos += 4;
                    if count == 0 {
                        diagnostics.push(diagnostic(
                            None,
                            "H.1-WITHDRAWAL",
                            "violation",
                            "UDP withdrawals are ignored",
                        ));
                        continue;
                    }
                    if id == 3 {
                        pos += 2;
                    }
                    for _ in 0..count {
                        let raw = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
                        pos += if raw & 0x8000 != 0 { 8 } else { 4 };
                    }
                    templates_in_message.push(tid);
                    history.templates.insert(tid, now_ms);
                }
                if bytes[pos_tail(bytes, offset, len, id)..offset + len]
                    .iter()
                    .any(|b| *b != 0)
                {
                    diagnostics.push(diagnostic(
                        None,
                        "H.1-PADDING",
                        "violation",
                        "exporter Set padding must be zero",
                    ));
                }
            }
            offset += len;
        }
        history
            .calls
            .retain(|_, time| now_ms.saturating_sub(*time) < 300_000);
        let has_receiver = out
            .events
            .iter()
            .any(|e| e.kind_token() == "receiver_information");
        if !has_receiver
            && out.events.iter().all(|e| e.kind_token() != "spot")
            && out.skips.is_empty()
        {
            diagnostics.push(diagnostic(
                None,
                "H.1-RECEIVER",
                "violation",
                "include receiver information in each datagram",
            ));
        }
        let mut padding_checked = Vec::new();
        for event in &out.events {
            let n = &event.value()["native"];
            let f = &event.value()["fields"];
            let sid = n["set_id"].as_u64().expect("Set ID") as u16;
            let elements = n["message"]["elements"].as_array().expect("structured");
            if event.kind_token() == "native"
                && elements.iter().any(|e| {
                    e["enterprise"] == 30351
                        && [3, 5, 6, 7, 10, 11, 14, 15, 16]
                            .iter()
                            .any(|id| e["id"] == *id)
                })
            {
                diagnostics.push(diagnostic(
                    Some(event),
                    "H.1-REQUIRED",
                    "violation",
                    "PSK sender attributes lack senderCallsign; record retained as native",
                ));
            }
            let required: &[&str] = if event.kind_token() == "spot" {
                &["sender_call", "mode", "information_source", "time"]
            } else {
                &[]
            };
            for name in required {
                if f.get(*name)
                    .is_none_or(|v| v.is_null() || v.as_str() == Some(""))
                {
                    diagnostics.push(diagnostic(
                        Some(event),
                        "H.1-REQUIRED",
                        "violation",
                        "missing or malformed required field",
                    ));
                }
            }
            for element in elements {
                let pen = element["enterprise"].as_u64().unwrap_or(0);
                let id = element["id"].as_u64().unwrap_or(0);
                let data = element["data"].as_str().unwrap_or("");
                // Base64 character count detects native scalar widths without interpreting unknown IEs.
                let width =
                    data.len() / 4 * 3 - data.bytes().rev().take_while(|b| *b == b'=').count();
                let invalid = if pen == 30351 {
                    match id {
                        1..=4 | 8..=10 | 13 => text_invalid(id, f),
                        5 => !(1..=8).contains(&width),
                        6 | 7 => !(1..=8).contains(&width),
                        11 => width != 1,
                        15 => width != 2,
                        16 => width != 1 && width != 2,
                        _ => false,
                    }
                } else {
                    pen == 0 && id == 150 && width != 4
                };
                if invalid {
                    diagnostics.push(diagnostic(
                        Some(event),
                        "H.1-FIELDS",
                        "violation",
                        "known information element has malformed content or width",
                    ));
                }
            }
            if n["message"]["options"] == true && n["message"]["scope_fields"] == 0 {
                diagnostics.push(diagnostic(
                    Some(event),
                    "H.1-SCOPE",
                    "violation",
                    "legacy fldigi zero scope count departs from RFC 7011",
                ));
            }
            if event.kind_token() == "spot" {
                if !has_receiver {
                    diagnostics.push(diagnostic(
                        Some(event),
                        "H.1-RECEIVER",
                        "violation",
                        "include receiver information in each datagram",
                    ));
                }
                if has_receiver && n.get("dependencies").is_none() {
                    diagnostics.push(diagnostic(
                        Some(event),
                        "H.1-ASSOCIATION",
                        "notice",
                        "multiple receiver records; receiver association is unavailable",
                    ));
                }
                if let Some(source) = f["information_source"].as_u64()
                    && (source & 0x7c != 0 || source & 3 == 0)
                {
                    diagnostics.push(diagnostic(
                        Some(event),
                        "H.1-SOURCE",
                        "violation",
                        "informationSource has undefined source bits",
                    ));
                }
                if let Some(call) = f["sender_call"].as_str() {
                    if history
                        .calls
                        .get(call)
                        .is_some_and(|previous| now_ms.saturating_sub(*previous) < 300_000)
                    {
                        diagnostics.push(diagnostic(
                            Some(event),
                            "H.1-DUPLICATE",
                            "violation",
                            "callsign already observed within five minutes",
                        ));
                    }
                    if !history.calls.contains_key(call) && history.calls.len() == 4096 {
                        return Err(error("collector duplicate history cap reached"));
                    }
                    history.calls.insert(call.into(), now_ms);
                }
            }
            if history
                .templates
                .get(&sid)
                .is_some_and(|time| now_ms.saturating_sub(*time) > 3_600_000)
            {
                diagnostics.push(diagnostic(
                    Some(event),
                    "H.1-TEMPLATES",
                    "violation",
                    "used template was not observed within one hour",
                ));
            }
            if history.messages < 3 && !templates_in_message.contains(&sid) {
                diagnostics.push(diagnostic(
                    Some(event),
                    "H.1-STARTUP",
                    "recommendation",
                    "repeat used templates in the first three observed datagrams",
                ));
            }
            // Only flag intervals when every maximum-sized next source-profile record fits.
            let signature: Vec<_> = elements
                .iter()
                .map(|e| {
                    (
                        e["enterprise"].as_u64().unwrap_or(0),
                        e["id"].as_u64().unwrap_or(0),
                        e["length"].as_u64().unwrap_or(0),
                    )
                })
                .collect();
            let not_full = if signature
                == [
                    (30351, 1, 65535),
                    (30351, 5, 5),
                    (30351, 6, 1),
                    (30351, 10, 65535),
                    (30351, 3, 65535),
                    (30351, 11, 1),
                    (0, 150, 4),
                ] {
                bytes.len() <= 952 - 80
            } else if signature
                == [
                    (30351, 1, 65535),
                    (0, 150, 4),
                    (30351, 5, 4),
                    (30351, 10, 65535),
                    (30351, 3, 65535),
                    (30351, 11, 1),
                ]
            {
                bytes.len() <= 1454 - 776
            } else {
                false
            };
            if event.kind_token() == "spot"
                && not_full
                && history
                    .last_message
                    .is_some_and(|time| now_ms.saturating_sub(time) < 300_000)
            {
                diagnostics.push(diagnostic(
                    Some(event),
                    "H.1-INTERVAL",
                    "violation",
                    "datagrams less than five minutes apart without a full profile packet",
                ));
            }
            let si = n["set_index"].as_u64().expect("Set index") as usize;
            if !padding_checked.contains(&si) {
                padding_checked.push(si);
                let set_start = set_offset(bytes, si);
                let length = usize::from(u16::from_be_bytes([
                    bytes[set_start + 2],
                    bytes[set_start + 3],
                ]));
                let total: usize = out
                    .events
                    .iter()
                    .filter(|e| e.value()["native"]["set_index"] == n["set_index"])
                    .map(|e| e.native_bytes().expect("evidence").len())
                    .sum();
                if bytes[set_start + 4 + total..set_start + length]
                    .iter()
                    .any(|b| *b != 0)
                {
                    diagnostics.push(diagnostic(
                        Some(event),
                        "H.1-PADDING",
                        "violation",
                        "exporter Set padding must be zero",
                    ));
                }
            }
        }
        for skip in &out.skips {
            let mut d = diagnostic(
                None,
                "H.1-TEMPLATES",
                "notice",
                "unknown template; Set skipped with exact message evidence",
            );
            d.set_index = skip.set_index;
            diagnostics.push(d);
        }
        history.last_message = Some(now_ms);
        history.messages = history.messages.saturating_add(1);
        Ok((out, diagnostics))
    }
}
fn text_invalid(id: u64, fields: &serde_json::Value) -> bool {
    let name = match id {
        1 => "sender_call",
        2 => "receiver_call",
        3 => "sender_locator",
        4 => "receiver_locator",
        8 => "reporting_software",
        9 => "antenna_information",
        10 => "mode",
        13 => "rig_information",
        _ => return false,
    };
    let Some(value) = fields.get(name) else {
        return false;
    };
    let Some(value) = value.as_str() else {
        return true;
    };
    if matches!(id, 1 | 2) {
        return value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'/');
    }
    if matches!(id, 3 | 4) && !value.is_empty() {
        let b = value.as_bytes();
        return ![2, 4, 6, 8, 10, 12, 14, 16].contains(&b.len())
            || !b.iter().enumerate().all(|(i, b)| match i {
                0 | 1 => (b'A'..=b'R').contains(&b.to_ascii_uppercase()),
                _ if (i / 2) % 2 == 1 => b.is_ascii_digit(),
                _ => (b'A'..=b'X').contains(&b.to_ascii_uppercase()),
            });
    }
    id == 10
        && (value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'/' | b'+')))
}
fn set_offset(bytes: &[u8], index: usize) -> usize {
    let mut pos = 16;
    for _ in 0..index {
        pos += usize::from(u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]));
    }
    pos
}
fn pos_tail(bytes: &[u8], offset: usize, len: usize, id: u16) -> usize {
    let mut pos = offset + 4;
    while pos + 4 <= offset + len {
        let count = usize::from(u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]));
        pos += 4;
        if count == 0 {
            continue;
        }
        if id == 3 {
            pos += 2;
        }
        for _ in 0..count {
            let raw = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
            pos += if raw & 0x8000 != 0 { 8 } else { 4 };
        }
    }
    pos
}
