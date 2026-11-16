// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
//! Wire fields in specification order. Optional tails preserve absent versus null.
use super::qt::{QColor, QDateTime, QTime, Utf8};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Heartbeat {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_schema: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub revision: Option<Utf8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Status {
    pub dial_frequency_hz: u64,
    pub mode: Utf8,
    pub dx_call: Utf8,
    pub report: Utf8,
    pub tx_mode: Utf8,
    pub tx_enabled: bool,
    pub transmitting: bool,
    pub decoding: bool,
    pub rx_df_hz: u32,
    pub tx_df_hz: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub de_call: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub de_grid: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dx_grid: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tx_watchdog: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sub_mode: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fast_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub special_operation_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub frequency_tolerance_hz: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tr_period_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub configuration_name: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tx_message: Option<Utf8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Decode {
    pub new: bool,
    pub time: QTime,
    pub snr_db: i32,
    #[serde(with = "crate::wsjtx::float64")]
    pub delta_time_seconds: f64,
    pub delta_frequency_hz: u32,
    pub mode: Utf8,
    pub message: Utf8,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub low_confidence: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub off_air: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Clear {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub window: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Reply {
    pub time: QTime,
    pub snr_db: i32,
    #[serde(with = "crate::wsjtx::float64")]
    pub delta_time_seconds: f64,
    pub delta_frequency_hz: u32,
    pub mode: Utf8,
    pub message: Utf8,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub low_confidence: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub modifiers: Option<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QsoLogged {
    pub time_off: QDateTime,
    pub dx_call: Utf8,
    pub dx_grid: Utf8,
    pub frequency_hz: u64,
    pub mode: Utf8,
    pub report_sent: Utf8,
    pub report_received: Utf8,
    pub tx_power: Utf8,
    pub comments: Utf8,
    pub name: Utf8,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_on: Option<QDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub operator_call: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub my_call: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub my_grid: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exchange_sent: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub exchange_received: Option<Utf8>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub propagation_mode: Option<Utf8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HaltTx {
    pub auto_tx_only: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FreeText {
    pub text: Utf8,
    pub send: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WsprDecode {
    pub new: bool,
    pub time: QTime,
    pub snr_db: i32,
    #[serde(with = "crate::wsjtx::float64")]
    pub delta_time_seconds: f64,
    pub frequency_hz: u64,
    pub drift_hz: i32,
    pub callsign: Utf8,
    pub grid: Utf8,
    pub power_dbm: i32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub off_air: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Location {
    pub location: Utf8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoggedAdif {
    pub adif: Utf8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HighlightCallsign {
    pub callsign: Utf8,
    pub background: QColor,
    pub foreground: QColor,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub highlight_last: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SwitchConfiguration {
    pub configuration_name: Utf8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Configure {
    pub mode: Utf8,
    pub frequency_tolerance_hz: u32,
    pub sub_mode: Utf8,
    pub fast_mode: bool,
    pub tr_period_seconds: u32,
    pub rx_df_hz: u32,
    pub dx_call: Utf8,
    pub dx_grid: Utf8,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub generate_messages: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AnnotationInfo {
    pub dx_call: Utf8,
    pub sort_order_provided: bool,
    pub sort_order: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Body {
    Heartbeat(Heartbeat),
    Status(Status),
    Decode(Decode),
    Clear(Clear),
    Reply(Reply),
    QsoLogged(Box<QsoLogged>),
    HaltTx(HaltTx),
    FreeText(FreeText),
    WsprDecode(WsprDecode),
    Location(Location),
    LoggedAdif(LoggedAdif),
    HighlightCallsign(HighlightCallsign),
    SwitchConfiguration(SwitchConfiguration),
    Configure(Configure),
    AnnotationInfo(AnnotationInfo),
    Close,
    Replay,
}

impl Body {
    pub fn message_type(&self) -> u32 {
        match self {
            Self::Heartbeat(..) => 0,
            Self::Status(..) => 1,
            Self::Decode(..) => 2,
            Self::Clear(..) => 3,
            Self::Reply(..) => 4,
            Self::QsoLogged(..) => 5,
            Self::HaltTx(..) => 8,
            Self::FreeText(..) => 9,
            Self::WsprDecode(..) => 10,
            Self::Location(..) => 11,
            Self::LoggedAdif(..) => 12,
            Self::HighlightCallsign(..) => 13,
            Self::SwitchConfiguration(..) => 14,
            Self::Configure(..) => 15,
            Self::AnnotationInfo(..) => 16,
            Self::Close => 6,
            Self::Replay => 7,
        }
    }
}
