// SPDX-License-Identifier: GPL-3.0-or-later
//! Transport-independent observations with caller-supplied endpoint context.
//! Every message owns its complete family representation for lossless return.
#![forbid(unsafe_code)]

use crate::wsjtx::{Body, Packet, QDateTime, QTime, Utf8};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message<F = Packet> {
    pub kind: Kind,
    /// Neutral wire identifier. It is neither an authenticated identity nor always a source.
    pub client_id: Utf8,
    pub context: Context,
    /// Protocol-specific fields, including schema and raw extensions.
    pub family: F,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub direction: Direction,
    /// Caller-supplied identities; never inferred from a wire client ID.
    pub source_id: Option<Utf8>,
    pub target_id: Option<Utf8>,
    /// Optional request/response correlation supplied by a protocol adapter or caller.
    pub correlation_id: Option<Utf8>,
    pub exchange: Exchange,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exchange {
    #[default]
    Unknown,
    Notification,
    Request,
    Response,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    #[default]
    Unknown,
    FromApplication,
    ToApplication,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Clear,
    Close,
    Reply,
    Configure,
    /// Other native operations retain their name and arguments in `Message::family`.
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    LoggedQso(Box<LoggedQso>),
    RadioState(RadioState),
    Decode(Decode),
    Control(Control),
    Notification(Operation),
    /// The family permits both a notification and a command; direction is unknown.
    Ambiguous(Operation),
    /// A native payload with no normalized view (for example, an RPC error).
    Native,
    /// Heartbeats and other lifecycle messages retain their native meaning.
    Lifecycle,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LoggedQso {
    pub call: Option<Utf8>,
    pub grid: Option<Utf8>,
    pub mode: Option<Utf8>,
    pub tx_frequency_hz: Option<u64>,
    pub dial_frequency_hz: Option<u64>,
    pub start: Option<QDateTime>,
    pub end: Option<QDateTime>,
    pub report_sent: Option<Utf8>,
    pub report_received: Option<Utf8>,
    pub station_call: Option<Utf8>,
    pub station_grid: Option<Utf8>,
    pub operator_call: Option<Utf8>,
    pub exchange_sent: Option<Utf8>,
    pub exchange_received: Option<Utf8>,
    /// An ADIF file is retained verbatim, including its header and extensions.
    pub adif: Option<Utf8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RadioState {
    pub dial_frequency_hz: u64,
    pub mode: Utf8,
    pub tx_mode: Utf8,
    pub tx_enabled: bool,
    pub transmitting: bool,
    pub decoding: bool,
    pub rx_offset_hz: i64,
    pub tx_offset_hz: i64,
    pub station_call: Option<Utf8>,
    pub station_grid: Option<Utf8>,
    pub dx_call: Utf8,
    pub dx_grid: Option<Utf8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decode {
    pub new: bool,
    pub time: QTime,
    pub snr_db: i32,
    #[serde(with = "crate::wsjtx::float64")]
    pub delta_time_seconds: f64,
    pub offset_hz: Option<u32>,
    pub frequency_hz: Option<u64>,
    pub mode: Option<Utf8>,
    pub text: Option<Utf8>,
    pub call: Option<Utf8>,
    pub grid: Option<Utf8>,
    pub low_confidence: Option<bool>,
    pub off_air: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    pub operation: Operation,
    /// A target explicitly identified by this protocol, independent of source identity.
    pub target_id: Utf8,
}

impl From<Packet> for Message {
    fn from(family: Packet) -> Self {
        Self::with_context(family, Context::default())
    }
}

impl Message {
    /// Normalize using endpoint knowledge provided by the caller, never the sender ID.
    pub fn with_context(family: Packet, context: Context) -> Self {
        let kind = match &family.body {
            Body::QsoLogged(q) => Kind::LoggedQso(Box::new(LoggedQso {
                call: Some(q.dx_call.clone()),
                grid: Some(q.dx_grid.clone()),
                mode: Some(q.mode.clone()),
                tx_frequency_hz: Some(q.frequency_hz),
                dial_frequency_hz: None,
                start: q.time_on.clone(),
                end: Some(q.time_off.clone()),
                report_sent: Some(q.report_sent.clone()),
                report_received: Some(q.report_received.clone()),
                station_call: q.my_call.clone(),
                station_grid: q.my_grid.clone(),
                operator_call: q.operator_call.clone(),
                exchange_sent: q.exchange_sent.clone(),
                exchange_received: q.exchange_received.clone(),
                adif: None,
            })),
            Body::LoggedAdif(q) => Kind::LoggedQso(Box::new(LoggedQso {
                adif: Some(q.adif.clone()),
                ..LoggedQso::default()
            })),
            Body::Status(s) => Kind::RadioState(RadioState {
                dial_frequency_hz: s.dial_frequency_hz,
                mode: s.mode.clone(),
                tx_mode: s.tx_mode.clone(),
                tx_enabled: s.tx_enabled,
                transmitting: s.transmitting,
                decoding: s.decoding,
                rx_offset_hz: i64::from(s.rx_df_hz),
                tx_offset_hz: i64::from(s.tx_df_hz),
                station_call: s.de_call.clone(),
                station_grid: s.de_grid.clone(),
                dx_call: s.dx_call.clone(),
                dx_grid: s.dx_grid.clone(),
            }),
            Body::Decode(d) => Kind::Decode(Decode {
                new: d.new,
                time: d.time,
                snr_db: d.snr_db,
                delta_time_seconds: d.delta_time_seconds,
                offset_hz: Some(d.delta_frequency_hz),
                frequency_hz: None,
                mode: Some(d.mode.clone()),
                text: Some(d.message.clone()),
                call: None,
                grid: None,
                low_confidence: d.low_confidence,
                off_air: d.off_air,
            }),
            Body::WsprDecode(d) => Kind::Decode(Decode {
                new: d.new,
                time: d.time,
                snr_db: d.snr_db,
                delta_time_seconds: d.delta_time_seconds,
                offset_hz: None,
                frequency_hz: Some(d.frequency_hz),
                mode: None,
                text: None,
                call: Some(d.callsign.clone()),
                grid: Some(d.grid.clone()),
                low_confidence: None,
                off_air: d.off_air,
            }),
            Body::Clear(_) | Body::Close => {
                let operation = if matches!(family.body, Body::Clear(_)) {
                    Operation::Clear
                } else {
                    Operation::Close
                };
                match context.direction {
                    Direction::Unknown => Kind::Ambiguous(operation),
                    Direction::ToApplication => Kind::Control(Control {
                        operation,
                        target_id: family.id.clone(),
                    }),
                    Direction::FromApplication if operation == Operation::Close => Kind::Lifecycle,
                    Direction::FromApplication => Kind::Notification(operation),
                }
            }
            Body::Heartbeat(_) => Kind::Lifecycle,
            _ => Kind::Control(Control {
                target_id: family.id.clone(),
                operation: match family.body {
                    Body::Reply(_) => Operation::Reply,
                    Body::Configure(_) => Operation::Configure,
                    _ => Operation::Other,
                },
            }),
        };
        Self {
            kind,
            client_id: family.id.clone(),
            context,
            family,
        }
    }
}
impl<F> Message<F> {
    /// Return the original packet. Changes to normalized observations do not
    /// silently become control commands or overwrite family-specific data.
    pub fn into_family(self) -> F {
        self.family
    }
}

impl From<Box<Packet>> for Message {
    fn from(family: Box<Packet>) -> Self {
        (*family).into()
    }
}
