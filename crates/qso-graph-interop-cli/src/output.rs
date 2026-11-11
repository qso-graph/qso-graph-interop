// SPDX-License-Identifier: GPL-3.0-or-later
//! Presentation of decoded packets; wire values remain unchanged.
use qso_graph_interop::wsjtx::{Body, Outcome, QDateTime, QTime, Utf8};
use serde_json::{Map, Value, json};
use std::net::SocketAddr;

fn time(ms: u32) -> Option<String> {
    let ms = QTime(ms).milliseconds_since_midnight()?;
    Some(format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    ))
}

// Gregorian calendar conversion from days since 1970-01-01.
fn timestamp(ms: i64) -> Option<String> {
    let days = ms.div_euclid(86_400_000);
    let z = days.checked_add(719_468)?;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    if !(0..=9999).contains(&year) {
        return None;
    }
    let clock = time(ms.rem_euclid(86_400_000) as u32)?;
    Some(format!("{year:04}-{month:02}-{day:02}T{clock}Z"))
}

fn insert_text(map: &mut Map<String, Value>, key: &str, v: &Utf8) {
    match &v.bytes {
        None => {
            map.insert(key.into(), Value::Null);
        }
        Some(bytes) => {
            map.insert(
                key.into(),
                Value::String(String::from_utf8_lossy(bytes).into_owned()),
            );
            if std::str::from_utf8(bytes).is_err() {
                map.insert(format!("{key}_bytes"), json!(bytes));
            }
        }
    }
}

fn kind(body: &Body) -> &'static str {
    match body {
        Body::Heartbeat(_) => "heartbeat",
        Body::Status(_) => "status",
        Body::Decode(_) => "decode",
        Body::Clear(_) => "clear",
        Body::Reply(_) => "reply",
        Body::QsoLogged(_) => "qso_logged",
        Body::Close => "close",
        Body::Replay => "replay",
        Body::HaltTx(_) => "halt_tx",
        Body::FreeText(_) => "free_text",
        Body::WsprDecode(_) => "wspr_decode",
        Body::Location(_) => "location",
        Body::LoggedAdif(_) => "logged_adif",
        Body::HighlightCallsign(_) => "highlight_callsign",
        Body::SwitchConfiguration(_) => "switch_configuration",
        Body::Configure(_) => "configure",
        Body::AnnotationInfo(_) => "annotation_info",
    }
}

pub fn object(outcome: &Outcome, peer: SocketAddr, received_ms: i64) -> Value {
    let mut map = Map::new();
    map.insert("received".into(), json!(timestamp(received_ms)));
    map.insert("from".into(), json!(peer.to_string()));
    match outcome {
        Outcome::Skipped {
            schema,
            message_type,
            reason,
        } => {
            map.insert("type".into(), json!("skipped"));
            map.insert("schema".into(), json!(schema));
            map.insert("client_id".into(), Value::Null);
            map.insert("message_type".into(), json!(message_type));
            map.insert(
                "reason".into(),
                json!(match reason {
                    qso_graph_interop::wsjtx::SkipReason::FutureSchema => "future_schema",
                    qso_graph_interop::wsjtx::SkipReason::UnknownType => "unknown_type",
                }),
            );
        }
        Outcome::Message(packet) => {
            map.insert("type".into(), json!(kind(&packet.body)));
            map.insert("schema".into(), json!(packet.schema));
            insert_text(&mut map, "client_id", &packet.id);
            let serialized = serde_json::to_value(&packet.body).expect("body serializes");
            if let Value::Object(outer) = serialized
                && let Some(Value::Object(fields)) = outer.into_values().next()
            {
                for (key, value) in fields {
                    if let Value::Object(obj) = &value {
                        if let Some(bytes) = obj.get("bytes") {
                            let text: Utf8 = serde_json::from_value(json!({"bytes": bytes}))
                                .expect("UTF-8 field");
                            insert_text(&mut map, &key, &text);
                            continue;
                        }
                        if obj.contains_key("julian_day") {
                            let date: QDateTime =
                                serde_json::from_value(value).expect("date field");
                            map.insert(
                                key,
                                json!(date.unix_milliseconds(packet.schema).and_then(timestamp)),
                            );
                            continue;
                        }
                    }
                    if key == "time" {
                        map.insert(
                            key,
                            json!(
                                value
                                    .as_u64()
                                    .and_then(|v| u32::try_from(v).ok())
                                    .and_then(time)
                            ),
                        );
                    } else {
                        map.insert(key, value);
                    }
                }
            }
        }
    }
    Value::Object(map)
}

