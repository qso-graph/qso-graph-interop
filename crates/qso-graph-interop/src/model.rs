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

/// Present text: Null, empty strings, and raw invalid bytes/units remain distinct.
/// Optional field accessors return None only when the member is absent.
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
/// Observer context with typed construction. Event admission validates IDs and budgets.
/// Unknown reader additions are retained.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "Value", into = "Value"))]
#[non_exhaustive]
pub struct Context {
    value: Value,
}
impl Default for Context {
    fn default() -> Self {
        Self::new(Direction::Unknown, Exchange::Unknown)
    }
}
impl Context {
    pub fn new(direction: Direction, exchange: Exchange) -> Self {
        let direction = match direction {
            Direction::FromApplication => "from_application",
            Direction::ToApplication => "to_application",
            Direction::Unknown => "unknown",
        };
        let exchange = match exchange {
            Exchange::Notification => "notification",
            Exchange::Request => "request",
            Exchange::Response => "response",
            Exchange::Unknown => "unknown",
        };
        Self {
            value: serde_json::json!({"direction":direction,"exchange":exchange}),
        }
    }
    pub fn from_value(value: Value) -> Result<Self, Error> {
        crate::json::validate_context(&value, false)?;
        Ok(Self { value })
    }
    pub fn with_source_id(mut self, id: Text) -> Self {
        self.value["source_id"] = id.value();
        self
    }
    pub fn with_target_id(mut self, id: Text) -> Self {
        self.value["target_id"] = id.value();
        self
    }
    pub fn with_correlation_id(mut self, id: Text) -> Self {
        self.value["correlation_id"] = id.value();
        self
    }
    pub fn value(&self) -> Value {
        self.value.clone()
    }
    pub fn as_ref(&self) -> ContextRef<'_> {
        ContextRef::new(&self.value)
    }
}
impl TryFrom<Value> for Context {
    type Error = Error;
    fn try_from(v: Value) -> Result<Self, Error> {
        Self::from_value(v)
    }
}
impl From<Context> for Value {
    fn from(c: Context) -> Value {
        c.value
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
/// Construction admits general-record depth, numeric, compact-size, and reader semantics.
/// Unknown additions remain in the original Value. Serialize preserves this representation;
/// use write with PublicationOptions for authored publication. PartialEq is representation
/// equality; semantic_eq implements specification comparisons.
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
        crate::json::admit_record(&value)?;
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
    ) -> Result<Recognized<'_>, Error> {
        crate::json::recognized(self, profile)
    }
    /// Read a compatible view with caller-owned request association for evidence checking.
    pub fn recognized_with(
        &self,
        profile: Option<&dyn crate::codec::Profile>,
        options: &crate::codec::VerifyOptions,
    ) -> Result<Recognized<'_>, Error> {
        crate::json::recognized_with(self, profile, options)
    }
}
/// An immutable compatible interpretation, borrowing its preserved event.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Recognized<'a> {
    pub(crate) event: &'a Event,
    pub(crate) view: ProfileView,
    pub(crate) context: Value,
}
impl Recognized<'_> {
    pub fn kind(&self) -> Kind {
        self.view.kind
    }
    pub fn fields(&self) -> FieldsRef<'_> {
        FieldsRef::new(self.view.kind, &self.view.fields)
    }
    pub fn context(&self) -> ContextRef<'_> {
        ContextRef::new(&self.context)
    }
    pub fn verification(&self) -> Verification {
        self.view.verification
    }
    pub fn event(&self) -> &Event {
        self.event
    }
}
/// A profile's validated common interpretation. Construction cannot bypass typed-view invariants.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProfileView {
    kind: Kind,
    fields: Value,
    verification: Verification,
}
impl ProfileView {
    pub fn new(
        kind: Kind,
        fields: Map<String, Value>,
        verification: Verification,
    ) -> Result<Self, Error> {
        let fields = Value::Object(fields);
        crate::json::validate_profile_fields(kind, &fields)?;
        Ok(Self {
            kind,
            fields,
            verification,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verification {
    /// A supplied profile established consistency with native evidence and caller association.
    /// This does not authenticate a sender or authorize an operation.
    Verified,
    /// A supported verifier, native interpretation, or required association was unavailable.
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
    pub(crate) instance: Option<Text>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub(crate) number: Option<Integer>,
}
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Hop {
    pub(crate) family: String,
    pub(crate) context: Context,
}
impl Hop {
    pub fn new(context: Context) -> Self {
        Self {
            family: "qgi".into(),
            context,
        }
    }
}

/// Opaque validated evidence. Unknown members survive round trips.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct NativeEvidence {
    value: Value,
}
impl NativeEvidence {
    pub fn new(format: &str, bytes: &[u8]) -> Result<Self, Error> {
        Self::from_value(serde_json::json!({"format":format,"data":crate::base64::encode(bytes)}))
    }
    pub fn from_value(value: Value) -> Result<Self, Error> {
        crate::json::admit_record(&value)?;
        crate::json::validate_native(&value)?;
        Ok(Self { value })
    }
    pub fn data(&self) -> Result<Vec<u8>, Error> {
        NativeRef { value: &self.value }.data()
    }
    pub fn as_value(&self) -> &Value {
        &self.value
    }
    pub fn into_value(self) -> Value {
        self.value
    }
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

impl Integer {
    pub const fn get(self) -> i64 {
        self.0
    }
}
impl Instant {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TimeOfDay {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Radio {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_instance(mut self, instance: Text) -> Self {
        self.instance = Some(instance);
        self
    }
    pub fn with_number(mut self, number: Integer) -> Self {
        self.number = Some(number);
        self
    }
    pub fn instance(&self) -> Option<&Text> {
        self.instance.as_ref()
    }
    pub fn number(&self) -> Option<Integer> {
        self.number
    }
}
impl Hop {
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FamilyRef<'a> {
    Wsjtx,
    N1mm,
    Fldigi,
    Qgi,
    Other(&'a str),
}
impl Event {
    pub fn family(&self) -> FamilyRef<'_> {
        match self.value["family"].as_str().expect("validated family") {
            "wsjtx" => FamilyRef::Wsjtx,
            "n1mm" => FamilyRef::N1mm,
            "fldigi" => FamilyRef::Fldigi,
            "qgi" => FamilyRef::Qgi,
            other => FamilyRef::Other(other),
        }
    }
    pub fn kind_token(&self) -> &str {
        self.value["kind"].as_str().expect("validated kind")
    }
    pub fn mapping(&self) -> u64 {
        crate::json::integer_u64(&self.value["mapping"]).expect("validated mapping")
    }
    pub fn context(&self) -> ContextRef<'_> {
        ContextRef::new(&self.value["context"])
    }
    pub fn wire_id(&self) -> Option<Text> {
        self.value.get("wire_id").map(text_value)
    }
    pub fn native(&self) -> Option<NativeRef<'_>> {
        self.value.get("native").map(|value| NativeRef { value })
    }
    /// Specification equality. PartialEq compares the preserved JSON representation.
    pub fn semantic_eq(&self, other: &Event) -> bool {
        crate::json::semantic_eq(self.value(), other.value())
    }
}
fn text_value(v: &Value) -> Text {
    if let Some(s) = v.as_str() {
        Text::from(s)
    } else if v.is_null() {
        Text::Null
    } else if let Some(a) = v.get("utf8_bytes").and_then(Value::as_array) {
        Text::Utf8 {
            utf8_bytes: a
                .iter()
                .map(|v| crate::json::integer_u64(v).expect("validated byte") as u8)
                .collect(),
        }
    } else {
        Text::Utf16 {
            utf16_units: v["utf16_units"]
                .as_array()
                .expect("validated text")
                .iter()
                .map(|v| crate::json::integer_u64(v).expect("validated unit") as u16)
                .collect(),
        }
    }
}
fn binary_value(v: &Value) -> Binary64 {
    if let Some(f) = v.as_f64() {
        Binary64::from(f)
    } else {
        Binary64::from_bits(
            u64::from_str_radix(
                v.as_str()
                    .expect("validated binary64")
                    .strip_prefix("ieee754:")
                    .expect("validated tag"),
                16,
            )
            .expect("validated bits"),
        )
    }
}
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct NativeRef<'a> {
    value: &'a Value,
}
impl NativeRef<'_> {
    pub fn format(&self) -> &str {
        self.value["format"].as_str().expect("validated format")
    }
    pub fn data(&self) -> Result<Vec<u8>, Error> {
        crate::base64::decode(self.value["data"].as_str().expect("validated data"))
    }
    pub fn message(&self) -> Option<&Map<String, Value>> {
        self.value.get("message").and_then(Value::as_object)
    }
    pub fn as_value(&self) -> &Value {
        self.value
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResultStatus {
    Ok,
    Error,
}

#[derive(Clone, Copy, Debug)]
/// Decoded text and signal measurements; offsets and frequencies use hertz, time deltas use seconds.
#[non_exhaustive]
pub struct DecodeFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> DecodeFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn time_of_day(&self) -> Option<TimeOfDay> {
        self.value
            .get("time_of_day")
            .map(|v| TimeOfDay(v.as_str().expect("validated time").into()))
    }
    pub fn time(&self) -> Option<Instant> {
        self.value
            .get("time")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
    pub fn snr_db(&self) -> Option<Integer> {
        self.value
            .get("snr_db")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn dt_seconds(&self) -> Option<Binary64> {
        self.value.get("dt_seconds").map(binary_value)
    }
    pub fn audio_offset_hz(&self) -> Option<Integer> {
        self.value
            .get("audio_offset_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn mode(&self) -> Option<Text> {
        self.value.get("mode").map(text_value)
    }
    pub fn text(&self) -> Option<Text> {
        self.value.get("text").map(text_value)
    }
    pub fn call(&self) -> Option<Text> {
        self.value.get("call").map(text_value)
    }
    pub fn grid(&self) -> Option<Text> {
        self.value.get("grid").map(text_value)
    }
    pub fn power_dbm(&self) -> Option<Integer> {
        self.value
            .get("power_dbm")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn drift_hz(&self) -> Option<Integer> {
        self.value
            .get("drift_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn is_new(&self) -> Option<bool> {
        self.value
            .get("is_new")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn low_confidence(&self) -> Option<bool> {
        self.value
            .get("low_confidence")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn off_air(&self) -> Option<bool> {
        self.value
            .get("off_air")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn radio(&self) -> Option<RadioRef<'_>> {
        self.value.get("radio").map(|v| RadioRef { value: v })
    }
}

#[derive(Clone, Copy, Debug)]
/// Radio observation; frequencies and offsets use hertz, periods use seconds.
#[non_exhaustive]
pub struct RadioStateFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> RadioStateFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn dial_frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("dial_frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn rx_offset_hz(&self) -> Option<Integer> {
        self.value
            .get("rx_offset_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn tx_offset_hz(&self) -> Option<Integer> {
        self.value
            .get("tx_offset_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn mode(&self) -> Option<Text> {
        self.value.get("mode").map(text_value)
    }
    pub fn tx_mode(&self) -> Option<Text> {
        self.value.get("tx_mode").map(text_value)
    }
    pub fn tx_enabled(&self) -> Option<bool> {
        self.value
            .get("tx_enabled")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn transmitting(&self) -> Option<bool> {
        self.value
            .get("transmitting")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn decoding(&self) -> Option<bool> {
        self.value
            .get("decoding")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn tx_watchdog(&self) -> Option<bool> {
        self.value
            .get("tx_watchdog")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn fast_mode(&self) -> Option<bool> {
        self.value
            .get("fast_mode")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn station_call(&self) -> Option<Text> {
        self.value.get("station_call").map(text_value)
    }
    pub fn station_grid(&self) -> Option<Text> {
        self.value.get("station_grid").map(text_value)
    }
    pub fn dx_call(&self) -> Option<Text> {
        self.value.get("dx_call").map(text_value)
    }
    pub fn dx_grid(&self) -> Option<Text> {
        self.value.get("dx_grid").map(text_value)
    }
    pub fn report(&self) -> Option<Text> {
        self.value.get("report").map(text_value)
    }
    pub fn sub_mode(&self) -> Option<Text> {
        self.value.get("sub_mode").map(text_value)
    }
    pub fn configuration_name(&self) -> Option<Text> {
        self.value.get("configuration_name").map(text_value)
    }
    pub fn tx_message(&self) -> Option<Text> {
        self.value.get("tx_message").map(text_value)
    }
    pub fn band(&self) -> Option<Text> {
        self.value.get("band").map(text_value)
    }
    pub fn frequency_tolerance_hz(&self) -> Option<Integer> {
        self.value
            .get("frequency_tolerance_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn tr_period_seconds(&self) -> Option<Integer> {
        self.value
            .get("tr_period_seconds")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn radio(&self) -> Option<RadioRef<'_>> {
        self.value.get("radio").map(|v| RadioRef { value: v })
    }
    pub fn focus_radio(&self) -> Option<Integer> {
        self.value
            .get("focus_radio")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn active_radio(&self) -> Option<Integer> {
        self.value
            .get("active_radio")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn running(&self) -> Option<bool> {
        self.value
            .get("running")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn stereo(&self) -> Option<bool> {
        self.value
            .get("stereo")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
}

#[derive(Clone, Copy, Debug)]
/// Shared logged, changed, and deleted QSO fields, including N1MM previous_start.
#[non_exhaustive]
pub struct QsoFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> QsoFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn record_id(&self) -> Option<Text> {
        self.value.get("record_id").map(text_value)
    }
    pub fn call(&self) -> Option<Text> {
        self.value.get("call").map(text_value)
    }
    pub fn grid(&self) -> Option<Text> {
        self.value.get("grid").map(text_value)
    }
    pub fn mode(&self) -> Option<Text> {
        self.value.get("mode").map(text_value)
    }
    pub fn station_call(&self) -> Option<Text> {
        self.value.get("station_call").map(text_value)
    }
    pub fn station_grid(&self) -> Option<Text> {
        self.value.get("station_grid").map(text_value)
    }
    pub fn operator_call(&self) -> Option<Text> {
        self.value.get("operator_call").map(text_value)
    }
    pub fn report_sent(&self) -> Option<Text> {
        self.value.get("report_sent").map(text_value)
    }
    pub fn report_received(&self) -> Option<Text> {
        self.value.get("report_received").map(text_value)
    }
    pub fn exchange_sent(&self) -> Option<Text> {
        self.value.get("exchange_sent").map(text_value)
    }
    pub fn exchange_received(&self) -> Option<Text> {
        self.value.get("exchange_received").map(text_value)
    }
    pub fn start(&self) -> Option<Instant> {
        self.value
            .get("start")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
    pub fn end(&self) -> Option<Instant> {
        self.value
            .get("end")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
    pub fn tx_frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("tx_frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn dial_frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("dial_frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn band(&self) -> Option<Text> {
        self.value.get("band").map(text_value)
    }
    pub fn contest_name(&self) -> Option<Text> {
        self.value.get("contest_name").map(text_value)
    }
    pub fn serial_sent(&self) -> Option<Text> {
        self.value.get("serial_sent").map(text_value)
    }
    pub fn serial_received(&self) -> Option<Text> {
        self.value.get("serial_received").map(text_value)
    }
    pub fn section(&self) -> Option<Text> {
        self.value.get("section").map(text_value)
    }
    pub fn precedence(&self) -> Option<Text> {
        self.value.get("precedence").map(text_value)
    }
    pub fn check(&self) -> Option<Text> {
        self.value.get("check").map(text_value)
    }
    pub fn tx_power(&self) -> Option<Text> {
        self.value.get("tx_power").map(text_value)
    }
    pub fn comments(&self) -> Option<Text> {
        self.value.get("comments").map(text_value)
    }
    pub fn name(&self) -> Option<Text> {
        self.value.get("name").map(text_value)
    }
    pub fn propagation_mode(&self) -> Option<Text> {
        self.value.get("propagation_mode").map(text_value)
    }
    pub fn adif(&self) -> Option<Text> {
        self.value.get("adif").map(text_value)
    }
    pub fn radio(&self) -> Option<RadioRef<'_>> {
        self.value.get("radio").map(|v| RadioRef { value: v })
    }
    pub fn previous_start(&self) -> Option<Instant> {
        self.value
            .get("previous_start")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct SpotFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> SpotFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn call(&self) -> Option<Text> {
        self.value.get("call").map(text_value)
    }
    pub fn grid(&self) -> Option<Text> {
        self.value.get("grid").map(text_value)
    }
    pub fn mode(&self) -> Option<Text> {
        self.value.get("mode").map(text_value)
    }
    pub fn text(&self) -> Option<Text> {
        self.value.get("text").map(text_value)
    }
    pub fn spotter_call(&self) -> Option<Text> {
        self.value.get("spotter_call").map(text_value)
    }
    pub fn source(&self) -> Option<Text> {
        self.value.get("source").map(text_value)
    }
    pub fn frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn time(&self) -> Option<Instant> {
        self.value
            .get("time")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
    pub fn time_of_day(&self) -> Option<TimeOfDay> {
        self.value
            .get("time_of_day")
            .map(|v| TimeOfDay(v.as_str().expect("validated time").into()))
    }
    pub fn receiver_call(&self) -> Option<Text> {
        self.value.get("receiver_call").map(text_value)
    }
    pub fn receiver_locator(&self) -> Option<Text> {
        self.value.get("receiver_locator").map(text_value)
    }
    pub fn sender_call(&self) -> Option<Text> {
        self.value.get("sender_call").map(text_value)
    }
    pub fn sender_locator(&self) -> Option<Text> {
        self.value.get("sender_locator").map(text_value)
    }
    pub fn reporting_software(&self) -> Option<Text> {
        self.value.get("reporting_software").map(text_value)
    }
    pub fn snr_db(&self) -> Option<Integer> {
        self.value
            .get("snr_db")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn radio(&self) -> Option<RadioRef<'_>> {
        self.value.get("radio").map(|v| RadioRef { value: v })
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ScoreFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> ScoreFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn contest_name(&self) -> Option<Text> {
        self.value.get("contest_name").map(text_value)
    }
    pub fn station_call(&self) -> Option<Text> {
        self.value.get("station_call").map(text_value)
    }
    pub fn band(&self) -> Option<Text> {
        self.value.get("band").map(text_value)
    }
    pub fn mode(&self) -> Option<Text> {
        self.value.get("mode").map(text_value)
    }
    pub fn qso_count(&self) -> Option<Integer> {
        self.value
            .get("qso_count")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn multiplier_count(&self) -> Option<Integer> {
        self.value
            .get("multiplier_count")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn points(&self) -> Option<Integer> {
        self.value
            .get("points")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn total_score(&self) -> Option<Integer> {
        self.value
            .get("total_score")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn time(&self) -> Option<Instant> {
        self.value
            .get("time")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
}

#[derive(Clone, Copy, Debug)]
/// Command fields, including fldigi RX byte-range start and length.
#[non_exhaustive]
pub struct ControlFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> ControlFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn radio(&self) -> Option<RadioRef<'_>> {
        self.value.get("radio").map(|v| RadioRef { value: v })
    }
    pub fn operation(&self) -> &str {
        let v = &self.value["operation"];
        v.as_str().expect("validated token")
    }
    pub fn window(&self) -> Option<Integer> {
        self.value
            .get("window")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn time_of_day(&self) -> Option<TimeOfDay> {
        self.value
            .get("time_of_day")
            .map(|v| TimeOfDay(v.as_str().expect("validated time").into()))
    }
    pub fn snr_db(&self) -> Option<Integer> {
        self.value
            .get("snr_db")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn dt_seconds(&self) -> Option<Binary64> {
        self.value.get("dt_seconds").map(binary_value)
    }
    pub fn audio_offset_hz(&self) -> Option<Integer> {
        self.value
            .get("audio_offset_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn rx_offset_hz(&self) -> Option<Integer> {
        self.value
            .get("rx_offset_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn frequency_tolerance_hz(&self) -> Option<Integer> {
        self.value
            .get("frequency_tolerance_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn mode(&self) -> Option<Text> {
        self.value.get("mode").map(text_value)
    }
    pub fn text(&self) -> Option<Text> {
        self.value.get("text").map(text_value)
    }
    pub fn sub_mode(&self) -> Option<Text> {
        self.value.get("sub_mode").map(text_value)
    }
    pub fn dx_call(&self) -> Option<Text> {
        self.value.get("dx_call").map(text_value)
    }
    pub fn dx_grid(&self) -> Option<Text> {
        self.value.get("dx_grid").map(text_value)
    }
    pub fn location(&self) -> Option<Text> {
        self.value.get("location").map(text_value)
    }
    pub fn call(&self) -> Option<Text> {
        self.value.get("call").map(text_value)
    }
    pub fn configuration_name(&self) -> Option<Text> {
        self.value.get("configuration_name").map(text_value)
    }
    pub fn low_confidence(&self) -> Option<bool> {
        self.value
            .get("low_confidence")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn auto_tx_only(&self) -> Option<bool> {
        self.value
            .get("auto_tx_only")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn highlight_last(&self) -> Option<bool> {
        self.value
            .get("highlight_last")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn fast_mode(&self) -> Option<bool> {
        self.value
            .get("fast_mode")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn generate_messages(&self) -> Option<bool> {
        self.value
            .get("generate_messages")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn sort_order_provided(&self) -> Option<bool> {
        self.value
            .get("sort_order_provided")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn modifiers(&self) -> Option<Integer> {
        self.value
            .get("modifiers")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn sort_order(&self) -> Option<Integer> {
        self.value
            .get("sort_order")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn tr_period_seconds(&self) -> Option<Integer> {
        self.value
            .get("tr_period_seconds")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn transmit(&self) -> Option<bool> {
        self.value
            .get("transmit")
            .map(|v| v.as_bool().expect("validated boolean"))
    }
    pub fn max_schema(&self) -> Option<Integer> {
        self.value
            .get("max_schema")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn version(&self) -> Option<Text> {
        self.value.get("version").map(text_value)
    }
    pub fn revision(&self) -> Option<Text> {
        self.value.get("revision").map(text_value)
    }
    pub fn background_color(&self) -> Option<ColorRef<'_>> {
        self.value
            .get("background_color")
            .map(|v| ColorRef { value: v })
    }
    pub fn foreground_color(&self) -> Option<ColorRef<'_>> {
        self.value
            .get("foreground_color")
            .map(|v| ColorRef { value: v })
    }
    pub fn rx_start(&self) -> Option<Integer> {
        self.value
            .get("rx_start")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn rx_length(&self) -> Option<Integer> {
        self.value
            .get("rx_length")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn app(&self) -> Option<Text> {
        self.value.get("app").map(text_value)
    }
    pub fn spectrum_name(&self) -> Option<Text> {
        self.value.get("spectrum_name").map(text_value)
    }
    pub fn low_scope_frequency(&self) -> Option<Integer> {
        self.value
            .get("low_scope_frequency")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn high_scope_frequency(&self) -> Option<Integer> {
        self.value
            .get("high_scope_frequency")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn scaling_factor(&self) -> Option<Text> {
        self.value.get("scaling_factor").map(text_value)
    }
    pub fn spectrum_data(&self) -> Option<Vec<u16>> {
        self.value.get("spectrum_data").map(|v| {
            v.as_array()
                .expect("validated levels")
                .iter()
                .map(|v| crate::json::integer_u64(v).expect("validated level") as u16)
                .collect()
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ResultFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> ResultFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn status(&self) -> ResultStatus {
        let v = &self.value["status"];
        match v.as_str() {
            Some("ok") => ResultStatus::Ok,
            _ => ResultStatus::Error,
        }
    }
    pub fn operation(&self) -> Option<&str> {
        self.value
            .get("operation")
            .map(|v| v.as_str().expect("validated token"))
    }
    pub fn native_code(&self) -> Option<Text> {
        self.value.get("native_code").map(text_value)
    }
    pub fn text(&self) -> Option<Text> {
        self.value.get("text").map(text_value)
    }
    pub fn frequency_hz(&self) -> Option<Integer> {
        self.value
            .get("frequency_hz")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct NotificationFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> NotificationFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn operation(&self) -> &str {
        let v = &self.value["operation"];
        v.as_str().expect("validated token")
    }
    pub fn window(&self) -> Option<Integer> {
        self.value
            .get("window")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct LifecycleFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> LifecycleFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn state(&self) -> &str {
        let v = &self.value["state"];
        v.as_str().expect("validated token")
    }
    pub fn max_schema(&self) -> Option<Integer> {
        self.value
            .get("max_schema")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn version(&self) -> Option<Text> {
        self.value.get("version").map(text_value)
    }
    pub fn revision(&self) -> Option<Text> {
        self.value.get("revision").map(text_value)
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct AmbiguousFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> AmbiguousFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn operation(&self) -> &str {
        let v = &self.value["operation"];
        v.as_str().expect("validated token")
    }
    pub fn window(&self) -> Option<Integer> {
        self.value
            .get("window")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
}

#[derive(Clone, Copy, Debug)]
/// Known native-kind family fields; opaque evidence remains on Event.
#[non_exhaustive]
pub struct NativeFieldsRef<'a> {
    value: &'a Value,
}
impl<'a> NativeFieldsRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn start(&self) -> Option<Instant> {
        self.value
            .get("start")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
    pub fn previous_start(&self) -> Option<Instant> {
        self.value
            .get("previous_start")
            .map(|v| Instant(v.as_str().expect("validated instant").into()))
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct RadioRef<'a> {
    value: &'a Value,
}
impl<'a> RadioRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn instance(&self) -> Option<Text> {
        self.value.get("instance").map(text_value)
    }
    pub fn number(&self) -> Option<Integer> {
        self.value
            .get("number")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ColorRef<'a> {
    value: &'a Value,
}
impl<'a> ColorRef<'a> {
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn spec(&self) -> Option<Integer> {
        self.value
            .get("spec")
            .map(|v| Integer(crate::json::integer_i64(v).expect("validated integer")))
    }
    pub fn channels(&self) -> Option<&[Value]> {
        self.value
            .get("channels")
            .map(|v| v.as_array().expect("validated array").as_slice())
    }
}

#[derive(Clone, Copy, Debug)]
/// Borrowed observer context, including unknown tokens. Recognized supplies compatibility fallback.
#[non_exhaustive]
pub struct ContextRef<'a> {
    value: &'a Value,
}
impl<'a> ContextRef<'a> {
    pub(crate) fn new(value: &'a Value) -> Self {
        Self { value }
    }
    /// The projected, validated JSON view. Unknown additions remain on Event.
    pub fn as_value(&self) -> &Value {
        self.value
    }
    pub fn direction(&self) -> DirectionRef<'_> {
        match self.value["direction"]
            .as_str()
            .expect("validated direction")
        {
            "from_application" => DirectionRef::FromApplication,
            "to_application" => DirectionRef::ToApplication,
            "unknown" => DirectionRef::Unknown,
            other => DirectionRef::Other(other),
        }
    }
    pub fn exchange(&self) -> ExchangeRef<'_> {
        match self.value["exchange"].as_str().expect("validated exchange") {
            "notification" => ExchangeRef::Notification,
            "request" => ExchangeRef::Request,
            "response" => ExchangeRef::Response,
            "unknown" => ExchangeRef::Unknown,
            other => ExchangeRef::Other(other),
        }
    }
    pub fn source_id(&self) -> Option<Text> {
        self.value.get("source_id").map(text_value)
    }
    pub fn target_id(&self) -> Option<Text> {
        self.value.get("target_id").map(text_value)
    }
    pub fn correlation_id(&self) -> Option<Text> {
        self.value.get("correlation_id").map(text_value)
    }
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum FieldsRef<'a> {
    Decode(DecodeFieldsRef<'a>),
    RadioState(RadioStateFieldsRef<'a>),
    QsoLogged(QsoFieldsRef<'a>),
    QsoChanged(QsoFieldsRef<'a>),
    QsoDeleted(QsoFieldsRef<'a>),
    Spot(SpotFieldsRef<'a>),
    Score(ScoreFieldsRef<'a>),
    Control(ControlFieldsRef<'a>),
    Result(ResultFieldsRef<'a>),
    Notification(NotificationFieldsRef<'a>),
    Lifecycle(LifecycleFieldsRef<'a>),
    Ambiguous(AmbiguousFieldsRef<'a>),
    Native(NativeFieldsRef<'a>),
}
impl<'a> FieldsRef<'a> {
    fn new(kind: Kind, value: &'a Value) -> Self {
        match kind {
            Kind::Decode => Self::Decode(DecodeFieldsRef { value }),
            Kind::RadioState => Self::RadioState(RadioStateFieldsRef { value }),
            Kind::QsoLogged => Self::QsoLogged(QsoFieldsRef { value }),
            Kind::QsoChanged => Self::QsoChanged(QsoFieldsRef { value }),
            Kind::QsoDeleted => Self::QsoDeleted(QsoFieldsRef { value }),
            Kind::Spot => Self::Spot(SpotFieldsRef { value }),
            Kind::Score => Self::Score(ScoreFieldsRef { value }),
            Kind::Control => Self::Control(ControlFieldsRef { value }),
            Kind::Result => Self::Result(ResultFieldsRef { value }),
            Kind::Notification => Self::Notification(NotificationFieldsRef { value }),
            Kind::Lifecycle => Self::Lifecycle(LifecycleFieldsRef { value }),
            Kind::Ambiguous => Self::Ambiguous(AmbiguousFieldsRef { value }),
            Kind::Native => Self::Native(NativeFieldsRef { value }),
        }
    }
    pub fn as_value(&self) -> &Value {
        match self {
            Self::Decode(view) => view.value,
            Self::RadioState(view) => view.value,
            Self::QsoLogged(view) => view.value,
            Self::QsoChanged(view) => view.value,
            Self::QsoDeleted(view) => view.value,
            Self::Spot(view) => view.value,
            Self::Score(view) => view.value,
            Self::Control(view) => view.value,
            Self::Result(view) => view.value,
            Self::Notification(view) => view.value,
            Self::Lifecycle(view) => view.value,
            Self::Ambiguous(view) => view.value,
            Self::Native(view) => view.value,
        }
    }
}
impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decode => "decode",
            Self::RadioState => "radio_state",
            Self::QsoLogged => "qso_logged",
            Self::QsoChanged => "qso_changed",
            Self::QsoDeleted => "qso_deleted",
            Self::Spot => "spot",
            Self::Score => "score",
            Self::Control => "control",
            Self::Result => "result",
            Self::Notification => "notification",
            Self::Lifecycle => "lifecycle",
            Self::Ambiguous => "ambiguous",
            Self::Native => "native",
        }
    }
    pub(crate) fn from_token(token: &str) -> Option<Self> {
        match token {
            "decode" => Some(Self::Decode),
            "radio_state" => Some(Self::RadioState),
            "qso_logged" => Some(Self::QsoLogged),
            "qso_changed" => Some(Self::QsoChanged),
            "qso_deleted" => Some(Self::QsoDeleted),
            "spot" => Some(Self::Spot),
            "score" => Some(Self::Score),
            "control" => Some(Self::Control),
            "result" => Some(Self::Result),
            "notification" => Some(Self::Notification),
            "lifecycle" => Some(Self::Lifecycle),
            "ambiguous" => Some(Self::Ambiguous),
            "native" => Some(Self::Native),
            _ => None,
        }
    }
}

/// Observed direction, preserving future tokens outside a recognized interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DirectionRef<'a> {
    FromApplication,
    ToApplication,
    Unknown,
    Other(&'a str),
}
/// Observed exchange, preserving future tokens outside a recognized interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExchangeRef<'a> {
    Notification,
    Request,
    Response,
    Unknown,
    Other(&'a str),
}
