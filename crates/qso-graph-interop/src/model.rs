// SPDX-License-Identifier: GPL-3.0-or-later
//! Neutral event envelopes and values. No native-family types belong here.
#![forbid(unsafe_code)]
use crate::{Error, error::invalid};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const SPEC: &str = "0.1.0";
pub const MAX_RECORD: usize = 1_048_576;
pub const MAX_CONTEXT: usize = 4_096;
pub const MAX_INTEGER: i64 = 9_007_199_254_740_991;

/// A present text value. Absence is represented by an omitted member, never `Null`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[non_exhaustive]
pub enum Text {
    String(String),
    Null,
    Utf8 { utf8_bytes: Vec<u8> },
    Utf16 { utf16_units: Vec<u16> },
}
impl Text {
    pub fn utf8(bytes: Option<&[u8]>) -> Self {
        match bytes {
            None => Self::Null,
            Some(b) => match std::str::from_utf8(b) {
                Ok(s) => Self::String(s.into()),
                Err(_) => Self::Utf8 {
                    utf8_bytes: b.to_vec(),
                },
            },
        }
    }
    pub fn utf16(units: Option<&[u16]>) -> Self {
        match units {
            None => Self::Null,
            Some(u) => match String::from_utf16(u) {
                Ok(s) => Self::String(s),
                Err(_) => Self::Utf16 {
                    utf16_units: u.to_vec(),
                },
            },
        }
    }
}
impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "i64", into = "i64"))]
#[non_exhaustive]
pub struct Integer(i64);
impl TryFrom<i64> for Integer {
    type Error = Error;
    fn try_from(n: i64) -> Result<Self, Error> {
        if (-MAX_INTEGER..=MAX_INTEGER).contains(&n) {
            Ok(Self(n))
        } else {
            Err(invalid("common integer out of range"))
        }
    }
}
impl From<Integer> for i64 {
    fn from(n: Integer) -> i64 {
        n.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
#[non_exhaustive]
pub struct TimeOfDay(String);
impl TryFrom<String> for TimeOfDay {
    type Error = Error;
    fn try_from(s: String) -> Result<Self, Error> {
        if crate::json::valid_clock(&s) {
            Ok(Self(s))
        } else {
            Err(invalid("invalid UTC time of day"))
        }
    }
}
impl From<TimeOfDay> for String {
    fn from(t: TimeOfDay) -> String {
        t.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
#[non_exhaustive]
pub struct Instant(String);
impl TryFrom<String> for Instant {
    type Error = Error;
    fn try_from(s: String) -> Result<Self, Error> {
        if crate::json::valid_instant(&s) {
            Ok(Self(s))
        } else {
            Err(invalid("invalid UTC instant"))
        }
    }
}
impl From<Instant> for String {
    fn from(t: Instant) -> String {
        t.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Direction {
    FromApplication,
    ToApplication,
    #[default]
    Unknown,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Exchange {
    Notification,
    Request,
    Response,
    #[default]
    Unknown,
}
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Context {
    pub direction: Direction,
    pub exchange: Exchange,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            deserialize_with = "present_text"
        )
    )]
    pub source_id: Option<Text>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            deserialize_with = "present_text"
        )
    )]
    pub target_id: Option<Text>,
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            default,
            deserialize_with = "present_text"
        )
    )]
    pub correlation_id: Option<Text>,
}
impl Context {
    pub fn new(direction: Direction, exchange: Exchange) -> Self {
        Self {
            direction,
            exchange,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum Kind {
    Decode,
    RadioState,
    QsoLogged,
    QsoChanged,
    QsoDeleted,
    Spot,
    Score,
    Control,
    Result,
    Notification,
    Lifecycle,
    Ambiguous,
    Native,
}

/// A preserved event. Unknown additions are retained; use `recognized` for a compatible view.
/// Construction validates the reader profile. Publication validates the producer profile explicitly.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[non_exhaustive]
pub struct Event {
    value: Value,
}
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Event {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = Box::<serde_json::value::RawValue>::deserialize(d)?;
        crate::json::parse(raw.get().as_bytes(), MAX_RECORD).map_err(serde::de::Error::custom)
    }
}
impl Event {
    pub fn from_value(value: Value) -> Result<Self, Error> {
        crate::json::validate(&value, false)?;
        Ok(Self { value })
    }
    pub fn command(
        family: &str,
        mapping: u32,
        context: Context,
        fields: Map<String, Value>,
        wire_id: Option<Text>,
    ) -> Result<Self, Error> {
        let mut v = serde_json::json!({"spec":SPEC,"family":family,"mapping":mapping,"kind":"control","context":context.value(),"fields":fields});
        if let Some(id) = wire_id {
            v["wire_id"] = id.value();
        }
        Self::from_value(v)
    }
    pub fn value(&self) -> &Value {
        &self.value
    }
    pub fn into_value(self) -> Value {
        self.value
    }
    pub fn is_command(&self) -> bool {
        self.value.get("native").is_none()
    }
    pub fn native_bytes(&self) -> Result<Vec<u8>, Error> {
        crate::base64::decode(
            self.value["native"]["data"]
                .as_str()
                .ok_or_else(|| invalid("event has no native evidence"))?,
        )
    }
    pub fn recognized(
        &self,
        profile: Option<&dyn crate::codec::Profile>,
    ) -> Result<Recognized, Error> {
        crate::json::recognized(self, profile)
    }
}
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Recognized {
    pub kind: String,
    pub fields: Value,
    pub context: Value,
    pub native: Option<Value>,
    pub verification: Verification,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verification {
    Verified,
    Unavailable,
}

#[cfg(feature = "serde")]
fn present_text<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Text>, D::Error> {
    Text::deserialize(d).map(Some)
}

/// Equipment label carried by a family; it never identifies an authenticated sender.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Radio {
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present_text"
        )
    )]
    pub instance: Option<Text>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub number: Option<Integer>,
}
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Hop {
    pub family: String,
    pub context: Context,
}
impl Hop {
    pub fn new(context: Context) -> Self {
        Self {
            family: "qgi".into(),
            context,
        }
    }
}