fn safe(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_control()
                || matches!(c,
                    '\u{ad}' | '\u{61c}' | '\u{200b}'..='\u{200f}' |
                    '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{206f}' |
                    '\u{feff}' | '\u{fff9}'..='\u{fffb}')
            {
                c.escape_default().collect::<String>()
            } else {
                c.to_string()
            }
            .chars()
            .collect::<Vec<_>>()
        })
        .collect()
}
fn field(map: &Map<String, Value>, key: &str) -> String {
    if let Some(Value::Array(bytes)) = map.get(&format!("{key}_bytes")) {
        let bytes: Vec<u8> = bytes
            .iter()
            .filter_map(|v| v.as_u64().and_then(|v| u8::try_from(v).ok()))
            .collect();
        let mut out = String::new();
        let mut rest = bytes.as_slice();
        while !rest.is_empty() {
            match std::str::from_utf8(rest) {
                Ok(s) => {
                    out.push_str(&safe(s));
                    break;
                }
                Err(e) => {
                    let n = e.valid_up_to();
                    if let Ok(s) = std::str::from_utf8(&rest[..n]) {
                        out.push_str(&safe(s));
                    }
                    let bad = e.error_len().unwrap_or(rest.len() - n);
                    for b in &rest[n..n + bad] {
                        out.push_str(&format!("\\x{b:02x}"));
                    }
                    rest = &rest[n + bad..];
                }
            }
        }
        return out;
    }
    match map.get(key) {
        Some(Value::String(s)) => safe(s),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}
fn mhz(map: &Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_u64)
        .map(|v| format!("{}.{:06} MHz", v / 1_000_000, v % 1_000_000))
        .unwrap_or_default()
}
fn add(parts: &mut Vec<String>, s: String) {
    if !s.is_empty() {
        parts.push(s);
    }
}

