// SPDX-License-Identifier: GPL-3.0-or-later
//! PSK Reporter IPFIX over caller-supplied UDP payloads, Annex H revision 1.
#![forbid(unsafe_code)]
mod submission;
mod wire;
use crate::{
    Category, Error, Event,
    codec::{
        Codec, DecodeOptions, EncodeOptions, EncodedCommands, Framing, Limits, Outcome, Profile,
        Skip, SkipReason, VerifyOptions,
    },
    error::{invalid, oversized},
    model::Verification,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
pub use submission::{SubmissionOptions, SubmissionProfile, encode_submission};
use wire::{Cursor, Template};

pub const PEN: u32 = 30351;
pub const MAX_DATAGRAM: usize = 60_000;
pub const MAX_FIELDS: usize = 64;
pub const MAX_SESSIONS: usize = 16;
pub const MAX_TEMPLATES: usize = 64;
pub const MAX_EVENTS: usize = 4096;
pub const MAX_TEMPLATE_BYTES: usize = 518;

/// A parse error carries Set/record locations when their boundaries are known.
#[derive(Debug)]
#[non_exhaustive]
pub struct IpfixError {
    pub category: Category,
    pub set_index: Option<usize>,
    pub record_index: Option<usize>,
    message: String,
}
impl std::fmt::Display for IpfixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "H.1 framing: {} (set {:?}, record {:?})",
            self.message, self.set_index, self.record_index
        )
    }
}
impl std::error::Error for IpfixError {}
fn failure(category: Category, message: &str) -> Error {
    Error::family(
        category,
        IpfixError {
            category,
            set_index: None,
            record_index: None,
            message: message.into(),
        },
    )
}
fn located(e: Error, set: usize, record: Option<usize>) -> Error {
    Error::family(
        e.category(),
        IpfixError {
            category: e.category(),
            set_index: Some(set),
            record_index: record,
            message: e.to_string(),
        },
    )
}