/// Exact evidence at the protocol-class boundary. Binary members are canonical padded base64.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Native {
    pub format: String,
    pub data: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub message: Option<Value>,
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present_text"
        )
    )]
    pub topic: Option<Text>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub retained: Option<bool>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub template: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub header: Option<RecordHeader>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub message_data: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub set_id: Option<u16>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub set_index: Option<Integer>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub record_index: Option<Integer>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dependencies: Option<Vec<Dependency>>,
}
impl Native {
    pub fn new(format: impl Into<String>, data: &[u8]) -> Self {
        Self {
            format: format.into(),
            data: crate::base64::encode(data),
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct RecordHeader {
    pub version: u16,
    pub length: u16,
    pub export_time: u32,
    pub sequence_number: u32,
    pub observation_domain_id: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Dependency {
    pub data: String,
    pub template: String,
    pub header: RecordHeader,
    pub set_id: u16,
}

/// IEEE 754 binary64, preserving nonfinite payloads and negative zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Binary64(u64);
impl Binary64 {
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
    pub const fn to_bits(self) -> u64 {
        self.0
    }
    pub const fn value(self) -> f64 {
        f64::from_bits(self.0)
    }
}
impl From<f64> for Binary64 {
    fn from(f: f64) -> Self {
        Self(f.to_bits())
    }
}
#[cfg(feature = "serde")]
impl Serialize for Binary64 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.value().is_finite() {
            s.serialize_f64(self.value())
        } else {
            s.serialize_str(&format!("ieee754:{:016x}", self.0))
        }
    }
}
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Binary64 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        if let Some(f) = v.as_f64().filter(|f| f.is_finite()) {
            return Ok(Self::from(f));
        }
        let s = v
            .as_str()
            .and_then(|s| s.strip_prefix("ieee754:"))
            .filter(|s| {
                s.len() == 16
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
            })
            .ok_or_else(|| serde::de::Error::custom("invalid binary64 representation"))?;
        let n = u64::from_str_radix(s, 16).map_err(serde::de::Error::custom)?;
        if f64::from_bits(n).is_finite() {
            return Err(serde::de::Error::custom(
                "finite binary64 must use a number",
            ));
        }
        Ok(Self(n))
    }
}

impl Text {
    /// The specification text representation, independent of optional serde integration.
    pub fn value(&self) -> Value {
        match self {
            Self::String(s) => Value::String(s.clone()),
            Self::Null => Value::Null,
            Self::Utf8 { utf8_bytes } => serde_json::json!({"utf8_bytes":utf8_bytes}),
            Self::Utf16 { utf16_units } => serde_json::json!({"utf16_units":utf16_units}),
        }
    }
}
impl Context {
    /// The observer context representation, preserving present null and empty IDs.
    pub fn value(&self) -> Value {
        let direction = match self.direction {
            Direction::FromApplication => "from_application",
            Direction::ToApplication => "to_application",
            Direction::Unknown => "unknown",
        };
        let exchange = match self.exchange {
            Exchange::Notification => "notification",
            Exchange::Request => "request",
            Exchange::Response => "response",
            Exchange::Unknown => "unknown",
        };
        let mut v = serde_json::json!({"direction":direction,"exchange":exchange});
        for (key, text) in [
            ("source_id", &self.source_id),
            ("target_id", &self.target_id),
            ("correlation_id", &self.correlation_id),
        ] {
            if let Some(text) = text {
                v[key] = text.value();
            }
        }
        v
    }
}
