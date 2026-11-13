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
/// Caller-owned association for one decode or verification operation.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct DecodeMetadata {
    pub(crate) native: serde_json::Map<String, Value>,
}
impl DecodeMetadata {
    /// Escape hatch for metadata defined by an external codec's protocol class.
    pub fn extension_metadata(native: serde_json::Map<String, Value>) -> Self {
        Self { native }
    }
    pub fn extensions(&self) -> &serde_json::Map<String, Value> {
        &self.native
    }
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DecodeOptions<'a> {
    pub(crate) native: serde_json::Map<String, Value>,
    // API-2: reconstructed hops are not request association or extension metadata.
    pub(crate) via: Option<Value>,
    pub(crate) session: &'a [u8],
    pub(crate) context: Value,
    pub(crate) expiry: Option<u64>,
    pub(crate) structured: bool,
}
impl Default for DecodeOptions<'_> {
    fn default() -> Self {
        Self::new(crate::Context::default())
    }
}
impl<'a> DecodeOptions<'a> {
    pub fn new(context: crate::Context) -> Self {
        Self {
            native: Default::default(),
            via: None,
            session: &[],
            context: context.value(),
            expiry: None,
            structured: false,
        }
    }
    pub fn with_context(mut self, context: crate::Context) -> Self {
        self.context = context.value();
        self
    }
    pub fn with_metadata(mut self, metadata: DecodeMetadata) -> Self {
        self.native = metadata.native;
        self
    }
    pub fn with_structured(mut self, enabled: bool) -> Self {
        self.structured = enabled;
        self
    }
    pub fn with_session(mut self, session: &'a [u8]) -> Self {
        self.session = session;
        self
    }
    pub fn with_expiry(mut self, expiry: Option<u64>) -> Self {
        self.expiry = expiry;
        self
    }
    pub fn context(&self) -> crate::model::ContextRef<'_> {
        crate::model::ContextRef::new(&self.context)
    }
    pub fn metadata(&self) -> DecodeMetadata {
        DecodeMetadata::extension_metadata(self.native.clone())
    }
    pub fn session(&self) -> &[u8] {
        self.session
    }
    pub fn expiry(&self) -> Option<u64> {
        self.expiry
    }
    pub fn structured(&self) -> bool {
        self.structured
    }
    pub fn verification(&self) -> VerifyOptions {
        VerifyOptions::new(self.metadata())
    }
    /// Parse the existing boundary options shape. Association admission remains family-scoped.
    pub fn from_value(value: &Value) -> Result<DecodeOptions<'static>, Error> {
        crate::json::bounds(value, crate::model::MAX_RECORD)?;
        let map = value
            .as_object()
            .ok_or_else(|| crate::error::invalid("options must be object"))?;
        if map.keys().any(|k| {
            !["context", "native", "session", "expiry", "structured"].contains(&k.as_str())
        }) {
            return Err(crate::error::invalid("unknown decode option"));
        }
        let mut result = DecodeOptions::default();
        if let Some(context) = value.get("context") {
            // API-2: boundary context-size admission precedes semantic errors.
            if serde_json::to_vec(context)
                .map_err(|_| crate::error::invalid("context JSON"))?
                .len()
                > crate::model::MAX_CONTEXT
            {
                return Err(crate::error::oversized("context exceeds 4096 bytes"));
            }
            crate::json::validate_context(context, true)?;
            result.context = context.clone();
        }
        if let Some(native) = value.get("native") {
            result.native = native
                .as_object()
                .ok_or_else(|| crate::error::invalid("native options must be object"))?
                .clone();
            result.via = result.native.remove("via");
        }
        if value.get("session").is_some() {
            return Err(crate::error::invalid("session option is reserved"));
        }
        if let Some(expiry) = value.get("expiry") {
            result.expiry = Some(
                expiry
                    .as_u64()
                    .ok_or_else(|| crate::error::invalid("expiry must be u64"))?,
            );
        }
        if let Some(structured) = value.get("structured") {
            result.structured = structured
                .as_bool()
                .ok_or_else(|| crate::error::invalid("structured must be boolean"))?;
        }
        Ok(result)
    }
}
/// Association only: no decoding context, state, or output switches.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct VerifyOptions {
    metadata: DecodeMetadata,
}
impl VerifyOptions {
    pub fn new(metadata: DecodeMetadata) -> Self {
        Self { metadata }
    }
    pub fn metadata(&self) -> &DecodeMetadata {
        &self.metadata
    }
    #[cfg(feature = "fldigi")]
    pub(crate) fn decode_options(&self) -> DecodeOptions<'static> {
        DecodeOptions::default().with_metadata(self.metadata.clone())
    }
}
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct EncodeOptions {
    pub(crate) native: serde_json::Map<String, Value>,
}
impl EncodeOptions {
    /// Escape hatch for options defined by an external codec.
    pub fn extension_metadata(native: serde_json::Map<String, Value>) -> Self {
        Self { native }
    }
    pub fn extensions(&self) -> &serde_json::Map<String, Value> {
        &self.native
    }
    /// Translate boundary JSON; the selected codec validates family-specific options.
    pub fn from_value(value: &Value) -> Result<Self, Error> {
        crate::json::bounds(value, crate::model::MAX_RECORD)?;
        Ok(Self::extension_metadata(
            value
                .as_object()
                .ok_or_else(|| crate::error::invalid("encoding options must be object"))?
                .clone(),
        ))
    }
}
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub enum PublicationMode {
    #[default]
    Preserved,
    Authored,
}
/// Authored validation does not by itself establish verified native evidence.
#[non_exhaustive]
pub struct PublicationOptions<'a> {
    mode: PublicationMode,
    profile: Option<&'a dyn Profile>,
    verify: VerifyOptions,
}
impl Default for PublicationOptions<'_> {
    fn default() -> Self {
        Self::new(PublicationMode::Preserved)
    }
}
impl<'a> PublicationOptions<'a> {
    pub fn new(mode: PublicationMode) -> Self {
        Self {
            mode,
            profile: None,
            verify: VerifyOptions::default(),
        }
    }
    pub fn with_profile(mut self, profile: &'a dyn Profile) -> Self {
        self.profile = Some(profile);
        self
    }
    pub fn with_verification(mut self, verify: VerifyOptions) -> Self {
        self.verify = verify;
        self
    }
    pub fn mode(&self) -> PublicationMode {
        self.mode
    }
    pub fn profile(&self) -> Option<&dyn Profile> {
        self.profile
    }
    pub fn verification(&self) -> &VerifyOptions {
        &self.verify
    }
}
/// A nonempty ordered plan. Metadata implies neither transmission nor atomic execution.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EncodedCommands {
    requests: Vec<EncodedRequest>,
}
impl EncodedCommands {
    pub fn new(requests: Vec<EncodedRequest>) -> Result<Self, Error> {
        if requests.is_empty() {
            return Err(crate::error::invalid("command batch must be nonempty"));
        }
        Ok(Self { requests })
    }
    pub fn requests(&self) -> &[EncodedRequest] {
        &self.requests
    }
    pub fn into_single(self) -> Result<Vec<u8>, Error> {
        if self.requests.len() != 1 {
            return Err(crate::error::invalid(
                "operation requires encode_commands for separate ordered requests",
            ));
        }
        Ok(self.requests.into_iter().next().expect("nonempty").bytes)
    }
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EncodedRequest {
    bytes: Vec<u8>,
    metadata: Option<DecodeMetadata>,
}
impl EncodedRequest {
    pub fn new(bytes: Vec<u8>, metadata: Option<DecodeMetadata>) -> Self {
        Self { bytes, metadata }
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn response_metadata(&self) -> Option<&DecodeMetadata> {
        self.metadata.as_ref()
    }
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

/// Optional family interpretation of a preserved event; traits remain open and object-safe.
pub trait Profile {
    fn family(&self) -> &str;
    fn mapping(&self) -> u32;
    fn native_version_supported(&self, event: &Event) -> bool;
    /// Check native consistency, returning Unavailable when interpretation or association is unavailable.
    fn verify(&self, event: &Event, options: &VerifyOptions) -> Result<Verification, Error>;
    /// Admit authored content; successful publication does not by itself establish verified evidence.
    fn validate_authored(&self, event: &Event, options: &VerifyOptions) -> Result<(), Error>;
    /// Supply a validated common interpretation, including for an external family.
    fn recognize(
        &self,
        event: &Event,
        options: &VerifyOptions,
    ) -> Result<crate::model::ProfileView, Error>;
}
/// Implementations commit state only after the native message and every output pass admission.
pub trait Codec: Profile {
    fn framing(&self) -> Framing;
    fn limits(&self) -> Limits;
    fn decode(&mut self, input: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, Error>;
    fn encode_commands(
        &self,
        event: &Event,
        options: &EncodeOptions,
    ) -> Result<EncodedCommands, Error>;
    /// Convenience for operations that produce exactly one request.
    fn encode_command(&self, event: &Event, options: &EncodeOptions) -> Result<Vec<u8>, Error> {
        self.encode_commands(event, options)?.into_single()
    }
    /// Rebuild native evidence. WSJT-X can canonicalize boolean bytes; use native_bytes for exact evidence.
    fn reencode(
        &self,
        event: &Event,
        encode: &EncodeOptions,
        verify: &VerifyOptions,
    ) -> Result<Vec<u8>, Error>;
    fn reset(&mut self, session: Option<&[u8]>);
    fn stream(&mut self) -> Option<&mut dyn StreamCodec> {
        None
    }
    fn carriage(&self) -> Option<&dyn EventCarriage> {
        None
    }
}
pub trait EventCarriage {
    /// Forward without appending a hop. Verification uses the original family's profile.
    fn encode_event(
        &self,
        event: &Event,
        options: &PublicationOptions<'_>,
    ) -> Result<Vec<u8>, Error>;
}
pub trait StreamCodec {
    /// Retain unconsumed input. Continue after Frame or Discarded, including at EOF.
    /// Only NeedMore means append input; errors still report their recovery consumption.
    fn decode_stream(&mut self, input: &[u8], eof: bool, options: &DecodeOptions<'_>)
    -> StreamStep;
}
#[derive(Debug)]
#[non_exhaustive]
pub struct StreamStep {
    consumed: usize,
    result: Result<StreamStatus, Error>,
}
impl StreamStep {
    pub fn new(consumed: usize, result: Result<StreamStatus, Error>) -> Self {
        Self { consumed, result }
    }
    pub fn consumed(&self) -> usize {
        self.consumed
    }
    pub fn into_result(self) -> Result<StreamStatus, Error> {
        self.result
    }
}
#[derive(Debug)]
#[non_exhaustive]
pub enum StreamStatus {
    NeedMore,
    Frame(Outcome),
    Discarded,
}
