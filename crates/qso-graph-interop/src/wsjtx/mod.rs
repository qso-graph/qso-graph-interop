// SPDX-License-Identifier: GPL-3.0-or-later
//! WSJT-X 3.0.2 and provisional v3.3.0-beta6 additions. See docs/PROTOCOL.md for source pins and wire rules.
#![forbid(unsafe_code)]

mod codec;
mod family;
mod mapping;
mod printable;
pub use family::Wsjtx;
pub(crate) mod float64;
pub mod qt;
mod types;
pub use qt::{QColor, QDateTime, QString, QTime, TimeZone, Utf8};
use serde::{Deserialize, Serialize};
pub use types::*;

pub const MAGIC: u32 = 0xadbc_cbda;
pub const MAX_SCHEMA: u32 = 3;
pub const PULSE_SECONDS: u32 = 15;
/// Portable maximum UDP payload (IPv4 without options); checked before any allocation.
pub const MAX_DATAGRAM: usize = 65_507;
pub const MAX_ID: usize = 256;
pub const WSJTX_TAG: &str = "v3.3.0-beta6";
pub const MAX_INHIBIT_ID: usize = 1024;
pub const MAX_INHIBIT_DATAGRAM: usize = 4096;
/// Raw construction does not establish encodability; encode validates native limits.
/// The original family payload and all unrecognized trailing bytes remain available.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Packet {
    pub schema: u32,
    pub id: Utf8,
    pub body: Body,
    pub extensions: Vec<u8>,
}
impl Packet {
    pub fn new(id: impl Into<Utf8>, body: Body) -> Self {
        Self {
            schema: MAX_SCHEMA,
            id: id.into(),
            body,
            extensions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Outcome {
    Message(Box<Packet>),
    #[non_exhaustive]
    Skipped {
        schema: u32,
        message_type: Option<u32>,
        reason: SkipReason,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SkipReason {
    FutureSchema,
    UnknownType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    #[non_exhaustive]
    Truncated {
        offset: usize,
    },
    WrongMagic,
    Length,
    #[non_exhaustive]
    Event {
        category: crate::Category,
        message: String,
    },
    OddQStringLength,
    UnsupportedSchema(u32),
    InvalidTimeSpec(u8),
    InconsistentDateTime,
    NonContiguousTail,
}
impl Error {
    pub fn truncated(offset: usize) -> Self {
        Self::Truncated { offset }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { offset } => write!(f, "WSJT-X field is truncated at byte {offset}"),
            Self::WrongMagic => f.write_str("invalid WSJT-X magic"),
            Self::Event { message, .. } => f.write_str(message),
            Self::Length => f.write_str("WSJT-X resource bound exceeded"),
            Self::OddQStringLength => f.write_str("QString byte count must be even"),
            Self::UnsupportedSchema(n) => write!(f, "unsupported WSJT-X schema {n}"),
            Self::InvalidTimeSpec(n) => write!(f, "invalid Qt time specification {n}"),
            Self::InconsistentDateTime => f.write_str("inconsistent Qt date/time suffix"),
            Self::NonContiguousTail => f.write_str("optional WSJT-X tail contains a hole"),
        }
    }
}
impl std::error::Error for Error {}

/// Decode one datagram. Unknown types and future schemas are skipped, never guessed.
pub fn decode(data: &[u8]) -> Result<Outcome, Error> {
    if data.len() > MAX_DATAGRAM {
        return Err(Error::Length);
    }
    let mut r = qt::Reader {
        data,
        at: 0,
        schema: MAX_SCHEMA,
    };
    if r.u32()? != MAGIC {
        return Err(Error::WrongMagic);
    }
    let schema = r.u32()?;
    if schema > MAX_SCHEMA {
        return Ok(Outcome::Skipped {
            schema,
            message_type: None,
            reason: SkipReason::FutureSchema,
        });
    }
    if schema == 0 {
        return Err(Error::UnsupportedSchema(schema));
    }
    r.schema = schema;
    let message_type = r.u32()?;
    if !codec::known(message_type) {
        return Ok(Outcome::Skipped {
            schema,
            message_type: Some(message_type),
            reason: SkipReason::UnknownType,
        });
    }
    if message_type == 18 && data.len() > MAX_INHIBIT_DATAGRAM {
        return Err(Error::Length);
    }
    let id = if message_type == 18 {
        r.inhibit_id()?
    } else {
        r.id()?
    };
    let body = codec::read_body(&mut r, message_type)?;
    if let Body::TxInhibit(command) = &body {
        validate_inhibit(schema, &id, command)?;
    }
    Ok(Outcome::Message(Box::new(Packet {
        schema,
        id,
        body,
        extensions: r.take(r.remaining())?.to_vec(),
    })))
}

/// Encode one datagram, refusing holes in optional tails,
/// unsupported schemas and any length beyond the decoder's bounds.
pub fn encode(packet: &Packet) -> Result<Vec<u8>, Error> {
    if !(1..=MAX_SCHEMA).contains(&packet.schema) {
        return Err(Error::UnsupportedSchema(packet.schema));
    }
    let id_limit = if matches!(packet.body, Body::TxInhibit(_)) {
        MAX_INHIBIT_ID
    } else {
        MAX_ID
    };
    if let Body::TxInhibit(command) = &packet.body {
        validate_inhibit(packet.schema, &packet.id, command)?;
        // 12 header bytes plus four u32 text lengths/TTL: 28 fixed bytes.
        let text_len = |text: &Utf8| text.bytes.as_ref().map_or(0, Vec::len);
        let body_len = 28
            + text_len(&packet.id)
            + text_len(&command.controller_id)
            + text_len(&command.station);
        if body_len.saturating_add(packet.extensions.len()) > MAX_INHIBIT_DATAGRAM {
            return Err(Error::Length);
        }
    }
    if packet.id.bytes.as_ref().is_some_and(|b| b.len() > id_limit) {
        return Err(Error::Length);
    }
    let mut w = qt::Writer::new(packet.schema);
    w.u32(MAGIC)?;
    w.u32(packet.schema)?;
    w.u32(packet.body.message_type())?;
    w.utf8(&packet.id)?;
    codec::write_body(&mut w, &packet.body)?;
    if w.omitted && !packet.extensions.is_empty() {
        return Err(Error::NonContiguousTail);
    }
    w.put(&packet.extensions)?;
    Ok(w.data)
}

fn validate_inhibit(schema: u32, id: &Utf8, command: &TxInhibit) -> Result<(), Error> {
    let invalid = |message: &str| Error::Event {
        category: crate::Category::InvalidValue,
        message: message.into(),
    };
    if schema < 2 {
        return Err(Error::UnsupportedSchema(schema));
    }
    if id.as_str().is_none() {
        return Err(invalid("TxInhibit target must be non-null UTF-8"));
    }
    for (text, identity) in [(&command.controller_id, true), (&command.station, false)] {
        if text.bytes.as_ref().is_some_and(|b| b.len() > 128) {
            return Err(invalid("TxInhibit text exceeds 128 UTF-8 bytes"));
        }
        if !identity && text.bytes.is_none() {
            continue;
        }
        let text = text
            .as_str()
            .ok_or_else(|| invalid("invalid TxInhibit UTF-8"))?;
        if (identity && text.is_empty())
            || text
                .chars()
                .any(|c| !printable::is_print(c) || (identity && c.is_whitespace()))
        {
            return Err(invalid("invalid TxInhibit text characters"));
        }
    }
    if command.ttl_ms != 0 && !(100..=30000).contains(&command.ttl_ms) {
        return Err(invalid("TxInhibit TTL must be 0 or 100–30000 ms"));
    }
    Ok(())
}

/// Highest mutually supported schema. A pre-negotiation heartbeat implies schema 2.
/// A response must also respect the schema used in the peer's current datagram.
pub fn negotiate_schema(
    local_max: u32,
    peer_schema: u32,
    heartbeat: &Heartbeat,
) -> Result<u32, Error> {
    let schema = local_max
        .min(MAX_SCHEMA)
        .min(peer_schema)
        .min(heartbeat.max_schema.unwrap_or(2));
    if schema == 0 {
        Err(Error::UnsupportedSchema(schema))
    } else {
        Ok(schema)
    }
}

pub fn mode_character(mode: &str) -> &str {
    match mode {
        "FT8" => "~",
        "FT4" => "+",
        "JT65" => "#",
        "JT9" => "@",
        "JT4" => "$",
        "MSK144" => "&",
        "Q65" => ":",
        "FST4" => "`",
        other => other,
    }
}

#[cfg(test)]
mod encoder_guards {
    use super::*;
    #[test]
    fn inconsistent_dates_tail_holes_and_extensions_are_rejected() {
        let mut qso = QsoLogged::default();
        qso.time_off.time_spec = 2;
        let mut packet = Packet::new("synthetic-instance", Body::QsoLogged(Box::new(qso)));
        assert_eq!(encode(&packet), Err(Error::InconsistentDateTime));
        let Body::QsoLogged(qso) = &mut packet.body else {
            panic!("QSO")
        };
        qso.time_off.time_spec = 99;
        assert_eq!(encode(&packet), Err(Error::InvalidTimeSpec(99)));
        let heartbeat = Heartbeat {
            version: Some("synthetic".into()),
            ..Heartbeat::default()
        };
        let packet = Packet::new("synthetic-instance", Body::Heartbeat(heartbeat));
        assert_eq!(encode(&packet), Err(Error::NonContiguousTail));
        let mut packet = Packet::new("synthetic-instance", Body::Heartbeat(Heartbeat::default()));
        packet.extensions = vec![1, 2, 3];
        assert_eq!(encode(&packet), Err(Error::NonContiguousTail));
    }
}

/// Native schema selection. The default encoding schema is 3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Schema(u32);
impl TryFrom<u32> for Schema {
    type Error = crate::Error;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if (1..=3).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::UnsupportedSchema(value).into())
        }
    }
}
impl Schema {
    pub fn get(self) -> u32 {
        self.0
    }
}
impl From<Schema> for crate::codec::EncodeOptions {
    fn from(schema: Schema) -> Self {
        let mut native = serde_json::Map::new();
        native.insert("schema".into(), schema.0.into());
        Self::extension_metadata(native)
    }
}
