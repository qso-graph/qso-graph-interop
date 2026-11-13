// SPDX-License-Identifier: GPL-3.0-or-later
//! Open sans-I/O codec interface. All state, time and transport boundaries are caller-scoped.
#![forbid(unsafe_code)]
use crate::{
    Error,
    model::{Event, Verification},
};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Framing {
    Datagram,
    Stream,
    RequestResponse,
    PublishSubscribe,
    Templated,
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Limits {
    /// Parser limits; `None` means inapplicable to the selected fixed binary class.
    pub parser: ParserLimits,
    pub message_bytes: usize,
    pub field_bytes: usize,
    pub events: usize,
    pub sessions: usize,
    pub templates_per_session: usize,
    pub template_bytes: usize,
    pub dependency_bytes: usize,
}
/// Class-specific parser admission. No implicit unbounded recursion or expansion is permitted.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct ParserLimits {
    pub depth: Option<usize>,
    pub elements: Option<usize>,
    pub attributes: Option<usize>,
    pub numeric_token_bytes: Option<usize>,
    pub decoded_text_bytes: Option<usize>,
}
impl Limits {
    pub const fn stateless(message_bytes: usize, field_bytes: usize) -> Self {
        Self {
            parser: ParserLimits {
                depth: None,
                elements: None,
                attributes: None,
                numeric_token_bytes: None,
                decoded_text_bytes: Some(field_bytes),
            },
            message_bytes,
            field_bytes,
            events: 1,
            sessions: 0,
            templates_per_session: 0,
            template_bytes: 0,
            dependency_bytes: 0,
        }
    }
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DecodeOptions<'a> {
    /// Caller-supplied class metadata, such as a request operation or PUBLISH topic/retain flag.
    pub native: serde_json::Map<String, Value>,
    pub session: &'a [u8],
    pub context: Value,
    /// Caller-selected expiry time or generation; codecs never read a clock.
    pub expiry: Option<u64>,
    pub structured: bool,
}
impl Default for DecodeOptions<'_> {
    fn default() -> Self {
        Self {
            native: serde_json::Map::new(),
            session: &[],
            context: serde_json::json!({"direction":"unknown","exchange":"unknown"}),
            expiry: None,
            structured: false,
        }
    }
}
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct EncodeOptions {
    /// Family-native version selection (WSJT-X: integer `schema`, default 3).
    pub native: serde_json::Map<String, Value>,
}
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Skip {
    pub reason: SkipReason,
    pub native_version: Option<String>,
    pub native_type: Option<u64>,
    /// Literal type name for families whose native discriminator is textual.
    pub native_name: Option<String>,
    pub set_index: Option<usize>,
    pub record_index: Option<usize>,
}
impl Skip {
    pub fn new(
        reason: SkipReason,
        native_version: Option<String>,
        native_type: Option<u64>,
    ) -> Self {
        Self {
            reason,
            native_version,
            native_type,
            native_name: None,
            set_index: None,
            record_index: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum SkipReason {
    FutureVersion,
    UnknownType,
    UnknownTemplate,
}
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct Outcome {
    pub events: Vec<Event>,
    pub skips: Vec<Skip>,
}
impl Outcome {
    pub fn event(event: Event) -> Self {
        Self {
            events: vec![event],
            skips: Vec::new(),
        }
    }
}

/// Optional family interpretation of a preserved event; the core never imports a family.
pub trait Profile {
    fn family(&self) -> &str;
    fn mapping(&self) -> u32;
    fn native_version_supported(&self, _event: &Event) -> bool {
        true
    }
    fn verify(&self, event: &Event) -> Result<Verification, Error>;
    /// Check evidence using caller-owned class metadata, never evidence-view association.
    fn verify_with(
        &self,
        event: &Event,
        _options: &DecodeOptions<'_>,
    ) -> Result<Verification, Error> {
        self.verify(event)
    }
    /// Remove additions recursively from the recognized common view only.
    fn fields(&self, event: &Event) -> Value {
        crate::json::known_fields(event.value())
    }
    fn validate_authored(&self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
    /// Producer validation with explicit request association for request/response families.
    fn validate_authored_with(
        &self,
        event: &Event,
        _options: &DecodeOptions<'_>,
    ) -> Result<(), Error> {
        self.validate_authored(event)
    }
}
/// Implementations must commit state only after the entire native message and every output pass admission.
pub trait Codec: Profile {
    fn framing(&self) -> Framing;
    fn limits(&self) -> Limits;
    fn decode(&mut self, input: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error>;
    fn encode_command(&self, event: &Event, options: &EncodeOptions) -> Result<Vec<u8>, Error>;
    fn reencode(&self, event: &Event, options: &EncodeOptions) -> Result<Vec<u8>, Error>;
    /// Ordered separate native requests. Callers execute and correlate each response.
    fn encode_commands(
        &self,
        event: &Event,
        options: &EncodeOptions,
    ) -> Result<Vec<Vec<u8>>, Error> {
        self.encode_command(event, options).map(|body| vec![body])
    }
    fn reencode_with(
        &self,
        event: &Event,
        options: &EncodeOptions,
        _decode: &DecodeOptions<'_>,
    ) -> Result<Vec<u8>, Error> {
        self.reencode(event, options)
    }
    fn reset(&mut self, session: Option<&[u8]>);
    /// Consume complete frames from the caller-owned buffer. `None` needs more input.
    /// On error the implementation consumes through its documented recovery boundary.
    fn decode_stream(
        &mut self,
        _buffer: &mut Vec<u8>,
        _eof: bool,
        _options: &DecodeOptions<'_>,
    ) -> Result<Option<Outcome>, Error> {
        Err(crate::error::invalid(
            "this codec does not use stream framing",
        ))
    }
}
