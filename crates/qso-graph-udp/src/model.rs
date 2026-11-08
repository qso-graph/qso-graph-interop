// SPDX-License-Identifier: GPL-3.0-or-later
//! A first common model. Normalized values are observations, never inferred calls
//! or timestamps. Every event owns its complete family packet for lossless return.
#![forbid(unsafe_code)]

use crate::wsjtx::{Body, Packet, QDateTime, QTime, Utf8, Variant};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub kind: Kind,
    pub source_id: Utf8,
    pub schema: u32,
    pub variant: Variant,
    pub family: Packet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    LoggedQso(Box<LoggedQso>),
    RadioState(RadioState),
    Decode(Decode),
    Control(Control),
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
    pub target_id: Utf8,
    /// The native command and all its arguments are in `Message::family.body`.
    pub message_type: u32,
}

impl From<Packet> for Message {
    fn from(family: Packet) -> Self {
        let kind = match &family.body {
            Body::QsoLogged(q) => Kind::LoggedQso(Box::new(LoggedQso {
                call: Some(q.dx_call.clone()),
                grid: Some(q.dx_grid.clone()),
                mode: Some(q.mode.clone()),
                tx_frequency_hz: (family.variant == Variant::Wsjtx).then_some(q.frequency_hz),
                dial_frequency_hz: (family.variant == Variant::Jtdx).then_some(q.frequency_hz),
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
            Body::JtdxStatus(s) => Kind::RadioState(RadioState {
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
            Body::Heartbeat(_) | Body::Close => Kind::Lifecycle,
            _ => Kind::Control(Control {
                target_id: family.id.clone(),
                message_type: family.body.message_type(),
            }),
        };
        Self {
            kind,
            source_id: family.id.clone(),
            schema: family.schema,
            variant: family.variant,
            family,
        }
    }
}
impl Message {
    /// Return the original packet. Changes to normalized observations do not
    /// silently become control commands or overwrite family-specific data.
    pub fn into_family(self) -> Packet {
        self.family
    }
}

impl From<Box<Packet>> for Message {
    fn from(family: Box<Packet>) -> Self {
        (*family).into()
    }
}
