// SPDX-License-Identifier: GPL-3.0-or-later
//! Semantic event carriage over datagrams and caller-buffered NDJSON.
#![forbid(unsafe_code)]
use crate::{
    Category, Error as CoreError,
    codec::*,
    error::{invalid, oversized},
    model::{Event, MAX_RECORD, Verification},
};
use serde_json::json;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Transport {
    Udp,
    Tcp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Delimiter {
    Lf,
    CrLf,
}
#[derive(Debug)]
#[non_exhaustive]
pub struct Error {
    cause: CoreError,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "qgi: {}", self.cause)
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}
fn family(e: CoreError) -> CoreError {
    CoreError::family(e.category, Error { cause: e })
}
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Qgi {
    transport: Transport,
    delimiter: Delimiter,
    discarding: bool,
}
impl Qgi {
    pub fn new(transport: Transport) -> Self {
        Self {
            transport,
            delimiter: Delimiter::Lf,
            discarding: false,
        }
    }
    pub fn with_delimiter(mut self, delimiter: Delimiter) -> Self {
        self.delimiter = delimiter;
        self
    }
    fn limit(&self) -> usize {
        match self.transport {
            Transport::Udp => 65_507,
            Transport::Tcp => MAX_RECORD,
        }
    }
    fn receive(&self, input: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, CoreError> {
        let e = crate::json::parse_carriage(input, self.limit())?;
        let mut v = e.into_value();
        // Receiving context is newly authored, even when the preserved content is newer.
        let hop = json!({"family":"qgi","context":options.context});
        let test = json!({"spec":"0.1.0","family":"qgi","mapping":1,"kind":"native","context":options.context,"fields":{},"native":{"format":"datagram","data":""}});
        crate::json::validate(&test, true)?;
        let m = v
            .as_object_mut()
            .ok_or_else(|| invalid("carried event must be an object"))?;
        let hops = m
            .entry("via")
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| invalid("via must be an array"))?;
        if hops.len() >= 8 {
            return Err(invalid("receiving would exceed eight hops"));
        }
        hops.push(hop);
        crate::json::bounds(&v, self.limit())?;
        let event = Event::from_value(v)?;
        crate::json::write(&event, false, None)?;
        Ok(Outcome::event(event))
    }
    /// Write or forward an event without adding a hop; receiving is the only hop-appending operation.
    pub fn encode_event(
        &self,
        event: &Event,
        profile: Option<&dyn Profile>,
    ) -> Result<Vec<u8>, CoreError> {
        let v = event.value();
        let newer = v["spec"] != "0.1.0"
            || v["mapping"].as_u64().is_some_and(|r| r > 1)
            || profile.is_some_and(|p| {
                v["family"] == p.family()
                    && v["mapping"]
                        .as_u64()
                        .is_some_and(|r| r > u64::from(p.mapping()))
            });
        let mut bytes = crate::json::write(event, !newer, profile).map_err(family)?;
        if bytes.len() > self.limit() {
            return Err(family(oversized(
                "qgi output exceeds transport frame limit",
            )));
        }
        crate::json::bounds(event.value(), self.limit()).map_err(family)?;
        if self.transport == Transport::Tcp {
            if self.delimiter == Delimiter::CrLf {
                bytes.push(b'\r');
            }
            bytes.push(b'\n');
        }
        Ok(bytes)
    }
}
impl Profile for Qgi {
    fn family(&self) -> &str {
        "qgi"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn verify(&self, _: &Event) -> Result<Verification, CoreError> {
        Ok(Verification::Unavailable)
    }
}
impl Codec for Qgi {
    fn framing(&self) -> Framing {
        match self.transport {
            Transport::Udp => Framing::Datagram,
            Transport::Tcp => Framing::Stream,
        }
    }
    fn limits(&self) -> Limits {
        let mut limits = Limits::stateless(self.limit(), self.limit());
        limits.parser.depth = Some(crate::json::MAX_DEPTH);
        limits.parser.elements = Some(crate::json::MAX_ELEMENTS);
        limits.parser.numeric_token_bytes = Some(crate::json::MAX_NUMBER);
        limits
    }
    fn reset(&mut self, _: Option<&[u8]>) {
        self.discarding = false;
    }
    fn decode(&mut self, input: &[u8], options: &DecodeOptions<'_>) -> Result<Outcome, CoreError> {
        if self.transport == Transport::Udp {
            return self.receive(input, options).map_err(family);
        }
        let lf = input.iter().position(|b| *b == b'\n');
        let size = lf.map_or(input.len(), |lf| {
            if lf > 0 && input[lf - 1] == b'\r' {
                lf - 1
            } else {
                lf
            }
        });
        if size > MAX_RECORD {
            return Err(family(oversized("qgi TCP frame exceeds 1 MiB")));
        }
        let lf = lf.ok_or_else(|| {
            family(CoreError::new(
                Category::Truncated,
                "unfinished qgi TCP record",
            ))
        })?;
        if lf + 1 != input.len() {
            return Err(family(invalid("decode expects one TCP record")));
        }
        self.receive(&input[..size], options).map_err(family)
    }
    fn decode_stream(
        &mut self,
        buffer: &mut Vec<u8>,
        eof: bool,
        options: &DecodeOptions<'_>,
    ) -> Result<Option<Outcome>, CoreError> {
        if self.transport != Transport::Tcp {
            return Err(family(invalid("UDP codec has no stream frames")));
        }
        if let Some(lf) = buffer.iter().position(|b| *b == b'\n') {
            let end = if lf > 0 && buffer[lf - 1] == b'\r' {
                lf - 1
            } else {
                lf
            };
            let result = if self.discarding {
                self.discarding = false;
                Ok(None)
            } else {
                self.receive(&buffer[..end], options).map(Some)
            };
            buffer.drain(..=lf);
            return result.map_err(family);
        }
        if self.discarding {
            buffer.clear();
            if eof {
                self.discarding = false;
            }
            return Ok(None);
        }
        if buffer.len() > MAX_RECORD + usize::from(buffer.last() == Some(&b'\r')) {
            buffer.clear();
            self.discarding = !eof;
            return Err(family(oversized("qgi TCP frame exceeds 1 MiB")));
        }
        if eof && !buffer.is_empty() {
            buffer.clear();
            return Err(family(CoreError::new(
                Category::Truncated,
                "unfinished qgi TCP record",
            )));
        }
        Ok(None)
    }
    fn encode_command(&self, event: &Event, _: &EncodeOptions) -> Result<Vec<u8>, CoreError> {
        self.encode_event(event, None)
    }
    fn reencode(&self, event: &Event, _: &EncodeOptions) -> Result<Vec<u8>, CoreError> {
        self.encode_event(event, None)
    }
}
