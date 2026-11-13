// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded sans-I/O protocol codecs. Callers own sockets, destinations and control policy.
//!
//! Neutral event views are available with no family features. The `serde` feature adds
//! trait integration; serde_json remains unconditional for preserved values and bounded
//! JSON. Every current family feature enables `serde`.
//!
//! `Codec::encode_commands` plans native requests; `Codec::reencode` rebuilds evidence.
//! QGI event carriage uses the optional `codec::EventCarriage` capability. Authored
//! publication uses `write` and `PublicationOptions`; it is distinct from native verification.
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
mod base64;
mod catalog;
pub mod codec;
pub mod error;
pub mod json;
pub mod model;
#[cfg(feature = "n1mm")]
pub mod n1mm;
pub use codec::{
    Codec, DecodeOptions, EncodeOptions, Outcome, Profile, PublicationMode, PublicationOptions,
    VerifyOptions,
};
pub use error::{Category, Error};
pub use json::{integer_i64, integer_u64, parse, semantic_eq, write};
pub use model::{Context, Direction, Event, Exchange, Kind, Text, Verification};
#[cfg(feature = "conformance")]
pub mod conformance;
#[cfg(feature = "qgi")]
pub mod qgi;
#[cfg(feature = "wsjtx")]
pub mod wsjtx;

#[cfg(feature = "fldigi")]
pub mod fldigi;
#[cfg(any(feature = "n1mm", feature = "fldigi"))]
mod xml;
