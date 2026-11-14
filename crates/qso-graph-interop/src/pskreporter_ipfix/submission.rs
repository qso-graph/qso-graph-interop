// SPDX-License-Identifier: GPL-3.0-or-later
//! Validated event-to-submission writers; scheduling belongs to the application.
use super::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubmissionProfile {
    #[default]
    Wsjtx,
    Fldigi,
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SubmissionOptions {
    pub profile: SubmissionProfile,
    pub export_time: u32,
    pub sequence_number: u32,
    pub observation_domain_id: u32,
    pub include_templates: bool,
    pub fldigi_short: bool,
}
impl Default for SubmissionOptions {
    fn default() -> Self {
        Self {
            profile: SubmissionProfile::Wsjtx,
            export_time: 0,
            sequence_number: 0,
            observation_domain_id: 0,
            include_templates: true,
            fldigi_short: false,
        }
    }
}
fn text(fields: &Value, name: &str, max: usize, required: bool) -> Result<Vec<u8>, Error> {
    if fields.get(name).is_some_and(|v| {
        v.as_str()
            .map(str::len)
            .or_else(|| v.get("utf8_bytes").and_then(Value::as_array).map(Vec::len))
            .is_some_and(|len| len > max)
    }) {
        return Err(invalid("IPFIX submission text length"));
    }
    let bytes = match fields.get(name) {
        Some(Value::String(s)) => s.as_bytes().to_vec(),
        Some(v) => v
            .get("utf8_bytes")
            .and_then(Value::as_array)
            .and_then(|a| {
                a.iter()
                    .map(|v| v.as_u64().and_then(|v| u8::try_from(v).ok()))
                    .collect()
            })
            .ok_or_else(|| invalid("IPFIX submission text must be UTF-8 bytes or string"))?,
        _ => {
            return Err(invalid(
                "IPFIX profile text must be explicit; use empty text for an unknown optional value",
            ));
        }
    };
    if name != "persistent_identifier" && std::str::from_utf8(&bytes).is_err() {
        return Err(invalid("IPFIX submission string is not UTF-8"));
    }
    if bytes.len() > max || (required && bytes.is_empty()) {
        return Err(invalid("IPFIX submission text length"));
    }
    if matches!(name, "sender_call" | "receiver_call")
        && !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'/')
    {
        return Err(invalid("IPFIX submission callsign"));
    }
    if matches!(name, "sender_locator" | "receiver_locator")
        && !bytes.is_empty()
        && (![2, 4, 6, 8, 10, 12, 14, 16].contains(&bytes.len())
            || !bytes.iter().enumerate().all(|(i, b)| match i {
                0 | 1 => (b'A'..=b'R').contains(&b.to_ascii_uppercase()),
                _ if (i / 2) % 2 == 1 => b.is_ascii_digit(),
                _ => (b'A'..=b'X').contains(&b.to_ascii_uppercase()),
            }))
    {
        return Err(invalid("IPFIX submission locator"));
    }
    if name == "mode"
        && !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'/' | b'+'))
    {
        return Err(invalid("IPFIX submission mode token"));
    }
    Ok(bytes)
}
fn variable(out: &mut Vec<u8>, data: &[u8]) {
    if data.len() < 255 {
        out.push(data.len() as u8);
    } else {
        out.push(255);
        out.extend((data.len() as u16).to_be_bytes());
    }
    out.extend(data);
}
fn number(fields: &Value, name: &str, min: i64, max: i64) -> Result<i64, Error> {
    crate::integer_i64(&fields[name])
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| invalid("IPFIX submission integer missing or out of range"))
}
fn seconds(fields: &Value) -> Result<u32, Error> {
    let s = fields["time"]
        .as_str()
        .ok_or_else(|| invalid("IPFIX submission time required"))?;
    if s.len() != 20 || !crate::json::valid_instant(s) {
        return Err(invalid("IPFIX submission requires whole UTC seconds"));
    }
    // At most 49,711 days: bounded conversion independent of local time zones.
    let year: u32 = s[..4].parse().map_err(|_| invalid("IPFIX time"))?;
    let month: u32 = s[5..7].parse().map_err(|_| invalid("IPFIX time"))?;
    let day: u32 = s[8..10].parse().map_err(|_| invalid("IPFIX time"))?;
    if !(1970..=2106).contains(&year) {
        return Err(invalid("IPFIX time outside u32 range"));
    }
    let leap = |y| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mut days = 0_u64;
    for y in 1970..year {
        days += if leap(y) { 366 } else { 365 };
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    days += months[..month as usize - 1].iter().sum::<u64>() + u64::from(day - 1);
    let h: u64 = s[11..13].parse().map_err(|_| invalid("IPFIX time"))?;
    let m: u64 = s[14..16].parse().map_err(|_| invalid("IPFIX time"))?;
    let sec: u64 = s[17..19].parse().map_err(|_| invalid("IPFIX time"))?;
    if sec > 59 {
        return Err(invalid("IPFIX time leap second not representable"));
    }
    u32::try_from(days * 86400 + h * 3600 + m * 60 + sec)
        .map_err(|_| invalid("IPFIX time outside u32 range"))
}
fn descriptor(id: u16, fields: &[(u16, u16)], scope: Option<u16>) -> Vec<u8> {
    let mut raw = Vec::new();
    raw.extend(id.to_be_bytes());
    raw.extend((fields.len() as u16).to_be_bytes());
    if let Some(scope) = scope {
        raw.extend(scope.to_be_bytes());
    }
    for (id, len) in fields {
        raw.extend((if *id == 150 { *id } else { *id | 0x8000 }).to_be_bytes());
        raw.extend(len.to_be_bytes());
        if *id != 150 {
            raw.extend(PEN.to_be_bytes());
        }
    }
    wire::pad_set(if scope.is_some() { 3 } else { 2 }, &raw)
}
/// Encode one submission in the source program's Set/field order, without truncation.
/// Header counters and template cadence are caller supplied. No scheduling or deduplication occurs.
pub fn encode_submission(
    receiver: &Event,
    spots: &[Event],
    options: &SubmissionOptions,
) -> Result<Vec<u8>, Error> {
    if receiver.value()["family"] != "pskreporter-ipfix"
        || receiver.kind_token() != "receiver_information"
        || receiver.mapping() != 1
    {
        return Err(invalid("IPFIX receiver event required"));
    }
    if spots.len() > MAX_EVENTS - 1 {
        return Err(oversized("IPFIX submission record limit"));
    }
    let ws = options.profile == SubmissionProfile::Wsjtx;
    if !ws && spots.is_empty() {
        return Err(invalid(
            "fldigi source profile does not submit receiver-only messages",
        ));
    }
    if ws && options.fldigi_short {
        return Err(invalid("fldigi_short requires fldigi profile"));
    }
    let rf = &receiver.value()["fields"];
    let allowed = [
        "receiver_call",
        "receiver_locator",
        "reporting_software",
        "antenna_information",
        "rig_information",
        "persistent_identifier",
    ];
    if rf
        .as_object()
        .expect("fields")
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
    {
        return Err(invalid("unsupported IPFIX receiver field"));
    }
    if ws && rf.get("persistent_identifier").is_some() || !ws && rf.get("rig_information").is_some()
    {
        return Err(invalid("receiver field absent from selected profile"));
    }
    if !ws
        && options.fldigi_short
        && ["reporting_software", "antenna_information"]
            .iter()
            .any(|k| rf.get(k).is_some())
    {
        return Err(invalid(
            "fldigi short receiver cannot carry software/antenna",
        ));
    }
    let mut rx = Vec::new();
    for (name, max, required) in [
        ("receiver_call", if ws { 32 } else { 254 }, false),
        ("receiver_locator", if ws { 16 } else { 254 }, false),
    ] {
        variable(&mut rx, &text(rf, name, max, required)?);
    }
    if !ws {
        let persistent = text(rf, "persistent_identifier", 8, true)?;
        if persistent.len() != 8 {
            return Err(invalid("fldigi persistent identifier requires eight bytes"));
        }
        rx.extend(persistent);
    }
    if ws || !options.fldigi_short {
        variable(
            &mut rx,
            &text(rf, "reporting_software", if ws { 80 } else { 254 }, false)?,
        );
        variable(
            &mut rx,
            &text(rf, "antenna_information", if ws { 128 } else { 254 }, false)?,
        );
    }
    if ws {
        variable(&mut rx, &text(rf, "rig_information", 128, false)?);
    }
    let (sid, rid) = if ws {
        (0x50e3, 0x50e2)
    } else {
        (0x022c, if options.fldigi_short { 0x0218 } else { 0x0219 })
    };
    let mut sets = Vec::new();
    if options.include_templates {
        sets.extend(if ws {
            descriptor(
                sid,
                &[
                    (1, 65535),
                    (5, 5),
                    (6, 1),
                    (10, 65535),
                    (3, 65535),
                    (11, 1),
                    (150, 4),
                ],
                None,
            )
        } else {
            descriptor(
                sid,
                &[
                    (1, 65535),
                    (150, 4),
                    (5, 4),
                    (10, 65535),
                    (3, 65535),
                    (11, 1),
                ],
                None,
            )
        });
        sets.extend(if ws {
            descriptor(
                rid,
                &[(2, 65535), (4, 65535), (8, 65535), (9, 65535), (13, 65535)],
                Some(1),
            )
        } else if options.fldigi_short {
            descriptor(rid, &[(2, 65535), (4, 65535), (12, 8)], Some(0))
        } else {
            descriptor(
                rid,
                &[(2, 65535), (4, 65535), (12, 8), (8, 65535), (9, 65535)],
                Some(0),
            )
        });
    }
    sets.extend(wire::pad_set(rid, &rx));
    let mut tx = Vec::new();
    for event in spots {
        if event.value()["family"] != "pskreporter-ipfix"
            || event.kind_token() != "spot"
            || event.mapping() != 1
        {
            return Err(invalid("IPFIX spot event required"));
        }
        let f = &event.value()["fields"];
        let allowed = [
            "sender_call",
            "sender_locator",
            "mode",
            "frequency_hz",
            "snr_db",
            "information_source",
            "time",
            "receiver_call",
            "receiver_locator",
            "reporting_software",
            "antenna_information",
            "rig_information",
            "persistent_identifier",
        ];
        if f.as_object()
            .expect("fields")
            .keys()
            .any(|k| !allowed.contains(&k.as_str()))
        {
            return Err(invalid("unsupported IPFIX sender field"));
        }
        // Receiver metadata in a spot is an assertion, never silently replaced.
        for name in [
            "receiver_call",
            "receiver_locator",
            "reporting_software",
            "antenna_information",
            "rig_information",
            "persistent_identifier",
        ] {
            if let Some(value) = f.get(name)
                && rf.get(name) != Some(value)
            {
                return Err(invalid("IPFIX spot/receiver mismatch"));
            }
        }
        if !ws && f.get("snr_db").is_some() {
            return Err(invalid("fldigi profile has no SNR"));
        }
        let mut record = Vec::new();
        variable(
            &mut record,
            &text(f, "sender_call", if ws { 32 } else { 254 }, true)?,
        );
        let time = seconds(f)?;
        let frequency = number(
            f,
            "frequency_hz",
            0,
            if ws {
                0xff_ffff_ffff
            } else {
                i64::from(u32::MAX)
            },
        )? as u64;
        if !ws {
            record.extend(time.to_be_bytes());
        }
        record.extend(&frequency.to_be_bytes()[if ws { 3 } else { 4 }..]);
        if ws {
            record.push(number(f, "snr_db", -128, 127)? as i8 as u8);
        }
        variable(
            &mut record,
            &text(f, "mode", if ws { 16 } else { 254 }, true)?,
        );
        variable(
            &mut record,
            &text(f, "sender_locator", if ws { 16 } else { 254 }, false)?,
        );
        let source = number(f, "information_source", 0, 255)? as u8;
        if source & 0x7c != 0 || source & 3 == 0 {
            return Err(invalid("IPFIX informationSource flags"));
        }
        record.push(source);
        if ws {
            record.extend(time.to_be_bytes());
        }
        if (16 + sets.len() + 4 + tx.len() + record.len() + 3) & !3 > if ws { 952 } else { 1454 } {
            return Err(oversized(
                "IPFIX selected program UDP payload limit; caller must split",
            ));
        }
        tx.extend(record);
    }
    if !spots.is_empty() {
        sets.extend(wire::pad_set(sid, &tx));
    }
    let mut result = Vec::new();
    result.extend(10_u16.to_be_bytes());
    result.extend(((16 + sets.len()) as u16).to_be_bytes());
    result.extend(options.export_time.to_be_bytes());
    result.extend(options.sequence_number.to_be_bytes());
    result.extend(options.observation_domain_id.to_be_bytes());
    result.extend(sets);
    if result.len() > if ws { 952 } else { 1454 } {
        return Err(oversized("IPFIX selected program UDP payload limit"));
    }
    Ok(result)
}
