// SPDX-License-Identifier: GPL-3.0-or-later
//! WSJT-X 3.0.2 (the same protocol as 2.7.0). See docs/PROTOCOL.md for source pins and wire rules.
#![forbid(unsafe_code)]

mod codec;
mod family;
mod mapping;
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
pub const WSJTX_TAG: &str = "v3.0.2";
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
    Truncated {
        offset: usize,
    },
    WrongMagic,
    Length,
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
    let id = r.id()?;
    let body = codec::read_body(&mut r, message_type)?;
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
    if packet.id.bytes.as_ref().is_some_and(|b| b.len() > MAX_ID) {
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