type Templates = BTreeMap<(u32, u16), Template>;
/// Each instance owns bounded exporter sessions. Failed messages never change state.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PskReporterIpfix {
    sessions: BTreeMap<Vec<u8>, Templates>,
}
#[derive(Clone)]
struct Record {
    template: Template,
    data: Vec<u8>,
    values: Vec<Vec<u8>>,
    set_id: u16,
    set_index: usize,
    record_index: usize,
}
impl Record {
    fn receiver(&self) -> bool {
        self.template
            .fields
            .iter()
            .any(|f| f.pen == PEN && f.id == 2)
            && !self.sender()
    }
    fn sender(&self) -> bool {
        self.template
            .fields
            .iter()
            .any(|f| f.pen == PEN && f.id == 1)
    }
    fn evidence(&self, header: &Value) -> Value {
        json!({"data":crate::base64::encode(&self.data),"template":crate::base64::encode(&self.template.bytes),"header":header,"set_id":self.set_id})
    }
}
impl PskReporterIpfix {
    fn decode_message(
        &mut self,
        bytes: &[u8],
        options: &DecodeOptions<'_>,
    ) -> Result<Outcome, Error> {
        if bytes.len() > MAX_DATAGRAM {
            return Err(oversized("IPFIX UDP payload limit"));
        }
        if options.session.len() > 128 {
            return Err(oversized("IPFIX session key limit"));
        }
        let mut c = Cursor::new(bytes);
        let version = c.u16()?;
        let length = c.u16()?;
        let export_time = c.u32()?;
        let sequence_number = c.u32()?;
        let domain = c.u32()?;
        if version < 10 {
            return Err(failure(
                Category::UnsupportedVersion,
                "unsupported IPFIX version",
            ));
        }
        if version != 10 {
            return Ok(Outcome {
                events: vec![],
                skips: vec![Skip::new(
                    SkipReason::FutureVersion,
                    Some(version.to_string()),
                    None,
                )],
            });
        }
        if usize::from(length) != bytes.len() {
            return Err(failure(
                Category::Malformed,
                "message length differs from datagram",
            ));
        }
        let header = json!({"version":version,"length":length,"export_time":export_time,"sequence_number":sequence_number,"observation_domain_id":domain});
        let mut state = self
            .sessions
            .get(options.session)
            .cloned()
            .unwrap_or_default();
        // expiry is an explicit export-time cutoff, not wall-clock time or arrival order.
        if let Some(cutoff) = options.expiry {
            state.retain(|_, t| t.refreshed >= cutoff);
        }
        let mut records = Vec::new();
        let mut skips = Vec::new();
        let mut set_index = 0;
        while c.left() != 0 {
            let set_id = c.u16().map_err(|e| located(e, set_index, None))?;
            let length = usize::from(c.u16().map_err(|e| located(e, set_index, None))?);
            if length < 4 {
                return Err(located(
                    failure(Category::Malformed, "set length below header"),
                    set_index,
                    None,
                ));
            }
            let payload = c
                .take(length - 4)
                .map_err(|e| located(e, set_index, None))?;
            if set_id == 2 || set_id == 3 {
                for (id, template) in wire::templates(payload, set_id == 3, u64::from(export_time))
                    .map_err(|e| located(e, set_index, None))?
                {
                    if let Some(template) = template {
                        if !state.contains_key(&(domain, id)) && state.len() == MAX_TEMPLATES {
                            return Err(located(
                                oversized("IPFIX template cache overflow"),
                                set_index,
                                None,
                            ));
                        }
                        state.insert((domain, id), template);
                    }
                }
            } else if set_id >= 256 {
                if let Some(template) = state.get(&(domain, set_id)) {
                    let mut data = Cursor::new(payload);
                    let mut record_index = 0;
                    while data.left() >= template.minimum() {
                        if records.len() + skips.len() == MAX_EVENTS {
                            return Err(located(
                                oversized("IPFIX event limit"),
                                set_index,
                                Some(record_index),
                            ));
                        }
                        let start = data.pos;
                        let values = wire::record(&mut data, template)
                            .map_err(|e| located(e, set_index, Some(record_index)))?;
                        records.push(Record {
                            template: template.clone(),
                            data: payload[start..data.pos].to_vec(),
                            values: values.into_iter().map(<[u8]>::to_vec).collect(),
                            set_id,
                            set_index,
                            record_index,
                        });
                        record_index += 1;
                    }
                    // A suffix shorter than the minimum record is Set padding, even if nonzero.
                } else {
                    if records.len() + skips.len() == MAX_EVENTS {
                        return Err(located(oversized("IPFIX outcome limit"), set_index, None));
                    }
                    let mut skip = Skip::new(
                        SkipReason::UnknownTemplate,
                        Some("10".into()),
                        Some(u64::from(set_id)),
                    );
                    skip.set_index = Some(set_index);
                    skip.evidence = Some(
                        json!({"message_data":crate::base64::encode(bytes),"header":header,"set_id":set_id,"data":crate::base64::encode(payload)}),
                    );
                    skips.push(skip);
                }
            } else {
                return Err(located(
                    failure(Category::Malformed, "reserved Set ID"),
                    set_index,
                    None,
                ));
            }
            set_index += 1;
        }
        let receivers: Vec<_> = records.iter().filter(|r| r.receiver()).collect();
        let receiver = if receivers.len() == 1 {
            receivers.first().copied()
        } else {
            None
        };
        let mut events = Vec::with_capacity(records.len());
        for r in &records {
            let kind = if r.sender() {
                "spot"
            } else if r.receiver() {
                "receiver_information"
            } else {
                "native"
            };
            let mut fields = map_fields(r);
            let mut native = r.evidence(&header);
            native["format"] = "ipfix_record".into();
            native["message_data"] = crate::base64::encode(bytes).into();
            native["set_index"] = r.set_index.into();
            native["record_index"] = r.record_index.into();
            if r.sender()
                && let Some(receiver) = receiver
            {
                fields.extend(map_fields(receiver));
                native["dependencies"] = json!([receiver.evidence(&header)]);
            }
            if options.structured {
                native["message"] = structured(r);
            }
            let event = Event::from_value(
                json!({"spec":crate::model::SPEC,"family":"pskreporter-ipfix","mapping":1,"native_version":"10","kind":kind,"context":options.context,"fields":fields,"native":native}),
            )?;
            crate::json::write(&event, &crate::PublicationOptions::default())?;
            events.push(event);
        }
        if !state.is_empty() {
            if !self.sessions.contains_key(options.session) && self.sessions.len() == MAX_SESSIONS {
                return Err(oversized("IPFIX session cache overflow"));
            }
            self.sessions.insert(options.session.to_vec(), state);
        } else {
            self.sessions.remove(options.session);
        }
        Ok(Outcome { events, skips })
    }
}
fn structured(r: &Record) -> Value {
    json!({"options":r.template.options,"scope_fields":r.template.scope,"elements":r.template.fields.iter().zip(&r.values).map(|(f,v)| json!({"enterprise":f.pen,"id":f.id,"length":f.len,"data":crate::base64::encode(v)})).collect::<Vec<_>>()})
}
fn map_fields(r: &Record) -> Map<String, Value> {
    let mut result = Map::new();
    if !r.sender() && !r.receiver() {
        return result;
    }
    for (f, bytes) in r.template.fields.iter().zip(&r.values) {
        let name = if f.pen == PEN {
            match f.id {
                1 if r.sender() => "sender_call",
                3 if r.sender() => "sender_locator",
                2 if r.receiver() => "receiver_call",
                4 if r.receiver() => "receiver_locator",
                8 if r.receiver() => "reporting_software",
                9 if r.receiver() => "antenna_information",
                12 if r.receiver() => "persistent_identifier",
                13 if r.receiver() => "rig_information",
                5 if r.sender() => "frequency_hz",
                6 if r.sender() => "snr_db",
                7 if r.sender() => "imd_db",
                10 if r.sender() => "mode",
                11 if r.sender() => "information_source",
                _ => continue,
            }
        } else if f.pen == 0 && f.id == 150 && r.sender() {
            "time"
        } else {
            continue;
        };
        // A repeated IE has no unambiguous scalar common value; preserve all native occurrences.
        if r.template
            .fields
            .iter()
            .filter(|other| other.pen == f.pen && other.id == f.id)
            .count()
            != 1
        {
            continue;
        }
        let value = match name {
            "frequency_hz" => wire::uint(bytes)
                .filter(|n| *n <= crate::model::MAX_INTEGER as u64)
                .map(Value::from),
            "snr_db" | "imd_db" => wire::signed(bytes)
                .filter(|n| n.unsigned_abs() <= crate::model::MAX_INTEGER as u64)
                .map(Value::from),
            "information_source" => (bytes.len() == 1).then(|| Value::from(bytes[0])),
            "time" => (bytes.len() == 4).then(|| {
                Value::from(timestamp(u32::from_be_bytes(
                    bytes.as_slice().try_into().expect("four"),
                )))
            }),
            _ => Some(crate::Text::utf8(Some(bytes)).value()),
        };
        if let Some(value) = value {
            result.insert(name.into(), value);
        }
    }
    result
}
/// UTC conversion for the entire native unsigned 32-bit seconds range.
fn timestamp(seconds: u32) -> String {
    let days = i64::from(seconds / 86400);
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let day_seconds = seconds % 86400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3600,
        day_seconds / 60 % 60,
        day_seconds % 60
    )
}
impl Profile for PskReporterIpfix {
    fn family(&self) -> &str {
        "pskreporter-ipfix"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn native_version_supported(&self, event: &Event) -> bool {
        event.value()["native_version"] == "10"
    }
    fn recognize(
        &self,
        event: &Event,
        options: &VerifyOptions,
    ) -> Result<crate::model::ProfileView, Error> {
        crate::json::compatible_view(
            event,
            self.native_version_supported(event),
            self.verify(event, options)?,
        )
    }
    fn verify(&self, event: &Event, _: &VerifyOptions) -> Result<Verification, Error> {
        let v = event.value();
        if v["family"] != self.family()
            || !self.native_version_supported(event)
            || event.is_command()
        {
            return Ok(Verification::Unavailable);
        }
        let n = &v["native"];
        if n["message_data"]
            .as_str()
            .is_some_and(|s| s.len() > 4 * MAX_DATAGRAM.div_ceil(3))
            || n["dependencies"].as_array().is_some_and(|a| a.len() > 1)
        {
            return Err(oversized("IPFIX message/dependency evidence limit"));
        }
        let bytes = crate::base64::decode(
            n["message_data"]
                .as_str()
                .ok_or_else(|| invalid("IPFIX message evidence"))?,
        )?;
        if bytes.len() > MAX_DATAGRAM {
            return Err(oversized("IPFIX message evidence limit"));
        }
        let domain = n["header"]["observation_domain_id"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| invalid("IPFIX domain"))?;
        let mut definitions = Vec::new();
        for evidence in std::iter::once(n).chain(n["dependencies"].as_array().into_iter().flatten())
        {
            if evidence["template"]
                .as_str()
                .is_some_and(|s| s.len() > 4 * MAX_TEMPLATE_BYTES.div_ceil(3))
            {
                return Err(oversized("IPFIX template evidence limit"));
            }
            let raw = crate::base64::decode(
                evidence["template"]
                    .as_str()
                    .ok_or_else(|| invalid("IPFIX template evidence"))?,
            )?;
            if raw.len() > MAX_TEMPLATE_BYTES {
                return Err(oversized("IPFIX template evidence limit"));
            }
            // Both forms are tried structurally; exact evidence comparison below prevents ambiguity.
            let parsed = wire::templates(&raw, false, 0)
                .ok()
                .filter(|v| v.len() == 1 && v[0].1.as_ref().is_some_and(|t| t.bytes == raw))
                .or_else(|| {
                    wire::templates(&raw, true, 0)
                        .ok()
                        .filter(|v| v.len() == 1 && v[0].1.as_ref().is_some_and(|t| t.bytes == raw))
                })
                .ok_or_else(|| invalid("IPFIX template evidence"))?;
            let (id, template) = parsed.into_iter().next().expect("one");
            definitions.push(((domain, id), template.expect("definition")));
        }
        let options = DecodeOptions {
            context: v["context"].clone(),
            structured: n.get("message").is_some(),
            ..DecodeOptions::default()
        };
        // At most two explicit definitions: try their four possible initial-cache subsets.
        // Installing a definition that actually arrived later must not reinterpret an earlier
        // unknown Set. No definition other than the record/dependency evidence is guessed.
        for mask in 0..(1 << definitions.len()) {
            let mut codec = Self::default();
            let mut state = Templates::new();
            for (index, (key, template)) in definitions.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    state.insert(*key, template.clone());
                }
            }
            codec.sessions.insert(vec![], state);
            let Ok(out) = codec.decode(&bytes, &options) else {
                continue;
            };
            let Some(actual) = out.events.into_iter().find(|e| {
                e.value()["native"]["set_index"] == n["set_index"]
                    && e.value()["native"]["record_index"] == n["record_index"]
            }) else {
                continue;
            };
            if crate::json::semantic_eq(&crate::json::known_fields(v), &actual.value()["fields"])
                && v["kind"] == actual.value()["kind"]
                && n == &actual.value()["native"]
            {
                return Ok(Verification::Verified);
            }
        }
        Err(invalid(
            "IPFIX evidence/common mismatch or record location unavailable",
        ))
    }

    fn validate_authored(&self, event: &Event, options: &VerifyOptions) -> Result<(), Error> {
        if self.verify(event, options)? != Verification::Verified {
            return Err(invalid("IPFIX authored evidence unavailable"));
        }
        Ok(())
    }
}
impl Codec for PskReporterIpfix {
    fn framing(&self) -> Framing {
        Framing::Templated
    }
    fn limits(&self) -> Limits {
        let mut limits = Limits::stateless(MAX_DATAGRAM, MAX_DATAGRAM - 20);
        limits.events = MAX_EVENTS;
        limits.sessions = MAX_SESSIONS;
        limits.templates_per_session = MAX_TEMPLATES;
        limits.template_bytes = MAX_TEMPLATE_BYTES;
        limits.dependency_bytes = MAX_DATAGRAM + MAX_TEMPLATE_BYTES;
        limits
    }
    fn decode(&mut self, bytes: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error> {
        self.decode_message(bytes, options)
    }
    fn encode_commands(&self, _: &Event, _: &EncodeOptions) -> Result<EncodedCommands, Error> {
        Err(invalid(
            "IPFIX has no control operations; use encode_submission",
        ))
    }
    fn reencode(
        &self,
        event: &Event,
        options: &EncodeOptions,
        verify: &VerifyOptions,
    ) -> Result<Vec<u8>, Error> {
        if !options.native.is_empty() || self.verify(event, verify)? != Verification::Verified {
            return Err(invalid("IPFIX reencode evidence"));
        }
        event.native_bytes()
    }
    fn reset(&mut self, session: Option<&[u8]>) {
        if let Some(session) = session {
            self.sessions.remove(session);
        } else {
            self.sessions.clear();
        }
    }
}