pub fn human(value: &Value, verbose: bool) -> String {
    let m = value.as_object().expect("object");
    let typ = field(m, "type");
    let label = typ
        .split('_')
        .map(|word| {
            let mut c = word.chars();
            c.next()
                .map(|v| v.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ");
    let label = label
        .replace("Qso", "QSO")
        .replace("Wspr", "WSPR")
        .replace("Adif", "ADIF")
        .replace("Tx", "TX")
        .replace("Cq", "CQ");
    let mut parts = Vec::new();
    match typ.as_str() {
        "heartbeat" => {
            add(&mut parts, field(m, "client_id"));
            add(&mut parts, field(m, "revision"));
            add(&mut parts, field(m, "version"));
            if !field(m, "max_schema").is_empty() {
                parts.push(format!("(schema {})", field(m, "max_schema")));
            }
        }
        "status" => {
            add(&mut parts, mhz(m, "dial_frequency_hz"));
            add(&mut parts, field(m, "mode"));
            parts.push(
                if m.get("transmitting") == Some(&json!(true)) {
                    "TX"
                } else {
                    "RX"
                }
                .into(),
            );
            let station = [field(m, "de_call"), field(m, "de_grid")]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !station.is_empty() {
                parts.push(format!("DE {station}"));
            }
            let dx = [field(m, "dx_call"), field(m, "dx_grid")]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            if !dx.is_empty() {
                parts.push(format!("DX {dx}"));
            }
            let report = field(m, "report");
            if !report.is_empty() {
                parts.push(format!("report {report}"));
            }
        }
        "decode" | "reply" => {
            parts.push(format!("{} dB", field(m, "snr_db")));
            parts.push(format!("{} s", field(m, "delta_time_seconds")));
            parts.push(format!("{} Hz", field(m, "delta_frequency_hz")));
            add(&mut parts, field(m, "mode"));
            add(&mut parts, field(m, "message"));
        }
        "qso_logged" => {
            for key in ["dx_call", "dx_grid"] {
                add(&mut parts, field(m, key));
            }
            add(&mut parts, mhz(m, "frequency_hz"));
            add(&mut parts, field(m, "mode"));
            for (key, prefix) in [("report_sent", "sent"), ("report_received", "rcvd")] {
                let v = field(m, key);
                if !v.is_empty() {
                    parts.push(format!("{prefix} {v}"));
                }
            }
            add(&mut parts, field(m, "time_off").replace('T', " "));
            for (key, label) in [
                ("tx_power", "power"),
                ("name", "name"),
                ("comments", "comments"),
                ("exchange_sent", "exchange sent"),
                ("exchange_received", "exchange received"),
                ("propagation_mode", "propagation"),
            ] {
                let v = field(m, key);
                if !v.is_empty() {
                    parts.push(format!("{label} {v}"));
                }
            }
        }
        _ => {
            for key in m.keys() {
                if !["type", "received", "from", "schema", "client_id"].contains(&key.as_str())
                    && !key.ends_with("_bytes")
                {
                    let v = field(m, key);
                    if !v.is_empty() {
                        parts.push(format!("{key}={v}"));
                    }
                }
            }
        }
    }
    let received = field(m, "received");
    let clock = received.get(11..19).unwrap_or("??:??:??");
    let mut line = format!("{clock}  {label:<20} {}", parts.join("  "))
        .trim_end()
        .to_string();
    if verbose {
        line.push_str(&format!(
            "  [{} schema {} id {}]",
            field(m, "from"),
            field(m, "schema"),
            field(m, "client_id")
        ));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use qso_graph_interop::wsjtx::{self, Body, Packet};

    #[test]
    fn hostile_text_is_escaped_and_json_preserves_bytes() {
        let packet = Packet::new(
            "test",
            Body::FreeText(wsjtx::FreeText {
                text: Utf8 {
                    bytes: Some(vec![b'A', 0x1b, b'\n', 0xff]),
                },
                send: true,
            }),
        );
        let result = object(
            &Outcome::Message(Box::new(packet)),
            "127.0.0.1:1234".parse().unwrap(),
            0,
        );
        let line = human(&result, true);
        assert!(line.contains(r"A\u{1b}\n\xff"), "{line}");
        assert!(!line.contains('\x1b'));
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
        assert_eq!(parsed["text"], "A\u{1b}\n\u{fffd}");
        assert_eq!(parsed["text_bytes"], json!([65, 27, 10, 255]));
        assert_eq!(parsed["send"], true);
        assert_eq!(parsed["received"], "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn unicode_format_controls_are_visible_but_json_and_wire_are_exact() {
        let text = "A\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}\u{200e}\u{200f}\u{61c}\u{2028}\u{2029}\u{ad}\u{200b}\u{200c}\u{200d}\u{2060}\u{2061}\u{2062}\u{2063}\u{2064}\u{206a}\u{206f}\u{feff}\u{fff9}\u{fffa}\u{fffb}Z";
        let packet = Packet::new(
            text,
            Body::FreeText(wsjtx::FreeText {
                text: text.into(),
                send: true,
            }),
        );
        let bytes = wsjtx::encode(&packet).unwrap();
        let outcome = wsjtx::decode(&bytes).unwrap();
        let Outcome::Message(ref decoded) = outcome else {
            panic!("packet");
        };
        assert_eq!(wsjtx::encode(decoded).unwrap(), bytes);
        let value = object(&outcome, "127.0.0.1:1".parse().unwrap(), 0);
        let json: Value = serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(json["text"], text);
        assert_eq!(json["client_id"], text);
        let line = human(&value, true);
        for c in text.chars().filter(|c| !c.is_ascii()) {
            assert!(!line.contains(c), "raw format control {c:?}");
            assert!(
                line.contains(&c.escape_default().to_string()),
                "missing visible escape {c:?}"
            );
        }
    }

    #[test]
    fn review_osc_invalid_utf8_and_bidi_probe_is_safe() {
        let mut bytes = b"A\x1b]52;c;QQ==\x07\r\n\0\xff".to_vec();
        bytes.extend_from_slice("\u{202e}abc\u{202c}\u{2028}next".as_bytes());
        let packet = Packet::new(
            Utf8 {
                bytes: Some(bytes.clone()),
            },
            Body::FreeText(wsjtx::FreeText {
                text: Utf8 {
                    bytes: Some(bytes.clone()),
                },
                send: true,
            }),
        );
        let wire = wsjtx::encode(&packet).unwrap();
        let outcome = wsjtx::decode(&wire).unwrap();
        let value = object(&outcome, "127.0.0.1:1".parse().unwrap(), 0);
        let json: Value = serde_json::from_str(&value.to_string()).unwrap();
        assert_eq!(json["text_bytes"], json!(bytes));
        assert_eq!(json["client_id_bytes"], json!(bytes));
        let line = human(&value, true);
        for c in [
            '\u{1b}', '\u{7}', '\r', '\n', '\0', '\u{202e}', '\u{202c}', '\u{2028}',
        ] {
            assert!(!line.contains(c), "unescaped {c:?}");
        }
        assert!(line.contains(r"\xff\u{202e}abc\u{202c}\u{2028}next"));
        let Outcome::Message(packet) = outcome else {
            panic!("packet");
        };
        assert_eq!(wsjtx::encode(&packet).unwrap(), wire);
    }

    #[test]
    fn calendar_and_invalid_times() {
        assert_eq!(
            timestamp(1_762_300_800_000).as_deref(),
            Some("2025-11-05T00:00:00.000Z")
        );
        assert_eq!(timestamp(-1).as_deref(), Some("1969-12-31T23:59:59.999Z"));
        assert_eq!(time(86_400_000), None);
    }
}
